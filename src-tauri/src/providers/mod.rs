//! Provider adapters.
//!
//! Every provider turns one or more *local* sources into a [`ProviderSnapshot`].
//! Nothing here talks to the network or touches credentials: sources are files
//! written by the bridge (Claude Code status line, Chrome extension) or logs a
//! CLI already keeps on disk (Codex). To add a provider, write a module with a
//! `snapshot(&Ctx) -> ProviderSnapshot` function and list it in [`collect`].

mod chatgpt;
mod claude;
mod codex;
mod demo;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WindowState {
    /// Fresh data from a connected source.
    Live,
    /// Older than the staleness threshold; still shown with its timestamp.
    Stale,
    /// The verified reset time passed after this data was captured, so the
    /// old percentage no longer applies and nothing newer has arrived yet.
    Reset,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub id: String,
    pub label: String,
    pub short_label: String,
    pub used_percent: Option<f64>,
    /// Unix epoch seconds.
    pub resets_at: Option<i64>,
    /// Reset wording copied from the page when no exact time was available.
    pub reset_text: Option<String>,
    pub limit_reached: bool,
    pub source: String,
    /// Unix epoch milliseconds.
    pub captured_at: i64,
    pub state: WindowState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub id: String,
    pub label: String,
    pub connected: bool,
    pub last_seen: Option<i64>,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderStatus {
    Live,
    Stale,
    Waiting,
    Unavailable,
    Demo,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSnapshot {
    pub id: String,
    pub name: String,
    pub usage_url: String,
    pub status: ProviderStatus,
    pub detail: String,
    pub plan: Option<String>,
    pub windows: Vec<UsageWindow>,
    pub last_updated: Option<i64>,
    pub sources: Vec<SourceInfo>,
}

/// What the providers need to know about the machine and settings.
pub struct Ctx<'a> {
    pub sources_dir: &'a Path,
    pub home_dir: &'a Path,
    pub stale_after_ms: i64,
    pub now_ms: i64,
    pub claude_statusline_installed: bool,
    pub chrome_host_registered: bool,
    pub codex_cache: &'a Mutex<codex::Cache>,
}

pub use codex::Cache as CodexCache;

pub fn collect(ctx: &Ctx, demo_mode: bool) -> Vec<ProviderSnapshot> {
    if demo_mode {
        return demo::snapshots(ctx.now_ms);
    }
    vec![claude::snapshot(ctx), chatgpt::snapshot(ctx), codex::snapshot(ctx)]
}

// ---------------------------------------------------------------------------
// Shared source-file format (written by usagedock-bridge)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct RawWindow {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub used_percent: Option<f64>,
    #[serde(default)]
    pub resets_at: Option<i64>,
    #[serde(default)]
    pub reset_text: Option<String>,
    #[serde(default)]
    pub limit_reached: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceDoc {
    pub captured_at: i64,
    #[serde(default)]
    pub plan: Option<String>,
    #[serde(default)]
    pub windows: Vec<RawWindow>,
}

pub fn read_source(dir: &Path, name: &str) -> Option<SourceDoc> {
    let bytes = fs::read(dir.join(name)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Build a display window and decide whether its data is still valid.
pub fn finish_window(
    ctx: &Ctx,
    raw: &RawWindow,
    label: String,
    short_label: String,
    source: &str,
    captured_at: i64,
) -> UsageWindow {
    let mut state = WindowState::Live;
    let mut used = raw.used_percent.map(|p| p.clamp(0.0, 100.0));
    let mut limit_reached = raw.limit_reached || used.is_some_and(|p| p >= 100.0);
    if let Some(reset) = raw.resets_at {
        if reset * 1000 <= ctx.now_ms && captured_at < reset * 1000 {
            state = WindowState::Reset;
            used = None;
            limit_reached = false;
        }
    }
    if state == WindowState::Live && ctx.now_ms - captured_at > ctx.stale_after_ms {
        state = WindowState::Stale;
    }
    UsageWindow {
        id: raw.id.clone(),
        label,
        short_label,
        used_percent: used,
        resets_at: raw.resets_at,
        reset_text: if raw.resets_at.is_some() { None } else { raw.reset_text.clone() },
        limit_reached,
        source: source.to_string(),
        captured_at,
        state,
    }
}

/// Overall status from the windows we ended up with.
pub fn status_for(windows: &[UsageWindow]) -> ProviderStatus {
    if windows.is_empty() {
        ProviderStatus::Waiting
    } else if windows.iter().any(|w| w.state == WindowState::Live) {
        ProviderStatus::Live
    } else {
        ProviderStatus::Stale
    }
}

pub fn home_path(home: &Path, parts: &[&str]) -> PathBuf {
    parts.iter().fold(home.to_path_buf(), |p, s| p.join(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx<'a>(cache: &'a Mutex<CodexCache>, now_ms: i64) -> Ctx<'a> {
        Ctx {
            sources_dir: Path::new("."),
            home_dir: Path::new("."),
            stale_after_ms: 30 * 60_000,
            now_ms,
            claude_statusline_installed: true,
            chrome_host_registered: false,
            codex_cache: cache,
        }
    }

    fn raw(used: f64, resets_at: i64) -> RawWindow {
        RawWindow { id: "five_hour".into(), label: None, used_percent: Some(used), resets_at: Some(resets_at), reset_text: None, limit_reached: false }
    }

    #[test]
    fn live_window_keeps_percentage() {
        let cache = Mutex::new(CodexCache::default());
        let now = 1_000_000_000_000;
        let w = finish_window(&ctx(&cache, now), &raw(42.0, now / 1000 + 3600), "a".into(), "a".into(), "t", now - 60_000);
        assert_eq!(w.state, WindowState::Live);
        assert_eq!(w.used_percent, Some(42.0));
    }

    #[test]
    fn passed_reset_drops_old_percentage() {
        let cache = Mutex::new(CodexCache::default());
        let now = 1_000_000_000_000;
        // Captured two hours ago; window reset an hour ago.
        let w = finish_window(&ctx(&cache, now), &raw(90.0, now / 1000 - 3600), "a".into(), "a".into(), "t", now - 7_200_000);
        assert_eq!(w.state, WindowState::Reset);
        assert_eq!(w.used_percent, None);
        assert!(!w.limit_reached);
    }

    #[test]
    fn old_capture_is_stale() {
        let cache = Mutex::new(CodexCache::default());
        let now = 1_000_000_000_000;
        let w = finish_window(&ctx(&cache, now), &raw(10.0, now / 1000 + 86_400), "a".into(), "a".into(), "t", now - 3_600_000);
        assert_eq!(w.state, WindowState::Stale);
        assert_eq!(w.used_percent, Some(10.0));
    }

    #[test]
    fn demo_is_always_labelled() {
        assert!(demo::snapshots(1_000_000_000_000).iter().all(|p| p.status == ProviderStatus::Demo));
    }
}
