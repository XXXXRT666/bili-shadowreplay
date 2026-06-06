use recorder::danmu::DanmuEntry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

const PLAY_RES_X: f64 = 1280.0;
const PLAY_RES_Y: f64 = 720.0;
const DANMAKU_BASE_FONT_SIZE: f64 = 25.0;
const DANMAKU_LINE_HEIGHT: f64 = 1.125;
const DANMAKU_EMOTE_SCALE: f64 = 1.0;
const DANMAKU_EMOTE_VERTICAL_OFFSET_EM: f64 = 0.0;
const DANMAKU_EMOTE_TEXT_GAP_EM: f64 = 0.1;
const SCROLL_SPEED_REFERENCE_WIDTH_PX: f64 = 512.0;
const SCROLL_BASE_DURATION_MS: f64 = 4500.0;
const DANMU_TRACK_GAP_PX: f64 = 1.0;
const DANMU_MAX_DELAY_MS: f64 = 500.0;
const DANMU_DELAY_STEP_MS: f64 = 100.0;
const PREVENT_SUBTITLE_VISIBLE_RATIO: f64 = 0.85;
const SPEED_PRESET_RATES: [f64; 5] = [0.6, 0.8, 1.0, 1.2, 1.4];

#[derive(Deserialize, Serialize, Clone)]
pub struct Danmu2AssOptions {
    pub font_size: f64,
    pub opacity: f64, // 透明度，范围 0.0-1.0，0.0为完全透明，1.0为完全不透明
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct DanmuRenderOptions {
    pub font_scale: f64,
    pub opacity: f64,
    pub display_area: f64,
    pub speed_preset: i32,
    pub max_on_screen: i32,
    pub bold: bool,
    pub font_family: String,
    pub prevent_subtitle_occlusion: bool,
}

#[derive(Debug, Clone)]
pub struct DanmuImageOverlay {
    pub image_path: PathBuf,
    pub start: f64,
    pub end: f64,
    pub x_start: f64,
    pub x_end: f64,
    pub y: f64,
    pub size: f64,
}

#[derive(Debug, Clone)]
pub struct DanmuAssRender {
    pub ass_content: String,
    pub image_overlays: Vec<DanmuImageOverlay>,
}

#[derive(Debug, Clone)]
enum DanmuRenderSegment {
    Text(String),
    Emote { image_path: PathBuf },
}

impl Default for Danmu2AssOptions {
    fn default() -> Self {
        Self {
            font_size: 36.0,
            opacity: 0.8, // 默认80%透明度
        }
    }
}

impl Default for DanmuRenderOptions {
    fn default() -> Self {
        Self {
            font_scale: 1.0,
            opacity: 1.0,
            display_area: 100.0,
            speed_preset: 2,
            max_on_screen: -1,
            bold: true,
            font_family: r#"SimHei, "Microsoft JhengHei", Arial, Helvetica, sans-serif"#
                .to_string(),
            prevent_subtitle_occlusion: true,
        }
    }
}

impl From<Danmu2AssOptions> for DanmuRenderOptions {
    fn from(options: Danmu2AssOptions) -> Self {
        Self {
            font_scale: (options.font_size / DANMAKU_BASE_FONT_SIZE).max(0.1),
            opacity: options.opacity,
            bold: false,
            ..Self::default()
        }
    }
}

pub fn danmu_to_ass(danmus: Vec<DanmuEntry>, options: Danmu2AssOptions) -> String {
    danmu_to_ass_with_emotes(danmus, options.into(), &HashMap::new()).ass_content
}

pub fn danmu_to_ass_with_emotes(
    danmus: Vec<DanmuEntry>,
    options: DanmuRenderOptions,
    emote_files: &HashMap<String, PathBuf>,
) -> DanmuAssRender {
    let layout = DanmuLayout::from_options(&options);
    let font_size = layout.font_size_px;
    let opacity = options.opacity.clamp(0.0, 1.0);

    // 将透明度转换为十六进制Alpha值 (0.0-1.0 -> 0x00-0xFF)
    let alpha = ((1.0 - opacity) * 255.0) as u8;
    let alpha_hex = format!("{:02X}", alpha);
    let font_name = ass_font_name(&options.font_family);
    let bold = if options.bold { -1 } else { 0 };

    // ASS header
    let header = format!(
        r"[Script Info]
Title: Bilibili Danmaku
ScriptType: v4.00+
Collisions: Normal
PlayResX: 1280
PlayResY: 720
Timer: 10.0000

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,{},{},&H{}FFFFFF,&H{}FFFFFF,&H{}000000,&H{}000000,{},0,0,0,100,100,0,0,1,0.5,0,2,20,20,2,0

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
",
        font_name, font_size, alpha_hex, alpha_hex, alpha_hex, alpha_hex, bold
    );

    let mut active_danmus = Vec::new();
    let mut image_overlays = Vec::new();
    let mut sorted_danmus = danmus;
    sorted_danmus.sort_by_key(|entry| entry.ts);

    // Convert danmus to ASS events
    let events = sorted_danmus
        .iter()
        .filter_map(|danmu| {
            active_danmus.retain(|active: &ActiveDanmuLayout| active.end_ms > danmu.ts as f64);
            if layout.max_on_screen > 0 && active_danmus.len() >= layout.max_on_screen as usize {
                return None;
            }

            let segments = parse_danmu_render_segments(&danmu.content, emote_files);
            let measurement = measure_danmu_render_segments(&segments, &layout);
            let placement = find_placement(&active_danmus, &layout, &measurement, danmu.ts as f64)?;

            active_danmus.push(ActiveDanmuLayout {
                top_px: placement.top_px,
                bottom_px: placement.top_px + measurement.height_px,
                width_px: measurement.width_px,
                scheduled_start_ms: placement.scheduled_start_ms,
                end_ms: placement.end_ms,
                speed_px_per_ms: placement.speed_px_per_ms,
            });

            if placement.end_ms <= 0.0 {
                return None;
            }

            let event_start_ms = placement.scheduled_start_ms.max(0.0);
            let event_end_ms = placement.end_ms;
            let elapsed_at_event_start_ms = event_start_ms - placement.scheduled_start_ms;
            let group_start_x =
                danmu_distance_px() - elapsed_at_event_start_ms * placement.speed_px_per_ms;
            let group_end_x = -measurement.width_px;

            // Convert timestamp to ASS time format (H:MM:SS.CC)
            let start_time = format_time(event_start_ms / 1000.0);
            let end_time = format_time(event_end_ms / 1000.0);

            let mut offset_x = 0.0;
            let mut lines = Vec::new();
            for (segment_index, segment) in segments.iter().enumerate() {
                match segment {
                    DanmuRenderSegment::Text(text) => {
                        if text.is_empty() {
                            continue;
                        }
                        let width = measure_text_width(&text, font_size);
                        if width <= 0.0 {
                            continue;
                        }
                        let text_anchor = text_anchor_for_segment(&segments, segment_index);
                        let (x_start, x_end, alignment) = match text_anchor {
                            TextAnchor::Left => (
                                group_start_x + offset_x,
                                group_end_x + offset_x,
                                1,
                            ),
                            TextAnchor::Center => (
                                group_start_x + offset_x + width / 2.0,
                                group_end_x + offset_x + width / 2.0,
                                2,
                            ),
                            TextAnchor::Right => (
                                group_start_x + offset_x + width,
                                group_end_x + offset_x + width,
                                3,
                            ),
                        };
                        lines.push(format!(
                            "Dialogue: 0,{},{},Default,,0,0,0,,{{\\an{}\\move({:.2},{:.2},{:.2},{:.2})}}{}",
                            start_time,
                            end_time,
                            alignment,
                            x_start,
                            placement.top_px + font_size,
                            x_end,
                            placement.top_px + font_size,
                            escape_text(&text)
                        ));
                        offset_x += width;
                    }
                    DanmuRenderSegment::Emote { image_path } => {
                        let emote_size = font_size * DANMAKU_EMOTE_SCALE;
                        let margin_left = emote_margin_left_px(&segments, segment_index, font_size);
                        let margin_right = emote_margin_right_px(&segments, segment_index, font_size);
                        let emote_x = offset_x + margin_left;
                        let emote_y = placement.top_px
                            + ((font_size * layout.line_height) - emote_size) / 2.0
                            + font_size * DANMAKU_EMOTE_VERTICAL_OFFSET_EM;
                        image_overlays.push(DanmuImageOverlay {
                            image_path: image_path.clone(),
                            start: event_start_ms / 1000.0,
                            end: event_end_ms / 1000.0,
                            x_start: group_start_x + emote_x,
                            x_end: group_end_x + emote_x,
                            y: emote_y,
                            size: emote_size,
                        });
                        offset_x += margin_left + emote_size + margin_right;
                    }
                }
            }

            if lines.is_empty() {
                None
            } else {
                Some(lines.join("\n"))
            }
        })
        .collect::<Vec<String>>()
        .join("\n");

    // Combine header and events
    DanmuAssRender {
        ass_content: format!("{header}\n{events}"),
        image_overlays,
    }
}

pub fn danmu_max_active_duration_ms(options: &DanmuRenderOptions) -> i64 {
    let layout = DanmuLayout::from_options(options);
    let min_speed_px_per_ms = ((SCROLL_SPEED_REFERENCE_WIDTH_PX + layout.font_size_px)
        / SCROLL_BASE_DURATION_MS)
        * speed_preset_rate(layout.speed_preset);
    ((PLAY_RES_X + PLAY_RES_X * 1.5) / min_speed_px_per_ms.max(0.001) + DANMU_MAX_DELAY_MS).ceil()
        as i64
}

fn parse_danmu_render_segments(
    content: &str,
    emote_files: &HashMap<String, PathBuf>,
) -> Vec<DanmuRenderSegment> {
    if content.is_empty() {
        return Vec::new();
    }

    let mut segments = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = content[cursor..].find('[') {
        let start = cursor + relative_start;
        if start > cursor {
            segments.push(DanmuRenderSegment::Text(content[cursor..start].to_string()));
        }

        let Some(relative_end) = content[start..].find(']') else {
            segments.push(DanmuRenderSegment::Text(content[start..].to_string()));
            return segments;
        };

        let end = start + relative_end + 1;
        let token = &content[start..end];
        if let Some(image_path) = emote_files.get(token) {
            segments.push(DanmuRenderSegment::Emote {
                image_path: image_path.clone(),
            });
        } else {
            segments.push(DanmuRenderSegment::Text(token.to_string()));
        }
        cursor = end;
    }

    if cursor < content.len() {
        segments.push(DanmuRenderSegment::Text(content[cursor..].to_string()));
    }
    segments
}

#[derive(Debug, Clone)]
struct DanmuLayout {
    font_size_px: f64,
    line_height: f64,
    display_area: f64,
    speed_preset: i32,
    max_on_screen: i32,
    prevent_subtitle_occlusion: bool,
}

impl DanmuLayout {
    fn from_options(options: &DanmuRenderOptions) -> Self {
        Self {
            font_size_px: (DANMAKU_BASE_FONT_SIZE * options.font_scale).max(1.0),
            line_height: DANMAKU_LINE_HEIGHT,
            display_area: options.display_area,
            speed_preset: options.speed_preset,
            max_on_screen: options.max_on_screen,
            prevent_subtitle_occlusion: options.prevent_subtitle_occlusion,
        }
    }
}

#[derive(Debug, Clone)]
struct DanmuMeasurement {
    width_px: f64,
    height_px: f64,
}

#[derive(Debug, Clone)]
struct DanmuPlacement {
    top_px: f64,
    scheduled_start_ms: f64,
    end_ms: f64,
    middle_ms: f64,
    speed_px_per_ms: f64,
}

#[derive(Debug, Clone)]
struct ActiveDanmuLayout {
    top_px: f64,
    bottom_px: f64,
    width_px: f64,
    scheduled_start_ms: f64,
    end_ms: f64,
    speed_px_per_ms: f64,
}

fn measure_danmu_render_segments(
    segments: &[DanmuRenderSegment],
    layout: &DanmuLayout,
) -> DanmuMeasurement {
    let font_size = layout.font_size_px;
    let mut width_px = 0.0_f64;
    let mut line_width_px = 0.0_f64;
    let mut line_count = 1_usize;
    for (segment_index, segment) in segments.iter().enumerate() {
        if matches!(segment, DanmuRenderSegment::Emote { image_path: _ }) {
            line_width_px += emote_margin_left_px(segments, segment_index, font_size)
                + font_size * DANMAKU_EMOTE_SCALE
                + emote_margin_right_px(segments, segment_index, font_size);
            continue;
        }

        let DanmuRenderSegment::Text(text) = segment else {
            continue;
        };
        for (index, line) in text.split('\n').enumerate() {
            if index > 0 {
                width_px = width_px.max(line_width_px);
                line_width_px = 0.0;
                line_count += 1;
            }
            line_width_px += measure_text_width(line, font_size);
        }
    }

    width_px = width_px.max(line_width_px).max(font_size);

    DanmuMeasurement {
        width_px,
        height_px: (font_size * layout.line_height * line_count as f64).max(font_size),
    }
}

fn measure_text_width(text: &str, font_size: f64) -> f64 {
    text.chars()
        .filter(|ch| *ch != '\n' && *ch != '\r')
        .map(|ch| {
            if ch.is_whitespace() {
                font_size * 0.33
            } else if is_cjk_or_fullwidth(ch) {
                font_size
            } else if ch.is_ascii_uppercase() || ch.is_ascii_digit() {
                font_size * 0.62
            } else {
                font_size * 0.55
            }
        })
        .sum()
}

fn is_cjk_or_fullwidth(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x11ff | 0x2e80..=0x9fff | 0xf900..=0xfaff | 0xff00..=0xffef
    )
}

