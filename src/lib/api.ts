import { invoke } from "@tauri-apps/api/core";
import type { FullState, IntegrationStatus, Settings, UsagePayload } from "./types";

export const api = {
  getState: () => invoke<FullState>("get_state"),
  updateSettings: (settings: Settings) => invoke<Settings>("update_settings", { settings }),
  refresh: () => invoke<UsagePayload>("refresh"),
  showDashboard: (tab?: string) => invoke<void>("show_dashboard", { tab }),
  showWidget: () => invoke<void>("show_widget"),
  hideWindow: (label: string) => invoke<void>("hide_window", { label }),
  setWidgetCollapsed: (collapsed: boolean) => invoke<void>("set_widget_collapsed", { collapsed }),
  fitWidget: (width: number, height: number, inset = 0) => invoke<void>("fit_widget", { width, height, inset }),
  finishLaunch: (animate: boolean) => invoke<void>("finish_launch", { animate }),
  setDock: (dock: string) => invoke<void>("set_dock", { dock }),
  resetWindowPositions: () => invoke<void>("reset_window_positions"),
  connectClaudeCode: () => invoke<IntegrationStatus>("connect_claude_code"),
  disconnectClaudeCode: () => invoke<IntegrationStatus>("disconnect_claude_code"),
  registerBrowserHost: () => invoke<IntegrationStatus>("register_browser_host"),
  unregisterBrowserHost: () => invoke<IntegrationStatus>("unregister_browser_host"),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  openFolder: (which: "extension" | "sources") => invoke<void>("open_folder", { which }),
  quit: () => invoke<void>("quit"),
};
