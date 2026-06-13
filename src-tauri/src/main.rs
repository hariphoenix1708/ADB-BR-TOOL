// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod adb;
mod backup;
mod device;

use adb::AdbDevice;
use device::AppInfo;
use backup::BackupRequest;
use tauri::Emitter;

#[tauri::command]
fn get_devices() -> Result<Vec<AdbDevice>, String> {
    adb::list_devices()
}

#[tauri::command]
fn get_apps(device_id: String) -> Result<Vec<AppInfo>, String> {
    device::get_installed_apps(&device_id)
}

#[tauri::command]
fn check_root(device_id: String) -> bool {
    device::is_root_available(&device_id)
}

#[tauri::command]
async fn start_backup(app: tauri::AppHandle, request: BackupRequest) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        backup::create_backup(request, |progress| {
            let _ = app.emit("backup-progress", progress);
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn start_restore(app: tauri::AppHandle, request: BackupRequest) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        backup::restore_backup(request, |progress| {
            let _ = app.emit("backup-progress", progress); // Reusing event name for simplicity
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_devices,
            get_apps,
            check_root,
            start_backup,
            start_restore
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