fn format_time(seconds: f64) -> String {
    let hours = (seconds / 3600.0) as i32;
    let minutes = ((seconds % 3600.0) / 60.0) as i32;
    let seconds = seconds % 60.0;
    format!("{hours}:{minutes:02}:{seconds:05.2}")
}

fn escape_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('{', "｛")
        .replace('}', "｝")
        .replace('\r', "")
        .replace('\n', "\\N")
}

fn ass_font_name(font_family: &str) -> String {
    font_family
        .split(',')
        .map(|part| part.trim().trim_matches('"').trim_matches('\'').trim())
        .find(|part| !part.is_empty())
        .unwrap_or("SimHei")
        .to_string()
}

fn has_visible_text(segment: Option<&DanmuRenderSegment>) -> bool {
    matches!(segment, Some(DanmuRenderSegment::Text(text)) if !text.trim().is_empty())
}

enum TextAnchor {
    Left,
    Center,
    Right,
}

fn is_emote_segment(segment: Option<&DanmuRenderSegment>) -> bool {
    matches!(segment, Some(DanmuRenderSegment::Emote { image_path: _ }))
}

fn text_anchor_for_segment(segments: &[DanmuRenderSegment], index: usize) -> TextAnchor {
    if is_emote_segment(segments.get(index + 1)) {
        return TextAnchor::Right;
    }
    if is_emote_segment(
        index
            .checked_sub(1)
            .and_then(|previous| segments.get(previous)),
    ) {
        return TextAnchor::Left;
    }
    TextAnchor::Center
}

