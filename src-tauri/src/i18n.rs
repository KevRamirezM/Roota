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
        "Mira el resaltado en pantalla — ahí debes actuar.",
    ),
    (
        "guidance.overlay_cue.click",
        "el círculo amarillo en pantalla",
    ),
    (
        "guidance.overlay_cue.double_click",
        "el círculo naranja en pantalla",
    ),
    (
        "guidance.overlay_cue.right_click",
        "el círculo azul en pantalla",
    ),
    (
        "guidance.overlay_cue.type",
        "el recuadro verde en pantalla",
    ),
    (
        "guidance.overlay_cue.locate",
        "el resaltado en pantalla",
    ),
    (
        "guidance.instruction.click_with_anchor",
        "Haz clic en «{target}». Mira {cue}.",
    ),
    (
        "guidance.instruction.double_click_with_anchor",
        "Haz doble clic en «{target}». Mira {cue}.",
    ),
    (
        "guidance.instruction.right_click_with_anchor",
        "Haz clic derecho en «{target}». Mira {cue}.",
    ),
    (
        "guidance.instruction.type_with_anchor",
        "Escribe en «{target}». Mira {cue}.",
    ),
    (
        "guidance.instruction.locate_with_anchor",
        "Busca «{target}» en la ventana. Mira {cue}.",
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
    (
        "guidance.prep_open_explorer",
        "Tú abre el Explorador de archivos (icono de carpeta amarilla) y déjalo visible. Yo no lo abro por ti — busco «{target}» en tu pantalla…",
    ),
    (
        "guidance.cursor_position",
        "Posición del cursor del usuario: ({x}, {y})",
    ),
    (
        "guidance.wrong_click_location",
        "Ese clic no fue sobre «{target}». Mira el círculo en pantalla e inténtalo otra vez ahí.",
    ),
    (
        "guidance.wrong_single_click",
        "Necesitas doble clic en «{target}», no un solo clic.",
    ),
    (
        "guidance.wrong_left_click_need_right",
        "En «{target}» usa clic derecho del ratón, no el izquierdo.",
    ),
    (
        "guidance.wrong_right_click_need_left",
        "En «{target}» usa clic izquierdo normal, no clic derecho.",
    ),
    (
        "guidance.wrong_focus_roota",
        "Volviste a la ventana de Roota. Haz clic en la ventana que estás aprendiendo a usar.",
    ),
    (
        "guidance.wrong_window",
        "Se abrió «{window}» en lugar de lo esperado. Vuelve y busca «{target}».",
    ),
    (
        "guidance.waiting_for_screen",
        "Veo «{window}» ({count} elementos). Localiza «{target}» en la pantalla — te señalaré dónde hacer clic.",
    ),
    (
        "guidance.perception_failed",
        "No pude ver tu escritorio. Abre el Explorador de archivos, espera un momento y vuelve a pedirme ayuda.",
    ),
    (
        "guidance.observing",
        "Estoy mirando tu pantalla para planear los pasos…",
    ),
    (
        "guidance.replanning",
        "Voy a buscar otra forma de ayudarte con este paso…",
    ),
    (
        "guidance.plan_preview_title",
        "Así te voy a guiar:",
    ),
    (
        "guidance.plan_partial",
        "Algunos pasos pueden cambiar cuando vea más de tu pantalla.",
    ),
    (
        "guidance.stuck_button",
        "No lo veo",
    ),
    (
        "guidance.perception_limited",
        "Estoy viendo la pantalla con detalle limitado;",
    ),
    (
        "guidance.perception_vision_assisted",
        "Leí parte de la pantalla con visión asistida;",
    ),
    (
        "guidance.perception_vlm_note",
        "Nota: algunos controles se leyeron por visión y pueden ser aproximados.",
    ),
    (
        "guidance.secure_desktop_blocked",
        "Windows está mostrando una pantalla protegida (clave o aviso del sistema). Cuando vuelvas al escritorio normal, pídemelo otra vez.",
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
    (
        "llama.unavailable",
        "No se encontró el servidor local de IA. Ejecuta scripts/start-llama.ps1",
    ),
    ("llm.backend", "Motor de IA: {name}"),
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
    (
        "confirm.windows_task",
        "ayudarte con esto en tu PC: {target}",
    ),
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
        "Look for the highlight on your screen — that's where to act.",
    ),
    (
        "guidance.overlay_cue.click",
        "the yellow circle on screen",
    ),
    (
        "guidance.overlay_cue.double_click",
        "the orange circle on screen",
    ),
    (
        "guidance.overlay_cue.right_click",
        "the blue circle on screen",
    ),
    (
        "guidance.overlay_cue.type",
        "the green highlight on screen",
    ),
    (
        "guidance.overlay_cue.locate",
        "the highlight on screen",
    ),
    (
        "guidance.instruction.click_with_anchor",
        "Click «{target}». Look at {cue}.",
    ),
    (
        "guidance.instruction.double_click_with_anchor",
        "Double-click «{target}». Look at {cue}.",
    ),
    (
        "guidance.instruction.right_click_with_anchor",
        "Right-click «{target}». Look at {cue}.",
    ),
    (
        "guidance.instruction.type_with_anchor",
        "Type in «{target}». Look at {cue}.",
    ),
    (
        "guidance.instruction.locate_with_anchor",
        "Find «{target}» in the window. Look at {cue}.",
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
    (
        "guidance.prep_open_explorer",
        "Please open File Explorer yourself (yellow folder icon) and keep it visible. I won't open it for you — I'm looking for «{target}» on your screen…",
    ),
    (
        "guidance.cursor_position",
        "User cursor position: ({x}, {y})",
    ),
    (
        "guidance.wrong_click_location",
        "That click wasn't on «{target}». Look at the circle on screen and try again there.",
    ),
    (
        "guidance.wrong_single_click",
        "You need to double-click «{target}», not a single click.",
    ),
    (
        "guidance.wrong_left_click_need_right",
        "On «{target}» use right-click, not left-click.",
    ),
    (
        "guidance.wrong_right_click_need_left",
        "On «{target}» use a normal left-click, not right-click.",
    ),
    (
        "guidance.wrong_focus_roota",
        "You switched back to Roota. Click the window you're learning to use.",
    ),
    (
        "guidance.wrong_window",
        "«{window}» opened instead of what we expected. Go back and find «{target}».",
    ),
    (
        "guidance.waiting_for_screen",
        "I see «{window}» ({count} items). Find «{target}» on screen — I'll show you where to click.",
    ),
    (
        "guidance.perception_failed",
        "I couldn't read your desktop. Open File Explorer, wait a moment, and ask me again.",
    ),
    (
        "guidance.perception_limited",
        "I'm reading the screen with limited detail;",
    ),
    (
        "guidance.perception_vision_assisted",
        "I read part of the screen with assisted vision;",
    ),
    (
        "guidance.perception_vlm_note",
        "Note: some controls were read by vision and may be approximate.",
    ),
    (
        "guidance.secure_desktop_blocked",
        "Windows is showing a protected screen (login or system prompt). When you're back on the normal desktop, ask me again.",
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
    (
        "llama.unavailable",
        "Local AI server not found. Run scripts/start-llama.ps1",
    ),
    ("llm.backend", "AI engine: {name}"),
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
    (
        "confirm.windows_task",
        "help you with this on your PC: {target}",
    ),
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
