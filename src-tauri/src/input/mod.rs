//! Read-only observation of cursor and mouse buttons (never injects input).

mod monitor;

pub use monitor::{InputMonitor, InputSample, PhysicalPoint};
