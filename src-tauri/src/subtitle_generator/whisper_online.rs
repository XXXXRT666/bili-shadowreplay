use async_trait::async_trait;
use base64::Engine;
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use reqwest::{
    multipart::{Form, Part},
    Client,
};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    io::Read,
    path::{Path, PathBuf},
    process::Stdio,
    time::UNIX_EPOCH,
};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::HeaderValue, Message},
    MaybeTlsStream, WebSocketStream,
};
use uuid::Uuid;

use crate::{
    config::{AsrHotword, AsrHotwordConfig},
    progress::progress_reporter::ProgressReporterTrait,
    subtitle_generator::{GenerateResult, SubtitleGenerator, SubtitleGeneratorType},
};

#[derive(Debug, Clone)]
pub struct WhisperOnline {
    client: Client,
    api_url: String,
    api_key: Option<String>,
    prompt: Option<String>,
    model: String,
    hotword_vocabulary_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WhisperResponse {
    segments: Vec<WhisperSegment>,
}

#[derive(Debug, Deserialize)]
struct WhisperSegment {
    start: f64,
    end: f64,
    text: String,
}

pub async fn new(
    api_url: Option<&str>,
    api_key: Option<&str>,
    prompt: Option<&str>,
    model: Option<&str>,
    hotword_vocabulary_id: Option<&str>,
) -> Result<WhisperOnline, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // 5 minutes timeout
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let api_url = api_url.unwrap_or("https://api.openai.com/v1");

    Ok(WhisperOnline {
        client,
        api_url: api_url.trim_end_matches('/').to_string(),
        api_key: api_key.map(std::string::ToString::to_string),
        prompt: prompt.map(std::string::ToString::to_string),
        model: model.unwrap_or("whisper-1").to_string(),
        hotword_vocabulary_id: hotword_vocabulary_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(std::string::ToString::to_string),
    })
}

async fn connect_realtime_websocket(
    endpoint: &str,
    api_key: &str,
    provider: &str,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, String> {
    let mut last_error = String::new();
    const MAX_ATTEMPTS: usize = 3;

    for attempt in 1..=MAX_ATTEMPTS {
        let mut request = endpoint
            .into_client_request()
            .map_err(|e| format!("Failed to create websocket request: {e}"))?;
        request.headers_mut().insert(
            "Authorization",
            HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|e| format!("Invalid API key header: {e}"))?,
        );
        request
            .headers_mut()
            .insert("user-agent", HeaderValue::from_static("bili-shadowreplay"));

        log::info!("{provider}: websocket connect attempt {attempt}/{MAX_ATTEMPTS}");
        match connect_async(request).await {
            Ok((socket, _)) => {
                if attempt > 1 {
                    log::info!("{provider}: websocket connected after {attempt} attempts");
                }
                return Ok(socket);
            }
            Err(error) => {
                last_error = error.to_string();
                if attempt == MAX_ATTEMPTS {
                    break;
                }
                let delay_seconds = attempt as u64;
                log::warn!(
                    "{provider}: websocket connect attempt {attempt}/{MAX_ATTEMPTS} failed: {last_error}; retrying in {delay_seconds}s"
                );
                sleep(Duration::from_secs(delay_seconds)).await;
            }
        }
    }

    Err(format!(
        "Failed to connect {provider} after {MAX_ATTEMPTS} attempts: {last_error}"
    ))
}

fn format_srt_time(timestamp: f64) -> String {
    let hours = (timestamp / 3600.0).floor();
    let minutes = ((timestamp - hours * 3600.0) / 60.0).floor();
    let seconds = (timestamp - hours * 3600.0 - minutes * 60.0).floor();
    let milliseconds =
        ((timestamp - hours * 3600.0 - minutes * 60.0 - seconds) * 1000.0).floor() as u32;
    format!("{hours:02}:{minutes:02}:{seconds:02},{milliseconds:03}")
}

fn mime_type_for_audio_path(audio_path: &Path) -> &'static str {
    let file_extension = audio_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("wav");

    match file_extension.to_lowercase().as_str() {
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "flac" => "audio/flac",
        _ => "audio/wav",
    }
}

fn is_qwen_asr_flash_model(model: &str) -> bool {
    matches!(model, "qwen3-asr-flash-realtime" | "qwen-asr-realtime")
}

fn is_fun_asr_realtime_model(model: &str) -> bool {
    model == "fun-asr-realtime"
}

fn is_fun_asr_filetrans_model(model: &str) -> bool {
    matches!(model, "fun-asr-filetrans" | "fun-asr")
}

fn is_qwen_asr_filetrans_model(model: &str) -> bool {
    model == "qwen3-asr-flash-filetrans"
}

fn filetrans_api_model(model: &str) -> &str {
    if is_fun_asr_filetrans_model(model) {
        "fun-asr"
    } else if is_qwen_asr_filetrans_model(model) {
        "qwen3-asr-flash-filetrans"
    } else {
        model
    }
}

fn filetrans_enable_words(model: &str) -> bool {
    is_qwen_asr_filetrans_model(model)
}

fn hotword_target_model(model: &str) -> Option<&'static str> {
    if is_fun_asr_realtime_model(model) {
        Some("fun-asr-realtime")
    } else if is_fun_asr_filetrans_model(model) {
        Some("fun-asr")
    } else {
        None
    }
}

fn audio_file_fingerprint(audio_path: &Path) -> String {
    if let Ok(mut file) = std::fs::File::open(audio_path) {
        let mut hasher = Sha1::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            match file.read(&mut buffer) {
                Ok(0) => return format!("{:x}", hasher.finalize()),
                Ok(bytes_read) => hasher.update(&buffer[..bytes_read]),
                Err(error) => {
                    log::warn!(
                        "ASR audio fingerprint: failed to hash {}: {}; falling back to metadata",
                        audio_path.display(),
                        error
                    );
                    break;
                }
            }
        }
    }

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    audio_path.to_string_lossy().hash(&mut hasher);
    if let Ok(metadata) = std::fs::metadata(audio_path) {
        metadata.len().hash(&mut hasher);
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                duration.as_secs().hash(&mut hasher);
                duration.subsec_nanos().hash(&mut hasher);
            }
        }
    }
    format!("{:016x}", hasher.finish())
}

fn dashscope_api_base(api_url: &str) -> String {
    let api_url = api_url.trim().trim_end_matches('/');
    if api_url.is_empty()
        || api_url == "https://api.openai.com/v1"
        || api_url == "https://dashscope.aliyuncs.com/compatible-mode/v1"
        || api_url.starts_with("wss://")
    {
        return "https://dashscope.aliyuncs.com/api/v1".to_string();
    }
    if let Some((base, _)) = api_url.split_once("/services/audio/asr/transcription") {
        return base.to_string();
    }
    api_url.to_string()
}

fn dashscope_submit_endpoint(api_url: &str) -> String {
    let api_url = api_url.trim().trim_end_matches('/');
    if api_url.contains("/services/audio/asr/transcription") {
        api_url.to_string()
    } else {
        format!(
            "{}/services/audio/asr/transcription",
            dashscope_api_base(api_url)
        )
    }
}

fn dashscope_task_endpoint(api_url: &str, task_id: &str) -> String {
    format!("{}/tasks/{task_id}", dashscope_api_base(api_url))
}

fn dashscope_customization_endpoint(api_url: &str) -> String {
    format!(
        "{}/services/audio/asr/customization",
        dashscope_api_base(api_url)
    )
}

fn dashscope_uploads_endpoint(api_url: &str) -> String {
    format!("{}/uploads", dashscope_api_base(api_url))
}

fn qwen_realtime_endpoint(api_url: &str, model: &str) -> String {
    let base_url = if api_url.trim().is_empty()
        || api_url == "https://api.openai.com/v1"
        || api_url == "https://dashscope.aliyuncs.com/compatible-mode/v1"
    {
        "wss://dashscope.aliyuncs.com/api-ws/v1/realtime"
    } else {
        api_url.trim_end_matches('/')
    };

    if base_url.contains("?model=") || base_url.contains("&model=") {
        base_url.to_string()
    } else {
        let separator = if base_url.contains('?') { "&" } else { "?" };
        format!("{base_url}{separator}model={model}")
    }
}

fn fun_asr_realtime_endpoint(api_url: &str) -> String {
    if api_url.trim().is_empty()
        || api_url == "https://api.openai.com/v1"
        || api_url == "https://dashscope.aliyuncs.com/compatible-mode/v1"
    {
        "wss://dashscope.aliyuncs.com/api-ws/v1/inference".to_string()
    } else {
        api_url.trim_end_matches('/').to_string()
    }
}

async fn extract_pcm16_mono_16k(audio_path: &Path) -> Result<Vec<u8>, String> {
    log::info!(
        "ASR realtime: extracting PCM audio path={}",
        audio_path.display()
    );
    let output = tokio::process::Command::new("ffmpeg")
        .args(["-i", audio_path.to_str().unwrap_or_default()])
        .args(["-vn", "-ac", "1", "-ar", "16000"])
        .args(["-f", "s16le", "-acodec", "pcm_s16le"])
        .arg("pipe:1")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to extract PCM audio: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to extract PCM audio: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    if output.stdout.is_empty() {
        return Err("Failed to extract PCM audio: output is empty".to_string());
    }
    log::info!(
        "ASR realtime: extracted PCM bytes={} duration_estimate={:.3}s",
        output.stdout.len(),
        output.stdout.len() as f64 / 32_000.0
    );
    Ok(output.stdout)
}

