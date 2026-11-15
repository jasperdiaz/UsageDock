//! usagedock-bridge
//!
//! Two jobs, both write-only into `%LOCALAPPDATA%\UsageDock\sources`:
//!
//! 1. `usagedock-bridge statusline` — configured as Claude Code's `statusLine`
//!    command. Claude Code passes documented session JSON on stdin; we keep only
//!    `rate_limits` (percentages + reset epochs) and drop everything else.
//! 2. Chrome native messaging host — Chrome launches us with the calling
//!    extension's origin as the first argument. The companion extension sends
//!    the usage numbers it read from the visible page text; we validate them and
//!    store them. No cookies, tokens or page HTML ever pass through here.

use serde_json::{json, Map, Value};
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEMA: u64 = 1;
const PROVIDERS: [&str; 3] = ["claude", "chatgpt", "codex"];

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("UsageDock")
}

fn sources_dir() -> PathBuf {
    data_dir().join("sources")
}

/// Write via temp file + rename so the desktop app never reads a half-written file.
fn write_atomic(name: &str, value: &Value) -> io::Result<()> {
    let dir = sources_dir();
    fs::create_dir_all(&dir)?;
    let tmp = dir.join(format!(".{name}.tmp"));
    let dst = dir.join(name);
    fs::write(&tmp, serde_json::to_vec_pretty(value).unwrap_or_default())?;
    fs::rename(&tmp, &dst)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let first = args.get(1).map(String::as_str).unwrap_or("");
    if first.starts_with("chrome-extension://") {
        native_messaging_loop();
    } else if first == "statusline" {
        statusline();
    } else {
        eprintln!("usagedock-bridge: run by Claude Code (`statusline`) or by Chrome native messaging.");
        std::process::exit(2);
    }
}

// ---------------------------------------------------------------------------
// Claude Code status line
// ---------------------------------------------------------------------------

fn statusline() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    let parsed: Value =
        serde_json::from_str(input.trim_start_matches('\u{feff}')).unwrap_or(Value::Null);

    let windows = statusline_windows(&parsed);
    if !windows.is_empty() {
        let doc = json!({
            "schema": SCHEMA,
            "provider": "claude",
            "source": "claude-code-statusline",
            "captured_at": now_ms(),
            "windows": windows,
        });
        if should_write("claude-code.json", &doc) {
            let _ = write_atomic("claude-code.json", &doc);
        }
    }

    // If the user had a status line before UsageDock was connected, run it and
    // show its output so connecting UsageDock doesn't take anything away.
    if let Some(out) = run_previous_statusline(&input) {
        print!("{out}");
        return;
    }
    print!("{}", default_line(&parsed));
}

fn statusline_windows(v: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    let Some(rl) = v.get("rate_limits") else { return out };
    for id in ["five_hour", "seven_day"] {
        let Some(w) = rl.get(id) else { continue };
        let used = w.get("used_percentage").and_then(Value::as_f64);
        let resets = w.get("resets_at").and_then(Value::as_i64);
        if let Some(used) = used {
            if (0.0..=1000.0).contains(&used) {
                out.push(json!({ "id": id, "used_percent": used, "resets_at": resets }));
            }
        }
    }
    out
}

/// Avoid rewriting the file (and waking the app) when nothing changed,
/// but still refresh the timestamp at least once a minute so staleness is honest.
fn should_write(name: &str, doc: &Value) -> bool {
    let path = sources_dir().join(name);
    let Ok(bytes) = fs::read(&path) else { return true };
    let Ok(prev) = serde_json::from_slice::<Value>(&bytes) else { return true };
    if prev.get("windows") != doc.get("windows") {
        return true;
    }
    let prev_at = prev.get("captured_at").and_then(Value::as_i64).unwrap_or(0);
    now_ms() - prev_at > 60_000
}

fn default_line(v: &Value) -> String {
    let pct = |id: &str| {
        v.pointer(&format!("/rate_limits/{id}/used_percentage"))
            .and_then(Value::as_f64)
    };
    let model = v
        .pointer("/model/display_name")
        .and_then(Value::as_str)
        .unwrap_or("Claude");
    match (pct("five_hour"), pct("seven_day")) {
        (Some(a), Some(b)) => format!("{model} · 5h {a:.0}% · week {b:.0}%"),
        (Some(a), None) => format!("{model} · 5h {a:.0}%"),
        (None, Some(b)) => format!("{model} · week {b:.0}%"),
        _ => model.to_string(),
    }
}

