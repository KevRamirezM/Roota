import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { MainScreen } from "./components/MainScreen";
import { OverlayGuidance } from "./components/OverlayGuidance";

export function App() {
  const [label, setLabel] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const win = getCurrentWindow();
        if (!cancelled) setLabel(win.label);
      } catch {
        if (!cancelled) setLabel("main");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (label === "overlay") return <OverlayGuidance />;
  return <MainScreen />;
}
