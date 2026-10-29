//! The slime's ears and voice. Speech recognition (Whisper) and speech synthesis (Kokoro) run on this computer through
//! sherpa-onnx; nothing is sent anywhere. The models are files the user downloads once into the data folder.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sherpa_onnx::{
    GenerationConfig, OfflineModelConfig, OfflineRecognizer, OfflineRecognizerConfig, OfflineTts,
    OfflineTtsConfig, OfflineTtsKokoroModelConfig, OfflineTtsModelConfig,
    OfflineWhisperModelConfig,
};

/// The compact model (126 MB). Processors without int8 instructions run the full model faster.
pub const KOKORO_DIR: &str = "kokoro-int8-multi-lang-v1_0";
/// The full model (334 MB), preferred when it is installed.
pub const KOKORO_FULL_DIR: &str = "kokoro-multi-lang-v1_0";
/// What the recognizer works on.
pub const LISTEN_RATE: u32 = 16_000;

pub fn voice_dir(data: &Path) -> PathBuf {
    data.join("voice")
}

/// Which models are installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Installed {
    pub speech: bool,
    /// Some Whisper model is installed, so the slime can listen.
    pub hearing: bool,
    pub hearing_base: bool,
    pub hearing_small: bool,
}

fn has_hearing(data: &Path, model: slimy_voice::Hearing) -> bool {
    let p = model.prefix();
    ["encoder", "decoder"]
        .iter()
        .map(|k| format!("{p}-{k}.int8.onnx"))
        .chain([format!("{p}-tokens.txt")])
        .all(|f| voice_dir(data).join(model.dir()).join(f).is_file())
}

/// The Whisper model to use: the chosen one when installed, else any installed one.
fn hearing_model(data: &Path) -> Option<slimy_voice::Hearing> {
    use slimy_voice::Hearing::{Base, Small};
    let chosen = load_config(data).hearing;
    [chosen, Base, Small]
        .into_iter()
        .find(|m| has_hearing(data, *m))
}

pub fn installed(data: &Path) -> Installed {
    let (base, small) = (
        has_hearing(data, slimy_voice::Hearing::Base),
        has_hearing(data, slimy_voice::Hearing::Small),
    );
    Installed {
        speech: speech_model(data).is_some(),
        hearing: base || small,
        hearing_base: base,
        hearing_small: small,
    }
}

/// The speech model folder and the model file in it: the full model when installed, else the compact one.
pub fn speech_model(data: &Path) -> Option<(PathBuf, &'static str)> {
    let v = voice_dir(data);
    let complete = |dir: &str, file: &str| {
        let d = v.join(dir);
        ["voices.bin", "tokens.txt", "lexicon-us-en.txt"]
            .iter()
            .all(|f| d.join(f).is_file())
            && d.join(file).is_file()
            && d.join("espeak-ng-data").is_dir()
    };
    if complete(KOKORO_FULL_DIR, "model.onnx") {
        Some((v.join(KOKORO_FULL_DIR), "model.onnx"))
    } else if complete(KOKORO_DIR, "model.int8.onnx") {
        Some((v.join(KOKORO_DIR), "model.int8.onnx"))
    } else {
        None
    }
}

struct Loaded<T> {
    at: PathBuf,
    engine: T,
}

static SPEAKER: Mutex<Option<Loaded<OfflineTts>>> = Mutex::new(None);
static LISTENER: Mutex<Option<Loaded<(OfflineRecognizer, String)>>> = Mutex::new(None);

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn path(dir: &Path, name: &str) -> Option<String> {
    Some(dir.join(name).to_string_lossy().into_owned())
}

/// Threads for synthesis: most of the cores, leaving a couple for the rest of the computer.
fn threads() -> i32 {
    if let Some(n) = std::env::var("SLIMY_VOICE_THREADS")
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
    {
        return n.clamp(1, 16);
    }
    std::thread::available_parallelism().map_or(4, |n| (n.get() as i32 - 2).clamp(2, 8))
}

