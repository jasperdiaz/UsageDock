import { useLayoutEffect, useRef } from "react";
import { ago, clockTime, fmtPct, level, resetLine, shown } from "../lib/format";
import { DUR, EASE_GAUGE, gaugeDelay, gsap, reducedMotion } from "../lib/motion";
import type { Settings, UsageWindow } from "../lib/types";

/* ------------------------------------------------------------------ */
/* Dial geometry: 270° sweep, open at the bottom, filling clockwise.  */

const SWEEP = 0.75;
const START = 135; // degrees clockwise from 3 o'clock, i.e. bottom-left

function polar(cx: number, cy: number, r: number, deg: number) {
  const a = (deg * Math.PI) / 180;
  return [cx + r * Math.cos(a), cy + r * Math.sin(a)] as const;
}

function arcPath(c: number, r: number) {
  const [x1, y1] = polar(c, c, r, START);
  const [x2, y2] = polar(c, c, r, START + 360 * SWEEP);
  return `M ${x1} ${y1} A ${r} ${r} 0 1 1 ${x2} ${y2}`;
}

/** White in normal flight; amber from 80%, red from 95%. */
export function arcColor(used: number | null) {
  const l = level(used);
  return l === "crit" ? "var(--tm-red)" : l === "high" ? "var(--tm-amber)" : "var(--tm-ink)";
}

/**
 * When the window resets, in local time: "5:10PM" today, "TUE 9AM" / "SAT 5:30AM"
 * further out. The T− countdown lives in the tooltip.
 */
export function resetClock(w: UsageWindow, now: number): string {
  if (w.state === "reset") return "SYNC";
  if (w.resetsAt == null) return w.limitReached ? "LIMIT" : "";
  const ms = w.resetsAt * 1000;
  if (ms <= now) return "RESET";
  const d = new Date(ms);
  const h12 = d.getHours() % 12 || 12;
  const ap = d.getHours() < 12 ? "AM" : "PM";
  const mins = d.getMinutes();
  const time = mins ? `${h12}:${String(mins).padStart(2, "0")}${ap}` : `${h12}${ap}`;
  const sameDay = new Date(now).toDateString() === d.toDateString();
  if (sameDay || ms - now < 12 * 3600 * 1000) return time;
  const day = d.toLocaleDateString("en-US", { weekday: "short" }).toUpperCase();
  return `${day} ${time}`;
}

/** Terse, consistent dial names across providers. */
export function shortName(w: UsageWindow) {
  if (w.id === "five_hour") return "5H";
  if (w.id === "seven_day" || w.id === "weekly") return "WEEK";
  if (w.id.endsWith("_opus")) return "OPUS";
  if (w.id.endsWith("_sonnet")) return "SONNET";
  return w.shortLabel.toUpperCase().slice(0, 8);
}

