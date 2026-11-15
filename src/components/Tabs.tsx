import { useLayoutEffect, useRef } from "react";

/** Segmented tabs using the transitions.dev sliding pill. */
export function Tabs<T extends string>({
  tabs,
  value,
  onChange,
}: {
  tabs: { id: T; label: string }[];
  value: T;
  onChange: (v: T) => void;
}) {
  const pill = useRef<HTMLSpanElement>(null);
  const refs = useRef<Record<string, HTMLButtonElement | null>>({});
  const first = useRef(true);

  useLayoutEffect(() => {
    const p = pill.current;
    const tab = refs.current[value];
    if (!p || !tab) return;
    const move = (animate: boolean) => {
      if (!animate) {
        const prev = p.style.transition;
        p.style.transition = "none";
        p.style.transform = `translateX(${tab.offsetLeft}px)`;
        p.style.width = `${tab.offsetWidth}px`;
        void p.offsetWidth;
        p.style.transition = prev;
      } else {
        p.style.transform = `translateX(${tab.offsetLeft}px)`;
        p.style.width = `${tab.offsetWidth}px`;
      }
    };
    move(!first.current);
    first.current = false;
    const onResize = () => move(false);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, [value]);

  return (
    <div className="t-tabs" role="tablist">
      <span ref={pill} className="t-tabs-pill" aria-hidden="true" />
      {tabs.map((t) => (
        <button
          key={t.id}
          ref={(el) => {
            refs.current[t.id] = el;
          }}
          className="t-tab"
          role="tab"
          aria-selected={t.id === value}
          onClick={() => onChange(t.id)}
        >
          {t.label}
        </button>
      ))}
    </div>
  );
}