fn build_tts(dir: &Path, model: &str) -> Result<OfflineTts, String> {
    let kokoro = OfflineTtsKokoroModelConfig {
        model: path(dir, model),
        voices: path(dir, "voices.bin"),
        tokens: path(dir, "tokens.txt"),
        data_dir: path(dir, "espeak-ng-data"),
        dict_dir: path(dir, "dict"),
        lexicon: Some(format!(
            "{},{}",
            dir.join("lexicon-us-en.txt").display(),
            dir.join("lexicon-zh.txt").display()
        )),
        ..Default::default()
    };
    let config = OfflineTtsConfig {
        model: OfflineTtsModelConfig {
            kokoro,
            num_threads: threads(),
            ..Default::default()
        },
        max_num_sentences: 1,
        ..Default::default()
    };
    OfflineTts::create(&config).ok_or_else(|| "voice_model".to_string())
}

fn build_recognizer(dir: &Path, prefix: &str, language: &str) -> Result<OfflineRecognizer, String> {
    let model = OfflineModelConfig {
        whisper: OfflineWhisperModelConfig {
            encoder: path(dir, &format!("{prefix}-encoder.int8.onnx")),
            decoder: path(dir, &format!("{prefix}-decoder.int8.onnx")),
            language: Some(language.to_string()),
            task: Some("transcribe".into()),
            tail_paddings: -1,
            ..Default::default()
        },
        tokens: path(dir, &format!("{prefix}-tokens.txt")),
        num_threads: 4,
        ..Default::default()
    };
    let config = OfflineRecognizerConfig {
        model_config: model,
        ..Default::default()
    };
    OfflineRecognizer::create(&config).ok_or_else(|| "voice_model".to_string())
}

/// Splits text into clauses at punctuation, each with the mark that ended it ("" for the last one).
#[cfg(any(feature = "french-voice", test))]
fn clauses(text: &str) -> Vec<(String, Option<char>)> {
    let mut out = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        if matches!(c, '.' | ',' | '!' | '?' | ';' | ':') {
            out.push((std::mem::take(&mut current), Some(c)));
        } else {
            current.push(c);
        }
    }
    if !current.trim().is_empty() {
        out.push((current, None));
    }
    out
}

/// French has no lexicon in the model: its words become phonemes (IPA, with espeak-ng) and the model reads those. The
/// library drops punctuation and glues words around it, so each clause is phonemized alone and its mark put back, which
/// is also what gives the voice its pauses and intonation. One list of Unicode codepoints per sentence.
#[cfg(feature = "french-voice")]
fn french_phonemes(model: &Path, text: &str) -> Result<Vec<Vec<i32>>, String> {
    // espeak-ng looks for its data in the folder this variable names: the model brings it along.
    std::env::set_var("PIPER_ESPEAKNG_DATA_DIRECTORY", model);
    let mut sentences: Vec<String> = Vec::new();
    let mut current = String::new();
    for (clause, mark) in clauses(text) {
        let words = clause.trim();
        if !words.is_empty() {
            let phonemes = espeak_rs::text_to_phonemes(words, "fr", None)
                .map_err(|_| "voice_failed".to_string())?;
            if !current.is_empty() && !current.ends_with(' ') {
                current.push(' ');
            }
            current.push_str(&phonemes.join(" ").replace('-', ""));
        }
        if let Some(mark) = mark {
            current.push(mark);
            if matches!(mark, '.' | '!' | '?') {
                sentences.push(std::mem::take(&mut current));
            }
        }
    }
    if !current.trim().is_empty() {
        sentences.push(current);
    }
    if std::env::var_os("SLIMY_DEBUG_RAW").is_some() {
        eprintln!("phonemes: {sentences:?}");
    }
    let out: Vec<Vec<i32>> = sentences
        .iter()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.chars().map(|c| c as i32).collect())
        .collect();
    if out.is_empty() {
        Err("voice_failed".into())
    } else {
        Ok(out)
    }
}

/// Without the `french-voice` feature (no GPL code in the build) French cannot be phonemized: refuse with a short code.
#[cfg(not(feature = "french-voice"))]
fn french_phonemes(_model: &Path, _text: &str) -> Result<Vec<Vec<i32>>, String> {
    Err("voice_french_missing".into())
}

/// Whether this build can speak French.
pub const FRENCH: bool = cfg!(feature = "french-voice");

