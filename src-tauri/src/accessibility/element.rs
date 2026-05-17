use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiElement {
    #[serde(rename = "type")]
    pub kind: String,
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub automation_id: Option<String>,
    pub window: String,
}

impl UiElement {
    pub fn center(&self) -> (i32, i32) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }

    pub fn matches(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return false;
        }
        let text = self.text.to_lowercase();
        if text.contains(&q) {
            return true;
        }
        if let Some(id) = &self.automation_id {
            if id.to_lowercase().contains(&q) {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UiSnapshot {
    pub window: String,
    pub elements: Vec<UiElement>,
}

impl UiSnapshot {
    pub fn find(&self, query: &str) -> Option<&UiElement> {
        self.elements.iter().find(|e| e.matches(query))
    }

    /// Best match across several query variants (exact > prefix > contains).
    pub fn find_best<'a>(&'a self, queries: &[String]) -> Option<&'a UiElement> {
        let mut best: Option<(&'a UiElement, i32)> = None;
        for element in &self.elements {
            for query in queries {
                let score = match_score(element, query);
                if score > 0 && best.map(|(_, s)| score > s).unwrap_or(true) {
                    best = Some((element, score));
                }
            }
        }
        best.map(|(e, _)| e)
    }

    /// Compact list of on-screen labels for the LLM (PRD accessibility output).
    pub fn visible_summary(&self, limit: usize) -> String {
        self.elements
            .iter()
            .take(limit)
            .map(|e| format!("- {} ({})", e.text, e.kind))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn match_score(element: &UiElement, query: &str) -> i32 {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return 0;
    }
    let text = element.text.to_lowercase();
    if text == q {
        return 100;
    }
    if text.starts_with(&q) {
        return 85;
    }
    if text.contains(&q) {
        return 70;
    }
    if let Some(id) = &element.automation_id {
        let id = id.to_lowercase();
        if id == q {
            return 90;
        }
        if id.contains(&q) {
            return 65;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn el(text: &str, x: i32, y: i32, w: i32, h: i32) -> UiElement {
        UiElement {
            kind: "button".into(),
            text: text.into(),
            x,
            y,
            width: w,
            height: h,
            automation_id: Some(text.to_lowercase()),
            window: "Explorer".into(),
        }
    }

    #[test]
    fn matches_is_case_insensitive() {
        let e = el("Descargas", 0, 0, 10, 10);
        assert!(e.matches("descargas"));
        assert!(e.matches("DESCARGAS"));
        assert!(!e.matches("documentos"));
    }

    #[test]
    fn snapshot_find_returns_first_match() {
        let snap = UiSnapshot {
            window: "Explorer".into(),
            elements: vec![
                el("Descargas", 0, 0, 100, 30),
                el("Documentos", 0, 40, 100, 30),
            ],
        };
        let found = snap.find("Descargas").unwrap();
        assert_eq!(found.text, "Descargas");
    }

    #[test]
    fn center_is_midpoint() {
        let e = el("x", 100, 200, 40, 20);
        assert_eq!(e.center(), (120, 210));
    }

    #[test]
    fn find_best_prefers_exact_match() {
        let snap = UiSnapshot {
            window: "Explorer".into(),
            elements: vec![
                el("Descargas", 0, 0, 100, 30),
                el("Mis Descargas", 0, 40, 100, 30),
            ],
        };
        let found = snap.find_best(&["descargas".into()]).unwrap();
        assert_eq!(found.text, "Descargas");
    }
}
