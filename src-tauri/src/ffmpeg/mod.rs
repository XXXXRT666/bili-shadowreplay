use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Stdio};
use std::sync::{Arc, OnceLock};

pub mod general;
pub mod hwaccel;
pub mod playlist;

use crate::constants;
use crate::progress::progress_reporter::{ProgressReporter, ProgressReporterTrait};
use crate::subtitle_generator::{powerlive, whisper_online};
use crate::subtitle_generator::{
    whisper_cpp, GenerateResult, SubtitleGenerator, SubtitleGeneratorType,
};
use async_ffmpeg_sidecar::event::FfmpegEvent;
use async_ffmpeg_sidecar::log_parser::FfmpegLogParser;
use futures::stream::{FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::io::BufReader;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::danmu2ass::DanmuImageOverlay;

// 视频元数据结构
#[derive(Debug, Clone, PartialEq)]
pub struct VideoMetadata {
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub video_codec: String,
    pub audio_codec: String,
}

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(target_os = "windows")]
#[allow(unused_imports)]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioWaveformData {
    pub peaks: Vec<f32>,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeekbarThumbnailCacheManifest {
    pub step_seconds: f64,
    pub duration: f64,
    pub frame_count: usize,
    pub width: u32,
    pub height: u32,
}

const SEEKBAR_THUMBNAIL_CACHE_DIR_EXT: &str = "seekbar-thumbs";
const SEEKBAR_THUMBNAIL_CACHE_MANIFEST_FILE: &str = "index.json";
const SEEKBAR_THUMBNAIL_CACHE_MIN_FRAME_COUNT: f64 = 240.0;
const SEEKBAR_THUMBNAIL_CACHE_DEFAULT_STEP_SECONDS: f64 = 5.0;
const SEEKBAR_THUMBNAIL_CACHE_WIDTH: u32 = 480;
const SEEKBAR_THUMBNAIL_CACHE_HEIGHT: u32 = 270;
const LOCAL_AUDIO_CHUNK_DURATION_SECONDS: u64 = 30;
const REALTIME_ASR_AUDIO_CHUNK_DURATION_SECONDS: u64 = 180;
const AUDIO_CHUNK_MAX_PARALLELISM: usize = 8;
const DANMU_IMAGE_OVERLAY_BATCH_SIZE: usize = 500;
const DANMU_IMAGE_OVERLAY_SINGLE_PASS_LIMIT: usize = 500;
const DANMU_LAYER_FPS: u32 = 60;
static FFMPEG_PATH_CACHE: OnceLock<PathBuf> = OnceLock::new();
static FFPROBE_PATH_CACHE: OnceLock<PathBuf> = OnceLock::new();

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.start, self.end)
    }
}

impl Range {
    pub fn duration(&self) -> f64 {
        self.end - self.start
    }

    pub fn is_in(&self, v: f64) -> bool {
        v >= self.start && v <= self.end
    }
}

pub async fn transcode(
    reporter: Option<&impl ProgressReporterTrait>,
    file: &Path,
    output_path: &Path,
    copy_codecs: bool,
) -> Result<(), String> {
    // ffmpeg -i fixed_\[30655190\]1742887114_0325084106_81.5.mp4 -c:v libx264 -c:a aac -b:v 6000k -b:a 64k -compression_level 0 -threads 0 output.mp3
    log::info!("Transcode: {} copy: {}", file.display(), copy_codecs);
    let mut ffmpeg_process = ffmpeg_command();
    #[cfg(target_os = "windows")]
    ffmpeg_process.creation_flags(CREATE_NO_WINDOW);

    ffmpeg_process.args(["-i", file.to_str().unwrap()]);

    if copy_codecs {
        ffmpeg_process.args(["-c:v", "copy"]).args(["-c:a", "copy"]);
    } else {
        let video_encoder = hwaccel::get_x264_encoder().await;
        hwaccel::apply_x264_encoder_args(
            &mut ffmpeg_process,
            video_encoder,
            Some(hwaccel::H264_SCALE_PAD_FILTER),
        );
        ffmpeg_process.args(["-c:a", "aac"]);
        hwaccel::apply_x264_quality_args(&mut ffmpeg_process, video_encoder);
        ffmpeg_process.args(["-threads", "0"]);
    }

    let child = ffmpeg_process
        .args([output_path.to_str().unwrap()])
        .args(["-y"])
        .args(["-progress", "pipe:2"])
        .stderr(Stdio::piped())
        .spawn();
    if let Err(e) = child {
        return Err(e.to_string());
    }

    let mut child = child.unwrap();
    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stderr);
    let mut parser = FfmpegLogParser::new(reader);
    while let Ok(event) = parser.parse_next_event().await {
        match event {
            FfmpegEvent::Progress(p) => {
                if reporter.is_none() {
                    continue;
                }
                reporter
                    .unwrap()
                    .update(format!("压制中：{}", p.time).as_str())
                    .await;
            }
            FfmpegEvent::LogEOF => break,
            FfmpegEvent::Error(e) => {
                log::error!("Transcode error: {e}");
                return Err(e.to_string());
            }
            _ => {}
        }
    }

    if let Err(e) = child.wait().await {
        return Err(e.to_string());
    }

    Ok(())
}

pub async fn trim_video(
    reporter: Option<&impl ProgressReporterTrait>,
    file: &Path,
    output_path: &Path,
    start_time: f64,
    duration: f64,
) -> Result<(), String> {
    // ffmpeg -i fixed_\[30655190\]1742887114_0325084106_81.5.mp4 -ss 0 -t 10 output.mp4
    log::info!("Trim video task start: {}", file.display());
    let mut ffmpeg_process = ffmpeg_command();
    #[cfg(target_os = "windows")]
    ffmpeg_process.creation_flags(CREATE_NO_WINDOW);

    ffmpeg_process.args(["-ss", &start_time.to_string()]);
    ffmpeg_process.args(["-i", file.to_str().unwrap()]);
    ffmpeg_process.args(["-t", &duration.to_string()]);
    ffmpeg_process.args(["-c", "copy"]);
    ffmpeg_process.args([output_path.to_str().unwrap()]);
    ffmpeg_process.args(["-y"]);
    ffmpeg_process.args(["-progress", "pipe:2"]);
    ffmpeg_process.stderr(Stdio::piped());
    let child = ffmpeg_process.spawn();
    if let Err(e) = child {
        return Err(e.to_string());
    }

    let mut child = child.unwrap();
    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stderr);
    let mut parser = FfmpegLogParser::new(reader);

    while let Ok(event) = parser.parse_next_event().await {
        match event {
            FfmpegEvent::Progress(p) => {
                if reporter.is_none() {
                    continue;
                }
                reporter
                    .unwrap()
                    .update(format!("切片中：{}", p.time).as_str())
                    .await;
            }
            FfmpegEvent::LogEOF => break,
            FfmpegEvent::Error(e) => {
                log::error!("Trim video error: {e}");
                return Err(e.to_string());
            }
            _ => {}
        }
    }

    if let Err(e) = child.wait().await {
        log::error!("Trim video error: {e}");
        return Err(e.to_string());
    }

    log::info!("Trim video task end: {}", output_path.display());
    Ok(())
}

/// Extract a sample audio from the video file for waveform display
pub async fn extract_audio_sample(file: &Path) -> Result<PathBuf, String> {
    // ffmpeg -i fixed_\[30655592\]1742887114_0325084106_81.5.mp4 -ar 16000 test.wav
    log::info!("Extract audio sample task start: {}", file.display());
    let output_path = file.with_extension("opus");
    let mut extract_error = None;

    let mut ffmpeg_process = ffmpeg_command();
    #[cfg(target_os = "windows")]
    ffmpeg_process.creation_flags(CREATE_NO_WINDOW);

    let child = ffmpeg_process
        .args(["-i", file.to_str().unwrap()])
        .args(["-c:a", "libopus"])
        .args(["-ar", "16000"])
        .args(["-ac", "1"])
        .args(["-vn"])
        .args(["-b:a", "64k"])
        .args(["-vbr", "on"])
        .args(["-compression_level", "10"])
        .args([output_path.to_str().unwrap()])
        .args(["-y"])
        .args(["-progress", "pipe:2"])
        .stderr(Stdio::piped())
        .spawn();

    if let Err(e) = child {
        return Err(e.to_string());
    }

    let mut child = child.unwrap();
    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stderr);
    let mut parser = FfmpegLogParser::new(reader);
    while let Ok(event) = parser.parse_next_event().await {
        match event {
            FfmpegEvent::Error(e) => {
                log::error!("Extract audio sample error: {e}");
                extract_error = Some(e.to_string());
            }
            FfmpegEvent::LogEOF => break,
            FfmpegEvent::Progress(p) => {
                log::info!("Extract audio sample progress: {}", p.time);
            }
            FfmpegEvent::Log(_level, _content) => {}
            _ => {}
        }
    }

    if let Err(e) = child.wait().await {
        log::error!("Extract audio sample error: {e}");
        return Err(e.to_string());
    }

    if let Some(error) = extract_error {
        log::error!("Extract audio sample error: {error}");
        Err(error)
    } else {
        log::info!("Extract audio sample task end: {}", output_path.display());
        Ok(output_path)
    }
}

pub async fn generate_audio_waveform(file: &Path) -> Result<AudioWaveformData, String> {
    let opus_path = file.with_extension("opus");
    if !opus_path.exists() {
        let _ = extract_audio_sample(file).await?;
    }

    let cache_path = file.with_extension("waveform.json");
    if cache_path.exists() {
        match tokio::fs::read_to_string(&cache_path).await {
            Ok(contents) => match serde_json::from_str::<AudioWaveformData>(&contents) {
                Ok(data) if !data.peaks.is_empty() && data.duration > 0.0 => return Ok(data),
                Ok(_) => {
                    log::warn!("Cached waveform data is invalid: {}", cache_path.display());
                }
                Err(error) => {
                    log::warn!(
                        "Failed to parse cached waveform data {}: {}",
                        cache_path.display(),
                        error
                    );
                }
            },
            Err(error) => {
                log::warn!(
                    "Failed to read cached waveform data {}: {}",
                    cache_path.display(),
                    error
                );
            }
        }
    }

    let duration_seconds = get_audio_duration(&opus_path).await? as usize;
    let peak_count = recommend_waveform_peak_count(duration_seconds);

    let temp_wav_path = file.with_extension("waveform.tmp.wav");
    let decode_result = decode_audio_for_waveform(&opus_path, &temp_wav_path).await;
    if let Err(error) = decode_result {
        let _ = tokio::fs::remove_file(&temp_wav_path).await;
        return Err(error);
    }

    let wav_path_for_task = temp_wav_path.clone();
    let waveform = match tokio::task::spawn_blocking(move || {
        compute_waveform_from_wav(&wav_path_for_task, peak_count)
    })
    .await
    {
        Ok(result) => result,
        Err(error) => Err(error.to_string()),
    };
    let _ = tokio::fs::remove_file(&temp_wav_path).await;

    let waveform = waveform?;
    if let Ok(contents) = serde_json::to_string(&waveform) {
        if let Err(error) = tokio::fs::write(&cache_path, contents).await {
            log::warn!(
                "Failed to cache waveform data {}: {}",
                cache_path.display(),
                error
            );
        }
    }

    Ok(waveform)
}

