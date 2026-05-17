//! Safety layer — Roota's irrevocable runtime guarantee.
//!
//! PRD §8.9 mandates that Roota *never* directly controls input
//! hardware: no synthetic clicks, no synthetic keystrokes. Every
//! emitted action passes through `SafetyGuard::review`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Highlight,
    Anchor,
    Arrow,
    Speak,
    ShowText,
    Scan,
    Click,
    DoubleClick,
    RightClick,
    TypeText,
    KeyPress,
    Drag,
    Scroll,
    FileOp,
}

const GUIDE_ACTIONS: &[ActionType] = &[
    ActionType::Highlight,
    ActionType::Anchor,
    ActionType::Arrow,
    ActionType::Speak,
    ActionType::ShowText,
    ActionType::Scan,
];

const AUTOMATION_ACTIONS: &[ActionType] = &[
    ActionType::Click,
    ActionType::DoubleClick,
    ActionType::RightClick,
    ActionType::TypeText,
    ActionType::KeyPress,
    ActionType::Drag,
    ActionType::Scroll,
    ActionType::FileOp,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideAction {
    #[serde(rename = "type")]
    pub kind: ActionType,
    pub target: Option<String>,
    pub payload: Option<String>,
}

#[derive(Debug, Error)]
pub enum UnsafeActionError {
    #[error("Refusing to emit automating action {0:?}; Roota only guides, it never executes.")]
    Automating(ActionType),
    #[error("Unknown action type {0:?}; refusing under strict mode.")]
    Unknown(ActionType),
}

pub struct SafetyGuard {
    strict: bool,
}

impl Default for SafetyGuard {
    fn default() -> Self {
        Self { strict: true }
    }
}

impl SafetyGuard {
    pub fn new(strict: bool) -> Self {
        Self { strict }
    }

    pub fn review(&self, action: GuideAction) -> Result<GuideAction, UnsafeActionError> {
        if AUTOMATION_ACTIONS.contains(&action.kind) {
            return Err(UnsafeActionError::Automating(action.kind));
        }
        if !GUIDE_ACTIONS.contains(&action.kind) && self.strict {
            return Err(UnsafeActionError::Unknown(action.kind));
        }
        Ok(action)
    }

    pub fn is_safe(&self, action: &GuideAction) -> bool {
        let cloned = action.clone();
        self.review(cloned).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(kind: ActionType) -> GuideAction {
        GuideAction {
            kind,
            target: Some("Descargas".into()),
            payload: None,
        }
    }

    #[test]
    fn guide_actions_pass() {
        let guard = SafetyGuard::default();
        for kind in [
            ActionType::Highlight,
            ActionType::Anchor,
            ActionType::Arrow,
            ActionType::Speak,
            ActionType::ShowText,
            ActionType::Scan,
        ] {
            assert!(guard.is_safe(&act(kind)));
        }
    }

    #[test]
    fn automation_actions_rejected() {
        let guard = SafetyGuard::default();
        for kind in [
            ActionType::Click,
            ActionType::DoubleClick,
            ActionType::RightClick,
            ActionType::TypeText,
            ActionType::KeyPress,
            ActionType::Drag,
            ActionType::Scroll,
            ActionType::FileOp,
        ] {
            assert!(!guard.is_safe(&act(kind)));
            assert!(matches!(
                guard.review(act(kind)),
                Err(UnsafeActionError::Automating(_))
            ));
        }
    }
}