fn qwen_realtime_session_update(language_hint: &str) -> serde_json::Value {
    let mut input_audio_transcription = serde_json::Map::new();
    if !language_hint.trim().is_empty() && language_hint != "auto" {
        input_audio_transcription.insert("language".to_string(), serde_json::json!(language_hint));
    }

    serde_json::json!({
        "event_id": format!("event_{}", Uuid::new_v4()),
        "type": "session.update",
        "session": {
            "input_audio_format": "pcm",
            "sample_rate": 16000,
            "input_audio_transcription": input_audio_transcription,
            "turn_detection": {
                "type": "server_vad",
                "threshold": 0.0,
                "silence_duration_ms": 400,
            },
        },
    })
}

const SUBTITLE_REFLOW_LINE_UNITS: f64 = 36.0;
const SUBTITLE_REFLOW_MAX_LINES: usize = 1;
const SUBTITLE_REFLOW_MAX_DURATION_MS: i64 = 6_000;
const SUBTITLE_REFLOW_MIN_DURATION_MS: i64 = 700;

fn subtitle_char_width(ch: char) -> f64 {
    if ch.is_ascii() {
        if ch.is_ascii_whitespace() {
            0.5
        } else {
            1.0
        }
    } else {
        2.0
    }
}

fn subtitle_text_width(text: &str) -> f64 {
    text.chars().map(subtitle_char_width).sum()
}

fn subtitle_break_priority(text: &str) -> u8 {
    let trimmed = text.trim_end();
    let Some(ch) = trimmed.chars().last() else {
        return 0;
    };
    match ch {
        '。' | '！' | '？' | '!' | '?' => 3,
        '，' | ',' | '、' | '；' | ';' | '：' | ':' => 2,
        ' ' | '\t' => 1,
        _ => 0,
    }
}

fn make_fallback_timed_tokens(sentence: &RealtimeSentence) -> Vec<TimedSubtitleToken> {
    let chars = sentence.text.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return Vec::new();
    }
    let duration = (sentence.end_ms - sentence.begin_ms).max(chars.len() as i64);
    chars
        .iter()
        .enumerate()
        .map(|(index, ch)| {
            let begin_ms =
                sentence.begin_ms + duration.saturating_mul(index as i64) / chars.len() as i64;
            let end_ms =
                sentence.begin_ms + duration.saturating_mul(index as i64 + 1) / chars.len() as i64;
            TimedSubtitleToken {
                begin_ms,
                end_ms: end_ms.max(begin_ms + 1),
                text: ch.to_string(),
            }
        })
        .collect()
}

fn normalize_timed_tokens(sentence: &RealtimeSentence) -> Vec<TimedSubtitleToken> {
    let tokens = if sentence.tokens.is_empty() {
        make_fallback_timed_tokens(sentence)
    } else {
        sentence.tokens.clone()
    };
    tokens
        .into_iter()
        .filter(|token| !token.text.trim().is_empty() && token.end_ms > token.begin_ms)
        .collect()
}

fn timed_tokens_from_json_words(words: Option<&serde_json::Value>) -> Vec<TimedSubtitleToken> {
    let Some(words) = words.and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };

    words
        .iter()
        .filter_map(|word| {
            let text = word
                .get("text")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let punctuation = word
                .get("punctuation")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let token_text = format!("{text}{punctuation}");
            let begin_ms = word
                .get("begin_time")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            let end_ms = word
                .get("end_time")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(begin_ms);
            if token_text.trim().is_empty() || end_ms <= begin_ms {
                None
            } else {
                Some(TimedSubtitleToken {
                    begin_ms,
                    end_ms,
                    text: token_text,
                })
            }
        })
        .collect()
}

fn timed_tokens_from_filetrans_words(
    words: Option<Vec<FileTranscriptionWord>>,
) -> Vec<TimedSubtitleToken> {
    words
        .unwrap_or_default()
        .into_iter()
        .filter_map(|word| {
            let token_text = format!("{}{}", word.text, word.punctuation.unwrap_or_default());
            if token_text.trim().is_empty() || word.end_time <= word.begin_time {
                None
            } else {
                Some(TimedSubtitleToken {
                    begin_ms: word.begin_time,
                    end_ms: word.end_time,
                    text: token_text,
                })
            }
        })
        .collect()
}

fn align_timed_tokens_to_sentence_text(
    sentence_text: &str,
    tokens: Vec<TimedSubtitleToken>,
) -> Vec<TimedSubtitleToken> {
    let source = sentence_text.trim();
    let mut cursor = 0_usize;
    tokens
        .into_iter()
        .map(|mut token| {
            let token_text = token.text.trim_start();
            if !token_text.is_empty() && cursor < source.len() {
                if let Some(relative_start) = source[cursor..].find(token_text) {
                    let prefix_start = cursor;
                    let token_start = cursor + relative_start;
                    let token_end = token_start + token_text.len();
                    let prefix = &source[prefix_start..token_start];
                    if !prefix.is_empty() {
                        token.text = format!("{prefix}{token_text}");
                    } else {
                        token.text = token_text.to_string();
                    }
                    cursor = token_end;
                }
            }
            token
        })
        .collect()
}

fn build_reflow_sentence(
    tokens: &[TimedSubtitleToken],
    start: usize,
    end: usize,
) -> RealtimeSentence {
    let begin_ms = tokens[start].begin_ms;
    let mut end_ms = tokens[end - 1].end_ms;
    if end_ms - begin_ms < SUBTITLE_REFLOW_MIN_DURATION_MS {
        end_ms = begin_ms + SUBTITLE_REFLOW_MIN_DURATION_MS;
    }
    let raw_text = tokens[start..end]
        .iter()
        .map(|token| token.text.as_str())
        .collect::<String>();
    RealtimeSentence {
        begin_ms,
        end_ms,
        text: wrap_subtitle_text(&raw_text),
        tokens: Vec::new(),
    }
}

fn split_sentence_for_subtitles(sentence: &RealtimeSentence) -> Vec<RealtimeSentence> {
    if sentence.text.trim().is_empty() || sentence.end_ms <= sentence.begin_ms {
        return Vec::new();
    }

    let tokens = normalize_timed_tokens(sentence);
    if tokens.len() <= 1 {
        return vec![RealtimeSentence {
            begin_ms: sentence.begin_ms,
            end_ms: sentence
                .end_ms
                .max(sentence.begin_ms + SUBTITLE_REFLOW_MIN_DURATION_MS),
            text: wrap_subtitle_text(&sentence.text),
            tokens: Vec::new(),
        }];
    }

    let max_units = SUBTITLE_REFLOW_LINE_UNITS * SUBTITLE_REFLOW_MAX_LINES as f64;
    let mut result = Vec::new();
    let mut start = 0_usize;
    while start < tokens.len() {
        let mut end = start;
        let mut units = 0.0_f64;
        let mut best_break: Option<(usize, u8)> = None;
        while end < tokens.len() {
            let next_units = subtitle_text_width(&tokens[end].text);
            let next_total = units + next_units;
            let next_duration = tokens[end].end_ms - tokens[start].begin_ms;
            let would_overflow = end > start
                && (next_total > max_units || next_duration > SUBTITLE_REFLOW_MAX_DURATION_MS);
            if would_overflow {
                break;
            }
            units = next_total;
            let priority = subtitle_break_priority(&tokens[end].text);
            if priority > 0 {
                let replace = best_break
                    .map(|(_, best_priority)| priority >= best_priority)
                    .unwrap_or(true);
                if replace {
                    best_break = Some((end + 1, priority));
                }
            }
            end += 1;
        }

        if end == start {
            end = start + 1;
        } else if end < tokens.len() {
            if let Some((break_end, _)) = best_break {
                if break_end > start {
                    end = break_end;
                }
            }
        }

        result.push(build_reflow_sentence(&tokens, start, end));
        start = end;
    }
    result
}

fn wrap_subtitle_text(text: &str) -> String {
    let text = text.trim();
    if subtitle_text_width(text) <= SUBTITLE_REFLOW_LINE_UNITS {
        return text.to_string();
    }

    let mut lines = Vec::<String>::new();
    let mut current = String::new();
    let mut current_units = 0.0_f64;
    let mut last_break_byte = None::<usize>;
    let mut last_break_priority = 0_u8;

    for ch in text.chars() {
        let ch_units = subtitle_char_width(ch);
        if !current.is_empty() && current_units + ch_units > SUBTITLE_REFLOW_LINE_UNITS {
            if let Some(byte_index) = last_break_byte {
                let line = current[..byte_index].trim().to_string();
                let rest = current[byte_index..].trim_start().to_string();
                if !line.is_empty() {
                    lines.push(line);
                }
                current = rest;
                current_units = subtitle_text_width(&current);
            } else {
                lines.push(current.trim().to_string());
                current.clear();
                current_units = 0.0;
            }
            last_break_byte = None;
            last_break_priority = 0;
        }

        current.push(ch);
        current_units += ch_units;
        let priority = subtitle_break_priority(&current);
        if priority > 0 && priority >= last_break_priority {
            last_break_byte = Some(current.len());
            last_break_priority = priority;
        }
    }

    if !current.trim().is_empty() {
        lines.push(current.trim().to_string());
    }

    if lines.len() <= SUBTITLE_REFLOW_MAX_LINES {
        lines.join("\n")
    } else {
        let mut compacted = lines[..SUBTITLE_REFLOW_MAX_LINES - 1].to_vec();
        compacted.push(lines[SUBTITLE_REFLOW_MAX_LINES - 1..].join(""));
        compacted.join("\n")
    }
}

