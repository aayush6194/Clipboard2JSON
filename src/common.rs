use std::error::Error;

pub trait ClipboardFunctions: Sized {
    fn new() -> Result<Self, &'static str>;
    fn watch_clipboard(&self);
}