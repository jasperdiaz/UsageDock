use crate::integrations::{self, IntegrationStatus};
use crate::providers::{self, Ctx, ProviderSnapshot};
use crate::{AppState, notify, tray};
use serde::Serialize;
use std::fs;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime};
use tauri::{AppHandle, Emitter, Manager};

const TICK: Duration = Duration::from_secs(2);
/// Re-evaluate even without changes so resets and staleness show up on time.
const HOUSEKEEPING: Duration = Duration::from_secs(30);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsagePayload {
    pub snapshots: Vec<ProviderSnapshot>,
    pub integrations: IntegrationStatus,
    pub generated_at: i64,
}

pub fn payload(app: &AppHandle) -> UsagePayload {
    let st = app.state::<AppState>();
    UsagePayload {
        snapshots: st.snapshots.lock().unwrap().clone(),
        integrations: st.integrations.lock().unwrap().clone(),
        generated_at: providers::now_ms(),
    }
}

pub fn recompute(app: &AppHandle) {
    let st = app.state::<AppState>();
    let settings = st.settings.lock().unwrap().clone();
    let integ = integrations::status(&st.paths);
    let now = providers::now_ms();
    let sources_dir = st.paths.sources();
    let ctx = Ctx {
        sources_dir: &sources_dir,
        home_dir: &st.paths.home,
        stale_after_ms: settings.stale_after_mins as i64 * 60_000,
        now_ms: now,
        claude_statusline_installed: integ.claude_statusline_installed,
        chrome_host_registered: integ.chrome_host_registered,
        codex_cache: &st.codex_cache,
    };
    let snaps = providers::collect(&ctx, settings.demo_mode);

    if !settings.demo_mode {
        notify::check(app, &settings, &snaps, now);
    }
    let visible = |id: &str| settings.visible_providers.get(id).copied().unwrap_or(true);
    tray::update_tooltip(app, &snaps, &visible);

    *st.snapshots.lock().unwrap() = snaps;
    *st.integrations.lock().unwrap() = integ;
    let _ = app.emit("usage://updated", payload(app));
}

/// Cheap change detector: names + mtimes + sizes of files in the sources dir.
fn sources_signature(app: &AppHandle) -> Vec<(String, SystemTime, u64)> {
    let st = app.state::<AppState>();
    let mut sig: Vec<_> = fs::read_dir(st.paths.sources())
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| {
                    let m = e.metadata().ok()?;
                    Some((e.file_name().to_string_lossy().into_owned(), m.modified().ok()?, m.len()))
                })
                .collect()
        })
        .unwrap_or_default();
    sig.sort();
    sig
}

pub fn spawn(app: AppHandle) {
    std::thread::Builder::new()
        .name("usagedock-sync".into())
        .spawn(move || {
            let mut last_sig = sources_signature(&app);
            let mut last_full = Instant::now();
            let mut last_house = Instant::now();
            recompute(&app);
            loop {
                std::thread::sleep(TICK);
                let st = app.state::<AppState>();
                if st.quitting.load(Ordering::SeqCst) {
                    break;
                }
                let interval = Duration::from_secs(st.settings.lock().unwrap().refresh_interval_secs);

                let sig = sources_signature(&app);
                let changed = sig != last_sig;
                if changed {
                    last_sig = sig;
                }
                let due_full = last_full.elapsed() >= interval;
                let due_house = last_house.elapsed() >= HOUSEKEEPING;
                if changed || due_full || due_house {
                    recompute(&app);
                    last_house = Instant::now();
                    if due_full {
                        last_full = Instant::now();
                    }
                }
                st.save_if_dirty();
            }
        })
        .expect("spawn sync thread");
}
