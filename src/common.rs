use crate::ClipboardData;
use failure::Error;

/// Defines common traits for the clipboard so that it's easier to abstract over
/// the underlying libraries. 
// @TODO: Add more functions when workng on WINAPI
pub trait ClipboardFunctions: Sized {
    /// Creates a new `Clipboard` with a pointer to the hidden window
    // @TODO: Better error handling?
    fn new() -> Result<Self, Error>;
    /// Fetches the data stored in the clipboard as a text-based format
    fn get_clipboard(&self) -> Result<ClipboardData, Error>; 
    /// Watches over the clipboard and passes the changed data to the callback
    fn watch_clipboard(&self, callback: &ClipboardSink);
}

/// Takes the clipboard data and writes it to a source
pub type ClipboardSink = Fn(ClipboardData) -> Result<(), Error>;
