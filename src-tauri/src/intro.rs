//! Finds a series intro by listening: the opening minutes of two episodes from the same folder
//! are turned into audio fingerprints, and the longest stretch they share is the intro.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::Result;
use rustfft::{num_complex::Complex, FftPlanner};
use serde::{Deserialize, Serialize};

const RATE: usize = 8000;
/// How much of each episode is compared.
const SPAN: f64 = 360.0;
const WINDOW: usize = 2048;
const HOP: usize = 800;
const FRAME_SECONDS: f64 = HOP as f64 / RATE as f64;
const MIN_INTRO: f64 = 12.0;
const MAX_INTRO: f64 = 180.0;
const VIDEO: &[&str] = &[
    "mkv", "mp4", "m4v", "mov", "webm", "avi", "wmv", "ts", "m2ts", "mpg", "mpeg", "flv",
];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Intro {
    pub start: f64,
    pub end: f64,
}

fn audio(path: &Path, start: f64, length: f64) -> Result<Vec<f32>> {
    crate::pcm::mono(path, start, length, RATE)
}

/// Marks a frame too quiet to compare; silence would otherwise match any other silence.
const SILENT: u32 = 1 << 31;

/// One 16-bit print per frame: the signs of how the energy difference between neighbouring
/// bands changed since the previous frame (Haitsma and Kalker). Robust to volume and EQ.
pub fn fingerprint(samples: &[f32]) -> Vec<u32> {
    const BANDS: usize = 17;
    if samples.len() < WINDOW {
        return Vec::new();
    }
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(WINDOW);
    let hann: Vec<f32> = (0..WINDOW)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / WINDOW as f32).cos())
        .collect();
    // Log-spaced band edges between 300 Hz and 2000 Hz, as FFT bins.
    let edges: Vec<usize> = (0..=BANDS)
        .map(|b| {
            let hz = 300.0 * (2000.0f32 / 300.0).powf(b as f32 / BANDS as f32);
            (hz * WINDOW as f32 / RATE as f32) as usize
        })
        .collect();
    let mut prev: Option<[f32; BANDS]> = None;
    let mut prints = Vec::new();
    let mut buf = vec![Complex::new(0.0f32, 0.0); WINDOW];
    let mut pos = 0;
    while pos + WINDOW <= samples.len() {
        for (i, c) in buf.iter_mut().enumerate() {
            *c = Complex::new(samples[pos + i] * hann[i], 0.0);
        }
        fft.process(&mut buf);
        let mut energy = [0f32; BANDS];
        for (b, e) in energy.iter_mut().enumerate() {
            *e = buf[edges[b]..edges[b + 1].max(edges[b] + 1)]
                .iter()
                .map(|c| c.norm_sqr())
                .sum::<f32>()
                .max(1e-9)
                .ln();
        }
        let rms = (samples[pos..pos + WINDOW]
            .iter()
            .map(|v| v * v)
            .sum::<f32>()
            / WINDOW as f32)
            .sqrt();
        if rms < 0.003 {
            prints.push(SILENT);
        } else if let Some(p) = prev {
            let mut bits = 0u32;
            for b in 0..BANDS - 1 {
                if (energy[b] - energy[b + 1]) - (p[b] - p[b + 1]) > 0.0 {
                    bits |= 1 << b;
                }
            }
            prints.push(bits);
        } else {
            prints.push(SILENT);
        }
        prev = Some(energy);
        pos += HOP;
    }
    prints
}

/// Bit errors between two frames; a silent frame counts as half wrong, like noise.
fn distance(x: u32, y: u32) -> u32 {
    if (x | y) & SILENT != 0 {
        8
    } else {
        (x ^ y).count_ones()
    }
}

/// The longest stretch of `a` that also occurs in `b` (at any offset), as frame indices into
/// `a`. Two stretches match where the mean bit error over a few seconds stays well below the
/// 8 of 16 that unrelated audio gives.
pub fn longest_shared(a: &[u32], b: &[u32]) -> Option<(usize, usize)> {
    const SMOOTH: usize = 30;
    const MAX_MEAN_ERRORS: f32 = 5.5;
    let limit = (MAX_MEAN_ERRORS * SMOOTH as f32) as u32;
    let min_frames = (MIN_INTRO / FRAME_SECONDS) as usize;
    let mut best: Option<(usize, usize)> = None;
    let n = a.len() as isize;
    let m = b.len() as isize;
    let mut errors = Vec::with_capacity(a.len());
    for offset in -(m - 1)..n {
        // Frame i of `a` against frame i - offset of `b`.
        let lo = offset.max(0) as usize;
        let hi = (m + offset).min(n) as usize;
        if hi < lo + min_frames.max(SMOOTH) {
            continue;
        }
        errors.clear();
        errors.extend((lo..hi).map(|i| distance(a[i], b[(i as isize - offset) as usize])));
        let mut run_start = None;
        let mut window: u32 = errors[..SMOOTH].iter().sum();
        let last = errors.len() - SMOOTH;
        for k in 0..=last {
            // `window` holds the errors of errors[k..k + SMOOTH].
            let close = window <= limit;
            match (close, run_start) {
                (true, None) => run_start = Some(k),
                (false, Some(s)) => {
                    consider(&mut best, lo + s, lo + k - 1 + SMOOTH);
                    run_start = None;
                }
                _ => {}
            }
            if k < last {
                window = window - errors[k] + errors[k + SMOOTH];
            }
        }
        if let Some(s) = run_start {
            consider(&mut best, lo + s, lo + last + SMOOTH);
        }
    }
    best.filter(|(s, e)| e - s >= min_frames)
}

