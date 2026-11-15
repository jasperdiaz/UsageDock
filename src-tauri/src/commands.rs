use crate::integrations::{self, IntegrationStatus};
use crate::settings::Settings;
use crate::sync::{self, UsagePayload};
use crate::{AppState, tray, windows};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FullState {
    settings: Settings,
    usage: UsagePayload,
    version: String,
}

#[tauri::command]
pub fn get_state(app: AppHandle) -> FullState {
    let st = app.state::<AppState>();
    FullState {
        settings: st.settings.lock().unwrap().clone(),
        usage: sync::payload(&app),
        version: app.package_info().version.to_string(),
    }
}

/// Apply new settings plus their side effects (window flags, tray, autostart).
/// Window geometry is owned by the backend, so the incoming copy's is ignored.
pub fn apply_settings(app: &AppHandle, incoming: Settings) -> Settings {
    let st = app.state::<AppState>();
    let (prev, next) = {
        let mut s = st.settings.lock().unwrap();
        let prev = s.clone();
        let mut next = incoming.normalized();
        next.widget_position = prev.widget_position;
        next.widget_size = prev.widget_size;
        next.dashboard_position = prev.dashboard_position;
        next.widget_collapsed = prev.widget_collapsed;
        *s = next.clone();
        (prev, next)
    };
    st.save_now();

    if prev.always_on_top != next.always_on_top || prev.show_in_tray != next.show_in_tray {
        windows::apply_window_flags(app);
    }
    if prev.dock != next.dock {
        windows::apply_dock(app);
    }
    if prev.show_in_tray != next.show_in_tray {
        tray::set_visible(app, next.show_in_tray);
    }
    if prev.start_with_windows != next.start_with_windows {
        sync_autostart(app, next.start_with_windows);
    }
    tray::sync_menu(app);
    let _ = app.emit("settings://changed", &next);

    let affects_data = prev.demo_mode != next.demo_mode
        || prev.visible_providers != next.visible_providers
        || prev.stale_after_mins != next.stale_after_mins
        || prev.thresholds != next.thresholds;
    if affects_data {
        sync::recompute(app);
    }
    next
}

pub fn sync_autostart(app: &AppHandle, enabled: bool) {
    let al = app.autolaunch();
    let current = al.is_enabled().unwrap_or(false);
    if enabled && !current {
        let _ = al.enable();
    } else if !enabled && current {
        let _ = al.disable();
    }
}

#[tauri::command]
pub fn update_settings(app: AppHandle, settings: Settings) -> Settings {
    apply_settings(&app, settings)
}

#[tauri::command]
pub fn refresh(app: AppHandle) -> UsagePayload {
    sync::recompute(&app);
    sync::payload(&app)
}

/// Async: on Windows, creating a window from a sync command can deadlock,
/// and this may create the dashboard on first use.
#[tauri::command]
pub async fn show_dashboard(app: AppHandle, tab: Option<String>) {
    windows::show_dashboard(&app, tab.as_deref());
}

#[tauri::command]
pub fn show_widget(app: AppHandle) {
    windows::show(&app, windows::WIDGET);
}

#[tauri::command]
pub fn hide_window(app: AppHandle, label: String) {
    if label == windows::WIDGET || label == windows::DASHBOARD {
        windows::hide_or_quit(&app, &label);
    }
}

#[tauri::command]
pub fn set_widget_collapsed(app: AppHandle, collapsed: bool) {
    windows::set_widget_collapsed(&app, collapsed);
    let st = app.state::<AppState>();
    let s = st.settings.lock().unwrap().clone();
    let _ = app.emit("settings://changed", &s);
}

/// The widget measured its content; size the window to it and re-anchor.
#[tauri::command]
pub fn fit_widget(app: AppHandle, width: f64, height: f64, inset: Option<f64>) {
    windows::fit_widget(&app, width, height, inset.unwrap_or(0.0));
}

/// The widget's splash is done: glide it from the centre to its spot.
#[tauri::command]
pub fn finish_launch(app: AppHandle, animate: Option<bool>) {
    windows::finish_launch(&app, animate.unwrap_or(true));
}

#[tauri::command]
pub fn set_dock(app: AppHandle, dock: String) {
    windows::set_dock(&app, &dock);
}

#[tauri::command]
pub fn reset_window_positions(app: AppHandle) {
    windows::reset_positions(&app);
    let st = app.state::<AppState>();
    st.save_now();
    let s = st.settings.lock().unwrap().clone();
    let _ = app.emit("settings://changed", &s);
}

fn after_integration(app: &AppHandle, r: Result<(), String>) -> Result<IntegrationStatus, String> {
    r?;
    sync::recompute(app);
    let st = app.state::<AppState>();
    Ok(st.integrations.lock().unwrap().clone())
}

#[tauri::command]
pub fn connect_claude_code(app: AppHandle) -> Result<IntegrationStatus, String> {
    let r = integrations::connect_claude_code(&app.state::<AppState>().paths);
    after_integration(&app, r)
}

#[tauri::command]
pub fn disconnect_claude_code(app: AppHandle) -> Result<IntegrationStatus, String> {
    let r = integrations::disconnect_claude_code(&app.state::<AppState>().paths);
    after_integration(&app, r)
}

#[tauri::command]
pub fn register_browser_host(app: AppHandle) -> Result<IntegrationStatus, String> {
    let r = integrations::register_chrome_host(&app.state::<AppState>().paths);
    after_integration(&app, r)
}

#[tauri::command]
pub fn unregister_browser_host(app: AppHandle) -> Result<IntegrationStatus, String> {
    let r = integrations::unregister_chrome_host(&app.state::<AppState>().paths);
    after_integration(&app, r)
}

const ALLOWED_URL_PREFIXES: [&str; 5] = [
    "https://claude.ai/",
    "https://chatgpt.com/",
    "https://code.claude.com/",
    "https://support.claude.com/",
    "https://help.openai.com/",
];

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    if !ALLOWED_URL_PREFIXES.iter().any(|p| url.starts_with(p)) {
        return Err("URL not allowed".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_folder(app: AppHandle, which: String) -> Result<(), String> {
    let st = app.state::<AppState>();
    let dir = match which.as_str() {
        "extension" => st.paths.extension_dir(),
        "sources" => st.paths.sources(),
        _ => return Err("unknown folder".into()),
    };
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    crate::quit(&app);
}
