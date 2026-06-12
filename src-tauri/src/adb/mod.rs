use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs::File;

#[derive(Debug, Serialize, Deserialize)]
pub struct AdbDevice {
    pub id: String,
    pub state: String,
    pub model: String,
}

pub fn get_adb_path() -> PathBuf {
    // In a real app, we'd bundle adb and resolve its path here.
    // For now, assume it's in PATH.
    PathBuf::from("adb")
}

pub fn execute_adb(args: &[&str]) -> Result<String, String> {
    let output = Command::new(get_adb_path())
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute ADB: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

pub fn execute_adb_to_file(args: &[&str], output_path: &Path) -> Result<(), String> {
    let file = File::create(output_path).map_err(|e| format!("Failed to create output file: {}", e))?;
    let status = Command::new(get_adb_path())
        .args(args)
        .stdout(Stdio::from(file))
        .stderr(Stdio::piped())
        .status()
        .map_err(|e| format!("Failed to execute ADB: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err("ADB command failed".to_string())
    }
}

pub fn check_dir_exists(device_id: &str, path: &str, needs_root: bool) -> bool {
    let cmd = format!("test -d \"{}\" && echo exists", path);
    let output = if needs_root {
        execute_adb(&["-s", device_id, "shell", "su", "-c", &cmd])
    } else {
        execute_adb(&["-s", device_id, "shell", &cmd])
    };

    match output {
        Ok(res) => res.trim() == "exists",
        Err(_) => false,
    }
}

pub fn list_devices() -> Result<Vec<AdbDevice>, String> {
    let output = execute_adb(&["devices", "-l"])?;
    let mut devices = Vec::new();

    for line in output.lines().skip(1) { // Skip "List of devices attached"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let id = parts[0].to_string();
            let state = parts[1].to_string();

            // Try to extract model if available (e.g. model:Pixel_6)
            let mut model = "Unknown".to_string();
            for part in parts.iter().skip(2) {
                if part.starts_with("model:") {
                    model = part.replace("model:", "");
                }
            }

            devices.push(AdbDevice { id, state, model });
        }
    }

    Ok(devices)
}
