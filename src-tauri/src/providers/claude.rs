use super::*;
use std::collections::BTreeMap;

const STATUSLINE_FILE: &str = "claude-code.json";
const WEB_FILE: &str = "claude-web.json";

fn labels(id: &str, given: Option<&str>) -> (String, String) {
    match id {
        "five_hour" => ("Current session".into(), "Session".into()),
        "seven_day" => ("Weekly · all models".into(), "Weekly".into()),
        "seven_day_opus" => ("Weekly · Opus".into(), "Opus".into()),
        "seven_day_sonnet" => ("Weekly · Sonnet".into(), "Sonnet".into()),
        _ => {
            let l = given.unwrap_or(id).to_string();
            (l.clone(), l)
        }
    }
}

/// Session first, then weekly, then model-specific limits.
fn order(id: &str) -> u8 {
    match id {
        "five_hour" => 0,
        "seven_day" => 1,
        "seven_day_sonnet" => 2,
        "seven_day_opus" => 3,
        _ => 9,
    }
}

pub fn snapshot(ctx: &Ctx) -> ProviderSnapshot {
    let statusline = read_source(ctx.sources_dir, STATUSLINE_FILE);
    let web = read_source(ctx.sources_dir, WEB_FILE);

    // Rate limits are account-wide, so both sources describe the same windows.
    // For each window keep whichever source saw it most recently.
    let mut by_id: BTreeMap<String, UsageWindow> = BTreeMap::new();
    let inputs = [
        (statusline.as_ref(), "Claude Code"),
        (web.as_ref(), "claude.ai (extension)"),
    ];
    for (doc, source) in inputs {
        let Some(doc) = doc else { continue };
        for raw in &doc.windows {
            let (label, short) = labels(&raw.id, raw.label.as_deref());
            let w = finish_window(ctx, raw, label, short, source, doc.captured_at);
            let newer = by_id.get(&raw.id).is_none_or(|old| old.captured_at < w.captured_at);
            if newer {
                by_id.insert(raw.id.clone(), w);
            }
        }
    }
    let mut windows: Vec<UsageWindow> = by_id.into_values().collect();
    windows.sort_by_key(|w| order(&w.id));

    let last_updated = [statusline.as_ref(), web.as_ref()]
        .iter()
        .flatten()
        .map(|d| d.captured_at)
        .max();

    let sources = vec![
        SourceInfo {
            id: "claude-code".into(),
            label: "Claude Code status line".into(),
            connected: ctx.claude_statusline_installed,
            last_seen: statusline.as_ref().map(|d| d.captured_at),
            detail: if ctx.claude_statusline_installed {
                "Updates whenever Claude Code is running (Pro/Max plans).".into()
            } else {
                "Not connected. Connect it in Sources to read your 5-hour and weekly limits.".into()
            },
        },
        SourceInfo {
            id: "chrome-extension".into(),
            label: "Chrome extension · claude.ai usage page".into(),
            connected: ctx.chrome_host_registered,
            last_seen: web.as_ref().map(|d| d.captured_at),
            detail: if ctx.chrome_host_registered {
                "Reads the numbers shown on claude.ai/settings/usage while that page is open.".into()
            } else {
                "Optional. Adds model-specific weekly limits from the claude.ai usage page.".into()
            },
        },
    ];

    let status = status_for(&windows);
    let detail = match status {
        ProviderStatus::Waiting if !ctx.claude_statusline_installed && !ctx.chrome_host_registered => {
            "No source connected".into()
        }
        ProviderStatus::Waiting => "Waiting for sync. Use Claude Code or open the usage page.".into(),
        ProviderStatus::Stale => "Data is out of date".into(),
        _ => String::new(),
    };

    ProviderSnapshot {
        id: "claude".into(),
        name: "Claude".into(),
        usage_url: "https://claude.ai/settings/usage".into(),
        status,
        detail,
        plan: web.and_then(|d| d.plan),
        windows,
        last_updated,
        sources,
    }
}
