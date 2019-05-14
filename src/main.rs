#[cfg(target_os = "linux")]
mod app;

#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dpy = app::App::new()?;
    dpy.get_targets();

    Ok(())
}

#[cfg(target_os = "windows")]
mod winapi;

fn main() {
    winapi::monitor_clipboard();
}