/// Speaks `text` with the voice: returns the samples and the rate to play them at (the pitch is the rate raised).
pub fn synthesize(
    data: &Path,
    text: &str,
    language: &str,
    voice: &slimy_voice::Voice,
) -> Result<(Vec<f32>, u32), String> {
    let (dir, model) = speech_model(data).ok_or_else(|| "voice_missing".to_string())?;
    let mut slot = lock(&SPEAKER);
    if slot.as_ref().is_none_or(|l| l.at != dir) {
        *slot = Some(Loaded {
            at: dir.clone(),
            engine: build_tts(&dir, model)?,
        });
    }
    let tts = &slot.as_ref().expect("loaded above").engine;
    // Playing at a higher rate raises the pitch and also speeds the voice up: ask the model for the opposite speed so the
    // tempo stays natural.
    let mut config = GenerationConfig {
        sid: voice.speaker,
        speed: (voice.speed / voice.pitch).clamp(0.6, 1.5),
        ..Default::default()
    };
    let phonemes_at = Instant::now();
    if language.starts_with("fr") {
        config.phoneme_codepoints = Some(french_phonemes(&dir, text)?);
    }
    let phonemes_took = phonemes_at.elapsed();
    let model_at = Instant::now();
    let audio = tts
        .generate_with_config::<fn(&[f32], f32) -> bool>(text, &config, None)
        .ok_or_else(|| "voice_failed".to_string())?;
    if std::env::var_os("SLIMY_DEBUG_RAW").is_some() {
        eprintln!(
            "synthesis: phonemes {:.2} s, model {:.2} s for {:.2} s of audio",
            phonemes_took.as_secs_f32(),
            model_at.elapsed().as_secs_f32(),
            audio.samples().len() as f32 / audio.sample_rate() as f32
        );
    }
    let rate = (audio.sample_rate() as f32 * voice.pitch).round() as u32;
    Ok((audio.samples().to_vec(), rate))
}

/// Turns what was said (16 kHz mono) into text.
pub fn transcribe(data: &Path, samples: &[f32], language: &str) -> Result<String, String> {
    let model = hearing_model(data).ok_or_else(|| "voice_missing".to_string())?;
    let dir = voice_dir(data).join(model.dir());
    let language = if language.starts_with("fr") {
        "fr"
    } else {
        "en"
    };
    let mut slot = lock(&LISTENER);
    if slot
        .as_ref()
        .is_none_or(|l| l.at != dir || l.engine.1 != language)
    {
        *slot = Some(Loaded {
            at: dir.clone(),
            engine: (
                build_recognizer(&dir, model.prefix(), language)?,
                language.to_string(),
            ),
        });
    }
    let recognizer = &slot.as_ref().expect("loaded above").engine.0;
    // Whisper drops the first and last words of a clip that starts or stops abruptly: a little silence around it fixes that.
    let silence = vec![0.0f32; (LISTEN_RATE as usize * 4) / 10];
    let padded = [silence.as_slice(), samples, silence.as_slice()].concat();
    let stream = recognizer.create_stream();
    stream.accept_waveform(LISTEN_RATE as i32, &padded);
    recognizer.decode(&stream);
    Ok(stream
        .get_result()
        .map(|r| r.text.trim().to_string())
        .unwrap_or_default())
}

