mod app;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dpy = app::App::new()?;
    dpy.get_targets();

    Ok(())
}
