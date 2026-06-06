use crate::bilibili_danmaku;
use crate::danmu2ass;
use crate::database::task::TaskRow;
use crate::database::video::VideoRow;
use crate::ffmpeg;
use crate::handlers::utils::get_disk_info_inner;
use crate::progress::progress_reporter::{EventEmitter, ProgressReporter, ProgressReporterTrait};
use crate::recorder_manager::ClipRangeParams;
use crate::subtitle_generator::item_to_srt;
use crate::task::{Task, TaskPriority};
use crate::webhook::events;
use base64::Engine;
use chrono::{Local, Utc};
use recorder::danmu::{decode_danmu_content, encode_danmu_content, DanmuEntry};
use recorder::platforms::bilibili;
use recorder::platforms::bilibili::profile::Profile;
use serde_json::json;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// 检测路径是否为网络协议路径（排除Windows盘符）
fn is_network_protocol(path_str: &str) -> bool {
    // 常见的网络协议
    let network_protocols = [
        "ftp://", "sftp://", "ftps://", "http://", "https://", "smb://", "cifs://", "nfs://",
        "afp://", "ssh://", "scp://",
    ];

    // 检查是否以网络协议开头
    for protocol in &network_protocols {
        if path_str.to_lowercase().starts_with(protocol) {
            return true;
        }
    }

    // 排除Windows盘符格式 (如 C:/, D:/, E:/ 等)
    if cfg!(windows) {
        // 检查是否为Windows盘符格式：单字母 + : + /
        if path_str.len() >= 3 {
            let chars: Vec<char> = path_str.chars().collect();
            if chars.len() >= 3
                && chars[0].is_ascii_alphabetic()
                && chars[1] == ':'
                && (chars[2] == '/' || chars[2] == '\\')
            {
                return false; // 这是Windows盘符，不是网络路径
            }
        }
    }

    false
}

/// 判断是否需要转换视频格式
/// FLV格式在现代浏览器中播放兼容性差，需要转换为MP4
fn should_convert_video_format(extension: &str) -> bool {
    // FLV格式在现代浏览器中播放兼容性差，需要转换为MP4
    matches!(extension.to_lowercase().as_str(), "flv")
}

/// 获取视频的最佳缩略图截取时间点
/// 根据视频长度选择最佳时间点，避开开头可能的黑屏
fn get_optimal_thumbnail_timestamp(duration: f64) -> f64 {
    // 根据视频长度选择最佳时间点
    if duration <= 10.0 {
        // 短视频（10秒以内）：选择1/3位置，避免开头黑屏
        duration / 3.0
    } else if duration <= 60.0 {
        // 1分钟以内：选择第3秒
        3.0
    } else if duration <= 300.0 {
        // 5分钟以内：选择第5秒
        5.0
    } else {
        // 长视频：选择第10秒，确保跳过开头可能的黑屏/logo
        10.0
    }
}

fn to_output_relative_path(output_root: &Path, full_path: &Path) -> PathBuf {
    full_path
        .strip_prefix(output_root)
        .map(|v| v.to_path_buf())
        .unwrap_or_else(|_| full_path.file_name().map(PathBuf::from).unwrap_or_default())
}

fn clip_media_prefix(include_subtitle: bool, include_danmu: bool) -> &'static str {
    match (include_subtitle, include_danmu) {
        (true, true) => "[subtitle-danmaku]",
        (true, false) => crate::constants::PREFIX_SUBTITLE,
        (false, true) => crate::constants::PREFIX_DANMAKU,
        (false, false) => "[none]",
    }
}

fn clip_output_filename(
    prefix: &str,
    timestamp: &str,
    ordinal_suffix: &str,
    extension: &str,
) -> String {
    format!("{prefix}[{timestamp}]{ordinal_suffix}.{extension}")
}

use crate::state::State;
use crate::state_type;

// 带进度的文件复制函数
async fn copy_file_with_progress(
    source: &Path,
    dest: &Path,
    reporter: &ProgressReporter,
) -> Result<(), String> {
    let mut source_file = File::open(source).map_err(|e| format!("无法打开源文件: {e}"))?;
    let mut dest_file = File::create(dest).map_err(|e| format!("无法创建目标文件: {e}"))?;

    let total_size = source_file
        .metadata()
        .map_err(|e| format!("无法获取文件大小: {e}"))?
        .len();
    let mut copied = 0u64;

    // 使用固定的小缓冲区避免大文件时的内存占用
    let buffer_size = 64 * 1024; // 64KB buffer for all files

    let mut buffer = vec![0u8; buffer_size];

    let mut last_reported_percent = 0;

    loop {
        let bytes_read = source_file
            .read(&mut buffer)
            .map_err(|e| format!("读取文件失败: {e}"))?;
        if bytes_read == 0 {
            break;
        }

        dest_file
            .write_all(&buffer[..bytes_read])
            .map_err(|e| format!("写入文件失败: {e}"))?;
        copied += bytes_read as u64;

        // 计算进度百分比，只在变化时更新
        let percent = if total_size > 0 {
            ((copied as f64 / total_size as f64) * 100.0) as u32
        } else {
            0
        };

        // 使用固定的进度报告频率
        let report_threshold = 1; // 每1%报告一次

        if percent != last_reported_percent && (percent % report_threshold == 0 || percent == 100) {
            reporter
                .update(&format!("正在复制视频文件... {percent}%"))
                .await;
            last_reported_percent = percent;
        }
    }

    dest_file
        .flush()
        .map_err(|e| format!("刷新文件缓冲区失败: {e}"))?;
    Ok(())
}

// 智能边拷贝边转换函数（针对网络文件优化）
async fn copy_and_convert_with_progress(
    source: &Path,
    dest: &Path,
    need_conversion: bool,
    reporter: &ProgressReporter,
) -> Result<(), String> {
    if !need_conversion {
        // 非转换文件直接使用原有拷贝逻辑
        return copy_file_with_progress(source, dest, reporter).await;
    }

    // 检查源文件是否在网络位置（启发式判断）
    let source_str = source.to_string_lossy();
    let is_network_source = source_str.starts_with("\\\\") ||  // UNC path (Windows网络共享)
                           is_network_protocol(&source_str); // 网络协议但排除Windows盘符

    if is_network_source {
        // 网络文件：先复制到本地临时位置，再转换
        reporter
            .update("检测到网络文件，使用先复制后转换策略...")
            .await;
        copy_then_convert_strategy(source, dest, reporter).await
    } else {
        // 本地文件：直接转换（更高效）
        reporter.update("检测到本地文件，使用直接转换策略...").await;
        ffmpeg::convert_video_format(source, dest, reporter).await
    }
}

// 网络文件处理策略：先复制到本地临时位置，再转换
async fn copy_then_convert_strategy(
    source: &Path,
    dest: &Path,
    reporter: &ProgressReporter,
) -> Result<(), String> {
    // 创建临时文件路径
    let temp_dir = std::env::temp_dir();
    let temp_filename = format!(
        "temp_video_{}.{}",
        chrono::Utc::now().timestamp(),
        source.extension().and_then(|e| e.to_str()).unwrap_or("tmp")
    );
    let temp_path = temp_dir.join(&temp_filename);

    // 确保临时目录存在
    if let Some(parent) = temp_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建临时目录失败: {e}"))?;
    }

    // 第一步：将网络文件复制到本地临时位置（使用优化的缓冲区）
    reporter
        .update("第1步：从网络复制文件到本地临时位置...")
        .await;
    copy_file_with_network_optimization(source, &temp_path, reporter).await?;

    // 第二步：从本地临时文件转换到目标位置
    reporter.update("第2步：从临时文件转换到目标格式...").await;
    let convert_result = ffmpeg::convert_video_format(&temp_path, dest, reporter).await;

    // 清理临时文件
    if temp_path.exists() {
        if let Err(e) = std::fs::remove_file(&temp_path) {
            log::warn!("删除临时文件失败: {} - {}", temp_path.display(), e);
        } else {
            log::info!("已清理临时文件: {}", temp_path.display());
        }
    }

    convert_result
}

// 针对网络文件优化的复制函数
async fn copy_file_with_network_optimization(
    source: &Path,
    dest: &Path,
    reporter: &ProgressReporter,
) -> Result<(), String> {
    let mut source_file = File::open(source).map_err(|e| format!("无法打开网络源文件: {e}"))?;
    let mut dest_file = File::create(dest).map_err(|e| format!("无法创建本地临时文件: {e}"))?;

    let total_size = source_file
        .metadata()
        .map_err(|e| format!("无法获取文件大小: {e}"))?
        .len();
    let mut copied = 0u64;

    // 使用固定的小缓冲区，避免大文件时内存占用过多
    let buffer_size = 64 * 1024; // 64KB buffer for network files

    let mut buffer = vec![0u8; buffer_size];
    let mut last_reported_percent = 0;
    let mut consecutive_errors = 0;
    const MAX_RETRIES: u32 = 3;

    loop {
        match source_file.read(&mut buffer) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    break; // 文件读取完成
                }

                // 重置错误计数
                consecutive_errors = 0;

                dest_file
                    .write_all(&buffer[..bytes_read])
                    .map_err(|e| format!("写入临时文件失败: {e}"))?;
                copied += bytes_read as u64;

                // 计算并报告进度
                let percent = if total_size > 0 {
                    ((copied as f64 / total_size as f64) * 100.0) as u32
                } else {
                    0
                };

                // 网络文件更频繁地报告进度
                if percent != last_reported_percent {
                    reporter
                        .update(&format!(
                            "正在从网络复制文件... {}% ({:.1}MB/{:.1}MB)",
                            percent,
                            copied as f64 / (1024.0 * 1024.0),
                            total_size as f64 / (1024.0 * 1024.0)
                        ))
                        .await;
                    last_reported_percent = percent;
                }
            }
            Err(e) => {
                consecutive_errors += 1;
                log::warn!("网络读取错误 (尝试 {consecutive_errors}/{MAX_RETRIES}): {e}");

                if consecutive_errors >= MAX_RETRIES {
                    return Err(format!("网络文件读取失败，已重试{MAX_RETRIES}次: {e}"));
                }

                // 等待一小段时间后重试
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                reporter
                    .update(&format!(
                        "网络连接中断，正在重试... ({consecutive_errors}/{MAX_RETRIES})"
                    ))
                    .await;
            }
        }
    }

    dest_file
        .flush()
        .map_err(|e| format!("刷新临时文件缓冲区失败: {e}"))?;
    reporter.update("网络文件复制完成").await;
    Ok(())
}

