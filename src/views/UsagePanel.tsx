import { api } from "../lib/api";
import { ago } from "../lib/format";
import type { ProviderSnapshot, ProviderStatus, Settings, UsagePayload } from "../lib/types";
import { IconExternal } from "../components/Icons";
import { ProviderIcon } from "../components/ProviderIcon";
import { Gauge } from "../components/Telemetry";
import { MissionClock } from "../components/MissionClock";

export function UsagePanel({
  usage,
  settings,
  now,
  onSetup,
}: {
  usage: UsagePayload;
  settings: Settings;
  now: number;
  onSetup: () => void;
}) {
  const list = usage.snapshots.filter((p) => settings.visibleProviders[p.id] ?? true);
  if (list.length === 0) {
    return <p className="pt-6 text-center text-[12px] text-ink-3">No providers selected. Turn some on in Settings.</p>;
  }
  return (
    <div className="flex flex-col">
      {settings.demoMode && (
        <p className="mb-2 border-l-2 border-[var(--tm-amber)] py-1 pl-2.5 text-[11.5px] leading-snug text-ink-2">
          Demo mode is on. These are sample values, not your usage.
        </p>
      )}
      {list.some((p) => p.windows.some((w) => w.id === "five_hour" && w.resetsAt != null && w.state !== "reset")) && (
        <div data-intro className="mb-2 flex justify-center border-b border-line pt-1 pb-3">
          <MissionClock providers={list} width={300} />
        </div>
      )}
      {list.map((p, i) => (
        <Provider key={p.id} p={p} settings={settings} now={now} onSetup={onSetup} first={i === 0} />
      ))}
    </div>
  );
}

const STATUS: Record<ProviderStatus, { text: string; color: string }> = {
  live: { text: "Live", color: "var(--tm-ink)" },
  stale: { text: "Stale", color: "var(--tm-amber)" },
  waiting: { text: "Waiting", color: "var(--tm-faint)" },
  unavailable: { text: "No meter", color: "var(--tm-faint)" },
  demo: { text: "Demo", color: "var(--tm-amber)" },
};

function Provider({
  p,
  settings,
  now,
  onSetup,
  first,
}: {
  p: ProviderSnapshot;
  settings: Settings;
  now: number;
  onSetup: () => void;
  first: boolean;
}) {
  const noSource = p.sources.every((s) => !s.connected);
  const st = STATUS[p.status];
  // Which sources fed the numbers, e.g. "Claude Code" or "Claude Code + claude.ai".
  const sources = [...new Set(p.windows.map((w) => w.source))].join(" + ");

  return (
    <article className={`py-3 ${first ? "pt-1" : "border-t border-line"}`}>
      <header data-intro className="flex items-center gap-2">
        <ProviderIcon id={p.id} size={16} />
        <h2 className="tm-label text-[11px] text-ink">{p.name}</h2>
        {p.plan && <span className="tm-label border border-line-2 px-1 py-[2px] text-[8px] text-ink-2">{p.plan}</span>}
        <span className="tm-label ml-auto flex items-center gap-1.5 text-[9px]" style={{ color: st.color }}>
          <span
            aria-hidden
            className="h-[5px] w-[5px] rounded-full"
            style={{ background: st.color, boxShadow: p.status === "live" ? "0 0 6px rgb(255 255 255 / 0.7)" : undefined }}
          />
          {st.text}
        </span>
      </header>

      {p.windows.length > 0 ? (
        <div className={`mt-2.5 flex flex-wrap gap-x-1 gap-y-3 ${p.status === "stale" ? "opacity-70" : ""}`}>
          {p.windows.map((w) => (
            <Gauge key={w.id} w={w} settings={settings} now={now} provider={p.name} size={62} />
          ))}
        </div>
      ) : (
        <p className="mt-2 text-[12px] leading-snug text-ink-2">
          {p.detail}
          {p.id === "claude" && noSource && (
            <button className="ml-1 font-semibold text-ink underline underline-offset-2" onClick={onSetup}>
              Connect a source
            </button>
          )}
        </p>
      )}

      <footer className="mt-2.5 flex items-center justify-between gap-2 text-[10.5px] text-ink-3">
        <span className="truncate">
          {p.lastUpdated ? `${sources ? `${sources} · ` : ""}updated ${ago(p.lastUpdated, now)}` : "No data received yet"}
        </span>
        <button
          className="tm-label flex shrink-0 items-center gap-1 px-1 py-0.5 text-[9px] text-ink-2 hover:text-ink"
          onClick={() => api.openUrl(p.usageUrl)}
        >
          {p.id === "chatgpt" ? "Open ChatGPT" : "Usage page"} <IconExternal />
        </button>
      </footer>
    </article>
  );
}
