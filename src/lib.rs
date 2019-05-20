mod common;
pub use common::{ClipboardData, ClipboardFunctions, ClipboardSink};

#[cfg(target_os = "linux")]
pub mod x11_clipboard;

#[cfg(target_os = "windows")]
pub mod winapi_clipboard;

#[cfg(target_os = "linux")]
pub type Clipboard = x11_clipboard::Clipboard;

#[cfg(target_os = "windows")]
pub type Clipboard = winapi_clipboard::Window;