#[derive(Debug, Clone)]
struct RealtimeSentence {
    begin_ms: i64,
    end_ms: i64,
    text: String,
    tokens: Vec<TimedSubtitleToken>,
}

#[derive(Debug, Clone)]
struct TimedSubtitleToken {
    begin_ms: i64,
    end_ms: i64,
    text: String,
}

#[derive(Debug, Clone)]
pub struct RealtimeSrtWriter {
    path: PathBuf,
    offset_ms: i64,
    next_index: usize,
}

impl RealtimeSrtWriter {
    pub fn new(path: &Path, offset_seconds: u64, next_index: usize) -> Self {
        Self {
            path: path.to_path_buf(),
            offset_ms: offset_seconds as i64 * 1000,
            next_index,
        }
    }

    async fn append_sentence(
        &self,
        local_index: usize,
        begin_ms: i64,
        end_ms: i64,
        text: &str,
    ) -> Result<(), String> {
        if text.trim().is_empty() || end_ms <= begin_ms {
            return Ok(());
        }

        let global_begin_ms = begin_ms.saturating_add(self.offset_ms);
        let global_end_ms = end_ms.saturating_add(self.offset_ms);
        let entry = format!(
            "{}\n{} --> {}\n{}\n\n",
            self.next_index + local_index,
            format_srt_time(global_begin_ms as f64 / 1000.0),
            format_srt_time(global_end_ms as f64 / 1000.0),
            text.trim()
        );

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await
            .map_err(|e| {
                format!(
                    "Failed to open realtime partial SRT {}: {e}",
                    self.path.display()
                )
            })?;
        file.write_all(entry.as_bytes()).await.map_err(|e| {
            format!(
                "Failed to write realtime partial SRT {}: {e}",
                self.path.display()
            )
        })?;
        file.flush().await.map_err(|e| {
            format!(
                "Failed to flush realtime partial SRT {}: {e}",
                self.path.display()
            )
        })?;
        log::info!(
            "Realtime partial SRT: appended path={} index={} {} --> {} text={}",
            self.path.display(),
            self.next_index + local_index,
            format_srt_time(global_begin_ms as f64 / 1000.0),
            format_srt_time(global_end_ms as f64 / 1000.0),
            text.trim()
        );
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct DashScopeSubmitResponse {
    output: DashScopeTaskOutput,
}

#[derive(Debug, Deserialize)]
struct DashScopeTaskResponse {
    output: DashScopeTaskOutput,
}

#[derive(Debug, Deserialize)]
struct DashScopeTaskOutput {
    task_id: Option<String>,
    task_status: String,
    result: Option<DashScopeQwenTaskResult>,
    results: Option<Vec<DashScopeFunTaskResult>>,
    code: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DashScopeQwenTaskResult {
    transcription_url: String,
}

#[derive(Debug, Deserialize)]
struct DashScopeFunTaskResult {
    transcription_url: Option<String>,
    subtask_status: String,
    code: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DashScopeHotwordResponse {
    output: DashScopeHotwordOutput,
}

#[derive(Debug, Deserialize)]
struct DashScopeHotwordOutput {
    vocabulary_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DashScopeUploadPolicyResponse {
    data: DashScopeUploadPolicy,
}

#[derive(Debug, Deserialize)]
struct DashScopeUploadPolicy {
    upload_dir: String,
    upload_host: String,
    oss_access_key_id: String,
    signature: String,
    policy: String,
    x_oss_object_acl: String,
    x_oss_forbid_overwrite: String,
}

#[derive(Debug, Deserialize)]
struct FileTranscriptionResult {
    transcripts: Vec<FileTranscriptionTranscript>,
}

#[derive(Debug, Deserialize)]
struct FileTranscriptionTranscript {
    sentences: Vec<FileTranscriptionSentence>,
}

#[derive(Debug, Deserialize)]
struct FileTranscriptionSentence {
    begin_time: i64,
    end_time: i64,
    text: String,
    words: Option<Vec<FileTranscriptionWord>>,
}

#[derive(Debug, Deserialize)]
struct FileTranscriptionWord {
    begin_time: i64,
    end_time: i64,
    text: String,
    punctuation: Option<String>,
}

#[derive(Debug)]
struct OssUploadResult {
    file_url: String,
    object_key: String,
    audio_fingerprint: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct FiletransTaskCache {
    entries: Vec<FiletransTaskCacheEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct FiletransTaskCacheEntry {
    model: String,
    language_hint: String,
    #[serde(default)]
    vocabulary_id: String,
    #[serde(default)]
    enable_words: bool,
    #[serde(default)]
    dashscope_base: String,
    #[serde(default)]
    oss_bucket: String,
    #[serde(default)]
    oss_object_key: String,
    audio_fingerprint: String,
    task_id: String,
    updated_at: i64,
}

async fn upload_audio_to_dashscope_temp_oss(
    client: &Client,
    api_url: &str,
    api_key: &str,
    model: &str,
    audio_path: &Path,
) -> Result<OssUploadResult, String> {
    let api_model = filetrans_api_model(model);
    let policy_endpoint = dashscope_uploads_endpoint(api_url);
    log::info!(
        "DashScope temp OSS upload: requesting policy endpoint={} model={} audio={}",
        policy_endpoint,
        api_model,
        audio_path.display()
    );
    let policy_response = client
        .get(&policy_endpoint)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .query(&[("action", "getPolicy"), ("model", api_model)])
        .send()
        .await
        .map_err(|e| format!("Failed to get DashScope upload policy: {e}"))?;

    let status = policy_response.status();
    let response_text = policy_response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "DashScope upload policy failed with status {status}: {response_text}"
        ));
    }
    let policy: DashScopeUploadPolicyResponse = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse DashScope upload policy: {e}; {response_text}"))?;

    let file_name = audio_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(std::string::ToString::to_string)
        .unwrap_or_else(|| "audio.mp3".to_string());
    let object_key = format!(
        "{}/{}",
        policy.data.upload_dir.trim_end_matches('/'),
        file_name
    );
    let audio_fingerprint = audio_file_fingerprint(audio_path);
    let audio_data = fs::read(audio_path)
        .await
        .map_err(|e| format!("Failed to read audio for DashScope upload: {e}"))?;
    let mime_type = mime_type_for_audio_path(audio_path);
    let audio_part = Part::bytes(audio_data)
        .mime_str(mime_type)
        .map_err(|e| format!("Failed to set DashScope upload MIME type: {e}"))?
        .file_name(file_name);
    let upload_host = policy.data.upload_host.clone();
    let form = Form::new()
        .text("OSSAccessKeyId", policy.data.oss_access_key_id.clone())
        .text("Signature", policy.data.signature.clone())
        .text("policy", policy.data.policy.clone())
        .text("x-oss-object-acl", policy.data.x_oss_object_acl.clone())
        .text(
            "x-oss-forbid-overwrite",
            policy.data.x_oss_forbid_overwrite.clone(),
        )
        .text("key", object_key.clone())
        .text("success_action_status", "200")
        .part("file", audio_part);

    log::info!(
        "DashScope temp OSS upload: uploading model={} object={} host={}",
        api_model,
        object_key,
        upload_host
    );
    let upload_response = client
        .post(&upload_host)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Failed to upload audio to DashScope temp OSS: {e}"))?;
    let upload_status = upload_response.status();
    let upload_text = upload_response.text().await.unwrap_or_default();
    if !upload_status.is_success() {
        return Err(format!(
            "DashScope temp OSS upload failed with status {upload_status}: {upload_text}"
        ));
    }

    let file_url = format!("oss://{object_key}");
    log::info!(
        "DashScope temp OSS upload: complete model={} object={} url={}",
        api_model,
        object_key,
        file_url
    );
    Ok(OssUploadResult {
        file_url,
        object_key,
        audio_fingerprint,
    })
}

fn filetrans_task_cache_path(audio_path: &Path) -> PathBuf {
    let file_name = audio_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("audio");
    audio_path.with_file_name(format!("{file_name}.task-cache.json"))
}

async fn load_filetrans_task_cache(audio_path: &Path) -> FiletransTaskCache {
    let cache_path = filetrans_task_cache_path(audio_path);
    let Ok(contents) = fs::read_to_string(&cache_path).await else {
        return FiletransTaskCache::default();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

async fn save_filetrans_task_cache(
    audio_path: &Path,
    cache: &FiletransTaskCache,
) -> Result<(), String> {
    let cache_path = filetrans_task_cache_path(audio_path);
    let contents = serde_json::to_string_pretty(cache)
        .map_err(|e| format!("Failed to serialize filetrans task cache: {e}"))?;
    fs::write(&cache_path, contents).await.map_err(|e| {
        format!(
            "Failed to write filetrans task cache {}: {e}",
            cache_path.display()
        )
    })
}

fn filetrans_task_cache_entry(
    api_url: &str,
    model: &str,
    language_hint: &str,
    vocabulary_id: Option<&str>,
    upload: &OssUploadResult,
    task_id: String,
) -> FiletransTaskCacheEntry {
    FiletransTaskCacheEntry {
        model: model.to_string(),
        language_hint: language_hint.to_string(),
        vocabulary_id: vocabulary_id.unwrap_or_default().trim().to_string(),
        enable_words: filetrans_enable_words(model),
        dashscope_base: dashscope_api_base(api_url),
        oss_bucket: "dashscope-temporary".to_string(),
        oss_object_key: upload.object_key.clone(),
        audio_fingerprint: upload.audio_fingerprint.clone(),
        task_id,
        updated_at: Utc::now().timestamp(),
    }
}

fn find_cached_filetrans_task<'a>(
    cache: &'a FiletransTaskCache,
    api_url: &str,
    model: &str,
    language_hint: &str,
    vocabulary_id: Option<&str>,
    audio_fingerprint: &str,
) -> Option<&'a FiletransTaskCacheEntry> {
    let dashscope_base = dashscope_api_base(api_url);
    let vocabulary_id = vocabulary_id.unwrap_or_default().trim();
    cache.entries.iter().find(|entry| {
        entry.model == model
            && entry.language_hint == language_hint
            && entry.vocabulary_id == vocabulary_id
            && entry.enable_words == filetrans_enable_words(model)
            && entry.dashscope_base == dashscope_base
            && entry.audio_fingerprint == audio_fingerprint
    })
}

async fn remember_filetrans_task(
    audio_path: &Path,
    api_url: &str,
    model: &str,
    language_hint: &str,
    vocabulary_id: Option<&str>,
    upload: &OssUploadResult,
    task_id: String,
) -> Result<(), String> {
    let mut cache = load_filetrans_task_cache(audio_path).await;
    let entry = filetrans_task_cache_entry(
        api_url,
        model,
        language_hint,
        vocabulary_id,
        upload,
        task_id,
    );
    cache.entries.retain(|existing| {
        !(existing.model == entry.model
            && existing.language_hint == entry.language_hint
            && existing.vocabulary_id == entry.vocabulary_id
            && existing.enable_words == entry.enable_words
            && existing.dashscope_base == entry.dashscope_base
            && existing.audio_fingerprint == entry.audio_fingerprint)
    });
    cache.entries.push(entry);
    save_filetrans_task_cache(audio_path, &cache).await
}

fn normalize_hotword_prefix(prefix: &str) -> String {
    let normalized = prefix
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .take(10)
        .collect::<String>();
    if normalized.is_empty() {
        "bsrasr".to_string()
    } else {
        normalized
    }
}

fn normalize_asr_hotwords(words: &[AsrHotword]) -> Vec<AsrHotword> {
    let mut normalized = Vec::new();
    for word in words {
        let text = word.text.trim();
        if text.is_empty() {
            continue;
        }
        let lang = word.lang.trim();
        normalized.push(AsrHotword {
            text: text.to_string(),
            weight: word.weight.clamp(1, 5),
            lang: if lang.is_empty() || lang == "auto" {
                String::new()
            } else {
                lang.to_string()
            },
        });
    }
    normalized
}

fn hotword_vocabulary_payload(words: &[AsrHotword]) -> Vec<serde_json::Value> {
    words
        .iter()
        .map(|word| {
            let mut item = serde_json::json!({
                "text": word.text,
                "weight": word.weight,
            });
            if !word.lang.trim().is_empty() {
                item["lang"] = serde_json::json!(word.lang.trim());
            }
            item
        })
        .collect()
}

fn asr_hotword_signature(prefix: &str, target_model: &str, words: &[AsrHotword]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(prefix.as_bytes());
    hasher.update(b"\n");
    hasher.update(target_model.as_bytes());
    for word in words {
        hasher.update(b"\n");
        hasher.update(word.text.as_bytes());
        hasher.update(b"\t");
        hasher.update(word.weight.to_string().as_bytes());
        hasher.update(b"\t");
        hasher.update(word.lang.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

async fn post_hotword_customization(
    client: &Client,
    api_url: &str,
    api_key: &str,
    payload: serde_json::Value,
) -> Result<String, String> {
    let endpoint = dashscope_customization_endpoint(api_url);
    let response = client
        .post(&endpoint)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to sync ASR hotwords: {e}"))?;

    let status = response.status();
    let response_text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "ASR hotword sync failed with status {status}: {response_text}"
        ));
    }
    Ok(response_text)
}

async fn create_hotword_vocabulary(
    client: &Client,
    api_url: &str,
    api_key: &str,
    target_model: &str,
    prefix: &str,
    words: &[AsrHotword],
) -> Result<String, String> {
    let payload = serde_json::json!({
        "model": "speech-biasing",
        "input": {
            "action": "create_vocabulary",
            "target_model": target_model,
            "prefix": prefix,
            "vocabulary": hotword_vocabulary_payload(words),
        },
    });
    log::info!(
        "ASR hotwords: creating vocabulary target_model={} prefix={} words={}",
        target_model,
        prefix,
        words.len()
    );
    let response_text = post_hotword_customization(client, api_url, api_key, payload).await?;
    let response: DashScopeHotwordResponse = serde_json::from_str(&response_text).map_err(|e| {
        format!("Failed to parse ASR hotword create response: {e}; {response_text}")
    })?;
    response
        .output
        .vocabulary_id
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| {
            format!("ASR hotword create response missing vocabulary_id: {response_text}")
        })
}

async fn update_hotword_vocabulary(
    client: &Client,
    api_url: &str,
    api_key: &str,
    vocabulary_id: &str,
    words: &[AsrHotword],
) -> Result<(), String> {
    let payload = serde_json::json!({
        "model": "speech-biasing",
        "input": {
            "action": "update_vocabulary",
            "vocabulary_id": vocabulary_id,
            "vocabulary": hotword_vocabulary_payload(words),
        },
    });
    log::info!(
        "ASR hotwords: updating vocabulary_id={} words={}",
        vocabulary_id,
        words.len()
    );
    post_hotword_customization(client, api_url, api_key, payload).await?;
    Ok(())
}

async fn delete_hotword_vocabulary(
    client: &Client,
    api_url: &str,
    api_key: &str,
    vocabulary_id: &str,
) -> Result<(), String> {
    let payload = serde_json::json!({
        "model": "speech-biasing",
        "input": {
            "action": "delete_vocabulary",
            "vocabulary_id": vocabulary_id,
        },
    });
    log::info!("ASR hotwords: deleting vocabulary_id={vocabulary_id}");
    post_hotword_customization(client, api_url, api_key, payload).await?;
    Ok(())
}

pub async fn sync_asr_hotwords(
    api_url: &str,
    api_key: &str,
    model: &str,
    hotwords: &AsrHotwordConfig,
) -> Result<AsrHotwordConfig, String> {
    let target_model = hotword_target_model(model);
    let prefix = normalize_hotword_prefix(&hotwords.prefix);
    let words = normalize_asr_hotwords(&hotwords.words);
    let mut next = hotwords.clone();
    next.prefix = prefix.clone();
    next.words = words.clone();

    if words.is_empty() {
        if !hotwords.vocabulary_id.trim().is_empty() && !api_key.trim().is_empty() {
            let client = Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .map_err(|e| format!("Failed to create HTTP client: {e}"))?;
            if let Err(error) =
                delete_hotword_vocabulary(&client, api_url, api_key, &hotwords.vocabulary_id).await
            {
                log::warn!(
                    "ASR hotwords: failed to delete vocabulary_id={}: {}",
                    hotwords.vocabulary_id,
                    error
                );
            }
        }
        next.vocabulary_id.clear();
        next.vocabulary_signature.clear();
        next.target_model.clear();
        return Ok(next);
    }

    let Some(target_model) = target_model else {
        log::warn!("ASR hotwords: model={model} does not support vocabulary_id; hotwords ignored");
        next.vocabulary_id.clear();
        next.vocabulary_signature.clear();
        next.target_model.clear();
        return Ok(next);
    };

    if api_key.trim().is_empty() {
        return Err("API key not configured".to_string());
    }

    let signature = asr_hotword_signature(&prefix, target_model, &words);
    if hotwords.target_model == target_model
        && hotwords.vocabulary_signature == signature
        && !hotwords.vocabulary_id.trim().is_empty()
    {
        log::info!(
            "ASR hotwords: vocabulary already synced target_model={} vocabulary_id={} words={}",
            target_model,
            hotwords.vocabulary_id,
            words.len()
        );
        next.target_model = target_model.to_string();
        return Ok(next);
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let mut vocabulary_id = hotwords.vocabulary_id.trim().to_string();
    if !vocabulary_id.is_empty() && hotwords.target_model == target_model {
        match update_hotword_vocabulary(&client, api_url, api_key, &vocabulary_id, &words).await {
            Ok(()) => {}
            Err(error) => {
                log::warn!(
                    "ASR hotwords: failed to update vocabulary_id={}: {}; creating a new one",
                    vocabulary_id,
                    error
                );
                vocabulary_id.clear();
            }
        }
    } else {
        vocabulary_id.clear();
    }

    if vocabulary_id.is_empty() {
        vocabulary_id =
            create_hotword_vocabulary(&client, api_url, api_key, target_model, &prefix, &words)
                .await?;
    }

    next.vocabulary_id = vocabulary_id;
    next.vocabulary_signature = signature;
    next.target_model = target_model.to_string();
    log::info!(
        "ASR hotwords: synced target_model={} vocabulary_id={} words={}",
        next.target_model,
        next.vocabulary_id,
        next.words.len()
    );
    Ok(next)
}

async fn submit_filetrans_task(
    client: &Client,
    api_url: &str,
    api_key: &str,
    model: &str,
    file_url: &str,
    language_hint: &str,
    hotword_vocabulary_id: Option<&str>,
) -> Result<String, String> {
    let endpoint = dashscope_submit_endpoint(api_url);
    let mut parameters = serde_json::json!({
        "channel_id": [0],
        "enable_itn": false,
    });
    if let Some(vocabulary_id) = hotword_vocabulary_id
        .map(str::trim)
        .filter(|vocabulary_id| !vocabulary_id.is_empty())
    {
        parameters["vocabulary_id"] = serde_json::json!(vocabulary_id);
    }
    if filetrans_enable_words(model) {
        parameters["enable_words"] = serde_json::json!(true);
    }
    if !language_hint.trim().is_empty() && language_hint != "auto" {
        if is_fun_asr_filetrans_model(model) {
            parameters["language_hints"] = serde_json::json!([language_hint]);
        } else {
            parameters["language"] = serde_json::json!(language_hint);
        }
    }

    let payload = if is_fun_asr_filetrans_model(model) {
        serde_json::json!({
            "model": "fun-asr",
            "input": {
                "file_urls": [file_url],
            },
            "parameters": parameters,
        })
    } else {
        serde_json::json!({
            "model": "qwen3-asr-flash-filetrans",
            "input": {
                "file_url": file_url,
            },
            "parameters": parameters,
        })
    };

    log::info!("Filetrans ASR: submitting model={model} endpoint={endpoint}");
    let mut request = client
        .post(endpoint)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .header("X-DashScope-Async", "enable")
        .json(&payload);
    if file_url.trim_start().starts_with("oss://") {
        request = request.header("X-DashScope-OssResourceResolve", "enable");
    }
    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to submit filetrans ASR task: {e}"))?;

    let status = response.status();
    let response_text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "Filetrans ASR submit failed with status {status}: {response_text}"
        ));
    }
    let response: DashScopeSubmitResponse = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse filetrans submit response: {e}; {response_text}"))?;
    response
        .output
        .task_id
        .ok_or_else(|| format!("Filetrans ASR submit response missing task_id: {response_text}"))
}

async fn poll_filetrans_result_url(
    client: &Client,
    api_url: &str,
    api_key: &str,
    model: &str,
    task_id: &str,
) -> Result<String, String> {
    let endpoint = dashscope_task_endpoint(api_url, task_id);
    for attempt in 1..=300 {
        let response = client
            .get(&endpoint)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("X-DashScope-Async", "enable")
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| format!("Failed to query filetrans ASR task: {e}"))?;

        let status = response.status();
        let response_text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!(
                "Filetrans ASR query failed with status {status}: {response_text}"
            ));
        }
        let response: DashScopeTaskResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                format!("Failed to parse filetrans query response: {e}; {response_text}")
            })?;

        match response.output.task_status.as_str() {
            "SUCCEEDED" => {
                if is_fun_asr_filetrans_model(model) {
                    let result = response
                        .output
                        .results
                        .and_then(|results| results.into_iter().next())
                        .ok_or_else(|| "Fun-ASR filetrans result missing results".to_string())?;
                    if result.subtask_status != "SUCCEEDED" {
                        return Err(format!(
                            "Fun-ASR filetrans subtask failed: {} {}",
                            result.code.unwrap_or_default(),
                            result.message.unwrap_or_default()
                        ));
                    }
                    return result.transcription_url.ok_or_else(|| {
                        "Fun-ASR filetrans result missing transcription_url".to_string()
                    });
                }
                return response
                    .output
                    .result
                    .map(|result| result.transcription_url)
                    .ok_or_else(|| "Qwen filetrans result missing transcription_url".to_string());
            }
            "FAILED" => {
                return Err(format!(
                    "Filetrans ASR task failed: {} {}",
                    response.output.code.unwrap_or_default(),
                    response.output.message.unwrap_or_default()
                ));
            }
            "PENDING" | "RUNNING" => {
                log::info!(
                    "Filetrans ASR: task_id={} status={} poll_attempt={}",
                    task_id,
                    response.output.task_status,
                    attempt
                );
                sleep(Duration::from_secs(2)).await;
            }
            status => {
                return Err(format!(
                    "Filetrans ASR task returned unknown status: {status}"
                ));
            }
        }
    }

    Err(format!("Filetrans ASR task timed out: {task_id}"))
}

