use std::io::{BufWriter, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, RecvTimeoutError},
    Arc,
};
use std::thread;
use std::time::Duration;
use std::{fs, path::Path, process::Command};

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use tauri::{AppHandle, Emitter};
use tracing::{error, info, warn};
use uuid::Uuid;
#[cfg(target_os = "macos")]
use vosk::{CompleteResult, DecodingState, Model, PartialResult, Recognizer, SpeakerModel};

#[cfg(target_os = "macos")]
use crate::model::TranscriptToken;
use crate::model::{RecordingOptions, TranscriptSegment, TranscriptionEngine};
use crate::speaker;
use crate::state::AppState;

#[cfg(target_os = "macos")]
use crate::system_audio::{SystemAudioCapture, SystemAudioFrame};

const CHANNEL_TIMEOUT: Duration = Duration::from_millis(200);
const WHISPER_MIN_SILENCE_SECONDS: f32 = 0.8;
const WHISPER_MIN_SEGMENT_SECONDS: f32 = 0.5;
const WHISPER_DEFAULT_SILENCE_THRESHOLD: f32 = 0.008;

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

pub struct TranscriptionRuntime {
    stop: Arc<AtomicBool>,
    input_thread: Option<thread::JoinHandle<()>>,
    worker: Option<thread::JoinHandle<()>>,
    #[cfg(target_os = "macos")]
    system_capture: Option<SystemAudioCapture>,
    #[cfg(target_os = "macos")]
    system_thread: Option<thread::JoinHandle<()>>,
}

impl TranscriptionRuntime {
    pub fn stop(&mut self) -> Result<()> {
        self.stop.store(true, Ordering::SeqCst);

        if let Some(handle) = self.input_thread.take() {
            let _ = handle.join();
        }

        #[cfg(target_os = "macos")]
        {
            if let Some(mut capture) = self.system_capture.take() {
                capture.stop();
            }

            if let Some(handle) = self.system_thread.take() {
                let _ = handle.join();
            }
        }

        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
        Ok(())
    }
}

struct EventEmitter {
    app: AppHandle,
}

impl EventEmitter {
    fn new(app: AppHandle) -> Self {
        Self { app }
    }

    #[cfg(target_os = "macos")]
    fn emit_partial(&self, segment: &TranscriptSegment) {
        if let Err(error) = self.app.emit("transcription:partial", segment) {
            error!(?error, "failed to emit partial segment");
        }
    }

    fn emit_final(&self, segment: &TranscriptSegment) {
        if let Err(error) = self.app.emit("transcription:final", segment) {
            error!(?error, "failed to emit final segment");
        }
    }

    fn emit_error(&self, message: &str) {
        if let Err(error) = self.app.emit("transcription:error", message) {
            error!(?error, "failed to emit transcription error");
        }
    }
}

pub fn start_transcription(
    app: AppHandle,
    state: AppState,
    options: RecordingOptions,
) -> Result<TranscriptionRuntime> {
    if !options.enable_input && !options.enable_output {
        return Err(anyhow!("At least one audio source must be enabled"));
    }
    #[cfg(not(target_os = "macos"))]
    if !options.enable_input && options.enable_output {
        return Err(anyhow!(
            "System audio capture is only supported on macOS. Enable microphone input on this platform."
        ));
    }

    let (tx, rx) = mpsc::channel::<Vec<i16>>();
    let stop = Arc::new(AtomicBool::new(false));

    let mut runtime = TranscriptionRuntime {
        stop: stop.clone(),
        input_thread: None,
        worker: None,
        #[cfg(target_os = "macos")]
        system_capture: None,
        #[cfg(target_os = "macos")]
        system_thread: None,
    };

    let mut sample_rate: Option<f32> = None;

    if options.enable_input {
        let (input_thread, input_sample_rate) = start_input_capture(stop.clone(), tx.clone())?;
        sample_rate = Some(input_sample_rate);
        runtime.input_thread = Some(input_thread);
    } else {
        info!("Microphone capture disabled by recording options");
    }

    #[cfg(not(target_os = "macos"))]
    if options.enable_output {
        warn!("System audio capture is only supported on macOS 14.2 or later");
    }

    #[cfg(target_os = "macos")]
    {
        let mut system_capture = None;
        let mut system_thread = None;
        let mut system_audio_error = None;

        if options.enable_output {
            ensure_system_audio_capture_permission()?;

            match start_system_audio(stop.clone(), tx.clone(), sample_rate) {
                Ok((capture, handle, actual_rate)) => {
                    if sample_rate.is_none() {
                        sample_rate = Some(actual_rate);
                    }
                    system_capture = Some(capture);
                    system_thread = Some(handle);
                }
                Err(error) => {
                    warn!(?error, "Failed to start system audio capture");
                    system_audio_error = Some(error);
                }
            }
        }

        if let Some(error) = system_audio_error {
            return Err(anyhow!(
                "Capture Output is enabled, but system audio capture failed to start: {error:#}. \
                Check macOS settings: Privacy & Security > Screen & System Audio Recording."
            ));
        }

        runtime.system_capture = system_capture;
        runtime.system_thread = system_thread;
    }

    let sample_rate =
        sample_rate.ok_or_else(|| anyhow!("Unable to determine audio sample rate"))?;

    let worker_app = app.clone();
    let worker_state = state.clone();
    let worker_stop = stop.clone();
    let worker_options = options.clone();

    let worker = thread::spawn(move || {
        if let Err(error) = run_recognizer(
            worker_app,
            worker_state,
            worker_stop,
            rx,
            sample_rate,
            worker_options,
        ) {
            error!(?error, "Recognizer thread crashed");
        }
    });

    runtime.worker = Some(worker);

    Ok(runtime)
}