async fn decode_audio_for_waveform(input_path: &Path, output_path: &Path) -> Result<(), String> {
    log::info!(
        "Decode waveform audio task start: {} -> {}",
        input_path.display(),
        output_path.display()
    );

    let mut ffmpeg_process = tokio::process::Command::new(ffmpeg_path());
    #[cfg(target_os = "windows")]
    ffmpeg_process.creation_flags(CREATE_NO_WINDOW);

    let command_output = ffmpeg_process
        .args(["-i", input_path.to_str().unwrap()])
        .args(["-ac", "1"])
        .args(["-ar", "16000"])
        .args(["-c:a", "pcm_s16le"])
        .args(["-f", "wav"])
        .args([output_path.to_str().unwrap()])
        .args(["-y"])
        .output()
        .await
        .map_err(|error| error.to_string())?;

    if !command_output.status.success() {
        let stderr = String::from_utf8_lossy(&command_output.stderr)
            .trim()
            .to_string();
        if stderr.is_empty() {
            return Err("Failed to decode waveform audio".to_string());
        }
        return Err(stderr);
    }

    Ok(())
}

fn compute_waveform_from_wav(path: &Path, peak_count: usize) -> Result<AudioWaveformData, String> {
    let mut reader = hound::WavReader::open(path).map_err(|error| error.to_string())?;
    let spec = reader.spec();
    let total_samples = reader.duration() as usize;

    if spec.sample_rate == 0 || spec.channels == 0 || total_samples == 0 {
        return Err("Waveform source audio is empty".to_string());
    }

    let mut peaks = vec![0.0_f32; peak_count];
    for (index, sample) in reader.samples::<i16>().enumerate() {
        let value = sample.map_err(|error| error.to_string())? as f32 / i16::MAX as f32;
        let bucket = (((index as u128) * (peak_count as u128)) / (total_samples as u128)) as usize;
        let bucket = bucket.min(peak_count - 1);
        if value.abs() > peaks[bucket].abs() {
            peaks[bucket] = value;
        }
    }

    let duration = total_samples as f64 / spec.sample_rate as f64 / spec.channels as f64;
    Ok(AudioWaveformData { peaks, duration })
}

fn recommend_waveform_peak_count(duration_seconds: usize) -> usize {
    duration_seconds.saturating_mul(4).clamp(4_000, 48_000)
}

pub async fn extract_audio_chunks(
    file: &Path,
    format: &str,
    chunk_duration_seconds: u64,
) -> Result<PathBuf, String> {
    // ffmpeg -i fixed_\[30655190\]1742887114_0325084106_81.5.mp4 -ar 16000 test.wav
    log::info!("Extract audio task start: {}", file.display());
    let output_path = file.with_extension(format);

    // 降低采样率以提高处理速度，同时保持足够的音质用于语音识别
    let sample_rate = if format == "mp3" { "22050" } else { "16000" };

    // First, get the duration of the input file
    let duration = get_audio_duration(file).await?;
    log::info!("Audio duration: {duration} seconds");

    // Split into chunks of 30 seconds
    let chunk_count = duration.div_ceil(chunk_duration_seconds) as usize;
    log::info!("Splitting into {chunk_count} chunks of {chunk_duration_seconds} seconds each");

    // Create output directory for chunks
    let output_dir = output_path.parent().unwrap();
    let base_name = output_path.file_stem().unwrap().to_str().unwrap();
    let chunk_dir = output_dir.join(format!("{base_name}_chunks"));

    if chunk_dir.exists() {
        tokio::fs::remove_dir_all(&chunk_dir)
            .await
            .map_err(|e| format!("Failed to clean chunk directory: {e}"))?;
    }
    tokio::fs::create_dir_all(&chunk_dir)
        .await
        .map_err(|e| format!("Failed to create chunk directory: {e}"))?;

    let cpu_parallelism = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(2);
    let parallelism = cpu_parallelism
        .saturating_sub(1)
        .max(1)
        .min(AUDIO_CHUNK_MAX_PARALLELISM)
        .min(chunk_count.max(1));
    log::info!(
        "Extract audio chunks in parallel: chunks={} parallelism={} sample_rate={} format={}",
        chunk_count,
        parallelism,
        sample_rate,
        format
    );

    let semaphore = Arc::new(Semaphore::new(parallelism));
    let mut handles = Vec::with_capacity(chunk_count);
    for index in 0..chunk_count {
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| format!("Failed to acquire chunk extraction permit: {e}"))?;
        let input_path = file.to_path_buf();
        let output_file = chunk_dir.join(format!("{base_name}_{index:03}.{format}"));
        let format = format.to_string();
        let sample_rate = sample_rate.to_string();
        let start_seconds = chunk_duration_seconds * index as u64;
        let duration_seconds = chunk_duration_seconds.min(duration.saturating_sub(start_seconds));
        handles.push(tokio::spawn(async move {
            let _permit = permit;
            extract_audio_chunk_segment(
                &input_path,
                &output_file,
                &format,
                &sample_rate,
                index,
                start_seconds,
                duration_seconds,
            )
            .await
        }));
    }

    for handle in handles {
        handle
            .await
            .map_err(|e| format!("Audio chunk extraction task panicked: {e}"))??;
    }

    log::info!(
        "Extract audio task end: {} chunks created in {}",
        chunk_count,
        chunk_dir.display()
    );
    Ok(chunk_dir)
}

async fn extract_audio_chunk_segment(
    input_path: &Path,
    output_path: &Path,
    format: &str,
    sample_rate: &str,
    index: usize,
    start_seconds: u64,
    duration_seconds: u64,
) -> Result<(), String> {
    log::debug!(
        "Extract audio chunk start: index={} start={} duration={} output={}",
        index,
        start_seconds,
        duration_seconds,
        output_path.display()
    );

    let mut ffmpeg_process = ffmpeg_command();
    #[cfg(target_os = "windows")]
    ffmpeg_process.creation_flags(CREATE_NO_WINDOW);

    ffmpeg_process
        .args(["-ss", &start_seconds.to_string()])
        .args(["-t", &duration_seconds.to_string()])
        .args(["-i", input_path.to_str().unwrap()])
        .args(["-ar", sample_rate])
        .args(["-vn"]);

    if format == "mp3" {
        ffmpeg_process
            .args(["-c:a", "mp3"])
            .args(["-b:a", "64k"])
            .args(["-compression_level", "0"]);
    } else {
        ffmpeg_process.args(["-c:a", "pcm_s16le"]);
    }

    let command_output = ffmpeg_process
        .args(["-threads", "1"])
        .args(["-y", output_path.to_str().unwrap()])
        .output()
        .await
        .map_err(|e| format!("Failed to extract audio chunk {index}: {e}"))?;

    if !command_output.status.success() {
        return Err(format!(
            "Failed to extract audio chunk {index}: {}",
            String::from_utf8_lossy(&command_output.stderr)
        ));
    }

    log::debug!(
        "Extract audio chunk end: index={} output={}",
        index,
        output_path.display()
    );
    Ok(())
}

async fn extract_filetrans_audio(file: &Path) -> Result<PathBuf, String> {
    let output_path = file.with_extension("asr.mp3");
    if output_path.exists() {
        let input_modified = tokio::fs::metadata(file)
            .await
            .and_then(|metadata| metadata.modified())
            .ok();
        let output_modified = tokio::fs::metadata(&output_path)
            .await
            .and_then(|metadata| metadata.modified())
            .ok();
        if let (Some(input_modified), Some(output_modified)) = (input_modified, output_modified) {
            if output_modified >= input_modified {
                log::info!(
                    "Filetrans audio: reuse existing audio path={}",
                    output_path.display()
                );
                return Ok(output_path);
            }
        }
    }

    log::info!(
        "Filetrans audio: extracting full audio input={} output={}",
        file.display(),
        output_path.display()
    );
    let mut ffmpeg_process = tokio::process::Command::new(ffmpeg_path());
    #[cfg(target_os = "windows")]
    ffmpeg_process.creation_flags(CREATE_NO_WINDOW);

    let command_output = ffmpeg_process
        .args(["-i", file.to_str().unwrap()])
        .args(["-vn"])
        .args(["-ar", "22050"])
        .args(["-c:a", "mp3"])
        .args(["-b:a", "64k"])
        .args(["-compression_level", "0"])
        .args(["-threads", "1"])
        .args(["-y", output_path.to_str().unwrap()])
        .output()
        .await
        .map_err(|e| format!("Failed to extract filetrans audio: {e}"))?;

    if !command_output.status.success() {
        return Err(format!(
            "Failed to extract filetrans audio: {}",
            String::from_utf8_lossy(&command_output.stderr)
        ));
    }
    Ok(output_path)
}

/// Get the duration of an audio/video file in seconds
async fn get_audio_duration(file: &Path) -> Result<u64, String> {
    // Use ffprobe with format option to get duration
    let mut ffprobe_process = tokio::process::Command::new(ffprobe_path());
    #[cfg(target_os = "windows")]
    ffprobe_process.creation_flags(CREATE_NO_WINDOW);

    let child = ffprobe_process
        .args(["-v", "quiet"])
        .args(["-show_entries", "format=duration"])
        .args(["-of", "csv=p=0"])
        .args(["-i", file.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    if let Err(e) = child {
        return Err(format!("Failed to spawn ffprobe process: {e}"));
    }

    let mut child = child.unwrap();
    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);
    let mut parser = FfmpegLogParser::new(reader);

    let mut duration = None;
    while let Ok(event) = parser.parse_next_event().await {
        match event {
            FfmpegEvent::LogEOF => break,
            FfmpegEvent::Log(_level, content) => {
                // The new command outputs duration directly as a float
                if let Ok(seconds_f64) = content.trim().parse::<f64>() {
                    duration = Some(seconds_f64.ceil() as u64);
                    log::debug!("Parsed duration: {seconds_f64} seconds");
                }
            }
            _ => {}
        }
    }

    if let Err(e) = child.wait().await {
        log::error!("Failed to get duration: {e}");
        return Err(e.to_string());
    }

    duration.ok_or_else(|| "Failed to parse duration".to_string())
}

pub async fn encode_video_subtitle_to_path(
    reporter: &impl ProgressReporterTrait,
    file: &Path,
    subtitle: &Path,
    srt_style: String,
    output_path: &Path,
) -> Result<PathBuf, String> {
    // ffmpeg -i fixed_\[30655190\]1742887114_0325084106_81.5.mp4 -vf "subtitles=test.srt:force_style='FontSize=24'" -c:v libx264 -c:a copy output.mp4
    log::info!("Encode video subtitle task start: {}", file.display());
    log::info!("SRT style: {srt_style}");

    // check output path exists - log but allow overwrite
    if output_path.exists() {
        log::info!(
            "Output path already exists, will overwrite: {}",
            output_path.display()
        );
    }

    let mut command_error = None;

    // if windows
    let subtitle = if cfg!(target_os = "windows") {
        // escape characters in subtitle path
        let subtitle = subtitle
            .to_str()
            .unwrap()
            .replace('\\', "\\\\")
            .replace(':', "\\:");
        format!("'{subtitle}'")
    } else {
        format!("'{}'", subtitle.display())
    };
    let vf = format!(
        "{},subtitles={subtitle}:force_style='{srt_style}'",
        hwaccel::H264_SCALE_PAD_FILTER
    );
    log::info!("vf: {vf}");

    let mut ffmpeg_process = ffmpeg_command();
    #[cfg(target_os = "windows")]
    ffmpeg_process.creation_flags(CREATE_NO_WINDOW);

    let video_encoder = hwaccel::get_x264_encoder().await;

    ffmpeg_process.args(["-i", file.to_str().unwrap()]);
    hwaccel::apply_x264_encoder_args(&mut ffmpeg_process, video_encoder, Some(vf.as_str()));
    ffmpeg_process.args(["-c:a", "copy"]);
    hwaccel::apply_x264_quality_args(&mut ffmpeg_process, video_encoder);
    let child = ffmpeg_process
        .args([output_path.to_str().unwrap()])
        .args(["-y"])
        .args(["-progress", "pipe:2"])
        .stderr(Stdio::piped())
        .spawn();

    if let Err(e) = child {
        return Err(e.to_string());
    }

    let mut child = child.unwrap();
    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stderr);
    let mut parser = FfmpegLogParser::new(reader);

    while let Ok(event) = parser.parse_next_event().await {
        match event {
            FfmpegEvent::Error(e) => {
                log::error!("Encode video subtitle error: {e}");
                command_error = Some(e.to_string());
            }
            FfmpegEvent::Progress(p) => {
                log::info!("Encode video subtitle progress: {}", p.time);
                reporter
                    .update(format!("压制中：{}", p.time).as_str())
                    .await;
            }
            FfmpegEvent::LogEOF => break,
            FfmpegEvent::Log(_level, _content) => {}
            _ => {}
        }
    }

    let status = child.wait().await.map_err(|e| {
        log::error!("Encode video subtitle error: {e}");
        e.to_string()
    })?;

    if let Some(error) = command_error {
        log::error!("Encode video subtitle error: {error}");
        Err(error)
    } else if !status.success() {
        Err(format!(
            "ffmpeg subtitle encode failed with status: {status}"
        ))
    } else if !output_path.exists() {
        Err(format!(
            "ffmpeg subtitle encode finished but output is missing: {}",
            output_path.display()
        ))
    } else {
        log::info!("Encode video subtitle task end: {}", output_path.display());
        Ok(output_path.to_path_buf())
    }
}

