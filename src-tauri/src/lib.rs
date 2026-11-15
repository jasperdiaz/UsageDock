mod commands;
mod integrations;
mod notify;
mod providers;
mod settings;
mod sync;
mod tray;
mod windows;

use settings::Settings;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};

pub struct AppState {
    pub settings: Mutex<Settings>,
    settings_path: PathBuf,
    settings_dirty: AtomicBool,
    pub snapshots: Mutex<Vec<providers::ProviderSnapshot>>,
    pub integrations: Mutex<integrations::IntegrationStatus>,
    pub paths: integrations::Paths,
    pub codex_cache: Mutex<providers::CodexCache>,
    pub notify_state: Mutex<notify::NotifyState>,
    pub notify_state_path: PathBuf,
    pub quitting: AtomicBool,
}

impl AppState {
    pub fn mark_dirty(&self) {
        self.settings_dirty.store(true, Ordering::SeqCst);
    }

    pub fn save_now(&self) {
        self.settings_dirty.store(false, Ordering::SeqCst);
        let s = self.settings.lock().unwrap().clone();
        if let Err(e) = s.save(&self.settings_path) {
            eprintln!("UsageDock: couldn't save settings: {e}");
        }
    }

    /// Window moves arrive in bursts; the sync thread flushes them every tick.
    pub fn save_if_dirty(&self) {
        if self.settings_dirty.load(Ordering::SeqCst) {
            self.save_now();
        }
    }
}

pub fn quit(app: &AppHandle) {
    let st = app.state::<AppState>();
    st.quitting.store(true, Ordering::SeqCst);
    st.save_now();
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be first: a second launch just brings the running instance forward.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let st = app.state::<AppState>();
            let compact = st.settings.lock().unwrap().start_compact;
            windows::show(app, if compact { windows::WIDGET } else { windows::DASHBOARD });
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let config_dir = app.path().app_config_dir()?;
            let home = app.path().home_dir()?;
            let data = integrations::data_dir();
            std::fs::create_dir_all(data.join("sources"))?;

            let settings_path = config_dir.join("settings.json");
            let settings = Settings::load(&settings_path);
            let bundled_extension = app
                .path()
                .resolve("extension", tauri::path::BaseDirectory::Resource)
                .ok();
            let paths = integrations::Paths {
                home,
                data: data.clone(),
                bridge: integrations::bridge_path(),
                bundled_extension,
            };
            let notify_state_path = data.join("notify-state.json");
            app.manage(AppState {
                settings: Mutex::new(settings.clone()),
                settings_path,
                settings_dirty: AtomicBool::new(false),
                snapshots: Mutex::new(Vec::new()),
                integrations: Mutex::new(integrations::status(&paths)),
                paths,
                codex_cache: Mutex::new(Default::default()),
                notify_state: Mutex::new(notify::NotifyState::load(&notify_state_path)),
                notify_state_path,
                quitting: AtomicBool::new(false),
            });

            commands::sync_autostart(&handle, settings.start_with_windows);
            windows::create_all(&handle)?;
            tray::create(&handle)?;

            let first = if settings.start_compact { windows::WIDGET } else { windows::DASHBOARD };
            windows::show(&handle, first);
            sync::spawn(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::update_settings,
            commands::refresh,
            commands::show_dashboard,
            commands::show_widget,
            commands::hide_window,
            commands::set_widget_collapsed,
            commands::fit_widget,
            commands::finish_launch,
            commands::set_dock,
            commands::reset_window_positions,
            commands::connect_claude_code,
            commands::disconnect_claude_code,
            commands::register_browser_host,
            commands::unregister_browser_host,
            commands::open_url,
            commands::open_folder,
            commands::quit,
        ])
        .build(tauri::generate_context!())
        .expect("error while building UsageDock")
        .run(|app, event| {
            // Keep running in the tray when every window is hidden.
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                let quitting = app.state::<AppState>().quitting.load(Ordering::SeqCst);
                if code.is_none() && !quitting {
                    api.prevent_exit();
                }
            }
        });
}
