const NAMES = { claude: "Claude", codex: "Codex", chatgpt: "ChatGPT" };

function ago(ms) {
  const s = (Date.now() - ms) / 1000;
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.round(s / 60)}m ago`;
  if (s < 86400) return `${Math.round(s / 3600)}h ago`;
  return `${Math.round(s / 86400)}d ago`;
}

const auto = document.getElementById("auto");
const now = document.getElementById("now");
const last = document.getElementById("last");

chrome.storage.local.get(["autoSync", "lastAutoSync"], ({ autoSync = true, lastAutoSync }) => {
  auto.checked = autoSync;
  if (lastAutoSync) last.textContent = `Last background check ${ago(lastAutoSync)}`;
});
auto.addEventListener("change", () => chrome.storage.local.set({ autoSync: auto.checked }));
now.addEventListener("click", () => {
  now.disabled = true;
  now.textContent = "Syncing…";
  chrome.runtime.sendMessage({ type: "syncNow" }, () => {
    now.textContent = "Started. Numbers update in a few seconds";
  });
});

chrome.storage.local.get(null, (all) => {
  const rows = document.getElementById("rows");
  for (const [id, name] of Object.entries(NAMES)) {
    const st = all[`status_${id}`];
    const row = document.createElement("div");
    row.className = "row";
    const left = document.createElement("span");
    left.textContent = name;
    const right = document.createElement("span");
    if (!st) {
      right.textContent = "No data yet";
      right.className = "muted";
    } else if (st.ok) {
      right.textContent = `Synced ${ago(st.at)}`;
      right.className = "ok";
    } else {
      right.textContent = st.error?.includes("not found") ? "App bridge not set up" : "Sync failed";
      right.title = st.error || "";
      right.className = "err";
    }
    row.append(left, right);
    rows.append(row);
  }
});
