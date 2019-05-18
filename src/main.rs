mod utils;

#[cfg(target_os = "linux")]
mod app;

#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dpy = app::Clipboard::new()?;
    dpy.watch_clipboard();
    Ok(())
}

#[cfg(target_os = "windows")]
mod winapi;

#[cfg(target_os = "windows")]
fn main() {
    winapi::monitor_clipboard();
}
