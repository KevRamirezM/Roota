import { t, type Lang } from "../i18n";
import type { PlanStepSummary } from "../types";
import { ActionBadge } from "./ActionBadge";

interface Props {
  summary: string;
  steps: PlanStepSummary[];
  lang?: Lang;
}

export function PlanPreview({ summary, steps, lang = "es" }: Props) {
  return (
    <section aria-labelledby="plan-preview-title">
      <p id="plan-preview-title" className="panel-status">
        {t("guidance.plan_preview_title", lang)}
      </p>
      <p className="panel-instruction">{summary}</p>
      <ol className="plan-preview-steps" aria-label={t("guidance.plan_preview_title", lang)}>
        {steps.map((step) => (
          <li key={step.index} className="plan-preview-step">
            <span className="plan-preview-step-index" aria-hidden>
              {step.index}
            </span>
            <span className="plan-preview-step-body">
              <ActionBadge action={step.action} lang={lang} />
              <span className="panel-hint plan-preview-step-target">{step.target}</span>
            </span>
          </li>
        ))}
      </ol>
    </section>
  );
}
