//! Subtitles from nothing: whisper.cpp writes down what is said, on this machine, and a local
//! Ollama (when one runs) translates it. Every transcript is kept for searching later.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const MODEL_FILE: &str = "ggml-large-v3-turbo-q5_0.bin";
const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin";
const RATE: usize = 16_000;
/// Audio is transcribed in pieces this long, so memory stays small and text appears early.
const CHUNK: f64 = 600.0;
const OLLAMA: &str = "http://127.0.0.1:11434";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transcript {
    pub path: String,
    pub lang: String,
    pub srt: PathBuf,
    pub translated: Option<PathBuf>,
    pub segments: Vec<Segment>,
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Progress {
    Model {
        done: u64,
        total: u64,
    },
    Listening {
        percent: f32,
        srt: Option<PathBuf>,
        lang: String,
    },
    Translating {
        percent: f32,
    },
    Done {
        srt: PathBuf,
        lang: String,
        translated: Option<PathBuf>,
    },
    Failed {
        message: String,
    },
}

/// One job at a time; a new request cancels nothing, it is refused while one runs.
#[derive(Default)]
pub struct Jobs {
    busy: AtomicBool,
    cancel: Arc<AtomicBool>,
    index: Mutex<Option<Vec<Transcript>>>,
}

fn data_dir(base: &Path) -> PathBuf {
    base.join("transcripts")
}

fn key(path: &str) -> String {
    let digest = Sha256::digest(path.as_bytes());
    digest.iter().take(10).map(|b| format!("{b:02x}")).collect()
}

/// SRT timestamps: 01:02:03,456.
pub fn srt_time(t: f64) -> String {
    let ms = (t.max(0.0) * 1000.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

pub fn to_srt(segments: &[Segment]) -> String {
    let mut out = String::new();
    for (i, s) in segments.iter().enumerate() {
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            srt_time(s.start),
            srt_time(s.end),
            s.text.trim()
        ));
    }
    out
}

fn download_model(dir: &Path, cancel: &AtomicBool, emit: &dyn Fn(Progress)) -> Result<PathBuf> {
    let path = dir.join(MODEL_FILE);
    if path.exists() {
        return Ok(path);
    }
    std::fs::create_dir_all(dir)?;
    let client = reqwest::blocking::Client::builder().timeout(None).build()?;
    let mut response = client.get(MODEL_URL).send()?.error_for_status()?;
    let total = response.content_length().unwrap_or(0);
    let part = path.with_extension("part");
    let mut file = std::fs::File::create(&part)?;
    let mut buf = vec![0u8; 1 << 20];
    let mut done = 0u64;
    let mut last = 0u64;
    loop {
        if cancel.load(Ordering::Relaxed) {
            drop(file);
            let _ = std::fs::remove_file(&part);
            bail!("cancelled");
        }
        let n = response.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        done += n as u64;
        if done - last > 8 << 20 {
            last = done;
            emit(Progress::Model { done, total });
        }
    }
    file.flush()?;
    if total > 0 && done != total {
        let _ = std::fs::remove_file(&part);
        bail!("model download incomplete ({done} of {total} bytes)");
    }
    std::fs::rename(&part, &path)?;
    Ok(path)
}

/// Transcribes one piece of audio; timestamps come back relative to the piece.
fn listen(
    ctx: &WhisperContext,
    samples: &[f32],
    lang: Option<&str>,
) -> Result<(Vec<Segment>, String)> {
    let mut state = ctx
        .create_state()
        .map_err(|e| anyhow!("whisper state: {e:?}"))?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some(lang.unwrap_or("auto")));
    params.set_translate(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);
    params.set_suppress_blank(true);
    params.set_n_threads(
        std::thread::available_parallelism()
            .map(|n| n.get().min(8) as i32)
            .unwrap_or(4),
    );
    let found = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&found);
    params.set_segment_callback_safe(move |s: whisper_rs::SegmentCallbackData| {
        let text = s.text.trim().to_string();
        if !text.is_empty() {
            sink.lock().unwrap().push(Segment {
                start: s.start_timestamp as f64 / 100.0,
                end: s.end_timestamp as f64 / 100.0,
                text,
            });
        }
    });
    state
        .full(params, samples)
        .map_err(|e| anyhow!("whisper: {e:?}"))?;
    let detected = whisper_rs::get_lang_str(state.full_lang_id_from_state())
        .unwrap_or("en")
        .to_string();
    let segments = std::mem::take(&mut *found.lock().unwrap());
    Ok((segments, detected))
}

