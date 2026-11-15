import type { ReactNode } from "react";
import { api } from "../lib/api";
import type { Dock, Settings } from "../lib/types";
import { Toggle } from "../components/Toggle";
import { ProviderIcon } from "../components/ProviderIcon";

const INTERVALS = [
  [15, "15 s"],
  [30, "30 s"],
  [60, "1 min"],
  [120, "2 min"],
  [300, "5 min"],
] as const;
const STALE = [
  [15, "15 min"],
  [30, "30 min"],
  [60, "1 hour"],
  [180, "3 hours"],
] as const;
const THRESHOLDS = [50, 70, 80, 90, 95, 100];
const GLOW_LEVELS = [
  [0.35, "Subtle"],
  [0.65, "Medium"],
  [1, "Strong"],
] as const;
const PROVIDERS = [
  ["claude", "Claude"],
  ["chatgpt", "ChatGPT"],
  ["codex", "Codex"],
] as const;

export function SettingsPanel({
  settings: s,
  update,
  version,
}: {
  settings: Settings;
  update: (patch: Partial<Settings>) => void;
  version: string;
}) {
  const toggleThreshold = (t: number) => {
    const next = s.thresholds.includes(t) ? s.thresholds.filter((x) => x !== t) : [...s.thresholds, t];
    update({ thresholds: next.sort((a, b) => a - b) });
  };

  return (
    <div className="flex flex-col gap-3">
      <Group title="General">
        <Row label="Start with Windows">
          <Toggle label="Start with Windows" on={s.startWithWindows} onChange={(v) => update({ startWithWindows: v })} />
        </Row>
        <Row label="Show in system tray" hint={!s.showInTray ? "Closing the last window will quit." : undefined}>
          <Toggle label="Show in system tray" on={s.showInTray} onChange={(v) => update({ showInTray: v })} />
        </Row>
        <Row label="Start in compact mode">
          <Toggle label="Start in compact mode" on={s.startCompact} onChange={(v) => update({ startCompact: v })} />
        </Row>
        <Row label="Always on top">
          <Toggle label="Always on top" on={s.alwaysOnTop} onChange={(v) => update({ alwaysOnTop: v })} />
        </Row>
      </Group>

      <Group title="Widget">
        <Row label="Dock" hint={s.dock === "free" ? "Free: stays where you drop it. Drag near an edge spot to dock." : "Drag the widget near any spot to re-dock."}>
          <DockPicker value={s.dock} onChange={(d) => api.setDock(d)} />
        </Row>
        <Row label="Layout">
          <Select
            value={s.widgetLayout}
            options={[["auto", "Auto"], ["stacked", "Stacked"], ["strip", "Strip"]] as const}
            onChange={(v) => update({ widgetLayout: v })}
            label="Widget layout"
          />
        </Row>
      </Group>

      <Group title="Display">
        <Row label="Glow edge" hint="A soft halo and lit top edge so the widget stands out on dark wallpapers.">
          <Toggle label="Glow edge" on={s.widgetGlow} onChange={(v) => update({ widgetGlow: v })} />
        </Row>
        {s.widgetGlow && (
          <Row label="Glow intensity">
            <div className="flex gap-1" role="radiogroup" aria-label="Glow intensity">
              {GLOW_LEVELS.map(([v, name]) => {
                const on = Math.abs(s.widgetGlowLevel - v) < 0.01;
                return (
                  <button
                    key={name}
                    role="radio"
                    aria-checked={on}
                    onClick={() => update({ widgetGlowLevel: v })}
                    className={`rounded-full px-2.5 py-0.5 text-[11px] transition-colors duration-150 ${
                      on ? "bg-white font-semibold text-black" : "border border-white/20 text-ink-3 hover:text-ink-2"
                    }`}
                  >
                    {name}
                  </button>
                );
              })}
            </div>
          </Row>
        )}
        <Row label="Show provider names" hint={s.widgetShowNames ? undefined : "Widget shows logos only, sized like a dial."}>
          <Toggle label="Show provider names" on={s.widgetShowNames} onChange={(v) => update({ widgetShowNames: v })} />
        </Row>
        <Row label="Show remaining instead of used">
          <Toggle label="Show remaining" on={s.showRemaining} onChange={(v) => update({ showRemaining: v })} />
        </Row>
        <Row label="Widget opacity">
          <div className="flex items-center gap-2">
            <input
              type="range"
              min={40}
              max={100}
              step={2}
              value={Math.round(s.opacity * 100)}
              onChange={(e) => update({ opacity: Number(e.target.value) / 100 })}
              aria-label="Widget opacity"
              className="w-24 accent-white"
            />
            <span className="tnum w-8 text-right text-[11px] text-ink-3">{Math.round(s.opacity * 100)}%</span>
          </div>
        </Row>
        <Row label="Providers">
          <div className="flex gap-1">
            {PROVIDERS.map(([id, name]) => {
              const on = s.visibleProviders[id] ?? true;
              return (
                <button
                  key={id}
                  aria-pressed={on}
                  title={name}
                  onClick={() => update({ visibleProviders: { ...s.visibleProviders, [id]: !on } })}
                  className={`flex items-center gap-1 rounded-[4px] border px-1.5 py-1 text-[11px] transition-[opacity,border-color] duration-150 ${
                    on ? "border-white/40 text-ink" : "border-transparent text-ink-3 opacity-60 hover:opacity-100"
                  }`}
                >
                  <ProviderIcon id={id} size={14} />
                  {name}
                </button>
              );
            })}
          </div>
        </Row>
        <Row label="Window positions">
          <button className="rounded-[4px] border border-white/25 px-2.5 py-1 text-[10px] font-semibold tracking-[0.1em] uppercase hover:border-white/60" onClick={() => api.resetWindowPositions()}>
            Reset
          </button>
        </Row>
      </Group>

      <Group title="Data">
        <Row label="Re-scan sources every">
          <Select value={s.refreshIntervalSecs} options={INTERVALS} onChange={(v) => update({ refreshIntervalSecs: v })} label="Refresh interval" />
        </Row>
        <Row label="Mark data stale after">
          <Select value={s.staleAfterMins} options={STALE} onChange={(v) => update({ staleAfterMins: v })} label="Stale after" />
        </Row>
        <Row label="Demo mode" hint="Sample values for trying the UI. Always labelled.">
          <Toggle label="Demo mode" on={s.demoMode} onChange={(v) => update({ demoMode: v })} />
        </Row>
      </Group>

      <Group title="Notifications">
        <Row label="Usage thresholds">
          <Toggle label="Threshold notifications" on={s.notifyThresholds} onChange={(v) => update({ notifyThresholds: v })} />
        </Row>
        <div className={`flex flex-wrap gap-1 px-3 pb-2.5 ${s.notifyThresholds ? "" : "pointer-events-none opacity-40"}`}>
          {THRESHOLDS.map((t) => {
            const on = s.thresholds.includes(t);
            return (
              <button
                key={t}
                aria-pressed={on}
                onClick={() => toggleThreshold(t)}
                className={`tnum rounded-full px-2.5 py-0.5 text-[11px] transition-colors duration-150 ${
                  on ? "bg-white font-semibold text-black" : "border border-white/20 text-ink-3 hover:text-ink-2"
                }`}
              >
                {t}%
              </button>
            );
          })}
        </div>
        <Row label="When a limit resets">
          <Toggle label="Notify on reset" on={s.notifyOnReset} onChange={(v) => update({ notifyOnReset: v })} />
        </Row>
      </Group>

      <div className="flex items-center justify-between px-1 text-[10.5px] text-ink-3">
        <span>
          UsageDock {version} · by <span className="font-medium text-[var(--tm-signature)]">Jasper Ted Mina Diaz</span>
        </span>
        <button className="rounded px-1.5 py-0.5 hover:bg-white/6 hover:text-[var(--color-crit)]" onClick={() => api.quit()}>
          Quit UsageDock
        </button>
      </div>
    </div>
  );
}

