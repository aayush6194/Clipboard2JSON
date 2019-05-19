mod common;
pub use common::ClipboardFunctions;

#[cfg(target_os = "linux")]
pub mod x11_clipboard;

#[cfg(target_os = "windows")]
pub mod winapi_clipboard;

#[cfg(target_os = "linux")]
pub type Clipboard = x11_clipboard::Clipboard;
pub type ClipboardData = x11_clipboard::ClipboardData;
