import { t, type Lang } from "../i18n";

const EXAMPLES = [
  "example.open_folder",
  "example.open_browser",
  "example.compose_email",
] as const;

interface Props {
  lang?: Lang;
  disabled?: boolean;
  onSelect: (text: string) => void;
}

export function ExamplePrompts({ lang = "es", disabled, onSelect }: Props) {
  return (
    <section className="examples-block" aria-label={t("main.examples_title", lang)}>
      <p className="examples-title">{t("main.examples_title", lang)}</p>
      <div className="example-chips">
        {EXAMPLES.map((key) => (
          <button
            key={key}
            type="button"
            className="chip"
            disabled={disabled}
            onClick={() => onSelect(t(key, lang))}
          >
            {t(key, lang)}
          </button>
        ))}
      </div>
    </section>
  );
}
