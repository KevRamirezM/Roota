import confetti from "canvas-confetti";
import type { CreateTypes, Options } from "canvas-confetti";

const BRAND_COLORS = ["#4d8ef7", "#34d399", "#fbbf24", "#ffffff", "#6ba0ff"];

let panelConfetti: CreateTypes | null = null;

function prefersReducedMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/** Attach confetti to the in-app canvas (call from ConfettiLayer on mount). */
export function bindConfettiCanvas(canvas: HTMLCanvasElement): void {
  unbindConfettiCanvas();
  panelConfetti = confetti.create(canvas, {
    resize: true,
    useWorker: false,
  });
}

export function unbindConfettiCanvas(): void {
  panelConfetti = null;
}

function fire(options: Options): void {
  const fn = panelConfetti ?? confetti;
  void fn({
    disableForReducedMotion: false,
    zIndex: 10_000,
    ...options,
  });
}

/** Short celebratory burst when the user finishes a guided task. */
export function celebrateTaskComplete(): void {
  if (prefersReducedMotion()) return;

  const runBurst = () => {
    if (!panelConfetti) {
      requestAnimationFrame(runBurst);
      return;
    }
    startBurst();
  };

  requestAnimationFrame(runBurst);
}

function startBurst(): void {
  const end = Date.now() + 2_000;
  let lastTick = 0;

  const tick = (now: number) => {
    if (now - lastTick > 100) {
      lastTick = now;
      fire({
        particleCount: 8,
        angle: 55 + Math.random() * 25,
        spread: 60,
        startVelocity: 42,
        origin: { x: 0.1, y: 0.65 },
        colors: BRAND_COLORS,
      });
      fire({
        particleCount: 8,
        angle: 115 + Math.random() * 25,
        spread: 60,
        startVelocity: 42,
        origin: { x: 0.9, y: 0.65 },
        colors: BRAND_COLORS,
      });
    }
    if (Date.now() < end) {
      requestAnimationFrame(tick);
    } else {
      fire({
        particleCount: 90,
        spread: 100,
        startVelocity: 36,
        decay: 0.9,
        scalar: 1,
        origin: { x: 0.5, y: 0.5 },
        colors: BRAND_COLORS,
      });
    }
  };

  requestAnimationFrame(tick);
}