fn start_input_capture(
    stop: Arc<AtomicBool>,
    tx: mpsc::Sender<Vec<i16>>,
) -> Result<(thread::JoinHandle<()>, f32)> {
    let (ready_tx, ready_rx) = mpsc::channel::<Result<f32, String>>();

    let handle = thread::spawn(move || {
        let started = (|| -> Result<(cpal::Stream, f32)> {
            let host = cpal::default_host();
            let device = host
                .default_input_device()
                .context("No default input device found")?;

            let config = device.default_input_config()?;
            let input_sample_rate = config.sample_rate().0 as f32;
            let sample_format = config.sample_format();
            let stream_config: cpal::StreamConfig = config.clone().into();
            let channels = stream_config.channels as usize;

            if channels == 0 {
                return Err(anyhow!("input device reported zero channels"));
            }

            let err_fn = |err| error!(?err, "An error occurred on the input audio stream");

            let input_stream = match sample_format {
                SampleFormat::I16 => {
                    let tx = tx.clone();
                    device.build_input_stream(
                        &stream_config,
                        move |data: &[i16], _| transmit_chunk_i16(data, channels, &tx),
                        err_fn,
                        None,
                    )?
                }
                SampleFormat::U16 => {
                    let tx = tx.clone();
                    device.build_input_stream(
                        &stream_config,
                        move |data: &[u16], _| transmit_chunk_u16(data, channels, &tx),
                        err_fn,
                        None,
                    )?
                }
                SampleFormat::F32 => {
                    let tx = tx.clone();
                    device.build_input_stream(
                        &stream_config,
                        move |data: &[f32], _| transmit_chunk_f32(data, channels, &tx),
                        err_fn,
                        None,
                    )?
                }
                other => return Err(anyhow!("Unsupported sample format {other:?}")),
            };

            input_stream
                .play()
                .context("failed to start audio input stream")?;

            Ok((input_stream, input_sample_rate))
        })();

        match started {
            Ok((stream, sample_rate)) => {
                if ready_tx.send(Ok(sample_rate)).is_err() {
                    return;
                }

                while !stop.load(Ordering::Relaxed) {
                    thread::sleep(CHANNEL_TIMEOUT);
                }

                if let Err(error) = stream.pause() {
                    warn!(?error, "failed to pause input stream during shutdown");
                }
            }
            Err(error) => {
                let _ = ready_tx.send(Err(format!("{error:#}")));
            }
        }
    });

    let sample_rate = ready_rx
        .recv()
        .context("failed to initialize input capture thread")?
        .map_err(|message| anyhow!(message))?;

    Ok((handle, sample_rate))
}

