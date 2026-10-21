use tauri::{AppHandle, Manager};

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
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
