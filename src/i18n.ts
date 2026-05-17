export type Lang = "es" | "en";



const ES: Record<string, string> = {

  "app.title": "Roota",

  "app.subtitle": "Tu compañera digital",

  "main.greeting": "¿Qué tarea quieres que haga por ti hoy?",

  "main.input_placeholder": "Escribe lo que quieres hacer…",

  "main.send_button": "Empezar",

  "main.cancel": "Cancelar guía",

  "main.examples_title": "Prueba con una de estas frases:",

  "confirm.title": "¿Está bien?",

  "confirm.body": "Voy a {action}. ¿Está bien?",

  "confirm.yes": "SÍ",

  "confirm.no": "NO",

  "feedback.step_label": "Paso {step} de {total}",

  "feedback.success_title": "¡Perfecto!",

  "feedback.success_body": "Sigamos con el siguiente paso.",

  "feedback.completed_title": "¡Listo!",

  "feedback.completed_body": "Hemos terminado la tarea juntos.",

  "feedback.error_title": "Vamos a intentarlo de nuevo",

  "feedback.cancelled_title": "De acuerdo",

  "feedback.cancelled_body": "No haremos nada. Cuando quieras, escribe otra tarea.",

  "feedback.classifying": "Estoy entendiendo lo que necesitas…",

  "feedback.waiting_confirm": "Confirma con SÍ o NO para continuar.",

  "guidance.overlay_hint": "Mira el resaltado en pantalla — ahí debes actuar.",

  "guidance.overlay_missing": "Busca «{target}» en la ventana abierta.",

  "guidance.safety_note": "Roota solo te guía; nunca hace clic ni escribe por ti.",
  "guidance.hint.click": "Haz clic aquí",
  "guidance.hint.double_click": "Doble clic aquí",
  "guidance.hint.right_click": "Clic derecho aquí",
  "guidance.hint.type": "Escribe aquí",
  "guidance.hint.locate": "Busca aquí",
  "guidance.hud_no_target": "Busca «{target}» en la ventana que tienes abierta.",
  "guidance.observing": "Estoy mirando tu pantalla para planear los pasos…",
  "guidance.replanning": "Voy a buscar otra forma de ayudarte con este paso…",
  "guidance.plan_preview_title": "Así te voy a guiar:",
  "guidance.stuck_button": "No lo veo",

  "action.click": "Clic con el botón izquierdo",

  "action.double_click": "Doble clic",

  "action.right_click": "Clic derecho",

  "action.type": "Escribir texto",

  "action.locate": "Buscar en pantalla",

  "example.open_folder": "Abre la carpeta Descargas",

  "example.open_browser": "Abre el navegador",

  "example.compose_email": "Escribe un correo a mi hija",
  "panel.shortcut_hint": "Pulsa Ctrl+Mayús+Espacio para ocultar o mostrar Roota",
  "panel.hide": "Ocultar",
};



const EN: Record<string, string> = {

  "app.title": "Roota",

  "app.subtitle": "Your digital companion",

  "main.greeting": "What task can I help you with today?",

  "main.input_placeholder": "Tell me what you'd like to do…",

  "main.send_button": "Start",

  "main.cancel": "Cancel guidance",

  "main.examples_title": "Try one of these phrases:",

  "confirm.title": "Is this okay?",

  "confirm.body": "I'm going to {action}. Is that okay?",

  "confirm.yes": "YES",

  "confirm.no": "NO",

  "feedback.step_label": "Step {step} of {total}",

  "feedback.success_title": "Perfect!",

  "feedback.success_body": "Let's go to the next step.",

  "feedback.completed_title": "All done!",

  "feedback.completed_body": "We finished the task together.",

  "feedback.error_title": "Let's try again",

  "feedback.cancelled_title": "Okay",

  "feedback.cancelled_body": "We won't do anything. When you're ready, ask for another task.",

  "feedback.classifying": "Understanding what you need…",

  "feedback.waiting_confirm": "Press YES or NO to continue.",

  "guidance.overlay_hint": "Look for the highlight on your screen — that's where to act.",

  "guidance.overlay_missing": "Find «{target}» in the open window.",

  "guidance.safety_note": "Roota only guides you; it never clicks or types for you.",
  "guidance.hint.click": "Click here",
  "guidance.hint.double_click": "Double-click here",
  "guidance.hint.right_click": "Right-click here",
  "guidance.hint.type": "Type here",
  "guidance.hint.locate": "Look here",
  "guidance.hud_no_target": "Find «{target}» in the window you have open.",
  "guidance.observing": "I'm looking at your screen to plan the steps…",
  "guidance.replanning": "I'll try another way to help you with this step…",
  "guidance.plan_preview_title": "Here's how I'll guide you:",
  "guidance.stuck_button": "I can't see it",

  "action.click": "Left-click",

  "action.double_click": "Double-click",

  "action.right_click": "Right-click",

  "action.type": "Type text",

  "action.locate": "Find on screen",

  "example.open_folder": "Open the Downloads folder",

  "example.open_browser": "Open the web browser",

  "example.compose_email": "Write an email to my daughter",
  "panel.shortcut_hint": "Press Ctrl+Shift+Space to hide or show Roota",
  "panel.hide": "Hide",
};



const CATALOGS: Record<Lang, Record<string, string>> = { es: ES, en: EN };



export function t(

  key: string,

  lang: Lang = "es",

  args: Record<string, string | number> = {},

): string {

  const catalog = CATALOGS[lang] ?? ES;

  let template = catalog[key] ?? ES[key] ?? key;

  for (const [name, value] of Object.entries(args)) {

    template = template.replace(`{${name}}`, String(value));

  }

  return template;

}



export function actionLabel(action: string, lang: Lang = "es"): string {

  return t(`action.${action}`, lang);

}