function Group({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section>
      <h3 className="tm-label mb-1.5 px-0.5 text-[9.5px] text-ink-3">{title}</h3>
      <div className="divide-y divide-line rounded-[6px] border border-line">{children}</div>
    </section>
  );
}

function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="flex min-h-[38px] items-center justify-between gap-3 px-3 py-1.5">
      <div className="min-w-0">
        <div className="text-[12px] text-ink">{label}</div>
        {hint && <div className="text-[10.5px] leading-snug text-ink-3">{hint}</div>}
      </div>
      {children}
    </div>
  );
}

const SPOTS: { id: Dock; x: number; y: number; w: number; h: number }[] = [
  { id: "top-left", x: 3, y: 3, w: 10, h: 7 },
  { id: "top-center", x: 22, y: 3, w: 20, h: 5 },
  { id: "top-right", x: 51, y: 3, w: 10, h: 7 },
  { id: "left-center", x: 3, y: 16.5, w: 10, h: 7 },
  { id: "right-center", x: 51, y: 16.5, w: 10, h: 7 },
  { id: "bottom-left", x: 3, y: 30, w: 10, h: 7 },
  { id: "bottom-center", x: 22, y: 32, w: 20, h: 5 },
  { id: "bottom-right", x: 51, y: 30, w: 10, h: 7 },
];

/** A tiny screen: click where the widget should live. */
function DockPicker({ value, onChange }: { value: Dock; onChange: (d: Dock) => void }) {
  return (
    <svg width={96} height={60} viewBox="0 0 64 40" role="radiogroup" aria-label="Dock position" className="shrink-0">
      <rect x={0.5} y={0.5} width={63} height={39} rx={3} fill="rgb(255 255 255 / 0.03)" stroke="rgb(255 255 255 / 0.18)" />
      {SPOTS.map((p) => {
        const on = value === p.id;
        return (
          <rect
            key={p.id}
            role="radio"
            aria-checked={on}
            aria-label={p.id.replace("-", " ")}
            tabIndex={0}
            x={p.x}
            y={p.y}
            width={p.w}
            height={p.h}
            rx={1.2}
            className="cursor-pointer outline-none"
            fill={on ? "#ffffff" : "rgb(255 255 255 / 0.12)"}
            onClick={() => onChange(p.id)}
            onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && onChange(p.id)}
          >
            <title>{p.id.replace("-", " ")}</title>
          </rect>
        );
      })}
    </svg>
  );
}

function Select<T extends number | string>({
  value,
  options,
  onChange,
  label,
}: {
  value: T;
  options: readonly (readonly [T, string])[];
  onChange: (v: T) => void;
  label: string;
}) {
  return (
    <select
      aria-label={label}
      value={value}
      onChange={(e) => onChange((typeof value === "number" ? Number(e.target.value) : e.target.value) as T)}
      className="rounded-[4px] border border-line-2 bg-black px-2 py-1 text-[11.5px] text-ink outline-none focus-visible:border-white"
    >
      {options.map(([v, l]) => (
        <option key={v} value={v}>
          {l}
        </option>
      ))}
    </select>
  );
}
