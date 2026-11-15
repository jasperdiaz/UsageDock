import { useState, type ReactNode } from "react";
import { api } from "../lib/api";
import { ago } from "../lib/format";
import type { UsagePayload } from "../lib/types";
import { ProviderIcon } from "../components/ProviderIcon";

export function SourcesPanel({ usage, now }: { usage: UsagePayload; now: number }) {
  const i = usage.integrations;
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = async (key: string, fn: () => Promise<unknown>) => {
    setBusy(key);
    setError(null);
    try {
      await fn();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  const seen = (provider: string, source: string) =>
    usage.snapshots.find((p) => p.id === provider)?.sources.find((s) => s.id === source)?.lastSeen ?? null;
  const codex = usage.snapshots.find((p) => p.id === "codex")?.sources.find((s) => s.id === "codex-logs");

  return (
    <div className="flex flex-col text-[12px]">
      {error && (
        <p role="alert" className="mb-2 border-l-2 border-[var(--tm-red)] py-1 pl-2.5 text-[11.5px] text-ink">
          {error}
        </p>
      )}

      <Card
        icon={<ProviderIcon id="claude" size={18} />}
        title="Claude Code status line"
        badge={i.claudeStatuslineInstalled ? "Connected" : "Not connected"}
        on={i.claudeStatuslineInstalled}
      >
        <p>
          Claude Code passes your 5-hour and weekly limits to its status line on Pro and Max plans. UsageDock adds a small bridge as
          the status line and keeps only those numbers.
        </p>
        {i.claudeStatuslinePrevious && (
          <p className="mt-1">Your existing status line still runs and still shows its output.</p>
        )}
        <Meta>
          {i.claudeStatuslineInstalled
            ? `Last update ${ago(seen("claude", "claude-code"), now)}. Updates while Claude Code is in use.`
            : i.claudeCodeFound
              ? "Edits ~/.claude/settings.json (backed up first). Disconnect restores it."
              : "Claude Code not found on this PC."}
        </Meta>
        <Actions>
          {i.claudeStatuslineInstalled ? (
            <Btn onClick={() => run("cc", api.disconnectClaudeCode)} busy={busy === "cc"}>
              Disconnect
            </Btn>
          ) : (
            <Btn primary onClick={() => run("cc", api.connectClaudeCode)} busy={busy === "cc"} disabled={!i.bridgeFound}>
              Connect Claude Code
            </Btn>
          )}
        </Actions>
      </Card>

      <Card
        icon={<span aria-hidden className="grid h-[16px] w-[16px] place-items-center rounded-[3px] border border-white/40 text-[9px] font-bold text-ink-2">⧉</span>}
        title="Browser extension"
        badge={i.chromeHostRegistered ? "Bridge ready" : "Not set up"}
        on={i.chromeHostRegistered}
      >
        <p>
          Optional companion for Chrome or Edge. Every 15 minutes it opens Claude's usage page in a minimized window, reads the
          numbers shown, and closes it. It also records ChatGPT limit notices. It never reads cookies or passwords.
        </p>
        {i.chromeHostRegistered && (
          <ol className="mt-1.5 list-decimal space-y-0.5 pl-4 text-ink-2">
            <li>
              Open <code className="text-ink">chrome://extensions</code> and turn on Developer mode
            </li>
            <li>Click Load unpacked and choose the extension folder</li>
            <li>Done. It syncs on its own while Chrome is open</li>
          </ol>
        )}
        <Meta>
          Last sync: Claude {ago(seen("claude", "chrome-extension"), now)} · Codex {ago(seen("codex", "chrome-extension"), now)} · ChatGPT{" "}
          {ago(seen("chatgpt", "chrome-extension"), now)}
        </Meta>
        <Actions>
          {i.chromeHostRegistered ? (
            <>
              <Btn primary onClick={() => run("folder", () => api.openFolder("extension"))}>
                Open extension folder
              </Btn>
              <Btn onClick={() => api.openUrl("https://claude.ai/settings/usage")}>Claude usage ↗</Btn>
              <Btn onClick={() => run("ext", api.unregisterBrowserHost)} busy={busy === "ext"}>
                Remove
              </Btn>
            </>
          ) : (
            <Btn primary onClick={() => run("ext", api.registerBrowserHost)} busy={busy === "ext"} disabled={!i.bridgeFound}>
              Set up bridge
            </Btn>
          )}
        </Actions>
      </Card>

      <Card icon={<ProviderIcon id="codex" size={18} />} title="Codex CLI" badge={codex?.connected ? "Detected" : "Not found"} on={!!codex?.connected}>
        <p>Read automatically from the rate-limit status the Codex CLI saves in its local session logs. Nothing to set up.</p>
        <Meta>Last update {ago(codex?.lastSeen ?? null, now)}</Meta>
      </Card>

      <Card icon={<ProviderIcon id="chatgpt" size={18} />} title="ChatGPT" badge="Limited" on={false}>
        <p>
          OpenAI doesn't publish a usage percentage for ChatGPT plans, so UsageDock won't estimate one. With the extension it records
          limit notices that ChatGPT shows, including when they reset.
        </p>
      </Card>

      <p className="px-1 pt-1 text-[10.5px] leading-snug text-ink-3">
        Everything stays on this PC. UsageDock makes no network requests and never reads passwords, cookies or tokens.{" "}
        <button className="underline hover:text-ink" onClick={() => api.openFolder("sources")}>
          Open data folder
        </button>
      </p>
      {!i.bridgeFound && (
        <p className="px-1 text-[10.5px] text-[var(--color-warn)]">usagedock-bridge.exe is missing next to the app. Reinstall UsageDock.</p>
      )}
    </div>
  );
}

function Card({ icon, title, badge, on, children }: { icon: ReactNode; title: string; badge: string; on: boolean; children: ReactNode }) {
  return (
    <section className="border-t border-line py-3 leading-snug text-ink-3 first:border-t-0 first:pt-1">
      <header className="mb-1.5 flex items-center gap-2">
        {icon}
        <h3 className="tm-label text-[10.5px] text-ink">{title}</h3>
        <span className="tm-label ml-auto flex items-center gap-1.5 text-[8.5px]" style={{ color: on ? "var(--tm-ink)" : "var(--tm-faint)" }}>
          <span aria-hidden className="h-[5px] w-[5px] rounded-full" style={{ background: on ? "#fff" : "var(--tm-faint)" }} />
          {badge}
        </span>
      </header>
      <div className="text-[11.5px] text-ink-2">{children}</div>
    </section>
  );
}

const Meta = ({ children }: { children: ReactNode }) => <p className="mt-1.5 text-[10.5px] text-ink-3">{children}</p>;
const Actions = ({ children }: { children: ReactNode }) => <div className="mt-2 flex flex-wrap gap-1.5">{children}</div>;

function Btn({
  children,
  onClick,
  primary,
  busy,
  disabled,
}: {
  children: ReactNode;
  onClick: () => void;
  primary?: boolean;
  busy?: boolean;
  disabled?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      disabled={busy || disabled}
      className={`rounded-[4px] border px-2.5 py-1 text-[10px] font-semibold tracking-[0.1em] uppercase transition-colors duration-150 disabled:opacity-50 ${
        primary ? "border-white bg-white text-black hover:bg-white/85" : "border-white/25 text-ink hover:border-white/60"
      }`}
    >
      {busy ? "Working…" : children}
    </button>
  );
}
