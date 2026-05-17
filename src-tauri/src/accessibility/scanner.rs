use crate::accessibility::element::UiSnapshot;

/// Hints for which top-level window to scan (e.g. explorer, chrome, gmail).
#[derive(Debug, Clone, Default)]
pub struct ScanContext {
    pub window_hints: Vec<String>,
}

impl ScanContext {
    pub fn from_expected_window(expected: Option<&str>) -> Self {
        let mut hints = Vec::new();
        if let Some(w) = expected {
            hints.push(w.to_lowercase());
        }
        match expected.map(|s| s.to_lowercase()) {
            Some(ref w) if w.contains("explorer") => {
                hints.extend(
                    ["explorador", "archivos", "files", "file explorer"]
                        .iter()
                        .map(|s| s.to_string()),
                );
            }
            Some(ref w) if w.contains("chrome") => {
                hints.extend(["google chrome", "chrome", "edge", "navegador"].iter().map(|s| s.to_string()));
            }
            Some(ref w) if w.contains("gmail") => {
                hints.extend(["gmail", "correo", "mail", "outlook"].iter().map(|s| s.to_string()));
            }
            Some(ref w) if w.contains("word") => {
                hints.extend(["word", "documento"].iter().map(|s| s.to_string()));
            }
            _ => {}
        }
        ScanContext { window_hints: hints }
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