export function Gauge({
  w,
  settings,
  now,
  provider,
  size = 52,
}: {
  w: UsageWindow;
  settings: Settings;
  now: number;
  provider: string;
  size?: number;
}) {
  const stroke = size > 56 ? 3 : 2.5;
  const big = size > 56;
  const r = (size - stroke - 6) / 2;
  const c = size / 2;
  const len = 2 * Math.PI * r * SWEEP;
  const { pct } = shown(w, settings.showRemaining);
  const fill = pct == null ? 0 : Math.min(100, Math.max(0, pct)) / 100;
  const color = arcColor(w.usedPercent);
  const hot = w.usedPercent != null && w.usedPercent >= 80;
  // Ticks mark where notifications fire.
  const wrapRef = useRef<HTMLDivElement>(null);
  const trackRef = useRef<SVGPathElement>(null);
  const arcRef = useRef<SVGPathElement>(null);
  const numRef = useRef<HTMLSpanElement>(null);
  // The value currently drawn, in percent; GSAP tweens this and we paint from it.
  const drawn = useRef({ v: 0, track: 0 });
  const introDone = useRef(false);

  // Dial motion: on first show the ring draws on and the value sweeps up from
  // zero (staggered across dials); after that every new reading tweens from
  // the old value, the number rolling with the arc. GSAP owns the dashoffset
  // and the digits, so React renders neither.
  useLayoutEffect(() => {
    const arc = arcRef.current;
    const track = trackRef.current;
    const wrap = wrapRef.current;
    if (!arc || !track || !wrap) return;
    const target = fill * 100;
    const d = drawn.current;
    const paint = (final: boolean) => {
      arc.setAttribute("stroke-dashoffset", String(len * (1 - d.v / 100)));
      track.setAttribute("stroke-dashoffset", String(len * (1 - d.track)));
      if (numRef.current && pct != null) numRef.current.textContent = final ? fmtPct(pct) : String(Math.round(d.v));
    };

    if (reducedMotion()) {
      d.v = target;
      d.track = 1;
      introDone.current = true;
      paint(true);
      return;
    }

    let tl: gsap.core.Timeline;
    if (!introDone.current) {
      d.v = 0;
      d.track = 0;
      paint(false);
      const delay = gaugeDelay(wrap);
      tl = gsap.timeline({ delay, onUpdate: () => paint(false), onComplete: () => { introDone.current = true; paint(true); } });
      tl.to(d, { track: 1, duration: DUR.track, ease: "power2.inOut" }, 0);
      tl.to(d, { v: target, duration: DUR.sweep, ease: EASE_GAUGE }, DUR.track * 0.35);
    } else {
      d.track = 1;
      tl = gsap.timeline({ onUpdate: () => paint(false), onComplete: () => paint(true) });
      tl.to(d, { v: target, duration: DUR.update, ease: EASE_GAUGE });
    }
    return () => {
      tl.kill();
    };
  }, [fill, len, pct]);

  const ticks = settings.notifyThresholds ? settings.thresholds.filter((t) => t > 0 && t < 100) : [];
  const title = [
    `${provider} · ${w.label}`,
    pct == null ? "No current reading" : `${fmtPct(pct)}% ${settings.showRemaining ? "left" : "used"}`,
    resetLine(w, now),
    w.resetsAt != null && w.state !== "reset" ? `at ${clockTime(w.resetsAt * 1000)}` : null,
    `Updated ${ago(w.capturedAt, now)}`,
  ]
    .filter(Boolean)
    .join("\n");

  return (
    <div ref={wrapRef} data-gauge className="flex flex-col items-center" style={{ width: size + 12 }} title={title}>
      <div className="relative" style={{ width: size, height: size }}>
        <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-hidden>
          <path
            ref={trackRef}
            d={arcPath(c, r)}
            fill="none"
            stroke="var(--tm-track)"
            strokeWidth={stroke}
            strokeDasharray={`${len} ${len}`}
          />
          <path
            ref={arcRef}
            className="tm-arc"
            d={arcPath(c, r)}
            fill="none"
            stroke={color}
            strokeWidth={stroke}
            strokeDasharray={`${len} ${len}`}
          />
          {ticks.map((t) => {
            const deg = START + 360 * SWEEP * (settings.showRemaining ? 1 - t / 100 : t / 100);
            const [x1, y1] = polar(c, c, r + stroke / 2 + 0.5, deg);
            const [x2, y2] = polar(c, c, r + stroke / 2 + 2.5, deg);
            return <line key={t} x1={x1} y1={y1} x2={x2} y2={y2} stroke="var(--tm-faint)" strokeWidth={1} />;
          })}
        </svg>
        {/* Webcast gauge readout: name on top, value, unit underneath. */}
        <div className="absolute inset-0 flex flex-col items-center justify-center pt-[3px] leading-none">
          <span className={`tm-label text-[var(--tm-dim)] ${big ? "text-[7.5px]" : "text-[6.5px]"}`}>{shortName(w)}</span>
          <span
            className={`font-semibold ${big ? "mt-[3px] text-[20px]" : "mt-[2px] text-[15px]"} ${pct == null ? "text-[var(--tm-faint)]" : ""}`}
            style={hot ? { color } : undefined}
          >
            {pct == null ? "\u2014" : fmtPct(pct)}
          </span>
          <span className={`tm-label text-[var(--tm-dim)] ${big ? "mt-[3px] text-[7px]" : "mt-[2px] text-[6px]"}`}>
            {pct == null ? "\u00a0" : settings.showRemaining ? "% LEFT" : "%"}
          </span>
        </div>
      </div>
      <span className="mt-[2px] whitespace-nowrap text-[9.5px] leading-none font-medium tracking-[0.02em] text-white/85">
        {resetClock(w, now) || "\u00a0"}
      </span>
    </div>
  );
}