fn run_previous_statusline(input: &str) -> Option<String> {
    let cfg = fs::read(data_dir().join("statusline-previous.json")).ok()?;
    let cfg: Value = serde_json::from_slice(&cfg).ok()?;
    let cmd = cfg.get("command")?.as_str()?.trim().to_string();
    if cmd.is_empty() {
        return None;
    }
    // Claude Code on Windows runs status lines through Git Bash when present,
    // so try bash first to keep the previous command's semantics, then cmd.
    let attempts: [(&str, &[&str]); 2] = [("bash", &["-c"]), ("cmd", &["/C"])];
    for (shell, flags) in attempts {
        let child = Command::new(shell)
            .args(flags)
            .arg(&cmd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let Ok(mut child) = child else { continue };
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
        }
        if let Ok(out) = child.wait_with_output() {
            return Some(String::from_utf8_lossy(&out.stdout).into_owned());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Chrome native messaging
// ---------------------------------------------------------------------------

fn read_message(stdin: &mut impl Read) -> Option<Value> {
    let mut len = [0u8; 4];
    stdin.read_exact(&mut len).ok()?;
    let len = u32::from_le_bytes(len) as usize;
    if len == 0 || len > 64 * 1024 {
        return None;
    }
    let mut buf = vec![0u8; len];
    stdin.read_exact(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

fn write_message(stdout: &mut impl Write, v: &Value) {
    let bytes = serde_json::to_vec(v).unwrap_or_default();
    let _ = stdout.write_all(&(bytes.len() as u32).to_le_bytes());
    let _ = stdout.write_all(&bytes);
    let _ = stdout.flush();
}

fn native_messaging_loop() {
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    while let Some(msg) = read_message(&mut stdin) {
        let reply = match msg.get("type").and_then(Value::as_str) {
            Some("ping") => json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }),
            Some("usage") => match sanitize_usage(&msg) {
                Ok((provider, doc)) => match write_atomic(&format!("{provider}-web.json"), &doc) {
                    Ok(()) => json!({ "ok": true, "stored": doc["windows"].as_array().map(|a| a.len()).unwrap_or(0) }),
                    Err(e) => json!({ "ok": false, "error": format!("write failed: {e}") }),
                },
                Err(e) => json!({ "ok": false, "error": e }),
            },
            _ => json!({ "ok": false, "error": "unknown message type" }),
        };
        write_message(&mut stdout, &reply);
    }
}

fn clean_text(v: Option<&Value>, max: usize) -> Option<String> {
    let s = v?.as_str()?;
    let s: String = s.chars().filter(|c| !c.is_control()).take(max).collect();
    let s = s.trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn sanitize_usage(msg: &Value) -> Result<(&'static str, Value), String> {
    let provider = msg.get("provider").and_then(Value::as_str).unwrap_or("");
    let provider = PROVIDERS
        .iter()
        .find(|p| **p == provider)
        .copied()
        .ok_or("unsupported provider")?;

    let page = msg.get("page").and_then(Value::as_str).unwrap_or("");
    let allowed_page = page.starts_with("https://claude.ai/") || page.starts_with("https://chatgpt.com/");
    if !allowed_page {
        return Err("page not allowed".into());
    }

    let now_s = now_ms() / 1000;
    let mut windows = Vec::new();
    for w in msg.get("windows").and_then(Value::as_array).into_iter().flatten().take(12) {
        let Some(id) = clean_text(w.get("id"), 40) else { continue };
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        let raw_used = w.get("used_percent").filter(|v| !v.is_null());
        let used = raw_used.and_then(Value::as_f64).filter(|p| (0.0..=100.0).contains(p));
        let limit_reached = w.get("limit_reached").and_then(Value::as_bool).unwrap_or(false);
        // Drop windows with an invalid percentage, or with nothing to show at all.
        if (raw_used.is_some() && used.is_none()) || (used.is_none() && !limit_reached) {
            continue;
        }
        let resets = w
            .get("resets_at")
            .and_then(Value::as_i64)
            .filter(|t| *t > now_s - 86_400 && *t < now_s + 40 * 86_400);
        let mut o = Map::new();
        o.insert("id".into(), json!(id));
        o.insert("label".into(), json!(clean_text(w.get("label"), 60)));
        o.insert("used_percent".into(), json!(used));
        o.insert("resets_at".into(), json!(resets));
        o.insert("reset_text".into(), json!(clean_text(w.get("reset_text"), 80)));
        o.insert("limit_reached".into(), json!(limit_reached));
        windows.push(Value::Object(o));
    }
    if windows.is_empty() {
        return Err("no usage windows in message".into());
    }

    let doc = json!({
        "schema": SCHEMA,
        "provider": provider,
        "source": "chrome-extension",
        "captured_at": now_ms(),
        "page": page.split(['?', '#']).next().unwrap_or(""),
        "plan": clean_text(msg.get("plan"), 40),
        "windows": windows,
    });
    Ok((provider, doc))
}
