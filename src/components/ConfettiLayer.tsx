import { useEffect, useRef } from "react";

import { bindConfettiCanvas, unbindConfettiCanvas } from "../lib/celebrate";

/** In-panel canvas for confetti (required for Tauri transparent webviews). */
export function ConfettiLayer() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    bindConfettiCanvas(canvas);
    return () => unbindConfettiCanvas();
  }, []);

  return <canvas ref={canvasRef} className="confetti-layer" aria-hidden />;
}
