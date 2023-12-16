// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use auto_launch::AutoLaunchBuilder;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use playback_rs::{Player, Song};
use std::fs::File;
use std::io::prelude::*;
use std::sync::Arc;
use tauri::Manager;
use tauri::{
    utils::platform::current_exe, CustomMenuItem, RunEvent, SystemTray, SystemTrayEvent,
    SystemTrayMenu, SystemTrayMenuItem, WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Message {
    r#type: String,
    username: String,
    message: String,
}

pub fn show_window(app: &tauri::AppHandle) {
    let window = app.get_window("main").unwrap();
    window.show().unwrap();
    window.set_focus().unwrap();
}

pub fn register_system_tray() -> impl Fn(&tauri::AppHandle, SystemTrayEvent) {
    |app, event| match event {
        SystemTrayEvent::LeftClick { .. } => show_window(app),
        SystemTrayEvent::DoubleClick { .. } => show_window(app),
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "quit" => {
                std::process::exit(0);
            }
            "hide" => {
                let window = app.get_window("main").unwrap();
                window.hide().unwrap();
            }
            "show" => show_window(app),
            _ => {}
        },
        _ => {}
    }
}

const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');

pub fn google_tts_to_file(text: &str, filename: &str) {
    let len = text.len();
    let text = utf8_percent_encode(text, FRAGMENT).to_string();

    let url = format!("https://translate.google.fr/translate_tts?ie=UTF-8&q={}&tl=ar&total=1&idx=0&textlen={}&tl=ar&client=tw-ob&ttsspeed=1", text, len);
    let response = minreq::get(url).send().unwrap().into_bytes();

    let mut file = File::create(filename).unwrap();
    file.write_all(&response).unwrap();

}

pub fn tts(message: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("TTS: {}", message);
    let filename = "tts.mp3";
    google_tts_to_file(&message, &filename);

    let player = Player::new(None).expect("Failed to open an audio output."); // Create a player to play audio with cpal.
    let song = Song::from_file(filename, None).expect("Failed to load or decode the song."); // Decode a song from a file

    player.play_song_next(&song, None).unwrap();

    // Wait until the song has ended to exit
    while player.has_current_song() {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    Ok(())
}

async fn listen_for_events(app: Arc<tauri::AppHandle>) -> Result<(), Box<dyn std::error::Error>> {
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let mut con = client.get_connection()?;
    let mut pubsub = con.as_pubsub();

    // Subscribe to readChat channel
    pubsub.subscribe("readChat")?;

    loop {
        let msg = pubsub.get_message()?;
        let payload: String = msg.get_payload()?;
        let message: Message = serde_json::from_str(&payload)?;

        match message.r#type.as_str() {
            "readChat" => {
                println!(
                    "Received message from {}: {}",
                    message.username, message.message
                );

                app.emit_all("read-chat", message.clone()).unwrap();

                tokio::spawn(async move {
                    match tts(message.message.to_lowercase()) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Error playing tts: {}", e);
                        }
                    }
                });

                let window = app.get_window("main").unwrap();
                window.show().unwrap();
                window.set_focus().unwrap();
                let _ = window.move_window(Position::TopRight);

                // Hide after 10 seconds
                let app_clone = app.clone();

                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                    let window = app_clone.get_window("main").unwrap();
                    window.hide().unwrap();
                    println!("Hiding window")
                });
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let hide = CustomMenuItem::new("hide".to_string(), "Hide");
    let tray_menu = SystemTrayMenu::new()
        .add_item(quit)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(hide);

    let app = tauri::Builder::default()
        .system_tray(SystemTray::new().with_menu(tray_menu))
        .plugin(tauri_plugin_positioner::init())
        .on_system_tray_event(register_system_tray())
        .on_system_tray_event(|app, event| {
            tauri_plugin_positioner::on_tray_event(app, &event);
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    let app_exe = current_exe().unwrap();
    let app_exe = dunce::canonicalize(app_exe).unwrap();
    let app_name = app_exe.file_stem().unwrap().to_str().unwrap();
    let app_path = app_exe.as_os_str().to_str().unwrap();

    let auto = AutoLaunchBuilder::new()
        .set_app_name(app_name)
        .set_app_path(app_path)
        .set_use_launch_agent(true)
        .build()
        .unwrap();

    let already_configured = auto.is_enabled().unwrap();

    if !already_configured {
        auto.enable().unwrap();
    }

    let safe_handler = Arc::new(app.handle());

    tokio::spawn(async move {
        match listen_for_events(safe_handler).await {
            Ok(_) => {}
            Err(e) => {
                println!("Error setting to pub sub: {}", e);
            }
        }
    });

    app.run(|app_handle, event| match event {
        RunEvent::WindowEvent { label, event, .. } => match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let window = app_handle.get_window(&label).unwrap();
                window.hide().unwrap();
            }
            _ => (),
        },
        _ => (),
    });
}
