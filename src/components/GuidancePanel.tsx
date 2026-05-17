import { actionLabel, t, type Lang } from "../i18n";
import type { AppPhase } from "../types";
import { ActionBadge } from "./ActionBadge";
import { StepProgress } from "./StepProgress";

interface Props {
  phase: AppPhase;
  lang?: Lang;
}

export function GuidancePanel({ phase, lang = "es" }: Props) {
  const cardClass = ["guidance-card"];
  let statusLabel = t("app.subtitle", lang);
  let instruction = t("main.greeting", lang);
  let hint: string | null = t("guidance.safety_note", lang);
  let showProgress = false;
  let step = 0;
  let total = 0;
  let action: string | null = null;

  switch (phase.kind) {
    case "idle":
      break;
    case "classifying":
      cardClass.push("is-waiting");
      statusLabel = t("feedback.classifying", lang);
      instruction = t("feedback.classifying", lang);
      hint = null;
      break;
    case "awaiting_confirmation":
      cardClass.push("is-waiting");
      statusLabel = t("confirm.title", lang);
      instruction = t("feedback.waiting_confirm", lang);
      hint = t("guidance.safety_note", lang);
      break;
    case "running": {
      const s = phase.step;
      step = s.index;
      total = s.total;
      action = s.action;
      showProgress = true;
      if (phase.stepSuccess) {
        cardClass.push("is-success");
        statusLabel = t("feedback.success_title", lang);
        instruction = t("feedback.success_body", lang);
        hint = null;
      } else {
        statusLabel = t("feedback.step_label", lang, { step: s.index, total: s.total });
        instruction = s.instruction;
        if (phase.anchor || s.anchorXy) {
          hint = t("guidance.overlay_hint", lang);
        } else {
          hint = t("guidance.overlay_missing", lang, { target: s.targetText });
        }
      }
      break;
    }
    case "completed":
      cardClass.push("is-success");
      statusLabel = t("feedback.completed_title", lang);
      instruction = t("feedback.completed_body", lang);
      hint = null;
      break;
    case "cancelled":
      statusLabel = t("feedback.cancelled_title", lang);
      instruction = t("feedback.cancelled_body", lang);
      hint = null;
      break;
    case "error":
      cardClass.push("is-error");
      statusLabel = t("feedback.error_title", lang);
      instruction = phase.message;
      hint = t("guidance.safety_note", lang);
      break;
  }

  const isLive =
    phase.kind === "classifying" ||
    phase.kind === "running" ||
    phase.kind === "awaiting_confirmation";

  return (
    <article
      className={cardClass.join(" ")}
      aria-live={phase.kind === "error" ? "assertive" : "polite"}
    >
      <p className="guidance-status">
        <span className={`status-dot${isLive ? " live" : ""}`} aria-hidden />
        {statusLabel}
      </p>

      {phase.kind === "running" && !phase.stepSuccess && showProgress && (
        <StepProgress step={step} total={total} lang={lang} />
      )}

      {phase.kind === "running" && !phase.stepSuccess && action && (
        <ActionBadge action={phase.step.action} lang={lang} />
      )}

      <p className="guidance-instruction">{instruction}</p>

      {hint && (
        <p className="guidance-hint">
          {phase.kind === "running" && phase.anchor ? (
            <>
              <strong>{actionLabel(phase.step.action, lang)}:</strong> {hint}
            </>
          ) : (
            hint
          )}
        </p>
      )}
    </article>
  );
}
