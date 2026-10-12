/// Interleaved PCM owned by one playing sound.
#[derive(Clone, Debug)]
pub struct Pcm {
    pub samples: Vec<f32>,
    pub channels: u16,
    pub rate: u32,
}

/// Add one sound into an interleaved output buffer.
///
/// `frame` is the source position in frames and may sit between samples.
/// The next position is returned while the sound still has frames left.
pub fn mix_pcm(
    source: &[f32],
    source_channels: u16,
    source_rate: u32,
    frame: f64,
    output: &mut [f32],
    output_channels: u16,
    output_rate: u32,
) -> Option<f64> {
    let source_channels = source_channels as usize;
    let output_channels = output_channels as usize;
    if source_channels == 0 || output_channels == 0 || source_rate == 0 || output_rate == 0 {
        return None;
    }
    let source_frames = source.len() / source_channels;
    if source_frames == 0 || frame >= source_frames as f64 {
        return None;
    }
    let output_frames = output.len() / output_channels;
    let step = source_rate as f64 / output_rate as f64;
    for index in 0..output_frames {
        let position = frame + index as f64 * step;
        if position >= source_frames as f64 {
            return None;
        }
        let left = position.floor() as usize;
        let right = (left + 1).min(source_frames - 1);
        let fraction = (position - left as f64) as f32;
        for channel in 0..output_channels {
            let start = sample_at(source, source_channels, left, channel, output_channels);
            let end = sample_at(source, source_channels, right, channel, output_channels);
            let sample = start + (end - start) * fraction;
            let slot = index * output_channels + channel;
            output[slot] = (output[slot] + sample).clamp(-1.0, 1.0);
        }
    }
    let next = frame + output_frames as f64 * step;
    if next >= source_frames as f64 {
        None
    } else {
        Some(next)
    }
}

fn sample_at(
    source: &[f32],
    source_channels: usize,
    frame: usize,
    output_channel: usize,
    output_channels: usize,
) -> f32 {
    if source_channels == 1 {
        return source.get(frame).copied().unwrap_or(0.0);
    }
    if output_channels == 1 {
        let mut sum = 0.0;
        for channel in 0..source_channels {
            sum += source
                .get(frame * source_channels + channel)
                .copied()
                .unwrap_or(0.0);
        }
        return sum / source_channels as f32;
    }
    let channel = output_channel.min(source_channels - 1);
    source
        .get(frame * source_channels + channel)
        .copied()
        .unwrap_or(0.0)
}

/// Pick the render end of a virtual cable. The capture end is what a chat app selects.
pub fn prefer_virtual_mic(names: &[String]) -> Option<&str> {
    if let Some(name) = names
        .iter()
        .find(|name| name.to_ascii_lowercase().contains("cable input"))
    {
        return Some(name);
    }
    names
        .iter()
        .find(|name| {
            let lower = name.to_ascii_lowercase();
            lower.contains("vb-audio") && !lower.contains("output")
        })
        .map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::{mix_pcm, prefer_virtual_mic};

    #[test]
    fn mix_pcm_should_interpolate_when_the_output_rate_is_higher() {
        let mut output = [0.0; 2];
        let next = mix_pcm(&[0.0, 1.0], 1, 100, 0.0, &mut output, 1, 200);
        assert!((output[0] - 0.0).abs() < 0.0001);
        assert!((output[1] - 0.5).abs() < 0.0001);
        assert_eq!(next, Some(1.0));
    }

    #[test]
    fn mix_pcm_should_copy_a_mono_sound_to_both_speakers() {
        let mut output = [0.0; 2];
        let next = mix_pcm(&[0.25], 1, 48_000, 0.0, &mut output, 2, 48_000);
        assert_eq!(output, [0.25, 0.25]);
        assert_eq!(next, None);
    }

    #[test]
    fn mix_pcm_should_clamp_overlapping_samples() {
        let mut output = [0.8];
        let next = mix_pcm(&[0.8], 1, 48_000, 0.0, &mut output, 1, 48_000);
        assert_eq!(output, [1.0]);
        assert_eq!(next, None);
    }

    #[test]
    fn prefer_virtual_mic_should_choose_cable_input_over_cable_output() {
        let names = vec![
            "Speakers".to_string(),
            "CABLE Output (VB-Audio Virtual Cable)".to_string(),
            "CABLE Input (VB-Audio Virtual Cable)".to_string(),
        ];
        assert_eq!(
            prefer_virtual_mic(&names),
            Some("CABLE Input (VB-Audio Virtual Cable)")
        );
    }

    #[test]
    fn prefer_virtual_mic_should_skip_the_capture_end() {
        let names = vec!["CABLE Output (VB-Audio Virtual Cable)".to_string()];
        assert_eq!(prefer_virtual_mic(&names), None);
    }
}