fn emote_margin_left_px(segments: &[DanmuRenderSegment], index: usize, font_size: f64) -> f64 {
    if has_visible_text(
        index
            .checked_sub(1)
            .and_then(|previous| segments.get(previous)),
    ) {
        font_size * DANMAKU_EMOTE_TEXT_GAP_EM
    } else {
        0.0
    }
}

fn emote_margin_right_px(segments: &[DanmuRenderSegment], index: usize, font_size: f64) -> f64 {
    if has_visible_text(segments.get(index + 1)) {
        font_size * DANMAKU_EMOTE_TEXT_GAP_EM
    } else {
        0.0
    }
}

fn speed_preset_rate(speed_preset: i32) -> f64 {
    let index = speed_preset.clamp(0, SPEED_PRESET_RATES.len() as i32 - 1) as usize;
    SPEED_PRESET_RATES[index]
}

fn available_height_px(layout: &DanmuLayout) -> f64 {
    let display_area_ratio = if layout.display_area.is_finite() && layout.display_area > 0.0 {
        layout.display_area.min(100.0) / 100.0
    } else {
        1.0
    };
    let subtitle_ratio = if layout.prevent_subtitle_occlusion {
        PREVENT_SUBTITLE_VISIBLE_RATIO
    } else {
        1.0
    };

    (PLAY_RES_Y * display_area_ratio.min(subtitle_ratio)).max(0.0)
}

