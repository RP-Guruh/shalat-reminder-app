// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{error::Error, io::Write, fs::File, io::Result, time::Duration, thread};
use serde::Deserialize;

use slint::{ModelRc, SharedString, VecModel, Timer};
use std::rc::Rc;
use std::io::{BufRead, BufReader};
use std::collections::HashMap;
use tokio::runtime::Runtime;
use reqwest;
use chrono::{Datelike, Local, Timelike, NaiveTime};
use misykat::hijri::HijriDate;
use misykat::jiff;
use rodio::{OutputStreamBuilder};


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

#[derive(Debug)]
struct JadwalShalat {
    nama: &'static str,
    waktu: String,
}   

fn search_city(locations: &[Lokasi], query: &str) -> Vec<Lokasi> {
    locations
        .iter()
        .filter(|lokasi| lokasi.city.to_lowercase().contains(&query.to_lowercase()))
        .cloned()
        .collect()
}

fn save_settings_to_ini(path: &str, city_id: u32, city_name: &str, city_gmt: &str, jadwal_adzan: &WaktuAdzan) -> Result<()> {
    let mut file = File::create(path)?;
    writeln!(file, "[location]")?;
    writeln!(file, "id = {}", city_id)?;
    writeln!(file, "name = {}", city_name)?;
    writeln!(file, "gmt = {}", city_gmt)?;


    writeln!(file, "\n[adzan]")?;
    writeln!(file, "shubuh = {}", jadwal_adzan.shubuh)?;
    writeln!(file, "dzuhur = {}", jadwal_adzan.dzuhur)?;
    writeln!(file, "ashar = {}", jadwal_adzan.ashar)?;
    writeln!(file, "maghrib = {}", jadwal_adzan.maghrib)?;
    writeln!(file, "isya = {}", jadwal_adzan.isya)?;

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
            in_location = &line[1..line.len()-1] == "location" || &line[1..line.len()-1] == "adzan";
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

async fn get_waktu_adzan(
    city_id: &str,
    day: &str,
    month: &str,
    year: &str,
) -> std::result::Result<WaktuAdzan, Box<dyn std::error::Error>> {
    let url = format!(
        "https://adzan-indonesia-api.vercel.app/adzan?cityId={}&month={}&year={}&date={}",
        city_id, month, year, day
    );

    let res: serde_json::Value = reqwest::get(url).await?.json().await?;

    let waktu_adzan = WaktuAdzan {
        shubuh: res["data"]["data"]["adzan"]["shubuh"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        dzuhur: res["data"]["data"]["adzan"]["dzuhur"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        ashar: res["data"]["data"]["adzan"]["ashr"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        maghrib: res["data"]["data"]["adzan"]["maghrib"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        isya: res["data"]["data"]["adzan"]["isya"]
            .as_str()
            .unwrap_or("")
            .to_string(),
    };

    Ok(waktu_adzan)
}


fn play_adzan(nama: &str) -> std::result::Result<(), Box<dyn Error>> {
    // Pindahkan semua ke dalam thread baru
    let nama = nama.to_string();
    thread::spawn(move || {
        let stream_handle = OutputStreamBuilder::open_default_stream().unwrap();
        let mixer = stream_handle.mixer();

        let file_path = if nama == "Subuh" {
            "audio/shubuh.mp3"
        } else {
            "audio/adzan.mp3"
        };

        let file = File::open(file_path).unwrap();
        let sink = rodio::play(mixer, BufReader::new(file)).unwrap();
        sink.set_volume(0.2);

        println!("Adzan '{}' dimulai", nama);

        thread::sleep(Duration::from_secs(240));
        drop(sink); 
    });

    Ok(())
}
fn main() -> std::result::Result<(), Box<dyn Error>> {

    let ui = AppWindow::new()?;
    let weak_ui = ui.as_weak();

    let timer = Timer::default();
    let mut last_adzan_played = String::new();
    timer.start(slint::TimerMode::Repeated, std::time::Duration::from_secs(1), move || {
        if let Some(ui) = weak_ui.upgrade() {
            let now = Local::now();
            let jam = format!("{:02}:{:02}:{:02}", now.hour(), now.minute(), now.second());
            ui.set_jam_saat_ini(SharedString::from(jam));
            
            let list_waktu_shalat = match read_location_settings("data/settings.ini") {
                Ok(settings) => {
                    vec![
                        JadwalShalat { nama: "Subuh", waktu: settings.get("shubuh").cloned().unwrap_or_default() },
                        JadwalShalat { nama: "Dzuhur", waktu: settings.get("dzuhur").cloned().unwrap_or_default() },
                        JadwalShalat { nama: "Ashar", waktu: settings.get("ashar").cloned().unwrap_or_default() },
                        JadwalShalat { nama: "Maghrib", waktu: settings.get("maghrib").cloned().unwrap_or_default() },
                        JadwalShalat { nama: "Isya", waktu: settings.get("isya").cloned().unwrap_or_default() },
                    ]
                }
                Err(e) => {
                    eprintln!("Gagal membaca settings: {}", e);
                    vec![
                        JadwalShalat { nama: "Shubuh", waktu: String::new() },
                        JadwalShalat { nama: "Dzuhur", waktu: String::new() },
                        JadwalShalat { nama: "Ashar", waktu: String::new() },
                        JadwalShalat { nama: "Maghrib", waktu: String::new() },
                        JadwalShalat { nama: "Isya", waktu: String::new() },
                    ]
                }
            };
            
            
            let mut index_selanjutnya = 0;
            for (i, j) in list_waktu_shalat.iter().enumerate() {
                if let Ok(waktu) = NaiveTime::parse_from_str(&j.waktu, "%H:%M") {
                    if waktu > now.time() {
                        index_selanjutnya = i;
                        break;
                    }
                }
            }
            if let Some(jadwal) = list_waktu_shalat.get(index_selanjutnya) {
                ui.set_next_shalat(SharedString::from(jadwal.nama));
            }
            for j in &list_waktu_shalat {
                if let Ok(waktu) = NaiveTime::parse_from_str(&j.waktu, "%H:%M") {
                    if waktu.hour() == now.hour() && waktu.minute() == now.minute() && last_adzan_played != j.nama {
                        if let Err(e) = play_adzan(j.nama) {
                            eprintln!("Failed to play adzan: {}", e);
                        }
                        last_adzan_played = j.nama.to_string();
                    }
                }
            }

       
          
        }
    });

    let date_gregorian = Local::now();
    let year: i16 = date_gregorian.year() as i16;
    let month: i8 = date_gregorian.month() as i8;
    let month_name = date_gregorian.format("%B").to_string();
    let day: i8 = date_gregorian.day() as i8;
    let date = jiff::civil::date(year, month, day);

    let hijr_date = HijriDate::from_gregorian(date, 0);

    let formatted = format!("{} {} {} H - {} {} {}", hijr_date.day, hijr_date.month_english, hijr_date.year, day, month_name, year);
    ui.set_tanggal_hari_ini(SharedString::from(formatted));

    match read_location_settings("data/settings.ini") {
        Ok(settings) => {
            let prayer_location = format!(
                "{} - {}",
                settings.get("name").unwrap_or(&"Unknown".to_string()),
                settings.get("gmt").unwrap_or(&"+0".to_string())
            );

            
            ui.set_location(SharedString::from(prayer_location));
            ui.set_adzan_ashar(SharedString::from(settings.get("ashar").unwrap_or(&"".to_string())));
            ui.set_adzan_dzuhur(SharedString::from(settings.get("dzuhur").unwrap_or(&"".to_string())));
            ui.set_adzan_isya(SharedString::from(settings.get("isya").unwrap_or(&"".to_string())));
            ui.set_adzan_maghrib(SharedString::from(settings.get("maghrib").unwrap_or(&"".to_string())));
            ui.set_adzan_shubuh(SharedString::from(settings.get("shubuh").unwrap_or(&"".to_string())));
        }
        Err(e) => {
            eprintln!("Gagal membaca settings: {}", e);
        }
    }
  
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



                let now = Local::now();
                let day = &now.day().to_string();
                let month = &now.month().to_string();
                let year = &now.year().to_string();
                let id_city = &city_id.to_string();
                let rt = Runtime::new().expect("Failed to create runtime");
                rt.block_on(async {
                    let waktu_adzan = get_waktu_adzan(id_city, day, month, year).await.expect("Failed to fetch prayer times");
                  
                    save_settings_to_ini("data/settings.ini", city_id, &city_name, &city_gmt, &waktu_adzan)
                    .expect("Failed to save settings");
                    show_message("Setting successfully saved!");
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_location(SharedString::from(format!("{}-{}", city_name, city_gmt)));
                        ui.set_adzan_shubuh(SharedString::from(waktu_adzan.shubuh));
                        ui.set_adzan_dzuhur(SharedString::from(waktu_adzan.dzuhur));
                        ui.set_adzan_ashar(SharedString::from(waktu_adzan.ashar));
                        ui.set_adzan_maghrib(SharedString::from(waktu_adzan.maghrib));
                        ui.set_adzan_isya(SharedString::from(waktu_adzan.isya));
                    }
                });

            }
        });
        dialog.show().unwrap();  
    }
    });
    ui.run()?;
    Ok(())
}

