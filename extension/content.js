// UsageDock Connector: content script.
//
// Reads the *visible text* of usage pages you already have open and sends the
// numbers (percentages, labels, reset times) to the UsageDock desktop app via
// the extension's background worker. It never reads cookies, storage, network
// traffic or page HTML, and it sends nothing anywhere except the local app.
//
// Page layouts change. Parsing is deliberately text-based and lenient; if
// nothing recognisable is found, nothing is sent.

(() => {
  const SCAN_DEBOUNCE_MS = 2000;
  const RESEND_MS = 60_000;
  let lastSent = "";
  let lastSentAt = 0;
  let timer = 0;

  const visibleLines = (root = document.body) =>
    (root?.innerText || "")
      .split("\n")
      .map((s) => s.replace(/[\s​‌‍﻿]+/g, " ").trim())
      .filter(Boolean)
      .slice(0, 2000);

  // ---- reset-time parsing -------------------------------------------------

  const UNIT = { d: 86400, day: 86400, days: 86400, h: 3600, hr: 3600, hrs: 3600, hour: 3600, hours: 3600, m: 60, min: 60, mins: 60, minute: 60, minutes: 60 };
  const WEEKDAYS = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];
  const MONTHS = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];

  /** Returns epoch seconds, or null if the wording isn't understood. */
  function parseReset(text, now = new Date()) {
    const t = text.toLowerCase();

    // "in 3 hr 12 min", "in 45 minutes", "in 2 days 4 hours"
    const rel = t.match(/\bin\s+((?:\d+\s*[a-z]+\s*)+)/);
    if (rel) {
      let secs = 0;
      for (const m of rel[1].matchAll(/(\d+)\s*([a-z]+)/g)) {
        const u = UNIT[m[2]];
        if (u) secs += Number(m[1]) * u;
      }
      if (secs > 0) return Math.round(now.getTime() / 1000 + secs);
    }

    // Absolute: optional weekday / month-day / "tomorrow", then a clock time.
    const time = t.match(/(\d{1,2})(?::(\d{2}))?\s*(am|pm)\b/) || t.match(/\b(\d{1,2}):(\d{2})\b/);
    if (!time) return null;
    let hour = Number(time[1]);
    const minute = Number(time[2] || 0);
    const ampm = time[3];
    if (ampm === "pm" && hour < 12) hour += 12;
    if (ampm === "am" && hour === 12) hour = 0;

    const d = new Date(now);
    d.setSeconds(0, 0);
    d.setHours(hour, minute);

    const md = t.match(/\b(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*\.?\s+(\d{1,2})\b/);
    const wd = WEEKDAYS.findIndex((w) => new RegExp(`\\b${w}`).test(t));
    if (md) {
      d.setMonth(MONTHS.indexOf(md[1]), Number(md[2]));
      if (d < now) d.setFullYear(d.getFullYear() + 1);
    } else if (/\btomorrow\b/.test(t)) {
      d.setDate(d.getDate() + 1);
    } else if (wd >= 0) {
      let add = (wd - d.getDay() + 7) % 7;
      if (add === 0 && d <= now) add = 7;
      d.setDate(d.getDate() + add);
    } else if (d <= now) {
      d.setDate(d.getDate() + 1);
    }
    return Math.round(d.getTime() / 1000);
  }

  // ---- percentage blocks --------------------------------------------------

  const PCT = /(\d{1,3}(?:\.\d+)?)\s*%/;
  // A meter line is *only* a percentage, e.g. "16% used" or "84% remaining".
  // Sentences that mention a percentage ("3% of your weekly limit used") are not.
  const METER = /^(\d{1,3}(?:\.\d+)?)\s*%\s*(used|remaining|left)?$/i;

  /**
   * Find "NN% used" lines and walk back a few lines to pick up the label and
   * "Resets ..." line that belong to the same block.
   * `requireWord`: only accept meters that say used/remaining/left, so share
   * breakdowns like "Claude Code 100%" are never mistaken for limits.
   */
  function percentBlocks(lines, requireWord = false) {
    const blocks = [];
    let prevEnd = -1;
    lines.forEach((line, i) => {
      const m = line.match(METER);
      if (!m || (requireWord && !m[2])) return;
      const value = Number(m[1]);
      if (!(value >= 0 && value <= 100)) return;
      const used = /remaining|left/i.test(m[2] || "") ? 100 - value : value;
      const isReset = (l) => /\bresets?\b/i.test(l);
      // Only lines since the previous block belong to this one.
      const before = lines.slice(Math.max(prevEnd + 1, i - 6), i);
      let resetLine = before.find(isReset) || null;
      let end = i;
      // Some layouts put "Resets ..." right after the percentage instead.
      if (!resetLine && lines[i + 1] && isReset(lines[i + 1])) {
        resetLine = lines[i + 1];
        end = i + 1;
      }
      const labels = before.filter((l) => !isReset(l) && !PCT.test(l) && l.length <= 48 && !/^learn more/i.test(l));
      blocks.push({ used, labels, resetLine });
      prevEnd = end;
    });
    return blocks;
  }

  const slug = (s) => s.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_|_$/g, "").slice(0, 30);

  function toWindow(id, label, block) {
    return {
      id,
      label,
      used_percent: block.used,
      resets_at: block.resetLine ? parseReset(block.resetLine) : null,
      reset_text: block.resetLine ? block.resetLine.slice(0, 80) : null,
      limit_reached: block.used >= 100,
    };
  }

  // Claude's usage page is either a route (/settings/usage) or, since late
  // 2026, a settings dialog opened over a chat page (/new#settings/usage).
  const onClaudeUsage = () =>
    location.pathname.startsWith("/settings/usage") || location.hash.startsWith("#settings/usage");

  function claudeUsage() {
    if (!onClaudeUsage()) return null;
    const root = document.querySelector('[role="dialog"]') || document.querySelector("main") || document.body;
    let lines = visibleLines(root);
    // Everything from the per-product breakdown down is shares, not limits.
    const cut = lines.findIndex((l) => /usage by product/i.test(l));
    if (cut >= 0) lines = lines.slice(0, cut);
    const windows = [];
    for (const b of percentBlocks(lines, true)) {
      // Use the label nearest the meter; earlier lines may be page headings.
      const near = b.labels.slice(-2).join(" ");
      let id = null;
      let label = b.labels[b.labels.length - 1] || "Usage";
      if (/current session|5[\s-]?hour/i.test(near)) id = "five_hour";
      else if (/opus/i.test(near)) id = "seven_day_opus";
      else if (/sonnet/i.test(near)) id = "seven_day_sonnet";
      else if (/all models|weekly|this week/i.test(near)) id = "seven_day";
      else id = `other_${slug(label)}`;
      if (!windows.some((w) => w.id === id)) windows.push(toWindow(id, label, b));
    }
    return windows.length ? { provider: "claude", windows } : null;
  }

  function codexUsage() {
    if (!/\/codex\b.*usage/.test(location.pathname)) return null;
    const windows = [];
    for (const b of percentBlocks(visibleLines())) {
      const text = b.labels.join(" ");
      let id = null;
      let label = null;
      if (/5[\s-]?h(ou)?r/i.test(text)) [id, label] = ["five_hour", "5-hour limit"];
      else if (/week/i.test(text)) [id, label] = ["weekly", "Weekly limit"];
      if (id && !windows.some((w) => w.id === id)) windows.push(toWindow(id, label, b));
    }
    return windows.length ? { provider: "codex", windows } : null;
  }

  /** ChatGPT has no usage meter; only record limit notices it actually shows. */
  function chatgptLimits() {
    if (location.pathname.includes("/codex")) return null;
    const windows = [];
    for (const line of visibleLines()) {
      if (line.length > 300) continue;
      const m = line.match(/you['’]ve (?:hit|reached) (?:your|the)\s+(.{0,40}?)\s*(?:usage\s+)?limit/i);
      if (!m) continue;
      const model = (m[1] || "").replace(/\b(plan|message|messages)\b/gi, "").trim();
      const label = model ? `${model} limit` : "Usage limit";
      const r = line.match(/((?:limit\s+)?resets?|try again|available again)[^.]*?((?:after|at|in|on|tomorrow)[^.]{2,50})/i);
      const resetText = r ? `${r[1]} ${r[2]}`.trim() : null;
      const id = `limit_${slug(model || "general")}`;
      if (windows.some((w) => w.id === id)) continue;
      windows.push({
        id,
        label: label.charAt(0).toUpperCase() + label.slice(1),
        used_percent: 100,
        resets_at: resetText ? parseReset(resetText) : null,
        reset_text: resetText,
        limit_reached: true,
      });
    }
    return windows.length ? { provider: "chatgpt", windows } : null;
  }

  function scan() {
    const found = location.hostname === "claude.ai" ? claudeUsage() : codexUsage() || chatgptLimits();
    if (!found) return;
    const key = JSON.stringify(found);
    if (key === lastSent && Date.now() - lastSentAt < RESEND_MS) return;
    lastSent = key;
    lastSentAt = Date.now();
    const page = location.origin + location.pathname;
    chrome.runtime.sendMessage({ type: "usage", page, ...found }).catch(() => {});
  }

  const schedule = () => {
    window.clearTimeout(timer);
    timer = window.setTimeout(scan, SCAN_DEBOUNCE_MS);
  };

  new MutationObserver(schedule).observe(document.documentElement, { childList: true, subtree: true, characterData: true });
  // Keep data fresh while the page stays open, but only when it's visible.
  window.setInterval(() => document.visibilityState === "visible" && scan(), RESEND_MS);
  schedule();
})();
