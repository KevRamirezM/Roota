use crate::accessibility::element::UiSnapshot;

/// Hints for which top-level window to scan (e.g. explorer, chrome, gmail).
#[derive(Debug, Clone, Default)]
pub struct ScanContext {
    pub window_hints: Vec<String>,
}

impl ScanContext {
    pub fn from_expected_window(expected: Option<&str>) -> Self {
        let mut ctx = ScanContext {
            window_hints: Vec::new(),
        };
        if let Some(w) = expected {
            ctx.window_hints.push(w.to_lowercase());
        }
        match expected.map(|s| s.to_lowercase()) {
            Some(ref w) if w.contains("explorer") => {
                ctx.window_hints.extend(
                    ["explorador", "archivos", "files", "file explorer"]
                        .iter()
                        .map(|s| s.to_string()),
                );
            }
            Some(ref w) if w.contains("chrome") => {
                ctx.window_hints.extend(
                    ["google chrome", "chrome", "edge", "navegador"]
                        .iter()
                        .map(|s| s.to_string()),
                );
            }
            Some(ref w) if w.contains("gmail") => {
                ctx.window_hints.extend(
                    ["gmail", "correo", "mail", "outlook"]
                        .iter()
                        .map(|s| s.to_string()),
                );
            }
            Some(ref w) if w.contains("word") => {
                ctx.window_hints
                    .extend(["word", "documento"].iter().map(|s| s.to_string()));
            }
            _ => {}
        }
        ctx
    }

    /// Add window-title hints inferred from the user's natural-language query.
    pub fn enrich_from_utterance(&mut self, utterance: &str) {
        let u = utterance.to_lowercase();
        let mut extra: Vec<&str> = Vec::new();

        if u.contains("cursor") {
            extra.extend(["cursor", "visual studio code", "vscode"]);
        }
        if u.contains("terminal") || u.contains("powershell") || u.contains("cmd") {
            extra.extend([
                "terminal",
                "consola",
                "powershell",
                "nueva terminal",
                "new terminal",
            ]);
        }
        if u.contains("configuración") || u.contains("configuracion") || u.contains("settings")
        {
            extra.extend(["configuración", "settings", "configuracion"]);
        }
        if u.contains("bluetooth") {
            extra.push("bluetooth");
        }
        if u.contains("wifi") || u.contains("wi-fi") {
            extra.extend(["wifi", "wi-fi", "red"]);
        }
        if u.contains("bloc de notas") || u.contains("notepad") {
            extra.extend(["notepad", "bloc de notas"]);
        }
        if u.contains("explorador") || u.contains("archivos") {
            extra.extend(["explorador", "file explorer"]);
        }

        for hint in extra {
            if !self
                .window_hints
                .iter()
                .any(|h| h.eq_ignore_ascii_case(hint))
            {
                self.window_hints.push(hint.to_string());
            }
        }
    }
}

pub trait Scanner: Send + Sync {
    fn name(&self) -> &str;

    fn snapshot(&self) -> UiSnapshot {
        self.snapshot_with_context(&ScanContext::default())
    }

    fn snapshot_with_context(&self, ctx: &ScanContext) -> UiSnapshot;
}

pub fn get_scanner() -> Box<dyn Scanner> {
    #[cfg(windows)]
    {
        match crate::accessibility::windows::WindowsScanner::new() {
            Ok(s) => return Box::new(s),
            Err(err) => {
                tracing::warn!(target: "roota.accessibility", "WindowsScanner unavailable: {err}; using stub");
            }
        }
    }
    Box::new(crate::accessibility::stub::StubScanner::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enrich_from_utterance_adds_cursor_hint() {
        let mut ctx = ScanContext::default();
        ctx.enrich_from_utterance("como abro una terminal en cursor");
        assert!(ctx.window_hints.iter().any(|h| h.contains("cursor")));
    }
}
