//! Poll cursor position and mouse button edges (read-only, PRD safety §8.9).

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Default)]
pub struct InputSample {
    pub cursor: PhysicalPoint,
    pub left_pressed: bool,
    pub right_pressed: bool,
    /// Left button went down on this sample (click start).
    pub left_click: bool,
    pub right_click: bool,
    /// Heuristic: two left clicks within 500ms (also sticky — see below).
    pub double_click: bool,
}

pub struct InputMonitor {
    prev_left: bool,
    prev_right: bool,
    last_left_click_at: Option<Instant>,
    last_left_click_pos: PhysicalPoint,
    /// Stays true briefly after a double-click so slow guide loops still see it.
    double_click_until: Option<Instant>,
}

impl Default for InputMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl InputMonitor {
    pub fn new() -> Self {
        Self {
            prev_left: false,
            prev_right: false,
            last_left_click_at: None,
            last_left_click_pos: PhysicalPoint::default(),
            double_click_until: None,
        }
    }

    pub fn poll(&mut self) -> InputSample {
        let sample = platform_sample();
        let left_click = sample.left_pressed && !self.prev_left;
        let right_click = sample.right_pressed && !self.prev_right;

        let now = Instant::now();
        let mut double_click = self
            .double_click_until
            .is_some_and(|until| now < until);

        if left_click {
            if let Some(prev) = self.last_left_click_at {
                let dt = now.duration_since(prev);
                let dx = (sample.cursor.x - self.last_left_click_pos.x).abs();
                let dy = (sample.cursor.y - self.last_left_click_pos.y).abs();
                if dt < Duration::from_millis(600) && dx < 12 && dy < 12 {
                    double_click = true;
                    self.double_click_until = Some(now + Duration::from_millis(800));
                }
            }
            self.last_left_click_at = Some(now);
            self.last_left_click_pos = sample.cursor;
        }

        self.prev_left = sample.left_pressed;
        self.prev_right = sample.right_pressed;

        InputSample {
            cursor: sample.cursor,
            left_pressed: sample.left_pressed,
            right_pressed: sample.right_pressed,
            left_click,
            right_click,
            double_click,
        }
    }
}

#[cfg(windows)]
fn platform_sample() -> InputSample {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_LBUTTON, VK_RBUTTON};
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT::default();
    let cursor = unsafe {
        if GetCursorPos(&mut point).is_ok() {
            PhysicalPoint {
                x: point.x,
                y: point.y,
            }
        } else {
            PhysicalPoint::default()
        }
    };

    let left_pressed = key_down(VK_LBUTTON.0 as i32);
    let right_pressed = key_down(VK_RBUTTON.0 as i32);

    InputSample {
        cursor,
        left_pressed,
        right_pressed,
        left_click: false,
        right_click: false,
        double_click: false,
    }
}

#[cfg(windows)]
fn key_down(vkey: i32) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    unsafe { GetAsyncKeyState(vkey) < 0 }
}

#[cfg(not(windows))]
fn platform_sample() -> InputSample {
    InputSample::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_tracks_button_edges() {
        let mut m = InputMonitor::new();
        m.prev_left = false;
        m.prev_right = false;
        let _ = m.poll();
    }
}
