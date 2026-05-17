use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionVerb {
    Click,
    DoubleClick,
    RightClick,
    Type,
    Locate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Intent {
    pub intent: String,
    pub target: String,
    #[serde(default)]
    pub params: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub raw_utterance: String,
}

impl Intent {
    pub fn unknown(raw: &str) -> Self {
        Intent {
            intent: "unknown".into(),
            target: String::new(),
            params: Default::default(),
            raw_utterance: raw.to_string(),
        }
    }

    pub fn is_known(&self) -> bool {
        self.intent != "unknown"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideStep {
    pub index: usize,
    pub total: usize,
    pub action: ActionVerb,
    pub target_text: String,
    pub instruction: String,
    /// Screen-space center (physical pixels) from UI Automation.
    pub anchor_xy: Option<(i32, i32)>,
    /// Screen-space bounding box (physical pixels): x, y, width, height.
    pub anchor_bounds: Option<(i32, i32, i32, i32)>,
}

#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub goal: String,
    pub intent: Option<Intent>,
    pub step_index: usize,
    pub total_steps: usize,
    pub completed: bool,
    pub history: Vec<GuideStep>,
}

impl SessionState {
    pub fn begin(&mut self, intent: Intent, total_steps: usize) {
        self.goal = intent.intent.clone();
        self.intent = Some(intent);
        self.step_index = 0;
        self.total_steps = total_steps;
        self.completed = false;
        self.history.clear();
    }

    /// Records a delivered step. `step_index` stays the 0-based blueprint cursor
    /// (do not overwrite with `step.index`, which is 1-based for display).
    pub fn record(&mut self, step: GuideStep) {
        self.history.push(step);
    }

    /// Marks the current blueprint step done and advances the cursor.
    pub fn advance(&mut self) {
        self.step_index += 1;
        if self.step_index >= self.total_steps {
            self.completed = true;
        }
    }

    pub fn reset(&mut self) {
        *self = SessionState::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_marks_completed_only_after_all_steps() {
        let mut s = SessionState::default();
        s.begin(Intent::unknown("hi"), 2);
        assert_eq!(s.step_index, 0);
        s.advance();
        assert_eq!(s.step_index, 1);
        assert!(!s.completed);
        s.advance();
        assert_eq!(s.step_index, 2);
        assert!(s.completed);
    }

    #[test]
    fn record_does_not_advance_cursor() {
        let mut s = SessionState::default();
        s.begin(Intent::unknown("hi"), 2);
        s.record(GuideStep {
            index: 1,
            total: 2,
            action: ActionVerb::Click,
            target_text: "x".into(),
            instruction: "y".into(),
            anchor_xy: None,
            anchor_bounds: None,
        });
        assert_eq!(s.step_index, 0);
    }

    #[test]
    fn intent_is_known_predicate() {
        let i = Intent {
            intent: "open_folder".into(),
            target: "Descargas".into(),
            params: Default::default(),
            raw_utterance: "abre".into(),
        };
        assert!(i.is_known());
        assert!(!Intent::unknown("?").is_known());
    }
}
