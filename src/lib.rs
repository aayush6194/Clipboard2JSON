mod common;
pub use common::{ClipboardData, ClipboardFunctions, ClipboardSink};

#[cfg(target_os = "linux")]
#[path = ""]
pub mod clipboard {
    pub mod x11_clipboard;
    pub type Clipboard = x11_clipboard::ClipboardOwner;
}

#[cfg(windows)]
#[path = ""]
pub mod clipboard {
    pub mod winapi_clipboard;
    pub type Clipboard = winapi_clipboard::ClipboardOwner;
}

pub use crate::clipboard::Clipboard;
