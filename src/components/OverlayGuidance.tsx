import { useEffect, useRef, useState } from "react";
import { actionLabel, t, type Lang } from "../i18n";
import { listenGuidance } from "../tauri-api";
import type { ActionVerb, GuidancePayload } from "../types";

const PULSE_MS = 1100;
const lang: Lang = "es";

const ACTION_COLORS: Record<
  ActionVerb,
  { ring: string; fill: string; glow: string }
> = {
  click: {
    ring: "rgba(255, 200, 87, 0.95)",
    fill: "rgba(255, 200, 87, 0.14)",
    glow: "rgba(255, 200, 87, 0.35)",
  },
  double_click: {
    ring: "rgba(255, 160, 60, 0.95)",
    fill: "rgba(255, 160, 60, 0.16)",
    glow: "rgba(255, 160, 60, 0.4)",
  },
  right_click: {
    ring: "rgba(120, 200, 255, 0.95)",
    fill: "rgba(120, 200, 255, 0.14)",
    glow: "rgba(120, 200, 255, 0.35)",
  },
  type: {
    ring: "rgba(130, 255, 180, 0.95)",
    fill: "rgba(130, 255, 180, 0.12)",
    glow: "rgba(130, 255, 180, 0.32)",
  },
  locate: {
    ring: "rgba(255, 248, 200, 0.9)",
    fill: "rgba(255, 248, 200, 0.1)",
    glow: "rgba(255, 248, 200, 0.28)",
  },
};

function centerOf(rect: { x: number; y: number; width: number; height: number }) {
  return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
}

export function OverlayGuidance() {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [guidance, setGuidance] = useState<GuidancePayload | null>(null);

  useEffect(() => {
    document.documentElement.classList.add("overlay");
    document.body.classList.add("overlay");
    return () => {
      document.documentElement.classList.remove("overlay");
      document.body.classList.remove("overlay");
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    void (async () => {
      unlisten = await listenGuidance((payload) => {
        setGuidance(payload.active ? payload : null);
      });
    })();
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let frame = 0;
    const resize = () => {
      const dpr = window.devicePixelRatio || 1;
      canvas.width = window.innerWidth * dpr;
      canvas.height = window.innerHeight * dpr;
      canvas.style.width = `${window.innerWidth}px`;
      canvas.style.height = `${window.innerHeight}px`;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    resize();
    window.addEventListener("resize", resize);

    const draw = (now: number) => {
      ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);
      if (guidance?.rect) {
        const colors = ACTION_COLORS[guidance.action];
        const { rect } = guidance;
        const center = centerOf(rect);
        const tNorm = (now % PULSE_MS) / PULSE_MS;
        const pulse = Math.sin(tNorm * Math.PI * 2) * 0.5 + 0.5;
        const pad = 10 + pulse * 6;
        const rx = rect.x - pad;
        const ry = rect.y - pad;
        const rw = rect.width + pad * 2;
        const rh = rect.height + pad * 2;
        const corner = Math.min(16, rw / 4, rh / 4);

        ctx.save();

        ctx.lineWidth = 4;
        ctx.strokeStyle = colors.ring;
        ctx.shadowColor = colors.glow;
        ctx.shadowBlur = 18 + pulse * 12;
        ctx.beginPath();
        ctx.roundRect(rx, ry, rw, rh, corner);
        ctx.stroke();
        ctx.shadowBlur = 0;

        const baseR = Math.max(28, Math.min(rect.width, rect.height) * 0.35);
        const r = baseR + pulse * 14;
        ctx.lineWidth = 5;
        ctx.strokeStyle = colors.ring.replace("0.95", `${0.55 + pulse * 0.4})`);
        ctx.beginPath();
        ctx.arc(center.x, center.y, r, 0, Math.PI * 2);
        ctx.stroke();

        if (guidance.action === "double_click") {
          const r2 = r + 18 + pulse * 8;
          ctx.lineWidth = 3;
          ctx.beginPath();
          ctx.arc(center.x, center.y, r2, 0, Math.PI * 2);
          ctx.stroke();
        }

        const bounce = Math.sin(tNorm * Math.PI * 2) * 8;
        const tipY = center.y - r - 18 - bounce;
        ctx.fillStyle = colors.ring;
        ctx.beginPath();
        ctx.moveTo(center.x, tipY - 26);
        ctx.lineTo(center.x - 16, tipY);
        ctx.lineTo(center.x + 16, tipY);
        ctx.closePath();
        ctx.fill();

        const hint = guidance.clickHint || t("guidance.hint.click", lang);
        ctx.font = '600 13px "Segoe UI", system-ui, sans-serif';
        const metrics = ctx.measureText(hint);
        const pillW = metrics.width + 20;
        const pillH = 28;
        const pillX = center.x - pillW / 2;
        const pillY = center.y + r + 16;
        const rad = 18;
        ctx.fillStyle = "rgba(255, 249, 237, 0.97)";
        ctx.beginPath();
        ctx.moveTo(pillX + rad, pillY);
        ctx.arcTo(pillX + pillW, pillY, pillX + pillW, pillY + pillH, rad);
        ctx.arcTo(pillX + pillW, pillY + pillH, pillX, pillY + pillH, rad);
        ctx.arcTo(pillX, pillY + pillH, pillX, pillY, rad);
        ctx.arcTo(pillX, pillY, pillX + pillW, pillY, rad);
        ctx.closePath();
        ctx.fill();
        ctx.fillStyle = "#0a1a32";
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.fillText(hint, center.x, pillY + pillH / 2);

        if (guidance.targetLabel) {
          ctx.font = '500 12px "Segoe UI", system-ui, sans-serif';
          ctx.fillStyle = "rgba(255, 249, 237, 0.92)";
          ctx.fillText(guidance.targetLabel, center.x, pillY + pillH + 22);
        }

        ctx.restore();
      }
      frame = requestAnimationFrame(draw);
    };
    frame = requestAnimationFrame(draw);

    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", resize);
    };
  }, [guidance]);

  return (
    <div
      className="overlay-root"
      aria-live={guidance ? "polite" : undefined}
      aria-hidden={!guidance}
    >
      <canvas ref={canvasRef} className="overlay-canvas" />
      {guidance && (
        <aside className="guidance-hud" role="status">
          <p className="guidance-hud-step">
            {t("feedback.step_label", lang, {
              step: guidance.stepIndex,
              total: guidance.stepTotal,
            })}
          </p>
          <p className="guidance-hud-action">
            <span className="guidance-hud-badge">{actionLabel(guidance.action, lang)}</span>
          </p>
          <p className="guidance-hud-instruction">{guidance.instruction}</p>
          {!guidance.hasTarget && (
            <p className="guidance-hud-missing">
              {t("guidance.hud_no_target", lang, { target: guidance.targetLabel })}
            </p>
          )}
          {guidance.hasTarget && (
            <p className="guidance-hud-hint">{t("guidance.overlay_hint", lang)}</p>
          )}
        </aside>
      )}
    </div>
  );
}


