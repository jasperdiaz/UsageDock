//! Connecting UsageDock to its data sources. Every change here is made only
//! when the user presses a button, is reversible, and backs up what it edits.

use serde::Serialize;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub const NATIVE_HOST_NAME: &str = "com.usagedock.bridge";
/// Fixed by the `key` in extension/manifest.json.
pub const EXTENSION_ID: &str = "paloehjfnkkcpjldoefecjeelnpkjhjj";
const BRIDGE_EXE: &str = "usagedock-bridge.exe";
const BROWSER_KEYS: [&str; 2] = [
    r"Software\Google\Chrome\NativeMessagingHosts",
    r"Software\Microsoft\Edge\NativeMessagingHosts",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationStatus {
    pub bridge_found: bool,
    pub claude_code_found: bool,
    pub claude_statusline_installed: bool,
    /// A non-UsageDock status line the user had before connecting (it keeps working).
    pub claude_statusline_previous: Option<String>,
    pub chrome_host_registered: bool,
    pub extension_dir: String,
    pub extension_id: String,
    pub sources_dir: String,
}

pub struct Paths {
    pub home: PathBuf,
    pub data: PathBuf,
    pub bridge: PathBuf,
    pub bundled_extension: Option<PathBuf>,
}

impl Paths {
    pub fn sources(&self) -> PathBuf {
        self.data.join("sources")
    }
    fn claude_settings(&self) -> PathBuf {
        self.home.join(".claude").join("settings.json")
    }
    fn previous_statusline(&self) -> PathBuf {
        self.data.join("statusline-previous.json")
    }
    fn native_manifest(&self) -> PathBuf {
        self.data.join("native-host").join(format!("{NATIVE_HOST_NAME}.json"))
    }
    pub fn extension_dir(&self) -> PathBuf {
        self.data.join("chrome-extension")
    }
}

/// `%LOCALAPPDATA%\UsageDock`. The bridge uses the same location.
pub fn data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("UsageDock")
}

pub fn bridge_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default()
        .join(BRIDGE_EXE)
}

fn bridge_command(bridge: &Path) -> String {
    // Forward slashes and quotes work in both Git Bash and cmd.
    format!("\"{}\" statusline", bridge.to_string_lossy().replace('\\', "/"))
}

fn is_ours(cmd: &str) -> bool {
    cmd.contains("usagedock-bridge")
}

