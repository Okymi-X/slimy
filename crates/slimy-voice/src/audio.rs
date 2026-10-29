/// Gives a synthesized clip its finishing touch: trims the silence the model leaves at both ends (keeping a short natural
/// breath), fades the edges so nothing clicks, and sets a steady loudness. Returns the clip ready to play.
pub fn finish(samples: &[f32], rate: u32) -> Vec<f32> {
    if samples.is_empty() || rate == 0 {
        return Vec::new();
    }
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak < 1e-4 {
        return samples.to_vec();
    }
    let quiet = peak * 0.02;
    let keep = (rate as f32 * 0.05) as usize;
    let first = samples
        .iter()
        .position(|s| s.abs() > quiet)
        .unwrap_or(0)
        .saturating_sub(keep);
    let last = samples
        .iter()
        .rposition(|s| s.abs() > quiet)
        .map_or(samples.len(), |i| (i + keep).min(samples.len()));
    let mut out = samples[first..last.max(first + 1)].to_vec();
    // steady loudness: about 0.1 RMS, never beyond a soft ceiling
    let rms = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
    let gain = if rms > 1e-5 {
        (0.1 / rms).clamp(0.3, 10.0)
    } else {
        1.0
    };
    for s in out.iter_mut() {
        let v = *s * gain;
        *s = if v.abs() > 0.8 {
            v.signum() * (0.8 + 0.15 * ((v.abs() - 0.8) / 0.15).tanh())
        } else {
            v
        };
    }
    let fade = ((rate as f32 * 0.012) as usize).min(out.len() / 2);
    for i in 0..fade {
        let k = i as f32 / fade as f32;
        out[i] *= k;
        let n = out.len();
        out[n - 1 - i] *= k;
    }
    out
}

/// A pause of the given length, in seconds, as samples.
pub fn silence(rate: u32, seconds: f32) -> Vec<f32> {
    vec![0.0; (rate as f32 * seconds) as usize]
}