#[cfg(feature = "gui")]
use {tauri::Manager, tauri::State as TauriState, tauri_plugin_notification::NotificationExt};

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn clip_range(
    state: state_type!(),
    event_id: String,
    params: ClipRangeParams,
) -> Result<VideoRow, String> {
    // check storage space, preserve 1GB for other usage
    let output = state.config.read().await.output.clone();
    let mut output = PathBuf::from(&output);
    if output.is_relative() {
        // get current working directory
        let cwd = std::env::current_dir().unwrap();
        output = cwd.join(output);
    }

    if let Ok(disk_info) = get_disk_info_inner(output).await {
        // if free space is less than 1GB, return error
        if disk_info.free < 1024 * 1024 * 1024 {
            return Err("Storage space is not enough, clip canceled".to_string());
        }
    }

    #[cfg(feature = "gui")]
    let emitter = EventEmitter::new(state.app_handle.clone());
    #[cfg(feature = "headless")]
    let emitter = EventEmitter::new(state.progress_manager.get_event_sender());
    let reporter = ProgressReporter::new(state.db.clone(), &emitter, &event_id).await?;
    let mut params_without_cover = params.clone();
    params_without_cover.cover = String::new();
    let task = TaskRow {
        id: event_id.clone(),
        task_type: "clip_range".to_string(),
        status: "pending".to_string(),
        message: String::new(),
        metadata: json!({
            "params": params_without_cover,
        })
        .to_string(),
        created_at: Utc::now().to_rfc3339(),
    };

    state.db.add_task(&task).await?;
    log::info!("Create task: {} {}", task.id, task.task_type);

    let (result_tx, result_rx) = tokio::sync::oneshot::channel();

    #[cfg(feature = "gui")]
    let state_clone = (*state).clone();
    #[cfg(feature = "headless")]
    let state_clone = state.clone();

    let task_id = event_id.clone();
    state
        .task_manager
        .add_task(Task::new(
            task_id.clone(),
            TaskPriority::Normal,
            async move {
                let result = match clip_range_inner(&state_clone, &reporter, params).await {
                    Ok(video) => {
                        reporter.finish(true, "切片完成").await;
                        let _ = state_clone
                            .db
                            .update_task(&task_id, "success", "切片完成", None)
                            .await;

                        if state_clone.config.read().await.auto_subtitle {
                            let subtitle_event_id = format!("{task_id}_subtitle");
                            let result = generate_video_subtitle_inner(
                                &state_clone,
                                subtitle_event_id,
                                video.id,
                            )
                            .await;
                            if let Ok(subtitle) = result {
                                let result =
                                    update_video_subtitle_inner(&state_clone, video.id, subtitle)
                                        .await;
                                if let Err(e) = result {
                                    log::error!("Update video subtitle error: {e}");
                                }
                            } else {
                                log::error!(
                                    "Generate video subtitle error: {}",
                                    result.err().unwrap()
                                );
                            }
                        }

                        let event = events::new_webhook_event(
                            events::CLIP_GENERATED,
                            events::Payload::Clip(video.clone()),
                        );

                        if let Err(e) = state_clone.webhook_poster.post_event(&event).await {
                            log::error!("Post webhook event error: {e}");
                        }

                        Ok(video)
                    }
                    Err(e) => {
                        reporter.finish(false, &format!("切片失败: {e}")).await;
                        let _ = state_clone
                            .db
                            .update_task(&task_id, "failed", &format!("切片失败: {e}"), None)
                            .await;
                        Err(e)
                    }
                };

                let task_result = result.as_ref().map(|_| ()).map_err(Clone::clone);
                let _ = result_tx.send(result);
                task_result
            },
        ))
        .await?;

    result_rx
        .await
        .unwrap_or_else(|_| Err("切片任务失败".to_string()))
}