/// Picks a general model from what Ollama has; None when Ollama does not answer.
fn ollama_model(client: &reqwest::blocking::Client) -> Option<String> {
    #[derive(Deserialize)]
    struct Tags {
        models: Vec<Tag>,
    }
    #[derive(Deserialize)]
    struct Tag {
        name: String,
    }
    let tags: Tags = client
        .get(format!("{OLLAMA}/api/tags"))
        .send()
        .ok()?
        .json()
        .ok()?;
    let names: Vec<String> = tags
        .models
        .into_iter()
        .map(|t| t.name)
        .filter(|n| !n.contains("coder") && !n.contains("embed"))
        .collect();
    // Smaller general models first: a film has over a thousand lines.
    for prefix in ["gemma", "qwen3", "llama", "mistral", "qwen"] {
        if let Some(n) = names.iter().find(|n| n.starts_with(prefix)) {
            return Some(n.clone());
        }
    }
    names.into_iter().next()
}

/// `3: Hallo` lines back into texts by number; lines the model skipped stay untranslated.
pub fn parse_numbered(reply: &str, originals: &[String]) -> Vec<String> {
    let mut out = originals.to_vec();
    for line in reply.lines() {
        let Some((n, text)) = line.trim().split_once(':') else {
            continue;
        };
        if let Ok(i) = n.trim().trim_start_matches('#').parse::<usize>() {
            if (1..=out.len()).contains(&i) && !text.trim().is_empty() {
                out[i - 1] = text.trim().to_string();
            }
        }
    }
    out
}

fn translate(
    segments: &[Segment],
    from: &str,
    to: &str,
    cancel: &AtomicBool,
    emit: &dyn Fn(Progress),
) -> Result<Option<Vec<Segment>>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()?;
    let Some(model) = ollama_model(&client) else {
        return Ok(None);
    };
    let target = if to == "de" { "Deutsche" } else { "Englische" };
    let mut out = Vec::with_capacity(segments.len());
    for (batch_no, batch) in segments.chunks(30).enumerate() {
        if cancel.load(Ordering::Relaxed) {
            bail!("cancelled");
        }
        let originals: Vec<String> = batch.iter().map(|s| s.text.replace('\n', " ")).collect();
        let numbered: String = originals
            .iter()
            .enumerate()
            .map(|(i, t)| format!("{}: {}\n", i + 1, t))
            .collect();
        let prompt = format!(
            "Übersetze diese nummerierten Untertitelzeilen (Sprache: {from}) ins {target}. Natürlich und knapp, wie Kino-Untertitel. \
             Antworte nur mit denselben Nummern im Format `Nummer: Text`, eine Zeile pro Nummer, ohne Erklärungen.\n\n{numbered}"
        );
        let body = serde_json::json!({
            "model": model,
            "stream": false,
            "think": false,
            "options": { "temperature": 0.2 },
            "messages": [{ "role": "user", "content": prompt }],
        });
        let reply: serde_json::Value = client
            .post(format!("{OLLAMA}/api/chat"))
            .json(&body)
            .send()?
            .error_for_status()?
            .json()?;
        let text = reply["message"]["content"].as_str().unwrap_or_default();
        for (seg, line) in batch.iter().zip(parse_numbered(text, &originals)) {
            out.push(Segment {
                text: line,
                ..seg.clone()
            });
        }
        emit(Progress::Translating {
            percent: ((batch_no + 1) * 30).min(segments.len()) as f32 / segments.len() as f32
                * 100.0,
        });
    }
    Ok(Some(out))
}

