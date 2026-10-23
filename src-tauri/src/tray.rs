use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Wry};

use crate::locale::labels;
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
    check_update: MenuItem<Wry>,
    idle_icon: Image<'static>,
    running_icon: Image<'static>,
}

impl Tray {
    pub fn new(app: &AppHandle, status: &Status) -> tauri::Result<Self> {
        let labels = labels();
        let toggle = MenuItem::with_id(app, "toggle", &labels.start, true, None::<&str>)?;
        let system_proxy = CheckMenuItem::with_id(
            app,
            "system_proxy",
            &labels.system_proxy,
            true,
            status.config.use_as_system_proxy,
            None::<&str>,
        )?;
        let check_update =
            MenuItem::with_id(app, "check_update", &labels.check_update, true, None::<&str>)?;
        let quit = MenuItem::with_id(app, "quit", &labels.quit, true, None::<&str>)?;
        let menu = Menu::with_items(
            app,
            &[&toggle, &system_proxy, &PredefinedMenuItem::separator(app)?, &check_update, &quit],
        )?;

        let idle_icon = Image::from_bytes(IDLE_ICON)?;
        let running_icon = Image::from_bytes(RUNNING_ICON)?;
        TrayIconBuilder::with_id("main")
            .icon(idle_icon.clone())
            .icon_as_template(ICON_IS_TEMPLATE)
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_menu_event(|app, event| match event.id().as_ref() {
                "toggle" => crate::commands::spawn_toggle_relay(app.clone()),
                "system_proxy" => crate::commands::spawn_toggle_system_proxy(app.clone()),
                "check_update" => crate::update::spawn_manual_update_check(app.clone()),
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
                    crate::window::show_main_window(tray.app_handle());
                }
            })
            .build(app)?;

        let tray = Self { toggle, system_proxy, check_update, idle_icon, running_icon };
        tray.sync(app, status);
        Ok(tray)
    }

    pub fn set_checking_update(&self, checking: bool) {
        let labels = labels();
        let _ = self.check_update.set_text(if checking {
            &labels.checking_update
        } else {
            &labels.check_update
        });
        let _ = self.check_update.set_enabled(!checking);
    }

    pub fn sync(&self, app: &AppHandle, status: &Status) {
        let labels = labels();
        let _ = self.toggle.set_text(if status.running { &labels.stop } else { &labels.start });
        let _ = self.system_proxy.set_checked(status.config.use_as_system_proxy);
        if let Some(tray) = app.tray_by_id("main") {
            let icon = if status.running { &self.running_icon } else { &self.idle_icon };
            let _ = tray.set_icon(Some(icon.clone()));
            let _ = tray.set_icon_as_template(ICON_IS_TEMPLATE);
            let _ = tray.set_tooltip(Some(if status.running {
                &labels.running
            } else {
                &labels.stopped
            }));
        }
    }
}
