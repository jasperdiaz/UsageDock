import type { UsageWindow } from "./types";

/** "6d 8h", "1h 22m", "14m", "<1m". */
export function duration(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m`;
  return "<1m";
}

/** "just now", "4m ago", "3h ago", "2d ago". */
export function ago(ms: number | null, now: number): string {
  if (ms == null) return "never";
  const s = Math.max(0, (now - ms) / 1000);
  if (s < 45) return "just now";
  if (s < 3600) return `${Math.round(s / 60)}m ago`;
  if (s < 86400) return `${Math.round(s / 3600)}h ago`;
  return `${Math.round(s / 86400)}d ago`;
}

export function clockTime(ms: number): string {
  return new Date(ms).toLocaleString(undefined, {
    weekday: "short",
    hour: "numeric",
    minute: "2-digit",
  });
}

export type Level = "ok" | "warn" | "high" | "crit" | "none";

export function level(used: number | null): Level {
  if (used == null) return "none";
  if (used >= 95) return "crit";
  if (used >= 80) return "high";
  if (used >= 50) return "warn";
  return "ok";
}

export const levelColor: Record<Level, string> = {
  ok: "var(--color-ok)",
  warn: "var(--color-warn)",
  high: "var(--color-high)",
  crit: "var(--color-crit)",
  none: "var(--color-ink-4)",
};

/** The percentage to display and its suffix, honouring "show remaining". */
export function shown(w: UsageWindow, remaining: boolean): { pct: number | null; suffix: string } {
  if (w.usedPercent == null) return { pct: null, suffix: "" };
  const used = Math.min(100, Math.max(0, w.usedPercent));
  return remaining ? { pct: 100 - used, suffix: "left" } : { pct: used, suffix: "used" };
}

/** Reset line: live countdown from a verified timestamp, else page wording. */
export function resetLine(w: UsageWindow, now: number): string | null {
  if (w.state === "reset") return "Reset · waiting for sync";
  if (w.resetsAt != null) {
    const left = w.resetsAt - now / 1000;
    return left > 0 ? `Resets in ${duration(left)}` : "Resetting now";
  }
  if (w.resetText) return w.resetText;
  return null;
}

export function fmtPct(p: number): string {
  return p > 0 && p < 1 ? "<1" : `${Math.round(p)}`;
}