async fn download_filetrans_sentences(
    client: &Client,
    transcription_url: &str,
) -> Result<Vec<RealtimeSentence>, String> {
    let response = client
        .get(transcription_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download filetrans result: {e}"))?;
    let status = response.status();
    let response_text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "Failed to download filetrans result with status {status}: {response_text}"
        ));
    }
    let result: FileTranscriptionResult = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse filetrans result JSON: {e}"))?;
    let sentences = result
        .transcripts
        .into_iter()
        .flat_map(|transcript| transcript.sentences)
        .filter_map(|sentence| {
            let text = sentence.text.trim().to_string();
            if text.is_empty() {
                None
            } else {
                let tokens = align_timed_tokens_to_sentence_text(
                    &text,
                    timed_tokens_from_filetrans_words(sentence.words),
                );
                Some(RealtimeSentence {
                    begin_ms: sentence.begin_time,
                    end_ms: sentence.end_time,
                    text,
                    tokens,
                })
            }
        })
        .collect::<Vec<_>>();
    Ok(sentences)
}

fn qwen_realtime_sentences_to_srt(sentences: &[RealtimeSentence]) -> String {
    sentences
        .iter()
        .enumerate()
        .filter(|(_, sentence)| {
            !sentence.text.trim().is_empty() && sentence.end_ms > sentence.begin_ms
        })
        .map(|(index, sentence)| {
            format!(
                "{}\n{} --> {}\n{}\n\n",
                index + 1,
                format_srt_time(sentence.begin_ms as f64 / 1000.0),
                format_srt_time(sentence.end_ms as f64 / 1000.0),
                sentence.text.trim()
            )
        })
        .collect::<String>()
}

