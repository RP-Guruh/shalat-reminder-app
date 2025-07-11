// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::error::Error;
use serde::Deserialize;
use slint::{ModelRc, SharedString, VecModel};
// use std::{fs::File, io::BufReader};
// use slint::{ModelRc, VecModel, SharedString};

// use std::rc::Rc;
// use std::cell::RefCell;

#[derive(Debug, Deserialize, Clone)]
#[warn(dead_code)]
struct Lokasi {
    id: u32,
    city: String,
    gmt: String,
}

#[derive(Debug, Deserialize)]
struct LokasiWrapper {
    data: Vec<Lokasi>,
}


slint::include_modules!();

fn search_city(locations: &[Lokasi], query: &str) -> Vec<Lokasi> {
    locations
        .iter()
        .filter(|lokasi| lokasi.city.to_lowercase().contains(&query.to_lowercase()))
        .cloned()
        .collect()
}


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
        let dialog = SelectLocationWindow::new().unwrap();
        let dialog_weak = dialog.as_weak();
    
        let filename = "data/lokasi.json";
        let json_file_path = std::path::Path::new(filename);
        let file = std::fs::File::open(json_file_path).expect("File tidak ditemukan");
        let lokasi_response: LokasiWrapper = serde_json::from_reader(file).expect("error parsing JSON");
    
        dialog.on_search_text_changed(move |input| {
            if let Some(dialog) = dialog_weak.upgrade() {
                let result = search_city(&lokasi_response.data, &input);
                let result_names: Vec<SharedString> = result
                    .iter()
                    .map(|l| SharedString::from(format!("{} - Gmt {}", l.city, l.gmt)))
                    .collect();
    
                println!("{:?}", result_names);
                dialog.set_city_list(ModelRc::new(VecModel::from_iter(result_names)));
            }
        });
    
        dialog.show().unwrap();
    });
    

    ui.run()?;

    Ok(())
}
