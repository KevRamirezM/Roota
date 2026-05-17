import { actionLabel, type Lang } from "../i18n";
import type { ActionVerb } from "../types";

interface Props {
  action: ActionVerb;
  lang?: Lang;
}

export function ActionBadge({ action, lang = "es" }: Props) {
  return <span className="action-badge">{actionLabel(action, lang)}</span>;
}
