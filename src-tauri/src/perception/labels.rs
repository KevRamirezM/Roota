//! Human-readable labels for UIA/OCR text — filters internal automation names.

/// Map known automation ids / internal names to Spanish UI labels.
pub fn humanize_label(name: &str, automation_id: Option<&str>) -> Option<String> {
    let trimmed = name.trim();
    if let Some(id) = automation_id {
        if let Some(label) = map_known_id(id) {
            return Some(label);
        }
    }
    if let Some(label) = map_known_id(trimmed) {
        return Some(label);
    }
    if is_internal_ui_label(trimmed) {
        return None;
    }
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// True when a string looks like a developer automation id, not user-visible text.
pub fn is_internal_ui_label(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() || t.chars().count() > 64 {
        return false;
    }
    if map_known_id(t).is_some() {
        return true;
    }
    // kebab-case: session-start, system-tray-overflow
    if t.contains('-')
        && !t.contains(' ')
        && t.chars()
            .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit())
    {
        return true;
    }
    // camelCase / PascalCase without spaces (SessionStartButton)
    if !t.contains(' ')
        && t.chars().any(|c| c.is_ascii_uppercase())
        && t.chars().any(|c| c.is_ascii_lowercase())
        && t.chars().all(|c| c.is_ascii_alphanumeric())
        && t.chars().count() >= 8
    {
        return true;
    }
    false
}

/// Sanitize a planner/LLM target before showing it to the user.
pub fn sanitize_plan_target(raw: &str) -> String {
    let trimmed = raw.trim();
    humanize_label(trimmed, None).unwrap_or_else(|| {
        if is_internal_ui_label(trimmed) {
            String::new()
        } else {
            trimmed.to_string()
        }
    })
}

fn map_known_id(id: &str) -> Option<String> {
    match id.to_lowercase().as_str() {
        "start" | "session-start" | "sessionstart" | "startbutton" | "startexperiencehost" => {
            Some("Inicio".into())
        }
        "searchbox" | "search" | "searchbutton" => Some("Buscar".into()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_start_maps_to_inicio() {
        assert_eq!(
            humanize_label("session-start", Some("session-start")),
            Some("Inicio".into())
        );
    }

    #[test]
    fn internal_kebab_is_filtered() {
        assert!(is_internal_ui_label("session-start"));
        assert_eq!(sanitize_plan_target("session-start"), "Inicio");
    }

    #[test]
    fn normal_labels_pass_through() {
        assert_eq!(humanize_label("Descargas", None), Some("Descargas".into()));
        assert!(!is_internal_ui_label("Google Chrome"));
    }
}
