// Mirrors the Rust structs (serde camelCase).

export type WindowState = "live" | "stale" | "reset";
export type ProviderStatus = "live" | "stale" | "waiting" | "unavailable" | "demo";
export type ProviderId = "claude" | "chatgpt" | "codex";

export interface UsageWindow {
  id: string;
  label: string;
  shortLabel: string;
  usedPercent: number | null;
  /** Unix epoch seconds. */
  resetsAt: number | null;
  resetText: string | null;
  limitReached: boolean;
  source: string;
  /** Unix epoch milliseconds. */
  capturedAt: number;
  state: WindowState;
}

export interface SourceInfo {
  id: string;
  label: string;
  connected: boolean;
  lastSeen: number | null;
  detail: string;
}

export interface ProviderSnapshot {
  id: ProviderId;
  name: string;
  usageUrl: string;
  status: ProviderStatus;
  detail: string;
  plan: string | null;
  windows: UsageWindow[];
  lastUpdated: number | null;
  sources: SourceInfo[];
}

export interface IntegrationStatus {
  bridgeFound: boolean;
  claudeCodeFound: boolean;
  claudeStatuslineInstalled: boolean;
  claudeStatuslinePrevious: string | null;
  chromeHostRegistered: boolean;
  extensionDir: string;
  extensionId: string;
  sourcesDir: string;
}

export interface UsagePayload {
  snapshots: ProviderSnapshot[];
  integrations: IntegrationStatus;
  generatedAt: number;
}

export interface Settings {
  startWithWindows: boolean;
  showInTray: boolean;
  startCompact: boolean;
  alwaysOnTop: boolean;
  refreshIntervalSecs: number;
  notifyThresholds: boolean;
  thresholds: number[];
  notifyOnReset: boolean;
  visibleProviders: Record<string, boolean>;
  opacity: number;
  showRemaining: boolean;
  demoMode: boolean;
  widgetCollapsed: boolean;
  widgetProvider: string;
  staleAfterMins: number;
  /** Screen anchor ("bottom-right", "top-center", …) or "free". */
  dock: Dock;
  widgetLayout: "auto" | "stacked" | "strip";
  widgetShowNames: boolean;
  widgetGlow: boolean;
  widgetGlowLevel: number;
  widgetPosition: [number, number] | null;
  widgetSize: [number, number] | null;
  dashboardPosition: [number, number] | null;
}

export type Dock =
  | "top-left" | "top-center" | "top-right"
  | "left-center" | "right-center"
  | "bottom-left" | "bottom-center" | "bottom-right"
  | "free";

export interface FullState {
  settings: Settings;
  usage: UsagePayload;
  version: string;
}
