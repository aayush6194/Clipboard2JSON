mod utils;

use clipboard2json::{Clipboard, ClipboardFunctions, ClipboardSink};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Welcome to Clipboard2JSON");
    let dpy = Clipboard::new()?;
    println!("Try copying some text and it should show up in a clipboard.json file in your folder");
    dpy.watch_clipboard(&ClipboardSink(utils::save_clipboard_to_file));
    Ok(())
}
