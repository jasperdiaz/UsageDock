import { useEffect, useLayoutEffect, useRef, useState, type MouseEvent } from "react";
import { introPanel } from "../lib/motion";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { api } from "../lib/api";
import { useAppState, useNow } from "../lib/useAppState";
import { IconClose, IconRefresh } from "../components/Icons";
import { Tabs } from "../components/Tabs";
import { UsagePanel } from "./UsagePanel";
import { SourcesPanel } from "./SourcesPanel";
import { SettingsPanel } from "./SettingsPanel";

type Tab = "usage" | "sources" | "settings";
const isTab = (t: unknown): t is Tab => t === "usage" || t === "sources" || t === "settings";
// The backend opens the dashboard with ?tab=… the first time it's created.
const initialTab: Tab = (() => {
  const t = new URLSearchParams(location.search).get("tab");
  return isTab(t) ? t : "usage";
})();

function drag(e: MouseEvent) {
  if (e.button !== 0 || (e.target as HTMLElement).closest("button, a, input, select, [data-nodrag]")) return;
  getCurrentWindow().startDragging();
}

export default function Dashboard() {
  const state = useAppState();
  const { settings, usage, setUsage, version } = state;
  const now = useNow();
  const [tab, setTab] = useState<Tab>(initialTab);
  const [spinning, setSpinning] = useState(false);
  const surface = useRef<HTMLDivElement>(null);
  const ready = !!settings && !!usage;

  // Same launch intro as the widget, once data first arrives.
  useLayoutEffect(() => {
    if (!ready) return;
    const tl = introPanel(surface.current);
    return () => {
      tl?.kill();
    };
  }, [ready]);

  useEffect(() => {
    const un = listen<string>("dashboard://tab", (e) => {
      if (isTab(e.payload)) setTab(e.payload);
    });
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") api.hideWindow("dashboard");
    };
    window.addEventListener("keydown", onKey);
    return () => {
      un.then((f) => f());
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  const refresh = async () => {
    setSpinning(true);
    setUsage(await api.refresh());
    window.setTimeout(() => setSpinning(false), 700);
  };

  return (
    <div ref={surface} className="surface flex h-full flex-col overflow-hidden rounded-[10px]">
      <header data-intro className="flex items-center gap-2 px-3.5 pt-3 pb-2" onMouseDown={drag}>
        <svg aria-hidden width={15} height={15} viewBox="16 28 224 200" className="shrink-0">
          {/* UsageDock mark (brand/final2/mark.svg) */}
          <path d="M 48.80 219.20 A 112 112 0 0 1 112.00 29.15 L 112.00 86.33 A 56 56 0 0 0 88.40 179.60 Z" fill="#fff" />
          <path d="M 144.00 29.15 A 112 112 0 0 1 207.20 219.20 L 167.60 179.60 A 56 56 0 0 0 144.00 86.33 Z" fill="#fff" />
        </svg>
        <span className="tm-label text-[11.5px] tracking-[0.22em] text-ink">UsageDock</span>
        {settings?.demoMode && (
          <span className="tm-label border border-[var(--tm-amber)] px-1 py-[2px] text-[8px] text-[var(--tm-amber)]">
            DEMO
          </span>
        )}
        <div className="ml-auto flex items-center gap-0.5">
          <button className="icon-btn" aria-label="Refresh" title="Re-read sources" onClick={refresh}>
            <IconRefresh className={spinning ? "spin" : undefined} />
          </button>
          <button className="icon-btn" aria-label="Close" title="Hide to tray" onClick={() => api.hideWindow("dashboard")}>
            <IconClose />
          </button>
        </div>
      </header>

      <div data-intro className="mb-3 border-b border-line px-3.5" onMouseDown={drag}>
        <Tabs<Tab>
          value={tab}
          onChange={setTab}
          tabs={[
            { id: "usage", label: "Usage" },
            { id: "sources", label: "Sources" },
            { id: "settings", label: "Settings" },
          ]}
        />
      </div>

      <div className="scroll min-h-0 flex-1 px-3.5 pb-3.5">
        {!settings || !usage ? null : tab === "usage" ? (
          <UsagePanel usage={usage} settings={settings} now={now} onSetup={() => setTab("sources")} />
        ) : tab === "sources" ? (
          <SourcesPanel usage={usage} now={now} />
        ) : (
          <SettingsPanel settings={settings} update={state.update} version={version} />
        )}
      </div>
    </div>
  );
}