/// Records from the default microphone until the person has spoken and paused (or `cancel` is set). `on_level` is called
/// about 25 times a second with the loudness. Returns 16 kHz mono; empty when nobody spoke.
pub fn record(cancel: &AtomicBool, mut on_level: impl FnMut(f32)) -> Result<Vec<f32>, String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::SampleFormat;
    let device = cpal::default_host()
        .default_input_device()
        .ok_or_else(|| "voice_no_mic".to_string())?;
    let config = device
        .default_input_config()
        .map_err(|_| "voice_no_mic".to_string())?;
    let rate = config.sample_rate().0;
    let channels = usize::from(config.channels()).max(1);
    let shared = Arc::new(Mutex::new(Vec::<f32>::new()));
    let sink = Arc::clone(&shared);
    let push = move |frames: &mut dyn Iterator<Item = f32>| {
        let mut mixed = Vec::new();
        let mut frame = Vec::with_capacity(channels);
        for s in frames {
            frame.push(s);
            if frame.len() == channels {
                mixed.push(frame.iter().sum::<f32>() / channels as f32);
                frame.clear();
            }
        }
        lock(&sink).extend(mixed);
    };
    let failed = |_| {};
    let stream_config: cpal::StreamConfig = config.clone().into();
    let stream = match config.sample_format() {
        SampleFormat::F32 => {
            let p = push.clone();
            device.build_input_stream(
                &stream_config,
                move |d: &[f32], _| p(&mut d.iter().copied()),
                failed,
                None,
            )
        }
        SampleFormat::I16 => {
            let p = push.clone();
            device.build_input_stream(
                &stream_config,
                move |d: &[i16], _| p(&mut d.iter().map(|s| f32::from(*s) / 32768.0)),
                failed,
                None,
            )
        }
        SampleFormat::U16 => {
            let p = push.clone();
            device.build_input_stream(
                &stream_config,
                move |d: &[u16], _| p(&mut d.iter().map(|s| (f32::from(*s) - 32768.0) / 32768.0)),
                failed,
                None,
            )
        }
        _ => return Err("voice_no_mic".into()),
    }
    .map_err(|_| "voice_no_mic".to_string())?;
    stream.play().map_err(|_| "voice_no_mic".to_string())?;

    let mut gate = slimy_voice::Endpointer::new(rate);
    let mut blocker = slimy_voice::DcBlocker::new();
    let mut all: Vec<f32> = Vec::new();
    loop {
        std::thread::sleep(Duration::from_millis(40));
        let mut fresh: Vec<f32> = std::mem::take(&mut *lock(&shared));
        blocker.process(&mut fresh);
        on_level(slimy_voice::level(&fresh));
        all.extend_from_slice(&fresh);
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        match gate.feed(&fresh) {
            slimy_voice::Heard::Finished | slimy_voice::Heard::TooLong => break,
            slimy_voice::Heard::GaveUp => return Ok(Vec::new()),
            _ => {}
        }
    }
    drop(stream);
    // Whisper does badly on a clip that opens with a long silence: keep only a little before the first word.
    let from = gate
        .speech_began()
        .map_or(0, |at| ((at - 0.4).max(0.0) * rate as f32) as usize)
        .min(all.len());
    let mut speech = to_listen_rate(&all[from..], rate);
    slimy_voice::normalize(&mut speech);
    Ok(speech)
}

/// Plays the samples through the default output, calling `on_level` about 25 times a second with the loudness of what is
/// sounding, until the end or until `cancel` is set.
pub fn play(
    samples: Vec<f32>,
    rate: u32,
    cancel: &AtomicBool,
    mut on_level: impl FnMut(f32),
) -> Result<(), String> {
    let (_stream, handle) =
        rodio::OutputStream::try_default().map_err(|_| "voice_no_speaker".to_string())?;
    let sink = rodio::Sink::try_new(&handle).map_err(|_| "voice_no_speaker".to_string())?;
    let window = (rate as usize / 25).max(1);
    let started = Instant::now();
    sink.append(rodio::buffer::SamplesBuffer::new(1, rate, samples.clone()));
    while !sink.empty() {
        if cancel.load(Ordering::Relaxed) {
            sink.stop();
            break;
        }
        let at = (started.elapsed().as_secs_f32() * rate as f32) as usize;
        let from = at.min(samples.len());
        on_level(slimy_voice::level(
            &samples[from..(from + window).min(samples.len())],
        ));
        std::thread::sleep(Duration::from_millis(40));
    }
    on_level(0.0);
    Ok(())
}

/// A pause after a sentence, in seconds.
const BREATH: f32 = 0.18;

