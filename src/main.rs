use clipboard_rs::{utils, Clipboard, ClipboardFunctions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dpy = Clipboard::new()?;
    dpy.watch_clipboard(&utils::save_clipboard_to_file);
    Ok(())
}
