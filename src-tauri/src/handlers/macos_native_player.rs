#[cfg(feature = "gui")]
use tauri::WebviewWindow;

#[cfg(all(feature = "gui", target_os = "macos"))]
use {
    objc2::encode::{Encode, Encoding, RefEncode},
    objc2::exception::catch as catch_objc_exception,
    objc2::{msg_send, rc::Retained, runtime::AnyClass, runtime::AnyObject},
    objc2_foundation::{NSPoint, NSRect, NSSize, NSString, NSURL},
    std::{ffi::c_void, panic::AssertUnwindSafe, path::Path, sync::mpsc, time::Duration},
    tokio::task,
};

#[cfg(all(feature = "gui", target_os = "macos"))]
#[link(name = "AVFoundation", kind = "framework")]
extern "C" {}

#[cfg(all(feature = "gui", target_os = "macos"))]
#[link(name = "AVKit", kind = "framework")]
extern "C" {}

#[cfg(all(feature = "gui", target_os = "macos"))]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CMTime {
    value: i64,
    timescale: i32,
    flags: u32,
    epoch: i64,
}

#[cfg(all(feature = "gui", target_os = "macos"))]
unsafe impl Encode for CMTime {
    const ENCODING: Encoding = Encoding::Struct(
        "CMTime",
        &[i64::ENCODING, i32::ENCODING, u32::ENCODING, i64::ENCODING],
    );
}

#[cfg(all(feature = "gui", target_os = "macos"))]
unsafe impl RefEncode for CMTime {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Self::ENCODING);
}

#[cfg(all(feature = "gui", target_os = "macos"))]
const CMTIME_FLAGS_VALID: u32 = 1;

#[cfg(all(feature = "gui", target_os = "macos"))]
const NS_WINDOW_BELOW: isize = -1;
#[cfg(all(feature = "gui", target_os = "macos"))]
const AV_PLAYER_VIEW_CONTROLS_STYLE_NONE: isize = 0;
#[cfg(all(feature = "gui", target_os = "macos"))]
const NS_WINDOW_STYLE_MASK_BORDERLESS: usize = 0;
#[cfg(all(feature = "gui", target_os = "macos"))]
const NS_WINDOW_STYLE_MASK_FULLSCREEN: usize = 1 << 14;
#[cfg(all(feature = "gui", target_os = "macos"))]
const NS_BACKING_STORE_BUFFERED: usize = 2;
#[cfg(all(feature = "gui", target_os = "macos"))]
const MACOS_NATIVE_PLAYER_UNDERLAY_LEVEL_OFFSET: isize = 1;
#[cfg(all(feature = "gui", target_os = "macos"))]
const MACOS_NATIVE_PLAYER_LEFT_BORDER_WIDTH: f64 = 1.0;

#[cfg(feature = "gui")]
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MacOSNativePlayerSnapshot {
    pub current_time: f64,
    pub duration: f64,
    pub rate: f64,
    pub playing: bool,
}

#[cfg(feature = "gui")]
#[tauri::command]
pub async fn macos_native_player_supported() -> bool {
    cfg!(target_os = "macos")
}

#[cfg(feature = "gui")]
#[tauri::command]
pub async fn macos_native_player_mount(
    window: WebviewWindow,
    player_id: String,
    source: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    windowed_y_offset: f64,
    is_fullscreen: bool,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let owner_label = window.label().to_string();
        let rect = MacOSNativeRect::new(x, y, width, height, windowed_y_offset);
        enqueue_webview(&window, move |webview| {
            mount_macos_native_player(
                webview,
                &owner_label,
                &player_id,
                &source,
                rect,
                is_fullscreen,
            )
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (
            window,
            player_id,
            source,
            x,
            y,
            width,
            height,
            windowed_y_offset,
            is_fullscreen,
        );
        Err("macOS macOS native player is only available on macOS.".to_string())
    }
}

#[cfg(feature = "gui")]
#[tauri::command]
pub async fn macos_native_player_update_bounds(
    window: WebviewWindow,
    player_id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    windowed_y_offset: f64,
    is_fullscreen: bool,
    reason: String,
) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let owner_label = window.label().to_string();
        let rect = MacOSNativeRect::new(x, y, width, height, windowed_y_offset);
        query_webview(&window, move |webview| {
            update_macos_native_player_bounds(
                webview,
                &owner_label,
                &player_id,
                rect,
                is_fullscreen,
                &reason,
            )
        })
        .await
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (
            window,
            player_id,
            x,
            y,
            width,
            height,
            windowed_y_offset,
            is_fullscreen,
            reason,
        );
        Ok(false)
    }
}

#[cfg(feature = "gui")]
#[tauri::command]
pub async fn macos_native_player_set_visible(
    window: WebviewWindow,
    player_id: String,
    visible: bool,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let owner_label = window.label().to_string();
        enqueue_webview(&window, move |webview| {
            set_macos_native_player_visibility(webview, &owner_label, &player_id, visible)
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, player_id, visible);
        Err("macOS macOS native player is only available on macOS.".to_string())
    }
}

#[cfg(feature = "gui")]
#[tauri::command]
pub async fn macos_native_player_set_presentation_mode(
    window: WebviewWindow,
    player_id: String,
    use_fallback: bool,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let owner_label = window.label().to_string();
        enqueue_webview(&window, move |webview| {
            set_macos_native_player_presentation_mode(
                webview,
                &owner_label,
                &player_id,
                use_fallback,
            )
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, player_id, use_fallback);
        Err("macOS macOS native player is only available on macOS.".to_string())
    }
}

