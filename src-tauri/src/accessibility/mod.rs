//! Read-only Windows UI Automation snapshots.

pub mod element;
pub mod scanner;
pub mod stub;
#[cfg(windows)]
pub mod windows;

pub use element::{UiElement, UiSnapshot};
pub use scanner::{get_scanner, Scanner};
pub use stub::StubScanner;
