import { useLayoutEffect, useRef, useState } from "react";
import { DUR, EASE_UI, gsap, reducedMotion } from "../lib/motion";

/** How long the splash holds at minimum, in ms. It also waits for data. */
export const SPLASH_MS = 2400;

const STEPS = ["Acquiring signal", "Syncing telemetry", "All systems go"];

/**
 * Launch splash, shown centred on screen before the widget glides to its spot.
 * A spinning ring, the mark, a status line and a progress hairline that fills
 * while we wait. When `leaving` turns true the bar completes, the panel fades
 * out and `onDone` fires.
 */
export function Splash({ leaving, onDone }: { leaving: boolean; onDone: () => void }) {
  const root = useRef<HTMLDivElement>(null);
  const bar = useRef<HTMLDivElement>(null);
  const fillTl = useRef<gsap.core.Timeline | null>(null);
  const [step, setStep] = useState(0);
  const done = useRef(onDone);
  done.current = onDone;

  // Arrival: fade up, then fill the bar most of the way over the hold time,
  // stepping the status line as it goes.
  useLayoutEffect(() => {
    if (reducedMotion()) {
      gsap.set(bar.current, { scaleX: 0.9 });
      setStep(1);
      return;
    }
    const hold = SPLASH_MS / 1000;
    const tl = gsap.timeline({ defaults: { ease: EASE_UI } });
    tl.from(root.current, { opacity: 0, y: 6, scale: 0.985, duration: DUR.fade });
    tl.fromTo(bar.current, { scaleX: 0 }, { scaleX: 0.9, duration: hold, ease: "power2.out" }, 0.1);
    tl.call(() => setStep(1), [], hold * 0.45);
    fillTl.current = tl;
    return () => {
      tl.kill();
    };
  }, []);

  // Departure: finish the bar, call it, fade out.
  useLayoutEffect(() => {
    if (!leaving) return;
    if (reducedMotion()) {
      done.current();
      return;
    }
    fillTl.current?.kill();
    setStep(2);
    const tl = gsap.timeline({ defaults: { ease: EASE_UI }, onComplete: () => done.current() });
    tl.to(bar.current, { scaleX: 1, duration: 0.35 });
    tl.to(root.current, { opacity: 0, scale: 0.985, duration: 0.4 }, "+=0.25");
    return () => {
      tl.kill();
    };
  }, [leaving]);

  const R = 29;
  return (
    <div ref={root} className="splash flex items-center gap-5 py-5 pr-7 pl-5" role="status" aria-live="polite">
      <div className="relative grid h-[68px] w-[68px] place-items-center">
        <svg className="splash-ring absolute inset-0" width={68} height={68} viewBox="0 0 68 68" aria-hidden>
          <circle cx={34} cy={34} r={R} fill="none" stroke="var(--tm-track)" strokeWidth={2.5} />
          <circle
            cx={34}
            cy={34}
            r={R}
            fill="none"
            stroke="var(--tm-ink)"
            strokeWidth={2.5}
            strokeLinecap="round"
            strokeDasharray={`${2 * Math.PI * R * 0.22} ${2 * Math.PI * R}`}
          />
        </svg>
        <svg aria-hidden width={28} height={28} viewBox="16 28 224 200">
          {/* UsageDock mark (brand/final2/mark.svg) */}
          <path d="M 48.80 219.20 A 112 112 0 0 1 112.00 29.15 L 112.00 86.33 A 56 56 0 0 0 88.40 179.60 Z" fill="#fff" />
          <path d="M 144.00 29.15 A 112 112 0 0 1 207.20 219.20 L 167.60 179.60 A 56 56 0 0 0 144.00 86.33 Z" fill="#fff" />
        </svg>
      </div>
      <div className="flex w-[232px] flex-col">
        <span className="flex items-baseline gap-[9px] leading-none">
          <span className="tm-label text-[18px] tracking-[0.2em] text-white">UsageDock</span>
          <span className="tm-label text-[11px] text-[var(--tm-dim)]">
            by <span className="text-[var(--tm-signature)]">Ted</span>
          </span>
        </span>
        <span className="tm-label mt-[10px] text-[10.5px] text-[var(--tm-dim)]">{STEPS[step]}</span>
        <div className="mt-[11px] h-[2px] w-full bg-[var(--tm-track)]">
          <div ref={bar} className="h-[2px] w-full origin-left bg-white" style={{ transform: "scaleX(0)" }} />
        </div>
      </div>
    </div>
  );
}
