// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{error::Error, io::Write, fs::File, io::Result};
use serde::Deserialize;
use slint::{ModelRc, SharedString, VecModel};
use std::rc::Rc;

use std::io::{BufRead, BufReader};
use std::collections::HashMap;
use tokio::runtime::Runtime;
use reqwest;
use chrono::{Datelike, Local};
slint::include_modules!();

// use std::{fs::File, io::BufReader};
// use slint::{ModelRc, VecModel, SharedString};

// use std::rc::Rc;
// use std::cell::RefCell;

#[derive(Debug, Deserialize, Clone)]
struct Lokasi {
    id: u32,
    city: String,
    gmt: String,
}

#[derive(Debug, Deserialize, Clone)]
struct WaktuAdzan {
    shubuh: String,
    dzuhur: String,
    ashar: String,
    maghrib: String,
    isya: String,
}

#[derive(Debug, Deserialize)]
#[derive(Clone)]
struct LokasiWrapper {
    data: Vec<Lokasi>,
}

fn search_city(locations: &[Lokasi], query: &str) -> Vec<Lokasi> {
    locations
        .iter()
        .filter(|lokasi| lokasi.city.to_lowercase().contains(&query.to_lowercase()))
        .cloned()
        .collect()
}


fn save_settings_to_ini(path: &str, city_id: u32, city_name: &str, city_gmt: &str) -> Result<()> {
    let mut file = File::create(path)?;
    writeln!(file, "[location]")?;
    writeln!(file, "id = {}", city_id)?;
    writeln!(file, "name = {}", city_name)?;
    writeln!(file, "gmt = {}", city_gmt)?;


    writeln!(file, "\n[adzan]")?;
    writeln!(file, "shubuh = {}", city_id)?;
    writeln!(file, "dzuhur = {}", city_name)?;
    writeln!(file, "ashar = {}", city_gmt)?;
    writeln!(file, "maghrib = {}", city_gmt)?;
    writeln!(file, "isya = {}", city_gmt)?;

    Ok(())
}

fn show_message(msg: &str) {
    let dialog = Rc::new(MessageWindow::new().unwrap());
    dialog.set_message_text(SharedString::from(msg));
    dialog.show().unwrap();
}

fn read_location_settings(path: &str) -> std::io::Result<HashMap<String, String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut in_location = false;
    let mut settings = HashMap::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.starts_with('[') && line.ends_with(']') {
            in_location = &line[1..line.len()-1] == "location";
            continue;
        }

        if in_location && line.contains('=') {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            let key = parts[0].trim().to_string();
            let value = parts[1].trim().to_string();
            settings.insert(key, value);
        }
    }

    Ok(settings)
}

async fn get_waktu_adzan(day: &str, month: &str, year: &str) -> std::result::Result<String, Box<dyn Error>> {
    let url = format!("https://adzan-indonesia-api.vercel.app/adzan?cityId=67&month={}&year={}&date={}", month, year, day);
    let res = reqwest::get(url).await?;
    let text = res.text().await?;
    Ok(text)
}

fn main() -> std::result::Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;
    // get waktu adzan

    
    match read_location_settings("data/settings.ini") {
        Ok(settings) => {
            println!("ID: {}", settings.get("id").unwrap_or(&"0".to_string()));
            println!("Name: {}", settings.get("name").unwrap_or(&"Unknown".to_string()));
            println!("GMT: {}", settings.get("gmt").unwrap_or(&"+0".to_string()));
            let prayer_location = format!(
                "{}-{}",
                settings.get("name").unwrap_or(&"Unknown".to_string()),
                settings.get("gmt").unwrap_or(&"+0".to_string())
            );
            ui.set_location(SharedString::from(prayer_location));
        }
        Err(e) => {
            eprintln!("Gagal membaca settings: {}", e);
        }
    }
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
    ui.on_show_select_location({
    let ui_weak = ui.as_weak(); 
    move || {
        let dialog = SelectLocationWindow::new().unwrap();
        let dialog_weak = dialog.as_weak();

        let filename = "data/lokasi.json";
        let json_file_path = std::path::Path::new(filename);
        let file = std::fs::File::open(json_file_path).expect("File tidak ditemukan");
        let lokasi_response: LokasiWrapper = serde_json::from_reader(file).expect("error parsing JSON");

        dialog.on_search_text_changed({
            let dialog_weak = dialog_weak.clone();
            let lokasi_response = lokasi_response.clone(); // clone kalau perlu
            move |input| {
                if let Some(dialog) = dialog_weak.upgrade() {
                    let result = search_city(&lokasi_response.data, &input);
                    let result_names: Vec<SharedString> = result
                        .iter()
                        .map(|l| SharedString::from(format!("{}-{}-Gmt {}", l.id, l.city, l.gmt)))
                        .collect();
                    dialog.set_city_list(ModelRc::new(VecModel::from_iter(result_names)));
                }
            }
        });

        dialog.on_save_location({
            let ui_weak = ui_weak.clone();
            move |selected_city| {
                let parts: Vec<&str> = selected_city.split('-').collect();
                let city_id: u32 = parts[0].parse().unwrap_or(0);
                let city_name = parts[1].to_string();
                let city_gmt = parts[2].to_string();

                save_settings_to_ini("data/settings.ini", city_id, &city_name, &city_gmt)
                    .expect("Failed to save settings");

                show_message("Setting successfully saved!");

                let now = Local::now();
                let day = &now.day().to_string();
                let month = &now.month().to_string();
                let year = &now.year().to_string();
                let rt = Runtime::new().expect("Failed to create runtime");
                rt.block_on(async {
                    match get_waktu_adzan(day, month, year).await {
                        Ok(text) => println!("Response:\n{}", text),
                        Err(e) => eprintln!("Gagal: {}", e),
                    }
                });

                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_location(SharedString::from(format!("{}-{}", city_name, city_gmt)));
                }
            }
        });

        dialog.show().unwrap();
    }
});
    ui.run()?;
    Ok(())
}

