import { useState } from "react";
import { useOrchestrator } from "../hooks/useOrchestrator";
import { t, type Lang } from "../i18n";
import { togglePanel } from "../tauri-api";
import { ConfirmationModal } from "./ConfirmationModal";
import { ExamplePrompts } from "./ExamplePrompts";
import { GuidancePanel } from "./GuidancePanel";

const lang: Lang = "es";

export function MainScreen() {
  const { phase, submit, respondToConfirmation, cancel, busy } = useOrchestrator();
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

  const showExamples = phase.kind === "idle" || phase.kind === "cancelled" || phase.kind === "completed";
  const canCancel = phase.kind === "running" || phase.kind === "classifying";

  return (
    <main className="app-shell">
      <header className="app-header" data-tauri-drag-region>
        <span className="brand-mark" aria-hidden />
        <h1 className="app-title">{t("app.title", lang)}</h1>
        <p className="app-tagline">{t("app.subtitle", lang)}</p>
        <p className="panel-shortcut-hint">{t("panel.shortcut_hint", lang)}</p>
        <p className="safety-note">{t("guidance.safety_note", lang)}</p>
      </header>

      <GuidancePanel phase={phase} lang={lang} />

      {showExamples && (
        <ExamplePrompts lang={lang} disabled={busy} onSelect={handleExample} />
      )}

      <form className="composer" onSubmit={handleSubmit}>
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
        <div className="composer-actions">
          <button
            type="submit"
            className="button button-primary"
            disabled={busy || !utterance.trim()}
          >
            {t("main.send_button", lang)}
          </button>
          {canCancel && (
            <button
              type="button"
              className="button button-ghost"
              onClick={() => void cancel()}
            >
              {t("main.cancel", lang)}
            </button>
          )}
          <button
            type="button"
            className="button button-ghost"
            onClick={() => void togglePanel()}
            title={t("panel.shortcut_hint", lang)}
          >
            {t("panel.hide", lang)}
          </button>
        </div>
      </form>

      <ConfirmationModal
        open={phase.kind === "awaiting_confirmation"}
        message={phase.kind === "awaiting_confirmation" ? phase.message : ""}
        lang={lang}
        onAccept={() => void respondToConfirmation(true)}
        onReject={() => void respondToConfirmation(false)}
      />
    </main>
  );
}
