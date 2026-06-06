use prost::Message;
use reqwest::{
    header::{HeaderMap, HeaderValue, COOKIE, REFERER, USER_AGENT},
    Client,
};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};

const VIEW_API_URL: &str = "https://api.bilibili.com/x/web-interface/view";
const SEG_API_URL: &str = "https://api.bilibili.com/x/v2/dm/web/seg.so";
const PBP_API_URL: &str = "https://api.bilibili.com/pbp/data";
const SEGMENT_DURATION_SECONDS: i64 = 360;
const MAX_RETRIES: usize = 4;
const BASE_BACKOFF_MILLIS: u64 = 1_200;

#[derive(Debug, Clone)]
pub struct ImportedDanmuEntry {
    pub ts: i64,
    pub content: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedVideoDanmuDownload {
    pub bvid: String,
    pub aid: i64,
    pub cid: i64,
    pub page: i32,
    pub saved_count: usize,
    pub total_segments: usize,
    pub skipped_segments: usize,
}

#[derive(Debug, Clone)]
pub struct ImportedVideoDanmuData {
    pub download: ImportedVideoDanmuDownload,
    pub entries: Vec<ImportedDanmuEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedVideoPbpData {
    pub bvid: String,
    pub aid: i64,
    pub cid: i64,
    pub page: i32,
    pub step_sec: f64,
    pub values: Vec<f64>,
}

#[derive(Debug, Deserialize)]
struct BilibiliApiResponse<T> {
    code: i64,
    #[serde(default)]
    message: String,
    data: Option<T>,
}

#[derive(Debug, Deserialize)]
struct BilibiliViewData {
    aid: i64,
    #[serde(default)]
    pages: Vec<BilibiliVideoPage>,
}

#[derive(Debug, Deserialize)]
struct BilibiliVideoPage {
    cid: i64,
    #[serde(default = "default_page_number")]
    page: i32,
    #[serde(default)]
    duration: i64,
}

#[derive(Debug, Deserialize)]
struct BilibiliPbpResponse {
    step_sec: f64,
    events: Option<BilibiliPbpEvents>,
}

#[derive(Debug, Deserialize)]
struct BilibiliPbpEvents {
    #[serde(default)]
    default: Vec<f64>,
}

#[derive(Debug)]
enum FetchError {
    Fatal(String),
    RateLimited(String),
}

enum SegmentFetchOutcome {
    Closed,
    Entries(Vec<ImportedDanmuEntry>),
}

#[derive(Clone, PartialEq, Message)]
struct DmSegMobileReply {
    #[prost(message, repeated, tag = "1")]
    elems: Vec<DanmakuElem>,
}

#[derive(Clone, PartialEq, Message)]
struct DanmakuElem {
    #[prost(int32, tag = "2")]
    progress: i32,
    #[prost(int32, tag = "3")]
    mode: i32,
    #[prost(string, tag = "7")]
    content: String,
}

fn default_page_number() -> i32 {
    1
}

fn backoff_duration(attempt: usize) -> Duration {
    let multiplier = 1u64 << attempt.min(6);
    Duration::from_millis(BASE_BACKOFF_MILLIS.saturating_mul(multiplier))
}

fn is_rate_limited(code: i64, message: &str) -> bool {
    code == -352 || message.contains("-352") || message.contains("频繁")
}

fn normalize_bvid(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("BV 号不能为空".to_string());
    }

    let normalized = if trimmed.len() >= 2 && trimmed[..2].eq_ignore_ascii_case("bv") {
        format!("BV{}", &trimmed[2..])
    } else {
        trimmed.to_string()
    };

    if !normalized.starts_with("BV") || normalized.len() < 12 {
        return Err("请输入有效的 BV 号".to_string());
    }

    Ok(normalized)
}

fn default_headers(bvid: &str, cookies: Option<&str>) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0"));
    headers.insert(
        REFERER,
        HeaderValue::from_str(&format!("https://www.bilibili.com/video/{bvid}"))
            .map_err(|e| format!("构建 Referer 失败: {e}"))?,
    );
    if let Some(cookies) = cookies.filter(|cookies| !cookies.trim().is_empty()) {
        headers.insert(
            COOKIE,
            HeaderValue::from_str(cookies).map_err(|e| format!("构建 Cookie 失败: {e}"))?,
        );
    }
    Ok(headers)
}

fn decode_json_body<T: for<'de> Deserialize<'de>>(body: &[u8]) -> Result<T, String> {
    let text = std::str::from_utf8(body).map_err(|e| format!("响应不是有效 UTF-8: {e}"))?;
    serde_json::from_str(text).map_err(|e| format!("解析 B 站接口响应失败: {e}"))
}

