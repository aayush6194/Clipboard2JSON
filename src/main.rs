use clipboard_rs::{Clipboard, ClipboardFunctions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dpy = Clipboard::new()?;
    dpy.watch_clipboard();
    Ok(())
}