fn danmu_distance_px() -> f64 {
    PLAY_RES_X + 1.0
}

fn create_placement_base(
    layout: &DanmuLayout,
    measurement: &DanmuMeasurement,
    scheduled_start_ms: f64,
) -> DanmuPlacement {
    let distance_px = danmu_distance_px();
    let speed_px_per_ms = ((SCROLL_SPEED_REFERENCE_WIDTH_PX + measurement.width_px)
        / SCROLL_BASE_DURATION_MS)
        * speed_preset_rate(layout.speed_preset);
    let duration_ms = (distance_px + measurement.width_px) / speed_px_per_ms.max(0.001);
    let end_ms = scheduled_start_ms + duration_ms;
    let middle_ms = scheduled_start_ms + distance_px / speed_px_per_ms.max(0.001);

    DanmuPlacement {
        top_px: 0.0,
        scheduled_start_ms,
        end_ms,
        middle_ms,
        speed_px_per_ms,
    }
}

fn danmu_right_at(danmu: &ActiveDanmuLayout, time_ms: f64) -> f64 {
    let elapsed_ms = (time_ms - danmu.scheduled_start_ms).max(0.0);
    let x = danmu_distance_px() - elapsed_ms * danmu.speed_px_per_ms;
    x + danmu.width_px
}

