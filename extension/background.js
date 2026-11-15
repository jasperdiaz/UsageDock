// Relays usage messages from the content script to the UsageDock desktop app
// through Chrome native messaging. Nothing is sent to any server.
//
// Background sync: every SYNC_MINUTES the worker opens claude.ai's own usage
// page in a minimized, unfocused window, lets the content script read the
// visible numbers, then closes the window. It is the same page you'd open by
// hand; no cookies are read and no private APIs are called.
const HOST = "com.usagedock.bridge";
const ALLOWED = new Set(["claude", "chatgpt", "codex"]);
const SYNC_MINUTES = 15;
const SYNC_URL = "https://claude.ai/settings/usage";
const GIVE_UP_MS = 45_000;

// ---- relay ---------------------------------------------------------------

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg?.type !== "usage" || !ALLOWED.has(msg.provider) || sender.id !== chrome.runtime.id) return;
  chrome.runtime.sendNativeMessage(HOST, msg, (reply) => {
    const error = chrome.runtime.lastError?.message || (reply && !reply.ok ? reply.error : null);
    const status = { at: Date.now(), ok: !error, error: error || null, windows: msg.windows.length };
    chrome.storage.local.set({ [`status_${msg.provider}`]: status });
    sendResponse(status);
    if (sender.tab) finishAutoSync(sender.tab.id);
  });
  return true;
});

// ---- background sync -----------------------------------------------------

async function settings() {
  const { autoSync = true } = await chrome.storage.local.get("autoSync");
  return { autoSync };
}

async function closeAutoWindow() {
  const { autoWindow } = await chrome.storage.session.get("autoWindow");
  if (!autoWindow) return;
  await chrome.storage.session.remove("autoWindow");
  try {
    await chrome.windows.remove(autoWindow.windowId);
  } catch {
    // Already closed by the user.
  }
}

async function finishAutoSync(tabId) {
  const { autoWindow } = await chrome.storage.session.get("autoWindow");
  if (autoWindow && autoWindow.tabId === tabId) {
    // Give the page a moment so a second, fuller scan can land too.
    setTimeout(closeAutoWindow, 3000);
  }
}

async function runAutoSync(force = false) {
  if (!force && !(await settings()).autoSync) return;

  // A stuck window from an earlier run: close it and start over.
  const { autoWindow } = await chrome.storage.session.get("autoWindow");
  if (autoWindow) {
    if (Date.now() - autoWindow.at < GIVE_UP_MS) return;
    await closeAutoWindow();
  }

  // Only while Chrome is actually open, so we never launch a window from nothing.
  const open = await chrome.windows.getAll({ windowTypes: ["normal"] });
  if (!open.length) return;

  const win = await chrome.windows.create({ url: SYNC_URL, state: "minimized", focused: false, type: "normal" });
  const tabId = win.tabs?.[0]?.id;
  await chrome.storage.session.set({ autoWindow: { windowId: win.id, tabId, at: Date.now() } });
  await chrome.storage.local.set({ lastAutoSync: Date.now() });
  setTimeout(closeAutoWindow, GIVE_UP_MS);
  chrome.alarms.create("autoclose", { delayInMinutes: 1 }); // in case the worker sleeps
}

function schedule() {
  chrome.alarms.create("autosync", { delayInMinutes: 1, periodInMinutes: SYNC_MINUTES });
}

chrome.runtime.onInstalled.addListener(schedule);
chrome.runtime.onStartup.addListener(schedule);
chrome.alarms.onAlarm.addListener((a) => {
  if (a.name === "autosync") runAutoSync();
  if (a.name === "autoclose") closeAutoWindow();
});

// Popup: "Sync now".
chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg?.type !== "syncNow" || sender.id !== chrome.runtime.id) return;
  runAutoSync(true).then(() => sendResponse({ ok: true }));
  return true;
});

// Safety net if the worker is suspended before the close timer fires.
chrome.windows.onRemoved.addListener(async (id) => {
  const { autoWindow } = await chrome.storage.session.get("autoWindow");
  if (autoWindow?.windowId === id) chrome.storage.session.remove("autoWindow");
});