#[cfg(feature = "gui")]
#[tauri::command]
pub async fn macos_native_player_set_host_window_inner_size_keep_top_left(
    window: WebviewWindow,
    width: u32,
    height: u32,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        enqueue_webview(&window, move |webview| {
            let (_, ns_window) = resolve_host_views(&webview)?;
            catch_objc(
                "Failed to resize the host window while keeping its top-left position.",
                || unsafe {
                    let backing_scale_factor: f64 = msg_send![ns_window, backingScaleFactor];
                    let safe_scale_factor =
                        if backing_scale_factor.is_finite() && backing_scale_factor > 0.0 {
                            backing_scale_factor
                        } else {
                            1.0
                        };
                    let current_frame: NSRect = msg_send![ns_window, frame];
                    let current_top_left = NSPoint {
                        x: current_frame.origin.x,
                        y: current_frame.origin.y + current_frame.size.height,
                    };
                    let content_rect = NSRect {
                        origin: NSPoint { x: 0.0, y: 0.0 },
                        size: NSSize {
                            width: width as f64 / safe_scale_factor,
                            height: height as f64 / safe_scale_factor,
                        },
                    };
                    let target_frame: NSRect =
                        msg_send![ns_window, frameRectForContentRect: content_rect];
                    let next_frame = NSRect {
                        origin: NSPoint {
                            x: current_top_left.x,
                            y: current_top_left.y - target_frame.size.height,
                        },
                        size: target_frame.size,
                    };
                    let _: () = msg_send![ns_window, setFrame: next_frame, display: true];
                },
            )
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, width, height);
        Err("Native host window resizing is only available on macOS.".to_string())
    }
}

#[cfg(feature = "gui")]
#[tauri::command]
pub async fn macos_native_player_focus_host(window: WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        enqueue_webview(&window, move |webview| {
            focus_macos_native_player_host_webview(webview)
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
        Ok(())
    }
}

#[cfg(feature = "gui")]
#[tauri::command]
pub async fn macos_native_player_play(
    window: WebviewWindow,
    player_id: String,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let owner_label = window.label().to_string();
        enqueue_webview(&window, move |webview| {
            let _ = webview;
            ignore_missing_macos_native_player_error(with_macos_native_player(
                &owner_label,
                &player_id,
                |player| {
                    let _: () = unsafe { msg_send![player, play] };
                    Ok(())
                },
            ))
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, player_id);
        Err("macOS macOS native player is only available on macOS.".to_string())
    }
}

#[cfg(feature = "gui")]
#[tauri::command]
pub async fn macos_native_player_pause(
    window: WebviewWindow,
    player_id: String,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let owner_label = window.label().to_string();
        enqueue_webview(&window, move |webview| {
            let _ = webview;
            ignore_missing_macos_native_player_error(with_macos_native_player(
                &owner_label,
                &player_id,
                |player| {
                    let _: () = unsafe { msg_send![player, pause] };
                    Ok(())
                },
            ))
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, player_id);
        Err("macOS macOS native player is only available on macOS.".to_string())
    }
}

#[cfg(feature = "gui")]
#[tauri::command]
pub async fn macos_native_player_seek(
    window: WebviewWindow,
    player_id: String,
    seconds: f64,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let owner_label = window.label().to_string();
        let target_time = seconds_to_cmtime(seconds);
        let zero_tolerance = seconds_to_cmtime(0.0);
        enqueue_webview(&window, move |webview| {
            let _ = webview;
            ignore_missing_macos_native_player_error(with_macos_native_player(
                &owner_label,
                &player_id,
                |player| {
                    // Use zero tolerance to avoid AVPlayer snapping to nearby keyframes.
                    let _: () = unsafe {
                        msg_send![
                            player,
                            seekToTime: target_time,
                            toleranceBefore: zero_tolerance,
                            toleranceAfter: zero_tolerance
                        ]
                    };
                    Ok(())
                },
            ))
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, player_id, seconds);
        Err("macOS macOS native player is only available on macOS.".to_string())
    }
}

#[cfg(feature = "gui")]
#[tauri::command]
pub async fn macos_native_player_set_rate(
    window: WebviewWindow,
    player_id: String,
    rate: f64,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let owner_label = window.label().to_string();
        let next_rate = rate.clamp(0.0, 16.0) as f32;
        enqueue_webview(&window, move |webview| {
            let _ = webview;
            ignore_missing_macos_native_player_error(with_macos_native_player(
                &owner_label,
                &player_id,
                |player| {
                    let _: () = unsafe { msg_send![player, setRate: next_rate] };
                    Ok(())
                },
            ))
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, player_id, rate);
        Err("macOS macOS native player is only available on macOS.".to_string())
    }
}

#[cfg(feature = "gui")]
#[tauri::command]
pub async fn macos_native_player_set_volume(
    window: WebviewWindow,
    player_id: String,
    volume: f64,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let owner_label = window.label().to_string();
        let next_volume = volume.clamp(0.0, 1.0) as f32;
        enqueue_webview(&window, move |webview| {
            let _ = webview;
            ignore_missing_macos_native_player_error(with_macos_native_player(
                &owner_label,
                &player_id,
                |player| {
                    let _: () = unsafe { msg_send![player, setVolume: next_volume] };
                    Ok(())
                },
            ))
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, player_id, volume);
        Err("macOS macOS native player is only available on macOS.".to_string())
    }
}

#[cfg(feature = "gui")]
#[tauri::command]
pub async fn macos_native_player_get_state(
    window: WebviewWindow,
    player_id: String,
) -> Result<MacOSNativePlayerSnapshot, String> {
    #[cfg(target_os = "macos")]
    {
        let owner_label = window.label().to_string();
        query_webview(&window, move |webview| {
            let _ = webview;
            with_macos_native_player(&owner_label, &player_id, |player| {
                let current_time: CMTime = unsafe { msg_send![player, currentTime] };
                let rate: f32 = unsafe { msg_send![player, rate] };
                let current_item: *mut AnyObject = unsafe { msg_send![player, currentItem] };
                let duration = if let Some(item) = unsafe { current_item.as_ref() } {
                    let time: CMTime = unsafe { msg_send![item, duration] };
                    cmtime_to_seconds(time)
                } else {
                    0.0
                };
                let current_seconds = cmtime_to_seconds(current_time);
                Ok(MacOSNativePlayerSnapshot {
                    current_time: current_seconds,
                    duration,
                    rate: rate as f64,
                    playing: rate.abs() > 0.001,
                })
            })
        })
        .await
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, player_id);
        Err("macOS macOS native player is only available on macOS.".to_string())
    }
}

#[cfg(feature = "gui")]
#[tauri::command]
pub async fn macos_native_player_unmount(
    window: WebviewWindow,
    player_id: String,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let owner_label = window.label().to_string();
        enqueue_webview(&window, move |webview| {
            let _ = webview;
            unmount_macos_native_player(&owner_label, &player_id)
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, player_id);
        Err("macOS macOS native player is only available on macOS.".to_string())
    }
}

