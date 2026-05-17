//! Prompt templates embedded at compile time.

pub const SYSTEM_PROMPT: &str = include_str!("../prompts/system_prompt.txt");
pub const INTENT_CLASSIFIER: &str = include_str!("../prompts/intent_classifier.txt");
pub const INSTRUCTION_STEP: &str = include_str!("../prompts/instruction_step.txt");

pub fn render_intent_classifier(utterance: &str) -> String {
    INTENT_CLASSIFIER.replace("{utterance}", utterance)
}

pub struct InstructionPromptContext<'a> {
    pub goal: &'a str,
    pub step_index: usize,
    pub total_steps: usize,
    pub action: &'a str,
    pub target: &'a str,
    pub window_title: &'a str,
    pub window_list: &'a str,
    pub visible_elements: &'a str,
    pub cursor_line: &'a str,
    pub target_on_screen: bool,
    pub perception_quality: &'a str,
    pub warnings_line: &'a str,
}

pub fn render_instruction_step(ctx: InstructionPromptContext<'_>) -> String {
    let anchor_status = if ctx.target_on_screen {
        "El sistema SÍ localizó el objetivo en pantalla; la persona verá un círculo amarillo ahí."
    } else {
        "El sistema NO localizó el objetivo todavía; guía con palabras hasta que lo encuentre."
    };
    INSTRUCTION_STEP
        .replace("{goal}", ctx.goal)
        .replace("{step_index}", &ctx.step_index.to_string())
        .replace("{total_steps}", &ctx.total_steps.to_string())
        .replace("{action}", ctx.action)
        .replace("{target}", ctx.target)
        .replace("{window_title}", ctx.window_title)
        .replace("{window_list}", ctx.window_list)
        .replace("{visible_elements}", ctx.visible_elements)
        .replace("{cursor_line}", ctx.cursor_line)
        .replace("{anchor_status}", anchor_status)
        .replace("{perception_quality}", ctx.perception_quality)
        .replace("{warnings_line}", ctx.warnings_line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_replaces_all_placeholders() {
        let out = render_instruction_step(InstructionPromptContext {
            goal: "open_folder",
            step_index: 1,
            total_steps: 2,
            action: "Doble clic",
            target: "Descargas",
            window_title: "Explorer",
            window_list: "* Explorer\n  Chrome",
            visible_elements: "- Descargas (Button)",
            cursor_line: "Pos: (10, 20)",
            target_on_screen: true,
            perception_quality: "full",
            warnings_line: "",
        });
        assert!(out.contains("Descargas"));
        assert!(out.contains("full"));
        assert!(out.contains("Doble clic"));
        assert!(!out.contains("{target}"));
        assert!(!out.contains("{warnings_line}"));
    }
}
