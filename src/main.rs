mod utils;

#[cfg(target_os = "linux")]
mod x11_clipboard;

#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dpy = x11_clipboard::Clipboard::new()?;
    dpy.watch_clipboard();
    Ok(())
}

#[cfg(target_os = "windows")]
mod winapi_clipboard;

#[cfg(target_os = "windows")]
fn main() {
    winapi_clipboard::monitor_clipboard();
}
