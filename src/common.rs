use crate::ClipboardData;
use std::error::Error;

pub trait ClipboardFunctions: Sized {
    fn new() -> Result<Self, &'static str>;
    fn watch_clipboard(&self, callback: &ClipboardSink);
}

pub type ClipboardSink = Fn(ClipboardData) -> Result<(), Box<dyn Error>>;
