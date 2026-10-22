use tauri::{AppHandle, Manager};

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        #[cfg(target_os = "macos")]
        publish_title_bar_metrics(&window);
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleBarMetrics {
    height: f64,
    controls_width: f64,
}

#[cfg(target_os = "macos")]
pub fn title_bar_metrics(window: &tauri::WebviewWindow) -> Option<TitleBarMetrics> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSWindow, NSWindowButton};

    MainThreadMarker::new()?;
    let ns_window = unsafe { &*window.ns_window().ok()?.cast::<NSWindow>() };
    if !ns_window.isVisible() {
        return None;
    }
    ns_window.display();
    let content = ns_window.contentView()?;
    let close = ns_window.standardWindowButton(NSWindowButton::CloseButton)?;
    let zoom = ns_window.standardWindowButton(NSWindowButton::ZoomButton)?;
    let close = close.convertRect_toView(close.bounds(), Some(&content));
    let zoom = zoom.convertRect_toView(zoom.bounds(), Some(&content));
    let center = close.origin.y + close.size.height / 2.0;
    let center_from_top =
        if content.isFlipped() { center } else { content.bounds().size.height - center };
    Some(TitleBarMetrics {
        height: center_from_top * 2.0,
        controls_width: zoom.origin.x + zoom.size.width,
    })
}

#[cfg(target_os = "macos")]
fn publish_title_bar_metrics(window: &tauri::WebviewWindow) {
    use tauri::Emitter;

    let target = window.clone();
    let _ = window.run_on_main_thread(move || {
        if let Some(metrics) = title_bar_metrics(&target) {
            let _ = target.emit("window://title-bar-metrics", metrics);
        }
    });
}

#[cfg(target_os = "macos")]
pub fn prepare_main_window(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn prepare_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        let content_size = window.inner_size()?;
        window.set_decorations(false)?;
        window.set_size(content_size)?;
        window.set_shadow(true)?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn apply_dock_visibility(app: &AppHandle, visible: bool) {
    let policy =
        if visible { tauri::ActivationPolicy::Regular } else { tauri::ActivationPolicy::Accessory };
    let _ = app.set_activation_policy(policy);
    #[cfg(debug_assertions)]
    if visible {
        let _ = app.run_on_main_thread(restore_dev_dock_icon);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn apply_dock_visibility(_app: &AppHandle, _visible: bool) {}

#[cfg(all(target_os = "macos", debug_assertions))]
fn restore_dev_dock_icon() {
    use objc2::{AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let data = NSData::with_bytes(include_bytes!("../icons/icon.png"));
    if let Some(icon) = NSImage::initWithData(NSImage::alloc(), &data) {
        unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&icon)) };
    }
}
