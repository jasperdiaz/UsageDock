import type { ProviderStatus } from "../lib/types";

const map: Record<ProviderStatus, { color: string; text: string }> = {
  live: { color: "var(--color-ok)", text: "Live" },
  stale: { color: "var(--color-warn)", text: "Stale" },
  waiting: { color: "var(--color-ink-3)", text: "Waiting for sync" },
  unavailable: { color: "var(--color-ink-4)", text: "Unavailable" },
  demo: { color: "var(--tm-amber)", text: "Demo" },
};

export function statusText(s: ProviderStatus) {
  return map[s].text;
}

export function StatusDot({ status }: { status: ProviderStatus }) {
  const c = map[status].color;
  return (
    <span
      aria-hidden
      className="inline-block h-[6px] w-[6px] shrink-0 rounded-full"
      style={{ background: c, boxShadow: status === "live" ? `0 0 6px ${c}` : undefined }}
    />
  );
}
