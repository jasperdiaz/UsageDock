import { useState } from "react";

/** Switch using the transitions.dev toggle (thumb double-bounce). */
export function Toggle({
  on,
  onChange,
  label,
  disabled,
}: {
  on: boolean;
  onChange: (v: boolean) => void;
  label: string;
  disabled?: boolean;
}) {
  const [init, setInit] = useState(false);
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      disabled={disabled}
      data-on={on ? "true" : "false"}
      onClick={() => {
        setInit(true);
        onChange(!on);
      }}
      className={`t-toggle ${init ? "is-init" : ""} relative h-[18px] w-[32px] shrink-0 rounded-full p-[2px] disabled:opacity-40 ${
        on ? "bg-white" : "bg-white/15"
      }`}
    >
      <span className={`t-toggle-thumb block h-[14px] w-[14px] rounded-full ${on ? "bg-black" : "bg-white"}`} />
    </button>
  );
}