fn transmit_chunk_i16(data: &[i16], channels: usize, tx: &mpsc::Sender<Vec<i16>>) {
    if data.is_empty() {
        return;
    }

    let mut converted = Vec::with_capacity(data.len() / channels + 1);
    for frame in data.chunks(channels) {
        let sum: i32 = frame.iter().map(|&sample| sample as i32).sum();
        let average = (sum / channels as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        converted.push(average);
    }

    if let Err(error) = tx.send(converted) {
        error!(?error, "failed to forward audio chunk to recognizer");
    }
}

fn transmit_chunk_u16(data: &[u16], channels: usize, tx: &mpsc::Sender<Vec<i16>>) {
    if data.is_empty() {
        return;
    }

    let mut converted = Vec::with_capacity(data.len() / channels + 1);
    for frame in data.chunks(channels) {
        let mut sum = 0i32;
        for &sample in frame {
            let centered = sample as i32 - 32768;
            sum += centered;
        }
        let average = (sum / channels as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        converted.push(average);
    }

    if let Err(error) = tx.send(converted) {
        error!(?error, "failed to forward audio chunk to recognizer");
    }
}

fn transmit_chunk_f32(data: &[f32], channels: usize, tx: &mpsc::Sender<Vec<i16>>) {
    if data.is_empty() {
        return;
    }

    let mut converted = Vec::with_capacity(data.len() / channels + 1);
    for frame in data.chunks(channels) {
        let mut sum = 0f32;
        for &sample in frame {
            sum += sample;
        }
        let clamped = (sum / channels as f32).clamp(-1.0, 1.0);
        let scaled = (clamped * i16::MAX as f32).round() as i16;
        converted.push(scaled);
    }

    if let Err(error) = tx.send(converted) {
        error!(?error, "failed to forward audio chunk to recognizer");
    }
}

#[cfg(target_os = "macos")]
fn start_system_audio(
    stop: Arc<AtomicBool>,
    tx: mpsc::Sender<Vec<i16>>,
    preferred_rate: Option<f32>,
) -> Result<(SystemAudioCapture, thread::JoinHandle<()>, f32)> {
    let (frame_tx, frame_rx) = mpsc::channel::<SystemAudioFrame>();
    let capture = SystemAudioCapture::start(preferred_rate.map(|rate| rate as f64), frame_tx)?;
    let actual_rate = capture.sample_rate() as f32;
    let target_rate = preferred_rate.unwrap_or(actual_rate);

    let forward_stop = stop.clone();
    let forward_tx = tx;
    let handle = thread::spawn(move || {
        forward_system_audio(frame_rx, forward_tx, forward_stop, target_rate);
    });

    Ok((capture, handle, actual_rate))
}

#[cfg(target_os = "macos")]
fn ensure_system_audio_capture_permission() -> Result<()> {
    let granted = unsafe {
        if CGPreflightScreenCaptureAccess() {
            true
        } else {
            CGRequestScreenCaptureAccess()
        }
    };

    if granted {
        Ok(())
    } else {
        let _ = open_system_audio_privacy_settings();
        Err(anyhow!(
            "System audio recording permission is denied. Allow MemoBreeze in \
            Privacy & Security > Screen & System Audio Recording, then retry."
        ))
    }
}

#[cfg(target_os = "macos")]
fn open_system_audio_privacy_settings() -> Result<()> {
    // Deep-link to the privacy pane used by screen/system audio capture permission.
    let status = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
        .status()
        .context("failed to open macOS privacy settings")?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("open command exited with status {status}"))
    }
}

