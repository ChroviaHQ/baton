use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Wry};

use crate::relay::Status;

#[cfg(target_os = "macos")]
const IDLE_ICON: &[u8] = include_bytes!("../icons/tray-idle.png");
#[cfg(target_os = "macos")]
const RUNNING_ICON: &[u8] = include_bytes!("../icons/tray-running.png");
#[cfg(not(target_os = "macos"))]
const IDLE_ICON: &[u8] = include_bytes!("../icons/tray-idle-color.png");
#[cfg(not(target_os = "macos"))]
const RUNNING_ICON: &[u8] = include_bytes!("../icons/tray-running-color.png");
const ICON_IS_TEMPLATE: bool = cfg!(target_os = "macos");

pub struct Tray {
    toggle: MenuItem<Wry>,
    system_proxy: CheckMenuItem<Wry>,
    labels: &'static Labels,
}

struct Labels {
    start: &'static str,
    stop: &'static str,
    system_proxy: &'static str,
    quit: &'static str,
    running: &'static str,
    stopped: &'static str,
}

const ZH: Labels = Labels {
    start: "启动中继",
    stop: "停止中继",
    system_proxy: "设为系统代理",
    quit: "退出 Baton",
    running: "Baton · 运行中",
    stopped: "Baton · 已停止",
};

const EN: Labels = Labels {
    start: "Start Relay",
    stop: "Stop Relay",
    system_proxy: "Use as System Proxy",
    quit: "Quit Baton",
    running: "Baton · Running",
    stopped: "Baton · Stopped",
};

fn labels() -> &'static Labels {
    match sys_locale::get_locale() {
        Some(locale) if locale.starts_with("zh") => &ZH,
        _ => &EN,
    }
}

pub fn build(app: &AppHandle, status: &Status) -> tauri::Result<Tray> {
    let labels = labels();
    let toggle = MenuItem::with_id(app, "toggle", labels.start, true, None::<&str>)?;
    let system_proxy = CheckMenuItem::with_id(
        app,
        "system_proxy",
        labels.system_proxy,
        true,
        status.config.use_as_system_proxy,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", labels.quit, true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&toggle, &system_proxy, &PredefinedMenuItem::separator(app)?, &quit],
    )?;

    TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(IDLE_ICON)?)
        .icon_as_template(ICON_IS_TEMPLATE)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => crate::spawn_toggle_relay(app.clone()),
            "system_proxy" => crate::spawn_toggle_system_proxy(app.clone()),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    let tray = Tray { toggle, system_proxy, labels };
    tray.sync(app, status);
    Ok(tray)
}

impl Tray {
    pub fn sync(&self, app: &AppHandle, status: &Status) {
        let labels = self.labels;
        let _ = self.toggle.set_text(if status.running { labels.stop } else { labels.start });
        let _ = self.system_proxy.set_checked(status.config.use_as_system_proxy);
        if let Some(tray) = app.tray_by_id("main") {
            let icon = if status.running { RUNNING_ICON } else { IDLE_ICON };
            if let Ok(image) = Image::from_bytes(icon) {
                let _ = tray.set_icon(Some(image));
                let _ = tray.set_icon_as_template(ICON_IS_TEMPLATE);
            }
            let _ = tray.set_tooltip(Some(if status.running {
                labels.running
            } else {
                labels.stopped
            }));
        }
    }
}
