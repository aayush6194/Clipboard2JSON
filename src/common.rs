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

/// Stores a function that takes the clipboard data and writes it to a source. 
/// It is stored in a struct because it is easier to implement Clone this way which
/// plays nicely with the static variables in the WinAPI implementation of the
/// clipboard. 
#[derive(Clone)]
pub struct ClipboardSink(pub fn(ClipboardData) -> Result<(), Error>);
