import { Fragment, useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type MouseEvent } from "react";
import { introPanel, reducedMotion } from "../lib/motion";
import { Splash, SPLASH_MS } from "../components/Splash";
import { MissionClock } from "../components/MissionClock";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api } from "../lib/api";
import { ago, fmtPct, resetLine, shown } from "../lib/format";
import { arcColor, Gauge, shortName } from "../components/Telemetry";
import type { ProviderSnapshot, Settings } from "../lib/types";
import { useAppState, useNow } from "../lib/useAppState";
import { IconChevron, IconExpand, IconMinus } from "../components/Icons";
import { ProviderIcon } from "../components/ProviderIcon";

/** Drag the frameless window from anywhere that isn't a control. */
function dragFromBackground(e: MouseEvent) {
  if (e.button !== 0) return;
  if ((e.target as HTMLElement).closest("button, a, input, [data-nodrag]")) return;
  getCurrentWindow().startDragging();
}

type Layout = "stacked" | "strip";

/** Transparent margin (px) kept around the panel when the glow is on. */
const GLOW_ROOM = 22;
/** Transparent margin (px) around the launch splash for its halo and shadow. */
const SPLASH_ROOM = 44;


function layoutFor(s: Settings): Layout {
  if (s.widgetLayout === "strip") return "strip";
  if (s.widgetLayout === "stacked") return "stacked";
  return s.dock === "top-center" || s.dock === "bottom-center" ? "strip" : "stacked";
}

export default function Widget() {
  const { settings, usage } = useAppState();
  const now = useNow();
  const root = useRef<HTMLDivElement>(null);
  const panel = useRef<HTMLDivElement>(null);

  // Only providers that have something to show; the widget shrinks to fit them.
  const providers = useMemo(
    () => (usage?.snapshots ?? []).filter((p) => (settings?.visibleProviders[p.id] ?? true) && p.windows.length > 0),
    [usage, settings?.visibleProviders],
  );
  const ready = !!settings && !!usage;
  // Room around the panel for the glow, so the window doesn't clip it.
  const inset = settings?.widgetGlow ? GLOW_ROOM : 0;
  const insetRef = useRef(inset);
  insetRef.current = inset;

  // Size the native window to whatever the instruments need.
  useLayoutEffect(() => {
    const el = root.current;
    if (!el) return;
    let last = "";
    const report = () => {
      const w = Math.ceil(el.offsetWidth);
      const h = Math.ceil(el.offsetHeight);
      const key = `${w}x${h}@${insetRef.current}`;
      if (key === last || w === 0) return;
      last = key;
      api.fitWidget(w, h, insetRef.current);
    };
    const ro = new ResizeObserver(report);
    // border-box so padding changes (the glow room) also trigger a re-measure.
    ro.observe(el, { box: "border-box" });
    report();
    // Webfont metrics differ from the fallback; measure again once loaded.
    document.fonts?.ready.then(report);
    return () => ro.disconnect();
  }, [ready, inset]);

  // Launch sequence: splash centred on screen ("boot") for at least SPLASH_MS
  // and until data arrives, then it signs off ("leaving") and the panel takes
  // over ("live") while the backend glides the window to its spot.
  const [phase, setPhase] = useState<"boot" | "leaving" | "live">("boot");
  const [held, setHeld] = useState(false);
  useEffect(() => {
    const t = window.setTimeout(() => setHeld(true), SPLASH_MS);
    return () => window.clearTimeout(t);
  }, []);
  useEffect(() => {
    if (ready && held && phase === "boot") setPhase("leaving");
  }, [ready, held, phase]);

  // Panel intro (panel fades up, stage marks and clock follow, dials sweep in
  // from Gauge), then size the window to the panel and start the glide.
  useLayoutEffect(() => {
    if (phase !== "live") return;
    const tl = introPanel(panel.current);
    const el = root.current;
    const raf = requestAnimationFrame(() => {
      const fit = el ? api.fitWidget(Math.ceil(el.offsetWidth), Math.ceil(el.offsetHeight), insetRef.current) : Promise.resolve();
      fit.finally(() => api.finishLaunch(!reducedMotion()));
    });
    return () => {
      cancelAnimationFrame(raf);
      tl?.kill();
    };
  }, [phase]);

  if (!settings || !usage || phase !== "live") {
    return (
      // Fixed room so the splash's halo and shadow aren't clipped by the window.
      <div ref={root} className="absolute top-0 left-0 w-max" style={{ padding: SPLASH_ROOM }}>
        <Splash leaving={phase === "leaving"} onDone={() => setPhase("live")} />
      </div>
    );
  }

  const collapsed = settings.widgetCollapsed;
  const layout = layoutFor(settings);
  const toggleCollapsed = () => api.setWidgetCollapsed(!collapsed);

  return (
    <div
      ref={root}
      className="absolute top-0 left-0 w-max"
      style={{ padding: inset }}
      onMouseDown={dragFromBackground}
      onDoubleClick={(e) => !(e.target as HTMLElement).closest("button") && toggleCollapsed()}
    >
    <div
      ref={panel}
      className={`telemetry relative ${settings.widgetGlow ? "tm-glow" : ""}`}
      style={{ "--glow": settings.widgetGlowLevel } as CSSProperties}
    >
      {providers.length === 0 ? (
        <NoData />
      ) : (
        <div className={`flex ${layout === "strip" ? "flex-row" : "flex-col"}`}>
          {providers.map((p, i) => (
            <Fragment key={p.id}>
              {/* Strip: the mission clock sits after the first stage, centre-screen like the webcast. */}
              {layout === "strip" && !collapsed && i === 1 && (
                <div data-intro className="flex items-center border-l border-white/10 px-3">
                  <MissionClock providers={providers} width={190} />
                </div>
              )}
              <div className={i === 0 ? "" : layout === "strip" ? "border-l border-white/10" : "border-t border-white/10"}>
                {collapsed ? <Readout p={p} settings={settings} now={now} /> : <Stage p={p} settings={settings} now={now} mirror={layout === "strip" && providers.length > 1 && i === providers.length - 1} />}
              </div>
            </Fragment>
          ))}
          {layout === "strip" && !collapsed && providers.length === 1 && (
            <div data-intro className="flex items-center border-l border-white/10 px-3">
              <MissionClock providers={providers} width={190} />
            </div>
          )}
        </div>
      )}

      <div className="tm-controls absolute top-1 right-1 flex gap-0.5 rounded-[5px] bg-black/75">
        <button
          className="tm-btn"
          aria-label={collapsed ? "Show gauges" : "Collapse to numbers"}
          title={collapsed ? "Show gauges" : "Collapse"}
          onClick={toggleCollapsed}
        >
          {collapsed ? <IconChevron dir="down" width={11} height={11} /> : <IconMinus width={11} height={11} />}
        </button>
        <button className="tm-btn" aria-label="Open dashboard" title="Open dashboard" onClick={() => api.showDashboard("usage")}>
          <IconExpand width={11} height={11} />
        </button>
      </div>
    </div>
    </div>
  );
}