fn consider(best: &mut Option<(usize, usize)>, start: usize, end: usize) {
    if best.is_none_or(|(s, e)| end - start > e - s) {
        *best = Some((start, end));
    }
}

/// Other episodes in the same folder, nearest by name first.
fn siblings(path: &Path) -> Vec<PathBuf> {
    let Some(dir) = path.parent() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| VIDEO.contains(&e.to_ascii_lowercase().as_str()))
        })
        .collect();
    files.sort();
    let Some(me) = files.iter().position(|p| p == path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for d in 1..files.len() {
        if me + d < files.len() {
            out.push(files[me + d].clone());
        }
        if d <= me {
            out.push(files[me - d].clone());
        }
    }
    out
}

/// Remembers results per file (path, size, modification time) for the session and on disk.
pub struct Cache {
    file: PathBuf,
    map: Mutex<HashMap<String, Option<Intro>>>,
}

impl Cache {
    pub fn load(file: PathBuf) -> Self {
        let map = std::fs::read_to_string(&file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Cache {
            file,
            map: Mutex::new(map),
        }
    }

    fn key(path: &Path) -> Option<String> {
        let meta = std::fs::metadata(path).ok()?;
        let modified = meta
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        Some(format!("{}|{}|{}", path.display(), meta.len(), modified))
    }

    pub fn detect(&self, path: &Path) -> Result<Option<Intro>> {
        let Some(key) = Self::key(path) else {
            return Ok(None);
        };
        if let Some(hit) = self.map.lock().unwrap().get(&key) {
            return Ok(*hit);
        }
        let found = detect(path)?;
        let mut map = self.map.lock().unwrap();
        map.insert(key, found);
        if let Ok(text) = serde_json::to_string(&*map) {
            let _ = std::fs::write(&self.file, text);
        }
        Ok(found)
    }
}

/// Compares the episode with up to two neighbours and returns the intro in its own timeline.
pub fn detect(path: &Path) -> Result<Option<Intro>> {
    let others = siblings(path);
    if others.is_empty() {
        return Ok(None);
    }
    let mine = fingerprint(&audio(path, 0.0, SPAN)?);
    for other in others.iter().take(2) {
        let Ok(samples) = audio(other, 0.0, SPAN) else {
            continue;
        };
        if let Some((s, e)) = longest_shared(&mine, &fingerprint(&samples)) {
            let intro = Intro {
                start: s as f64 * FRAME_SECONDS,
                end: e as f64 * FRAME_SECONDS,
            };
            if intro.end - intro.start <= MAX_INTRO {
                return Ok(Some(intro));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic noise with a slowly changing tone, so neighbouring frames differ.
    fn signal(seed: u32, seconds: f64) -> Vec<f32> {
        let mut x = seed;
        (0..(seconds * RATE as f64) as usize)
            .map(|i| {
                x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let noise = (x >> 8) as f32 / (1u32 << 24) as f32 - 0.5;
                let f = 400.0 + 300.0 * ((i as f32 / RATE as f32) * 0.7 + seed as f32).sin();
                0.6 * (2.0 * std::f32::consts::PI * f * i as f32 / RATE as f32).sin() + 0.4 * noise
            })
            .collect()
    }

    #[test]
    fn finds_a_shared_intro_at_different_offsets() {
        let intro = signal(7, 40.0);
        // Episode A: 20 s cold open, intro, rest. Episode B: 55 s cold open, intro, rest.
        let a: Vec<f32> = [signal(1, 20.0), intro.clone(), signal(2, 60.0)].concat();
        let b: Vec<f32> = [signal(3, 55.0), intro, signal(4, 60.0)].concat();
        let (s, e) = longest_shared(&fingerprint(&a), &fingerprint(&b)).expect("intro found");
        let (start, end) = (s as f64 * FRAME_SECONDS, e as f64 * FRAME_SECONDS);
        assert!((start - 20.0).abs() < 2.5, "start {start}");
        assert!((end - 60.0).abs() < 2.5, "end {end}");
    }

    #[test]
    fn unrelated_episodes_share_nothing() {
        let a = fingerprint(&signal(11, 90.0));
        let b = fingerprint(&signal(12, 90.0));
        assert_eq!(longest_shared(&a, &b), None);
    }
}

#[cfg(test)]
mod real_files {
    /// Needs testmedia/serie (made by hand with ffmpeg, not in git): run with --ignored.
    #[test]
    #[ignore]
    fn finds_the_intro_in_real_episodes() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../testmedia/serie");
        let started = std::time::Instant::now();
        let intro = super::detect(&dir.join("Serie S01E01.mkv"))
            .unwrap()
            .expect("intro");
        eprintln!(
            "E01 intro {:.1}-{:.1} s in {:?}",
            intro.start,
            intro.end,
            started.elapsed()
        );
        assert!((intro.start - 25.0).abs() < 3.0 && (intro.end - 60.0).abs() < 3.0);
        let intro = super::detect(&dir.join("Serie S01E02.mkv"))
            .unwrap()
            .expect("intro");
        eprintln!("E02 intro {:.1}-{:.1} s", intro.start, intro.end);
        assert!((intro.start - 50.0).abs() < 3.0 && (intro.end - 85.0).abs() < 3.0);
    }
}
