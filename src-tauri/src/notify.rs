//! Threshold and reset notifications. State persists so a restart doesn't
//! repeat notifications for the same window.

use crate::AppState;
use crate::providers::{ProviderSnapshot, WindowState};
use crate::settings::Settings;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

#[derive(Default, Serialize, Deserialize)]
pub struct NotifyState {
    /// "{provider}:{window}:{reset bucket}" → highest threshold already announced.
    fired: BTreeMap<String, u8>,
    /// "{provider}:{window}" → reset time we'll announce once it passes.
    pending_resets: BTreeMap<String, i64>,
}

impl NotifyState {
    pub fn load(path: &std::path::Path) -> Self {
        fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
}

pub fn fmt_duration(secs: i64) -> String {
    let secs = secs.max(0);
    let (d, h, m) = (secs / 86_400, (secs % 86_400) / 3600, (secs % 3600) / 60);
    if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m")
    } else {
        "under a minute".into()
    }
}

fn send(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

pub fn check(app: &AppHandle, settings: &Settings, snaps: &[ProviderSnapshot], now_ms: i64) {
    let st = app.state::<AppState>();
    let now = now_ms / 1000;
    let mut ns = st.notify_state.lock().unwrap();
    let mut changed = false;

    for p in snaps {
        if !settings.visible_providers.get(&p.id).copied().unwrap_or(true) {
            continue;
        }
        for w in &p.windows {
            let key = format!("{}:{}", p.id, w.id);

            if let Some(r) = w.resets_at.filter(|r| *r > now) {
                if w.used_percent.is_some_and(|u| u > 0.0) && ns.pending_resets.get(&key) != Some(&r) {
                    ns.pending_resets.insert(key.clone(), r);
                    changed = true;
                }
            }

            if !settings.notify_thresholds || w.state != WindowState::Live {
                continue;
            }
            let Some(used) = w.used_percent else { continue };
            // Highest threshold crossed; announce only that one.
            let Some(&t) = settings.thresholds.iter().filter(|t| used >= **t as f64).max() else {
                continue;
            };
            // Bucket the reset time so small jitter in resets_at doesn't re-arm it.
            let bucket = w.resets_at.map(|r| r / 600).unwrap_or(0);
            let fkey = format!("{key}:{bucket}");
            if ns.fired.get(&fkey).copied().unwrap_or(0) >= t {
                continue;
            }
            ns.fired.insert(fkey, t);
            changed = true;
            let when = w
                .resets_at
                .map(|r| format!("Resets in {}.", fmt_duration(r - now)))
                .or_else(|| w.reset_text.clone())
                .unwrap_or_default();
            let headline = if used >= 100.0 { "limit reached".to_string() } else { format!("{used:.0}% used") };
            send(app, &format!("{} · {} {}", p.name, w.label, headline), &when);
        }
    }

    let due: Vec<(String, i64)> = ns
        .pending_resets
        .iter()
        .filter(|(_, r)| **r <= now)
        .map(|(k, r)| (k.clone(), *r))
        .collect();
    for (key, r) in due {
        ns.pending_resets.remove(&key);
        changed = true;
        // Skip resets that happened long ago (e.g. while the PC was off).
        if settings.notify_on_reset && now - r < 15 * 60 {
            let (pid, wid) = key.split_once(':').unwrap_or((&key, ""));
            let found = snaps
                .iter()
                .find(|p| p.id == pid)
                .and_then(|p| p.windows.iter().find(|w| w.id == wid).map(|w| (p.name.clone(), w.label.clone())));
            if let Some((pname, wlabel)) = found {
                send(app, &format!("{pname} · {wlabel} has reset"), "Your allowance for this window is renewed.");
            }
        }
    }

    // Forget announcements for windows that ended more than 8 days ago.
    let cutoff = (now - 8 * 86_400) / 600;
    let before = ns.fired.len();
    ns.fired.retain(|k, _| {
        k.rsplit(':').next().and_then(|b| b.parse::<i64>().ok()).is_none_or(|b| b == 0 || b > cutoff)
    });
    changed |= ns.fired.len() != before;

    if changed {
        if let Ok(bytes) = serde_json::to_vec(&*ns) {
            let _ = fs::write(&st.notify_state_path, bytes);
        }
    }
}