/// Says `text` sentence by sentence: the first sentence starts playing while the next ones are still being synthesized, so
/// the slime answers quickly and without a long wait. Each clip is trimmed, faded and levelled. `on_level` follows the
/// loudness for the mouth; `on_first` is told how long it took to start (for measuring). Stops early if `cancel` is set.
pub fn speak(
    data: &Path,
    text: &str,
    language: &str,
    voice: &slimy_voice::Voice,
    cancel: &AtomicBool,
    mut on_level: impl FnMut(f32),
    mut on_first: impl FnMut(Duration),
) -> Result<(), String> {
    use std::sync::mpsc;
    let sentences = slimy_voice::sentences(text, 24);
    if sentences.is_empty() {
        return Ok(());
    }
    let begun = Instant::now();
    let (send, receive) = mpsc::channel::<Result<(Vec<f32>, u32), String>>();
    std::thread::scope(|scope| -> Result<(), String> {
        scope.spawn(move || {
            for sentence in &sentences {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let made = synthesize(data, sentence, language, voice).map(|(samples, rate)| {
                    // `synthesize` returns the pitched rate; the clip is finished at the model's own rate.
                    let model_rate = (rate as f32 / voice.pitch).round() as u32;
                    let mut clip = slimy_voice::finish(&samples, model_rate);
                    clip.extend(slimy_voice::silence(model_rate, BREATH));
                    (clip, rate)
                });
                let failed = made.is_err();
                if send.send(made).is_err() || failed {
                    break;
                }
            }
        });
        let (_stream, handle) =
            rodio::OutputStream::try_default().map_err(|_| "voice_no_speaker".to_string())?;
        let sink = rodio::Sink::try_new(&handle).map_err(|_| "voice_no_speaker".to_string())?;
        // (when it starts sounding, its samples, its rate): what the mouth follows
        let mut segments: Vec<(Instant, std::sync::Arc<Vec<f32>>, u32)> = Vec::new();
        let mut first = true;
        let mut done = false;
        loop {
            if cancel.load(Ordering::Relaxed) {
                sink.stop();
                break;
            }
            match receive.recv_timeout(Duration::from_millis(30)) {
                Ok(Ok((clip, rate))) => {
                    let start = match segments.last() {
                        Some((at, samples, r)) if !sink.empty() => {
                            *at + Duration::from_secs_f32(samples.len() as f32 / *r as f32)
                        }
                        _ => Instant::now(),
                    };
                    let shared = std::sync::Arc::new(clip);
                    sink.append(rodio::buffer::SamplesBuffer::new(
                        1,
                        rate,
                        shared.as_ref().clone(),
                    ));
                    segments.push((start, shared, rate));
                    if first {
                        first = false;
                        on_first(begun.elapsed());
                    }
                }
                Ok(Err(e)) => {
                    sink.stop();
                    return Err(e);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => done = true,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            let now = Instant::now();
            let level = segments.iter().rev().find(|(at, _, _)| *at <= now).map_or(
                0.0,
                |(at, samples, rate)| {
                    let from = (now.duration_since(*at).as_secs_f32() * *rate as f32) as usize;
                    let window = (*rate as usize / 25).max(1);
                    if from >= samples.len() {
                        0.0
                    } else {
                        slimy_voice::level(&samples[from..(from + window).min(samples.len())])
                    }
                },
            );
            on_level(level);
            if done && sink.empty() {
                break;
            }
        }
        on_level(0.0);
        Ok(())
    })
}

/// Settings the user can change; kept in `voice.json`.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// Say the slime's answers out loud.
    #[serde(default = "yes")]
    pub replies: bool,
    #[serde(default)]
    pub style: slimy_voice::Style,
    /// The Whisper model; unknown values mean the base model.
    #[serde(default)]
    pub hearing: slimy_voice::Hearing,
}

fn yes() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Config {
            replies: true,
            style: slimy_voice::Style::Auto,
            hearing: slimy_voice::Hearing::Base,
        }
    }
}