fn select_page<'a>(
    pages: &'a [BilibiliVideoPage],
    page_number: i32,
) -> Result<&'a BilibiliVideoPage, String> {
    if pages.is_empty() {
        return Err("该 BV 没有可用分 P".to_string());
    }

    pages
        .iter()
        .find(|page| page.page == page_number)
        .ok_or_else(|| format!("BV 中不存在 P{page_number}"))
}

async fn fetch_view_data(
    client: &Client,
    bvid: &str,
    cookies: Option<&str>,
) -> Result<BilibiliViewData, FetchError> {
    let headers = default_headers(bvid, cookies).map_err(FetchError::Fatal)?;

    for attempt in 0..MAX_RETRIES {
        let response = match client
            .get(VIEW_API_URL)
            .headers(headers.clone())
            .query(&[("bvid", bvid)])
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => {
                if attempt + 1 < MAX_RETRIES {
                    sleep(backoff_duration(attempt)).await;
                    continue;
                }
                return Err(FetchError::Fatal(format!("获取视频信息失败: {err}")));
            }
        };

        let status = response.status();
        let body = match response.bytes().await {
            Ok(body) => body,
            Err(err) => {
                if attempt + 1 < MAX_RETRIES {
                    sleep(backoff_duration(attempt)).await;
                    continue;
                }
                return Err(FetchError::Fatal(format!("读取视频信息失败: {err}")));
            }
        };

        if !status.is_success() {
            if attempt + 1 < MAX_RETRIES {
                sleep(backoff_duration(attempt)).await;
                continue;
            }
            return Err(FetchError::Fatal(format!(
                "获取视频信息失败，HTTP {}",
                status
            )));
        }

        let payload: BilibiliApiResponse<BilibiliViewData> =
            decode_json_body(&body).map_err(FetchError::Fatal)?;
        if payload.code == 0 {
            return payload
                .data
                .ok_or_else(|| FetchError::Fatal("视频信息响应缺少 data 字段".to_string()));
        }

        if is_rate_limited(payload.code, &payload.message) {
            if attempt + 1 < MAX_RETRIES {
                sleep(backoff_duration(attempt)).await;
                continue;
            }
            return Err(FetchError::RateLimited(format!(
                "获取视频信息被限流: {} ({})",
                payload.message, payload.code
            )));
        }

        return Err(FetchError::Fatal(format!(
            "获取视频信息失败: {} ({})",
            payload.message, payload.code
        )));
    }

    Err(FetchError::Fatal("获取视频信息失败".to_string()))
}

async fn fetch_segment(
    client: &Client,
    bvid: &str,
    cookies: Option<&str>,
    cid: i64,
    segment_index: usize,
) -> Result<SegmentFetchOutcome, FetchError> {
    let headers = default_headers(bvid, cookies).map_err(FetchError::Fatal)?;

    for attempt in 0..MAX_RETRIES {
        let response = match client
            .get(SEG_API_URL)
            .headers(headers.clone())
            .query(&[
                ("type", "1".to_string()),
                ("oid", cid.to_string()),
                ("segment_index", segment_index.to_string()),
            ])
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => {
                log::warn!(
                    "下载弹幕分段 {segment_index} 失败: {err}, attempt {}/{}",
                    attempt + 1,
                    MAX_RETRIES
                );
                if attempt + 1 < MAX_RETRIES {
                    sleep(backoff_duration(attempt)).await;
                    continue;
                }
                return Err(FetchError::Fatal(format!(
                    "下载弹幕分段 {segment_index} 失败: {err}"
                )));
            }
        };

        let status = response.status();
        let body = match response.bytes().await {
            Ok(body) => body,
            Err(err) => {
                log::warn!(
                    "读取弹幕分段 {segment_index} 失败: {err}, attempt {}/{}",
                    attempt + 1,
                    MAX_RETRIES
                );
                if attempt + 1 < MAX_RETRIES {
                    sleep(backoff_duration(attempt)).await;
                    continue;
                }
                return Err(FetchError::Fatal(format!(
                    "读取弹幕分段 {segment_index} 失败: {err}"
                )));
            }
        };

        if !status.is_success() {
            if attempt + 1 < MAX_RETRIES {
                sleep(backoff_duration(attempt)).await;
                continue;
            }
            return Err(FetchError::Fatal(format!(
                "下载弹幕分段 {segment_index} 失败，HTTP {}",
                status
            )));
        }

        if body.as_ref() == b"\x10\x01" {
            return Ok(SegmentFetchOutcome::Closed);
        }

        if body.first() == Some(&b'{') {
            let payload: BilibiliApiResponse<serde_json::Value> =
                decode_json_body(&body).map_err(FetchError::Fatal)?;
            if is_rate_limited(payload.code, &payload.message) {
                if attempt + 1 < MAX_RETRIES {
                    sleep(backoff_duration(attempt)).await;
                    continue;
                }
                return Err(FetchError::RateLimited(format!(
                    "弹幕分段 {segment_index} 被限流: {} ({})",
                    payload.message, payload.code
                )));
            }

            return Err(FetchError::Fatal(format!(
                "弹幕分段 {segment_index} 返回错误: {} ({})",
                payload.message, payload.code
            )));
        }

        let reply = DmSegMobileReply::decode(body.as_ref())
            .map_err(|e| FetchError::Fatal(format!("解析弹幕分段 {segment_index} 失败: {e}")))?;

        let total_before = reply.elems.len();

        let entries: Vec<_> = reply
            .elems
            .into_iter()
            .filter(|elem| (1..=7).contains(&elem.mode))
            .filter(|elem| elem.progress >= 0)
            .filter_map(|elem| {
                let content = elem.content.trim().replace('\r', " ").replace('\n', " ");
                if content.is_empty() {
                    return None;
                }

                Some(ImportedDanmuEntry {
                    ts: i64::from(elem.progress),
                    content,
                })
            })
            .collect();

        let total_after = entries.len();

        log::info!(
            "下载弹幕分段 {segment_index} 成功，原始弹幕数: {total_before}, 过滤后有效弹幕数: {total_after}"
        );

        return Ok(SegmentFetchOutcome::Entries(entries));
    }

    Err(FetchError::Fatal(format!(
        "下载弹幕分段 {segment_index} 失败"
    )))
}