fn read_json(path: &Path) -> Result<Option<Value>, String> {
    match fs::read(path) {
        Ok(b) if b.iter().all(u8::is_ascii_whitespace) => Ok(None),
        Ok(b) => serde_json::from_slice(&b)
            .map(Some)
            .map_err(|e| format!("{} isn't valid JSON ({e}); left it unchanged.", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Couldn't read {}: {e}", path.display())),
    }
}

fn write_json(path: &Path, v: &Value) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.usagedock-tmp");
    let mut bytes = serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn current_statusline_cmd(paths: &Paths) -> Option<String> {
    let v = read_json(&paths.claude_settings()).ok()??;
    v.pointer("/statusLine/command")?.as_str().map(str::to_string)
}

pub fn status(paths: &Paths) -> IntegrationStatus {
    let cmd = current_statusline_cmd(paths);
    let previous = read_json(&paths.previous_statusline())
        .ok()
        .flatten()
        .and_then(|v| v.get("command")?.as_str().map(str::to_string));
    IntegrationStatus {
        bridge_found: paths.bridge.is_file(),
        claude_code_found: paths.home.join(".claude").is_dir(),
        claude_statusline_installed: cmd.as_deref().is_some_and(is_ours),
        claude_statusline_previous: previous,
        chrome_host_registered: chrome_host_registered(paths),
        extension_dir: paths.extension_dir().to_string_lossy().into_owned(),
        extension_id: EXTENSION_ID.into(),
        sources_dir: paths.sources().to_string_lossy().into_owned(),
    }
}

// ---------------------------------------------------------------------------
// Claude Code status line
// ---------------------------------------------------------------------------

pub fn connect_claude_code(paths: &Paths) -> Result<(), String> {
    if !paths.bridge.is_file() {
        return Err(format!("Bridge not found at {}", paths.bridge.display()));
    }
    let settings_path = paths.claude_settings();
    let mut root = read_json(&settings_path)?.unwrap_or_else(|| json!({}));
    let obj = root
        .as_object_mut()
        .ok_or("~/.claude/settings.json isn't a JSON object; left it unchanged.")?;

    // One-time backup of the user's file before UsageDock first touches it.
    let backup = settings_path.with_extension("json.usagedock-backup");
    if settings_path.is_file() && !backup.exists() {
        fs::copy(&settings_path, &backup).map_err(|e| format!("Backup failed: {e}"))?;
    }

    let existing = obj.get("statusLine").cloned();
    let existing_cmd = existing
        .as_ref()
        .and_then(|v| v.get("command"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if existing.is_some() && !is_ours(existing_cmd) {
        // Keep the previous status line: the bridge runs it and shows its output.
        write_json(&paths.previous_statusline(), existing.as_ref().unwrap())?;
    }

    let mut line = Map::new();
    line.insert("type".into(), json!("command"));
    line.insert("command".into(), json!(bridge_command(&paths.bridge)));
    if let Some(p) = existing.as_ref().and_then(|v| v.get("padding")) {
        line.insert("padding".into(), p.clone());
    }
    obj.insert("statusLine".into(), Value::Object(line));
    write_json(&settings_path, &root)
}

pub fn disconnect_claude_code(paths: &Paths) -> Result<(), String> {
    let settings_path = paths.claude_settings();
    let Some(mut root) = read_json(&settings_path)? else { return Ok(()) };
    let Some(obj) = root.as_object_mut() else { return Ok(()) };
    let ours = obj
        .get("statusLine")
        .and_then(|v| v.get("command"))
        .and_then(Value::as_str)
        .is_some_and(is_ours);
    if ours {
        match read_json(&paths.previous_statusline())? {
            Some(prev) => {
                obj.insert("statusLine".into(), prev);
            }
            None => {
                obj.shift_remove("statusLine");
            }
        }
        write_json(&settings_path, &root)?;
    }
    let _ = fs::remove_file(paths.previous_statusline());
    Ok(())
}

// ---------------------------------------------------------------------------
// Chrome / Edge native messaging host
// ---------------------------------------------------------------------------

fn chrome_host_registered(paths: &Paths) -> bool {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = format!(r"{}\{NATIVE_HOST_NAME}", BROWSER_KEYS[0]);
    let Ok(k) = hkcu.open_subkey(key) else { return false };
    let Ok(manifest) = k.get_value::<String, _>("") else { return false };
    if Path::new(&manifest) != paths.native_manifest() {
        return false;
    }
    let Ok(Some(m)) = read_json(Path::new(&manifest)) else { return false };
    m.get("path")
        .and_then(Value::as_str)
        .is_some_and(|p| Path::new(p).is_file())
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

/// Registers the bridge as a native messaging host for Chrome and Edge
/// (per-user, HKCU only) and copies the extension to a stable folder the user
/// can load with "Load unpacked".
pub fn register_chrome_host(paths: &Paths) -> Result<(), String> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    if !paths.bridge.is_file() {
        return Err(format!("Bridge not found at {}", paths.bridge.display()));
    }
    let manifest_path = paths.native_manifest();
    let manifest = json!({
        "name": NATIVE_HOST_NAME,
        "description": "UsageDock bridge: receives usage numbers from the UsageDock Connector extension",
        "path": paths.bridge.to_string_lossy(),
        "type": "stdio",
        "allowed_origins": [format!("chrome-extension://{EXTENSION_ID}/")],
    });
    write_json(&manifest_path, &manifest)?;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for base in BROWSER_KEYS {
        let (key, _) = hkcu
            .create_subkey(format!(r"{base}\{NATIVE_HOST_NAME}"))
            .map_err(|e| format!("Registry write failed: {e}"))?;
        key.set_value("", &manifest_path.to_string_lossy().to_string())
            .map_err(|e| format!("Registry write failed: {e}"))?;
    }

    if let Some(src) = paths.bundled_extension.as_ref().filter(|p| p.is_dir()) {
        let dst = paths.extension_dir();
        let _ = fs::remove_dir_all(&dst);
        copy_dir(src, &dst).map_err(|e| format!("Couldn't copy extension: {e}"))?;
    }
    Ok(())
}

pub fn unregister_chrome_host(paths: &Paths) -> Result<(), String> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for base in BROWSER_KEYS {
        let _ = hkcu.delete_subkey_all(format!(r"{base}\{NATIVE_HOST_NAME}"));
    }
    let _ = fs::remove_file(paths.native_manifest());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_paths(name: &str) -> Paths {
        let root = std::env::temp_dir().join(format!("usagedock-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let home = root.join("home");
        fs::create_dir_all(home.join(".claude")).unwrap();
        let bridge = root.join("usagedock-bridge.exe");
        fs::write(&bridge, b"stub").unwrap();
        Paths { home, data: root.join("data"), bridge, bundled_extension: None }
    }

    fn settings(p: &Paths) -> Value {
        serde_json::from_slice(&fs::read(p.claude_settings()).unwrap()).unwrap()
    }

    #[test]
    fn connect_preserves_other_keys_and_disconnect_removes() {
        let p = temp_paths("plain");
        fs::write(p.claude_settings(), r#"{"model":"opus","permissions":{"allow":["Bash"]}}"#).unwrap();
        connect_claude_code(&p).unwrap();
        let s = settings(&p);
        assert_eq!(s["model"], "opus");
        assert_eq!(s["permissions"]["allow"][0], "Bash");
        assert!(s["statusLine"]["command"].as_str().unwrap().contains("usagedock-bridge"));
        // Key order is preserved (statusLine appended at the end).
        let keys: Vec<_> = s.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, ["model", "permissions", "statusLine"]);
        assert!(p.claude_settings().with_extension("json.usagedock-backup").is_file());
        assert!(status(&p).claude_statusline_installed);

        disconnect_claude_code(&p).unwrap();
        let s = settings(&p);
        assert!(s.get("statusLine").is_none());
        assert_eq!(s["model"], "opus");
    }

    #[test]
    fn existing_statusline_is_kept_and_restored() {
        let p = temp_paths("existing");
        fs::write(p.claude_settings(), r#"{"statusLine":{"type":"command","command":"my-line.sh","padding":1}}"#).unwrap();
        connect_claude_code(&p).unwrap();
        let s = settings(&p);
        assert_eq!(s["statusLine"]["padding"], 1);
        assert_eq!(status(&p).claude_statusline_previous.as_deref(), Some("my-line.sh"));
        // Connecting twice must not overwrite the saved original with our own command.
        connect_claude_code(&p).unwrap();
        assert_eq!(status(&p).claude_statusline_previous.as_deref(), Some("my-line.sh"));

        disconnect_claude_code(&p).unwrap();
        assert_eq!(settings(&p)["statusLine"]["command"], "my-line.sh");
        assert!(status(&p).claude_statusline_previous.is_none());
    }

    #[test]
    fn invalid_json_is_left_alone() {
        let p = temp_paths("invalid");
        fs::write(p.claude_settings(), "{ not json").unwrap();
        assert!(connect_claude_code(&p).is_err());
        assert_eq!(fs::read_to_string(p.claude_settings()).unwrap(), "{ not json");
    }

    #[test]
    fn missing_settings_file_is_created() {
        let p = temp_paths("missing");
        connect_claude_code(&p).unwrap();
        assert!(settings(&p)["statusLine"]["command"].is_string());
    }
}
