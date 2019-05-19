use crate::ClipboardData;
use std::error::Error;

/// Defines common traits for the clipboard so that it's easier to abstract over
/// the underlying libraries. 
// @TODO: Add more functions when workng on WINAPI
pub trait ClipboardFunctions: Sized {
    /// Creates a new `Clipboard` with a pointer to the hidden window
    // @TODO: Better error handling?
    fn new() -> Result<Self, &'static str>;
    /// Watches over the clipboard and passes the changed data to the callback
    fn watch_clipboard(&self, callback: &ClipboardSink);
}

/// Takes the clipboard data and writes it to a source
pub type ClipboardSink = Fn(ClipboardData) -> Result<(), Box<dyn Error>>;