async fn clip_range_inner(
    state: &State,
    reporter: &ProgressReporter,
    params: ClipRangeParams,
) -> Result<VideoRow, String> {
    log::info!(
        "[{}]Clip room_id: {}, ts: {}, ranges: {:?}",
        reporter.event_id,
        params.room_id,
        params.live_id,
        params.ranges,
    );

    let clip_file = state.config.read().await.generate_clip_name(&params);

    let file = state
        .recorder_manager
        .clip_range(Some(reporter), clip_file, &params)
        .await?;
    log::info!("Clip range done, doing post processing");
    // get file metadata from fs
    let metadata = std::fs::metadata(&file).map_err(|e| {
        log::error!("Get file metadata error: {} {}", e, file.display());
        e.to_string()
    })?;
    let mut cover_generate_ffmpeg = true;
    let cover_file = file.with_extension("jpg");
    if !params.cover.is_empty() {
        if let Some(base64) = params.cover.split("base64,").nth(1) {
            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(base64) {
                // write cover file to fs
                tokio::fs::write(&cover_file, bytes).await.map_err(|e| {
                    log::error!("Write cover file error: {} {}", e, cover_file.display());
                    e.to_string()
                })?;
                cover_generate_ffmpeg = false;
            } else {
                log::error!("Decode base64 error: {}", params.cover);
            }
        } else {
            log::error!("Invalid cover base64: {}", params.cover);
        }
    }
    // generate cover file from video as fallback
    if cover_generate_ffmpeg {
        ffmpeg::generate_thumbnail(&file, 0.0).await?;
    }
    let _ = crate::ffmpeg::extract_audio_sample(&file).await?;
    let output_root = PathBuf::from(state.config.read().await.output.as_str());
    let relative_file_path = to_output_relative_path(&output_root, &file);
    let relative_cover_path = to_output_relative_path(&output_root, &cover_file);
    let relative_file_name = relative_file_path.to_string_lossy().to_string();
    // add video to db
    let Ok(size) = i64::try_from(metadata.len()) else {
        log::error!(
            "Failed to convert metadata length to i64: {}",
            metadata.len()
        );
        return Err("Failed to convert metadata length to i64".to_string());
    };
    let duration = params.ranges.iter().map(|r| r.duration()).sum::<f64>();
    let video = state
        .db
        .add_video(&VideoRow {
            id: 0,
            status: 0,
            room_id: params.room_id.clone(),
            created_at: Local::now().to_rfc3339(),
            cover: relative_cover_path.to_string_lossy().to_string(),
            file: relative_file_name.clone(),
            note: params.note.clone(),
            length: duration as i64,
            size,
            bvid: String::new(),
            title: String::new(),
            desc: String::new(),
            tags: String::new(),
            area: 0,
            platform: params.platform.clone(),
        })
        .await?;
    state
        .db
        .new_message(
            "生成新切片",
            &format!(
                "生成了房间 {} 的切片，长度 {}s：{}",
                &params.room_id, duration, relative_file_name
            ),
        )
        .await?;
    if state.config.read().await.clip_notify {
        #[cfg(feature = "gui")]
        state
            .app_handle
            .notification()
            .builder()
            .title("BiliShadowReplay - 切片完成")
            .body(format!(
                "生成了房间 {} 的切片: {}",
                &params.room_id, relative_file_name
            ))
            .show()
            .unwrap();
    }

    reporter.finish(true, "切片完成").await;

    Ok(video)
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn upload_procedure(
    state: state_type!(),
    event_id: String,
    uid: String,
    room_id: String,
    video_id: i64,
    profile: Profile,
) -> Result<String, String> {
    #[cfg(feature = "gui")]
    let emitter = EventEmitter::new(state.app_handle.clone());
    #[cfg(feature = "headless")]
    let emitter = EventEmitter::new(state.progress_manager.get_event_sender());
    let reporter = ProgressReporter::new(state.db.clone(), &emitter, &event_id).await?;
    let task = TaskRow {
        id: event_id.clone(),
        task_type: "upload_procedure".to_string(),
        status: "pending".to_string(),
        message: String::new(),
        metadata: json!({
            "uid": uid,
            "room_id": room_id,
            "video_id": video_id,
            "profile": profile,
        })
        .to_string(),
        created_at: Utc::now().to_rfc3339(),
    };
    state.db.add_task(&task).await?;
    log::info!("Create task: {task:?}");
    match upload_procedure_inner(&state, &reporter, uid, room_id, video_id, profile).await {
        Ok(bvid) => {
            reporter.finish(true, "投稿成功").await;
            state
                .db
                .update_task(&event_id, "success", "投稿成功", None)
                .await?;
            Ok(bvid)
        }
        Err(e) => {
            reporter.finish(false, &format!("投稿失败: {e}")).await;
            state
                .db
                .update_task(&event_id, "failed", &format!("投稿失败: {e}"), None)
                .await?;
            Err(e)
        }
    }
}

async fn upload_procedure_inner(
    state: &state_type!(),
    reporter: &ProgressReporter,
    uid: String,
    room_id: String,
    video_id: i64,
    mut profile: Profile,
) -> Result<String, String> {
    let account = state.db.get_account("bilibili", &uid).await?;
    // get video info from dbs
    let mut video_row = state.db.get_video(video_id).await?;
    // construct file path
    let output = state.config.read().await.output.clone();
    let file = Path::new(&output).join(&video_row.file);
    let path = Path::new(&file);
    let client = reqwest::Client::new();

    let cover_path = file.with_extension("jpg");
    let cover_bytes = tokio::fs::read(&cover_path).await.map_err(|e| {
        log::error!("Read cover file error: {} {}", e, cover_path.display());
        e.to_string()
    })?;
    let cover_base64 = format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(cover_bytes)
    );
    let cover_url =
        bilibili::api::upload_cover(&client, &account.to_account(), &cover_base64).await;

    reporter.update("投稿预处理中").await;

    match bilibili::api::prepare_video(&client, &account.to_account(), path).await {
        Ok(video) => {
            profile.cover = cover_url.unwrap_or(String::new());
            if let Ok(ret) =
                bilibili::api::submit_video(&client, &account.to_account(), &profile, &video).await
            {
                // update video status and details
                // 1 means uploaded
                video_row.status = 1;
                video_row.bvid = ret.bvid.clone();
                video_row.title = profile.title;
                video_row.desc = profile.desc;
                video_row.tags = profile.tag;
                video_row.area = profile.tid as i64;
                state.db.update_video(&video_row).await?;
                state
                    .db
                    .new_message(
                        "投稿成功",
                        &format!("投稿了房间 {} 的切片：{}", room_id, ret.bvid),
                    )
                    .await?;
                if state.config.read().await.post_notify {
                    #[cfg(feature = "gui")]
                    state
                        .app_handle
                        .notification()
                        .builder()
                        .title("BiliShadowReplay - 投稿成功")
                        .body(format!("投稿了房间 {} 的切片: {}", room_id, ret.bvid))
                        .show()
                        .unwrap();
                }
                reporter.finish(true, "投稿成功").await;
                Ok(ret.bvid)
            } else {
                reporter.finish(false, "投稿失败").await;
                Err("Submit video failed".to_string())
            }
        }
        Err(e) => {
            reporter
                .finish(false, &format!("Preload video failed: {e}"))
                .await;
            Err(format!("Preload video failed: {e}"))
        }
    }
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn cancel(state: state_type!(), event_id: String) -> Result<(), String> {
    log::info!("Cancel task: {event_id}");
    let cancel_result = state.task_manager.cancel_task(&event_id).await;
    match cancel_result {
        Ok(()) => {
            state
                .db
                .update_task(&event_id, "cancelled", "任务取消", None)
                .await?;
        }
        Err(e) if e == "Task not found" => {
            let task = state.db.get_task(&event_id).await?;
            if matches!(task.status.as_str(), "pending" | "processing") {
                state
                    .db
                    .update_task(&event_id, "cancelled", "任务取消", None)
                    .await?;
            }
        }
        Err(e) => return Err(e),
    }
    Ok(())
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn get_video(state: state_type!(), id: i64) -> Result<VideoRow, String> {
    Ok(state.db.get_video(id).await?)
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn get_videos(state: state_type!(), room_id: String) -> Result<Vec<VideoRow>, String> {
    state
        .db
        .get_videos(&room_id)
        .await
        .map_err(|e| e.to_string())
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn get_all_videos(state: state_type!()) -> Result<Vec<VideoRow>, String> {
    state.db.get_all_videos().await.map_err(|e| e.to_string())
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn get_video_cover(state: state_type!(), id: i64) -> Result<String, String> {
    state
        .db
        .get_video_cover(id)
        .await
        .map_err(|e| e.to_string())
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn delete_video(state: state_type!(), id: i64) -> Result<(), String> {
    // get video info from db
    let video = state.db.get_video(id).await?;
    let config = state.config.read().await;

    // Emit webhook events
    let event =
        events::new_webhook_event(events::CLIP_DELETED, events::Payload::Clip(video.clone()));
    if let Err(e) = state.webhook_poster.post_event(&event).await {
        log::error!("Post webhook event error: {e}");
    }

    // delete video from db
    state.db.delete_video(id).await?;

    // delete video files
    let filepath = Path::new(&config.output).join(&video.file);
    let file = Path::new(&filepath);
    if let Err(e) = std::fs::remove_file(file) {
        log::warn!("删除视频文件失败: {} - {}", file.display(), e);
    } else {
        log::info!("已删除视频文件: {}", file.display());
    }

    // delete all related files
    let srt_path = file.with_extension("srt");
    let _ = tokio::fs::remove_file(srt_path).await;
    let wav_path = file.with_extension("wav");
    let _ = tokio::fs::remove_file(wav_path).await;
    let mp3_path = file.with_extension("mp3");
    let _ = tokio::fs::remove_file(mp3_path).await;
    let opus_path = file.with_extension("opus");
    let _ = tokio::fs::remove_file(opus_path).await;
    let waveform_path = file.with_extension("waveform.json");
    let _ = tokio::fs::remove_file(waveform_path).await;
    let danmu_path = file.with_extension("danmu.txt");
    let _ = tokio::fs::remove_file(danmu_path).await;
    let pbp_path = file.with_extension("pbp.json");
    let _ = tokio::fs::remove_file(pbp_path).await;
    let cover_path = Path::new(&config.output).join(&video.cover);
    let _ = tokio::fs::remove_file(cover_path).await;

    Ok(())
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn get_video_typelist(
    state: state_type!(),
) -> Result<Vec<bilibili::response::Typelist>, String> {
    let account = state.db.get_account_by_platform("bilibili").await?;
    let client = reqwest::Client::new();
    match bilibili::api::get_video_typelist(&client, &account.to_account()).await {
        Ok(typelist) => Ok(typelist),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn update_video_cover(
    state: state_type!(),
    id: i64,
    cover: String,
) -> Result<(), String> {
    let video = state.db.get_video(id).await?;
    let output_root = PathBuf::from(state.config.read().await.output.as_str());
    let output_path = output_root.join(&video.file);
    let cover_path = output_path.with_extension("jpg");
    // decode cover and write into file
    let base64 = cover.split("base64,").nth(1).unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64)
        .unwrap();
    tokio::fs::write(&cover_path, bytes)
        .await
        .map_err(|e| e.to_string())?;
    let cover_relative_path = to_output_relative_path(&output_root, &cover_path);
    let cover_relative = cover_relative_path.to_string_lossy().to_string();
    log::debug!("Update video cover: {id} {cover_relative}");
    Ok(state.db.update_video_cover(id, &cover_relative).await?)
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn get_video_subtitle(state: state_type!(), id: i64) -> Result<String, String> {
    log::debug!("Get video subtitle: {id}");
    let video = state.db.get_video(id).await?;
    let filepath = Path::new(state.config.read().await.output.as_str()).join(&video.file);
    let file = Path::new(&filepath);
    // read file content
    if let Ok(content) = std::fs::read_to_string(file.with_extension("srt")) {
        Ok(content)
    } else {
        Ok(String::new())
    }
}

fn parse_danmu_entries(content: &str) -> Vec<DanmuEntry> {
    let mut entries = content
        .lines()
        .filter_map(|line| {
            let (ts, content) = line.split_once(':')?;
            let ts = ts.parse::<i64>().ok()?;
            let (content, render_emotes) = decode_danmu_content(content);
            Some(DanmuEntry {
                ts,
                content,
                render_emotes,
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.ts);
    entries
}

fn serialize_danmu_entries(entries: &[DanmuEntry]) -> String {
    let mut serialized = String::new();
    for entry in entries {
        serialized.push_str(&entry.ts.to_string());
        serialized.push(':');
        serialized.push_str(&encode_danmu_content(&entry.content, entry.render_emotes));
        serialized.push('\n');
    }
    serialized
}

async fn clip_video_danmu_sidecar(
    input_path: &Path,
    output_path: &Path,
    ranges: &[ffmpeg::Range],
    render_danmu_emotes: bool,
    danmu_render_options: &danmu2ass::DanmuRenderOptions,
) -> Result<Vec<DanmuEntry>, String> {
    let input_danmu_path = input_path.with_extension("danmu.txt");
    let output_danmu_path = output_path.with_extension("danmu.txt");
    if !input_danmu_path.exists() {
        let _ = tokio::fs::remove_file(&output_danmu_path).await;
        return Ok(Vec::new());
    }

    let content = tokio::fs::read_to_string(&input_danmu_path)
        .await
        .map_err(|e| format!("读取原弹幕失败: {e}"))?;
    let entries = parse_danmu_entries(&content);
    let mut clipped_entries = Vec::new();
    let mut anchor_ms = 0_i64;
    let lookback_ms = danmu2ass::danmu_max_active_duration_ms(danmu_render_options);
    for range in ranges {
        let start_ms = (range.start * 1000.0).round() as i64;
        let end_ms = (range.end * 1000.0).round() as i64;
        clipped_entries.extend(
            entries
                .iter()
                .filter(|entry| entry.ts >= start_ms - lookback_ms && entry.ts < end_ms)
                .map(|entry| DanmuEntry {
                    ts: entry.ts - start_ms + anchor_ms,
                    content: entry.content.clone(),
                    render_emotes: entry.render_emotes && render_danmu_emotes,
                }),
        );
        anchor_ms += (range.duration() * 1000.0).round() as i64;
    }

    if clipped_entries.is_empty() {
        let _ = tokio::fs::remove_file(&output_danmu_path).await;
        return Ok(Vec::new());
    }

    tokio::fs::write(
        &output_danmu_path,
        serialize_danmu_entries(&clipped_entries),
    )
    .await
    .map_err(|e| format!("写入切片弹幕失败: {e}"))?;
    Ok(clipped_entries)
}

fn srt_time_to_ms(time: &srtparse::Time) -> i64 {
    ((time.hours * 3_600 + time.minutes * 60 + time.seconds) * 1_000 + time.milliseconds) as i64
}

fn ms_to_srt_time(ms: i64) -> srtparse::Time {
    let ms = ms.max(0) as u64;
    let total_seconds = ms / 1_000;
    srtparse::Time {
        hours: total_seconds / 3_600,
        minutes: (total_seconds % 3_600) / 60,
        seconds: total_seconds % 60,
        milliseconds: ms % 1_000,
    }
}

async fn clip_video_subtitle_sidecar(
    input_path: &Path,
    output_path: &Path,
    ranges: &[ffmpeg::Range],
) -> Result<bool, String> {
    let input_subtitle_path = input_path.with_extension("srt");
    let output_subtitle_path = output_path.with_extension("srt");
    if !input_subtitle_path.exists() {
        let _ = tokio::fs::remove_file(&output_subtitle_path).await;
        return Ok(false);
    }

    let content = tokio::fs::read_to_string(&input_subtitle_path)
        .await
        .map_err(|e| format!("读取原字幕失败: {e}"))?;
    if content.trim().is_empty() {
        let _ = tokio::fs::remove_file(&output_subtitle_path).await;
        return Ok(false);
    }

    let items = srtparse::from_str(&content).map_err(|e| format!("解析原字幕失败: {e}"))?;
    let mut clipped_items = Vec::new();
    let mut anchor_ms = 0_i64;
    for range in ranges {
        let range_start_ms = (range.start * 1000.0).round() as i64;
        let range_end_ms = (range.end * 1000.0).round() as i64;
        for item in &items {
            let item_start_ms = srt_time_to_ms(&item.start_time);
            let item_end_ms = srt_time_to_ms(&item.end_time);
            let clipped_start_ms = item_start_ms.max(range_start_ms);
            let clipped_end_ms = item_end_ms.min(range_end_ms);
            if clipped_end_ms <= clipped_start_ms {
                continue;
            }

            let mut clipped_item = item.clone();
            clipped_item.pos = clipped_items.len() + 1;
            clipped_item.start_time = ms_to_srt_time(clipped_start_ms - range_start_ms + anchor_ms);
            clipped_item.end_time = ms_to_srt_time(clipped_end_ms - range_start_ms + anchor_ms);
            clipped_items.push(clipped_item);
        }
        anchor_ms += (range.duration() * 1000.0).round() as i64;
    }

    if clipped_items.is_empty() {
        let _ = tokio::fs::remove_file(&output_subtitle_path).await;
        return Ok(false);
    }

    let subtitle = clipped_items
        .iter()
        .map(item_to_srt)
        .collect::<Vec<_>>()
        .join("");
    tokio::fs::write(&output_subtitle_path, subtitle)
        .await
        .map_err(|e| format!("写入切片字幕失败: {e}"))?;
    Ok(true)
}

async fn read_video_danmu_entries(
    input_path: &Path,
    render_danmu_emotes: bool,
) -> Result<Vec<DanmuEntry>, String> {
    let input_danmu_path = input_path.with_extension("danmu.txt");
    if !input_danmu_path.exists() {
        return Ok(Vec::new());
    }

    let content = tokio::fs::read_to_string(&input_danmu_path)
        .await
        .map_err(|e| format!("读取弹幕失败: {e}"))?;
    Ok(parse_danmu_entries(&content)
        .into_iter()
        .map(|entry| DanmuEntry {
            ts: entry.ts,
            content: entry.content,
            render_emotes: entry.render_emotes && render_danmu_emotes,
        })
        .collect())
}

async fn burn_video_subtitle(
    reporter: &ProgressReporter,
    input_path: &Path,
    subtitle_path: &Path,
    srt_style: &str,
    output_path: &Path,
) -> Result<PathBuf, String> {
    ffmpeg::encode_video_subtitle_to_path(
        reporter,
        input_path,
        subtitle_path,
        srt_style.to_string(),
        output_path,
    )
    .await
}

async fn burn_video_danmu(
    state: &state_type!(),
    reporter: Option<&ProgressReporter>,
    input_path: &Path,
    entries: Vec<DanmuEntry>,
    render_danmu_emotes: bool,
    danmu_render_options: Option<danmu2ass::DanmuRenderOptions>,
    output_path: &Path,
) -> Result<Option<PathBuf>, String> {
    if entries.is_empty() {
        return Ok(None);
    }

    let config = state.config.read().await;
    let emote_files = if render_danmu_emotes {
        danmaku_emote_file_map(state)
    } else {
        HashMap::new()
    };
    let render_options =
        danmu_render_options.unwrap_or_else(|| config.danmu_ass_options.clone().into());
    let danmu_render = danmu2ass::danmu_to_ass_with_emotes(entries, render_options, &emote_files);
    drop(config);

    let ass_file_path = input_path.with_extension("ass");
    tokio::fs::write(&ass_file_path, danmu_render.ass_content)
        .await
        .map_err(|e| format!("写入弹幕 ASS 失败: {e}"))?;

    let encoded_result = if render_danmu_emotes && !danmu_render.image_overlays.is_empty() {
        ffmpeg::encode_video_danmu_with_images_to_path(
            reporter,
            input_path,
            &ass_file_path,
            &danmu_render.image_overlays,
            output_path,
        )
        .await
    } else {
        ffmpeg::encode_video_danmu_to_path(reporter, input_path, &ass_file_path, output_path).await
    };
    let _ = tokio::fs::remove_file(&ass_file_path).await;
    if encoded_result.is_err() {
        let _ = tokio::fs::remove_file(output_path).await;
    }
    encoded_result.map(Some)
}

fn danmaku_emote_file_map(state: &state_type!()) -> HashMap<String, PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    #[cfg(feature = "gui")]
    {
        if let Ok(resource_dir) = state.app_handle.path().resource_dir() {
            candidates.push(resource_dir.join("danmaku-emotes"));
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("public").join("danmaku-emotes"));
        candidates.push(cwd.join("dist").join("danmaku-emotes"));
        if let Some(parent) = cwd.parent() {
            candidates.push(parent.join("public").join("danmaku-emotes"));
            candidates.push(parent.join("dist").join("danmaku-emotes"));
        }
    }

    let Some(dir) = candidates.into_iter().find(|candidate| candidate.is_dir()) else {
        return HashMap::new();
    };

    let Ok(entries) = std::fs::read_dir(dir) else {
        return HashMap::new();
    };

    entries
        .flatten()
        .filter_map(|entry| {
            if !entry.file_type().map(|ty| ty.is_file()).unwrap_or(false) {
                return None;
            }
            let path = entry.path();
            if path
                .extension()
                .and_then(|ext| ext.to_str())?
                .to_lowercase()
                != "png"
            {
                return None;
            }
            let stem = path.file_stem()?.to_str()?;
            Some((format!("[{stem}]"), path))
        })
        .collect()
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn get_video_danmu(state: state_type!(), id: i64) -> Result<Vec<DanmuEntry>, String> {
    log::debug!("Get video danmu: {id}");
    let video = state.db.get_video(id).await?;
    let filepath = Path::new(state.config.read().await.output.as_str()).join(&video.file);
    let danmu_path = filepath.with_extension("danmu.txt");

    match tokio::fs::read_to_string(&danmu_path).await {
        Ok(content) => Ok(parse_danmu_entries(&content)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("读取弹幕失败: {e}")),
    }
}

fn serialize_video_pbp(data: &bilibili_danmaku::ImportedVideoPbpData) -> Result<String, String> {
    serde_json::to_string(data).map_err(|e| format!("序列化高能进度条失败: {e}"))
}

async fn save_video_pbp_sidecar(
    video_path: &Path,
    data: &bilibili_danmaku::ImportedVideoPbpData,
) -> Result<(), String> {
    tokio::fs::write(
        video_path.with_extension("pbp.json"),
        serialize_video_pbp(data)?,
    )
    .await
    .map_err(|e| format!("保存高能进度条失败: {e}"))
}

async fn remove_video_pbp_sidecar(video_path: &Path) {
    let _ = tokio::fs::remove_file(video_path.with_extension("pbp.json")).await;
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn get_video_pbp(
    state: state_type!(),
    id: i64,
) -> Result<Option<bilibili_danmaku::ImportedVideoPbpData>, String> {
    log::debug!("Get video pbp: {id}");
    let video = state.db.get_video(id).await?;
    let filepath = Path::new(state.config.read().await.output.as_str()).join(&video.file);
    let pbp_path = filepath.with_extension("pbp.json");

    match tokio::fs::read_to_string(&pbp_path).await {
        Ok(content) => serde_json::from_str(&content)
            .map(Some)
            .map_err(|e| format!("读取高能进度条失败: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("读取高能进度条失败: {e}")),
    }
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn generate_video_subtitle(
    state: state_type!(),
    event_id: String,
    id: i64,
) -> Result<String, String> {
    generate_video_subtitle_inner(&state, event_id, id).await
}

async fn generate_video_subtitle_inner(
    state: &State,
    event_id: String,
    id: i64,
) -> Result<String, String> {
    #[cfg(feature = "gui")]
    let emitter = EventEmitter::new(state.app_handle.clone());
    #[cfg(feature = "headless")]
    let emitter = EventEmitter::new(state.progress_manager.get_event_sender());
    let reporter = ProgressReporter::new(state.db.clone(), &emitter, &event_id).await?;
    let task = TaskRow {
        id: event_id.clone(),
        task_type: "generate_video_subtitle".to_string(),
        status: "pending".to_string(),
        message: String::new(),
        metadata: json!({
            "video_id": id,
        })
        .to_string(),
        created_at: Utc::now().to_rfc3339(),
    };
    state.db.add_task(&task).await?;
    log::info!("Create task: {task:?}");
    let (
        generator_type,
        whisper_model,
        whisper_prompt,
        openai_api_key,
        openai_api_endpoint,
        online_asr_model,
        oss_access_key_id,
        oss_access_key_secret,
        oss_bucket,
        oss_endpoint,
        oss_object_prefix,
        asr_hotword_vocabulary_id,
        language_hint,
    ) = {
        let mut config = state.config.write().await;
        if config.subtitle_generator_type == "whisper_online" {
            reporter.update("同步 ASR 热词").await;
            if let Err(error) =
                crate::handlers::config::ensure_asr_hotwords_synced(&mut config).await
            {
                drop(config);
                reporter
                    .finish(false, &format!("ASR 热词同步失败: {error}"))
                    .await;
                state
                    .db
                    .update_task(
                        &event_id,
                        "failed",
                        &format!("ASR 热词同步失败: {error}"),
                        None,
                    )
                    .await?;
                return Err(error);
            }
        }
        (
            config.subtitle_generator_type.clone(),
            config.whisper_model.clone(),
            config.whisper_prompt.clone(),
            config.openai_api_key.clone(),
            config.openai_api_endpoint.clone(),
            config.online_asr_model.clone(),
            config.oss_access_key_id.clone(),
            config.oss_access_key_secret.clone(),
            config.oss_bucket.clone(),
            config.oss_endpoint.clone(),
            config.oss_object_prefix.clone(),
            config.asr_hotwords.vocabulary_id.clone(),
            config.whisper_language.clone(),
        )
    };
    let language_hint = language_hint.as_str();

    let video = state.db.get_video(id).await?;
    let filepath = Path::new(state.config.read().await.output.as_str()).join(&video.file);
    let file = Path::new(&filepath);

    match ffmpeg::generate_video_subtitle(
        Some(&reporter),
        file,
        &generator_type,
        &whisper_model,
        &whisper_prompt,
        &openai_api_key,
        &openai_api_endpoint,
        &online_asr_model,
        &oss_access_key_id,
        &oss_access_key_secret,
        &oss_bucket,
        &oss_endpoint,
        &oss_object_prefix,
        &asr_hotword_vocabulary_id,
        language_hint,
    )
    .await
    {
        Ok(result) => {
            reporter.finish(true, "字幕生成完成").await;
            // for local whisper, we need to update the task status to success
            state
                .db
                .update_task(
                    &event_id,
                    "success",
                    "字幕生成完成",
                    Some(
                        json!({
                            "task_id": result.subtitle_id,
                            "service": result.generator_type.as_str(),
                        })
                        .to_string()
                        .as_str(),
                    ),
                )
                .await?;

            let subtitle = result
                .subtitle_content
                .iter()
                .map(item_to_srt)
                .collect::<String>();

            let result = update_video_subtitle_inner(state, id, subtitle.clone()).await;
            if let Err(e) = result {
                log::error!("Update video subtitle error: {e}");
            }
            Ok(subtitle)
        }
        Err(e) => {
            reporter.finish(false, &format!("字幕生成失败: {e}")).await;
            state
                .db
                .update_task(&event_id, "failed", &format!("字幕生成失败: {e}"), None)
                .await?;
            Err(e)
        }
    }
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn update_video_subtitle(
    state: state_type!(),
    id: i64,
    subtitle: String,
) -> Result<(), String> {
    update_video_subtitle_inner(&state, id, subtitle).await
}

async fn update_video_subtitle_inner(
    state: &State,
    id: i64,
    subtitle: String,
) -> Result<(), String> {
    let video = state.db.get_video(id).await?;
    let filepath = Path::new(state.config.read().await.output.as_str()).join(&video.file);
    let file = Path::new(&filepath);
    let subtitle_path = file.with_extension("srt");
    log::info!(
        "Update video subtitle: writing srt path={} bytes={}",
        subtitle_path.display(),
        subtitle.len()
    );
    if let Err(e) = std::fs::write(subtitle_path, subtitle) {
        log::warn!("Update video subtitle error: {e}");
    }
    Ok(())
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn update_video_note(state: state_type!(), id: i64, note: String) -> Result<(), String> {
    log::info!("Update video note: {id} -> {note}");
    let mut video = state.db.get_video(id).await?;
    video.note = note;
    state.db.update_video(&video).await?;
    Ok(())
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn encode_video_subtitle(
    state: state_type!(),
    event_id: String,
    id: i64,
    srt_style: String,
) -> Result<VideoRow, String> {
    encode_video_media(state, event_id, id, true, false, true, srt_style, None).await
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn encode_video_media(
    state: state_type!(),
    event_id: String,
    id: i64,
    include_subtitle: bool,
    include_danmu: bool,
    render_danmu_emotes: bool,
    srt_style: String,
    danmu_render_options: Option<danmu2ass::DanmuRenderOptions>,
) -> Result<VideoRow, String> {
    if !include_subtitle && !include_danmu {
        return Err("请至少选择字幕或弹幕".to_string());
    }

    #[cfg(feature = "gui")]
    let emitter = EventEmitter::new(state.app_handle.clone());
    #[cfg(feature = "headless")]
    let emitter = EventEmitter::new(state.progress_manager.get_event_sender());
    let reporter = ProgressReporter::new(state.db.clone(), &emitter, &event_id).await?;
    let task = TaskRow {
        id: event_id.clone(),
        task_type: "encode_video_media".to_string(),
        status: "pending".to_string(),
        message: String::new(),
        metadata: json!({
            "video_id": id,
            "include_subtitle": include_subtitle,
            "include_danmu": include_danmu,
            "render_danmu_emotes": render_danmu_emotes,
            "srt_style": srt_style,
            "danmu_render_options": danmu_render_options.clone(),
        })
        .to_string(),
        created_at: Utc::now().to_rfc3339(),
    };
    state.db.add_task(&task).await?;
    log::info!("Create task: {task:?}");
    match encode_video_media_inner(
        &state,
        &reporter,
        id,
        include_subtitle,
        include_danmu,
        render_danmu_emotes,
        srt_style,
        danmu_render_options,
    )
    .await
    {
        Ok(video) => {
            reporter.finish(true, "压制完成").await;
            state
                .db
                .update_task(&event_id, "success", "压制完成", None)
                .await?;
            Ok(video)
        }
        Err(e) => {
            reporter.finish(false, &format!("压制失败: {e}")).await;
            state
                .db
                .update_task(&event_id, "failed", &format!("压制失败: {e}"), None)
                .await?;
            Err(e)
        }
    }
}

async fn encode_video_media_inner(
    state: &state_type!(),
    reporter: &ProgressReporter,
    id: i64,
    include_subtitle: bool,
    include_danmu: bool,
    render_danmu_emotes: bool,
    srt_style: String,
    danmu_render_options: Option<danmu2ass::DanmuRenderOptions>,
) -> Result<VideoRow, String> {
    let video = state.db.get_video(id).await?;
    let config = state.config.read().await;
    let output_root = PathBuf::from(config.output.clone());
    drop(config);
    let filepath = output_root.join(&video.file);
    let subtitle_path = filepath.with_extension("srt");

    if include_subtitle && !subtitle_path.exists() {
        return Err("字幕文件不存在".to_string());
    }
    let danmu_entries = if include_danmu {
        let entries = read_video_danmu_entries(&filepath, render_danmu_emotes).await?;
        if entries.is_empty() {
            return Err("弹幕文件不存在或为空".to_string());
        }
        entries
    } else {
        Vec::new()
    };

    let mut final_output_full_path = filepath.clone();
    let mut intermediate_paths = Vec::new();
    if include_subtitle {
        reporter.update("正在压制字幕").await;
        let output_path = final_output_full_path.with_file_name(format!(
            "{}{}",
            crate::constants::PREFIX_SUBTITLE,
            final_output_full_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("video.mp4")
        ));
        let encoded_path = burn_video_subtitle(
            reporter,
            &final_output_full_path,
            &subtitle_path,
            &srt_style,
            &output_path,
        )
        .await?;
        if final_output_full_path != filepath {
            intermediate_paths.push(final_output_full_path);
        }
        final_output_full_path = encoded_path;
    }
    if include_danmu {
        reporter.update("正在压制弹幕").await;
        let output_path = final_output_full_path.with_file_name(format!(
            "{}{}",
            crate::constants::PREFIX_DANMAKU,
            final_output_full_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("video.mp4")
        ));
        if let Some(encoded_path) = burn_video_danmu(
            state,
            Some(reporter),
            &final_output_full_path,
            danmu_entries,
            render_danmu_emotes,
            danmu_render_options,
            &output_path,
        )
        .await?
        {
            if final_output_full_path != filepath {
                intermediate_paths.push(final_output_full_path);
            }
            final_output_full_path = encoded_path;
        }
    }

    if final_output_full_path == filepath {
        return Err("没有生成压制视频".to_string());
    }
    if subtitle_path.exists() {
        let final_subtitle_path = final_output_full_path.with_extension("srt");
        tokio::fs::copy(&subtitle_path, &final_subtitle_path)
            .await
            .map_err(|e| format!("复制压制视频字幕失败: {e}"))?;
    }
    for path in intermediate_paths {
        let _ = tokio::fs::remove_file(path).await;
    }

    let output_relative_path = to_output_relative_path(&output_root, &final_output_full_path);
    let file_metadata = final_output_full_path
        .metadata()
        .map_err(|e| e.to_string())?;
    let title_suffix = match (include_subtitle, include_danmu) {
        (true, true) => " (压制字幕弹幕)",
        (true, false) => " (压制字幕)",
        (false, true) => " (压制弹幕)",
        (false, false) => "",
    };

    let new_video = state
        .db
        .add_video(&VideoRow {
            id: 0,
            status: video.status,
            room_id: video.room_id,
            created_at: Local::now().to_rfc3339(),
            cover: video.cover.clone(),
            file: output_relative_path.to_string_lossy().to_string(),
            note: video.note.clone(),
            length: video.length,
            size: i64::try_from(file_metadata.len()).map_err(|e| e.to_string())?,
            bvid: video.bvid.clone(),
            title: format!("{}{}", video.title, title_suffix),
            desc: video.desc.clone(),
            tags: video.tags.clone(),
            area: video.area,
            platform: video.platform,
        })
        .await?;

    Ok(new_video)
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn generic_ffmpeg_command(
    _state: state_type!(),
    args: Vec<String>,
) -> Result<String, String> {
    let args_str: Vec<&str> = args.iter().map(std::string::String::as_str).collect();
    ffmpeg::generic_ffmpeg_command(&args_str).await
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn import_external_video(
    state: state_type!(),
    event_id: String,
    file_path: String,
    title: String,
    room_id: String,
) -> Result<VideoRow, String> {
    #[cfg(feature = "gui")]
    let emitter = EventEmitter::new(state.app_handle.clone());
    #[cfg(feature = "headless")]
    let emitter = EventEmitter::new(state.progress_manager.get_event_sender());

    let reporter = ProgressReporter::new(state.db.clone(), &emitter, &event_id).await?;

    let source_path = Path::new(&file_path);
    if !source_path.exists() {
        return Err("文件不存在".to_string());
    }

    reporter.update("正在提取视频元数据...").await;
    let metadata = ffmpeg::extract_video_metadata(source_path).await?;
    let output_root = PathBuf::from(state.config.read().await.output.clone());
    let sanitized_room_id = sanitize_filename(&room_id);
    let sanitized_video_name = sanitize_filename(&title);
    let video_folder_name = if sanitized_video_name.is_empty() {
        "untitled".to_string()
    } else {
        sanitized_video_name.clone()
    };
    let output_dir = output_root
        .join("imported")
        .join(&sanitized_room_id)
        .join(video_folder_name);
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
    }

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let extension = source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("mp4");
    let target_basename = format!(
        "{}{}{}.{}",
        crate::constants::PREFIX_IMPORTED,
        sanitized_video_name,
        timestamp,
        extension
    );
    let target_full_path = output_dir.join(&target_basename);

    let need_conversion = should_convert_video_format(extension);
    let final_target_full_path = if need_conversion {
        let mp4_target_full_path = target_full_path.with_extension("mp4");

        reporter.update("准备转换视频格式 (FLV → MP4)...").await;

        copy_and_convert_with_progress(source_path, &mp4_target_full_path, true, &reporter).await?;

        mp4_target_full_path
    } else {
        // 其他格式使用智能拷贝
        copy_and_convert_with_progress(source_path, &target_full_path, false, &reporter).await?;
        target_full_path
    };

    // 步骤3: 生成缩略图
    reporter.update("正在生成视频缩略图...").await;

    // 生成缩略图，使用智能时间点选择
    let thumbnail_timestamp = get_optimal_thumbnail_timestamp(metadata.duration);
    let cover_path =
        match ffmpeg::generate_thumbnail(&final_target_full_path, thumbnail_timestamp).await {
            Ok(path) => to_output_relative_path(&output_root, &path)
                .to_string_lossy()
                .to_string(),
            Err(e) => {
                log::warn!("生成缩略图失败: {e}");
                String::new() // 使用空字符串，前端会显示默认图标
            }
        };

    // 步骤4: 保存到数据库
    reporter.update("正在保存视频信息...").await;

    let Ok(size) = i64::try_from(
        final_target_full_path
            .metadata()
            .map_err(|e| e.to_string())?
            .len(),
    ) else {
        log::error!(
            "Failed to convert metadata length to i64: {}",
            final_target_full_path
                .metadata()
                .map_err(|e| e.to_string())?
                .len()
        );
        return Err("Failed to convert metadata length to i64".to_string());
    };

    // 添加到数据库
    let target_relative_path = to_output_relative_path(&output_root, &final_target_full_path);
    let video = VideoRow {
        id: 0,
        room_id, // 使用传入的 room_id
        platform: "imported".to_string(),
        title,
        file: target_relative_path.to_string_lossy().to_string(),
        note: String::new(),
        length: metadata.duration as i64,
        size,
        status: 1, // 导入完成
        cover: cover_path,
        desc: String::new(),
        tags: String::new(),
        bvid: String::new(),
        area: 0,
        created_at: Utc::now().to_rfc3339(),
    };

    let result = state.db.add_video(&video).await?;

    // 完成进度通知
    reporter.finish(true, "视频导入完成").await;

    // 发送通知消息
    state
        .db
        .new_message("视频导入完成", &format!("成功导入视频：{}", result.title))
        .await?;

    log::info!("导入视频成功: {} -> {}", file_path, result.file);
    Ok(result)
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn download_video_danmu(
    state: state_type!(),
    event_id: String,
    video_id: i64,
    bvid: String,
    page: i32,
) -> Result<bilibili_danmaku::ImportedVideoDanmuDownload, String> {
    #[cfg(feature = "gui")]
    let emitter = EventEmitter::new(state.app_handle.clone());
    #[cfg(feature = "headless")]
    let emitter = EventEmitter::new(state.progress_manager.get_event_sender());
    let reporter = ProgressReporter::new(state.db.clone(), &emitter, &event_id).await?;

    let outcome = async {
        let video = state.db.get_video(video_id).await?;
        if video.platform != "imported" {
            return Err("只有导入视频支持下载弹幕".to_string());
        }
        let account = state
            .db
            .get_account_by_platform("bilibili")
            .await
            .map_err(|_| "请先添加一个带 SESSDATA 的 B 站账号".to_string())?;
        if !account.cookies.contains("SESSDATA=") {
            return Err("当前 B 站账号 cookies 缺少 SESSDATA".to_string());
        }

        let output_dir = state.config.read().await.output.clone();
        let video_path = Path::new(&output_dir).join(&video.file);
        if !video_path.exists() {
            return Err("视频文件不存在，无法保存弹幕".to_string());
        }

        reporter.update("正在获取 BV 视频信息...").await;

        let client = reqwest::Client::new();
        let download =
            bilibili_danmaku::download_video_danmaku(&client, &bvid, page, Some(&account.cookies))
                .await?;

        reporter.update("正在保存弹幕文件...").await;

        let mut serialized = String::new();
        for entry in &download.entries {
            serialized.push_str(&entry.ts.to_string());
            serialized.push(':');
            serialized.push_str(&entry.content);
            serialized.push('\n');
        }

        let danmu_path = video_path.with_extension("danmu.txt");
        tokio::fs::write(&danmu_path, serialized)
            .await
            .map_err(|e| format!("保存弹幕文件失败: {e}"))?;

        if video.bvid != download.download.bvid {
            let mut updated_video = video.clone();
            updated_video.bvid = download.download.bvid.clone();
            state.db.update_video(&updated_video).await?;
        }

        match bilibili_danmaku::download_video_pbp(
            &client,
            &download.download.bvid,
            download.download.aid,
            download.download.cid,
            download.download.page,
            Some(&account.cookies),
        )
        .await
        {
            Ok(Some(pbp_data)) => {
                if let Err(err) = save_video_pbp_sidecar(&video_path, &pbp_data).await {
                    remove_video_pbp_sidecar(&video_path).await;
                    log::warn!("{err}");
                } else {
                    log::info!(
                        "高能进度条下载完成: {} cid={} points={}",
                        download.download.bvid,
                        download.download.cid,
                        pbp_data.values.len()
                    );
                }
            }
            Ok(None) => {
                remove_video_pbp_sidecar(&video_path).await;
                log::info!(
                    "该视频没有可用高能进度条: {} cid={}",
                    download.download.bvid,
                    download.download.cid
                );
            }
            Err(err) => {
                remove_video_pbp_sidecar(&video_path).await;
                log::warn!("下载高能进度条失败，不影响弹幕下载: {err}");
            }
        }

        state
            .db
            .new_message(
                "弹幕下载完成",
                &format!(
                    "已为导入视频 {} 下载 {} 条弹幕（P{}）",
                    video.title, download.download.saved_count, download.download.page
                ),
            )
            .await?;

        Ok(download.download)
    }
    .await;

    match outcome {
        Ok(result) => {
            reporter
                .finish(
                    true,
                    &format!(
                        "弹幕下载完成，保存 {} 条（P{}）",
                        result.saved_count, result.page
                    ),
                )
                .await;
            Ok(result)
        }
        Err(err) => {
            reporter.finish(false, &err).await;
            Err(err)
        }
    }
}

// 通用视频切片函数（支持所有类型的视频）
#[cfg_attr(feature = "gui", tauri::command)]
pub async fn clip_video(
    state: state_type!(),
    event_id: String,
    parent_video_id: i64,
    ranges: Vec<ffmpeg::Range>,
    merge_ranges: bool,
    clip_title: String,
    include_subtitle: bool,
    include_danmu: bool,
    render_danmu_emotes: bool,
    srt_style: String,
    danmu_render_options: Option<danmu2ass::DanmuRenderOptions>,
) -> Result<Vec<VideoRow>, String> {
    // 获取父视频信息
    let parent_video = state.db.get_video(parent_video_id).await?;

    #[cfg(feature = "gui")]
    let emitter = EventEmitter::new(state.app_handle.clone());
    #[cfg(feature = "headless")]
    let emitter = EventEmitter::new(state.progress_manager.get_event_sender());
    let reporter = ProgressReporter::new(state.db.clone(), &emitter, &event_id).await?;

    // 创建任务记录
    let task = TaskRow {
        id: event_id.clone(),
        task_type: "clip_video".to_string(),
        status: "pending".to_string(),
        message: String::new(),
        metadata: json!({
            "parent_video_id": parent_video_id,
            "ranges": ranges.clone(),
            "merge_ranges": merge_ranges,
            "clip_title": clip_title,
            "include_subtitle": include_subtitle,
            "include_danmu": include_danmu,
            "render_danmu_emotes": render_danmu_emotes,
            "srt_style": srt_style,
            "danmu_render_options": danmu_render_options.clone(),
        })
        .to_string(),
        created_at: Utc::now().to_rfc3339(),
    };
    state.db.add_task(&task).await?;

    match clip_video_inner(
        &state,
        &reporter,
        parent_video,
        ranges,
        merge_ranges,
        clip_title,
        include_subtitle,
        include_danmu,
        render_danmu_emotes,
        srt_style,
        danmu_render_options,
    )
    .await
    {
        Ok(video) => {
            reporter.finish(true, "切片完成").await;
            state
                .db
                .update_task(&event_id, "success", "切片完成", None)
                .await?;
            Ok(video)
        }
        Err(e) => {
            reporter.finish(false, &format!("切片失败: {e}")).await;
            state
                .db
                .update_task(&event_id, "failed", &format!("切片失败: {e}"), None)
                .await?;
            Err(e)
        }
    }
}

async fn clip_ranges_from_video_file(
    reporter: Option<&ProgressReporter>,
    input_path: &Path,
    output_path: &Path,
    ranges: &[ffmpeg::Range],
    extension: &str,
) -> Result<(), String> {
    if ranges.len() == 1 {
        let range = &ranges[0];
        return ffmpeg::clip_from_video_file(
            reporter,
            input_path,
            output_path,
            range.start,
            range.duration(),
        )
        .await;
    }

    let output_stem = output_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("clip");
    let output_dir = output_path.parent().unwrap_or_else(|| Path::new("."));
    let mut part_paths = Vec::new();

    for (index, range) in ranges.iter().enumerate() {
        let part_path = output_dir.join(format!(
            "{}.part{:02}.{}",
            output_stem,
            index + 1,
            extension
        ));
        if let Err(e) = ffmpeg::clip_from_video_file(
            reporter,
            input_path,
            &part_path,
            range.start,
            range.duration(),
        )
        .await
        {
            for path in part_paths {
                let _ = tokio::fs::remove_file(path).await;
            }
            return Err(e);
        }
        part_paths.push(part_path);
    }

    let result = ffmpeg::general::concat_videos(reporter, &part_paths, output_path).await;
    for path in part_paths {
        let _ = tokio::fs::remove_file(path).await;
    }
    result
}

async fn clip_video_inner(
    state: &state_type!(),
    reporter: &ProgressReporter,
    parent_video: VideoRow,
    ranges: Vec<ffmpeg::Range>,
    merge_ranges: bool,
    clip_title: String,
    include_subtitle: bool,
    include_danmu: bool,
    render_danmu_emotes: bool,
    srt_style: String,
    danmu_render_options: Option<danmu2ass::DanmuRenderOptions>,
) -> Result<Vec<VideoRow>, String> {
    let mut ranges = ranges
        .into_iter()
        .filter(|range| range.end > range.start)
        .collect::<Vec<_>>();
    ranges.sort_by(|a, b| a.start.total_cmp(&b.start));
    if ranges.is_empty() {
        return Err("请至少选择一个有效选区".to_string());
    }
    if ranges.iter().any(|range| range.duration() < 1.0) {
        return Err("每个导出选区长度都不能少于1秒".to_string());
    }

    let output_root_path = {
        let config = state.config.read().await;
        PathBuf::from(config.output.clone())
    };
    let danmu_render_options = if let Some(options) = danmu_render_options {
        options
    } else {
        let config = state.config.read().await;
        config.danmu_ass_options.clone().into()
    };

    // 构建输入文件路径
    let input_path = output_root_path.join(&parent_video.file);

    if !input_path.exists() {
        return Err("原视频文件不存在".to_string());
    }
    if include_subtitle && !input_path.with_extension("srt").exists() {
        return Err("字幕文件不存在".to_string());
    }
    if include_danmu && !input_path.with_extension("danmu.txt").exists() {
        return Err("弹幕文件不存在".to_string());
    }

    let timestamp = Local::now().format("%Y%m%d%H%M").to_string();
    let extension = input_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("mp4");

    // 获取原文件名（不含扩展名）
    let original_filename = input_path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("video");

    // 输出目录：设置里的切片保存路径/outputs/原视频文件名/{clip|压制}
    let output_root = output_root_path.as_path();
    let output_base_dir = output_root.join("outputs").join(original_filename);
    let clip_output_dir = output_base_dir.join("clip");
    let encoded_output_dir = output_base_dir.join("压制");
    if !clip_output_dir.exists() {
        std::fs::create_dir_all(&clip_output_dir).map_err(|e| e.to_string())?;
    }
    if (include_subtitle || include_danmu) && !encoded_output_dir.exists() {
        std::fs::create_dir_all(&encoded_output_dir).map_err(|e| e.to_string())?;
    }

    let clip_groups = if merge_ranges || ranges.len() == 1 {
        vec![ranges]
    } else {
        ranges
            .into_iter()
            .map(|range| vec![range])
            .collect::<Vec<_>>()
    };
    let mut results = Vec::new();

    for (index, clip_ranges) in clip_groups.iter().enumerate() {
        let ordinal_suffix = if clip_groups.len() == 1 {
            String::new()
        } else {
            format!("-{:02}", index + 1)
        };
        let output_filename =
            clip_output_filename("[none]", &timestamp, &ordinal_suffix, extension);
        let output_full_path = clip_output_dir.join(&output_filename);

        reporter.update("开始切片处理").await;
        clip_ranges_from_video_file(
            Some(reporter),
            &input_path,
            &output_full_path,
            clip_ranges,
            extension,
        )
        .await?;

        let mut final_output_full_path = output_full_path.clone();
        let clipped_subtitle_available =
            clip_video_subtitle_sidecar(&input_path, &output_full_path, clip_ranges).await?;
        let clipped_danmus = if include_danmu {
            clip_video_danmu_sidecar(
                &input_path,
                &output_full_path,
                clip_ranges,
                render_danmu_emotes,
                &danmu_render_options,
            )
            .await?
        } else {
            let _ = tokio::fs::remove_file(output_full_path.with_extension("danmu.txt")).await;
            Vec::new()
        };
        let has_clipped_danmu = !clipped_danmus.is_empty();
        let burn_subtitle = include_subtitle && clipped_subtitle_available;
        let has_burned_media = burn_subtitle || has_clipped_danmu;
        let final_media_prefix = clip_media_prefix(burn_subtitle, has_clipped_danmu);
        let final_media_dir = if has_burned_media {
            &encoded_output_dir
        } else {
            &clip_output_dir
        };
        let final_media_path = final_media_dir.join(clip_output_filename(
            final_media_prefix,
            &timestamp,
            &ordinal_suffix,
            extension,
        ));

        if burn_subtitle {
            reporter.update("正在压制字幕").await;
            let subtitle_path = output_full_path.with_extension("srt");
            let subtitle_output_path = if has_clipped_danmu {
                encoded_output_dir.join(clip_output_filename(
                    crate::constants::PREFIX_SUBTITLE,
                    &timestamp,
                    &ordinal_suffix,
                    extension,
                ))
            } else {
                final_media_path.clone()
            };
            let encoded_path = burn_video_subtitle(
                reporter,
                &final_output_full_path,
                &subtitle_path,
                &srt_style,
                &subtitle_output_path,
            )
            .await?;
            let encoded_subtitle_path = encoded_path.with_extension("srt");
            let _ = tokio::fs::copy(&subtitle_path, &encoded_subtitle_path).await;
            let _ = tokio::fs::remove_file(&subtitle_path).await;
            if final_output_full_path != output_full_path {
                let _ = tokio::fs::remove_file(&final_output_full_path).await;
            } else {
                let _ = tokio::fs::remove_file(&output_full_path).await;
            }
            final_output_full_path = encoded_path;
        }

        if has_clipped_danmu {
            reporter.update("正在压制弹幕").await;
            if let Some(encoded_path) = burn_video_danmu(
                state,
                Some(reporter),
                &final_output_full_path,
                clipped_danmus,
                render_danmu_emotes,
                Some(danmu_render_options.clone()),
                &final_media_path,
            )
            .await?
            {
                let original_danmu_path = output_full_path.with_extension("danmu.txt");
                let encoded_danmu_path = encoded_path.with_extension("danmu.txt");
                if original_danmu_path.exists() {
                    let _ = tokio::fs::copy(&original_danmu_path, &encoded_danmu_path).await;
                    let _ = tokio::fs::remove_file(&original_danmu_path).await;
                }
                let final_subtitle_path = final_output_full_path.with_extension("srt");
                let encoded_subtitle_path = encoded_path.with_extension("srt");
                if final_subtitle_path.exists() {
                    let _ = tokio::fs::copy(&final_subtitle_path, &encoded_subtitle_path).await;
                    let _ = tokio::fs::remove_file(final_subtitle_path).await;
                }
                if final_output_full_path != output_full_path {
                    let _ = tokio::fs::remove_file(&final_output_full_path).await;
                } else {
                    let _ = tokio::fs::remove_file(&output_full_path).await;
                }
                final_output_full_path = encoded_path;
            }
        }
        remove_video_pbp_sidecar(&final_output_full_path).await;

        let thumbnail_full_path = final_output_full_path.with_extension("jpg");
        let clip_duration = clip_ranges
            .iter()
            .map(|range| range.duration())
            .sum::<f64>();
        let clip_thumbnail_timestamp = get_optimal_thumbnail_timestamp(clip_duration);
        let clip_cover_path =
            match ffmpeg::generate_thumbnail(&final_output_full_path, clip_thumbnail_timestamp)
                .await
            {
                Ok(_) => to_output_relative_path(output_root, &thumbnail_full_path)
                    .to_string_lossy()
                    .to_string(),
                Err(e) => {
                    log::warn!("生成切片缩略图失败: {e}");
                    String::new()
                }
            };

        let output_relative_path = to_output_relative_path(output_root, &final_output_full_path);
        let file_metadata = final_output_full_path
            .metadata()
            .map_err(|e| e.to_string())?;
        let row_title = if clip_groups.len() == 1 {
            clip_title.clone()
        } else {
            format!("{} {}", clip_title, index + 1)
        };
        let clip_video = VideoRow {
            id: 0,
            room_id: parent_video.room_id.clone(),
            platform: "clip".to_string(),
            title: row_title,
            file: output_relative_path.to_string_lossy().to_string(),
            note: String::new(),
            length: clip_duration as i64,
            size: i64::try_from(file_metadata.len()).map_err(|e| e.to_string())?,
            status: 1,
            cover: clip_cover_path,
            desc: String::new(),
            tags: String::new(),
            bvid: String::new(),
            area: parent_video.area,
            created_at: Local::now().to_rfc3339(),
        };

        let result = state.db.add_video(&clip_video).await?;
        state
            .db
            .new_message("视频切片完成", &format!("生成切片：{}", result.title))
            .await?;
        results.push(result);
    }

    Ok(results)
}

// 获取文件大小
#[cfg_attr(feature = "gui", tauri::command)]
pub async fn get_file_size(file_path: String) -> Result<u64, String> {
    let path = Path::new(&file_path);
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(e) => Err(format!("无法获取文件信息: {e}")),
    }
}

// 辅助函数：清理文件名
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .chars()
        .take(50) // 限制长度
        .collect()
}

/// 批量导入结果结构
#[derive(serde::Serialize, serde::Deserialize)]
pub struct BatchImportResult {
    pub successful_imports: i32,
    pub failed_imports: i32,
    pub imported_video_ids: Vec<i64>,
    pub errors: Vec<String>,
}

/// 批量导入外部视频文件
///
/// # 参数
/// - `state`: 应用状态
/// - `event_id`: 进度事件ID
/// - `file_paths`: 要导入的文件路径列表
/// - `room_id`: 房间ID
///
/// # 返回值
/// 返回批量导入结果，包含成功数量、失败数量、视频ID列表和错误信息
#[cfg_attr(feature = "gui", tauri::command)]
pub async fn batch_import_external_videos(
    state: state_type!(),
    event_id: String,
    file_paths: Vec<String>,
    room_id: String,
) -> Result<BatchImportResult, String> {
    if file_paths.is_empty() {
        return Ok(BatchImportResult {
            successful_imports: 0,
            failed_imports: 0,
            imported_video_ids: Vec::new(),
            errors: Vec::new(),
        });
    }

    let mut successful_imports = 0;
    let mut failed_imports = 0;
    let mut imported_video_ids = Vec::new();
    let mut errors = Vec::new();

    // 设置批量进度事件发射器
    #[cfg(feature = "gui")]
    let emitter = EventEmitter::new(state.app_handle.clone());
    #[cfg(feature = "headless")]
    let emitter = EventEmitter::new(state.progress_manager.get_event_sender());
    let batch_reporter = ProgressReporter::new(state.db.clone(), &emitter, &event_id).await?;

    let total_files = file_paths.len();

    for (index, file_path) in file_paths.iter().enumerate() {
        let current_index = index + 1;
        let file_name = Path::new(file_path)
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();

        // 更新批量进度，只显示进度信息
        batch_reporter
            .update(&format!(
                "正在导入第{current_index}个，共{total_files}个文件"
            ))
            .await;

        // 为每个文件创建独立的事件ID
        let file_event_id = format!("{event_id}_file_{index}");

        // 从文件名生成标题（去掉扩展名）
        let title = file_name.clone();

        // 调用现有的单文件导入函数
        match import_external_video(
            state.clone(),
            file_event_id,
            file_path.clone(),
            title,
            room_id.clone(),
        )
        .await
        {
            Ok(video) => {
                imported_video_ids.push(video.id);
                successful_imports += 1;
                log::info!("批量导入成功: {} (ID: {})", file_path, video.id);
            }
            Err(e) => {
                let error_msg = format!("导入失败 {file_path}: {e}");
                errors.push(error_msg.clone());
                failed_imports += 1;
                log::error!("批量导入失败: {error_msg}");
            }
        }
    }

    // 完成批量导入
    let result_msg = if failed_imports == 0 {
        format!("批量导入完成：成功导入{successful_imports}个文件")
    } else {
        format!("批量导入完成：成功{successful_imports}个，失败{failed_imports}个")
    };
    batch_reporter
        .finish(failed_imports == 0, &result_msg)
        .await;

    // 发送通知消息
    state
        .db
        .new_message("批量视频导入完成", &result_msg)
        .await?;

    Ok(BatchImportResult {
        successful_imports,
        failed_imports,
        imported_video_ids,
        errors,
    })
}

// 查询当前导入进度
#[cfg_attr(feature = "gui", tauri::command)]
pub async fn get_import_progress(
    state: state_type!(),
) -> Result<Option<serde_json::Value>, String> {
    // 查询进行中的FLV转换任务
    let all_tasks = state.db.get_tasks().await.map_err(|e| e.to_string())?;

    // 查找状态为 "pending" 或 "running" 的 import_flv_conversion 任务
    for task in &all_tasks {
        if task.task_type == "import_flv_conversion"
            && (task.status == "pending" || task.status == "running")
        {
            // 解析任务元数据
            let metadata: serde_json::Value =
                serde_json::from_str(&task.metadata).unwrap_or_default();

            return Ok(Some(serde_json::json!({
                "task_id": task.id,
                "file_name": metadata.get("file_name").and_then(|v| v.as_str()).unwrap_or("未知文件"),
                "file_size": metadata.get("file_size").and_then(serde_json::Value::as_u64).unwrap_or(0),
                "message": task.message,
                "status": task.status,
                "created_at": task.created_at
            })));
        }
    }

    Ok(None)
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn generate_audio_sample(state: state_type!(), video_id: i64) -> Result<(), String> {
    let video = state.db.get_video(video_id).await?;
    let video_path = Path::new(&state.config.read().await.output).join(&video.file);
    let opus_path = video_path.with_extension("opus");
    if !opus_path.exists() {
        let _ = crate::ffmpeg::extract_audio_sample(&video_path).await?;
    }
    Ok(())
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn generate_audio_waveform(
    state: state_type!(),
    video_id: i64,
) -> Result<ffmpeg::AudioWaveformData, String> {
    let video = state.db.get_video(video_id).await?;
    let video_path = Path::new(&state.config.read().await.output).join(&video.file);
    ffmpeg::generate_audio_waveform(&video_path).await
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn enqueue_seekbar_thumbnail_cache_task(
    state: state_type!(),
    video_id: i64,
) -> Result<Option<TaskRow>, String> {
    if !state.config.read().await.use_seekbar_thumbnail_cache {
        return Ok(None);
    }

    let video = state.db.get_video(video_id).await?;

    if let Some(existing_task) = state.db.get_tasks().await?.into_iter().find(|task| {
        let status = task.status.to_lowercase();
        if task.task_type != "generate_seekbar_thumbnail_cache"
            || (status != "pending" && status != "processing")
        {
            return false;
        }
        serde_json::from_str::<serde_json::Value>(&task.metadata)
            .ok()
            .and_then(|metadata| metadata.get("video_id").and_then(serde_json::Value::as_i64))
            == Some(video_id)
    }) {
        return Ok(Some(existing_task));
    }

    let task = state
        .db
        .generate_task(
            "generate_seekbar_thumbnail_cache",
            "",
            &json!({
                "video_id": video_id,
                "video_title": video.title,
            })
            .to_string(),
        )
        .await?;

    #[cfg(feature = "gui")]
    let emitter = EventEmitter::new(state.app_handle.clone());
    #[cfg(feature = "headless")]
    let emitter = EventEmitter::new(state.progress_manager.get_event_sender());
    let reporter = ProgressReporter::new(state.db.clone(), &emitter, &task.id).await?;

    #[cfg(feature = "gui")]
    let state_clone = (*state).clone();
    #[cfg(feature = "headless")]
    let state_clone = state.clone();

    let task_id = task.id.clone();
    state
        .task_manager
        .add_task(Task::new(task_id.clone(), TaskPriority::Low, async move {
            reporter.update("正在提取进度条预览图缓存...").await;

            let video = state_clone.db.get_video(video_id).await?;
            let video_path = Path::new(&state_clone.config.read().await.output).join(&video.file);
            match ffmpeg::generate_seekbar_thumbnail_cache(&video_path).await {
                Ok(manifest) => {
                    let message = "进度条预览图缓存提取完成";
                    reporter.finish(true, message).await;
                    let metadata = json!({
                        "video_id": video_id,
                        "video_title": video.title,
                        "frame_count": manifest.frame_count,
                        "step_seconds": manifest.step_seconds,
                    })
                    .to_string();
                    let _ = state_clone
                        .db
                        .update_task(&task_id, "success", message, Some(&metadata))
                        .await;
                    Ok(())
                }
                Err(error) => {
                    let message = format!("进度条预览图缓存提取失败: {error}");
                    reporter.finish(false, &message).await;
                    let _ = state_clone
                        .db
                        .update_task(&task_id, "failed", &message, None)
                        .await;
                    Err(message)
                }
            }
        }))
        .await?;

    Ok(Some(task))
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn has_seekbar_thumbnail_cache(
    state: state_type!(),
    video_id: i64,
) -> Result<bool, String> {
    let video = state.db.get_video(video_id).await?;
    let video_path = Path::new(&state.config.read().await.output).join(&video.file);
    Ok(ffmpeg::has_seekbar_thumbnail_cache(&video_path).await)
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn generate_seekbar_thumbnail_cache(
    state: state_type!(),
    video_id: i64,
) -> Result<(), String> {
    let video = state.db.get_video(video_id).await?;
    let video_path = Path::new(&state.config.read().await.output).join(&video.file);
    let _ = ffmpeg::generate_seekbar_thumbnail_cache(&video_path).await?;
    Ok(())
}

#[cfg_attr(feature = "gui", tauri::command)]
pub async fn generate_seekbar_thumbnail(
    state: state_type!(),
    video_id: i64,
    timestamp: f64,
) -> Result<String, String> {
    let video = state.db.get_video(video_id).await?;
    let video_path = Path::new(&state.config.read().await.output).join(&video.file);
    let safe_timestamp = if timestamp.is_finite() {
        timestamp.max(0.0)
    } else {
        0.0
    };
    let thumbnail_bytes = ffmpeg::read_seekbar_thumbnail_cache_bytes(&video_path, safe_timestamp)
        .await?
        .ok_or_else(|| "Seekbar thumbnail cache is not ready".to_string())?;
    Ok(format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(thumbnail_bytes)
    ))
}
