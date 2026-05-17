//! Open File Explorer so the accessibility scanner has a window to read (guide-only).

#[cfg(windows)]
pub fn launch_file_explorer() {
    match std::process::Command::new("explorer").spawn() {
        Ok(_) => tracing::info!(target: "roota.shell.explorer", "launched explorer.exe"),
        Err(err) => {
            tracing::warn!(target: "roota.shell.explorer", "could not launch explorer: {err}")
        }
    }
}

#[cfg(not(windows))]
pub fn launch_file_explorer() {}
