//! Desktop shell: floating assistant panel visibility.

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod explorer;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub mod panel;
