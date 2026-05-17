//! Roota Tauri library entry point. `main.rs` is a thin passthrough so
//! mobile builds (if we ever add them) keep working.

pub mod accessibility;
pub mod commands;
pub mod i18n;
pub mod llm;
pub mod orchestration;
pub mod overlay;
pub mod prompts;
pub mod safety;
pub mod settings;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod shell;

use std::sync::Arc;

use tauri::Manager;
use tokio::sync::Mutex;
use tracing_subscriber::EnvFilter;

use crate::accessibility::scanner::{get_scanner, Scanner};
use crate::orchestration::{default_registry, Orchestrator, TemplateRegistry};
use crate::settings::Settings;

pub struct AppState {
    pub orchestrator: Arc<Orchestrator>,
    pub running_session: Mutex<Option<tokio::task::JoinHandle<()>>>,
    pub settings: Settings,
}

fn init_tracing(settings: &Settings) {
    let level = settings.log_level.to_lowercase();
    let filter = EnvFilter::try_new(&level)
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}

fn load_templates() -> TemplateRegistry {
    let mut registry = default_registry();
    let mut tries: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        tries.push(cwd.join("src-tauri").join("templates"));
        tries.push(cwd.join("templates"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            tries.push(dir.join("templates"));
        }
    }
    for path in tries {
        if path.exists() {
            registry.merge_json_dir(&path);
            break;
        }
    }
    registry
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let settings = Settings::from_env();
    init_tracing(&settings);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    let llm = runtime.block_on(crate::llm::build_llm(&settings));
    let scanner: Arc<dyn Scanner> = Arc::from(get_scanner());
    let templates = Arc::new(load_templates());
    let orchestrator = Arc::new(Orchestrator::new(
        llm,
        scanner.clone(),
        templates,
        settings.ui_language,
        settings.llm_intent_timeout_seconds,
    ));

    tracing::info!(
        target: "roota",
        "Boot: llm={} scanner={} lang={:?}",
        orchestrator.llm_name(),
        orchestrator.scanner_name(),
        settings.ui_language,
    );

    let app_state = AppState {
        orchestrator,
        running_session: Mutex::new(None),
        settings: settings.clone(),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_runtime(runtime))
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::start_session,
            commands::confirm_response,
            commands::cancel_session,
            commands::show_overlay_anchor,
            commands::clear_overlay,
            commands::toggle_panel,
            commands::panel_visible,
        ])
        .setup(|app| {
            if let Some(overlay) = app.get_webview_window("overlay") {
                let _ = overlay.set_ignore_cursor_events(true);
                use tauri::window::Color;
                let _ = overlay.set_background_color(Some(Color(0, 0, 0, 0)));
            }

            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

                if let Some(panel) = app.get_webview_window(shell::panel::PANEL_LABEL) {
                    let _ = shell::panel::position_bottom_right(&panel);
                    let _ = panel.set_always_on_top(true);
                    let _ = shell::panel::hide(app.handle());
                }

                if let Err(err) = app.global_shortcut().on_shortcut(
                    shell::panel::TOGGLE_SHORTCUT,
                    move |app, _shortcut, event| {
                        if event.state == ShortcutState::Pressed {
                            if let Err(e) = shell::panel::toggle(app) {
                                tracing::warn!(target: "roota.shell.panel", "toggle failed: {e}");
                            }
                        }
                    },
                ) {
                    tracing::warn!(
                        target: "roota.shell.panel",
                        "could not register {}: {err}",
                        shell::panel::TOGGLE_SHORTCUT
                    );
                } else {
                    tracing::info!(
                        target: "roota.shell.panel",
                        "registered shortcut {}",
                        shell::panel::TOGGLE_SHORTCUT
                    );
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running roota app");
}

fn tauri_plugin_runtime(
    runtime: tokio::runtime::Runtime,
) -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("roota-runtime")
        .setup(move |app, _api| {
            app.manage(RuntimeHandle(runtime.handle().clone()));
            // Keep the runtime alive for the lifetime of the app.
            std::mem::forget(runtime);
            Ok(())
        })
        .build()
}

pub struct RuntimeHandle(pub tokio::runtime::Handle);