/// Encode video subtitle using ffmpeg, output is file name with prefix [subtitle]
#[allow(dead_code)]
pub async fn encode_video_subtitle(
    reporter: &impl ProgressReporterTrait,
    file: &Path,
    subtitle: &Path,
    srt_style: String,
) -> Result<String, String> {
    let output_filename = format!(
        "{}{}",
        constants::PREFIX_SUBTITLE,
        file.file_name().unwrap().to_str().unwrap()
    );
    let output_path = file.with_file_name(&output_filename);
    encode_video_subtitle_to_path(reporter, file, subtitle, srt_style, &output_path)
        .await
        .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
}

pub async fn encode_video_danmu_to_path(
    reporter: Option<&impl ProgressReporterTrait>,
    file: &Path,
    subtitle: &Path,
    output_file_path: &Path,
) -> Result<PathBuf, String> {
    // ffmpeg -i fixed_\[30655190\]1742887114_0325084106_81.5.mp4 -vf ass=subtitle.ass -c:v libx264 -c:a copy output.mp4
    log::info!("Encode video danmu task start: {}", file.display());

    // check output path exists - log but allow overwrite
    if output_file_path.exists() {
        log::info!(
            "Output path already exists, will overwrite: {}",
            output_file_path.display()
        );
    }

    let mut command_error = None;

    // if windows
    let subtitle = if cfg!(target_os = "windows") {
        // escape characters in subtitle path
        let subtitle = subtitle
            .to_str()
            .unwrap()
            .replace('\\', "\\\\")
            .replace(':', "\\:");
        format!("'{subtitle}'")
    } else {
        format!("'{}'", subtitle.display())
    };

    let mut ffmpeg_process = ffmpeg_command();
    #[cfg(target_os = "windows")]
    ffmpeg_process.creation_flags(CREATE_NO_WINDOW);

    let video_encoder = hwaccel::get_x264_encoder().await;

    let vf = format!("{},ass={subtitle}", hwaccel::H264_SCALE_PAD_FILTER);
    ffmpeg_process.args(["-i", file.to_str().unwrap()]);
    hwaccel::apply_x264_encoder_args(&mut ffmpeg_process, video_encoder, Some(vf.as_str()));
    ffmpeg_process.args(["-c:a", "copy"]);
    hwaccel::apply_x264_quality_args(&mut ffmpeg_process, video_encoder);
    let child = ffmpeg_process
        .args([output_file_path.to_str().unwrap()])
        .args(["-y"])
        .args(["-progress", "pipe:2"])
        .stderr(Stdio::piped())
        .spawn();

    if let Err(e) = child {
        return Err(e.to_string());
    }

    let mut child = child.unwrap();
    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stderr);
    let mut parser = FfmpegLogParser::new(reader);

    while let Ok(event) = parser.parse_next_event().await {
        match event {
            FfmpegEvent::Error(e) => {
                log::error!("Encode video danmu error: {e}");
                command_error = Some(e.to_string());
            }
            FfmpegEvent::Progress(p) => {
                log::debug!("Encode video danmu progress: {}", p.time);
                if reporter.is_none() {
                    continue;
                }
                reporter
                    .unwrap()
                    .update(format!("压制中：{}", p.time).as_str())
                    .await;
            }
            FfmpegEvent::Log(_level, _content) => {}
            FfmpegEvent::LogEOF => break,
            _ => {}
        }
    }

    let status = child.wait().await.map_err(|e| {
        log::error!("Encode video danmu error: {e}");
        e.to_string()
    })?;

    if let Some(error) = command_error {
        log::error!("Encode video danmu error: {error}");
        Err(error)
    } else if !status.success() {
        Err(format!("ffmpeg danmu encode failed with status: {status}"))
    } else if !output_file_path.exists() {
        Err(format!(
            "ffmpeg danmu encode finished but output is missing: {}",
            output_file_path.display()
        ))
    } else {
        log::info!(
            "Encode video danmu task end: {}",
            output_file_path.display()
        );
        Ok(output_file_path.to_path_buf())
    }
}

pub async fn encode_video_danmu(
    reporter: Option<&impl ProgressReporterTrait>,
    file: &Path,
    subtitle: &Path,
) -> Result<PathBuf, String> {
    let danmu_filename = format!(
        "{}{}",
        constants::PREFIX_DANMAKU,
        file.file_name().unwrap().to_str().unwrap()
    );
    let output_file_path = file.with_file_name(danmu_filename);
    encode_video_danmu_to_path(reporter, file, subtitle, &output_file_path).await
}

fn escape_ffmpeg_filter_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace(':', "\\:")
}

fn danmu_emote_layer_filter_script(overlays: &[DanmuImageOverlay]) -> String {
    let mut script = String::new();
    script.push_str("[0:v]format=rgba[v0];\n");

    let mut previous_video = "v0".to_string();
    for (index, overlay) in overlays.iter().enumerate() {
        let image_label = format!("emote{index}");
        let next_video = format!("v{}", index + 1);
        let duration = (overlay.end - overlay.start).max(0.001);
        script.push_str(&format!(
            "movie='{}'[{image_label}];\n",
            escape_ffmpeg_filter_path(&overlay.image_path)
        ));
        script.push_str(&format!(
            "[{previous_video}][{image_label}]overlay=x='({:.3}+({:.3}-{:.3})*(t-{:.3})/{:.3})*main_w/1280':y='{:.3}*main_h/720':enable='between(t,{:.3},{:.3})':format=auto[{next_video}];\n",
            overlay.x_start,
            overlay.x_end,
            overlay.x_start,
            overlay.start,
            duration,
            overlay.y,
            overlay.start,
            overlay.end
        ));
        previous_video = next_video;
    }

    script.push_str(&format!("[{previous_video}]format=argb[vout]\n"));
    script
}

fn danmu_image_overlay_batch_size(overlay_count: usize) -> usize {
    if overlay_count <= DANMU_IMAGE_OVERLAY_SINGLE_PASS_LIMIT {
        overlay_count.max(1)
    } else {
        DANMU_IMAGE_OVERLAY_BATCH_SIZE
    }
}

fn danmu_emote_cache_key(path: &Path, target_size: u32) -> String {
    let metadata = std::fs::metadata(path).ok();
    let length = metadata.as_ref().map(|value| value.len()).unwrap_or(0);
    let modified = metadata
        .and_then(|value| value.modified().ok())
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok());
    let modified_secs = modified.as_ref().map(|value| value.as_secs()).unwrap_or(0);
    let modified_nanos = modified
        .as_ref()
        .map(|value| value.subsec_nanos())
        .unwrap_or(0);
    let fingerprint = format!(
        "{}|{}|{}|{}|{}",
        path.to_string_lossy(),
        length,
        modified_secs,
        modified_nanos,
        target_size
    );
    format!("{:x}", md5::compute(fingerprint))
}

fn danmu_emote_target_size(overlay: &DanmuImageOverlay, reference_height: u32) -> u32 {
    (overlay.size * reference_height as f64 / 720.0)
        .round()
        .max(1.0) as u32
}

async fn convert_danmu_emote_to_scaled_rgba_cache(
    input: &Path,
    output: &Path,
    target_size: u32,
) -> Result<(), String> {
    if output.exists() {
        return Ok(());
    }

    let parent = output
        .parent()
        .ok_or_else(|| format!("Invalid danmu emote cache path: {}", output.display()))?;
    tokio::fs::create_dir_all(parent).await.map_err(|e| {
        format!(
            "Failed to create danmu emote cache dir {}: {e}",
            parent.display()
        )
    })?;

    let temporary_output = parent.join(format!(
        "{}.{}.tmp.png",
        output
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("emote"),
        Uuid::new_v4()
    ));
    let mut ffmpeg_process = tokio::process::Command::new(ffmpeg_path());
    #[cfg(target_os = "windows")]
    ffmpeg_process.creation_flags(CREATE_NO_WINDOW);

    let scale_filter = format!("scale=w={target_size}:h={target_size},format=rgba");
    let command_output = ffmpeg_process
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-y",
            "-i",
            input.to_string_lossy().as_ref(),
            "-frames:v",
            "1",
            "-vf",
            scale_filter.as_str(),
            "-pix_fmt",
            "rgba",
            temporary_output.to_string_lossy().as_ref(),
        ])
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| {
            format!(
                "Failed to convert danmu emote to scaled RGBA cache {} size={}: {e}",
                input.display(),
                target_size
            )
        })?;

    if !command_output.status.success() {
        let _ = tokio::fs::remove_file(&temporary_output).await;
        let stderr = String::from_utf8_lossy(&command_output.stderr)
            .trim()
            .to_string();
        return Err(format!(
            "Failed to convert danmu emote to scaled RGBA cache {} size={}: {}{}",
            input.display(),
            target_size,
            command_output.status,
            if stderr.is_empty() {
                String::new()
            } else {
                format!("\n{stderr}")
            }
        ));
    }

    if output.exists() {
        let _ = tokio::fs::remove_file(&temporary_output).await;
        return Ok(());
    }

    tokio::fs::rename(&temporary_output, output)
        .await
        .map_err(|e| {
            let _ = std::fs::remove_file(&temporary_output);
            format!(
                "Failed to move danmu emote scaled RGBA cache {} -> {}: {e}",
                temporary_output.display(),
                output.display()
            )
        })
}

