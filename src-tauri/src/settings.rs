use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub start_with_windows: bool,
    pub show_in_tray: bool,
    pub start_compact: bool,
    pub always_on_top: bool,
    /// How often local sources are re-scanned, in seconds.
    pub refresh_interval_secs: u64,
    pub notify_thresholds: bool,
    pub thresholds: Vec<u8>,
    pub notify_on_reset: bool,
    pub visible_providers: BTreeMap<String, bool>,
    /// 0.4..=1.0, applied to the widget background.
    pub opacity: f64,
    pub show_remaining: bool,
    pub demo_mode: bool,
    pub widget_collapsed: bool,
    /// Provider shown in the widget, or "all" for the combined view.
    pub widget_provider: String,
    /// Data older than this (minutes) is flagged as stale.
    pub stale_after_mins: u64,
    /// Where the widget sits: a screen anchor from `DOCKS`, or "free".
    pub dock: String,
    /// "auto" (strip when docked top/bottom centre, else stacked), "stacked" or "strip".
    pub widget_layout: String,
    /// Show provider names next to their logos in the widget; off = logo only.
    pub widget_show_names: bool,
    /// Subtle halo and lit edge around the widget panel.
    pub widget_glow: bool,
    /// Glow strength, 0.1..=1.0 (Subtle 0.35, Medium 0.65, Strong 1.0).
    pub widget_glow_level: f64,
    pub widget_position: Option<(i32, i32)>,
    pub widget_size: Option<(f64, f64)>,
    pub dashboard_position: Option<(i32, i32)>,
}

pub const DOCKS: [&str; 8] = [
    "top-left", "top-center", "top-right", "left-center",
    "right-center", "bottom-left", "bottom-center", "bottom-right",
];

impl Default for Settings {
    fn default() -> Self {
        let visible_providers = [("claude", true), ("chatgpt", true), ("codex", true)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        Self {
            start_with_windows: false,
            show_in_tray: true,
            start_compact: true,
            always_on_top: true,
            refresh_interval_secs: 60,
            notify_thresholds: true,
            thresholds: vec![80, 95, 100],
            notify_on_reset: false,
            visible_providers,
            opacity: 0.96,
            show_remaining: false,
            demo_mode: false,
            widget_collapsed: false,
            widget_provider: "claude".into(),
            stale_after_mins: 30,
            dock: "bottom-right".into(),
            widget_layout: "auto".into(),
            widget_show_names: true,
            widget_glow: true,
            widget_glow_level: 0.35,
            widget_position: None,
            widget_size: None,
            dashboard_position: None,
        }
    }
}

impl Settings {
    /// Clamp values that come from the UI or a hand-edited file.
    pub fn normalized(mut self) -> Self {
        self.refresh_interval_secs = self.refresh_interval_secs.clamp(15, 3600);
        self.opacity = if self.opacity.is_finite() { self.opacity.clamp(0.4, 1.0) } else { 0.96 };
        self.stale_after_mins = self.stale_after_mins.clamp(5, 24 * 60);
        self.widget_glow_level = if self.widget_glow_level.is_finite() { self.widget_glow_level.clamp(0.1, 1.0) } else { 0.35 };
        self.thresholds.retain(|t| (1..=100).contains(t));
        self.thresholds.sort_unstable();
        self.thresholds.dedup();
        for p in ["claude", "chatgpt", "codex"] {
            self.visible_providers.entry(p.into()).or_insert(true);
        }
        if self.dock != "free" && !DOCKS.contains(&self.dock.as_str()) {
            self.dock = "bottom-right".into();
        }
        if !["auto", "stacked", "strip"].contains(&self.widget_layout.as_str()) {
            self.widget_layout = "auto".into();
        }
        if self.widget_provider != "all" && !self.visible_providers.contains_key(&self.widget_provider) {
            self.widget_provider = "claude".into();
        }
        self
    }

    pub fn load(path: &Path) -> Self {
        fs::read(path)
            .ok()
            // Tolerate a UTF-8 BOM from editors that add one (e.g. PowerShell 5).
            .and_then(|b| serde_json::from_slice::<Settings>(b.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&b)).ok())
            .unwrap_or_default()
            .normalized()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp: PathBuf = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        fs::rename(tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_normalize() {
        let dir = std::env::temp_dir().join(format!("usagedock-settings-{}", std::process::id()));
        let path = dir.join("settings.json");
        let mut s = Settings::default();
        s.always_on_top = false;
        s.opacity = 0.7;
        s.widget_position = Some((10, 20));
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);

        fs::write(&path, r#"{"opacity": 5, "refreshIntervalSecs": 1, "thresholds": [100, 0, 80, 80]}"#).unwrap();
        let n = Settings::load(&path);
        assert_eq!(n.opacity, 1.0);
        assert_eq!(n.refresh_interval_secs, 15);
        assert_eq!(n.thresholds, vec![80, 100]);
        assert!(n.show_in_tray, "missing keys fall back to defaults");

        fs::write(&path, "garbage").unwrap();
        assert_eq!(Settings::load(&path), Settings::default().normalized());
    }
}