/** A provider is one "stage": its mark and name, then its dials. */
/**
 * A provider is one "stage": its mark and name, then its dials. `mirror` puts
 * the mark on the far right, so a centred strip is symmetrical around the clock.
 */
function Stage({ p, settings, now, mirror = false }: { p: ProviderSnapshot; settings: Settings; now: number; mirror?: boolean }) {
  const id = <StageId p={p} now={now} logoOnly={!settings.widgetShowNames} />;
  return (
    <section
      className={`flex items-center gap-0.5 py-2 ${mirror ? "pr-2 pl-1.5" : "pr-1.5 pl-2"} ${p.status === "stale" ? "opacity-60" : ""}`}
      aria-label={p.name}
    >
      {!mirror && id}
      {p.windows.slice(0, 3).map((w) => (
        <Gauge key={w.id} w={w} settings={settings} now={now} provider={p.name} />
      ))}
      {mirror && <div className="ml-0.5">{id}</div>}
    </section>
  );
}

function StageId({ p, now, logoOnly }: { p: ProviderSnapshot; now: number; logoOnly: boolean }) {
  // Only demo is labelled here; stale providers are dimmed and the age is in the tooltip.
  const note = p.status === "demo" ? "DEMO" : null;
  if (logoOnly) {
    // Same footprint as a dial (52px ring + the reset-time row) so the logo
    // lines up with the rings beside it.
    return (
      <div data-intro className="flex w-[60px] flex-col items-center" title={`${p.name}: ${p.detail}${p.lastUpdated ? `
Updated ${ago(p.lastUpdated, now)}` : ""}`} aria-label={p.name}>
        <div className="grid h-[52px] w-[52px] place-items-center">
          <ProviderIcon id={p.id} size={30} />
        </div>
        <span
          className={`tm-label mt-[2px] h-[10px] text-[7px] leading-[10px] ${p.status === "demo" ? "text-[var(--tm-amber)]" : "text-[var(--tm-dim)]"}`}
        >
          {note ?? " "}
        </span>
      </div>
    );
  }
  return (
    <div data-intro className="mr-0.5 flex w-[42px] flex-col items-center gap-[6px]" title={`${p.name}: ${p.detail}${p.lastUpdated ? `
Updated ${ago(p.lastUpdated, now)}` : ""}`}>
      <ProviderIcon id={p.id} size={15} />
      <span className="tm-label text-[7.5px] text-white">{p.name}</span>
      {note && <span className={`tm-label text-[7px] ${p.status === "demo" ? "text-[var(--tm-amber)]" : "text-[var(--tm-dim)]"}`}>{note}</span>}
    </div>
  );
}

/** Collapsed: numbers only. */
function Readout({ p, settings, now }: { p: ProviderSnapshot; settings: Settings; now: number }) {
  return (
    <section className={`flex items-center gap-2.5 py-[7px] pr-3 pl-2.5 ${p.status === "stale" ? "opacity-60" : ""}`} aria-label={p.name}>
      <ProviderIcon id={p.id} size={13} />
      {p.windows.slice(0, 3).map((w) => {
        const { pct } = shown(w, settings.showRemaining);
        return (
          <span key={w.id} className="flex w-[44px] items-baseline gap-[3px] leading-none" title={`${w.label}\n${resetLine(w, now) ?? ""}`}>
            <span className="text-[13px] font-semibold" style={{ color: arcColor(w.usedPercent) }}>
              {pct == null ? "—" : fmtPct(pct)}
            </span>
            <span className="tm-label text-[7.5px] text-[var(--tm-dim)]">{shortName(w)}</span>
          </span>
        );
      })}
    </section>
  );
}

function NoData() {
  return (
    <div className="flex items-center gap-3 py-2.5 pr-12 pl-3">
      <div className="flex flex-col gap-1">
        <span className="tm-label text-white">No signal</span>
        <span className="text-[11px] leading-tight text-[var(--tm-dim)]">Connect a source to see usage.</span>
      </div>
      <button
        className="rounded-[5px] border border-white/20 px-2 py-1 text-[10.5px] font-medium tracking-[0.06em] uppercase hover:bg-white/10"
        onClick={() => api.showDashboard("sources")}
      >
        Set up
      </button>
    </div>
  );
}
