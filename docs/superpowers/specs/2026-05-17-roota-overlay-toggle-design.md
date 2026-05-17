# Roota — Toggleable Overlay Assistant (Design Spec)

**Date:** 2026-05-17  
**Status:** Approved — option A (hidden at startup, `Ctrl+Shift+Space`)  
**Goal:** Let users show or hide Roota's control UI with a global keyboard shortcut so the accessibility scanner sees the real foreground app (Explorer, Chrome, etc.), not Roota.

---

## Problem

Roota currently runs as a **large centered window** (`main`, 960×720). While it is open:

- Windows UI Automation often targets **Roota's webview** (focused input, window title "Roota").
- We work around this with `minimize on SÍ` and `is_roota_*` filters, but users still fight focus and screen space.
- The PRD wireframe describes a **floating assistant**, not a full desktop app.

The **anchor overlay** (`overlay` window) already works as a separate fullscreen transparent layer for yellow pulses. This spec only changes the **control panel** (chat, confirm, guidance text).

---

## Success criteria

1. User can **hide** Roota's control UI completely so it does not block or pollute scans.
2. User can **show** it again with a **system-wide shortcut** without using the taskbar.
3. When hidden during guidance, **anchor overlay still works**; scanner prefers non-Roota windows.
4. Senior-friendly: shortcut uses **modifier keys** (not a single character), documented in the panel.
5. Default behavior documented and testable on Windows 10/11.

---

## Recommended approach: Compact floating panel + global shortcut

Transform `main` into a **frameless, always-on-top assistant panel** (PRD §7 “Floating Assistant UI”):

| Property | Value |
|----------|--------|
| Position | Bottom-right of primary monitor |
| Size | ~440×520 logical px (resizable optional later) |
| Decorations | Off (frameless) |
| Always on top | Yes (below anchor overlay z-order if needed) |
| Default visibility | **Hidden** at launch (tray / shortcut to open) |
| Toggle shortcut | **`Ctrl+Shift+Space`** (default; configurable later) |

**Two windows remain:**

```
assistant (main)  — togglable HUD: input, guidance, confirmation
overlay           — fullscreen click-through anchors (unchanged)
```

### Toggle behavior

| Action | Effect |
|--------|--------|
| Shortcut pressed, panel hidden | Show panel, focus text input |
| Shortcut pressed, panel visible | Hide panel (no taskbar focus steal) |
| User clicks SÍ on confirmation | Hide panel (replaces current `minimize`) so Explorer is foreground |
| User cancels / completes session | Panel may stay visible or auto-hide (config: stay visible) |

### Scanner integration

- Keep existing `is_roota_title` / skip Roota HWND logic in `windows.rs`.
- When panel is **hidden**, it must not be foreground → scans succeed without minimize hack.
- Emit `PanelVisibilityChanged` event to frontend (optional) for status chip.

### System tray (optional phase 1.5)

- Tray icon: "Show Roota" / "Hide Roota" for users who forget the shortcut.
- Defer if timeboxed; shortcut alone satisfies MVP.

---

## Alternatives considered

### A. Minimize-only (current partial fix)

- **Pros:** Minimal code.
- **Cons:** Taskbar clutter; user must restore window; does not match PRD floating UI.

### B. Full merge into one fullscreen overlay

- Single window for both HUD and anchors.
- **Cons:** Complex hit-testing; harder WCAG; breaks current clean split.

### C. Separate process / companion

- **Cons:** Hackathon scope; unnecessary.

**Recommendation:** Compact panel + `tauri-plugin-global-shortcut` (Approach in table above).

---

## Architecture

### Rust (`src-tauri`)

| Module | Responsibility |
|--------|----------------|
| `shell/panel.rs` | `show_panel`, `hide_panel`, `toggle_panel`, `is_panel_visible` |
| `lib.rs` | Register `tauri-plugin-global-shortcut`, wire default shortcut in `setup` |
| `commands.rs` | Expose toggle to frontend; replace `minimize()` with `hide_panel()` on confirm |
| `settings.rs` | `panel_shortcut` string (future), `start_hidden: bool` |

### Tauri config

- Update `main` window: `decorations: false`, `alwaysOnTop: true`, `visible: false`, smaller size, `skipTaskbar: true` when hidden (platform-specific API).

### Capabilities

- Add `global-shortcut:default` (+ register/unregister if needed).
- `core:window:allow-minimize` → keep or replace with hide/show only.

### Frontend

- `usePanelToggle` hook listening for `roota://panel-visible` events.
- Header hint: "Pulsa Ctrl+Shift+Espacio para ocultar o mostrar".
- CSS: compact floating card aesthetic (already close in `theme.css`).

---

## Data flow

```
User presses Ctrl+Shift+Space
        │
        ▼
global-shortcut plugin (Rust)
        │
        ▼
PanelController::toggle()
        │
        ├─ hide: main.hide(), skipTaskbar, release focus
        └─ show: main.show(), set_focus, unminimize
        │
        ▼
(optional) emit roota://panel-visible { visible: bool }
```

During guidance:

```
confirm SÍ → hide_panel() → orchestrator continues
          → overlay shows anchors on Explorer
          → scanner sees Explorer (not Roota)
```

---

## Error handling

- Shortcut registration fails (conflict with OS): log warning, show in-panel message, tray fallback later.
- Toggle while modal open: allow hide (modal is in same webview — closing panel hides modal; acceptable for MVP).
- Active session + hide: **do not cancel** session; overlay remains until goal/error.

---

## Testing

| Test | Type |
|------|------|
| `PanelController` state after toggle | Rust unit test with mock labels |
| `is_roota_title` still excludes assistant | Existing detector tests |
| Manual: hidden → Explorer scan finds Descargas | Smoke |
| Manual: shortcut toggles while app in background | Smoke |

---

## Out of scope (this iteration)

- User-editable shortcut UI in settings screen
- macOS / Linux builds
- Voice hotkey
- Auto-hide timer

---

## Open decisions (user input)

1. **Default at startup:** hidden vs visible? *(Plan assumes hidden.)*
2. **Shortcut:** `Ctrl+Shift+Space` vs other? *(Plan assumes default above.)*
3. **System tray icon in v1?** *(Plan: optional follow-up.)*

---

## Approval

Once defaults or answers are confirmed, implementation follows  
`docs/superpowers/plans/2026-05-17-roota-overlay-toggle.md`.