async fn prepare_danmu_emote_overlay_cache(
    overlays: &[DanmuImageOverlay],
    output_dir: &Path,
    reference_height: u32,
) -> Result<Vec<DanmuImageOverlay>, String> {
    let cache_dir = output_dir.join(".danmaku-emote-rgba-cache");
    let mut cached_paths: HashMap<(PathBuf, u32), PathBuf> = HashMap::new();
    let mut cached_overlays = Vec::with_capacity(overlays.len());

    for overlay in overlays {
        let target_size = danmu_emote_target_size(overlay, reference_height);
        let cache_key = (overlay.image_path.clone(), target_size);
        let cached_path = if let Some(cached_path) = cached_paths.get(&cache_key) {
            cached_path.clone()
        } else {
            let cache_path = cache_dir.join(format!(
                "{}.{}.rgba.png",
                danmu_emote_cache_key(&overlay.image_path, target_size),
                target_size
            ));
            convert_danmu_emote_to_scaled_rgba_cache(&overlay.image_path, &cache_path, target_size)
                .await?;
            cached_paths.insert(cache_key, cache_path.clone());
            cache_path
        };

        let mut cached_overlay = overlay.clone();
        cached_overlay.image_path = cached_path;
        cached_overlays.push(cached_overlay);
    }

    log::info!(
        "Prepared danmu emote scaled RGBA cache: unique={} overlays={} reference_height={} dir={}",
        cached_paths.len(),
        overlays.len(),
        reference_height,
        cache_dir.display()
    );

    Ok(cached_overlays)
}

pub async fn encode_video_danmu_with_images_to_path(
    reporter: Option<&impl ProgressReporterTrait>,
    file: &Path,
    subtitle: &Path,
    overlays: &[DanmuImageOverlay],
    output_file_path: &Path,
) -> Result<PathBuf, String> {
    if overlays.is_empty() {
        return encode_video_danmu_to_path(reporter, file, subtitle, output_file_path).await;
    }

    log::info!(
        "Encode video danmu with image overlays task start: file={} subtitle={} overlays={}",
        file.display(),
        subtitle.display(),
        overlays.len()
    );

    let batch_size = danmu_image_overlay_batch_size(overlays.len());
    log::info!(
        "Encode video danmu transparent layer in batches: overlays={} batch_size={}",
        overlays.len(),
        batch_size
    );
    let stem = output_file_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("danmu-image");
    let output_dir = output_file_path.parent().unwrap_or_else(|| Path::new("."));
    let layer_path = output_dir.join(format!("{stem}.danmaku-layer.mov"));
    let result =
        render_danmu_layer_to_path(reporter, file, overlays, &layer_path, batch_size).await;
    if let Err(err) = result {
        let _ = tokio::fs::remove_file(&layer_path).await;
        let _ = tokio::fs::remove_file(output_file_path).await;
        return Err(err);
    }

    let result =
        compose_video_with_danmu_layer(reporter, file, subtitle, &layer_path, output_file_path)
            .await;
    let _ = tokio::fs::remove_file(&layer_path).await;
    if let Err(err) = result {
        let _ = tokio::fs::remove_file(output_file_path).await;
        return Err(err);
    }

    if !output_file_path.exists() {
        return Err(format!(
            "ffmpeg danmu image encode finished but output is missing: {}",
            output_file_path.display()
        ));
    }
    Ok(output_file_path.to_path_buf())
}

async fn render_danmu_layer_to_path(
    reporter: Option<&impl ProgressReporterTrait>,
    file: &Path,
    overlays: &[DanmuImageOverlay],
    layer_path: &Path,
    batch_size: usize,
) -> Result<PathBuf, String> {
    let metadata = extract_video_metadata(file).await?;
    if metadata.width == 0 || metadata.height == 0 {
        return Err("ffmpeg danmu layer render failed: invalid video size".to_string());
    }
    if !metadata.duration.is_finite() || metadata.duration <= 0.0 {
        return Err("ffmpeg danmu layer render failed: invalid video duration".to_string());
    }

    let extension = layer_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("mov");
    let stem = layer_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("danmaku-layer");
    let output_dir = layer_path.parent().unwrap_or_else(|| Path::new("."));
    let cached_overlays =
        prepare_danmu_emote_overlay_cache(overlays, output_dir, metadata.height).await?;
    let chunks = cached_overlays
        .chunks(batch_size.max(1))
        .collect::<Vec<_>>();
    let parallelism = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(2)
        .max(1)
        .min(chunks.len().max(1));
    log::info!(
        "Render danmu transparent layer parallel batches: batches={} parallelism={}",
        chunks.len(),
        parallelism
    );

    if chunks.len() == 1 {
        render_danmu_layer_batch(metadata, chunks[0].to_vec(), layer_path.to_path_buf(), 1, 1)
            .await?;
    } else {
        let semaphore = Arc::new(Semaphore::new(parallelism));
        let mut tasks = FuturesUnordered::new();
        let mut batch_paths = Vec::with_capacity(chunks.len());

        for (index, chunk) in chunks.iter().enumerate() {
            let batch_output = output_dir.join(format!("{stem}.part{:03}.{extension}", index + 1));
            batch_paths.push(batch_output.clone());
            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|e| format!("Failed to acquire danmu layer render permit: {e}"))?;
            let metadata = metadata.clone();
            let chunk = chunk.to_vec();
            let chunk_count = chunks.len();

            tasks.push(tokio::spawn(async move {
                let _permit = permit;
                render_danmu_layer_batch(metadata, chunk, batch_output, index + 1, chunk_count)
                    .await
            }));
        }

        while let Some(result) = tasks.next().await {
            match result {
                Ok(Ok(path)) => {
                    log::info!(
                        "Render danmu transparent layer batch done: {}",
                        path.display()
                    );
                }
                Ok(Err(err)) => {
                    for path in &batch_paths {
                        let _ = tokio::fs::remove_file(path).await;
                    }
                    return Err(err);
                }
                Err(err) => {
                    for path in &batch_paths {
                        let _ = tokio::fs::remove_file(path).await;
                    }
                    return Err(format!("Render danmu layer batch task failed: {err}"));
                }
            }
        }

        let result = compose_danmu_layer_batches(reporter, &batch_paths, layer_path).await;
        for path in &batch_paths {
            let _ = tokio::fs::remove_file(path).await;
        }
        result?;
    }

    if !layer_path.exists() {
        return Err(format!(
            "ffmpeg danmu layer render finished but layer is missing: {}",
            layer_path.display()
        ));
    }
    Ok(layer_path.to_path_buf())
}

async fn render_danmu_layer_batch(
    metadata: VideoMetadata,
    overlays: Vec<DanmuImageOverlay>,
    batch_output: PathBuf,
    batch_index: usize,
    batch_count: usize,
) -> Result<PathBuf, String> {
    log::info!(
        "Render danmu transparent layer batch {}/{}: overlays={} output={}",
        batch_index,
        batch_count,
        overlays.len(),
        batch_output.display()
    );

    let filter_script_path = batch_output.with_extension("danmu-layer-filter.txt");
    tokio::fs::write(
        &filter_script_path,
        danmu_emote_layer_filter_script(&overlays),
    )
    .await
    .map_err(|e| format!("Failed to write danmu layer filter script: {e}"))?;

    let args = vec![
        "-f".to_string(),
        "lavfi".to_string(),
        "-i".to_string(),
        format!(
            "color=c=black@0.0:s={}x{}:r={}:d={:.3},format=rgba",
            metadata.width, metadata.height, DANMU_LAYER_FPS, metadata.duration
        ),
        "-/filter_complex".to_string(),
        filter_script_path.to_string_lossy().to_string(),
        "-map".to_string(),
        "[vout]".to_string(),
        "-an".to_string(),
        "-c:v".to_string(),
        "qtrle".to_string(),
        "-pix_fmt".to_string(),
        "argb".to_string(),
        "-y".to_string(),
        "-progress".to_string(),
        "pipe:2".to_string(),
        batch_output.to_string_lossy().to_string(),
    ];

    let result = run_ffmpeg_with_progress(
        Option::<&ProgressReporter>::None,
        &args,
        "Render danmu transparent layer",
        "渲染弹幕层",
    )
    .await;
    let _ = tokio::fs::remove_file(&filter_script_path).await;
    if let Err(err) = result {
        let _ = tokio::fs::remove_file(&batch_output).await;
        return Err(err);
    }

    if !batch_output.exists() {
        return Err(format!(
            "ffmpeg danmu layer batch finished but output is missing: {}",
            batch_output.display()
        ));
    }
    Ok(batch_output)
}

async fn compose_danmu_layer_batches(
    reporter: Option<&impl ProgressReporterTrait>,
    batch_paths: &[PathBuf],
    layer_path: &Path,
) -> Result<PathBuf, String> {
    if batch_paths.is_empty() {
        return Err("No danmu layer batches to compose".to_string());
    }

    let mut filter_script = String::new();
    filter_script.push_str("[0:v]format=rgba[v0];\n");
    let mut previous_video = "v0".to_string();
    for index in 1..batch_paths.len() {
        let next_video = format!("v{index}");
        filter_script.push_str(&format!(
            "[{previous_video}][{index}:v]overlay=x=0:y=0:eof_action=pass:format=auto[{next_video}];\n"
        ));
        previous_video = next_video;
    }
    filter_script.push_str(&format!("[{previous_video}]format=argb[vout]\n"));

    let filter_script_path = layer_path.with_extension("danmu-layer-compose-filter.txt");
    tokio::fs::write(&filter_script_path, filter_script)
        .await
        .map_err(|e| format!("Failed to write danmu layer compose filter script: {e}"))?;

    let mut args = Vec::new();
    for path in batch_paths {
        args.push("-i".to_string());
        args.push(path.to_string_lossy().to_string());
    }
    args.extend([
        "-/filter_complex".to_string(),
        filter_script_path.to_string_lossy().to_string(),
        "-map".to_string(),
        "[vout]".to_string(),
        "-an".to_string(),
        "-c:v".to_string(),
        "qtrle".to_string(),
        "-pix_fmt".to_string(),
        "argb".to_string(),
        "-y".to_string(),
        "-progress".to_string(),
        "pipe:2".to_string(),
        layer_path.to_string_lossy().to_string(),
    ]);

    let result = run_ffmpeg_with_progress(
        reporter,
        &args,
        "Compose danmu transparent layer batches",
        "合成弹幕层",
    )
    .await;
    let _ = tokio::fs::remove_file(&filter_script_path).await;
    if let Err(err) = result {
        let _ = tokio::fs::remove_file(layer_path).await;
        return Err(err);
    }

    if !layer_path.exists() {
        return Err(format!(
            "ffmpeg danmu layer batch compose finished but output is missing: {}",
            layer_path.display()
        ));
    }
    Ok(layer_path.to_path_buf())
}

