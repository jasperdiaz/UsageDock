import { gsap } from "gsap";
import { CustomEase } from "gsap/CustomEase";

gsap.registerPlugin(CustomEase);

/**
 * Motion language: launch-webcast telemetry.
 * - "spacex": the curve spacex.com uses for its fades, cubic-bezier(.25,.8,.25,1).
 *   A soft, unhurried ease-out with no overshoot.
 * - Gauges sweep with power4.out: a fast spin-up that settles gently, like a
 *   dial catching up with live telemetry. Nothing bounces.
 */
CustomEase.create("spacex", "M0,0 C0.25,0.8 0.25,1 1,1");

export const EASE_UI = "spacex";
export const EASE_GAUGE = "power4.out";

export const DUR = {
  /** Panel and labels fading in. */
  fade: 0.7,
  /** First sweep of a dial from zero. */
  sweep: 1.4,
  /** Track ring drawing on before the value sweeps. */
  track: 0.8,
  /** A dial moving to a new reading. */
  update: 0.9,
  /** Delay between dials in the intro. */
  stagger: 0.08,
  /** When the dials start, after the panel has begun to appear. */
  gaugeStart: 0.2,
};

export const reducedMotion = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/**
 * Intro for a panel: the surface fades up, then anything marked
 * `data-intro` follows in document order. Returns the timeline to kill on unmount.
 */
export function introPanel(surface: Element | null): gsap.core.Timeline | null {
  if (!surface || reducedMotion()) return null;
  const tl = gsap.timeline({ defaults: { ease: EASE_UI } });
  tl.from(surface, { opacity: 0, y: 6, duration: DUR.fade, clearProps: "opacity,transform" });
  const parts = surface.querySelectorAll("[data-intro]");
  if (parts.length) {
    tl.from(parts, { opacity: 0, y: 4, duration: DUR.fade, stagger: 0.06, clearProps: "opacity,transform" }, 0.15);
  }
  return tl;
}

/** Stagger slot for a dial: its position among the dials on the page. */
export function gaugeDelay(el: Element): number {
  const all = Array.from(document.querySelectorAll("[data-gauge]"));
  const i = Math.max(0, all.indexOf(el));
  return DUR.gaugeStart + i * DUR.stagger;
}

export { gsap };
