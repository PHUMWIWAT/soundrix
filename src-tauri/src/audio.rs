use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, SampleFormat, StreamConfig};
use serde::{Deserialize, Serialize};
use soundrix_core::{mix_pcm, prefer_virtual_mic, Pcm};
use tauri::{AppHandle, Manager};

const MIC_MISSING: &str = "No virtual microphone output. Install VB-Audio Cable, then choose CABLE Input here and set the chat app microphone to CABLE Output.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayMode {
    Both,
    Speakers,
    Microphone,
}

impl Default for PlayMode {
    fn default() -> Self {
        Self::Both
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    mode: PlayMode,
    speaker_device: String,
    mic_device: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: PlayMode::Both,
            speaker_device: String::new(),
            mic_device: String::new(),
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioStatus {
    pub mode: PlayMode,
    pub speaker_device: String,
    pub mic_device: String,
    pub speaker_choice: String,
    pub mic_choice: String,
    pub devices: Vec<String>,
    pub playing: Vec<String>,
    pub hint: String,
}

struct Voice {
    sound_id: String,
    pcm: Arc<Pcm>,
    speakers: Option<f64>,
    mic: Option<f64>,
}

struct Mixer {
    voices: Vec<Voice>,
}

pub struct Engine {
    host: cpal::Host,
    settings_path: PathBuf,
    settings: Settings,
    mixer: Arc<Mutex<Mixer>>,
    speakers: Option<cpal::Stream>,
    mic: Option<cpal::Stream>,
    speaker_open: String,
    mic_open: String,
    last_error: Arc<Mutex<String>>,
}

pub struct AudioState(pub Mutex<Engine>);

impl Engine {
    pub fn open(app: &AppHandle) -> Result<Self, String> {
        let dir = app.path().app_data_dir().map_err(|error| error.to_string())?;
        let settings_path = dir.join("audio.json");
        let settings = fs::read_to_string(&settings_path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Ok(Self {
            host: cpal::default_host(),
            settings_path,
            settings,
            mixer: Arc::new(Mutex::new(Mixer { voices: Vec::new() })),
            speakers: None,
            mic: None,
            speaker_open: String::new(),
            mic_open: String::new(),
            last_error: Arc::new(Mutex::new(String::new())),
        })
    }

    pub fn status(&mut self) -> AudioStatus {
        self.recover();
        let devices = output_names(&self.host);
        let speaker = speaker_name(&self.host, &self.settings, &devices);
        let mic = mic_name(&self.settings, &devices);
        let needs_speakers = matches!(self.settings.mode, PlayMode::Both | PlayMode::Speakers);
        let needs_mic = matches!(self.settings.mode, PlayMode::Both | PlayMode::Microphone);
        let mut hints = Vec::new();
        if needs_speakers {
            if let Err(message) = &speaker {
                hints.push(message.clone());
            }
        }
        if needs_mic {
            hints.push(mic_hint(mic.as_deref().map_err(String::as_str)));
        }
        AudioStatus {
            mode: self.settings.mode,
            speaker_device: speaker.unwrap_or_default(),
            mic_device: mic.unwrap_or_default(),
            speaker_choice: self.settings.speaker_device.clone(),
            mic_choice: self.settings.mic_device.clone(),
            devices,
            playing: self.playing_ids(),
            hint: hints.join(" "),
        }
    }

    pub fn apply(
        &mut self,
        mode: PlayMode,
        speaker_device: String,
        mic_device: String,
    ) -> Result<AudioStatus, String> {
        self.settings.mode = mode;
        self.settings.speaker_device = speaker_device;
        self.settings.mic_device = mic_device;
        self.save()?;
        self.speakers = None;
        self.mic = None;
        self.speaker_open.clear();
        self.mic_open.clear();
        self.reopen_active()?;
        Ok(self.status())
    }

    pub fn play(&mut self, pcm: Pcm, sound_id: String, mode: PlayMode) -> Result<(), String> {
        self.recover();
        let speakers = matches!(mode, PlayMode::Both | PlayMode::Speakers);
        let mic = matches!(mode, PlayMode::Both | PlayMode::Microphone);
        if speakers {
            self.ensure(false)?;
        }
        if mic {
            self.ensure(true)?;
        }
        let mut mixer = self.mixer.lock().map_err(|error| error.to_string())?;
        mixer.voices.push(Voice {
            sound_id,
            pcm: Arc::new(pcm),
            speakers: speakers.then_some(0.0),
            mic: mic.then_some(0.0),
        });
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.voices.clear();
        }
    }

    fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.settings_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let text = serde_json::to_string_pretty(&self.settings).map_err(|error| error.to_string())?;
        fs::write(&self.settings_path, text).map_err(|error| error.to_string())
    }

    fn recover(&mut self) {
        let mut error = match self.last_error.lock() {
            Ok(error) => error,
            Err(poisoned) => poisoned.into_inner(),
        };
        if error.is_empty() {
            return;
        }
        error.clear();
        drop(error);
        self.speakers = None;
        self.mic = None;
        self.speaker_open.clear();
        self.mic_open.clear();
    }

    fn reopen_active(&mut self) -> Result<(), String> {
        let (speakers, mic) = {
            let mixer = self.mixer.lock().map_err(|error| error.to_string())?;
            (
                mixer.voices.iter().any(|voice| voice.speakers.is_some()),
                mixer.voices.iter().any(|voice| voice.mic.is_some()),
            )
        };
        if speakers {
            self.ensure(false)?;
        }
        if mic {
            self.ensure(true)?;
        }
        Ok(())
    }

    fn ensure(&mut self, mic_bus: bool) -> Result<(), String> {
        let devices = output_names(&self.host);
        let name = if mic_bus {
            mic_name(&self.settings, &devices)?
        } else {
            speaker_name(&self.host, &self.settings, &devices)?
        };
        let current = if mic_bus {
            &self.mic_open
        } else {
            &self.speaker_open
        };
        let open = if mic_bus {
            self.mic.is_some()
        } else {
            self.speakers.is_some()
        };
        if open && current == &name {
            return Ok(());
        }
        let device = find_device(&self.host, &name)?;
        let stream = open_output(
            &device,
            self.mixer.clone(),
            mic_bus,
            self.last_error.clone(),
        )?;
        if mic_bus {
            self.mic = Some(stream);
            self.mic_open = name;
        } else {
            self.speakers = Some(stream);
            self.speaker_open = name;
        }
        Ok(())
    }

    fn playing_ids(&self) -> Vec<String> {
        let Ok(mixer) = self.mixer.lock() else {
            return Vec::new();
        };
        let mut playing = Vec::new();
        for voice in &mixer.voices {
            if !playing.contains(&voice.sound_id) {
                playing.push(voice.sound_id.clone());
            }
        }
        playing
    }
}

fn device_name(device: &cpal::Device) -> Option<String> {
    device
        .description()
        .ok()
        .map(|description| description.name().to_string())
}

fn output_names(host: &cpal::Host) -> Vec<String> {
    host.output_devices()
        .map(|devices| devices.filter_map(|device| device_name(&device)).collect())
        .unwrap_or_default()
}

fn speaker_name(host: &cpal::Host, settings: &Settings, devices: &[String]) -> Result<String, String> {
    if settings.speaker_device.is_empty() {
        return host
            .default_output_device()
            .and_then(|device| device_name(&device))
            .ok_or_else(|| "No speaker output is available.".to_string());
    }
    named_device(&settings.speaker_device, devices)
}

fn mic_name(settings: &Settings, devices: &[String]) -> Result<String, String> {
    if settings.mic_device.is_empty() {
        return prefer_virtual_mic(devices)
            .map(str::to_string)
            .ok_or_else(|| MIC_MISSING.to_string());
    }
    named_device(&settings.mic_device, devices)
}

fn named_device(name: &str, devices: &[String]) -> Result<String, String> {
    if devices.iter().any(|device| device == name) {
        Ok(name.to_string())
    } else {
        Err(format!("Output device \"{name}\" is not connected."))
    }
}

fn mic_hint(mic: Result<&str, &str>) -> String {
    match mic {
        Err(message) => message.to_string(),
        Ok(name) => {
            let lower = name.to_ascii_lowercase();
            if lower.contains("cable output") {
                "CABLE Output is the microphone side. Choose CABLE Input as the microphone output, and set the chat app microphone to CABLE Output.".to_string()
            } else if lower.contains("cable input") {
                "Set the chat app microphone to CABLE Output. Soundrix plays into CABLE Input.".to_string()
            } else {
                format!("Set the chat app microphone to \"{name}\" when that device is the capture side of a virtual cable.")
            }
        }
    }
}

fn find_device(host: &cpal::Host, name: &str) -> Result<cpal::Device, String> {
    let devices = host.output_devices().map_err(|error| error.to_string())?;
    for device in devices {
        if device_name(&device).as_deref() == Some(name) {
            return Ok(device);
        }
    }
    Err(format!("Output device \"{name}\" is not connected."))
}

fn preferred_config(device: &cpal::Device) -> Result<cpal::SupportedStreamConfig, String> {
    let default = device.default_output_config().map_err(|error| error.to_string())?;
    if default.sample_format() == SampleFormat::F32 {
        return Ok(default);
    }
    let rate = default.sample_rate();
    let channels = default.channels();
    let supported = device
        .supported_output_configs()
        .map_err(|error| error.to_string())?;
    if let Some(range) = supported.into_iter().find(|range| {
        range.channels() == channels
            && range.sample_format() == SampleFormat::F32
            && range.min_sample_rate() <= rate
            && rate <= range.max_sample_rate()
    }) {
        return Ok(range.with_sample_rate(rate));
    }
    Ok(default)
}

fn open_output(
    device: &cpal::Device,
    mixer: Arc<Mutex<Mixer>>,
    mic_bus: bool,
    errors: Arc<Mutex<String>>,
) -> Result<cpal::Stream, String> {
    let supported = preferred_config(device)?;
    let channels = supported.channels();
    let rate = supported.sample_rate();
    let format = supported.sample_format();
    // Shared-mode devices often reject a 256-frame buffer. Fall back before giving up.
    let sizes = [
        BufferSize::Fixed(256),
        BufferSize::Fixed(512),
        BufferSize::Fixed(1024),
        BufferSize::Default,
    ];
    let mut last = "Could not open the output device.".to_string();
    for size in sizes {
        let mut config = supported.config();
        config.buffer_size = size;
        match build_stream(device, config, format, mixer.clone(), channels, rate, mic_bus, errors.clone())
        {
            Ok(stream) => {
                stream.play().map_err(|error| error.to_string())?;
                return Ok(stream);
            }
            Err(error) => last = error,
        }
    }
    Err(last)
}

fn build_stream(
    device: &cpal::Device,
    config: StreamConfig,
    format: SampleFormat,
    mixer: Arc<Mutex<Mixer>>,
    channels: u16,
    rate: u32,
    mic_bus: bool,
    errors: Arc<Mutex<String>>,
) -> Result<cpal::Stream, String> {
    let report = |errors: &Arc<Mutex<String>>| {
        let errors = errors.clone();
        move |error: cpal::Error| {
            if let Ok(mut slot) = errors.lock() {
                *slot = error.to_string();
            }
        }
    };
    match format {
        SampleFormat::F32 => device
            .build_output_stream(
                config,
                move |data: &mut [f32], _| render(&mixer, data, channels, rate, mic_bus),
                report(&errors),
                None,
            )
            .map_err(|error| error.to_string()),
        SampleFormat::I16 => {
            let mut scratch = Vec::<f32>::new();
            device
                .build_output_stream(
                    config,
                    move |data: &mut [i16], _| {
                        scratch.resize(data.len(), 0.0);
                        render(&mixer, &mut scratch, channels, rate, mic_bus);
                        for (output, sample) in data.iter_mut().zip(scratch.iter()) {
                            *output = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                        }
                    },
                    report(&errors),
                    None,
                )
                .map_err(|error| error.to_string())
        }
        _ => Err(
            "This output device uses a sample format Soundrix cannot play. Choose another device."
                .to_string(),
        ),
    }
}

fn render(mixer: &Mutex<Mixer>, output: &mut [f32], channels: u16, rate: u32, mic_bus: bool) {
    output.fill(0.0);
    let Ok(mut mixer) = mixer.lock() else {
        return;
    };
    for voice in &mut mixer.voices {
        let cursor = if mic_bus {
            &mut voice.mic
        } else {
            &mut voice.speakers
        };
        let Some(frame) = *cursor else {
            continue;
        };
        *cursor = mix_pcm(
            &voice.pcm.samples,
            voice.pcm.channels,
            voice.pcm.rate,
            frame,
            output,
            channels,
            rate,
        );
    }
    mixer
        .voices
        .retain(|voice| voice.speakers.is_some() || voice.mic.is_some());
}

pub fn parse_mode(value: &str) -> Result<PlayMode, String> {
    match value {
        "both" => Ok(PlayMode::Both),
        "speakers" => Ok(PlayMode::Speakers),
        "microphone" => Ok(PlayMode::Microphone),
        _ => Err("Play mode must be both, speakers, or microphone.".to_string()),
    }
}
