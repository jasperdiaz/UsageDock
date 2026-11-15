//! Demo mode: fixed sample values for UI testing. Every snapshot carries the
//! `Demo` status and the UI labels it as such, so sample values are never shown
//! as live data.

use super::*;

fn w(id: &str, label: &str, short: &str, used: f64, resets_in_s: i64, now_ms: i64) -> UsageWindow {
    UsageWindow {
        id: id.into(),
        label: label.into(),
        short_label: short.into(),
        used_percent: Some(used),
        resets_at: Some(now_ms / 1000 + resets_in_s),
        reset_text: None,
        limit_reached: used >= 100.0,
        source: "Demo".into(),
        captured_at: now_ms,
        state: WindowState::Live,
    }
}

fn snap(id: &str, name: &str, url: &str, plan: &str, windows: Vec<UsageWindow>, now_ms: i64) -> ProviderSnapshot {
    ProviderSnapshot {
        id: id.into(),
        name: name.into(),
        usage_url: url.into(),
        status: ProviderStatus::Demo,
        detail: "Demo data. Not your real usage.".into(),
        plan: Some(plan.into()),
        windows,
        last_updated: Some(now_ms),
        sources: vec![SourceInfo {
            id: "demo".into(),
            label: "Demo mode".into(),
            connected: true,
            last_seen: Some(now_ms),
            detail: "Sample values for UI testing. Turn off in Settings.".into(),
        }],
    }
}

pub fn snapshots(now: i64) -> Vec<ProviderSnapshot> {
    // Anchor resets to the hour so countdowns move but values stay put.
    let hour = 3600;
    let into_hour = (now / 1000) % hour;
    vec![
        snap(
            "claude",
            "Claude",
            "https://claude.ai/settings/usage",
            "Max",
            vec![
                w("five_hour", "Current session", "Session", 50.0, 2 * hour - into_hour + 22 * 60, now),
                w("seven_day", "Weekly · all models", "Weekly", 11.0, 6 * 24 * hour + 8 * hour - into_hour, now),
                w("seven_day_sonnet", "Weekly · Sonnet", "Sonnet", 4.0, 6 * 24 * hour + 8 * hour - into_hour, now),
            ],
            now,
        ),
        snap(
            "chatgpt",
            "ChatGPT",
            "https://chatgpt.com/",
            "Plus",
            vec![w("limit_gpt", "Thinking model limit", "Thinking", 100.0, 3 * hour - into_hour, now)],
            now,
        ),
        snap(
            "codex",
            "Codex",
            "https://chatgpt.com/codex/settings/usage",
            "Plus",
            vec![
                w("five_hour", "5-hour limit", "5h", 82.0, hour - into_hour + 40 * 60, now),
                w("weekly", "Weekly limit", "Weekly", 37.0, 3 * 24 * hour - into_hour, now),
            ],
            now,
        ),
    ]
}
