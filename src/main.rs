// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::error::Error;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;
    
    // main window
    ui.on_request_increase_value({
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_counter(ui.get_counter() + 1);
        }
    });

    // about window
    ui.on_show_about(|| {
        let dialog = AboutWindow::new().unwrap();
        dialog.show().unwrap();
    });

    // credit window
    ui.on_show_credit(|| {
        let dialog = CreditWindow::new().unwrap();
        dialog.show().unwrap();
    });

    // select location window
    ui.on_show_select_location(|| {
        let window  = SelectLocationWindow::new().unwrap();
        window.show().unwrap();
    });

    ui.run()?;

    Ok(())
}