async fn compose_video_with_danmu_layer(
    reporter: Option<&impl ProgressReporterTrait>,
    file: &Path,
    subtitle: &Path,
    layer_path: &Path,
    output_file_path: &Path,
) -> Result<PathBuf, String> {
    let filter_script_path = output_file_path.with_extension("danmu-compose-filter.txt");
    tokio::fs::write(
        &filter_script_path,
        format!(
            "[0:v]fps={},ass='{}'[base];\n[base][1:v]overlay=x=0:y=0:eof_action=pass:format=auto,format=yuv420p[vout]\n",
            DANMU_LAYER_FPS,
            escape_ffmpeg_filter_path(subtitle)
        ),
    )
    .await
    .map_err(|e| format!("Failed to write danmu compose filter script: {e}"))?;

    let args = vec![
        "-i".to_string(),
        file.to_string_lossy().to_string(),
        "-i".to_string(),
        layer_path.to_string_lossy().to_string(),
        "-/filter_complex".to_string(),
        filter_script_path.to_string_lossy().to_string(),
        "-map".to_string(),
        "[vout]".to_string(),
        "-map".to_string(),
        "0:a?".to_string(),
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        "veryfast".to_string(),
        "-crf".to_string(),
        "18".to_string(),
        "-r".to_string(),
        DANMU_LAYER_FPS.to_string(),
        "-c:a".to_string(),
        "copy".to_string(),
        "-movflags".to_string(),
        "+faststart".to_string(),
        "-y".to_string(),
        "-progress".to_string(),
        "pipe:2".to_string(),
        output_file_path.to_string_lossy().to_string(),
    ];

    let result = run_ffmpeg_with_progress(
        reporter,
        &args,
        "Compose video with danmu layer",
        "合成弹幕",
    )
    .await;
    let _ = tokio::fs::remove_file(&filter_script_path).await;
    result?;

    if !output_file_path.exists() {
        return Err(format!(
            "ffmpeg danmu layer compose finished but output is missing: {}",
            output_file_path.display()
        ));
    }
    Ok(output_file_path.to_path_buf())
}

async fn run_ffmpeg_with_progress(
    reporter: Option<&impl ProgressReporterTrait>,
    args: &[String],
    task_name: &str,
    progress_label: &str,
) -> Result<(), String> {
    let mut ffmpeg_process = tokio::process::Command::new(ffmpeg_path());
    #[cfg(target_os = "windows")]
    ffmpeg_process.creation_flags(CREATE_NO_WINDOW);

    let child = ffmpeg_process.args(args).stderr(Stdio::piped()).spawn();
    if let Err(e) = child {
        return Err(e.to_string());
    }

    let mut child = child.unwrap();
    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stderr);
    let mut parser = FfmpegLogParser::new(reader);
    let mut recent_logs = Vec::new();
    let mut command_error = None;

    while let Ok(event) = parser.parse_next_event().await {
        match event {
            FfmpegEvent::Error(e) => {
                log::error!("{task_name} error: {e}");
                command_error = Some(e.to_string());
            }
            FfmpegEvent::Progress(p) => {
                log::debug!("{task_name} progress: {}", p.time);
                if reporter.is_none() {
                    continue;
                }
                reporter
                    .unwrap()
                    .update(format!("{progress_label}：{}", p.time).as_str())
                    .await;
            }
            FfmpegEvent::Log(_level, content) => {
                if !content.trim().is_empty() {
                    recent_logs.push(content.clone());
                    if recent_logs.len() > 20 {
                        recent_logs.remove(0);
                    }
                    log::debug!("{task_name} ffmpeg: {content}");
                }
            }
            FfmpegEvent::LogEOF => break,
            _ => {}
        }
    }

    let status = child.wait().await.map_err(|e| {
        log::error!("{task_name} error: {e}");
        e.to_string()
    })?;

    if let Some(error) = command_error {
        log::error!("{task_name} error: {error}");
        if !recent_logs.is_empty() {
            log::error!(
                "{task_name} recent ffmpeg logs:\n{}",
                recent_logs.join("\n")
            );
        }
        Err(error)
    } else if !status.success() {
        let tail = recent_logs.join("\n");
        if !tail.is_empty() {
            log::error!("{task_name} failed ffmpeg tail:\n{tail}");
        }
        Err(format!("{task_name} failed with status: {status}"))
    } else {
        Ok(())
    }
}

#[allow(dead_code)]
pub async fn encode_video_danmu_with_images(
    reporter: Option<&impl ProgressReporterTrait>,
    file: &Path,
    subtitle: &Path,
    overlays: &[DanmuImageOverlay],
) -> Result<PathBuf, String> {
    let danmu_filename = format!(
        "{}{}",
        constants::PREFIX_DANMAKU,
        file.file_name().unwrap().to_str().unwrap()
    );
    let output_file_path = file.with_file_name(danmu_filename);
    encode_video_danmu_with_images_to_path(reporter, file, subtitle, overlays, &output_file_path)
        .await
}

pub async fn generic_ffmpeg_command(args: &[&str]) -> Result<String, String> {
    let mut ffmpeg_process = ffmpeg_command();
    #[cfg(target_os = "windows")]
    ffmpeg_process.creation_flags(CREATE_NO_WINDOW);

    let child = ffmpeg_process.args(args).stderr(Stdio::piped()).spawn();
    if let Err(e) = child {
        return Err(e.to_string());
    }

    let mut child = child.unwrap();
    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stderr);
    let mut parser = FfmpegLogParser::new(reader);

    let mut logs = Vec::new();

    while let Ok(event) = parser.parse_next_event().await {
        match event {
            FfmpegEvent::Log(_level, content) => {
                logs.push(content);
            }
            FfmpegEvent::LogEOF => break,
            _ => {}
        }
    }

    if let Err(e) = child.wait().await {
        log::error!("Generic ffmpeg command error: {e}");
        return Err(e.to_string());
    }

    Ok(logs.join("\n"))
}

#[allow(clippy::too_many_arguments)]
pub async fn generate_video_subtitle(
    reporter: Option<&ProgressReporter>,
    file: &Path,
    generator_type: &str,
    whisper_model: &str,
    whisper_prompt: &str,
    openai_api_key: &str,
    openai_api_endpoint: &str,
    online_asr_model: &str,
    _oss_access_key_id: &str,
    _oss_access_key_secret: &str,
    _oss_bucket: &str,
    _oss_endpoint: &str,
    _oss_object_prefix: &str,
    asr_hotword_vocabulary_id: &str,
    language_hint: &str,
) -> Result<GenerateResult, String> {
    log::info!(
        "Generate video subtitle: file={} generator_type={} online_asr_model={} language_hint={}",
        file.display(),
        generator_type,
        online_asr_model,
        language_hint
    );
    match generator_type {
        "whisper" => {
            if whisper_model.is_empty() {
                return Err("Whisper model not configured".to_string());
            }
            if let Ok(generator) = whisper_cpp::new(Path::new(&whisper_model), whisper_prompt).await
            {
                let chunk_dir =
                    extract_audio_chunks(file, "wav", LOCAL_AUDIO_CHUNK_DURATION_SECONDS).await?;

                let mut full_result = GenerateResult {
                    subtitle_id: String::new(),
                    subtitle_content: vec![],
                    generator_type: SubtitleGeneratorType::Whisper,
                };

                let mut chunk_paths = vec![];
                for entry in std::fs::read_dir(&chunk_dir)
                    .map_err(|e| format!("Failed to read chunk directory: {e}"))?
                {
                    let entry =
                        entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
                    let path = entry.path();
                    chunk_paths.push(path);
                }

                // sort chunk paths by name
                chunk_paths
                    .sort_by_key(|path| path.file_name().unwrap().to_str().unwrap().to_string());

                let mut results = Vec::new();
                for path in chunk_paths {
                    let result = generator
                        .generate_subtitle(reporter, &path, language_hint)
                        .await;
                    results.push(result);
                }

                for (i, result) in results.iter().enumerate() {
                    if let Ok(result) = result {
                        full_result.subtitle_id = result.subtitle_id.clone();
                        full_result.concat(result, 30 * i as u64);
                    }
                }

                // delete chunk directory
                let _ = tokio::fs::remove_dir_all(chunk_dir).await;

                Ok(full_result)
            } else {
                Err("Failed to initialize Whisper model".to_string())
            }
        }
        "whisper_online" => {
            if openai_api_key.is_empty() {
                return Err("API key not configured".to_string());
            }
            log::info!(
                "Generate online subtitle: initializing generator model={} endpoint={}",
                online_asr_model,
                openai_api_endpoint
            );
            if let Ok(generator) = whisper_online::new(
                Some(openai_api_endpoint),
                Some(openai_api_key),
                Some(whisper_prompt),
                Some(online_asr_model),
                Some(asr_hotword_vocabulary_id),
            )
            .await
            {
                let is_filetrans_asr = matches!(
                    online_asr_model,
                    "fun-asr-filetrans" | "fun-asr" | "qwen3-asr-flash-filetrans"
                );
                if is_filetrans_asr {
                    let audio_path = extract_filetrans_audio(file).await?;
                    let partial_srt_path = file.with_extension("srt.partial");
                    tokio::fs::write(&partial_srt_path, "").await.map_err(|e| {
                        format!(
                            "Failed to initialize filetrans partial SRT {}: {e}",
                            partial_srt_path.display()
                        )
                    })?;
                    log::info!(
                        "Generate online subtitle: filetrans full audio source={} audio={} partial_srt={}",
                        file.display(),
                        audio_path.display(),
                        partial_srt_path.display()
                    );
                    let result = generator
                        .generate_subtitle_with_partial(
                            reporter,
                            &audio_path,
                            language_hint,
                            Some(partial_srt_path.as_path()),
                            0,
                            1,
                        )
                        .await?;
                    log::info!(
                        "Generate online subtitle: filetrans finished subtitles={}",
                        result.subtitle_content.len()
                    );
                    return Ok(result);
                }

                let uses_long_online_chunks = matches!(
                    online_asr_model,
                    "fun-asr-realtime" | "qwen3-asr-flash-realtime" | "qwen-asr-realtime"
                );
                let chunk_duration_seconds = if uses_long_online_chunks {
                    REALTIME_ASR_AUDIO_CHUNK_DURATION_SECONDS
                } else {
                    LOCAL_AUDIO_CHUNK_DURATION_SECONDS
                };
                let chunk_dir = extract_audio_chunks(file, "mp3", chunk_duration_seconds).await?;
                log::info!(
                    "Generate online subtitle: extracted audio chunks dir={}",
                    chunk_dir.display()
                );

                let mut full_result = GenerateResult {
                    subtitle_id: String::new(),
                    subtitle_content: vec![],
                    generator_type: SubtitleGeneratorType::WhisperOnline,
                };

                let mut chunk_paths = vec![];
                for entry in std::fs::read_dir(&chunk_dir)
                    .map_err(|e| format!("Failed to read chunk directory: {e}"))?
                {
                    let entry =
                        entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
                    let path = entry.path();
                    chunk_paths.push(path);
                }
                // sort chunk paths by name
                chunk_paths
                    .sort_by_key(|path| path.file_name().unwrap().to_str().unwrap().to_string());
                log::info!(
                    "Generate online subtitle: chunk count={}",
                    chunk_paths.len()
                );
                let partial_srt_path = file.with_extension("srt.partial");
                if uses_long_online_chunks {
                    tokio::fs::write(&partial_srt_path, "").await.map_err(|e| {
                        format!(
                            "Failed to initialize realtime partial SRT {}: {e}",
                            partial_srt_path.display()
                        )
                    })?;
                    log::info!(
                        "Generate online subtitle: realtime partial srt initialized path={}",
                        partial_srt_path.display()
                    );
                }

                for (index, path) in chunk_paths.iter().enumerate() {
                    let offset_seconds = chunk_duration_seconds * index as u64;
                    let next_subtitle_index = full_result.subtitle_content.len() + 1;
                    log::info!(
                        "Generate online subtitle: processing chunk {}/{} path={} offset_seconds={} next_subtitle_index={}",
                        index + 1,
                        chunk_paths.len(),
                        path.display(),
                        offset_seconds,
                        next_subtitle_index
                    );
                    let result = generator
                        .generate_subtitle_with_partial(
                            reporter,
                            path,
                            language_hint,
                            uses_long_online_chunks.then_some(partial_srt_path.as_path()),
                            offset_seconds,
                            next_subtitle_index,
                        )
                        .await;
                    match &result {
                        Ok(result) => log::info!(
                            "Generate online subtitle: chunk {}/{} success subtitles={}",
                            index + 1,
                            chunk_paths.len(),
                            result.subtitle_content.len()
                        ),
                        Err(error) => log::error!(
                            "Generate online subtitle: chunk {}/{} failed: {}",
                            index + 1,
                            chunk_paths.len(),
                            error
                        ),
                    }
                    let result = result?;
                    full_result.subtitle_id = result.subtitle_id.clone();
                    full_result.concat(&result, offset_seconds);
                }

                // delete chunk directory
                let _ = tokio::fs::remove_dir_all(chunk_dir).await;
                log::info!(
                    "Generate online subtitle: finished chunks={} merged_subtitles={}",
                    chunk_paths.len(),
                    full_result.subtitle_content.len()
                );

                Ok(full_result)
            } else {
                Err("Failed to initialize Whisper Online".to_string())
            }
        }
        "powerlive" => {
            if let Ok(generator) = powerlive::new(
                "pk_d2755cd38ef03f7ed3a92be1f1471e4adea90a1a5d4b3900345298a68fba0821",
            )
            .await
            {
                let opus_file = file.with_extension("opus");
                if !opus_file.exists() {
                    return Err("Opus file not found".to_string());
                }
                let result = generator
                    .generate_subtitle(reporter, &opus_file, language_hint)
                    .await;
                match result {
                    Ok(result) => Ok(result),
                    Err(e) => Err(e),
                }
            } else {
                Err("Failed to initialize PowerLive".to_string())
            }
        }
        _ => Err(format!("Unknown subtitle generator type: {generator_type}")),
    }
}

