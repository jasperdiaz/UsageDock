//! Codex: the Codex CLI records the rate-limit status it receives with every
//! turn in its own session logs (`~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`).
//! We read only the tail of the newest log and only the `rate_limits` object.
//! `auth.json` and every other Codex file are left alone.

use super::*;
use serde_json::Value;
use std::io::{Read, Seek, SeekFrom};
use std::time::SystemTime;

const TAIL_BYTES: u64 = 512 * 1024;
const WEB_FILE: &str = "codex-web.json";

#[derive(Default)]
pub struct Cache {
    key: Option<(PathBuf, SystemTime, u64)>,
    doc: Option<SourceDoc>,
}

fn sorted_dirs(p: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(p)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect())
        .unwrap_or_default();
    v.sort();
    v
}

/// Newest few rollout files, newest first. Only the two most recent day
/// folders are listed, so this stays cheap however long the history gets.
///
/// The size is part of the cache key because on Windows the mtime stops
/// updating while Codex holds the rollout open and appends to it; keying on
/// mtime alone froze the cache on the first read of a long session.
fn recent_rollouts(sessions: &Path) -> Vec<(PathBuf, SystemTime, u64)> {
    let mut days = Vec::new();
    'outer: for y in sorted_dirs(sessions).into_iter().rev() {
        for m in sorted_dirs(&y).into_iter().rev() {
            for d in sorted_dirs(&m).into_iter().rev() {
                days.push(d);
                if days.len() == 2 {
                    break 'outer;
                }
            }
        }
    }
    let mut files: Vec<(PathBuf, SystemTime, u64)> = days
        .iter()
        .filter_map(|d| fs::read_dir(d).ok())
        .flat_map(|rd| rd.flatten())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .filter_map(|p| {
            let m = fs::metadata(&p).ok()?;
            Some((p, m.modified().ok()?, m.len()))
        })
        .collect();
    files.sort_by(|a, b| b.1.cmp(&a.1));
    files.truncate(5);
    files
}

fn read_tail(path: &Path) -> Option<String> {
    let mut f = fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    f.seek(SeekFrom::Start(len.saturating_sub(TAIL_BYTES))).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    Some(String::from_utf8_lossy(&buf).into_owned())
}

/// Minimal RFC 3339 parser for "YYYY-MM-DDTHH:MM:SS(.fff)Z" → epoch ms.
fn parse_rfc3339_ms(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 20 {
        return None;
    }
    let num = |r: std::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, se) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let mut ms = 0;
    if b.get(19) == Some(&b'.') {
        let frac: String = s[20..].chars().take_while(|c| c.is_ascii_digit()).collect();
        let f3 = format!("{:0<3}", &frac[..frac.len().min(3)]);
        ms = f3.parse().ok()?;
    }
    if !s.ends_with('Z') {
        return None;
    }
    // Days from civil (Howard Hinnant).
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 86_400) + h * 3600 + mi * 60 + se) * 1000 + ms)
}

fn window_from(id_hint: &str, v: &Value) -> Option<RawWindow> {
    let used = v.get("used_percent")?.as_f64()?;
    let minutes = v.get("window_minutes").and_then(Value::as_i64);
    let id = match minutes {
        Some(300) => "five_hour".to_string(),
        Some(10080) => "weekly".to_string(),
        _ => id_hint.to_string(),
    };
    let label = match minutes {
        Some(300) => "5-hour limit".to_string(),
        Some(10080) => "Weekly limit".to_string(),
        Some(m) if m % 1440 == 0 => format!("{}-day limit", m / 1440),
        Some(m) if m % 60 == 0 => format!("{}-hour limit", m / 60),
        _ => "Usage limit".to_string(),
    };
    Some(RawWindow {
        id,
        label: Some(label),
        used_percent: Some(used),
        resets_at: v.get("resets_at").and_then(Value::as_i64),
        reset_text: None,
        limit_reached: false,
    })
}

