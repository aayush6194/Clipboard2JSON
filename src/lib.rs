//! # Clipboard2JSON
//! Clipboard2JSON is a tool that watches the window's clipboard and writes the 
//! contents to a JSON file when the clipboard selection changes. It abstracts 
//! over the WinAPI and the X11 library to provide a common interface for working
//! with the clipboard.
mod common;
pub use common::ClipboardFunctions;

#[cfg(target_os = "linux")]
pub mod x11_clipboard;

#[cfg(target_os = "windows")]
pub mod winapi_clipboard;

#[cfg(target_os = "linux")]
pub type Clipboard = x11_clipboard::Clipboard;
pub type ClipboardData = x11_clipboard::ClipboardData;