fn count_realtime_srt_sentences(sentences: &[RealtimeSentence]) -> usize {
    sentences
        .iter()
        .filter(|sentence| !sentence.text.trim().is_empty() && sentence.end_ms > sentence.begin_ms)
        .count()
}

fn log_subtitle_text(provider: &str, index: usize, begin_ms: i64, end_ms: i64, text: &str) {
    log::info!(
        "{} subtitle #{} {} --> {} text={}",
        provider,
        index,
        format_srt_time(begin_ms as f64 / 1000.0),
        format_srt_time(end_ms as f64 / 1000.0),
        text.trim()
    );
}

async fn generate_qwen_realtime_subtitle(
    api_url: &str,
    api_key: Option<&str>,
    model: &str,
    audio_path: &Path,
    language_hint: &str,
    partial_srt_writer: Option<&RealtimeSrtWriter>,
    reporter: Option<&(impl ProgressReporterTrait + 'static)>,
) -> Result<GenerateResult, String> {
    log::info!(
        "Qwen realtime ASR: start model={} audio={} language_hint={}",
        model,
        audio_path.display(),
        language_hint
    );
    let api_key = api_key
        .filter(|api_key| !api_key.trim().is_empty())
        .ok_or_else(|| "API key not configured".to_string())?;

    if let Some(reporter) = reporter {
        reporter.update("提取实时识别音频").await;
    }
    let pcm_audio = extract_pcm16_mono_16k(audio_path).await?;
    let endpoint = qwen_realtime_endpoint(api_url, model);
    log::info!("Qwen realtime ASR: websocket endpoint={endpoint}");

    if let Some(reporter) = reporter {
        reporter.update("连接实时识别服务").await;
    }
    let mut socket = connect_realtime_websocket(&endpoint, api_key, "Qwen realtime ASR").await?;
    log::info!("Qwen realtime ASR: websocket connected");

    socket
        .send(Message::Text(
            qwen_realtime_session_update(language_hint)
                .to_string()
                .into(),
        ))
        .await
        .map_err(|e| format!("Failed to send session update: {e}"))?;
    log::debug!("Qwen realtime ASR: session.update sent");

    loop {
        let Some(message) = socket.next().await else {
            return Err("Qwen realtime ASR closed before session update".to_string());
        };
        let message = message.map_err(|e| format!("Failed to read websocket message: {e}"))?;
        if !message.is_text() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(message.to_text().unwrap_or(""))
            .map_err(|e| format!("Failed to parse websocket message: {e}"))?;
        let event_type = value.get("type").and_then(serde_json::Value::as_str);
        log::debug!("Qwen realtime ASR: setup event={event_type:?}");
        match event_type {
            Some("session.updated") => {
                log::info!("Qwen realtime ASR: session updated");
                break;
            }
            Some("error") => {
                return Err(format!(
                    "Qwen realtime ASR error: {}",
                    value.get("error").unwrap_or(&value)
                ));
            }
            _ => {}
        }
    }

    if let Some(reporter) = reporter {
        reporter.update("发送实时识别音频").await;
    }
    const PCM_CHUNK_BYTES: usize = 16_000;
    let chunk_count = pcm_audio.chunks(PCM_CHUNK_BYTES).len();
    log::info!(
        "Qwen realtime ASR: sending audio chunks count={} chunk_bytes={} total_bytes={}",
        chunk_count,
        PCM_CHUNK_BYTES,
        pcm_audio.len()
    );
    for (index, chunk) in pcm_audio.chunks(PCM_CHUNK_BYTES).enumerate() {
        socket
            .send(Message::Text(
                serde_json::json!({
                    "event_id": format!("event_{}", Uuid::new_v4()),
                    "type": "input_audio_buffer.append",
                    "audio": base64::engine::general_purpose::STANDARD.encode(chunk),
                })
                .to_string()
                .into(),
            ))
            .await
            .map_err(|e| format!("Failed to send audio chunk: {e}"))?;
        if index == 0 || index + 1 == chunk_count || (index + 1) % 50 == 0 {
            log::debug!(
                "Qwen realtime ASR: sent audio chunk {}/{} bytes={}",
                index + 1,
                chunk_count,
                chunk.len()
            );
        }
    }
    socket
        .send(Message::Text(
            serde_json::json!({
                "event_id": format!("event_{}", Uuid::new_v4()),
                "type": "session.finish",
            })
            .to_string()
            .into(),
        ))
        .await
        .map_err(|e| format!("Failed to finish realtime session: {e}"))?;
    log::info!("Qwen realtime ASR: session.finish sent");

    if let Some(reporter) = reporter {
        reporter.update("等待实时识别结果").await;
    }
    let mut starts_by_item = HashMap::<String, i64>::new();
    let mut ends_by_item = HashMap::<String, i64>::new();
    let mut sentences = Vec::<RealtimeSentence>::new();
    let mut last_end_ms = 0_i64;
    let mut event_count = 0_u64;

    while let Some(message) = socket.next().await {
        let message = message.map_err(|e| format!("Failed to read websocket message: {e}"))?;
        if !message.is_text() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(message.to_text().unwrap_or(""))
            .map_err(|e| format!("Failed to parse websocket message: {e}"))?;
        event_count += 1;
        let event_type = value.get("type").and_then(serde_json::Value::as_str);
        log::debug!("Qwen realtime ASR: result event #{event_count} type={event_type:?}");
        match event_type {
            Some("input_audio_buffer.speech_started") => {
                if let (Some(item_id), Some(start_ms)) = (
                    value.get("item_id").and_then(serde_json::Value::as_str),
                    value
                        .get("audio_start_ms")
                        .and_then(serde_json::Value::as_i64),
                ) {
                    starts_by_item.insert(item_id.to_string(), start_ms);
                    log::debug!(
                        "Qwen realtime ASR: speech started item_id={} start_ms={}",
                        item_id,
                        start_ms
                    );
                }
            }
            Some("input_audio_buffer.speech_stopped") => {
                if let (Some(item_id), Some(end_ms)) = (
                    value.get("item_id").and_then(serde_json::Value::as_str),
                    value
                        .get("audio_end_ms")
                        .and_then(serde_json::Value::as_i64),
                ) {
                    ends_by_item.insert(item_id.to_string(), end_ms);
                    log::debug!(
                        "Qwen realtime ASR: speech stopped item_id={} end_ms={}",
                        item_id,
                        end_ms
                    );
                }
            }
            Some("conversation.item.input_audio_transcription.completed") => {
                let text = value
                    .get("transcript")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if text.is_empty() {
                    continue;
                }
                let item_id = value
                    .get("item_id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                let begin_ms = starts_by_item.remove(item_id).unwrap_or(last_end_ms);
                let end_ms = ends_by_item
                    .remove(item_id)
                    .unwrap_or_else(|| begin_ms.saturating_add(1000));
                if item_id.is_empty() {
                    log::warn!(
                        "Qwen realtime ASR: completed transcription missing item_id; using fallback timing begin_ms={} end_ms={}",
                        begin_ms,
                        end_ms
                    );
                } else if end_ms <= begin_ms {
                    log::warn!(
                        "Qwen realtime ASR: completed transcription has invalid timing item_id={} begin_ms={} end_ms={}",
                        item_id,
                        begin_ms,
                        end_ms
                    );
                } else {
                    log::debug!(
                        "Qwen realtime ASR: completed item_id={} begin_ms={} end_ms={} chars={}",
                        item_id,
                        begin_ms,
                        end_ms,
                        text.chars().count()
                    );
                }
                log_subtitle_text(
                    "Qwen realtime ASR",
                    sentences.len() + 1,
                    begin_ms,
                    end_ms,
                    &text,
                );
                let raw_sentence = RealtimeSentence {
                    begin_ms,
                    end_ms,
                    text,
                    tokens: Vec::new(),
                };
                let reflowed_sentences = split_sentence_for_subtitles(&raw_sentence);
                if let Some(writer) = partial_srt_writer {
                    let base_index = count_realtime_srt_sentences(&sentences);
                    for (offset, sentence) in reflowed_sentences.iter().enumerate() {
                        writer
                            .append_sentence(
                                base_index + offset,
                                sentence.begin_ms,
                                sentence.end_ms,
                                &sentence.text,
                            )
                            .await?;
                    }
                }
                last_end_ms = last_end_ms.max(raw_sentence.end_ms);
                sentences.extend(reflowed_sentences);
            }
            Some("conversation.item.input_audio_transcription.failed") => {
                return Err(format!("Qwen realtime ASR transcription failed: {value}"));
            }
            Some("error") => {
                return Err(format!(
                    "Qwen realtime ASR error: {}",
                    value.get("error").unwrap_or(&value)
                ));
            }
            Some("session.finished") => break,
            _ => {}
        }
    }

    log::info!(
        "Qwen realtime ASR: finished events={} raw_sentences={} valid_sentences={}",
        event_count,
        sentences.len(),
        count_realtime_srt_sentences(&sentences)
    );
    let subtitle = qwen_realtime_sentences_to_srt(&sentences);
    let subtitle_content = if subtitle.trim().is_empty() {
        Vec::new()
    } else {
        srtparse::from_str(&subtitle).map_err(|e| format!("Failed to parse subtitle: {e}"))?
    };

    Ok(GenerateResult {
        generator_type: SubtitleGeneratorType::WhisperOnline,
        subtitle_id: String::new(),
        subtitle_content,
    })
}

async fn generate_fun_asr_realtime_subtitle(
    api_url: &str,
    api_key: Option<&str>,
    model: &str,
    audio_path: &Path,
    language_hint: &str,
    hotword_vocabulary_id: Option<&str>,
    partial_srt_writer: Option<&RealtimeSrtWriter>,
    reporter: Option<&(impl ProgressReporterTrait + 'static)>,
) -> Result<GenerateResult, String> {
    log::info!(
        "Fun-ASR realtime: start model={} audio={} language_hint={}",
        model,
        audio_path.display(),
        language_hint
    );
    let api_key = api_key
        .filter(|api_key| !api_key.trim().is_empty())
        .ok_or_else(|| "API key not configured".to_string())?;

    if let Some(reporter) = reporter {
        reporter.update("提取实时识别音频").await;
    }
    let pcm_audio = extract_pcm16_mono_16k(audio_path).await?;
    let endpoint = fun_asr_realtime_endpoint(api_url);
    log::info!("Fun-ASR realtime: websocket endpoint={endpoint}");

    if let Some(reporter) = reporter {
        reporter.update("连接实时识别服务").await;
    }
    let mut socket = connect_realtime_websocket(&endpoint, api_key, "Fun-ASR realtime").await?;
    log::info!("Fun-ASR realtime: websocket connected");

    let task_id = Uuid::new_v4().to_string();
    let mut parameters = serde_json::json!({
        "format": "pcm",
        "sample_rate": 16000,
    });
    if let Some(vocabulary_id) = hotword_vocabulary_id
        .map(str::trim)
        .filter(|vocabulary_id| !vocabulary_id.is_empty())
    {
        parameters["vocabulary_id"] = serde_json::json!(vocabulary_id);
    }
    if !language_hint.trim().is_empty() && language_hint != "auto" {
        parameters["language_hints"] = serde_json::json!([language_hint]);
    }
    socket
        .send(Message::Text(
            serde_json::json!({
                "header": {
                    "action": "run-task",
                    "task_id": task_id,
                    "streaming": "duplex",
                },
                "payload": {
                    "task_group": "audio",
                    "task": "asr",
                    "function": "recognition",
                    "model": model,
                    "parameters": parameters,
                    "input": {},
                },
            })
            .to_string()
            .into(),
        ))
        .await
        .map_err(|e| format!("Failed to send run-task: {e}"))?;
    log::info!("Fun-ASR realtime: run-task sent task_id={task_id}");

    loop {
        let Some(message) = socket.next().await else {
            return Err("Fun-ASR realtime closed before task started".to_string());
        };
        let message = message.map_err(|e| format!("Failed to read websocket message: {e}"))?;
        if !message.is_text() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(message.to_text().unwrap_or(""))
            .map_err(|e| format!("Failed to parse websocket message: {e}"))?;
        match value
            .get("header")
            .and_then(|header| header.get("event"))
            .and_then(serde_json::Value::as_str)
        {
            Some("task-started") => {
                log::info!("Fun-ASR realtime: task-started task_id={task_id}");
                break;
            }
            Some("task-failed") => return Err(format!("Fun-ASR realtime task failed: {value}")),
            event => {
                log::debug!("Fun-ASR realtime: setup event={event:?}");
            }
        }
    }

    if let Some(reporter) = reporter {
        reporter.update("发送实时识别音频").await;
    }
    const PCM_CHUNK_BYTES: usize = 16_000;
    let chunk_count = pcm_audio.chunks(PCM_CHUNK_BYTES).len();
    log::info!(
        "Fun-ASR realtime: sending audio chunks count={} chunk_bytes={} total_bytes={}",
        chunk_count,
        PCM_CHUNK_BYTES,
        pcm_audio.len()
    );
    for (index, chunk) in pcm_audio.chunks(PCM_CHUNK_BYTES).enumerate() {
        socket
            .send(Message::Binary(chunk.to_vec().into()))
            .await
            .map_err(|e| format!("Failed to send audio chunk: {e}"))?;
        if index == 0 || index + 1 == chunk_count || (index + 1) % 50 == 0 {
            log::debug!(
                "Fun-ASR realtime: sent audio chunk {}/{} bytes={}",
                index + 1,
                chunk_count,
                chunk.len()
            );
        }
    }
    socket
        .send(Message::Text(
            serde_json::json!({
                "header": {
                    "action": "finish-task",
                    "task_id": task_id,
                    "streaming": "duplex",
                },
                "payload": {
                    "input": {},
                },
            })
            .to_string()
            .into(),
        ))
        .await
        .map_err(|e| format!("Failed to finish realtime task: {e}"))?;
    log::info!("Fun-ASR realtime: finish-task sent task_id={task_id}");

    if let Some(reporter) = reporter {
        reporter.update("等待实时识别结果").await;
    }
    let mut sentences = Vec::<RealtimeSentence>::new();
    let mut event_count = 0_u64;
    while let Some(message) = socket.next().await {
        let message = message.map_err(|e| format!("Failed to read websocket message: {e}"))?;
        if !message.is_text() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(message.to_text().unwrap_or(""))
            .map_err(|e| format!("Failed to parse websocket message: {e}"))?;
        match value
            .get("header")
            .and_then(|header| header.get("event"))
            .and_then(serde_json::Value::as_str)
        {
            Some("result-generated") => {
                event_count += 1;
                let Some(sentence) = value
                    .get("payload")
                    .and_then(|payload| payload.get("output"))
                    .and_then(|output| output.get("sentence"))
                else {
                    continue;
                };
                if sentence
                    .get("heartbeat")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
                    || !sentence
                        .get("sentence_end")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false)
                {
                    continue;
                }
                let text = sentence
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if text.is_empty() {
                    continue;
                }
                let begin_ms = sentence
                    .get("begin_time")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0);
                let end_ms = sentence
                    .get("end_time")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or_else(|| begin_ms.saturating_add(1000));
                log::debug!(
                    "Fun-ASR realtime: final sentence begin_ms={} end_ms={} chars={}",
                    begin_ms,
                    end_ms,
                    text.chars().count()
                );
                log_subtitle_text(
                    "Fun-ASR realtime",
                    sentences.len() + 1,
                    begin_ms,
                    end_ms,
                    &text,
                );
                let tokens = align_timed_tokens_to_sentence_text(
                    &text,
                    timed_tokens_from_json_words(sentence.get("words")),
                );
                let raw_sentence = RealtimeSentence {
                    begin_ms,
                    end_ms,
                    text,
                    tokens,
                };
                log::debug!(
                    "Fun-ASR realtime: final sentence words={} reflow_source_chars={}",
                    raw_sentence.tokens.len(),
                    raw_sentence.text.chars().count()
                );
                let reflowed_sentences = split_sentence_for_subtitles(&raw_sentence);
                if let Some(writer) = partial_srt_writer {
                    let base_index = count_realtime_srt_sentences(&sentences);
                    for (offset, sentence) in reflowed_sentences.iter().enumerate() {
                        writer
                            .append_sentence(
                                base_index + offset,
                                sentence.begin_ms,
                                sentence.end_ms,
                                &sentence.text,
                            )
                            .await?;
                    }
                }
                sentences.extend(reflowed_sentences);
            }
            Some("task-finished") => {
                log::info!("Fun-ASR realtime: task-finished task_id={task_id}");
                break;
            }
            Some("task-failed") => return Err(format!("Fun-ASR realtime task failed: {value}")),
            event => {
                log::debug!("Fun-ASR realtime: result event={event:?}");
            }
        }
    }

    log::info!(
        "Fun-ASR realtime: finished result_events={} raw_sentences={} valid_sentences={}",
        event_count,
        sentences.len(),
        count_realtime_srt_sentences(&sentences)
    );
    let subtitle = qwen_realtime_sentences_to_srt(&sentences);
    let subtitle_content = if subtitle.trim().is_empty() {
        Vec::new()
    } else {
        srtparse::from_str(&subtitle).map_err(|e| format!("Failed to parse subtitle: {e}"))?
    };

    Ok(GenerateResult {
        generator_type: SubtitleGeneratorType::WhisperOnline,
        subtitle_id: String::new(),
        subtitle_content,
    })
}

async fn generate_filetrans_subtitle(
    client: &Client,
    api_url: &str,
    api_key: Option<&str>,
    model: &str,
    audio_path: &Path,
    language_hint: &str,
    hotword_vocabulary_id: Option<&str>,
    partial_srt_writer: Option<&RealtimeSrtWriter>,
    reporter: Option<&(impl ProgressReporterTrait + 'static)>,
) -> Result<GenerateResult, String> {
    let api_key = api_key
        .filter(|api_key| !api_key.trim().is_empty())
        .ok_or_else(|| "API key not configured".to_string())?;
    let audio_fingerprint = audio_file_fingerprint(audio_path);

    let cached_task_id = {
        let cache = load_filetrans_task_cache(audio_path).await;
        find_cached_filetrans_task(
            &cache,
            api_url,
            model,
            language_hint,
            hotword_vocabulary_id,
            &audio_fingerprint,
        )
        .map(|entry| entry.task_id.clone())
    };

    if let Some(reporter) = reporter {
        reporter.update("查询文件转写任务").await;
    }
    let mut reusable_result = None;
    if cached_task_id.is_none() {
        log::info!(
            "Filetrans ASR: no cached task_id model={} fingerprint={}",
            model,
            audio_fingerprint
        );
    }
    if let Some(cached_task_id) = cached_task_id {
        log::info!(
            "Filetrans ASR: trying cached task_id={} model={} fingerprint={}",
            cached_task_id,
            model,
            audio_fingerprint
        );
        match poll_filetrans_result_url(client, api_url, api_key, model, &cached_task_id).await {
            Ok(transcription_url) => {
                log::info!(
                    "Filetrans ASR: reused cached task_id={} model={} fingerprint={}",
                    cached_task_id,
                    model,
                    audio_fingerprint
                );
                reusable_result = Some((cached_task_id, transcription_url));
            }
            Err(error) => {
                log::warn!(
                    "Filetrans ASR: cached task_id={} unavailable: {}; submitting new task",
                    cached_task_id,
                    error
                );
            }
        }
    }

    let (task_id, transcription_url) = if let Some(result) = reusable_result {
        result
    } else {
        if let Some(reporter) = reporter {
            reporter.update("上传音频到 DashScope 临时 OSS").await;
        }
        let upload =
            upload_audio_to_dashscope_temp_oss(client, api_url, api_key, model, audio_path).await?;

        if let Some(reporter) = reporter {
            reporter.update("提交文件转写任务").await;
        }
        let task_id = submit_filetrans_task(
            client,
            api_url,
            api_key,
            model,
            &upload.file_url,
            language_hint,
            hotword_vocabulary_id,
        )
        .await?;
        log::info!("Filetrans ASR: submitted model={model} task_id={task_id}");
        if let Err(error) = remember_filetrans_task(
            audio_path,
            api_url,
            model,
            language_hint,
            hotword_vocabulary_id,
            &upload,
            task_id.clone(),
        )
        .await
        {
            log::warn!("Filetrans ASR: failed to save task cache: {error}");
        } else {
            log::info!(
                "Filetrans ASR: saved task cache task_id={} model={} object={}",
                task_id,
                model,
                upload.object_key
            );
        }

        if let Some(reporter) = reporter {
            reporter.update("等待文件转写结果").await;
        }
        let transcription_url =
            poll_filetrans_result_url(client, api_url, api_key, model, &task_id).await?;
        (task_id, transcription_url)
    };
    log::info!("Filetrans ASR: transcription url ready task_id={task_id}");

    let raw_sentences = download_filetrans_sentences(client, &transcription_url).await?;
    let sentences = raw_sentences
        .iter()
        .flat_map(split_sentence_for_subtitles)
        .collect::<Vec<_>>();
    for (index, sentence) in sentences.iter().enumerate() {
        log_subtitle_text(
            "Filetrans ASR",
            index + 1,
            sentence.begin_ms,
            sentence.end_ms,
            &sentence.text,
        );
        if let Some(writer) = partial_srt_writer {
            writer
                .append_sentence(index, sentence.begin_ms, sentence.end_ms, &sentence.text)
                .await?;
        }
    }
    log::info!(
        "Filetrans ASR: finished model={} task_id={} raw_sentences={} reflowed_sentences={} valid_sentences={}",
        model,
        task_id,
        raw_sentences.len(),
        sentences.len(),
        count_realtime_srt_sentences(&sentences)
    );

    let subtitle = qwen_realtime_sentences_to_srt(&sentences);
    let subtitle_content = if subtitle.trim().is_empty() {
        Vec::new()
    } else {
        srtparse::from_str(&subtitle).map_err(|e| format!("Failed to parse subtitle: {e}"))?
    };

    Ok(GenerateResult {
        generator_type: SubtitleGeneratorType::WhisperOnline,
        subtitle_id: task_id,
        subtitle_content,
    })
}

impl WhisperOnline {
    pub async fn generate_subtitle_with_partial(
        &self,
        reporter: Option<&(impl ProgressReporterTrait + 'static)>,
        audio_path: &Path,
        language_hint: &str,
        partial_srt_path: Option<&Path>,
        offset_seconds: u64,
        next_subtitle_index: usize,
    ) -> Result<GenerateResult, String> {
        log::info!(
            "Generating subtitle online for {:?} with {}",
            audio_path,
            self.model
        );
        let start_time = std::time::Instant::now();

        // Read audio file
        if let Some(reporter) = reporter {
            reporter.update("读取音频文件中").await;
        }
        if is_fun_asr_realtime_model(&self.model) {
            let partial_srt_writer = partial_srt_path
                .map(|path| RealtimeSrtWriter::new(path, offset_seconds, next_subtitle_index));
            let result = generate_fun_asr_realtime_subtitle(
                &self.api_url,
                self.api_key.as_deref(),
                &self.model,
                audio_path,
                language_hint,
                self.hotword_vocabulary_id.as_deref(),
                partial_srt_writer.as_ref(),
                reporter,
            )
            .await
            .map_err(|e| format!("Fun-ASR realtime failed: {e}"))?;
            log::info!("Time taken: {} seconds", start_time.elapsed().as_secs_f64());
            return Ok(result);
        }

        if is_qwen_asr_flash_model(&self.model) {
            let partial_srt_writer = partial_srt_path
                .map(|path| RealtimeSrtWriter::new(path, offset_seconds, next_subtitle_index));
            let result = generate_qwen_realtime_subtitle(
                &self.api_url,
                self.api_key.as_deref(),
                &self.model,
                audio_path,
                language_hint,
                partial_srt_writer.as_ref(),
                reporter,
            )
            .await
            .map_err(|e| format!("Qwen realtime ASR failed: {e}"))?;
            log::info!("Time taken: {} seconds", start_time.elapsed().as_secs_f64());
            return Ok(result);
        }

        if is_fun_asr_filetrans_model(&self.model) || is_qwen_asr_filetrans_model(&self.model) {
            let partial_srt_writer = partial_srt_path
                .map(|path| RealtimeSrtWriter::new(path, offset_seconds, next_subtitle_index));
            let result = generate_filetrans_subtitle(
                &self.client,
                &self.api_url,
                self.api_key.as_deref(),
                &self.model,
                audio_path,
                language_hint,
                self.hotword_vocabulary_id.as_deref(),
                partial_srt_writer.as_ref(),
                reporter,
            )
            .await
            .map_err(|e| format!("Filetrans ASR failed: {e}"))?;
            log::info!("Time taken: {} seconds", start_time.elapsed().as_secs_f64());
            return Ok(result);
        }

        let audio_data = fs::read(audio_path)
            .await
            .map_err(|e| format!("Failed to read audio file: {e}"))?;

        // Build form data with proper file part
        let file_part = reqwest::multipart::Part::bytes(audio_data)
            .mime_str(mime_type_for_audio_path(audio_path))
            .map_err(|e| format!("Failed to set MIME type: {e}"))?
            .file_name(
                audio_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            );

        let mut form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("model", self.model.clone())
            .text("response_format", "verbose_json")
            .text("temperature", "0.0");

        form = form.text("language", language_hint.to_string());

        if let Some(prompt) = self.prompt.clone() {
            form = form.text("prompt", prompt);
        }

        // Build HTTP request
        let mut req_builder = self
            .client
            .post(format!("{}/audio/transcriptions", self.api_url));

        if let Some(api_key) = &self.api_key {
            req_builder = req_builder.header("Authorization", format!("Bearer {api_key}"));
        }

        if let Some(reporter) = reporter {
            reporter.update("上传音频中").await;
        }
        let response = req_builder
            .timeout(std::time::Duration::from_secs(3 * 60)) // 3 minutes timeout
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("API request failed with status {status}: {error_text}");
            return Err(format!(
                "API request failed with status {status}: {error_text}"
            ));
        }

        // Get the raw response text first for debugging
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to get response text: {e}"))?;

        // Try to parse as JSON
        let whisper_response: WhisperResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                println!("{response_text}");
                log::error!("Failed to parse JSON response. Raw response: {response_text}");
                format!("Failed to parse response: {e}")
            })?;

        // Generate SRT format subtitle
        let mut subtitle = String::new();
        for (i, segment) in whisper_response.segments.iter().enumerate() {
            log_subtitle_text(
                "Online ASR",
                i + 1,
                (segment.start * 1000.0).floor() as i64,
                (segment.end * 1000.0).floor() as i64,
                &segment.text,
            );
            let line = format!(
                "{}\n{} --> {}\n{}\n\n",
                i + 1,
                format_srt_time(segment.start),
                format_srt_time(segment.end),
                segment.text.trim(),
            );

            subtitle.push_str(&line);
        }

        log::info!("Time taken: {} seconds", start_time.elapsed().as_secs_f64());

        let subtitle_content =
            srtparse::from_str(&subtitle).map_err(|e| format!("Failed to parse subtitle: {e}"))?;

        Ok(GenerateResult {
            generator_type: SubtitleGeneratorType::WhisperOnline,
            subtitle_id: String::new(),
            subtitle_content,
        })
    }
}