impl Jobs {
    fn index_file(base: &Path) -> PathBuf {
        data_dir(base).join("index.json")
    }

    fn with_index<T>(&self, base: &Path, f: impl FnOnce(&mut Vec<Transcript>) -> T) -> T {
        let mut guard = self.index.lock().unwrap();
        let index = guard.get_or_insert_with(|| {
            std::fs::read_to_string(Self::index_file(base))
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        });
        f(index)
    }

    fn save(&self, base: &Path, t: Transcript) {
        self.with_index(base, |index| {
            index.retain(|x| x.path != t.path);
            index.push(t);
            if let Ok(text) = serde_json::to_string(index) {
                let _ = std::fs::write(Self::index_file(base), text);
            }
        });
    }

    pub fn find(&self, base: &Path, path: &str) -> Option<Transcript> {
        self.with_index(base, |index| {
            index
                .iter()
                .find(|t| t.path == path && t.srt.exists())
                .cloned()
        })
    }

    /// Case-insensitive search through every transcript; at most `limit` hits.
    pub fn search(&self, base: &Path, query: &str, limit: usize) -> Vec<Hit> {
        let q = query.trim().to_lowercase();
        if q.len() < 2 {
            return Vec::new();
        }
        self.with_index(base, |index| {
            let mut hits = Vec::new();
            for t in index.iter() {
                for s in &t.segments {
                    if s.text.to_lowercase().contains(&q) {
                        hits.push(Hit {
                            path: t.path.clone(),
                            time: s.start,
                            text: s.text.clone(),
                        });
                        if hits.len() >= limit {
                            return hits;
                        }
                    }
                }
            }
            hits
        })
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Writes subtitles for `media`, reporting through `emit`. Blocks; run it off the main thread.
    pub fn run(
        &self,
        base: &Path,
        media: &Path,
        duration: f64,
        ui_lang: &str,
        emit: impl Fn(Progress),
    ) {
        if self.busy.swap(true, Ordering::AcqRel) {
            emit(Progress::Failed {
                message: "busy".into(),
            });
            return;
        }
        self.cancel.store(false, Ordering::Relaxed);
        let result = self.transcribe(base, media, duration, ui_lang, &emit);
        self.busy.store(false, Ordering::Release);
        match result {
            Ok(t) => emit(Progress::Done {
                srt: t.srt,
                lang: t.lang,
                translated: t.translated,
            }),
            Err(e) => emit(Progress::Failed {
                message: format!("{e:#}"),
            }),
        }
    }

    fn transcribe(
        &self,
        base: &Path,
        media: &Path,
        duration: f64,
        ui_lang: &str,
        emit: &dyn Fn(Progress),
    ) -> Result<Transcript> {
        if duration <= 0.0 {
            bail!("unknown duration");
        }
        let model = download_model(&base.join("whisper"), &self.cancel, emit)?;
        let ctx = WhisperContext::new_with_params(
            model.to_string_lossy().as_ref(),
            WhisperContextParameters::default(),
        )
        .map_err(|e| anyhow!("whisper model: {e:?}"))?;
        let dir = data_dir(base);
        std::fs::create_dir_all(&dir)?;
        let path_text = media.to_string_lossy().into_owned();
        let srt = dir.join(format!("{}.srt", key(&path_text)));
        let mut segments: Vec<Segment> = Vec::new();
        let mut lang: Option<String> = None;
        let mut start = 0.0;
        while start < duration {
            if self.cancel.load(Ordering::Relaxed) {
                bail!("cancelled");
            }
            let length = CHUNK.min(duration - start);
            let samples =
                crate::pcm::mono(media, start, length, RATE).context("reading the audio")?;
            if samples.len() > RATE {
                let (found, detected) = listen(&ctx, &samples, lang.as_deref())?;
                lang.get_or_insert(detected);
                segments.extend(found.into_iter().map(|s| Segment {
                    start: s.start + start,
                    end: s.end + start,
                    ..s
                }));
                std::fs::write(&srt, to_srt(&segments))?;
            }
            start += CHUNK;
            emit(Progress::Listening {
                percent: (start.min(duration) / duration * 100.0) as f32,
                srt: (!segments.is_empty()).then(|| srt.clone()),
                lang: lang.clone().unwrap_or_default(),
            });
        }
        let lang = lang.unwrap_or_else(|| "en".into());
        let mut translated = None;
        if lang != ui_lang && !segments.is_empty() {
            match translate(&segments, &lang, ui_lang, &self.cancel, emit) {
                Ok(Some(t)) => {
                    let file = dir.join(format!("{}.{ui_lang}.srt", key(&path_text)));
                    std::fs::write(&file, to_srt(&t))?;
                    translated = Some(file);
                }
                Ok(None) => {}
                Err(e) => eprintln!("translation skipped: {e:#}"),
            }
        }
        let transcript = Transcript {
            path: path_text,
            lang,
            srt,
            translated,
            segments,
        };
        self.save(base, transcript.clone());
        Ok(transcript)
    }
}

#[derive(Clone, Serialize)]
pub struct Hit {
    pub path: String,
    pub time: f64,
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srt_has_numbered_cues_with_millis() {
        let s = to_srt(&[Segment {
            start: 3723.4567,
            end: 3725.0,
            text: " Hallo ".into(),
        }]);
        assert_eq!(s, "1\n01:02:03,457 --> 01:02:05,000\nHallo\n\n");
    }

