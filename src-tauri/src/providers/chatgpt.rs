//! ChatGPT: OpenAI doesn't publish per-plan message usage for ChatGPT, and
//! the web app shows no running percentage. The only verifiable signal is a
//! limit notice ChatGPT displays when you hit a cap, which the companion
//! extension records. Without one, the provider reports "Unavailable" and
//! never guesses a number.

use super::*;

const WEB_FILE: &str = "chatgpt-web.json";

pub fn snapshot(ctx: &Ctx) -> ProviderSnapshot {
    let web = read_source(ctx.sources_dir, WEB_FILE);
    let mut windows: Vec<UsageWindow> = Vec::new();
    if let Some(doc) = &web {
        for raw in &doc.windows {
            let label = raw.label.clone().unwrap_or_else(|| "Usage limit".into());
            let w = finish_window(ctx, raw, label.clone(), label, "chatgpt.com (extension)", doc.captured_at);
            // A limit notice that has since reset says nothing about current usage,
            // and one without a reset time is only trusted for a few hours.
            let expired = raw.resets_at.is_none() && ctx.now_ms - doc.captured_at > 6 * 3_600_000;
            if w.state != WindowState::Reset && !expired {
                windows.push(w);
            }
        }
    }

    let (status, detail) = if !windows.is_empty() {
        (status_for(&windows), String::new())
    } else if ctx.chrome_host_registered {
        (
            ProviderStatus::Unavailable,
            "No limit reached. ChatGPT doesn't publish a usage percentage.".into(),
        )
    } else {
        (
            ProviderStatus::Unavailable,
            "OpenAI doesn't publish ChatGPT plan usage.".into(),
        )
    };

    ProviderSnapshot {
        id: "chatgpt".into(),
        name: "ChatGPT".into(),
        usage_url: "https://chatgpt.com/".into(),
        status,
        detail,
        plan: web.as_ref().and_then(|d| d.plan.clone()),
        windows,
        last_updated: web.as_ref().map(|d| d.captured_at),
        sources: vec![SourceInfo {
            id: "chrome-extension".into(),
            label: "Chrome extension · ChatGPT limit notices".into(),
            connected: ctx.chrome_host_registered,
            last_seen: web.as_ref().map(|d| d.captured_at),
            detail: "Records \"you've reached your limit\" notices with their reset time.".into(),
        }],
    }
}
