import type { ReactNode } from "react";
import { actionLabel, t, type Lang } from "../i18n";
import type { AppPhase } from "../types";
import { ActionBadge } from "./ActionBadge";
import { PlanPreview } from "./PlanPreview";
import { StepProgress } from "./StepProgress";

interface Props {
  phase: AppPhase;
  lang?: Lang;
  children?: ReactNode;
}

export function GuidancePanel({ phase, lang = "es", children }: Props) {
  const panelClass = ["roota-panel"];
  let statusLabel = t("app.subtitle", lang);
  let instruction = t("main.greeting", lang);
  let hint: string | null = null;
  let showProgress = false;
  let step = 0;
  let total = 0;
  let action: string | null = null;
  let ctaLabel = t("main.send_button", lang);

  switch (phase.kind) {
    case "idle":
      break;
    case "classifying":
      panelClass.push("is-waiting");
      statusLabel = t("feedback.classifying", lang);
      instruction = t("feedback.classifying", lang);
      ctaLabel = t("feedback.classifying", lang);
      break;
    case "awaiting_confirmation":
      panelClass.push("is-waiting");
      statusLabel = t("confirm.title", lang);
      instruction = t("feedback.waiting_confirm", lang);
      ctaLabel = t("confirm.title", lang);
      hint = t("guidance.safety_note", lang);
      break;
    case "observing":
      panelClass.push("is-waiting");
      statusLabel = t("guidance.observing", lang);
      instruction = t("guidance.observing", lang);
      ctaLabel = t("guidance.observing", lang);
      hint = t("guidance.safety_note", lang);
      break;
    case "plan_preview":
      panelClass.push("is-waiting");
      statusLabel = t("app.subtitle", lang);
      instruction = "";
      ctaLabel = t("main.send_button", lang);
      hint = t("guidance.safety_note", lang);
      break;
    case "replanning":
      panelClass.push("is-waiting");
      statusLabel = t("guidance.replanning", lang);
      instruction = t("guidance.replanning", lang);
      ctaLabel = t("guidance.replanning", lang);
      hint = t("guidance.safety_note", lang);
      break;
    case "running": {
      const s = phase.step;
      step = s.index;
      total = s.total;
      action = s.action;
      showProgress = true;
      if (phase.stepSuccess) {
        panelClass.push("is-success");
        statusLabel = t("feedback.success_title", lang);
        instruction = t("feedback.success_body", lang);
        ctaLabel = t("feedback.success_title", lang);
      } else {
        statusLabel = t("feedback.step_label", lang, { step: s.index, total: s.total });
        instruction = s.instruction;
        ctaLabel = actionLabel(s.action, lang);
        if (phase.anchor || s.anchorXy) {
          hint = t("guidance.overlay_hint", lang);
        } else {
          hint = t("guidance.overlay_missing", lang, { target: s.targetText });
        }
      }
      break;
    }
    case "completed":
      panelClass.push("is-success");
      statusLabel = t("feedback.completed_title", lang);
      instruction = t("feedback.completed_body", lang);
      ctaLabel = t("feedback.completed_title", lang);
      break;
    case "cancelled":
      statusLabel = t("feedback.cancelled_title", lang);
      instruction = t("feedback.cancelled_body", lang);
      break;
    case "error":
      panelClass.push("is-error");
      statusLabel = t("feedback.error_title", lang);
      instruction = phase.message;
      ctaLabel = t("feedback.error_title", lang);
      hint = t("guidance.safety_note", lang);
      break;
  }

  const isLive =
    phase.kind === "classifying" ||
    phase.kind === "observing" ||
    phase.kind === "plan_preview" ||
    phase.kind === "replanning" ||
    phase.kind === "running" ||
    phase.kind === "awaiting_confirmation";

  return (
    <article
      className={panelClass.join(" ")}
      aria-live={phase.kind === "error" ? "assertive" : "polite"}
    >
      <header className="panel-head">
        <p className="panel-status">
          <span className={`status-dot${isLive ? " live" : ""}`} aria-hidden />
          {statusLabel}
        </p>
        <span className="panel-cta" aria-hidden>
          {ctaLabel}
        </span>
      </header>

      {phase.kind !== "plan_preview" && (
        <p className="panel-instruction">{instruction}</p>
      )}

      {phase.kind === "plan_preview" && (
        <PlanPreview summary={phase.summary} steps={phase.steps} lang={lang} />
      )}

      <div className="panel-meta">
        {phase.kind === "running" && !phase.stepSuccess && showProgress && (
          <StepProgress step={step} total={total} lang={lang} />
        )}
        {phase.kind === "running" && !phase.stepSuccess && action && (
          <ActionBadge action={phase.step.action} lang={lang} />
        )}
      </div>

      {hint && (
        <p className="panel-hint">
          {phase.kind === "running" && phase.anchor ? (
            <>
              <strong>{actionLabel(phase.step.action, lang)}:</strong> {hint}
            </>
          ) : (
            hint
          )}
        </p>
      )}

      {children}
    </article>
  );
}
