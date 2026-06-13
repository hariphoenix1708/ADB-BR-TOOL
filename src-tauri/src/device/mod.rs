use crate::adb::execute_adb;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppInfo {
    pub package_name: String,
    pub path: String,
    pub version: String,
    pub is_system: bool,
}

pub fn is_root_available(device_id: &str) -> bool {
    match execute_adb(&["-s", device_id, "shell", "su", "-c", "id"]) {
        Ok(output) => output.contains("uid=0"),
        Err(_) => false,
    }
}

pub fn get_installed_apps(device_id: &str) -> Result<Vec<AppInfo>, String> {
    // Basic implementation: just get package names and paths
    let output = execute_adb(&["-s", device_id, "shell", "pm", "list", "packages", "-f"])?;
    let mut apps = Vec::new();

    for line in output.lines() {
        if line.starts_with("package:") {
            let line = line.strip_prefix("package:").unwrap();
            if let Some((path, package)) = line.rsplit_once('=') {
                apps.push(AppInfo {
                    package_name: package.to_string(),
                    path: path.to_string(),
                    version: "Unknown".to_string(), // Requires further dumpsys parsing
                    is_system: path.starts_with("/system") || path.starts_with("/vendor") || path.starts_with("/product"),
                });
            }
        }
    }

    Ok(apps)
}
