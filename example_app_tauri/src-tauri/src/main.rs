// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    audio_engine_service::service::init();
    example_app_tauri_lib::run();
}
