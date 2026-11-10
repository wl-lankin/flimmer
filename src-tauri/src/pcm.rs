//! Raw audio out of any media file, through an invisible mpv writing 16-bit mono PCM.

use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};

use crate::mpv::ffi::{MPV_EVENT_END_FILE, MPV_EVENT_SHUTDOWN};
use crate::mpv::Mpv;

/// `length` seconds from `start` at `rate` Hz, mono, as -1.0..1.0 samples.
pub fn mono(path: &Path, start: f64, length: f64, rate: usize) -> Result<Vec<f32>> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let out = std::env::temp_dir().join(format!("flimmer-{}-{stamp}.pcm", std::process::id()));
    let options: Vec<(&str, String)> = vec![
        ("config", "no".into()),
        ("vo", "null".into()),
        ("vid", "no".into()),
        ("sid", "no".into()),
        ("ao", "pcm".into()),
        ("ao-pcm-file", out.to_string_lossy().into_owned()),
        ("ao-pcm-waveheader", "no".into()),
        ("audio-samplerate", rate.to_string()),
        ("audio-channels", "mono".into()),
        ("audio-format", "s16".into()),
        ("untimed", "yes".into()),
        ("start", format!("{start}")),
        ("length", format!("{length}")),
        ("resume-playback", "no".into()),
        ("ytdl", "no".into()),
        ("load-scripts", "no".into()),
        ("idle", "yes".into()),
    ];
    let mpv = Mpv::new(&options)?;
    let result = (|| {
        mpv.command(&["loadfile".into(), path.to_string_lossy().into_owned()])?;
        let deadline = Instant::now() + Duration::from_secs(300);
        loop {
            if Instant::now() > deadline {
                bail!("audio extraction timed out");
            }
            match mpv.wait_event(0.5) {
                MPV_EVENT_END_FILE | MPV_EVENT_SHUTDOWN => break,
                _ => {}
            }
        }
        Ok(())
    })();
    // SAFETY: this handle has no render context and is not used again.
    unsafe { mpv.destroy() };
    result?;
    let bytes = std::fs::read(&out).unwrap_or_default();
    let _ = std::fs::remove_file(&out);
    Ok(bytes
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
        .collect())
}
