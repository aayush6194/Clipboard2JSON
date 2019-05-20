mod utils;

use clipboard2json::{Clipboard, ClipboardFunctions, ClipboardSink};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dpy = Clipboard::new()?;
    dpy.watch_clipboard(&ClipboardSink(utils::save_clipboard_to_file));
    Ok(())
}