fn parse_rollout(text: &str) -> Option<SourceDoc> {
    for line in text.lines().rev() {
        if !line.contains("\"rate_limits\"") {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        let Some(rl) = v.pointer("/payload/rate_limits").or_else(|| v.get("rate_limits")) else {
            continue;
        };
        let windows: Vec<RawWindow> = [("primary", "primary"), ("secondary", "secondary")]
            .iter()
            .filter_map(|(k, hint)| window_from(hint, rl.get(*k)?))
            .collect();
        if windows.is_empty() {
            continue;
        }
        let captured_at = v
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(parse_rfc3339_ms)?;
        let plan = rl.get("plan_type").and_then(Value::as_str).map(|p| {
            let mut c = p.chars();
            c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
        });
        return Some(SourceDoc { captured_at, plan, windows });
    }
    None
}

fn from_logs(ctx: &Ctx) -> Option<SourceDoc> {
    let sessions = home_path(ctx.home_dir, &[".codex", "sessions"]);
    let files = recent_rollouts(&sessions);
    let newest = files.first()?.clone();
    let mut cache = ctx.codex_cache.lock().ok()?;
    if cache.key.as_ref() == Some(&newest) {
        return cache.doc.clone();
    }
    let doc = files.iter().find_map(|(p, _, _)| parse_rollout(&read_tail(p)?));
    cache.key = Some(newest);
    cache.doc = doc.clone();
    doc
}

pub fn snapshot(ctx: &Ctx) -> ProviderSnapshot {
    let codex_installed = home_path(ctx.home_dir, &[".codex", "sessions"]).is_dir();
    let logs = if codex_installed { from_logs(ctx) } else { None };
    let web = read_source(ctx.sources_dir, WEB_FILE);

    let mut windows: Vec<UsageWindow> = Vec::new();
    for (doc, source) in [(logs.as_ref(), "Codex CLI"), (web.as_ref(), "chatgpt.com (extension)")] {
        let Some(doc) = doc else { continue };
        for raw in &doc.windows {
            let label = raw.label.clone().unwrap_or_else(|| raw.id.clone());
            let short = match raw.id.as_str() {
                "five_hour" => "5h".to_string(),
                "weekly" => "Weekly".to_string(),
                _ => label.clone(),
            };
            let w = finish_window(ctx, raw, label, short, source, doc.captured_at);
            match windows.iter_mut().find(|x| x.id == w.id) {
                Some(old) if old.captured_at < w.captured_at => *old = w,
                Some(_) => {}
                None => windows.push(w),
            }
        }
    }
    windows.sort_by_key(|w| if w.id == "five_hour" { 0 } else { 1 });

    let last_updated = [logs.as_ref(), web.as_ref()].iter().flatten().map(|d| d.captured_at).max();
    let status = status_for(&windows);
    let detail = match status {
        ProviderStatus::Waiting if !codex_installed => "Codex CLI not found on this PC".into(),
        ProviderStatus::Waiting => "Waiting for sync. Limits appear after your next Codex turn.".into(),
        ProviderStatus::Stale => "Data is out of date".into(),
        _ => String::new(),
    };

    ProviderSnapshot {
        id: "codex".into(),
        name: "Codex".into(),
        usage_url: "https://chatgpt.com/codex/settings/usage".into(),
        status,
        detail,
        plan: logs.as_ref().and_then(|d| d.plan.clone()),
        windows,
        last_updated,
        sources: vec![
            SourceInfo {
                id: "codex-logs".into(),
                label: "Codex CLI session logs".into(),
                connected: codex_installed,
                last_seen: logs.as_ref().map(|d| d.captured_at),
                detail: if codex_installed {
                    "Reads the rate-limit status Codex logs locally after each turn.".into()
                } else {
                    "Install the Codex CLI to enable this source.".into()
                },
            },
            SourceInfo {
                id: "chrome-extension".into(),
                label: "Chrome extension · Codex usage page".into(),
                connected: ctx.chrome_host_registered,
                last_seen: web.as_ref().map(|d| d.captured_at),
                detail: "Optional. Reads chatgpt.com/codex/settings/usage while it's open.".into(),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_timestamp() {
        assert_eq!(parse_rfc3339_ms("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_rfc3339_ms("2026-11-11T15:46:04.123Z"), Some(1_794_411_964_123));
    }

    #[test]
    fn parses_rollout_line() {
        let line = r#"{"timestamp":"2026-11-11T15:46:04.000Z","type":"event_msg","payload":{"type":"token_count","rate_limits":{"primary":{"used_percent":14.0,"window_minutes":300,"resets_at":1794401208},"secondary":{"used_percent":94.0,"window_minutes":10080,"resets_at":1794592597},"plan_type":"plus"}}}"#;
        let doc = parse_rollout(line).unwrap();
        assert_eq!(doc.windows.len(), 2);
        assert_eq!(doc.windows[0].id, "five_hour");
        assert_eq!(doc.windows[1].used_percent, Some(94.0));
        assert_eq!(doc.plan.as_deref(), Some("Plus"));
    }
}