pub async fn download_video_pbp(
    client: &Client,
    bvid: &str,
    aid: i64,
    cid: i64,
    page: i32,
    cookies: Option<&str>,
) -> Result<Option<ImportedVideoPbpData>, String> {
    if cid <= 0 {
        return Ok(None);
    }

    let headers = default_headers(bvid, cookies)?;
    let response = client
        .get(PBP_API_URL)
        .headers(headers)
        .query(&[
            ("cid", cid.to_string()),
            ("aid", aid.to_string()),
            ("bvid", bvid.to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("获取高能进度条失败: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        return Ok(None);
    }

    let body = response
        .bytes()
        .await
        .map_err(|e| format!("读取高能进度条失败: {e}"))?;
    let payload: BilibiliPbpResponse = decode_json_body(&body)?;
    if !payload.step_sec.is_finite() || payload.step_sec <= 0.0 {
        return Ok(None);
    }

    let values = payload
        .events
        .map(|events| events.default)
        .unwrap_or_default()
        .into_iter()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .collect::<Vec<_>>();

    if values.is_empty() || !values.iter().any(|value| *value > 0.0) {
        return Ok(None);
    }

    Ok(Some(ImportedVideoPbpData {
        bvid: bvid.to_string(),
        aid,
        cid,
        page,
        step_sec: payload.step_sec,
        values,
    }))
}

pub async fn download_video_danmaku(
    client: &Client,
    raw_bvid: &str,
    page_number: i32,
    cookies: Option<&str>,
) -> Result<ImportedVideoDanmuData, String> {
    let bvid = normalize_bvid(raw_bvid)?;
    let view_data = match fetch_view_data(client, &bvid, cookies).await {
        Ok(data) => data,
        Err(FetchError::Fatal(err) | FetchError::RateLimited(err)) => return Err(err),
    };

    let selected_page = select_page(&view_data.pages, page_number)?;
    let total_segments = ((selected_page.duration.max(0) + SEGMENT_DURATION_SECONDS - 1)
        / SEGMENT_DURATION_SECONDS)
        .max(1) as usize;
    log::info!(
        "Downloading danmaku for {} page {}, total segments: {}",
        bvid,
        selected_page.page,
        total_segments
    );
    let mut entries = Vec::new();
    let mut skipped_segments = 0usize;

    for segment_index in 1..=total_segments {
        match fetch_segment(client, &bvid, cookies, selected_page.cid, segment_index).await {
            Ok(SegmentFetchOutcome::Closed) => {
                log::info!(
                    "Danmaku is closed for {} page {}, stop at segment {}",
                    bvid,
                    selected_page.page,
                    segment_index
                );
                break;
            }
            Ok(SegmentFetchOutcome::Entries(mut segment_entries)) => {
                entries.append(&mut segment_entries);
            }
            Err(FetchError::RateLimited(err)) => {
                skipped_segments += 1;
                log::warn!("{err}");
            }
            Err(FetchError::Fatal(err)) => return Err(err),
        }
    }

    entries.sort_by(|left, right| {
        left.ts
            .cmp(&right.ts)
            .then(left.content.cmp(&right.content))
    });
    entries.dedup_by(|left, right| left.ts == right.ts && left.content == right.content);

    Ok(ImportedVideoDanmuData {
        download: ImportedVideoDanmuDownload {
            bvid,
            aid: view_data.aid,
            cid: selected_page.cid,
            page: selected_page.page,
            saved_count: entries.len(),
            total_segments,
            skipped_segments,
        },
        entries,
    })
}
