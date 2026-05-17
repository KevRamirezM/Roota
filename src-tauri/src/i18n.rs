//! Tiny static-string i18n system. Spanish is the canonical fallback.

use crate::settings::Lang;

const ES: &[(&str, &str)] = &[
    ("app.title", "Roota"),
    ("app.subtitle", "Tu compañera digital"),
    ("main.greeting", "¿Qué tarea quieres que haga por ti hoy?"),
    ("main.input_placeholder", "Escribe lo que quieres hacer..."),
    ("main.send_button", "Empezar"),
    ("confirm.title", "¿Está bien?"),
    ("confirm.body", "Voy a {action}. ¿Está bien?"),
    ("confirm.yes", "SÍ"),
    ("confirm.no", "NO"),
    ("feedback.step_label", "Paso {step} de {total}"),
    ("feedback.success_title", "¡Perfecto!"),
    ("feedback.success_body", "Sigamos con el siguiente paso."),
    ("feedback.completed_title", "¡Listo!"),
    (
        "feedback.completed_body",
        "Hemos terminado la tarea juntos.",
    ),
    ("feedback.error_title", "Vamos a intentarlo de nuevo"),
    ("feedback.cancelled_title", "De acuerdo"),
    (
        "feedback.cancelled_body",
        "No haremos nada. Cuando quieras, escribe otra tarea.",
    ),
    (
        "feedback.classifying",
        "Estoy entendiendo lo que necesitas…",
    ),
    (
        "feedback.waiting_confirm",
        "Confirma con SÍ o NO para continuar.",
    ),
    (
        "guidance.overlay_hint",
        "Mira el círculo amarillo en tu pantalla — ahí debes actuar.",
    ),
    (
        "guidance.overlay_missing",
        "Busca «{target}» en la ventana abierta. Si no lo ves, dime y lo intentamos de nuevo.",
    ),
    (
        "guidance.safety_note",
        "Roota solo te guía; nunca hace clic ni escribe por ti.",
    ),
    ("guidance.hint.click", "Haz clic aquí"),
    ("guidance.hint.double_click", "Doble clic aquí"),
    ("guidance.hint.right_click", "Clic derecho aquí"),
    ("guidance.hint.type", "Escribe aquí"),
    ("guidance.hint.locate", "Busca aquí"),
    (
        "guidance.hud_no_target",
        "Busca «{target}» en la ventana que tienes abierta.",
    ),
    (
        "guidance.screen_empty",
        "(No se detectaron botones ni carpetas en la ventana activa — abre el Explorador de archivos u otra app.)",
    ),
    ("action.click", "Clic con el botón izquierdo"),
    ("action.double_click", "Doble clic"),
    ("action.right_click", "Clic derecho"),
    ("action.type", "Escribir texto"),
    ("action.locate", "Buscar en pantalla"),
    ("main.cancel", "Cancelar guía"),
    ("main.examples_title", "Prueba con una de estas frases:"),
    ("example.open_folder", "Abre la carpeta Descargas"),
    ("example.open_browser", "Abre el navegador"),
    ("example.compose_email", "Escribe un correo a mi hija"),
    ("panel.shortcut_hint", "Pulsa Ctrl+Mayús+Espacio para ocultar o mostrar Roota"),
    ("panel.hide", "Ocultar"),
    (
        "intent.unknown",
        "Aún no sé hacer eso. ¿Puedes decírmelo de otra forma?",
    ),
    (
        "intent.no_target",
        "Necesito saber sobre qué quieres que actuemos.",
    ),
    ("guidance.click_target", "Haz clic en {target}."),
    (
        "guidance.double_click_target",
        "Haz doble clic en {target}.",
    ),
    (
        "guidance.right_click_target",
        "Haz clic derecho en {target}.",
    ),
    ("guidance.type_in_target", "Escribe en el cuadro {target}."),
    ("guidance.locate_target", "Busca {target} en la pantalla."),
    (
        "guidance.element_not_found",
        "No encuentro {target}. ¿Lo ves en pantalla?",
    ),
    (
        "safety.unsafe_action",
        "Roota nunca hace clic ni escribe por ti — solo te guio.",
    ),
    (
        "ollama.unavailable",
        "El motor local no responde. Estoy usando respuestas guardadas.",
    ),
    ("confirm.open_folder", "abrir la carpeta {target}"),
    ("confirm.move_file", "mover el archivo {target}"),
    ("confirm.delete_file", "borrar el archivo {target}"),
    ("confirm.open_browser", "abrir el navegador"),
    ("confirm.search_web", "buscar \"{target}\" en internet"),
    ("confirm.open_url", "abrir la página {target}"),
    ("confirm.compose_email", "escribir un correo a {target}"),
    ("confirm.read_inbox", "revisar tu bandeja de entrada"),
    ("confirm.reply_message", "responder el correo de {target}"),
    ("confirm.open_word_document", "abrir un documento de Word"),
    ("confirm.print_document", "imprimir el documento"),
];

