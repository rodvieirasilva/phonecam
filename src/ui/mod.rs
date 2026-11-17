#[cfg(feature = "gui")]
pub mod gtk_app;

#[cfg(feature = "gui")]
pub fn run_desktop_ui(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    gtk_app::run_gtk_app(port)
}

#[cfg(not(feature = "gui"))]
pub fn run_desktop_ui(_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Notice: PhoneCam compiled without native GTK4 GUI support.");
    Ok(())
}