fn overlaps_vertically(top_px: f64, bottom_px: f64, danmu: &ActiveDanmuLayout) -> bool {
    top_px <= danmu.bottom_px && bottom_px >= danmu.top_px
}

fn can_place_at(
    active_danmus: &[ActiveDanmuLayout],
    top_px: f64,
    measurement: &DanmuMeasurement,
    placement: &DanmuPlacement,
) -> bool {
    let bottom_px = top_px + measurement.height_px;
    let distance_px = danmu_distance_px();

    active_danmus.iter().all(|danmu| {
        if !overlaps_vertically(top_px, bottom_px, danmu) {
            return true;
        }

        if danmu.end_ms < placement.middle_ms {
            return danmu_right_at(danmu, placement.scheduled_start_ms) < distance_px;
        }

        false
    })
}

fn find_placement_at_start(
    active_danmus: &[ActiveDanmuLayout],
    layout: &DanmuLayout,
    measurement: &DanmuMeasurement,
    scheduled_start_ms: f64,
) -> Option<DanmuPlacement> {
    let available_height_px = available_height_px(layout);
    if measurement.height_px >= available_height_px {
        return None;
    }

    let placement_base = create_placement_base(layout, measurement, scheduled_start_ms);
    let max_top_px = available_height_px - measurement.height_px;
    let mut sorted_active_danmus = active_danmus.to_vec();
    sorted_active_danmus.sort_by(|a, b| {
        a.top_px
            .total_cmp(&b.top_px)
            .then_with(|| a.bottom_px.total_cmp(&b.bottom_px))
    });

    let mut candidate_top_positions = vec![0.0];
    for danmu in &sorted_active_danmus {
        let next_top = danmu.bottom_px + DANMU_TRACK_GAP_PX;
        if next_top <= max_top_px {
            candidate_top_positions.push(next_top);
        }
    }

    for top_px in candidate_top_positions {
        if can_place_at(active_danmus, top_px, measurement, &placement_base) {
            return Some(DanmuPlacement {
                top_px,
                ..placement_base
            });
        }
    }

    None
}

