use std::path::PathBuf;

use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncBufReadExt, BufReader},
    sync::RwLock,
};

const NO_EMOTE_PREFIX: &str = "__BSR_NO_EMOTE__";

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DanmuEntry {
    pub ts: i64,
    pub content: String,
    pub render_emotes: bool,
}

pub fn decode_danmu_content(content: &str) -> (String, bool) {
    if let Some(stripped) = content.strip_prefix(NO_EMOTE_PREFIX) {
        (stripped.to_string(), false)
    } else {
        (content.to_string(), true)
    }
}

pub fn encode_danmu_content(content: &str, render_emotes: bool) -> String {
    if render_emotes {
        content.to_string()
    } else {
        format!("{NO_EMOTE_PREFIX}{content}")
    }
}

pub struct DanmuStorage {
    cache: RwLock<Vec<DanmuEntry>>,
    file: RwLock<File>,
}

impl DanmuStorage {
    pub async fn new(file_path: &PathBuf) -> Option<DanmuStorage> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(file_path)
            .await;
        if file.is_err() {
            log::error!("Open danmu file failed: {}", file.err().unwrap());
            return None;
        }
        let file = file.unwrap();
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut preload_cache: Vec<DanmuEntry> = Vec::new();
        while let Ok(Some(line)) = lines.next_line().await {
            let Some((ts_str, content)) = line.split_once(':') else {
                log::warn!("Skip malformed danmu line: {line}");
                continue;
            };
            let Ok(ts) = ts_str.parse::<i64>() else {
                log::warn!("Skip malformed danmu timestamp: {line}");
                continue;
            };
            let (content, render_emotes) = decode_danmu_content(content);
            preload_cache.push(DanmuEntry {
                ts,
                content,
                render_emotes,
            });
        }
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(file_path)
            .await
            .expect("create danmu.txt failed");
        Some(DanmuStorage {
            cache: RwLock::new(preload_cache),
            file: RwLock::new(file),
        })
    }

    pub async fn add_line(&self, ts: i64, content: &str) {
        self.cache.write().await.push(DanmuEntry {
            ts,
            content: content.to_string(),
            render_emotes: true,
        });
        let _ = self
            .file
            .write()
            .await
            .write(format!("{ts}:{}\n", encode_danmu_content(content, true)).as_bytes())
            .await;
    }

    // get entries with ts relative to live start time
    pub async fn get_entries(&self, live_start_ts: i64) -> Vec<DanmuEntry> {
        let mut danmus: Vec<DanmuEntry> = self
            .cache
            .read()
            .await
            .iter()
            .map(|entry| DanmuEntry {
                ts: entry.ts - live_start_ts,
                content: entry.content.clone(),
                render_emotes: entry.render_emotes,
            })
            .collect();
        // filter out danmus with ts < 0
        danmus.retain(|entry| entry.ts >= 0);
        danmus
    }
}
