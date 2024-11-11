// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use auto_launch::AutoLaunchBuilder;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use playback_rs::{Player, Song};
use rand::Rng;
use std::fs::File;
use std::io::prelude::*;
use std::ptr;
use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;
use tauri::{
    utils::platform::current_exe, CustomMenuItem, RunEvent, SystemTray, SystemTrayEvent,
    SystemTrayMenu, SystemTrayMenuItem, WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};
use winapi::um::{
    wingdi::{BitBlt, SRCCOPY},
    winuser::{
        GetDC, GetDesktopWindow, GetSystemMetrics, RedrawWindow, ReleaseDC, RDW_ALLCHILDREN,
        RDW_ERASE, RDW_INVALIDATE, SM_CXSCREEN, SM_CYSCREEN,
    },
};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Message {
    r#type: String,
    username: String,
    message: String,
}

// Constants for controlling the screen shake effect
const SHAKE_DURATION: i32 = 8;
const SHAKE_INTENSITY: i32 = 6;
const SHAKE_SPEED: i32 = 2;

fn shake_screen() {
    // Get screen dimensions
    let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    println!("Width: {}, Height: {}", width, height);

    // Get handles for the entire screen (NULL == whole screen in Windows API)
    let window_handle = unsafe { GetDesktopWindow() };
    let device_context_handle = unsafe { GetDC(window_handle) };

    // Variables to track cumulative screen displacement
    let mut farx: i32 = 0;
    let mut fary: i32 = 0;

    // Calculate total iterations based on duration and speed
    let iterations = 1000 / (20 / SHAKE_SPEED) * SHAKE_DURATION;

    for _ in 0..iterations {
        // Generate random displacement values
        let mut randx =
            rand::thread_rng().gen_range(0..(SHAKE_INTENSITY * 20)) - SHAKE_INTENSITY * 10;
        let mut randy =
            rand::thread_rng().gen_range(0..(SHAKE_INTENSITY * 20)) - SHAKE_INTENSITY * 10;

        // Correct screen if too distorted on X axis
        if farx.abs() > SHAKE_INTENSITY * 20 {
            unsafe {
                RedrawWindow(
                    window_handle,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    RDW_ERASE | RDW_INVALIDATE | RDW_ALLCHILDREN,
                );
                randx = -farx; // Fixed: should be randx instead of randy
            }
        }

        // Correct screen if too distorted on Y axis
        if fary.abs() > SHAKE_INTENSITY * 20 {
            unsafe {
                RedrawWindow(
                    window_handle,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    RDW_ERASE | RDW_INVALIDATE | RDW_ALLCHILDREN,
                );
                randy = -fary; // Fixed: should be randy instead of randx
            }
        }

        // Update cumulative displacement
        farx += randx;
        fary += randy;

        unsafe {
            // Perform the screen shake by copying and offsetting screen contents
            BitBlt(
                device_context_handle,
                randx,
                randy,
                width,
                height,
                device_context_handle,
                0,
                0,
                SRCCOPY,
            );
            // Wait before next shake
            std::thread::sleep(Duration::from_millis((20 / SHAKE_SPEED) as u64));
        }
    }

    // Cleanup: redraw screen and release device context
    unsafe {
        RedrawWindow(
            window_handle,
            ptr::null_mut(),
            ptr::null_mut(),
            RDW_ERASE | RDW_INVALIDATE | RDW_ALLCHILDREN,
        );
        ReleaseDC(window_handle, device_context_handle);
    }
}

fn play_juicer(app: &tauri::AppHandle) {
    // Deserialize the settings from the storage
    let player = Player::new(None).expect("Failed to open an audio output."); // Create a player to play audio with cpal.

    let resource_path = app
        .path_resolver()
        .resolve_resource("_up_/public/audio/juicer.mp3")
        .expect("failed to resolve resource");

    let song = Song::from_file(resource_path.to_str().unwrap(), None)
        .expect("Failed to load or decode the song."); // Decode a song from a file

    player.play_song_next(&song, None);

    while player.has_current_song() {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
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
        let app_clone = app.clone();

        match message.r#type.as_str() {
            "readChat" => {
                println!(
                    "Received message from {}: {}",
                    message.username, message.message
                );

                app_clone.emit_all("read-chat", message.clone()).unwrap();

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

                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                    let window = app_clone.get_window("main").unwrap();
                    window.hide().unwrap();
                    println!("Hiding window")
                });
            }

            "shake" => {
                tokio::spawn(async move {
                    shake_screen();
                });

                // Spawn screen shake on a separate thread
                tokio::spawn(async move {
                    play_juicer(&app_clone);
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