#[cfg(target_os = "macos")]
fn forward_system_audio(
    rx: mpsc::Receiver<SystemAudioFrame>,
    tx: mpsc::Sender<Vec<i16>>,
    stop: Arc<AtomicBool>,
    target_rate: f32,
) {
    while !stop.load(Ordering::Relaxed) {
        match rx.recv_timeout(CHANNEL_TIMEOUT) {
            Ok(frame) => {
                if frame.data.is_empty() {
                    continue;
                }

                let source_rate = frame.sample_rate as f32;
                let data = frame.data;
                let samples = if (source_rate - target_rate).abs() > f32::EPSILON {
                    resample_buffer(&data, source_rate, target_rate)
                } else {
                    data
                };

                if samples.is_empty() {
                    continue;
                }

                let normalized = normalize_system_audio_level(&samples);
                let chunk = convert_f32_to_i16(&normalized);
                if let Err(error) = tx.send(chunk) {
                    error!(?error, "failed to forward system audio chunk");
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

#[cfg(target_os = "macos")]
fn resample_buffer(data: &[f32], source_rate: f32, target_rate: f32) -> Vec<f32> {
    if data.is_empty() {
        return Vec::new();
    }

    if (source_rate - target_rate).abs() <= f32::EPSILON {
        return data.to_vec();
    }

    let ratio = target_rate / source_rate;
    let output_len = ((data.len() as f32) * ratio).round().max(1.0) as usize;
    let mut output = Vec::with_capacity(output_len);
    let step = source_rate / target_rate;

    for index in 0..output_len {
        let position = index as f32 * step;
        let base = position.floor() as usize;
        let frac = position - base as f32;

        if base + 1 < data.len() {
            let a = data[base];
            let b = data[base + 1];
            output.push(a + (b - a) * frac);
        } else {
            output.push(*data.last().unwrap());
        }
    }

    output
}

#[cfg(target_os = "macos")]
fn convert_f32_to_i16(data: &[f32]) -> Vec<i16> {
    let mut converted = Vec::with_capacity(data.len());
    for &sample in data {
        let clamped = sample.clamp(-1.0, 1.0);
        converted.push((clamped * i16::MAX as f32).round() as i16);
    }
    converted
}

#[cfg(target_os = "macos")]
fn normalize_system_audio_level(data: &[f32]) -> Vec<f32> {
    if data.is_empty() {
        return Vec::new();
    }

    let energy = data.iter().map(|sample| sample * sample).sum::<f32>() / data.len() as f32;
    let rms = energy.sqrt();

    // System tap audio can be much quieter than microphone input.
    // Apply conservative AGC to keep recognition sensitivity stable.
    let gain = if rms > 0.0 && rms < 0.05 {
        (0.08 / rms).clamp(1.0, 6.0)
    } else {
        1.0
    };

    data.iter()
        .map(|sample| (sample * gain).clamp(-1.0, 1.0))
        .collect()
}

fn run_recognizer(
    app: AppHandle,
    state: AppState,
    stop: Arc<AtomicBool>,
    rx: mpsc::Receiver<Vec<i16>>,
    sample_rate: f32,
    options: RecordingOptions,
) -> Result<()> {
    match options.engine {
        TranscriptionEngine::Vosk => {
            run_vosk_recognizer(app, state, stop, rx, sample_rate, options)
        }
        TranscriptionEngine::Whisper => {
            run_whisper_recognizer(app, state, stop, rx, sample_rate, options)
        }
    }
}

#[cfg(target_os = "macos")]
fn run_vosk_recognizer(
    app: AppHandle,
    state: AppState,
    stop: Arc<AtomicBool>,
    rx: mpsc::Receiver<Vec<i16>>,
    sample_rate: f32,
    options: RecordingOptions,
) -> Result<()> {
    let model = Model::new(&options.model_path).context("Failed to load Vosk model")?;
    let mut recognizer =
        Recognizer::new(&model, sample_rate).ok_or_else(|| anyhow!("Failed to init recognizer"))?;
    recognizer.set_words(true);

    let speaker_model = if let Some(path) = options.speaker_model_path.as_deref() {
        Some(SpeakerModel::new(path).ok_or_else(|| anyhow!("Failed to load speaker model"))?)
    } else {
        None
    };

    if let Some(ref model) = speaker_model {
        recognizer.set_speaker_model(model);
    }

    let emitter = EventEmitter::new(app.clone());
    let mut last_end = 0.0_f32;
    let mut pending_id: Option<String> = None;

    while !stop.load(Ordering::Relaxed) {
        match rx.recv_timeout(CHANNEL_TIMEOUT) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    continue;
                }

                if is_silence(&chunk, options.energy_threshold) {
                    continue;
                }

                match recognizer.accept_waveform(&chunk) {
                    Ok(DecodingState::Finalized) => {
                        let result = recognizer.result();
                        if let Some((segment, embedding)) =
                            build_segment_from_result(result, &mut pending_id, &mut last_end)
                        {
                            let updated = state.push_final(segment.clone(), embedding);
                            speaker::update_labels(&app, &state);
                            emitter.emit_final(&updated);
                        }
                    }
                    Ok(DecodingState::Running) => {
                        let partial = recognizer.partial_result();
                        if let Some(partial_segment) =
                            build_partial_segment(partial, &mut pending_id, last_end)
                        {
                            let updated = state.push_partial(partial_segment.clone(), None);
                            emitter.emit_partial(&updated);
                        }
                    }
                    Ok(DecodingState::Failed) => {
                        emitter.emit_error("Recognizer failed while processing audio chunk");
                    }
                    Err(error) => {
                        emitter.emit_error(&format!("Recognizer input error: {error:?}"));
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    let final_result = recognizer.final_result();
    if let Some((segment, embedding)) =
        build_segment_from_result(final_result, &mut pending_id, &mut last_end)
    {
        let updated = state.push_final(segment.clone(), embedding);
        speaker::update_labels(&app, &state);
        emitter.emit_final(&updated);
    }

    info!("Recognizer stopped");
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn run_vosk_recognizer(
    app: AppHandle,
    state: AppState,
    stop: Arc<AtomicBool>,
    rx: mpsc::Receiver<Vec<i16>>,
    sample_rate: f32,
    options: RecordingOptions,
) -> Result<()> {
    let _ = (app, state, stop, rx, sample_rate, options);
    Err(anyhow!(
        "Vosk engine is currently supported on macOS only. Switch transcription engine to Whisper."
    ))
}

fn run_whisper_recognizer(
    app: AppHandle,
    state: AppState,
    stop: Arc<AtomicBool>,
    rx: mpsc::Receiver<Vec<i16>>,
    sample_rate: f32,
    options: RecordingOptions,
) -> Result<()> {
    if options.whisper_model_path.trim().is_empty() {
        return Err(anyhow!("Whisper model path must not be empty"));
    }

    let emitter = EventEmitter::new(app.clone());
    let mut timeline_seconds = 0.0_f32;
    let mut active_start_seconds = 0.0_f32;
    let mut active_buffer = Vec::<i16>::new();
    let mut active = false;
    let mut trailing_silence_samples = 0usize;
    let min_silence_samples = (sample_rate * WHISPER_MIN_SILENCE_SECONDS) as usize;
    let min_segment_samples = (sample_rate * WHISPER_MIN_SEGMENT_SECONDS) as usize;

    let flush_segment = |app: &AppHandle,
                         state: &AppState,
                         options: &RecordingOptions,
                         samples: &[i16],
                         start: f32,
                         end: f32|
     -> Result<()> {
        let text = transcribe_whisper_segment(samples, sample_rate, options)?;
        if text.trim().is_empty() {
            return Ok(());
        }

        let segment = TranscriptSegment {
            id: Uuid::new_v4().to_string(),
            speaker: "Unknown".to_string(),
            text,
            start,
            end,
            tokens: Vec::new(),
            is_final: true,
        };

        let updated = state.push_final(segment, None);
        speaker::update_labels(app, state);
        emitter.emit_final(&updated);
        Ok(())
    };

    while !stop.load(Ordering::Relaxed) {
        match rx.recv_timeout(CHANNEL_TIMEOUT) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    continue;
                }

                let chunk_duration = chunk.len() as f32 / sample_rate;
                let chunk_is_silence = is_silence_for_whisper(&chunk, options.energy_threshold);

                if !active {
                    if chunk_is_silence {
                        timeline_seconds += chunk_duration;
                        continue;
                    }
                    active = true;
                    active_start_seconds = timeline_seconds;
                    active_buffer.clear();
                    trailing_silence_samples = 0;
                }

                active_buffer.extend_from_slice(&chunk);

                if chunk_is_silence {
                    trailing_silence_samples += chunk.len();
                } else {
                    trailing_silence_samples = 0;
                }

                timeline_seconds += chunk_duration;

                if trailing_silence_samples >= min_silence_samples {
                    let speech_len = active_buffer.len().saturating_sub(trailing_silence_samples);
                    if speech_len >= min_segment_samples {
                        let segment_end =
                            timeline_seconds - trailing_silence_samples as f32 / sample_rate;
                        if let Err(error) = flush_segment(
                            &app,
                            &state,
                            &options,
                            &active_buffer[..speech_len],
                            active_start_seconds,
                            segment_end,
                        ) {
                            emitter.emit_error(&format!("Whisper transcription failed: {error}"));
                        }
                    }
                    active = false;
                    active_buffer.clear();
                    trailing_silence_samples = 0;
                }
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    if active && active_buffer.len() >= min_segment_samples {
        let speech_len = active_buffer.len().saturating_sub(trailing_silence_samples);
        if speech_len >= min_segment_samples {
            let segment_end = timeline_seconds - trailing_silence_samples as f32 / sample_rate;
            if let Err(error) = flush_segment(
                &app,
                &state,
                &options,
                &active_buffer[..speech_len],
                active_start_seconds,
                segment_end,
            ) {
                emitter.emit_error(&format!("Whisper transcription failed: {error}"));
            }
        }
    }

    info!("Whisper recognizer stopped");
    Ok(())
}

fn transcribe_whisper_segment(
    samples: &[i16],
    sample_rate: f32,
    options: &RecordingOptions,
) -> Result<String> {
    let tmp_dir = std::env::temp_dir();
    let base = format!("memobreeze-whisper-{}", Uuid::new_v4());
    let wav_path = tmp_dir.join(format!("{base}.wav"));
    let out_prefix = tmp_dir.join(&base);
    let out_txt = tmp_dir.join(format!("{base}.txt"));

    write_wav_mono_i16(&wav_path, samples, sample_rate)?;

    let mut cmd = Command::new(options.whisper_command.trim());
    cmd.arg("-ng")
        .arg("-m")
        .arg(&options.whisper_model_path)
        .arg("-f")
        .arg(&wav_path)
        .arg("-otxt")
        .arg("-of")
        .arg(&out_prefix);

    if let Some(lang) = options.whisper_language.as_deref() {
        if !lang.trim().is_empty() {
            cmd.arg("-l").arg(lang.trim());
        }
    }

    let output = cmd.output().with_context(|| {
        format!(
            "Failed to execute Whisper command '{}'",
            options.whisper_command
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = fs::remove_file(&wav_path);
        let _ = fs::remove_file(&out_txt);
        return Err(anyhow!(
            "Whisper command failed with status {}: {}",
            output.status,
            stderr.trim()
        ));
    }

    let text_result = fs::read_to_string(&out_txt)
        .with_context(|| format!("Failed to read Whisper output file: {}", out_txt.display()));
    let _ = fs::remove_file(&wav_path);
    let _ = fs::remove_file(&out_txt);
    let text = text_result?;

    Ok(normalize_whisper_text(&text))
}

fn write_wav_mono_i16(path: &Path, samples: &[i16], sample_rate: f32) -> Result<()> {
    let sample_rate = sample_rate.max(1.0).round() as u32;
    let channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let block_align: u16 = channels * (bits_per_sample / 8);
    let byte_rate: u32 = sample_rate * block_align as u32;
    let data_len: u32 = (samples.len() * 2) as u32;
    let riff_chunk_size: u32 = 36 + data_len;

    let file = fs::File::create(path)
        .with_context(|| format!("Failed to create wav file: {}", path.display()))?;
    let mut file = BufWriter::new(file);

    file.write_all(b"RIFF")
        .context("Failed to write wav RIFF header")?;
    file.write_all(&riff_chunk_size.to_le_bytes())
        .context("Failed to write wav chunk size")?;
    file.write_all(b"WAVE")
        .context("Failed to write wav WAVE header")?;
    file.write_all(b"fmt ")
        .context("Failed to write wav fmt header")?;
    file.write_all(&16u32.to_le_bytes())
        .context("Failed to write wav fmt chunk size")?;
    file.write_all(&1u16.to_le_bytes())
        .context("Failed to write wav audio format")?;
    file.write_all(&channels.to_le_bytes())
        .context("Failed to write wav channel count")?;
    file.write_all(&sample_rate.to_le_bytes())
        .context("Failed to write wav sample rate")?;
    file.write_all(&byte_rate.to_le_bytes())
        .context("Failed to write wav byte rate")?;
    file.write_all(&block_align.to_le_bytes())
        .context("Failed to write wav block align")?;
    file.write_all(&bits_per_sample.to_le_bytes())
        .context("Failed to write wav bits per sample")?;
    file.write_all(b"data")
        .context("Failed to write wav data header")?;
    file.write_all(&data_len.to_le_bytes())
        .context("Failed to write wav data length")?;

    let mut pcm_data = Vec::with_capacity(std::mem::size_of_val(samples));
    for &sample in samples {
        pcm_data.extend_from_slice(&sample.to_le_bytes());
    }
    file.write_all(&pcm_data)
        .context("Failed to write wav PCM samples")?;
    file.flush().context("Failed to flush wav writer")?;

    Ok(())
}

fn normalize_whisper_text(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(target_os = "macos")]
fn build_partial_segment(
    partial: PartialResult<'_>,
    pending_id: &mut Option<String>,
    anchor: f32,
) -> Option<TranscriptSegment> {
    let text = partial.partial.trim();
    if text.is_empty() {
        return None;
    }

    let id = pending_id
        .get_or_insert_with(|| Uuid::new_v4().to_string())
        .clone();

    let tokens = partial
        .partial_result
        .into_iter()
        .map(|word| TranscriptToken {
            text: word.word.to_string(),
            start: word.start,
            end: word.end,
            confidence: word.conf,
        })
        .collect();

    Some(TranscriptSegment {
        id,
        speaker: "Unknown".to_string(),
        text: text.to_string(),
        start: anchor,
        end: anchor,
        tokens,
        is_final: false,
    })
}

#[cfg(target_os = "macos")]
fn build_segment_from_result(
    result: CompleteResult<'_>,
    pending_id: &mut Option<String>,
    last_end: &mut f32,
) -> Option<(TranscriptSegment, Option<Vec<f32>>)> {
    match result {
        CompleteResult::Single(single) => {
            let text = single.text.trim();
            if text.is_empty() {
                *pending_id = None;
                return None;
            }

            let id = pending_id
                .take()
                .unwrap_or_else(|| Uuid::new_v4().to_string());

            let mut start = f32::MAX;
            let mut end = f32::MIN;
            let tokens: Vec<TranscriptToken> = single
                .result
                .into_iter()
                .map(|word| {
                    start = start.min(word.start);
                    end = end.max(word.end);
                    TranscriptToken {
                        text: word.word.to_string(),
                        start: word.start,
                        end: word.end,
                        confidence: word.conf,
                    }
                })
                .collect();

            if start == f32::MAX {
                start = *last_end;
            }
            if end == f32::MIN {
                end = (start + 0.5).max(*last_end);
            }

            *last_end = end;

            Some((
                TranscriptSegment {
                    id,
                    speaker: "Unknown".to_string(),
                    text: text.to_string(),
                    start,
                    end,
                    tokens,
                    is_final: true,
                },
                single.speaker_info.map(|info| info.vector),
            ))
        }
        CompleteResult::Multiple(multi) => {
            let alternative = multi.alternatives.first()?;
            let text = alternative.text.trim();
            if text.is_empty() {
                *pending_id = None;
                return None;
            }

            let id = pending_id
                .take()
                .unwrap_or_else(|| Uuid::new_v4().to_string());

            let mut start = f32::MAX;
            let mut end = f32::MIN;
            let tokens: Vec<TranscriptToken> = alternative
                .result
                .iter()
                .map(|word| {
                    start = start.min(word.start);
                    end = end.max(word.end);
                    TranscriptToken {
                        text: word.word.to_string(),
                        start: word.start,
                        end: word.end,
                        confidence: alternative.confidence,
                    }
                })
                .collect();

            if start == f32::MAX {
                start = *last_end;
            }
            if end == f32::MIN {
                end = (start + 0.5).max(*last_end);
            }

            *last_end = end;

            Some((
                TranscriptSegment {
                    id,
                    speaker: "Unknown".to_string(),
                    text: text.to_string(),
                    start,
                    end,
                    tokens,
                    is_final: true,
                },
                None,
            ))
        }
    }
}

fn is_silence(chunk: &[i16], threshold: f32) -> bool {
    if threshold <= 0.0 {
        return false;
    }

    let energy: f32 = chunk
        .iter()
        .map(|sample| sample.abs() as f32 / i16::MAX as f32)
        .sum::<f32>()
        / chunk.len() as f32;

    energy < threshold
}

fn is_silence_for_whisper(chunk: &[i16], threshold: f32) -> bool {
    // Whisper segmentation needs a practical default even when threshold is 0.
    let threshold = if threshold <= 0.0 {
        WHISPER_DEFAULT_SILENCE_THRESHOLD
    } else {
        threshold
    };
    is_silence(chunk, threshold)
}