#[async_trait]
impl SubtitleGenerator for WhisperOnline {
    async fn generate_subtitle(
        &self,
        reporter: Option<&(impl ProgressReporterTrait + 'static)>,
        audio_path: &Path,
        language_hint: &str,
    ) -> Result<GenerateResult, String> {
        self.generate_subtitle_with_partial(reporter, audio_path, language_hint, None, 0, 1)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    // Mock reporter for testing
    #[derive(Clone)]
    struct MockReporter {}

    #[async_trait]
    impl ProgressReporterTrait for MockReporter {
        async fn update(&self, message: &str) {
            println!("Mock update: {message}");
        }

        async fn finish(&self, success: bool, message: &str) {
            if success {
                println!("Mock finish: {message}");
            } else {
                println!("Mock error: {message}");
            }
        }
    }

    impl MockReporter {
        fn new() -> Self {
            MockReporter {}
        }
    }

    #[test]
    fn test_align_timed_tokens_preserves_english_spaces() {
        let tokens = vec![
            TimedSubtitleToken {
                begin_ms: 0,
                end_ms: 300,
                text: "hello".to_string(),
            },
            TimedSubtitleToken {
                begin_ms: 300,
                end_ms: 900,
                text: "world.".to_string(),
            },
        ];
        let aligned = align_timed_tokens_to_sentence_text("hello world.", tokens);
        let text = aligned
            .iter()
            .map(|token| token.text.as_str())
            .collect::<String>();
        assert_eq!(text, "hello world.");
    }

    #[test]
    fn test_split_sentence_for_subtitles_uses_token_timing() {
        let text = "现在这句话非常非常长需要被拆成多条字幕否则单行显示会太挤而且阅读速度也不稳定。";
        let tokens = text
            .chars()
            .enumerate()
            .map(|(index, ch)| TimedSubtitleToken {
                begin_ms: index as i64 * 120,
                end_ms: index as i64 * 120 + 120,
                text: ch.to_string(),
            })
            .collect::<Vec<_>>();
        let sentence = RealtimeSentence {
            begin_ms: 0,
            end_ms: tokens.last().map(|token| token.end_ms).unwrap_or(0),
            text: text.to_string(),
            tokens,
        };

        let split = split_sentence_for_subtitles(&sentence);
        assert!(split.len() > 1);
        assert_eq!(
            split
                .iter()
                .map(|sentence| sentence.text.replace('\n', ""))
                .collect::<String>(),
            text
        );
        assert!(split
            .windows(2)
            .all(|window| window[0].end_ms <= window[1].begin_ms));
    }

    #[tokio::test]
    async fn test_create_whisper_online() {
        let result = new(
            Some("https://api.openai.com/v1"),
            Some("test-key"),
            None,
            Some("whisper-1"),
            None,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "requres api key"]
    async fn test_generate_subtitle() {
        let result = new(
            Some("https://api.openai.com/v1"),
            Some("sk-****"),
            None,
            Some("whisper-1"),
            None,
        )
        .await;
        assert!(result.is_ok());
        let result = result.unwrap();
        let result = result
            .generate_subtitle(
                Some(&MockReporter::new()),
                Path::new("tests/audio/test.wav"),
                "auto",
            )
            .await;
        println!("{result:?}");
        assert!(result.is_ok());
        let result = result.unwrap();
        println!("{:?}", result.subtitle_content);
    }
}
