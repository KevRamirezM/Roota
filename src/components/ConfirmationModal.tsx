import { useEffect, useRef } from "react";
import { t, type Lang } from "../i18n";

interface Props {
  open: boolean;
  message: string;
  lang?: Lang;
  onAccept: () => void;
  onReject: () => void;
}

export function ConfirmationModal({ open, message, lang = "es", onAccept, onReject }: Props) {
  const dialogRef = useRef<HTMLDialogElement | null>(null);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "y" || e.key === "Y" || e.key === "Enter") {
        e.preventDefault();
        onAccept();
      } else if (e.key === "n" || e.key === "N" || e.key === "Escape") {
        e.preventDefault();
        onReject();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onAccept, onReject]);

  return (
    <dialog ref={dialogRef} className="confirm-dialog" aria-labelledby="confirm-title">
      <h2 id="confirm-title" className="confirm-title">
        {t("confirm.title", lang)}
      </h2>
      <p className="confirm-body">{t("confirm.body", lang, { action: message })}</p>
      <div className="confirm-actions">
        <button type="button" className="confirm-button yes" onClick={onAccept} autoFocus>
          {t("confirm.yes", lang)}
        </button>
        <button type="button" className="confirm-button no" onClick={onReject}>
          {t("confirm.no", lang)}
        </button>
      </div>
    </dialog>
  );
}
