import { t, type Lang } from "../i18n";

const EXAMPLES = [
  { key: "example.open_folder", icon: "✦" },
  { key: "example.open_browser", icon: "◇" },
  { key: "example.compose_email", icon: "◻" },
] as const;

interface Props {
  lang?: Lang;
  disabled?: boolean;
  onSelect: (text: string) => void;
}

export function ExamplePrompts({ lang = "es", disabled, onSelect }: Props) {
  return (
    <nav className="quick-actions" aria-label={t("main.examples_title", lang)}>
      {EXAMPLES.map((item, index) => (
        <span key={item.key} className="quick-actions-item">
          {index > 0 && <span className="quick-action-sep" aria-hidden>
            ·
          </span>}
          <button
            type="button"
            className="quick-action"
            disabled={disabled}
            onClick={() => onSelect(t(item.key, lang))}
          >
            <span className="quick-action-icon" aria-hidden>
              {item.icon}
            </span>
            {t(item.key, lang)}
          </button>
        </span>
      ))}
    </nav>
  );
}
