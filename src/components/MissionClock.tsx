import { useEffect, useState } from "react";
import type { ProviderSnapshot } from "../lib/types";

/** Ticks once a second while mounted. Only the clock re-renders. */
function useSecond() {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, []);
  return now;
}

type Event = { key: string; label: string; at: number };

/** The upcoming 5-hour resets, one per provider, soonest first. */
function events(providers: ProviderSnapshot[], now: number): Event[] {
  const out: Event[] = [];
  for (const p of providers) {
    const w = p.windows.find((x) => x.id === "five_hour" && x.state !== "reset" && x.resetsAt != null);
    if (w && w.resetsAt! * 1000 > now) out.push({ key: p.id, label: `${p.name} 5H`, at: w.resetsAt! * 1000 });
  }
  return out.sort((a, b) => a.at - b.at);
}

function hms(ms: number) {
  const s = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  return [h, m, sec].map((n) => String(n).padStart(2, "0")).join(":");
}

/**
 * Launch-webcast mission timeline: a hairline with event markers for each
 * provider's next 5-hour reset, "now" as the filled part, and a T− clock
 * counting down to the soonest one.
 */
export function MissionClock({ providers, width = 220 }: { providers: ProviderSnapshot[]; width?: number }) {
  const now = useSecond();
  const ev = events(providers, now);
  if (ev.length === 0) return null;

  const last = ev[ev.length - 1].at;
  // Show a little of the past so "now" sits a short way in, like the stream.
  const span = Math.max(last - now, 60_000) / 0.82;
  const start = now - span * 0.18;
  const x = (t: number) => ((t - start) / span) * 100;
  const next = ev[0];

  return (
    <div className="flex flex-col items-center" style={{ width }} aria-label={`Next reset: ${next.label} in ${hms(next.at - now)}`}>
      <div className="relative w-full" style={{ height: ev.length > 1 ? 30 : 21 }}>
        {/* track, and elapsed part in white */}
        <div className="absolute top-[15px] right-0 left-0 h-px bg-white/25" />
        <div className="absolute top-[15px] left-0 h-px bg-white" style={{ width: `${x(now)}%` }} />
        <div className="absolute top-[12.5px] h-[6px] w-px bg-white" style={{ left: `${x(now)}%` }} />
        {ev.map((e, i) => {
          const left = Math.min(x(e.at), 96);
          // Keep labels inside the panel: anchor them by which side they're on.
          const anchor = left > 72 ? "right-[-3px]" : left < 28 ? "left-[-3px]" : "left-1/2 -translate-x-1/2";
          return (
            <div key={e.key} className="absolute top-[12px] h-[7px] w-[7px] -translate-x-1/2" style={{ left: `${left}%` }}>
              <span className={`block h-[7px] w-[7px] rounded-full border bg-black ${i === 0 ? "border-white" : "border-white/50"}`} />
              {/* Alternate above/below so close events don't collide. */}
              <span
                className={`tm-label absolute whitespace-nowrap text-[6.5px] ${anchor} ${i % 2 === 0 ? "bottom-[11px]" : "top-[11px]"} ${
                  i === 0 ? "text-white" : "text-[var(--tm-dim)]"
                }`}
              >
                {e.label}
              </span>
            </div>
          );
        })}
      </div>
      <div className="mt-[5px] flex items-baseline gap-[5px] leading-none">
        <span className="text-[11px] font-semibold text-[var(--tm-dim)]">T&minus;</span>
        <span className="text-[19px] font-semibold tracking-[0.02em]">{hms(next.at - now)}</span>
      </div>
      <span className="tm-label mt-[4px] text-[6.5px] text-[var(--tm-dim)]">{next.label} reset</span>
    </div>
  );
}