fn find_placement(
    active_danmus: &[ActiveDanmuLayout],
    layout: &DanmuLayout,
    measurement: &DanmuMeasurement,
    start_ms: f64,
) -> Option<DanmuPlacement> {
    let mut delay_ms = 0.0;
    while delay_ms <= DANMU_MAX_DELAY_MS {
        if let Some(placement) =
            find_placement_at_start(active_danmus, layout, measurement, start_ms + delay_ms)
        {
            return Some(placement);
        }
        delay_ms += DANMU_DELAY_STEP_MS;
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_before_emote_uses_right_anchor() {
        let segments = vec![
            DanmuRenderSegment::Text("远远大于楚子航啊".to_string()),
            DanmuRenderSegment::Emote {
                image_path: PathBuf::from("汤圆.png"),
            },
        ];

        assert!(matches!(
            text_anchor_for_segment(&segments, 0),
            TextAnchor::Right
        ));
    }

    #[test]
    fn text_after_emote_uses_left_anchor() {
        let segments = vec![
            DanmuRenderSegment::Emote {
                image_path: PathBuf::from("汤圆.png"),
            },
            DanmuRenderSegment::Text("远远大于楚子航啊".to_string()),
        ];

        assert!(matches!(
            text_anchor_for_segment(&segments, 1),
            TextAnchor::Left
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time_zero() {
        assert_eq!(format_time(0.0), "0:00:00.00");
    }

    #[test]
    fn test_format_time_seconds() {
        assert_eq!(format_time(5.5), "0:00:05.50");
    }

    #[test]
    fn test_format_time_minutes() {
        assert_eq!(format_time(65.25), "0:01:05.25");
    }

    #[test]
    fn test_format_time_hours() {
        assert_eq!(format_time(3661.99), "1:01:01.99");
    }

    #[test]
    fn test_escape_text_backslash() {
        assert_eq!(escape_text("a\\b"), "a\\\\b");
    }

    #[test]
    fn test_escape_text_braces() {
        assert_eq!(escape_text("{test}"), "｛test｝");
    }

    #[test]
    fn test_escape_text_carriage_return() {
        // \r is removed, \n becomes \N (ASS line break)
        assert_eq!(escape_text("line\r\n"), "line\\N");
    }

    #[test]
    fn test_escape_text_plain() {
        assert_eq!(escape_text("hello world"), "hello world");
    }

    #[test]
    fn test_danmu2ass_options_default() {
        let opts = Danmu2AssOptions::default();
        assert_eq!(opts.font_size, 36.0);
        assert_eq!(opts.opacity, 0.8);
    }

    #[test]
    fn test_danmu_to_ass_empty() {
        let result = danmu_to_ass(vec![], Danmu2AssOptions::default());
        assert!(result.contains("[Script Info]"));
        assert!(result.contains("[V4+ Styles]"));
        assert!(result.contains("[Events]"));
    }

    #[test]
    fn test_danmu_to_ass_with_entries() {
        let danmus = vec![
            DanmuEntry {
                ts: 1000,
                content: "hello".to_string(),
            },
            DanmuEntry {
                ts: 5000,
                content: "world".to_string(),
            },
        ];
        let result = danmu_to_ass(danmus, Danmu2AssOptions::default());
        assert!(result.contains("[Events]"));
        assert!(result.contains("hello"));
        assert!(result.contains("world"));
    }

    #[test]
    fn test_danmu_to_ass_opacity() {
        // opacity 1.0 -> alpha 0x00
        let result = danmu_to_ass(
            vec![],
            Danmu2AssOptions {
                font_size: 36.0,
                opacity: 1.0,
            },
        );
        assert!(result.contains("&H00FFFFFF"));

        // opacity 0.0 -> alpha 0xFF
        let result = danmu_to_ass(
            vec![],
            Danmu2AssOptions {
                font_size: 36.0,
                opacity: 0.0,
            },
        );
        assert!(result.contains("&HFFFFFFFF"));
    }

    #[test]
    fn test_danmu_to_ass_font_size() {
        let result = danmu_to_ass(
            vec![],
            Danmu2AssOptions {
                font_size: 48.0,
                opacity: 0.8,
            },
        );
        assert!(result.contains(",48,"));
    }
}