pub fn load_config(data: &Path) -> Config {
    std::fs::read_to_string(data.join("voice.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_config(data: &Path, config: &Config) -> std::io::Result<()> {
    std::fs::create_dir_all(data)?;
    let staging = data.join("voice.json.tmp");
    std::fs::write(
        &staging,
        serde_json::to_string_pretty(config).expect("a config serializes"),
    )?;
    std::fs::rename(staging, data.join("voice.json"))
}

/// The two downloads, from the official sherpa-onnx releases.
pub struct Download {
    pub name: &'static str,
    pub url: &'static str,
    pub megabytes: u64,
}

/// The full model: on processors without int8 instructions (most laptops) it is about three times faster than the compact one.
pub const SPEECH: Download = Download { name: "speech", url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/kokoro-multi-lang-v1_0.tar.bz2", megabytes: 334 };
pub const HEARING: Download = Download { name: "hearing", url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-whisper-base.tar.bz2", megabytes: 198 };
/// The larger Whisper model, optional, for better precision.
pub const HEARING_SMALL: Download = Download { name: "hearing_small", url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-whisper-small.tar.bz2", megabytes: 610 };

/// The most a download may be: a wrong file cannot fill the disk.
const MAX_DOWNLOAD: u64 = 800 * 1024 * 1024;

/// Unpacks a `.tar.bz2` under `into`. Entries that would land outside it are skipped by the tar reader.
pub fn unpack(archive: &Path, into: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive).map_err(|_| "voice_install".to_string())?;
    std::fs::create_dir_all(into).map_err(|_| "voice_install".to_string())?;
    let mut tar = tar::Archive::new(bzip2::read::BzDecoder::new(file));
    for entry in tar.entries().map_err(|_| "voice_install".to_string())? {
        let mut entry = entry.map_err(|_| "voice_install".to_string())?;
        // `unpack_in` refuses `..` and absolute paths.
        entry
            .unpack_in(into)
            .map_err(|_| "voice_install".to_string())?;
    }
    Ok(())
}

/// Downloads and unpacks one model. `progress(done, total)` is called as bytes arrive.
pub async fn install(
    data: &Path,
    what: &Download,
    mut progress: impl FnMut(u64, u64),
) -> Result<(), String> {
    use std::io::Write;
    let dir = voice_dir(data);
    let downloads = dir.join("dl");
    std::fs::create_dir_all(&downloads).map_err(|_| "voice_install".to_string())?;
    let target = downloads.join(format!("{}.tar.bz2", what.name));
    let part = downloads.join(format!("{}.part", what.name));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(1800))
        .build()
        .map_err(|_| "network".to_string())?;
    let mut response = client
        .get(what.url)
        .send()
        .await
        .map_err(|_| "network".to_string())?;
    if !response.status().is_success() {
        return Err("network".into());
    }
    let total = response
        .content_length()
        .unwrap_or(what.megabytes * 1024 * 1024);
    if total > MAX_DOWNLOAD {
        return Err("voice_install".into());
    }
    let mut file = std::fs::File::create(&part).map_err(|_| "voice_install".to_string())?;
    let mut done = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(|_| "network".to_string())? {
        done += chunk.len() as u64;
        if done > MAX_DOWNLOAD {
            let _ = std::fs::remove_file(&part);
            return Err("voice_install".into());
        }
        file.write_all(&chunk)
            .map_err(|_| "voice_install".to_string())?;
        progress(done, total);
    }
    drop(file);
    std::fs::rename(&part, &target).map_err(|_| "voice_install".to_string())?;
    let (archive, into) = (target.clone(), dir.clone());
    tauri::async_runtime::spawn_blocking(move || unpack(&archive, &into))
        .await
        .map_err(|_| "voice_install".to_string())??;
    let _ = std::fs::remove_file(&target);
    let now = installed(data);
    if (what.name == "speech" && now.speech)
        || (what.name == "hearing" && now.hearing_base)
        || (what.name == "hearing_small" && now.hearing_small)
    {
        Ok(())
    } else {
        Err("voice_install".into())
    }
}

/// A mono buffer at `from` Hz as 16 kHz, by linear interpolation: enough for speech recognition.
pub fn to_listen_rate(samples: &[f32], from: u32) -> Vec<f32> {
    if from == LISTEN_RATE || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = f64::from(from) / f64::from(LISTEN_RATE);
    let len = (samples.len() as f64 / ratio) as usize;
    (0..len)
        .map(|i| {
            let at = i as f64 * ratio;
            let a = at as usize;
            let frac = (at - a as f64) as f32;
            let b = (a + 1).min(samples.len() - 1);
            samples[a] * (1.0 - frac) + samples[b] * frac
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_installed_in_an_empty_folder() {
        assert_eq!(
            installed(Path::new("/nonexistent-slimy")),
            Installed {
                speech: false,
                hearing: false,
                hearing_base: false,
                hearing_small: false
            }
        );
    }

    #[test]
    fn text_is_cut_into_clauses_that_keep_their_mark() {
        assert_eq!(
            clauses("Salut, ça va ? Oui"),
            vec![
                ("Salut".into(), Some(',')),
                (" ça va ".into(), Some('?')),
                (" Oui".into(), None)
            ]
        );
        assert!(clauses("  ").is_empty());
        assert_eq!(clauses("Fin."), vec![("Fin".into(), Some('.'))]);
    }

    #[test]
    fn settings_round_trip_and_a_bad_file_gives_the_defaults() {
        let dir = std::env::temp_dir().join(format!("slimy-voice-cfg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(load_config(&dir), Config::default());
        let chosen = Config {
            replies: false,
            style: slimy_voice::Style::Kawaii,
            hearing: slimy_voice::Hearing::Small,
        };
        save_config(&dir, &chosen).unwrap();
        assert_eq!(load_config(&dir), chosen);
        std::fs::write(dir.join("voice.json"), "{ nope").unwrap();
        assert_eq!(load_config(&dir), Config::default());
        std::fs::write(dir.join("voice.json"), r#"{"style":"loud"}"#).unwrap();
        assert_eq!(
            load_config(&dir),
            Config::default(),
            "an unknown style is refused"
        );
        std::fs::write(dir.join("voice.json"), r#"{"hearing":"huge"}"#).unwrap();
        assert_eq!(
            load_config(&dir).hearing,
            slimy_voice::Hearing::Base,
            "an unknown model is the base model"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn archive(dir: &Path, entries: &[(&str, &[u8])]) -> PathBuf {
        let path = dir.join("test.tar.bz2");
        let mut builder = tar::Builder::new(bzip2::write::BzEncoder::new(
            std::fs::File::create(&path).unwrap(),
            bzip2::Compression::fast(),
        ));
        for (name, body) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            // `append_data` would refuse a bad path: write the name by hand to test the reader's guard.
            header.as_old_mut().name[..name.len()].copy_from_slice(name.as_bytes());
            header.set_cksum();
            builder.append(&header, *body).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap();
        path
    }

    #[test]
    fn unpacking_stays_inside_the_folder() {
        let base = std::env::temp_dir().join(format!("slimy-voice-tar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let file = archive(
            &base,
            &[("model/ok.txt", b"fine"), ("../escaped.txt", b"bad")],
        );
        let into = base.join("out");
        unpack(&file, &into).unwrap();
        assert_eq!(
            std::fs::read_to_string(into.join("model/ok.txt")).unwrap(),
            "fine"
        );
        assert!(
            !base.join("escaped.txt").exists(),
            "the path that climbs out was not written"
        );
        assert!(unpack(&base.join("missing.tar.bz2"), &into).is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_full_speech_model_is_preferred_and_the_compact_one_still_works() {
        let data = std::env::temp_dir().join(format!("slimy-speech-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data);
        let make = |dir: &str, model: &str| {
            let d = voice_dir(&data).join(dir);
            std::fs::create_dir_all(d.join("espeak-ng-data")).unwrap();
            for f in ["voices.bin", "tokens.txt", "lexicon-us-en.txt", model] {
                std::fs::write(d.join(f), b"x").unwrap();
            }
        };
        assert!(speech_model(&data).is_none());
        make(KOKORO_DIR, "model.int8.onnx");
        assert_eq!(speech_model(&data).unwrap().1, "model.int8.onnx");
        make(KOKORO_FULL_DIR, "model.onnx");
        assert_eq!(
            speech_model(&data).unwrap().1,
            "model.onnx",
            "the full model wins"
        );
        assert!(installed(&data).speech);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[test]
    fn resampling_keeps_the_duration() {
        let one_second: Vec<f32> = (0..48_000).map(|i| (i as f32 * 0.01).sin()).collect();
        let out = to_listen_rate(&one_second, 48_000);
        assert_eq!(out.len(), 16_000);
        assert_eq!(to_listen_rate(&one_second[..100], 16_000).len(), 100);
        assert!(to_listen_rate(&[], 44_100).is_empty());
    }
}
