//! WAV import / export.

use std::path::Path;

pub fn resample(data: &[f32], from: f32, to: f32) -> Vec<f32> {
    if (from - to).abs() < 1.0 || data.is_empty() {
        return data.to_vec();
    }
    let ratio = from / to;
    let n = (data.len() as f32 / ratio) as usize;
    (0..n)
        .map(|i| {
            let x = i as f32 * ratio;
            let i0 = x as usize;
            let i1 = (i0 + 1).min(data.len() - 1);
            let f = x - i0 as f32;
            data[i0] * (1.0 - f) + data[i1] * f
        })
        .collect()
}

/// Loads a WAV file, mixes it down to mono and resamples it to `target_sr`.
pub fn load_wav(path: &Path, target_sr: f32) -> Result<Vec<f32>, String> {
    let mut r = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = r.spec();
    let ch = spec.channels.max(1) as usize;
    let raw: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => r.samples::<f32>().filter_map(Result::ok).collect(),
        hound::SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            r.samples::<i32>().filter_map(Result::ok).map(|s| s as f32 / max).collect()
        }
    };
    let mono: Vec<f32> = raw.chunks(ch).map(|c| c.iter().sum::<f32>() / ch as f32).collect();
    Ok(resample(&mono, spec.sample_rate as f32, target_sr))
}

/// Saves interleaved stereo samples as 16-bit WAV.
pub fn save_wav_stereo(path: &Path, data: &[f32], sr: u32) -> Result<(), String> {
    let spec = hound::WavSpec { channels: 2, sample_rate: sr, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &s in data {
        w.write_sample((s.clamp(-1.0, 1.0) * 32767.0) as i16).map_err(|e| e.to_string())?;
    }
    w.finalize().map_err(|e| e.to_string())
}
