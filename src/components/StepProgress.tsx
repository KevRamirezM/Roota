import { t, type Lang } from "../i18n";

interface Props {
  step: number;
  total: number;
  lang?: Lang;
}

export function StepProgress({ step, total, lang = "es" }: Props) {
  const pct = total > 0 ? Math.min(100, Math.round((step / total) * 100)) : 0;
  return (
    <div className="step-progress">
      <p className="step-progress-label">
        {t("feedback.step_label", lang, { step, total })}
      </p>
      <div className="step-progress-track">
        <div
          className="step-progress-fill"
          style={{ width: `${pct}%` }}
          role="progressbar"
          aria-valuenow={step}
          aria-valuemin={1}
          aria-valuemax={total}
        />
      </div>
    </div>
  );
}
