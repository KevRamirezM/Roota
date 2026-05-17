import { useState } from "react";
import { useOrchestrator } from "../hooks/useOrchestrator";
import { t, type Lang } from "../i18n";
import { togglePanel } from "../tauri-api";
import { ConfirmationModal } from "./ConfirmationModal";
import { ExamplePrompts } from "./ExamplePrompts";
import { GuidancePanel } from "./GuidancePanel";

const lang: Lang = "es";

export function MainScreen() {
  const { phase, submit, respondToConfirmation, cancel, requestStuckHelp, busy } =
    useOrchestrator();
  const [utterance, setUtterance] = useState("");

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const text = utterance.trim();
    if (!text || busy) return;
    setUtterance("");
    await submit(text);
  };

  const handleExample = (text: string) => {
    setUtterance(text);
  };

  const showExamples =
    phase.kind === "idle" || phase.kind === "cancelled" || phase.kind === "completed";
  const canCancel =
    phase.kind === "running" ||
    phase.kind === "classifying" ||
    phase.kind === "observing" ||
    phase.kind === "plan_preview" ||
    phase.kind === "replanning";
  const showStuckHelp = phase.kind === "running" && !phase.stepSuccess;

  const handleStop = () => {
    if (canCancel) {
      void cancel();
    } else {
      void togglePanel();
    }
  };

  return (
    <div className="roota-shell">
      <div className="roota-chrome" data-tauri-drag-region>
        <span className="chrome-brand" aria-hidden title={t("app.title", lang)} />
        <button
          type="button"
          className="chrome-hide"
          onClick={() => void togglePanel()}
          title={t("panel.shortcut_hint", lang)}
        >
          <span className="chrome-hide-chevron" aria-hidden>
            ▾
          </span>
          {t("panel.hide", lang)}
        </button>
        <button
          type="button"
          className="chrome-stop"
          onClick={handleStop}
          title={canCancel ? t("main.cancel", lang) : t("panel.hide", lang)}
          aria-label={canCancel ? t("main.cancel", lang) : t("panel.hide", lang)}
        >
          <span className="chrome-stop-icon" aria-hidden />
        </button>
      </div>

      <GuidancePanel phase={phase} lang={lang}>
        {showStuckHelp && (
          <button
            type="button"
            className="quick-action plan-stuck-help"
            onClick={() => void requestStuckHelp()}
          >
            {t("guidance.stuck_button", lang)}
          </button>
        )}

        {showExamples && (
          <ExamplePrompts lang={lang} disabled={busy} onSelect={handleExample} />
        )}

        <form className="composer" onSubmit={handleSubmit}>
          <div className="composer-inner">
            <input
              className="input"
              type="text"
              value={utterance}
              onChange={(e) => setUtterance(e.target.value)}
              placeholder={t("main.input_placeholder", lang)}
              aria-label={t("main.greeting", lang)}
              autoFocus
              disabled={busy}
            />
            <button
              type="submit"
              className="composer-send"
              disabled={busy || !utterance.trim()}
              aria-label={t("main.send_button", lang)}
            >
              <span className="composer-send-icon" aria-hidden>
                ▶
              </span>
            </button>
          </div>
          <div className="composer-footer">
            <p className="composer-footer-hint">{t("guidance.safety_note", lang)}</p>
            {canCancel && (
              <button type="button" className="composer-cancel" onClick={() => void cancel()}>
                {t("main.cancel", lang)}
              </button>
            )}
          </div>
        </form>
      </GuidancePanel>

      <ConfirmationModal
        open={phase.kind === "awaiting_confirmation"}
        message={phase.kind === "awaiting_confirmation" ? phase.message : ""}
        lang={lang}
        onAccept={() => void respondToConfirmation(true)}
        onReject={() => void respondToConfirmation(false)}
      />
    </div>
  );
}
