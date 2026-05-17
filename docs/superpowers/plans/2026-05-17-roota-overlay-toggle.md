# Toggleable Overlay Assistant — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans or subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Roota’s control UI becomes a hideable floating panel toggled with a global keyboard shortcut, so UI Automation scans the user’s real apps when the panel is hidden.

**Architecture:** Keep `main` as the assistant panel (reconfigured frameless / always-on-top); keep `overlay` for anchors. Add `tauri-plugin-global-shortcut` and a small Rust `PanelController` for show/hide/toggle. Replace `minimize()` on confirm with `hide_panel()`.

**Tech Stack:** Tauri 2, `tauri-plugin-global-shortcut`, Rust, React, existing scanner/detector.

**Spec:** `docs/superpowers/specs/2026-05-17-roota-overlay-toggle-design.md`

---

## File map

| File | Action |
|------|--------|
| `src-tauri/Cargo.toml` | Add `tauri-plugin-global-shortcut` |
| `package.json` | Add `@tauri-apps/plugin-global-shortcut` |
| `src-tauri/capabilities/default.json` | Global shortcut + window permissions |
| `src-tauri/tauri.conf.json` | Panel window props |
| `src-tauri/src/shell/mod.rs` | New — panel controller |
| `src-tauri/src/shell/panel.rs` | New — show/hide/toggle |
| `src-tauri/src/lib.rs` | Plugin + shortcut registration |
| `src-tauri/src/commands.rs` | `toggle_panel`, use hide on confirm |
| `src-tauri/src/settings.rs` | `start_hidden`, shortcut string (optional) |
| `src/i18n.ts` + `src-tauri/src/i18n.rs` | Shortcut hint strings |
| `src/components/MainScreen.tsx` | Shortcut hint UI |
| `src/tauri-api.ts` | Optional invoke helpers |

---

### Task 1: Add global-shortcut plugin

**Files:**
- Modify: `src-tauri/Cargo.toml`, `package.json`, `src-tauri/capabilities/default.json`

- [ ] **Step 1:** Run `cargo tauri add global-shortcut` from repo root (or add deps manually per tauri-v2 skill).

- [ ] **Step 2:** Add to `capabilities/default.json`:
  ```json
  "global-shortcut:default",
  "global-shortcut:allow-register",
  "global-shortcut:allow-unregister"
  ```

- [ ] **Step 3:** Register plugin in `lib.rs`:
  ```rust
  .plugin(tauri_plugin_global_shortcut::Builder::new().build())
  ```

- [ ] **Step 4:** Build once: `cargo check --manifest-path src-tauri/Cargo.toml`

---

### Task 2: Panel controller (Rust)

**Files:**
- Create: `src-tauri/src/shell/mod.rs`, `src-tauri/src/shell/panel.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1:** Write failing test for toggle state tracking (in-memory flag if window APIs unavailable in CI).

- [ ] **Step 2:** Implement `PanelController`:
  - `show(app)` — `get_webview_window("main")`, `.show()`, `.set_focus()`, position bottom-right
  - `hide(app)` — `.hide()`, do not cancel orchestrator
  - `toggle(app)` — branch on visibility
  - `is_visible(app) -> bool`

- [ ] **Step 3:** Export `pub mod shell` from `lib.rs`.

- [ ] **Step 4:** Run `cargo test --manifest-path src-tauri/Cargo.toml shell::`

---

### Task 3: Register default shortcut

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1:** In `.setup()`, after windows exist:
  ```rust
  use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
  let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space);
  app.global_shortcut().on_shortcut(shortcut, |app, _shortcut, event| {
      if event.state == ShortcutState::Pressed {
          shell::panel::toggle(app);
      }
  })?;
  ```

- [ ] **Step 2:** If `settings.start_hidden`, call `hide` on main at end of setup.

- [ ] **Step 3:** Log registration success/failure at `info`/`warn`.

---

### Task 4: Reconfigure main window as floating panel

**Files:**
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1:** Update `main` window:
  - `decorations: false`
  - `alwaysOnTop: true`
  - `visible: false` (if start hidden)
  - `width: 440`, `height: 520`
  - `resizable: true` (optional)
  - Remove center or set position via Rust on show

- [ ] **Step 2:** In `PanelController::show`, use `PhysicalPosition` / monitor work area for bottom-right placement (Windows).

- [ ] **Step 3:** Manual smoke: app starts hidden, shortcut shows panel.

---

### Task 5: Wire commands and confirm flow

**Files:**
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs` handler list

- [ ] **Step 1:** Add commands:
  ```rust
  #[tauri::command]
  fn toggle_panel(app: AppHandle) -> Result<bool, AppError>
  #[tauri::command]
  fn panel_visible(app: AppHandle) -> Result<bool, AppError>
  ```

- [ ] **Step 2:** Replace `main.minimize()` in `confirm_response` with `shell::panel::hide(&app)`.

- [ ] **Step 3:** Emit `roota://panel-visible` on toggle for frontend (optional).

- [ ] **Step 4:** Run existing `cargo test --lib` (27+ tests) — all pass.

---

### Task 6: Frontend hints and compact chrome

**Files:**
- Modify: `src/components/MainScreen.tsx`, `src/i18n.ts`, `src-tauri/src/i18n.rs`, `src/theme.css`

- [ ] **Step 1:** Add i18n keys:
  - `panel.shortcut_hint` — "Pulsa Ctrl+Mayús+Espacio para ocultar o mostrar Roota"
  - `panel.hidden_scan_hint` — "Con Roota oculta, veo mejor tu pantalla"

- [ ] **Step 2:** Show hint in header/footer of `MainScreen`.

- [ ] **Step 3:** Tune `.app-shell` for smaller panel (less padding if needed).

- [ ] **Step 4:** `npm run build` — no TS errors.

---

### Task 7: Verification (required before done)

- [ ] **Step 1:** `cargo test --manifest-path src-tauri/Cargo.toml --lib`
- [ ] **Step 2:** `npm run build`
- [ ] **Step 3:** Manual script:
  1. Start `npm run tauri:dev`
  2. Press `Ctrl+Shift+Space` → panel appears
  3. Press again → panel hides
  4. Open Explorer with Descargas visible
  5. Show panel, run "Abre la carpeta Descargas", SÍ → panel hides, yellow anchor on Descargas
  6. Complete step → success UI when panel shown again

---

## Rollback

If global-shortcut conflicts on a machine, document fallback: system tray only (Task 1.5) or in-app "Ocultar" button calling `hide_panel`.

---

## Commit suggestion (when user asks)

```
feat(shell): toggleable floating panel with global shortcut

Hide assistant UI so UIA scans foreground apps; Ctrl+Shift+Space to show/hide.
```
