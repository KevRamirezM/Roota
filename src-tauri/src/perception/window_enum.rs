//! `EnumWindows`-based listing of top-level windows + Roota/visibility filters.
//!
//! Returns plain metadata; UIA element counts are populated later by the
//! UIA perceiver. Non-windows targets get a deterministic stub fixture so
//! the rest of the perception pipeline compiles and tests on CI.

use crate::perception::frame::{Rect, WindowId};

/// Window markers Roota uses to skip its own webviews from perception.
pub const ROOTA_TITLE_MARKERS: &[&str] = &["roota"];

#[derive(Debug, Clone)]
pub struct WindowMeta {
    pub id: WindowId,
    pub title: String,
    pub class_name: String,
    pub bounds: Rect,
    pub is_foreground: bool,
    pub is_visible: bool,
    pub is_minimized: bool,
    pub is_roota: bool,
    /// Owner HWND (only set for dialogs that have an owner).
    pub owner_id: Option<WindowId>,
    /// 0..u32::MAX; smaller = topmost. Populated by EnumWindows callback order.
    pub z_order: u32,
}

impl WindowMeta {
    pub fn area(&self) -> i64 {
        self.bounds.area()
    }

    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        self.bounds.contains(x, y)
    }
}

#[cfg(windows)]
pub fn list_visible_windows() -> Vec<WindowMeta> {
    windows_impl::list_visible_windows()
}

#[cfg(not(windows))]
pub fn list_visible_windows() -> Vec<WindowMeta> {
    sample_windows_fixture()
}

/// Deterministic fixture (also used by tests on Windows so we never depend on
/// the live desktop).
pub fn sample_windows_fixture() -> Vec<WindowMeta> {
    vec![
        WindowMeta {
            id: WindowId(1),
            title: "Inicio - Explorador de archivos".into(),
            class_name: "CabinetWClass".into(),
            bounds: Rect::new(0, 0, 1280, 720),
            is_foreground: false,
            is_visible: true,
            is_minimized: false,
            is_roota: false,
            owner_id: None,
            z_order: 1,
        },
        WindowMeta {
            id: WindowId(2),
            title: "Google Chrome".into(),
            class_name: "Chrome_WidgetWin_1".into(),
            bounds: Rect::new(1280, 0, 1280, 720),
            is_foreground: true,
            is_visible: true,
            is_minimized: false,
            is_roota: false,
            owner_id: None,
            z_order: 0,
        },
        WindowMeta {
            id: WindowId(99),
            title: "Roota".into(),
            class_name: "Tao".into(),
            bounds: Rect::new(2000, 600, 360, 480),
            is_foreground: false,
            is_visible: true,
            is_minimized: false,
            is_roota: true,
            owner_id: None,
            z_order: 2,
        },
    ]
}

pub fn is_roota_title(title: &str) -> bool {
    let lower = title.to_lowercase();
    ROOTA_TITLE_MARKERS.iter().any(|m| lower.contains(m))
}

#[cfg(windows)]
mod windows_impl {
    use std::cell::RefCell;

    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetForegroundWindow, GetWindow, GetWindowRect,
        GetWindowTextW, IsIconic, IsWindowVisible, GW_OWNER,
    };

    use super::{is_roota_title, WindowMeta};
    use crate::perception::frame::{Rect, WindowId};

    thread_local! {
        static SINK: RefCell<Vec<WindowMeta>> = const { RefCell::new(Vec::new()) };
    }

    pub fn list_visible_windows() -> Vec<WindowMeta> {
        SINK.with(|s| s.borrow_mut().clear());
        let foreground = unsafe { GetForegroundWindow() };

        unsafe extern "system" fn enum_proc(hwnd: HWND, _lparam: LPARAM) -> BOOL {
            if let Some(meta) = collect_window(hwnd) {
                SINK.with(|s| s.borrow_mut().push(meta));
            }
            BOOL(1)
        }

        unsafe {
            let _ = EnumWindows(Some(enum_proc), LPARAM(0));
        }

        let mut out: Vec<WindowMeta> = SINK.with(|s| s.borrow_mut().drain(..).collect());

        for (idx, w) in out.iter_mut().enumerate() {
            w.z_order = idx as u32;
            if !foreground.0.is_null() {
                w.is_foreground = w.id.0 == foreground.0 as u64;
            }
        }

        out.retain(|w| !w.is_minimized && w.bounds.area() > 0);
        out
    }

    unsafe fn collect_window(hwnd: HWND) -> Option<WindowMeta> {
        if hwnd.0.is_null() {
            return None;
        }
        if !IsWindowVisible(hwnd).as_bool() {
            return None;
        }
        let minimized = IsIconic(hwnd).as_bool();

        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return None;
        }
        if rect.right - rect.left <= 1 || rect.bottom - rect.top <= 1 {
            return None;
        }

        let title = read_text(|buf| GetWindowTextW(hwnd, buf));
        let class_name = read_text(|buf| GetClassNameW(hwnd, buf));
        if title.trim().is_empty() && class_name.trim().is_empty() {
            return None;
        }

        let owner_hwnd = GetWindow(hwnd, GW_OWNER).unwrap_or_default();
        let owner_id = if owner_hwnd.0.is_null() {
            None
        } else {
            Some(WindowId(owner_hwnd.0 as u64))
        };

        let is_roota = is_roota_title(&title);

        Some(WindowMeta {
            id: WindowId(hwnd.0 as u64),
            title,
            class_name,
            bounds: Rect::from_ltrb(rect.left, rect.top, rect.right, rect.bottom),
            is_foreground: false,
            is_visible: true,
            is_minimized: minimized,
            is_roota,
            owner_id,
            z_order: 0,
        })
    }

    fn read_text<F>(mut call: F) -> String
    where
        F: FnMut(&mut [u16]) -> i32,
    {
        let mut buf = [0u16; 256];
        let len = call(&mut buf);
        if len <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_windows_filters_roota() {
        let wins = sample_windows_fixture();
        let filtered: Vec<_> = wins.into_iter().filter(|w| !w.is_roota).collect();
        assert!(
            filtered
                .iter()
                .all(|w| !w.title.to_lowercase().contains("roota"))
        );
        assert!(filtered.iter().any(|w| w.title.contains("Explorador")));
    }

    #[test]
    fn fixture_marks_roota_window() {
        let wins = sample_windows_fixture();
        assert!(wins.iter().any(|w| w.is_roota));
    }

    #[test]
    fn is_roota_title_is_case_insensitive() {
        assert!(is_roota_title("ROOTA — overlay"));
        assert!(!is_roota_title("Notepad"));
    }
}
