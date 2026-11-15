import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import type { Settings, UsagePayload } from "./types";

/** Shared state for both windows: settings, usage snapshots, and a clock. */
export function useAppState() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [usage, setUsage] = useState<UsagePayload | null>(null);
  const [version, setVersion] = useState("");

  useEffect(() => {
    let alive = true;
    api.getState().then((s) => {
      if (!alive) return;
      setSettings(s.settings);
      setUsage(s.usage);
      setVersion(s.version);
    });
    const unsubs = [
      listen<UsagePayload>("usage://updated", (e) => setUsage(e.payload)),
      listen<Settings>("settings://changed", (e) => setSettings(e.payload)),
    ];
    return () => {
      alive = false;
      unsubs.forEach((p) => p.then((f) => f()));
    };
  }, []);

  useEffect(() => {
    if (settings) document.documentElement.style.setProperty("--surface-alpha", String(settings.opacity));
  }, [settings?.opacity]);

  const update = useCallback(
    async (patch: Partial<Settings>) => {
      if (!settings) return;
      const optimistic = { ...settings, ...patch };
      setSettings(optimistic);
      setSettings(await api.updateSettings(optimistic));
    },
    [settings],
  );

  return { settings, usage, setUsage, version, update };
}

/** Re-render on an interval so countdowns move. Minute precision is enough. */
export function useNow(intervalMs = 15_000) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), intervalMs);
    return () => window.clearInterval(id);
  }, [intervalMs]);
  return now;
}