    #[test]
    fn numbered_reply_maps_back_and_keeps_gaps() {
        let originals = vec!["Hello".to_string(), "World".to_string(), "Bye".to_string()];
        let out = parse_numbered("1: Hallo\nnoise\n3: Tschüss: bis bald\n", &originals);
        assert_eq!(out, vec!["Hallo", "World", "Tschüss: bis bald"]);
    }

    /// Needs the model in ~/Library/Application Support/de.wolfganglinz.flimmer/whisper and
    /// testmedia/speech.m4a: run with --ignored. Downloads the model on first run.
    #[test]
    #[ignore]
    fn transcribes_real_speech() {
        let base = dirs_base();
        let model =
            download_model(&base.join("whisper"), &AtomicBool::new(false), &|_| {}).unwrap();
        let ctx = WhisperContext::new_with_params(
            model.to_str().unwrap(),
            WhisperContextParameters::default(),
        )
        .unwrap();
        let media = Path::new(env!("CARGO_MANIFEST_DIR")).join("../testmedia/speech.m4a");
        let samples = crate::pcm::mono(&media, 0.0, 60.0, RATE).unwrap();
        let (segments, lang) = listen(&ctx, &samples, None).unwrap();
        eprintln!("{lang}: {segments:?}");
        assert!(!segments.is_empty());
    }

    fn dirs_base() -> PathBuf {
        PathBuf::from(std::env::var("HOME").unwrap())
            .join("Library/Application Support/de.wolfganglinz.flimmer")
    }
}

#[cfg(test)]
mod ollama {
    use super::*;

    /// Needs a local Ollama: run with --ignored.
    #[test]
    #[ignore]
    fn translates_through_local_ollama() {
        let segments: Vec<Segment> = [
            "Good evening and welcome to the lake.",
            "The sun sets behind the mountains.",
            "See you tomorrow!",
        ]
        .iter()
        .enumerate()
        .map(|(i, t)| Segment {
            start: i as f64 * 3.0,
            end: i as f64 * 3.0 + 2.5,
            text: t.to_string(),
        })
        .collect();
        let started = std::time::Instant::now();
        let out = translate(&segments, "en", "de", &AtomicBool::new(false), &|_| {})
            .unwrap()
            .expect("ollama answers");
        eprintln!(
            "{:?} in {:?}",
            out.iter().map(|s| &s.text).collect::<Vec<_>>(),
            started.elapsed()
        );
        assert_eq!(out.len(), 3);
        assert_ne!(out[0].text, segments[0].text);
    }
}
