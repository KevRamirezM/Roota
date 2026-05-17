//! Prompt templates embedded at compile time.

pub const SYSTEM_PROMPT: &str = include_str!("../prompts/system_prompt.txt");
pub const INTENT_CLASSIFIER: &str = include_str!("../prompts/intent_classifier.txt");
pub const INSTRUCTION_STEP: &str = include_str!("../prompts/instruction_step.txt");
pub const TASK_PLANNER: &str = include_str!("../prompts/task_planner.txt");
pub const TASK_BRIEF: &str = include_str!("../prompts/task_brief.txt");
pub const WINDOWS_DESKTOP_GUIDE: &str = include_str!("../prompts/windows_desktop_guide.txt");
pub const WINDOWS_DESKTOP_HINTS: &str = include_str!("../prompts/windows_desktop_hints.txt");

pub fn render_intent_classifier(utterance: &str, allowed_intents: &[String]) -> String {
    let mut lines: Vec<String> = allowed_intents
        .iter()
        .map(|i| format!("- {i}"))
        .collect();
    if !lines.iter().any(|l| l.contains("windows_task")) {
        lines.push("- windows_task".into());
    }
    if !lines.iter().any(|l| l.contains("unknown")) {
        lines.push("- unknown".into());
    }
    INTENT_CLASSIFIER
        .replace("{allowed_intents}", &lines.join("\n"))
        .replace("{utterance}", utterance)
}

pub fn render_task_brief(utterance: &str, goal_target: &str) -> String {
    TASK_BRIEF
        .replace("{windows_hints}", WINDOWS_DESKTOP_HINTS)
        .replace("{utterance}", utterance)
        .replace("{goal_target}", goal_target)
}

pub struct TaskPlannerContext<'a> {
    pub utterance: &'a str,
    pub goal_target: &'a str,
    pub task_brief_block: &'a str,
    pub window_list: &'a str,
    pub visible_elements: &'a str,
}

pub fn render_task_planner(ctx: TaskPlannerContext<'_>) -> String {
    TASK_PLANNER
        .replace("{windows_guide}", WINDOWS_DESKTOP_GUIDE)
        .replace("{utterance}", ctx.utterance)
        .replace("{goal_target}", ctx.goal_target)
        .replace("{task_brief_block}", ctx.task_brief_block)
        .replace("{window_list}", ctx.window_list)
        .replace("{visible_elements}", ctx.visible_elements)
}

pub struct InstructionPromptContext<'a> {
    pub goal_summary: &'a str,
    pub step_index: usize,
    pub total_steps: usize,
    pub click_hint: &'a str,
    pub overlay_cue: &'a str,
    pub target: &'a str,
    pub window_title: &'a str,
    pub window_list: &'a str,
    pub visible_elements: &'a str,
    pub spatial_hint: &'a str,
    pub cursor_line: &'a str,
    pub target_on_screen: bool,
    pub perception_quality: &'a str,
    pub warnings_line: &'a str,
    pub element_source_note: &'a str,
}

pub fn render_instruction_step(ctx: InstructionPromptContext<'_>) -> String {
    let anchor_status = if ctx.target_on_screen {
        "Estado: el resaltado en pantalla ya marca el objetivo; tu frase debe coincidir con ese lugar."
    } else {
        "Estado: aún no hay resaltado; guía con palabras hasta que aparezca."
    };
    INSTRUCTION_STEP
        .replace("{goal_summary}", ctx.goal_summary)
        .replace("{step_index}", &ctx.step_index.to_string())
        .replace("{total_steps}", &ctx.total_steps.to_string())
        .replace("{click_hint}", ctx.click_hint)
        .replace("{overlay_cue}", ctx.overlay_cue)
        .replace("{target}", ctx.target)
        .replace("{window_title}", ctx.window_title)
        .replace("{window_list}", ctx.window_list)
        .replace("{visible_elements}", ctx.visible_elements)
        .replace("{spatial_hint}", ctx.spatial_hint)
        .replace("{cursor_line}", ctx.cursor_line)
        .replace("{anchor_status}", anchor_status)
        .replace("{perception_quality}", ctx.perception_quality)
        .replace("{warnings_line}", ctx.warnings_line)
        .replace("{element_source_note}", ctx.element_source_note)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_planner_includes_windows_guide() {
        let out = render_task_planner(TaskPlannerContext {
            utterance: "abre wifi",
            goal_target: "Wi‑Fi",
            task_brief_block: "test",
            window_list: "Chrome",
            visible_elements: "Configuración",
        });
        assert!(!out.contains("{windows_guide}"));
        assert!(out.contains("Win+E"));
        assert!(out.len() > WINDOWS_DESKTOP_GUIDE.len() / 2);
    }

    #[test]
    fn task_brief_includes_windows_hints() {
        let out = render_task_brief("abre descargas", "Descargas");
        assert!(out.contains("Win+E"));
        assert!(!out.contains("{windows_hints}"));
    }

    #[test]
    fn render_replaces_all_placeholders() {
        let out = render_instruction_step(InstructionPromptContext {
            goal_summary: "abrir la carpeta Descargas",
            step_index: 1,
            total_steps: 2,
            click_hint: "Doble clic aquí",
            overlay_cue: "el círculo naranja en pantalla",
            target: "Descargas",
            window_title: "Explorer",
            window_list: "* Explorer\n  Chrome",
            visible_elements: "→ OBJETIVO - Descargas (Button)",
            spatial_hint: "«Descargas» en Explorer",
            cursor_line: "Pos: (10, 20)",
            target_on_screen: true,
            perception_quality: "full",
            warnings_line: "",
            element_source_note: "",
        });
        assert!(out.contains("Descargas"));
        assert!(out.contains("full"));
        assert!(out.contains("Doble clic aquí"));
        assert!(out.contains("círculo naranja"));
        assert!(!out.contains("{target}"));
        assert!(!out.contains("{warnings_line}"));
    }
}