#[cfg(feature = "gui")]
#[allow(dead_code)]
pub fn cleanup_macos_native_players_for_window(window: &WebviewWindow) {
    #[cfg(target_os = "macos")]
    {
        let label = window.label().to_string();
        let cleanup_context = format!("Failed to clean up macOS native players for {label}.");
        let _ = window.with_webview(move |_webview| {
            match catch_objc_result(&cleanup_context, || cleanup_macos_native_players(&label)) {
                Ok(removed_count) => {
                    if removed_count > 0 {
                        log::info!("Unmounted macOS native players for {label}");
                    }
                }
                Err(error) => {
                    log::error!("macos_native_player cleanup failed on close for {label}: {error}");
                }
            }
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
    }
}

#[cfg(feature = "gui")]
#[allow(dead_code)]
pub fn sync_macos_native_player_dock_preview_for_window(
    window: &WebviewWindow,
    player_id: &str,
    show_fallback: bool,
) {
    #[cfg(target_os = "macos")]
    {
        let label = window.label().to_string();
        let player_id = player_id.to_string();
        let sync_context =
            format!("Failed to sync macOS native player dock preview for {label}::{player_id}.");
        let _ = window.with_webview(move |webview| {
            match catch_objc_result(&sync_context, || {
                let (_, ns_window) = resolve_host_views(&webview)?;
                let content_view = host_content_view(ns_window)
                    .ok_or_else(|| "Failed to resolve host content view.".to_string())?;
                let window_identifier = player_window_identifier(&label, &player_id);
                let fallback_identifier =
                    NSString::from_str(&dock_fallback_identifier(&window_identifier));
                let is_parent_miniaturized: bool = unsafe { msg_send![ns_window, isMiniaturized] };
                let should_show_fallback = show_fallback && is_parent_miniaturized;

                let Some(child_window) = find_player_window(&window_identifier) else {
                    return Ok(false);
                };

                if let Some(fallback_view) =
                    find_player_view_in_subtree(content_view, &fallback_identifier)
                {
                    sync_fallback_player_view(
                        fallback_view,
                        child_window,
                        None,
                        should_show_fallback,
                    );
                };

                if should_show_fallback {
                    let _: () = unsafe { msg_send![child_window, setAlphaValue: 0.0f64] };
                } else if !show_fallback {
                    let is_parent_fullscreen = is_host_window_fullscreen(ns_window);
                    restore_macos_native_player_child_window(
                        child_window,
                        ns_window,
                        is_parent_fullscreen,
                        true,
                    );
                }

                Ok(sync_parent_miniwindow_image(ns_window, child_window))
            }) {
                Ok(true) => {
                    log::debug!("Synced macOS native player dock preview for {label}::{player_id}");
                }
                Ok(false) => {}
                Err(error) => {
                    log::warn!(
                        "macos_native_player dock preview sync failed for {label}::{player_id}: {error}"
                    );
                }
            }
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, player_id, show_fallback);
    }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
#[derive(Clone, Copy)]
struct MacOSNativeRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    windowed_y_offset: f64,
}

#[cfg(all(feature = "gui", target_os = "macos"))]
impl MacOSNativeRect {
    fn new(x: f64, y: f64, width: f64, height: f64, windowed_y_offset: f64) -> Self {
        Self {
            x,
            y,
            width: width.max(0.0),
            height: height.max(0.0),
            windowed_y_offset: windowed_y_offset.max(0.0),
        }
    }

    fn host_view_rect(self) -> Self {
        Self {
            x: self.x,
            y: self.y - self.windowed_y_offset,
            width: self.width,
            height: self.height,
            windowed_y_offset: 0.0,
        }
    }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn is_host_window_fullscreen(ns_window: &AnyObject) -> bool {
    unsafe {
        let style_mask: usize = msg_send![ns_window, styleMask];
        (style_mask & NS_WINDOW_STYLE_MASK_FULLSCREEN) != 0
    }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn macos_native_player_target_window_level(
    parent_window: &AnyObject,
    is_fullscreen: bool,
) -> isize {
    unsafe {
        let parent_window_level: isize = msg_send![parent_window, level];
        if is_fullscreen {
            parent_window_level - MACOS_NATIVE_PLAYER_UNDERLAY_LEVEL_OFFSET
        } else {
            parent_window_level
        }
    }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn apply_macos_native_player_child_window_visuals(child_window: &AnyObject) {
    let Ok(color_class) = get_class(c"NSColor") else {
        return;
    };

    unsafe {
        let clear_color: Retained<AnyObject> = msg_send![color_class, clearColor];
        let _: () = msg_send![child_window, setOpaque: false];
        let _: () = msg_send![child_window, setBackgroundColor: &*clear_color];
        let _: () = msg_send![child_window, setHasShadow: false];
        let _: () = msg_send![child_window, invalidateShadow];
    }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn macos_native_player_border_identifier(window_identifier: &str) -> Retained<NSString> {
    NSString::from_str(&format!("{window_identifier}::left-border"))
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn find_subview_by_identifier<'a>(
    root_view: &'a AnyObject,
    identifier: &NSString,
) -> Option<&'a AnyObject> {
    unsafe {
        let subviews: *mut AnyObject = msg_send![root_view, subviews];
        let subviews = subviews.as_ref()?;
        let count: usize = msg_send![subviews, count];

        for index in 0..count {
            let subview: *mut AnyObject = msg_send![subviews, objectAtIndex: index];
            let Some(subview) = subview.as_ref() else {
                continue;
            };
            let view_identifier: *mut AnyObject = msg_send![subview, identifier];
            if let Some(view_identifier) = view_identifier.as_ref() {
                let matches: bool = msg_send![view_identifier, isEqualToString: identifier];
                if matches {
                    return Some(subview);
                }
            }
        }
    }

    None
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn ensure_macos_native_player_left_border(
    player_view: &AnyObject,
    window_identifier: &str,
    height: f64,
) -> Result<(), String> {
    let border_identifier = macos_native_player_border_identifier(window_identifier);
    let border_frame = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(MACOS_NATIVE_PLAYER_LEFT_BORDER_WIDTH, height.max(0.0)),
    );

    unsafe {
        if let Some(existing_view) = find_subview_by_identifier(player_view, &border_identifier) {
            let _: () = msg_send![existing_view, setFrame: border_frame];
            let layer: *mut AnyObject = msg_send![existing_view, layer];
            if let Some(layer) = layer.as_ref() {
                let color_class = get_class(c"NSColor")?;
                let separator_color: Retained<AnyObject> = msg_send![color_class, separatorColor];
                let cg_color: *mut AnyObject = msg_send![&*separator_color, CGColor];
                let _: () = msg_send![layer, setBackgroundColor: cg_color];
            }
        } else {
            let view_class = get_class(c"NSView")?;
            let created_view: Retained<AnyObject> = msg_send![view_class, new];
            let _: () = msg_send![&*created_view, setIdentifier: &*border_identifier];
            let _: () = msg_send![&*created_view, setWantsLayer: true];
            let _: () = msg_send![&*created_view, setFrame: border_frame];
            let layer: *mut AnyObject = msg_send![&*created_view, layer];
            if let Some(layer) = layer.as_ref() {
                let color_class = get_class(c"NSColor")?;
                let separator_color: Retained<AnyObject> = msg_send![color_class, separatorColor];
                let cg_color: *mut AnyObject = msg_send![&*separator_color, CGColor];
                let _: () = msg_send![layer, setBackgroundColor: cg_color];
            }
            let _: () = msg_send![player_view, addSubview: &*created_view];
        }
    }

    Ok(())
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn restore_macos_native_player_child_window(
    child_window: &AnyObject,
    parent_window: &AnyObject,
    is_fullscreen: bool,
    enforce_order_front: bool,
) {
    unsafe {
        apply_macos_native_player_child_window_visuals(child_window);
        let nil_sender: *mut AnyObject = std::ptr::null_mut();
        let parent_window_number: i32 = msg_send![parent_window, windowNumber];
        let target_window_level =
            macos_native_player_target_window_level(parent_window, is_fullscreen);
        let current_window_level: isize = msg_send![child_window, level];

        let _: () = msg_send![child_window, setAlphaValue: 1.0f64];
        if current_window_level != target_window_level {
            let _: () = msg_send![child_window, setLevel: target_window_level];
        }
        if enforce_order_front {
            let _: () = msg_send![child_window, orderFront: nil_sender];
        }
        let _: () = msg_send![
            child_window,
            orderWindow: NS_WINDOW_BELOW,
            relativeTo: parent_window_number
        ];
    }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn enqueue_webview<F>(window: &WebviewWindow, f: F) -> Result<(), String>
where
    F: FnOnce(tauri::webview::PlatformWebview) -> Result<(), String> + Send + 'static,
{
    window
        .with_webview(move |webview| {
            if let Err(error) =
                catch_objc_result("macOS macOS native player UI task failed.", || f(webview))
            {
                log::error!("macos_native_player UI task failed: {error}");
            }
        })
        .map_err(|error| error.to_string())
}

#[cfg(all(feature = "gui", target_os = "macos"))]
async fn query_webview<T, F>(window: &WebviewWindow, f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(tauri::webview::PlatformWebview) -> Result<T, String> + Send + 'static,
{
    let (tx, rx) = mpsc::sync_channel(1);
    window
        .with_webview(move |webview| {
            let _ = tx.send(catch_objc_result(
                "macOS macOS native player query failed.",
                || f(webview),
            ));
        })
        .map_err(|error| error.to_string())?;

    task::spawn_blocking(move || {
        rx.recv_timeout(Duration::from_millis(600))
            .map_err(|_| "Timed out waiting for the macOS native player.".to_string())
    })
    .await
    .map_err(|error| error.to_string())?
    .and_then(|result| result)
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn mount_macos_native_player(
    webview: tauri::webview::PlatformWebview,
    owner_label: &str,
    player_id: &str,
    source: &str,
    rect: MacOSNativeRect,
    is_fullscreen: bool,
) -> Result<(), String> {
    let (webview_view, ns_window) = resolve_host_views(&webview)?;
    configure_transparent_webview(webview_view)?;
    configure_transparent_window(ns_window)?;
    let window_identifier = player_window_identifier(owner_label, player_id);
    let fallback_identifier = dock_fallback_identifier(&window_identifier);
    remove_existing_container(&window_identifier);

    let frame = convert_rect_to_screen(webview_view, ns_window, rect);
    let content_view = host_content_view(ns_window)
        .ok_or_else(|| "Failed to resolve host content view.".to_string())?;
    let host_frame = convert_rect_to_host_view(webview_view, content_view, rect.host_view_rect());
    let url = make_native_url(source)?;
    let player_view_class = get_class(c"AVPlayerView")?;
    let player_class = get_class(c"AVPlayer")?;
    let window_class = get_class(c"NSWindow")?;
    let player_view: Retained<AnyObject> = unsafe { msg_send![player_view_class, new] };
    let raw_player: *mut AnyObject = unsafe { msg_send![player_class, alloc] };
    let player: *mut AnyObject = unsafe { msg_send![raw_player, initWithURL: &*url] };
    let player = unsafe { Retained::from_raw(player) }
        .ok_or_else(|| "Failed to initialize AVPlayer.".to_string())?;
    let raw_window: *mut AnyObject = unsafe { msg_send![window_class, alloc] };
    let child_window: *mut AnyObject = unsafe {
        msg_send![
            raw_window,
            initWithContentRect: frame,
            styleMask: NS_WINDOW_STYLE_MASK_BORDERLESS,
            backing: NS_BACKING_STORE_BUFFERED,
            defer: false
        ]
    };
    let child_window = unsafe { Retained::from_raw(child_window) }
        .ok_or_else(|| "Failed to initialize player window.".to_string())?;
    let identifier = NSString::from_str(&window_identifier);
    let dock_identifier = NSString::from_str(&fallback_identifier);
    remove_player_view_from_subtree(content_view, &dock_identifier);

    catch_objc(
        "Failed to attach the macOS native player view.",
        || unsafe {
            let player_frame = window_local_rect(frame.size.width, frame.size.height);
            let parent_window_number: i32 = msg_send![ns_window, windowNumber];
            let target_window_level =
                macos_native_player_target_window_level(ns_window, is_fullscreen);
            let nil_sender: *mut AnyObject = std::ptr::null_mut();
            let (): () = msg_send![&*player_view, setIdentifier: &*identifier];
            let (): () = msg_send![&*player_view, setFrame: player_frame];
            let (): () =
                msg_send![&*player_view, setControlsStyle: AV_PLAYER_VIEW_CONTROLS_STYLE_NONE];
            let (): () = msg_send![&*player_view, setShowsFrameSteppingButtons: false];
            let (): () = msg_send![&*player_view, setShowsSharingServiceButton: false];
            let (): () = msg_send![&*player_view, setShowsFullScreenToggleButton: false];
            let (): () = msg_send![&*player_view, setPlayer: &*player];
            apply_macos_native_player_child_window_visuals(&*child_window);
            let (): () = msg_send![&*child_window, setIgnoresMouseEvents: true];
            let (): () = msg_send![&*child_window, setCanHide: false];
            let (): () = msg_send![&*child_window, setLevel: target_window_level];
            let (): () = msg_send![&*child_window, setContentView: &*player_view];
            let (): () =
                msg_send![ns_window, addChildWindow: &*child_window, ordered: NS_WINDOW_BELOW];
            let (): () = msg_send![&*child_window, orderFront: nil_sender];
            let (): () = msg_send![&*child_window, orderWindow: NS_WINDOW_BELOW, relativeTo: parent_window_number];
            let (): () = msg_send![ns_window, makeKeyAndOrderFront: nil_sender];

            // Fallback AVPlayerView lives in host window for Dock miniaturize transition.
            let dock_player_view: Retained<AnyObject> = msg_send![player_view_class, new];
            let (): () = msg_send![&*dock_player_view, setIdentifier: &*dock_identifier];
            let (): () = msg_send![&*dock_player_view, setFrame: host_frame];
            let (): () =
                msg_send![&*dock_player_view, setControlsStyle: AV_PLAYER_VIEW_CONTROLS_STYLE_NONE];
            let (): () = msg_send![&*dock_player_view, setShowsFrameSteppingButtons: false];
            let (): () = msg_send![&*dock_player_view, setShowsSharingServiceButton: false];
            let (): () = msg_send![&*dock_player_view, setShowsFullScreenToggleButton: false];
            let (): () = msg_send![&*dock_player_view, setHidden: true];
            let null_player: *mut AnyObject = std::ptr::null_mut();
            let (): () = msg_send![&*dock_player_view, setPlayer: null_player];
            let zero_frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0));
            let (): () = msg_send![&*dock_player_view, setFrame: zero_frame];
            let (): () = msg_send![
                content_view,
                addSubview: &*dock_player_view,
                positioned: NS_WINDOW_BELOW,
                relativeTo: webview_view
            ];
        },
    )?;
    ensure_macos_native_player_left_border(&*player_view, &window_identifier, frame.size.height)?;
    log::info!("Mounted macOS native player for {player_id}");
    Ok(())
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn update_macos_native_player_bounds(
    webview: tauri::webview::PlatformWebview,
    owner_label: &str,
    player_id: &str,
    rect: MacOSNativeRect,
    is_fullscreen: bool,
    _debug_reason: &str,
) -> Result<bool, String> {
    let (webview_view, ns_window) = resolve_host_views(&webview)?;
    let window_identifier = player_window_identifier(owner_label, player_id);
    let Some(child_window) = find_player_window(&window_identifier) else {
        return Ok(false);
    };
    let is_parent_miniaturized: bool = unsafe { msg_send![ns_window, isMiniaturized] };
    if is_parent_miniaturized {
        return Ok(true);
    }
    let content_view = host_content_view(ns_window)
        .ok_or_else(|| "Failed to resolve host content view.".to_string())?;
    let fallback_identifier = NSString::from_str(&dock_fallback_identifier(&window_identifier));
    let frame = convert_rect_to_screen(webview_view, ns_window, rect);
    let host_frame = convert_rect_to_host_view(webview_view, content_view, rect.host_view_rect());

    unsafe {
        apply_macos_native_player_child_window_visuals(child_window);
        let parent_window_number: i32 = msg_send![ns_window, windowNumber];
        let target_window_level = macos_native_player_target_window_level(ns_window, is_fullscreen);
        let current_frame: NSRect = msg_send![child_window, frame];
        let did_set_frame = !nsrect_matches(current_frame, frame);
        if did_set_frame {
            let (): () = msg_send![child_window, setFrame: frame, display: true];
        }
        let current_window_level: isize = msg_send![child_window, level];
        let did_set_level = current_window_level != target_window_level;
        if did_set_level {
            let (): () = msg_send![child_window, setLevel: target_window_level];
        }
        if let Some(player_view) = player_view_for_window(child_window) {
            let player_frame = window_local_rect(frame.size.width, frame.size.height);
            let (): () = msg_send![player_view, setFrame: player_frame];
            ensure_macos_native_player_left_border(
                player_view,
                &window_identifier,
                frame.size.height,
            )?;
        }
        let fallback_visible = if let Some(fallback_view) =
            find_player_view_in_subtree(content_view, &fallback_identifier)
        {
            let is_hidden: bool = msg_send![fallback_view, isHidden];
            sync_fallback_player_view(fallback_view, child_window, Some(host_frame), !is_hidden);
            !is_hidden
        } else {
            false
        };
        let mut did_restore_alpha = false;
        if !fallback_visible {
            let current_alpha: f64 = msg_send![child_window, alphaValue];
            if current_alpha < 0.999 {
                let (): () = msg_send![child_window, setAlphaValue: 1.0f64];
                did_restore_alpha = true;
            }
            if did_set_frame || did_set_level || did_restore_alpha {
                let (): () = msg_send![
                    child_window,
                    orderWindow: NS_WINDOW_BELOW,
                    relativeTo: parent_window_number
                ];
            }
        }
    }

    Ok(true)
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn set_macos_native_player_visibility(
    webview: tauri::webview::PlatformWebview,
    owner_label: &str,
    player_id: &str,
    visible: bool,
) -> Result<(), String> {
    let (_, ns_window) = resolve_host_views(&webview)?;
    let window_identifier = player_window_identifier(owner_label, player_id);
    let Some(child_window) = find_player_window(&window_identifier) else {
        return Ok(());
    };

    unsafe {
        if visible {
            let is_parent_miniaturized: bool = msg_send![ns_window, isMiniaturized];
            if is_parent_miniaturized {
                let (): () = msg_send![child_window, setAlphaValue: 1.0f64];
                return Ok(());
            }
            let is_parent_fullscreen = is_host_window_fullscreen(ns_window);
            restore_macos_native_player_child_window(
                child_window,
                ns_window,
                is_parent_fullscreen,
                true,
            );
        } else {
            let (): () = msg_send![child_window, setAlphaValue: 0.0f64];
        }
    }

    Ok(())
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn set_macos_native_player_presentation_mode(
    webview: tauri::webview::PlatformWebview,
    owner_label: &str,
    player_id: &str,
    use_fallback: bool,
) -> Result<(), String> {
    let (_, ns_window) = resolve_host_views(&webview)?;
    let content_view = host_content_view(ns_window)
        .ok_or_else(|| "Failed to resolve host content view.".to_string())?;
    let window_identifier = player_window_identifier(owner_label, player_id);
    let fallback_identifier = NSString::from_str(&dock_fallback_identifier(&window_identifier));
    let Some(child_window) = find_player_window(&window_identifier) else {
        return Ok(());
    };

    unsafe {
        let is_parent_miniaturized: bool = msg_send![ns_window, isMiniaturized];
        let should_show_fallback = use_fallback || is_parent_miniaturized;

        if let Some(fallback_view) = find_player_view_in_subtree(content_view, &fallback_identifier)
        {
            sync_fallback_player_view(fallback_view, child_window, None, should_show_fallback);
        }

        if should_show_fallback {
            let _: () = msg_send![child_window, setAlphaValue: 0.0f64];
            return Ok(());
        }

        let is_parent_fullscreen = is_host_window_fullscreen(ns_window);
        restore_macos_native_player_child_window(
            child_window,
            ns_window,
            is_parent_fullscreen,
            true,
        );
    }

    Ok(())
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn focus_macos_native_player_host_webview(
    webview: tauri::webview::PlatformWebview,
) -> Result<(), String> {
    let (webview_view, ns_window) = resolve_host_views(&webview)?;

    catch_objc("Failed to focus the native host webview.", || unsafe {
        let nil_sender: *mut AnyObject = std::ptr::null_mut();
        let _: () = msg_send![ns_window, makeKeyAndOrderFront: nil_sender];
        let _: bool = msg_send![ns_window, makeFirstResponder: webview_view];
    })?;

    Ok(())
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn unmount_macos_native_player(owner_label: &str, player_id: &str) -> Result<(), String> {
    let window_identifier = player_window_identifier(owner_label, player_id);
    if remove_existing_container(&window_identifier) {
        log::info!("Unmounted macOS native player for {player_id}");
    }
    Ok(())
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn sync_parent_miniwindow_image(parent_window: &AnyObject, child_window: &AnyObject) -> bool {
    unsafe {
        let child_miniwindow_image: *mut AnyObject = msg_send![child_window, miniwindowImage];
        let Some(child_miniwindow_image) = child_miniwindow_image.as_ref() else {
            return false;
        };

        let _: () = msg_send![parent_window, setMiniwindowImage: child_miniwindow_image];
        true
    }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
#[allow(dead_code)]
fn cleanup_macos_native_players(owner_label: &str) -> Result<usize, String> {
    remove_all_macos_native_players(owner_label)
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn with_macos_native_player<T, F>(owner_label: &str, player_id: &str, f: F) -> Result<T, String>
where
    F: FnOnce(&AnyObject) -> Result<T, String>,
{
    let window_identifier = player_window_identifier(owner_label, player_id);
    let child_window = find_player_window(&window_identifier)
        .ok_or_else(|| "macOS macOS native player is not mounted.".to_string())?;
    let player_view = player_view_for_window(child_window)
        .ok_or_else(|| "macOS macOS native player view is unavailable.".to_string())?;
    let player: *mut AnyObject = unsafe { msg_send![player_view, player] };
    let player = unsafe { player.as_ref() }
        .ok_or_else(|| "macOS macOS native player instance is unavailable.".to_string())?;

    f(player)
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn ignore_missing_macos_native_player_error(result: Result<(), String>) -> Result<(), String> {
    match result {
        Err(error) if error == "macOS macOS native player is not mounted." => Ok(()),
        other => other,
    }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn resolve_host_views(
    webview: &tauri::webview::PlatformWebview,
) -> Result<(&AnyObject, &AnyObject), String> {
    let webview_view = as_object(webview.inner(), "WKWebView")?;
    let ns_window = unsafe {
        let ptr = webview.ns_window();
        (ptr as *mut AnyObject).as_ref()
    }
    .ok_or_else(|| "Failed to resolve the native host window.".to_string())?;

    Ok((webview_view, ns_window))
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn configure_transparent_webview(webview_view: &AnyObject) -> Result<(), String> {
    let number_class = get_class(c"NSNumber")?;
    let draws_background_key = NSString::from_str("drawsBackground");
    let disabled_background: Retained<AnyObject> =
        unsafe { msg_send![number_class, numberWithBool: false] };

    catch_objc(
        "Failed to enable transparent WebKit composition for the macOS native player.",
        || unsafe {
            let _: () = msg_send![
                webview_view,
                setValue: &*disabled_background,
                forKey: &*draws_background_key
            ];
        },
    )?;

    Ok(())
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn configure_transparent_window(ns_window: &AnyObject) -> Result<(), String> {
    let color_class = get_class(c"NSColor")?;
    let clear_color: Retained<AnyObject> = unsafe { msg_send![color_class, clearColor] };

    catch_objc(
        "Failed to enable transparent window composition for the macOS native player.",
        || unsafe {
            let _: () = msg_send![ns_window, setOpaque: false];
            let _: () = msg_send![ns_window, setBackgroundColor: &*clear_color];
        },
    )?;

    Ok(())
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn player_window_identifier(owner_label: &str, player_id: &str) -> String {
    format!("{owner_label}::{player_id}")
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn dock_fallback_identifier(window_identifier: &str) -> String {
    format!("{window_identifier}::dock-fallback")
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn remove_existing_container(window_identifier: &str) -> bool {
    let mut removed = false;

    if let Some(child_window) = find_player_window(window_identifier) {
        dispose_macos_native_player_window(child_window);
        removed = true;
    }

    let fallback_identifier = dock_fallback_identifier(window_identifier);
    if remove_player_view_by_identifier(&fallback_identifier) {
        removed = true;
    }

    removed
}

#[cfg(all(feature = "gui", target_os = "macos"))]
#[allow(dead_code)]
fn remove_all_macos_native_players(owner_label: &str) -> Result<usize, String> {
    let identifier_prefix = NSString::from_str(&format!("{owner_label}::"));
    let player_view_class = get_class(c"AVPlayerView")?;
    let mut matched: Vec<*mut AnyObject> = Vec::new();

    unsafe {
        let application_class = get_class(c"NSApplication")?;
        let application: *mut AnyObject = msg_send![application_class, sharedApplication];
        let Some(application) = application.as_ref() else {
            return Ok(0);
        };
        let windows: *mut AnyObject = msg_send![application, windows];
        let Some(windows) = windows.as_ref() else {
            return Ok(0);
        };
        let count: usize = msg_send![windows, count];

        for index in 0..count {
            let child_window: *mut AnyObject = msg_send![windows, objectAtIndex: index];
            let Some(child_window_ref) = child_window.as_ref() else {
                continue;
            };
            let content_view: *mut AnyObject = msg_send![child_window_ref, contentView];
            let Some(content_view) = content_view.as_ref() else {
                continue;
            };
            let is_player_view: bool = msg_send![content_view, isKindOfClass: player_view_class];
            if !is_player_view {
                continue;
            }
            let view_identifier: *mut AnyObject = msg_send![content_view, identifier];
            let Some(view_identifier) = view_identifier.as_ref() else {
                continue;
            };
            let matches_owner: bool = msg_send![view_identifier, hasPrefix: &*identifier_prefix];
            if matches_owner {
                matched.push(child_window);
            }
        }
    }

    let removed_count = matched.len();

    for child_window in matched {
        if let Some(child_window) = unsafe { child_window.as_ref() } {
            dispose_macos_native_player_window(child_window);
        }
    }

    Ok(removed_count)
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn dispose_macos_native_player_window(child_window: &AnyObject) {
    unsafe {
        if let Some(player_view) = player_view_for_window(child_window) {
            let player: *mut AnyObject = msg_send![player_view, player];
            if let Some(player) = player.as_ref() {
                let _: () = msg_send![player, pause];
                let null_item: *mut AnyObject = std::ptr::null_mut();
                let _: () = msg_send![player, replaceCurrentItemWithPlayerItem: null_item];
            }

            let null_view: *mut AnyObject = std::ptr::null_mut();
            let _: () = msg_send![child_window, setContentView: null_view];
        }

        let nil_sender: *mut AnyObject = std::ptr::null_mut();
        let _: () = msg_send![child_window, orderOut: nil_sender];
        let parent_window: *mut AnyObject = msg_send![child_window, parentWindow];
        if let Some(parent_window) = parent_window.as_ref() {
            let _: () = msg_send![parent_window, removeChildWindow: child_window];
        }
    }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn find_player_window<'a>(window_identifier: &str) -> Option<&'a AnyObject> {
    let identifier = NSString::from_str(window_identifier);
    unsafe {
        let application_class = get_class(c"NSApplication").ok()?;
        let application: *mut AnyObject = msg_send![application_class, sharedApplication];
        let application = application.as_ref()?;
        let windows: *mut AnyObject = msg_send![application, windows];
        let Some(windows) = windows.as_ref() else {
            return None;
        };
        let count: usize = msg_send![windows, count];

        for index in 0..count {
            let child_window: *mut AnyObject = msg_send![windows, objectAtIndex: index];
            let Some(child_window) = child_window.as_ref() else {
                continue;
            };
            let Some(player_view) = player_view_for_window(child_window) else {
                continue;
            };
            let view_identifier: *mut AnyObject = msg_send![player_view, identifier];
            let Some(view_identifier) = view_identifier.as_ref() else {
                continue;
            };
            let matches: bool = msg_send![view_identifier, isEqualToString: &*identifier];
            if matches {
                return Some(child_window);
            }
        }

        None
    }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn player_view_for_window<'a>(child_window: &'a AnyObject) -> Option<&'a AnyObject> {
    let player_view_class = get_class(c"AVPlayerView").ok()?;

    unsafe {
        let content_view: *mut AnyObject = msg_send![child_window, contentView];
        let content_view = content_view.as_ref()?;
        let is_player_view: bool = msg_send![content_view, isKindOfClass: player_view_class];
        if is_player_view {
            Some(content_view)
        } else {
            None
        }
    }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn player_for_window(child_window: &AnyObject) -> Option<*mut AnyObject> {
    let player_view = player_view_for_window(child_window)?;
    let player: *mut AnyObject = unsafe { msg_send![player_view, player] };
    if player.is_null() {
        None
    } else {
        Some(player)
    }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn sync_fallback_player_view(
    fallback_view: &AnyObject,
    child_window: &AnyObject,
    frame: Option<NSRect>,
    visible: bool,
) {
    unsafe {
        if let Some(frame) = frame {
            let _: () = msg_send![fallback_view, setFrame: frame];
        }

        if visible {
            if let Some(player) = player_for_window(child_window) {
                let _: () = msg_send![fallback_view, setPlayer: player];
            }
            let _: () = msg_send![fallback_view, setHidden: false];
            return;
        }

        let null_player: *mut AnyObject = std::ptr::null_mut();
        let zero_frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0));
        let _: () = msg_send![fallback_view, setHidden: true];
        let _: () = msg_send![fallback_view, setPlayer: null_player];
        let _: () = msg_send![fallback_view, setFrame: zero_frame];
    }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn host_content_view<'a>(ns_window: &'a AnyObject) -> Option<&'a AnyObject> {
    unsafe {
        let content_view: *mut AnyObject = msg_send![ns_window, contentView];
        content_view.as_ref()
    }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn find_player_view_in_subtree<'a>(
    root_view: &'a AnyObject,
    identifier: &NSString,
) -> Option<&'a AnyObject> {
    let player_view_class = get_class(c"AVPlayerView").ok()?;

    unsafe {
        let is_player_view: bool = msg_send![root_view, isKindOfClass: player_view_class];
        if is_player_view {
            let view_identifier: *mut AnyObject = msg_send![root_view, identifier];
            if let Some(view_identifier) = view_identifier.as_ref() {
                let matches: bool = msg_send![view_identifier, isEqualToString: identifier];
                if matches {
                    return Some(root_view);
                }
            }
        }

        let subviews: *mut AnyObject = msg_send![root_view, subviews];
        let subviews = subviews.as_ref()?;
        let count: usize = msg_send![subviews, count];
        for index in 0..count {
            let subview: *mut AnyObject = msg_send![subviews, objectAtIndex: index];
            let Some(subview) = subview.as_ref() else {
                continue;
            };
            if let Some(found) = find_player_view_in_subtree(subview, identifier) {
                return Some(found);
            }
        }
    }

    None
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn remove_player_view_from_subtree(root_view: &AnyObject, identifier: &NSString) -> bool {
    if let Some(view) = find_player_view_in_subtree(root_view, identifier) {
        unsafe {
            let _: () = msg_send![view, removeFromSuperview];
        }
        return true;
    }

    false
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn remove_player_view_by_identifier(identifier: &str) -> bool {
    let identifier = NSString::from_str(identifier);

    unsafe {
        let application_class = match get_class(c"NSApplication") {
            Ok(class) => class,
            Err(_) => return false,
        };
        let application: *mut AnyObject = msg_send![application_class, sharedApplication];
        let Some(application) = application.as_ref() else {
            return false;
        };
        let windows: *mut AnyObject = msg_send![application, windows];
        let Some(windows) = windows.as_ref() else {
            return false;
        };
        let count: usize = msg_send![windows, count];
        for index in 0..count {
            let window: *mut AnyObject = msg_send![windows, objectAtIndex: index];
            let Some(window) = window.as_ref() else {
                continue;
            };
            let Some(content_view) = host_content_view(window) else {
                continue;
            };
            if remove_player_view_from_subtree(content_view, &identifier) {
                return true;
            }
        }
    }

    false
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn convert_rect_to_host_view(
    webview_view: &AnyObject,
    content_view: &AnyObject,
    rect: MacOSNativeRect,
) -> NSRect {
    let bounds: NSRect = unsafe { msg_send![webview_view, bounds] };
    let flipped: bool = unsafe { msg_send![webview_view, isFlipped] };
    let local_y = if flipped {
        rect.y
    } else {
        bounds.size.height - rect.y - rect.height
    };

    let local_rect = NSRect::new(
        NSPoint::new(rect.x, local_y),
        NSSize::new(rect.width, rect.height),
    );

    unsafe { msg_send![webview_view, convertRect: local_rect, toView: content_view] }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn convert_rect_to_screen(
    webview_view: &AnyObject,
    ns_window: &AnyObject,
    rect: MacOSNativeRect,
) -> NSRect {
    let bounds: NSRect = unsafe { msg_send![webview_view, bounds] };
    let flipped: bool = unsafe { msg_send![webview_view, isFlipped] };
    let local_y = if flipped {
        rect.y
    } else {
        bounds.size.height - rect.y - rect.height
    };
    let local_rect = NSRect::new(
        NSPoint::new(rect.x, local_y),
        NSSize::new(rect.width, rect.height),
    );
    let nil_view: *mut AnyObject = std::ptr::null_mut();
    let window_rect: NSRect =
        unsafe { msg_send![webview_view, convertRect: local_rect, toView: nil_view] };

    unsafe { msg_send![ns_window, convertRectToScreen: window_rect] }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn window_local_rect(width: f64, height: f64) -> NSRect {
    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height))
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn nsrect_matches(a: NSRect, b: NSRect) -> bool {
    const EPSILON: f64 = 0.5;
    (a.origin.x - b.origin.x).abs() <= EPSILON
        && (a.origin.y - b.origin.y).abs() <= EPSILON
        && (a.size.width - b.size.width).abs() <= EPSILON
        && (a.size.height - b.size.height).abs() <= EPSILON
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn make_native_url(source: &str) -> Result<Retained<NSURL>, String> {
    if url::Url::parse(source).is_ok() {
        let ns_source = NSString::from_str(source);
        unsafe { NSURL::URLWithString(&ns_source) }
            .ok_or_else(|| format!("Failed to parse media URL: {source}"))
    } else {
        NSURL::from_file_path(Path::new(source))
            .ok_or_else(|| format!("Failed to create file URL for: {source}"))
    }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn get_class(name: &std::ffi::CStr) -> Result<&'static AnyClass, String> {
    AnyClass::get(name).ok_or_else(|| {
        format!(
            "Objective-C class {} is unavailable.",
            name.to_string_lossy()
        )
    })
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn as_object<'a>(ptr: *mut c_void, label: &str) -> Result<&'a AnyObject, String> {
    unsafe { (ptr as *mut AnyObject).as_ref() }
        .ok_or_else(|| format!("Failed to access native {label} handle."))
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn cmtime_to_seconds(time: CMTime) -> f64 {
    if time.timescale <= 0 || (time.flags & CMTIME_FLAGS_VALID) == 0 {
        return 0.0;
    }

    (time.value as f64 / time.timescale as f64).max(0.0)
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn seconds_to_cmtime(seconds: f64) -> CMTime {
    let clamped = seconds.max(0.0);
    CMTime {
        value: (clamped * 600.0).round() as i64,
        timescale: 600,
        flags: CMTIME_FLAGS_VALID,
        epoch: 0,
    }
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn catch_objc<R>(context: &str, f: impl FnOnce() -> R) -> Result<R, String> {
    catch_objc_exception(AssertUnwindSafe(f)).map_err(|exception| match exception {
        Some(exception) => format!("{context} {exception}"),
        None => format!("{context} Unknown Objective-C exception."),
    })
}

#[cfg(all(feature = "gui", target_os = "macos"))]
fn catch_objc_result<R>(context: &str, f: impl FnOnce() -> Result<R, String>) -> Result<R, String> {
    catch_objc_exception(AssertUnwindSafe(f)).map_err(|exception| match exception {
        Some(exception) => format!("{context} {exception}"),
        None => format!("{context} Unknown Objective-C exception."),
    })?
}