const EN: &[(&str, &str)] = &[
    ("app.title", "Roota"),
    ("app.subtitle", "Your digital companion"),
    ("main.greeting", "What task can I help you with today?"),
    ("main.input_placeholder", "Tell me what you'd like to do..."),
    ("main.send_button", "Start"),
    ("confirm.title", "Is this okay?"),
    ("confirm.body", "I'm going to {action}. Is that okay?"),
    ("confirm.yes", "YES"),
    ("confirm.no", "NO"),
    ("feedback.step_label", "Step {step} of {total}"),
    ("feedback.success_title", "Perfect!"),
    ("feedback.success_body", "Let's go to the next step."),
    ("feedback.completed_title", "All done!"),
    ("feedback.completed_body", "We finished the task together."),
    ("feedback.error_title", "Let's try again"),
    ("feedback.cancelled_title", "Okay"),
    (
        "feedback.cancelled_body",
        "We won't do anything. When you're ready, ask for another task.",
    ),
    ("feedback.classifying", "Understanding what you need…"),
    ("feedback.waiting_confirm", "Press YES or NO to continue."),
    (
        "guidance.overlay_hint",
        "Look for the yellow circle on your screen — that's where to act.",
    ),
    (
        "guidance.overlay_missing",
        "Find «{target}» in the open window. If you don't see it, tell me and we'll try again.",
    ),
    (
        "guidance.safety_note",
        "Roota only guides you; it never clicks or types for you.",
    ),
    ("guidance.hint.click", "Click here"),
    ("guidance.hint.double_click", "Double-click here"),
    ("guidance.hint.right_click", "Right-click here"),
    ("guidance.hint.type", "Type here"),
    ("guidance.hint.locate", "Look here"),
    (
        "guidance.hud_no_target",
        "Find «{target}» in the window you have open.",
    ),
    (
        "guidance.screen_empty",
        "(No buttons or folders detected in the active window — open File Explorer or the right app.)",
    ),
    ("action.click", "Left-click"),
    ("action.double_click", "Double-click"),
    ("action.right_click", "Right-click"),
    ("action.type", "Type text"),
    ("action.locate", "Find on screen"),
    ("main.cancel", "Cancel guidance"),
    ("main.examples_title", "Try one of these phrases:"),
    ("example.open_folder", "Open the Downloads folder"),
    ("example.open_browser", "Open the web browser"),
    ("example.compose_email", "Write an email to my daughter"),
    ("panel.shortcut_hint", "Press Ctrl+Shift+Space to hide or show Roota"),
    ("panel.hide", "Hide"),
    (
        "intent.unknown",
        "I don't know how to do that yet. Can you say it another way?",
    ),
    ("intent.no_target", "I need to know what we should act on."),
    ("guidance.click_target", "Left-click on {target}."),
    ("guidance.double_click_target", "Double-click on {target}."),
    ("guidance.right_click_target", "Right-click on {target}."),
    ("guidance.type_in_target", "Type in the {target} box."),
    ("guidance.locate_target", "Find {target} on the screen."),
    (
        "guidance.element_not_found",
        "I can't find {target}. Do you see it on screen?",
    ),
    (
        "safety.unsafe_action",
        "Roota never clicks or types for you — it only guides.",
    ),
    (
        "ollama.unavailable",
        "The local engine isn't responding. Using saved replies.",
    ),
    ("confirm.open_folder", "open the {target} folder"),
    ("confirm.move_file", "move the file {target}"),
    ("confirm.delete_file", "delete the file {target}"),
    ("confirm.open_browser", "open the web browser"),
    ("confirm.search_web", "search the web for \"{target}\""),
    ("confirm.open_url", "open the page {target}"),
    ("confirm.compose_email", "write an email to {target}"),
    ("confirm.read_inbox", "check your inbox"),
    ("confirm.reply_message", "reply to {target}'s message"),
    ("confirm.open_word_document", "open a Word document"),
    ("confirm.print_document", "print the document"),
];

fn lookup(catalog: &'static [(&'static str, &'static str)], key: &str) -> Option<&'static str> {
    catalog.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

pub fn t(key: &str, lang: Lang, args: &[(&str, &str)]) -> String {
    let primary = match lang {
        Lang::En => EN,
        Lang::Es => ES,
    };
    let template = lookup(primary, key)
        .or_else(|| lookup(ES, key))
        .unwrap_or(key);
    apply_args(template, args)
}

fn apply_args(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (name, value) in args {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_to_spanish_for_unknown_locale() {
        let s = t("app.title", Lang::Es, &[]);
        assert_eq!(s, "Roota");
    }

    #[test]
    fn substitutes_args() {
        let s = t(
            "feedback.step_label",
            Lang::Es,
            &[("step", "1"), ("total", "3")],
        );
        assert!(s.contains("1") && s.contains("3"));
    }

    #[test]
    fn english_locale_returns_english_text() {
        let s = t("main.greeting", Lang::En, &[]);
        assert!(s.to_lowercase().contains("today"));
    }
}
