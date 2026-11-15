use crate::providers::{ProviderSnapshot, ProviderStatus};
use crate::windows::{self, DASHBOARD, WIDGET};
use crate::{AppState, sync};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

pub const TRAY_ID: &str = "usagedock";

pub struct TrayMenu {
    pub compact: CheckMenuItem<Wry>,
    pub on_top: CheckMenuItem<Wry>,
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let st = app.state::<AppState>();
    let s = st.settings.lock().unwrap().clone();

    let show = MenuItem::with_id(app, "show", "Show dashboard", true, None::<&str>)?;
    let compact = CheckMenuItem::with_id(app, "compact", "Compact widget", true, false, None::<&str>)?;
    let on_top = CheckMenuItem::with_id(app, "on_top", "Always on top", true, s.always_on_top, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "Refresh", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit UsageDock", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &compact, &on_top, &sep1, &refresh, &settings, &sep2, &quit])?;

    app.manage(TrayMenu { compact, on_top });

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("UsageDock")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id().as_ref() {
            "show" => windows::show_dashboard(app, Some("usage")),
            "compact" => toggle_widget(app),
            "on_top" => {
                let st = app.state::<AppState>();
                let mut next = st.settings.lock().unwrap().clone();
                next.always_on_top = !next.always_on_top;
                crate::commands::apply_settings(app, next);
            }
            "refresh" => sync::recompute(app),
            "settings" => windows::show_dashboard(app, Some("settings")),
            "quit" => crate::quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = e
            {
                let app = tray.app_handle();
                if windows::is_visible(app, DASHBOARD) {
                    windows::hide_or_quit(app, DASHBOARD);
                } else if let Some(w) = windows::dashboard(app, None) {
                    let pos = rect.position.to_physical::<i32>(1.0);
                    let size = rect.size.to_physical::<u32>(1.0);
                    windows::place_near_tray(&w, pos, size);
                    windows::show_dashboard(app, None);
                }
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    let tray = builder.build(app)?;
    let _ = tray.set_visible(s.show_in_tray);
    Ok(())
}

fn tray(app: &AppHandle) -> Option<TrayIcon> {
    app.tray_by_id(TRAY_ID)
}

pub fn toggle_widget(app: &AppHandle) {
    if windows::is_visible(app, WIDGET) {
        windows::hide_or_quit(app, WIDGET);
    } else {
        windows::show(app, WIDGET);
    }
}

/// Keep the menu's check marks in step with reality.
pub fn sync_menu(app: &AppHandle) {
    let Some(m) = app.try_state::<TrayMenu>() else { return };
    let st = app.state::<AppState>();
    let on_top = st.settings.lock().unwrap().always_on_top;
    let _ = m.compact.set_checked(windows::is_visible(app, WIDGET));
    let _ = m.on_top.set_checked(on_top);
}

pub fn set_visible(app: &AppHandle, visible: bool) {
    if let Some(t) = tray(app) {
        let _ = t.set_visible(visible);
    }
}

/// Short tooltip, e.g. "Claude  Session 50% · Weekly 11%". Windows caps it at 127 chars.
pub fn update_tooltip(app: &AppHandle, snaps: &[ProviderSnapshot], visible: &dyn Fn(&str) -> bool) {
    let Some(t) = tray(app) else { return };
    let mut lines = vec!["UsageDock".to_string()];
    for p in snaps.iter().filter(|p| visible(&p.id)) {
        let parts: Vec<String> = p
            .windows
            .iter()
            .take(2)
            .filter_map(|w| Some(format!("{} {:.0}%", w.short_label, w.used_percent?)))
            .collect();
        let tag = if p.status == ProviderStatus::Demo { " (demo)" } else { "" };
        if !parts.is_empty() {
            lines.push(format!("{}{tag}: {}", p.name, parts.join(" · ")));
        }
    }
    let mut text = lines.join("\n");
    if text.len() > 127 {
        text = text.chars().take(124).collect::<String>() + "…";
    }
    let _ = t.set_tooltip(Some(text));
}
