import { level, levelColor } from "../lib/format";
import type { WindowState } from "../lib/types";

/** Progress bar. Colour follows how much is *used*, whichever value is displayed. */
export function UsageBar({
  pct,
  used,
  state,
  height = 6,
  label,
}: {
  pct: number | null;
  used: number | null;
  state: WindowState;
  height?: number;
  label: string;
}) {
  const color = levelColor[level(used)];
  return (
    <div
      className="bar"
      data-state={state}
      style={{ height }}
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={pct == null ? undefined : Math.round(pct)}
    >
      <div className="bar-fill" style={{ ["--bar-pct" as string]: pct ?? 0, ["--bar-color" as string]: color }} />
    </div>
  );
}
