use std::fs::File;
use std::path::Path;

use symphonia::core::audio::sample::Sample;
use symphonia::core::audio::{Audio, AudioBuffer, GenericAudioBufferRef};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::codecs::CodecParameters;
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use soundrix_core::Pcm;

pub fn decode_file(path: &Path) -> Result<Pcm, String> {
    let file = File::open(path).map_err(|error| format!("Could not open {}: {error}", path.display()))?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
        hint.with_extension(extension);
    }
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            stream,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|error| error.to_string())?;
    let (track_id, params) = {
        let track = format
            .default_track(TrackType::Audio)
            .ok_or_else(|| "This file has no audio track.".to_string())?;
        let params = match &track.codec_params {
            Some(CodecParameters::Audio(params)) => params.clone(),
            _ => return Err("This file has no audio track.".to_string()),
        };
        (track.id, params)
    };
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .map_err(|error| error.to_string())?;

    let mut samples = Vec::new();
    let mut channels = 0u16;
    let mut rate = 0u32;
    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(Error::DecodeError(_)) => continue,
            Err(error) => return Err(error.to_string()),
        };
        if packet.track_id != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(Error::DecodeError(_) | Error::IoError(_)) => continue,
            Err(error) => return Err(error.to_string()),
        };
        let planes = decoded.num_planes() as u16;
        let decoded_rate = decoded.spec().rate();
        if planes == 0 || decoded_rate == 0 {
            continue;
        }
        if channels == 0 {
            channels = planes;
            rate = decoded_rate;
        } else if planes != channels || decoded_rate != rate {
            return Err("This file changes channel count or sample rate mid-stream.".to_string());
        }
        append_interleaved(&decoded, &mut samples);
    }
    if samples.is_empty() || channels == 0 || rate == 0 {
        return Err("This file has no audio samples.".to_string());
    }
    Ok(Pcm {
        samples,
        channels,
        rate,
    })
}

fn append_interleaved(decoded: &GenericAudioBufferRef<'_>, samples: &mut Vec<f32>) {
    match decoded {
        GenericAudioBufferRef::U8(buffer) => {
            copy_planes(buffer, samples, |sample| (sample as f32 - 128.0) / 128.0)
        }
        GenericAudioBufferRef::U16(buffer) => {
            copy_planes(buffer, samples, |sample| (sample as f32 - 32_768.0) / 32_768.0)
        }
        GenericAudioBufferRef::U24(buffer) => copy_planes(buffer, samples, |sample| {
            (sample.clamped().0 as f32 - 8_388_608.0) / 8_388_608.0
        }),
        GenericAudioBufferRef::U32(buffer) => copy_planes(buffer, samples, |sample| {
            ((sample as f64 - 2_147_483_648.0) / 2_147_483_648.0) as f32
        }),
        GenericAudioBufferRef::S8(buffer) => {
            copy_planes(buffer, samples, |sample| sample as f32 / 128.0)
        }
        GenericAudioBufferRef::S16(buffer) => {
            copy_planes(buffer, samples, |sample| sample as f32 / 32_768.0)
        }
        GenericAudioBufferRef::S24(buffer) => {
            copy_planes(buffer, samples, |sample| sample.clamped().0 as f32 / 8_388_608.0)
        }
        GenericAudioBufferRef::S32(buffer) => {
            copy_planes(buffer, samples, |sample| sample as f32 / 2_147_483_648.0)
        }
        GenericAudioBufferRef::F32(buffer) => copy_planes(buffer, samples, |sample| sample),
        GenericAudioBufferRef::F64(buffer) => copy_planes(buffer, samples, |sample| sample as f32),
    }
}

fn copy_planes<S, F>(buffer: &AudioBuffer<S>, samples: &mut Vec<f32>, convert: F)
where
    S: Sample,
    F: Fn(S) -> f32,
{
    let frames = buffer.frames();
    let planes = buffer.num_planes();
    for frame in 0..frames {
        for plane in 0..planes {
            let sample = buffer
                .plane(plane)
                .and_then(|data| data.get(frame))
                .copied();
            samples.push(sample.map(&convert).unwrap_or(0.0));
        }
    }
}