/// Trying to run ffmpeg for version
pub async fn check_ffmpeg() -> Result<String, String> {
    let child = tokio::process::Command::new(ffmpeg_path())
        .arg("-version")
        .stdout(Stdio::piped())
        .spawn();
    if let Err(e) = child {
        log::error!("Failed to spawn ffmpeg process: {e}");
        return Err(e.to_string());
    }

    let mut child = child.unwrap();

    let stdout = child.stdout.take();
    if stdout.is_none() {
        log::error!("Failed to take ffmpeg output");
        return Err("Failed to take ffmpeg output".into());
    }

    let stdout = stdout.unwrap();
    let reader = BufReader::new(stdout);
    let mut parser = FfmpegLogParser::new(reader);

    let mut version = None;
    while let Ok(event) = parser.parse_next_event().await {
        match event {
            FfmpegEvent::ParsedVersion(v) => version = Some(v.version),
            FfmpegEvent::LogEOF => break,
            _ => {}
        }
    }

    if let Some(version) = version {
        Ok(version)
    } else {
        Err("Failed to parse version from output".into())
    }
}

pub fn ffmpeg_command() -> tokio::process::Command {
    let mut command = tokio::process::Command::new(ffmpeg_path());
    command.kill_on_drop(true);
    command
}

fn executable_path(name: &str) -> PathBuf {
    let mut path = Path::new(name).to_path_buf();
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path
}

fn ffmpeg_candidate_paths() -> Vec<PathBuf> {
    let mut candidates = vec![executable_path("ffmpeg")];
    if !cfg!(windows) {
        candidates.extend([
            PathBuf::from("/opt/anaconda3/bin/ffmpeg"),
            PathBuf::from("/opt/homebrew/bin/ffmpeg"),
            PathBuf::from("/usr/local/bin/ffmpeg"),
            PathBuf::from("/usr/bin/ffmpeg"),
        ]);
    }

    let mut seen = HashSet::new();
    candidates.retain(|path| seen.insert(path.to_string_lossy().to_string()));
    candidates
}

fn ffmpeg_filter_names(path: &Path) -> Option<HashSet<String>> {
    let output = StdCommand::new(path)
        .args(["-hide_banner", "-filters"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let filters = String::from_utf8_lossy(&output.stdout);
    let names = filters
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            parts.next()?;
            parts.next().map(ToOwned::to_owned)
        })
        .collect::<HashSet<_>>();
    Some(names)
}

fn ffmpeg_encoder_names(path: &Path) -> Option<HashSet<String>> {
    let output = StdCommand::new(path)
        .args(["-hide_banner", "-encoders"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let encoders = String::from_utf8_lossy(&output.stdout);
    let names = encoders
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            parts.next()?;
            parts.next().map(ToOwned::to_owned)
        })
        .collect::<HashSet<_>>();
    Some(names)
}

fn ffmpeg_supports_danmaku_encode(path: &Path) -> bool {
    let Some(filters) = ffmpeg_filter_names(path) else {
        return false;
    };
    let Some(encoders) = ffmpeg_encoder_names(path) else {
        return false;
    };
    ["ass", "subtitles", "movie", "overlay", "scale"]
        .iter()
        .all(|name| filters.contains(*name))
        && encoders.contains("libx264")
}

fn ffmpeg_is_available(path: &Path) -> bool {
    StdCommand::new(path)
        .args(["-hide_banner", "-version"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn select_ffmpeg_path() -> PathBuf {
    let candidates = ffmpeg_candidate_paths();
    for path in &candidates {
        if ffmpeg_supports_danmaku_encode(path) {
            log::info!(
                "Selected ffmpeg with danmaku filters and libx264: {}",
                path.display()
            );
            return path.clone();
        }
    }

    for path in &candidates {
        if ffmpeg_is_available(path) {
            log::warn!(
                "Selected ffmpeg without full danmaku filter support: {}",
                path.display()
            );
            return path.clone();
        }
    }

    executable_path("ffmpeg")
}

pub fn ffmpeg_path() -> PathBuf {
    FFMPEG_PATH_CACHE.get_or_init(select_ffmpeg_path).clone()
}

fn ffprobe_path() -> PathBuf {
    FFPROBE_PATH_CACHE
        .get_or_init(|| {
            let selected_ffmpeg = ffmpeg_path();
            if selected_ffmpeg.is_absolute() {
                if let Some(parent) = selected_ffmpeg.parent() {
                    let mut path = parent.join("ffprobe");
                    if cfg!(windows) {
                        path.set_extension("exe");
                    }
                    if ffmpeg_is_available(&path) {
                        return path;
                    }
                }
            }
            executable_path("ffprobe")
        })
        .clone()
}

// 从视频文件切片
pub async fn clip_from_video_file(
    reporter: Option<&impl ProgressReporterTrait>,
    input_path: &Path,
    output_path: &Path,
    start_time: f64,
    duration: f64,
) -> Result<(), String> {
    let output_folder = output_path.parent().unwrap();
    if !output_folder.exists() {
        std::fs::create_dir_all(output_folder).unwrap();
    }

    trim_video(reporter, input_path, output_path, start_time, duration).await
}

/// Extract basic information from a video file.
///
/// # Arguments
/// * `file_path` - The path to the video file.
///
/// # Returns
/// A `Result` containing the video metadata or an error message.
pub async fn extract_video_metadata(file_path: &Path) -> Result<VideoMetadata, String> {
    let mut ffprobe_process = tokio::process::Command::new(ffprobe_path());
    #[cfg(target_os = "windows")]
    ffprobe_process.creation_flags(CREATE_NO_WINDOW);

    let output = ffprobe_process
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            &format!("{}", file_path.display()),
        ])
        .output()
        .await
        .map_err(|e| format!("执行ffprobe失败: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "ffprobe执行失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| format!("解析ffprobe输出失败: {e}"))?;

    let format_duration = json["format"]["duration"]
        .as_str()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0);

    // 解析视频流信息
    let streams = json["streams"].as_array().ok_or("未找到视频流信息")?;

    if streams.is_empty() {
        return Err("未找到视频流".to_string());
    }

    let mut metadata = VideoMetadata {
        duration: 0.0,
        width: 0,
        height: 0,
        video_codec: String::new(),
        audio_codec: String::new(),
    };

    for stream in streams {
        let codec_name = stream["codec_type"].as_str().unwrap_or("");
        if codec_name == "video" {
            metadata.video_codec = stream["codec_name"].as_str().unwrap_or("").to_owned();
            metadata.width = stream["width"].as_u64().unwrap_or(0) as u32;
            metadata.height = stream["height"].as_u64().unwrap_or(0) as u32;
            metadata.duration = stream["duration"]
                .as_str()
                .unwrap_or("0.0")
                .parse::<f64>()
                .unwrap_or(0.0);
        } else if codec_name == "audio" {
            metadata.audio_codec = stream["codec_name"].as_str().unwrap_or("").to_owned();
        }
    }
    if metadata.duration <= 0.0 {
        metadata.duration = format_duration;
    }
    Ok(metadata)
}

fn seekbar_thumbnail_cache_dir(video_full_path: &Path) -> PathBuf {
    video_full_path.with_extension(SEEKBAR_THUMBNAIL_CACHE_DIR_EXT)
}

fn seekbar_thumbnail_cache_manifest_path(video_full_path: &Path) -> PathBuf {
    seekbar_thumbnail_cache_dir(video_full_path).join(SEEKBAR_THUMBNAIL_CACHE_MANIFEST_FILE)
}

fn seekbar_thumbnail_cache_step_seconds(duration: f64) -> f64 {
    if !duration.is_finite() || duration <= 0.0 {
        return SEEKBAR_THUMBNAIL_CACHE_DEFAULT_STEP_SECONDS;
    }
    let step_for_min_frame_count = duration / SEEKBAR_THUMBNAIL_CACHE_MIN_FRAME_COUNT;
    if !step_for_min_frame_count.is_finite() || step_for_min_frame_count <= 0.0 {
        return SEEKBAR_THUMBNAIL_CACHE_DEFAULT_STEP_SECONDS;
    }
    step_for_min_frame_count.min(SEEKBAR_THUMBNAIL_CACHE_DEFAULT_STEP_SECONDS)
}

fn seekbar_thumbnail_decode_attempts() -> Vec<(&'static str, &'static [&'static str])> {
    let mut attempts = Vec::new();
    #[cfg(target_os = "macos")]
    attempts.push(("videotoolbox", &["-hwaccel", "videotoolbox"][..]));
    attempts.push(("auto", &["-hwaccel", "auto"][..]));
    attempts.push(("software", &[][..]));
    attempts
}

fn seekbar_thumbnail_cache_frame_path(cache_dir: &Path, index: usize) -> PathBuf {
    cache_dir.join(format!("{index:06}.jpg"))
}

pub async fn load_seekbar_thumbnail_cache_manifest(
    video_full_path: &Path,
) -> Option<SeekbarThumbnailCacheManifest> {
    let manifest_path = seekbar_thumbnail_cache_manifest_path(video_full_path);
    let contents = tokio::fs::read_to_string(&manifest_path).await.ok()?;
    let manifest = serde_json::from_str::<SeekbarThumbnailCacheManifest>(&contents).ok()?;
    if manifest.frame_count == 0 || manifest.step_seconds <= 0.0 || manifest.duration < 0.0 {
        return None;
    }
    Some(manifest)
}

pub async fn has_seekbar_thumbnail_cache(video_full_path: &Path) -> bool {
    let Some(manifest) = load_seekbar_thumbnail_cache_manifest(video_full_path).await else {
        return false;
    };
    let cache_dir = seekbar_thumbnail_cache_dir(video_full_path);
    let probe_index = manifest.frame_count.saturating_sub(1);
    seekbar_thumbnail_cache_frame_path(&cache_dir, probe_index).exists()
}

pub async fn generate_seekbar_thumbnail_cache(
    video_full_path: &Path,
) -> Result<SeekbarThumbnailCacheManifest, String> {
    let metadata = extract_video_metadata(video_full_path).await?;
    let duration = metadata.duration;
    if !duration.is_finite() || duration <= 0.0 {
        return Err("视频时长无效，无法生成预览图缓存".to_string());
    }

    let step_seconds = seekbar_thumbnail_cache_step_seconds(duration);
    let cache_dir = seekbar_thumbnail_cache_dir(video_full_path);

    let vf = format!(
        "fps=1/{step_seconds:.6},scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2:black",
        SEEKBAR_THUMBNAIL_CACHE_WIDTH,
        SEEKBAR_THUMBNAIL_CACHE_HEIGHT,
        SEEKBAR_THUMBNAIL_CACHE_WIDTH,
        SEEKBAR_THUMBNAIL_CACHE_HEIGHT
    );
    let output_pattern = cache_dir.join("%06d.jpg");
    let output_pattern_str = output_pattern.to_string_lossy().into_owned();
    let input_path = format!("{}", video_full_path.display());

    let mut ffmpeg_success = false;
    let mut ffmpeg_error = String::new();
    let decode_attempts = seekbar_thumbnail_decode_attempts();

    for (attempt_name, hwaccel_args) in decode_attempts {
        if cache_dir.exists() {
            let _ = tokio::fs::remove_dir_all(&cache_dir).await;
        }
        tokio::fs::create_dir_all(&cache_dir)
            .await
            .map_err(|e| format!("创建预览图缓存目录失败: {e}"))?;

        let mut ffmpeg_process = tokio::process::Command::new(ffmpeg_path());
        #[cfg(target_os = "windows")]
        ffmpeg_process.creation_flags(CREATE_NO_WINDOW);

        let output = ffmpeg_process
            .args(hwaccel_args)
            .args(["-i", input_path.as_str()])
            .args(["-vf", &vf])
            .args(["-q:v", "3"])
            .args(["-start_number", "0"])
            .args(["-y", output_pattern_str.as_str()])
            .output()
            .await
            .map_err(|e| format!("生成预览图缓存失败: {e}"))?;

        if output.status.success() {
            ffmpeg_success = true;
            if attempt_name != "software" {
                log::info!(
                    "Seekbar thumbnail cache extraction uses hw decode profile: {}",
                    attempt_name
                );
            }
            break;
        }

        ffmpeg_error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if ffmpeg_error.is_empty() {
            ffmpeg_error = "未知错误".to_string();
        }
        log::warn!(
            "Seekbar thumbnail cache extraction failed with decode profile {}: {}",
            attempt_name,
            ffmpeg_error
        );
    }

    if !ffmpeg_success {
        return Err(format!("ffmpeg生成预览图缓存失败: {ffmpeg_error}"));
    }

    let mut frame_count = 0usize;
    let mut entries = tokio::fs::read_dir(&cache_dir)
        .await
        .map_err(|e| format!("读取预览图缓存目录失败: {e}"))?;
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| format!("遍历预览图缓存目录失败: {e}"))?
    {
        let path = entry.path();
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("jpg"))
            .unwrap_or(false)
        {
            frame_count += 1;
        }
    }

    if frame_count == 0 {
        return Err("生成预览图缓存失败: 未产生任何帧".to_string());
    }

    let manifest = SeekbarThumbnailCacheManifest {
        step_seconds,
        duration,
        frame_count,
        width: SEEKBAR_THUMBNAIL_CACHE_WIDTH,
        height: SEEKBAR_THUMBNAIL_CACHE_HEIGHT,
    };
    let manifest_path = seekbar_thumbnail_cache_manifest_path(video_full_path);
    let manifest_contents =
        serde_json::to_string(&manifest).map_err(|e| format!("序列化预览图索引失败: {e}"))?;
    tokio::fs::write(&manifest_path, manifest_contents)
        .await
        .map_err(|e| format!("写入预览图索引失败: {e}"))?;

    Ok(manifest)
}

pub async fn read_seekbar_thumbnail_cache_bytes(
    video_full_path: &Path,
    timestamp: f64,
) -> Result<Option<Vec<u8>>, String> {
    let Some(manifest) = load_seekbar_thumbnail_cache_manifest(video_full_path).await else {
        return Ok(None);
    };
    if manifest.frame_count == 0 {
        return Ok(None);
    }

    let clamped_time = if timestamp.is_finite() {
        timestamp.max(0.0).min(manifest.duration.max(0.0))
    } else {
        0.0
    };
    let raw_index = (clamped_time / manifest.step_seconds).round();
    let max_index = manifest.frame_count.saturating_sub(1) as f64;
    let frame_index = raw_index.clamp(0.0, max_index) as usize;

    let cache_dir = seekbar_thumbnail_cache_dir(video_full_path);
    let frame_path = seekbar_thumbnail_cache_frame_path(&cache_dir, frame_index);
    if !frame_path.exists() {
        return Ok(None);
    }

    let bytes = tokio::fs::read(&frame_path)
        .await
        .map_err(|e| format!("读取预览图缓存失败: {e}"))?;
    if bytes.is_empty() {
        return Ok(None);
    }
    Ok(Some(bytes))
}

/// Generate thumbnail file from video, capturing a frame at the specified timestamp.
///
/// # Arguments
/// * `video_full_path` - The full path to the video file.
/// * `timestamp` - The timestamp (in seconds) to capture the thumbnail.
///
/// # Returns
/// The path to the generated thumbnail image.
pub async fn generate_thumbnail(video_full_path: &Path, timestamp: f64) -> Result<PathBuf, String> {
    let mut ffmpeg_process = ffmpeg_command();
    #[cfg(target_os = "windows")]
    ffmpeg_process.creation_flags(CREATE_NO_WINDOW);

    let thumbnail_full_path = video_full_path.with_extension("jpg");

    let output = ffmpeg_process
        .args(["-i", &format!("{}", video_full_path.display())])
        .args(["-ss", &timestamp.to_string()])
        .args(["-vframes", "1"])
        .args(["-y", thumbnail_full_path.to_str().unwrap()])
        .output()
        .await
        .map_err(|e| format!("生成缩略图失败: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "ffmpeg生成缩略图失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // 记录生成的缩略图信息
    if let Ok(metadata) = std::fs::metadata(&thumbnail_full_path) {
        log::info!(
            "生成缩略图完成: {} (文件大小: {} bytes)",
            thumbnail_full_path.display(),
            metadata.len()
        );
    } else {
        log::info!("生成缩略图完成: {}", thumbnail_full_path.display());
    }
    Ok(thumbnail_full_path)
}

/// Generate thumbnail bytes from video, capturing a frame at the specified timestamp.
///
/// Returns JPEG bytes rendered by ffmpeg through stdout pipe.
#[allow(dead_code)]
pub async fn generate_thumbnail_bytes(
    video_full_path: &Path,
    timestamp: f64,
) -> Result<Vec<u8>, String> {
    let mut ffmpeg_process = tokio::process::Command::new(ffmpeg_path());
    #[cfg(target_os = "windows")]
    ffmpeg_process.creation_flags(CREATE_NO_WINDOW);

    let output = ffmpeg_process
        .args(["-ss", &timestamp.to_string()])
        .args(["-i", &format!("{}", video_full_path.display())])
        .args(["-vframes", "1"])
        .args(["-f", "image2pipe"])
        .args(["-vcodec", "mjpeg"])
        .arg("pipe:1")
        .output()
        .await
        .map_err(|e| format!("生成缩略图字节失败: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "ffmpeg生成缩略图字节失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    if output.stdout.is_empty() {
        return Err("ffmpeg生成缩略图字节失败: 输出为空".to_string());
    }

    Ok(output.stdout)
}

// 执行FFmpeg转换的通用函数
pub async fn execute_ffmpeg_conversion(
    mut cmd: tokio::process::Command,
    reporter: &ProgressReporter,
    mode_name: &str,
) -> Result<(), String> {
    use async_ffmpeg_sidecar::event::FfmpegEvent;
    use async_ffmpeg_sidecar::log_parser::FfmpegLogParser;
    use std::process::Stdio;
    use tokio::io::BufReader;

    let mut child = cmd
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动FFmpeg进程失败: {e}"))?;

    let stderr = child.stderr.take().unwrap();
    let reader = BufReader::new(stderr);
    let mut parser = FfmpegLogParser::new(reader);

    let mut conversion_error = None;
    while let Ok(event) = parser.parse_next_event().await {
        match event {
            FfmpegEvent::Progress(p) => {
                reporter
                    .update(&format!("正在转换视频格式... {} ({})", p.time, mode_name))
                    .await;
            }
            FfmpegEvent::LogEOF => break,
            FfmpegEvent::Log(level, content) => {
                if matches!(level, async_ffmpeg_sidecar::event::LogLevel::Error)
                    && content.contains("Error")
                {
                    conversion_error = Some(content);
                }
            }
            FfmpegEvent::Error(e) => {
                conversion_error = Some(e);
            }
            _ => {} // 忽略其他事件类型
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| format!("等待FFmpeg进程失败: {e}"))?;

    if !status.success() {
        let error_msg = conversion_error
            .unwrap_or_else(|| format!("FFmpeg退出码: {}", status.code().unwrap_or(-1)));
        return Err(format!("视频格式转换失败 ({mode_name}): {error_msg}"));
    }

    reporter
        .update(&format!("视频格式转换完成 100% ({mode_name})"))
        .await;
    Ok(())
}

// 尝试流复制转换（无损，速度快）
pub async fn try_stream_copy_conversion(
    source: &Path,
    dest: &Path,
    reporter: &ProgressReporter,
) -> Result<(), String> {
    reporter.update("正在转换视频格式... 0% (无损模式)").await;

    // 构建ffmpeg命令 - 流复制模式
    let mut cmd = tokio::process::Command::new(ffmpeg_path());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    cmd.args([
        "-i",
        &source.to_string_lossy(),
        "-c:v",
        "copy", // 直接复制视频流，零损失
        "-c:a",
        "copy", // 直接复制音频流，零损失
        "-avoid_negative_ts",
        "make_zero", // 修复时间戳问题
        "-movflags",
        "+faststart", // 优化web播放
        "-progress",
        "pipe:2", // 输出进度到stderr
        "-y",     // 覆盖输出文件
        &dest.to_string_lossy(),
    ]);

    execute_ffmpeg_conversion(cmd, reporter, "无损转换").await
}

// 高质量重编码转换（兼容性好，质量高）
pub async fn try_high_quality_conversion(
    source: &Path,
    dest: &Path,
    reporter: &ProgressReporter,
) -> Result<(), String> {
    reporter.update("正在转换视频格式... 0% (高质量模式)").await;

    // 构建ffmpeg命令 - 高质量重编码
    let mut cmd = tokio::process::Command::new(ffmpeg_path());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    cmd.args([
        "-i",
        &source.to_string_lossy(),
        "-c:v",
        "libx264", // H.264编码器
        "-preset",
        "slow", // 慢速预设，更好的压缩效率
        "-crf",
        "18", // 高质量设置 (18-23范围，越小质量越高)
        "-c:a",
        "aac", // AAC音频编码器
        "-b:a",
        "192k", // 高音频码率
        "-avoid_negative_ts",
        "make_zero", // 修复时间戳问题
        "-movflags",
        "+faststart", // 优化web播放
        "-progress",
        "pipe:2", // 输出进度到stderr
        "-y",     // 覆盖输出文件
        &dest.to_string_lossy(),
    ]);

    execute_ffmpeg_conversion(cmd, reporter, "高质量转换").await
}

// 带进度的视频格式转换函数（智能质量保持策略）
pub async fn convert_video_format(
    source: &Path,
    dest: &Path,
    reporter: &ProgressReporter,
) -> Result<(), String> {
    // 先尝试stream copy（无损转换），如果失败则使用高质量重编码
    match try_stream_copy_conversion(source, dest, reporter).await {
        Ok(()) => Ok(()),
        Err(stream_copy_error) => {
            reporter.update("流复制失败，使用高质量重编码模式...").await;
            log::warn!("Stream copy failed: {stream_copy_error}, falling back to re-encoding");
            try_high_quality_conversion(source, dest, reporter).await
        }
    }
}

/// Check if all videos have same encoding and resolution
pub async fn check_videos(video_paths: &[&Path]) -> bool {
    // check if all playlist paths exist
    let mut video_codec = "".to_owned();
    let mut audio_codec = "".to_owned();
    let mut width = 0;
    let mut height = 0;
    for video_path in video_paths.iter() {
        if !Path::new(video_path).exists() {
            continue;
        }
        let metadata = match extract_video_metadata(Path::new(video_path)).await {
            Ok(metadata) => metadata,
            Err(error) => {
                log::error!("Failed to extract video metadata: {error}");
                return false;
            }
        };

        // check video codec
        if !video_codec.is_empty() && metadata.video_codec != video_codec {
            log::error!("Video codec does not match: {}", video_path.display());
            return false;
        } else {
            video_codec = metadata.video_codec;
        }

        // check audio codec
        if !audio_codec.is_empty() && metadata.audio_codec != audio_codec {
            log::error!("Audio codec does not match: {}", video_path.display());
            return false;
        } else {
            audio_codec = metadata.audio_codec;
        }

        // check width
        if width > 0 && metadata.width != width {
            log::error!("Video width does not match: {}", video_path.display());
            return false;
        } else {
            width = metadata.width;
        }

        // check height
        if height > 0 && metadata.height != height {
            log::error!("Video height does not match: {}", video_path.display());
            return false;
        } else {
            height = metadata.height;
        }
    }

    true
}

// tests
#[cfg(test)]
mod tests {
    use super::*;

    // 测试 Range 结构体
    #[test]
    fn test_range_creation() {
        let range = Range {
            start: 10.0,
            end: 30.0,
        };
        assert_eq!(range.start, 10.0);
        assert_eq!(range.end, 30.0);
        assert_eq!(range.duration(), 20.0);
    }

    #[test]
    fn test_range_duration() {
        let range = Range {
            start: 0.0,
            end: 60.0,
        };
        assert_eq!(range.duration(), 60.0);

        let range2 = Range {
            start: 15.5,
            end: 45.5,
        };
        assert_eq!(range2.duration(), 30.0);
    }

    #[test]
    fn test_range_display() {
        let range = Range {
            start: 5.0,
            end: 25.0,
        };
        assert_eq!(range.to_string(), "[5, 25]");
    }

    #[test]
    fn test_range_edge_cases() {
        let zero_range = Range {
            start: 0.0,
            end: 0.0,
        };
        assert_eq!(zero_range.duration(), 0.0);

        let negative_start = Range {
            start: -5.0,
            end: 10.0,
        };
        assert_eq!(negative_start.duration(), 15.0);

        let large_range = Range {
            start: 1000.0,
            end: 2000.0,
        };
        assert_eq!(large_range.duration(), 1000.0);
    }

    // 测试视频元数据提取
    #[tokio::test]
    async fn test_extract_video_metadata() {
        let test_video = Path::new("tests/video/test.mp4");
        if test_video.exists() {
            let metadata = extract_video_metadata(test_video).await.unwrap();
            println!("metadata: {:?}", metadata);
            assert!(metadata.duration > 0.0);
            assert!(metadata.width > 0);
            assert!(metadata.height > 0);
        }
    }

    // 测试音频时长获取
    #[tokio::test]
    async fn test_get_audio_duration() {
        let test_audio = Path::new("tests/audio/test.wav");
        if test_audio.exists() {
            let duration = get_audio_duration(test_audio).await.unwrap();
            assert!(duration > 0);
        }
    }

    // 测试缩略图生成
    #[tokio::test]
    async fn test_generate_thumbnail() {
        let file = Path::new("tests/video/test.mp4");
        if file.exists() {
            let thumbnail_file = generate_thumbnail(file, 0.0).await.unwrap();
            assert!(thumbnail_file.exists());
            assert_eq!(thumbnail_file.extension().unwrap(), "jpg");
            // clean up
            let _ = std::fs::remove_file(thumbnail_file);
        }
    }

    // 测试 FFmpeg 版本检查
    #[tokio::test]
    async fn test_check_ffmpeg() {
        let result = check_ffmpeg().await;
        match result {
            Ok(version) => {
                assert!(!version.is_empty());
                // FFmpeg 版本字符串可能不包含 "ffmpeg" 这个词，所以检查是否包含数字
                assert!(version.chars().any(|c| c.is_ascii_digit()));
            }
            Err(_) => {
                // FFmpeg 可能没有安装，这是正常的
                println!("FFmpeg not available for testing");
            }
        }
    }

    // 测试通用 FFmpeg 命令
    #[tokio::test]
    async fn test_generic_ffmpeg_command() {
        let result = generic_ffmpeg_command(&["-version"]).await;
        match result {
            Ok(_output) => {
                // 输出可能为空或者不包含 "ffmpeg" 字符串，我们只检查函数能正常执行
                println!("FFmpeg command executed successfully");
            }
            Err(_) => {
                // FFmpeg 可能没有安装，这是正常的
                println!("FFmpeg not available for testing");
            }
        }
    }

    // 测试硬件加速能力探测
    #[tokio::test]
    async fn test_list_supported_hwaccels() {
        match super::hwaccel::list_supported_hwaccels().await {
            Ok(hwaccels) => {
                println!("hwaccels: {:?}", hwaccels);
                let mut sorted = hwaccels.clone();
                sorted.sort();
                sorted.dedup();
                assert_eq!(sorted.len(), hwaccels.len());
            }
            Err(_) => {
                println!("FFmpeg hardware acceleration query not available for testing");
            }
        }
    }

    // 测试字幕生成错误处理
    #[tokio::test]
    async fn test_generate_video_subtitle_errors() {
        let test_file = Path::new("tests/video/test.mp4");

        // 测试 Whisper 类型 - 模型未配置
        let result = generate_video_subtitle(
            None,
            test_file,
            "whisper",
            "",
            "",
            "",
            "",
            "whisper-1",
            "",
            "",
            "",
            "",
            "",
            "",
            "zh",
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Whisper model not configured"));

        // 测试 Whisper Online 类型 - API key 未配置
        let result = generate_video_subtitle(
            None,
            test_file,
            "whisper_online",
            "",
            "",
            "",
            "",
            "whisper-1",
            "",
            "",
            "",
            "",
            "",
            "",
            "zh",
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("API key not configured"));

        // 测试未知类型
        let result = generate_video_subtitle(
            None,
            test_file,
            "unknown_type",
            "",
            "",
            "",
            "",
            "whisper-1",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
        )
        .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Unknown subtitle generator type"));
    }

    // 测试路径构建函数
    #[test]
    fn test_ffmpeg_paths() {
        let ffmpeg_path = ffmpeg_path();
        let ffprobe_path = ffprobe_path();

        #[cfg(windows)]
        {
            assert_eq!(ffmpeg_path.extension().unwrap(), "exe");
            assert_eq!(ffprobe_path.extension().unwrap(), "exe");
        }

        #[cfg(not(windows))]
        {
            assert_eq!(ffmpeg_path.file_name().unwrap(), "ffmpeg");
            assert_eq!(ffprobe_path.file_name().unwrap(), "ffprobe");
        }
    }

    // 测试文件名和路径处理
    #[test]
    fn test_filename_processing() {
        let test_file = Path::new("tests/video/test.mp4");

        // 测试字幕文件名生成
        let subtitle_filename = format!(
            "{}{}",
            constants::PREFIX_SUBTITLE,
            test_file.file_name().unwrap().to_str().unwrap()
        );
        assert!(subtitle_filename.starts_with(constants::PREFIX_SUBTITLE));
        assert!(subtitle_filename.contains("test.mp4"));

        // 测试弹幕文件名生成
        let danmu_filename = format!(
            "{}{}",
            constants::PREFIX_DANMAKU,
            test_file.file_name().unwrap().to_str().unwrap()
        );
        assert!(danmu_filename.starts_with(constants::PREFIX_DANMAKU));
        assert!(danmu_filename.contains("test.mp4"));
    }

    // 测试音频分块目录结构
    #[test]
    fn test_audio_chunk_directory_structure() {
        let test_file = Path::new("tests/audio/test.wav");
        let output_path = test_file.with_extension("wav");
        let output_dir = output_path.parent().unwrap();
        let base_name = output_path.file_stem().unwrap().to_str().unwrap();
        let chunk_dir = output_dir.join(format!("{base_name}_chunks"));

        assert!(chunk_dir.to_string_lossy().contains("_chunks"));
        assert!(chunk_dir.to_string_lossy().contains("test"));
    }

    #[test]
    fn test_range_is_in_inside() {
        let r = Range {
            start: 1.0,
            end: 5.0,
        };
        assert!(r.is_in(3.0));
    }

    #[test]
    fn test_range_is_in_at_boundaries() {
        let r = Range {
            start: 1.0,
            end: 5.0,
        };
        assert!(r.is_in(1.0));
        assert!(r.is_in(5.0));
    }

    #[test]
    fn test_range_is_in_outside() {
        let r = Range {
            start: 1.0,
            end: 5.0,
        };
        assert!(!r.is_in(0.9));
        assert!(!r.is_in(5.1));
    }

    #[test]
    fn test_video_metadata_equality() {
        let m1 = VideoMetadata {
            duration: 10.0,
            width: 1920,
            height: 1080,
            video_codec: "h264".to_string(),
            audio_codec: "aac".to_string(),
        };
        let m2 = m1.clone();
        assert_eq!(m1, m2);
    }

    #[test]
    fn test_video_metadata_different_resolution() {
        let m1 = VideoMetadata {
            duration: 10.0,
            width: 1920,
            height: 1080,
            video_codec: "h264".to_string(),
            audio_codec: "aac".to_string(),
        };
        let m2 = VideoMetadata {
            duration: 10.0,
            width: 1280,
            height: 720,
            video_codec: "h264".to_string(),
            audio_codec: "aac".to_string(),
        };
        assert_ne!(m1, m2);
    }

    #[test]
    fn test_video_metadata_different_codec() {
        let m1 = VideoMetadata {
            duration: 10.0,
            width: 1920,
            height: 1080,
            video_codec: "h264".to_string(),
            audio_codec: "aac".to_string(),
        };
        let m2 = VideoMetadata {
            duration: 10.0,
            width: 1920,
            height: 1080,
            video_codec: "hevc".to_string(),
            audio_codec: "aac".to_string(),
        };
        assert_ne!(m1, m2);
    }
}
