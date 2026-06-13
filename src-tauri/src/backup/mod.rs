use crate::adb::{execute_adb, execute_adb_to_file, execute_adb_from_file, check_dir_exists};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct ApkRecord {
    pub local_filename: String,
    pub device_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataRecord {
    pub local_filename: String,
    pub device_path: String,
    pub requires_root: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupManifest {
    pub package_name: String,
    pub apks: Vec<ApkRecord>,
    pub data_archives: Vec<DataRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackupProgress {
    pub package_name: String,
    pub status: String,
    pub percentage: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupRequest {
    pub device_id: String,
    pub apps: Vec<String>,
    pub output_dir: String,
    pub backup_apk: bool,
    pub backup_data: bool,
    pub backup_obb: bool,
    pub backup_media: bool,
    pub backup_user: bool,
    pub backup_user_de: bool,
}

fn log_msg(output_dir: &Path, msg: &str) {
    let log_path = output_dir.join("backup.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = writeln!(file, "{}", msg);
    }
}

pub fn create_backup(
    request: BackupRequest,
    progress_callback: impl Fn(BackupProgress),
) -> Result<(), String> {
    let output_path = PathBuf::from(&request.output_dir);

    // Ensure output directory exists
    if !output_path.exists() {
        fs::create_dir_all(&output_path)
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;
    }

    log_msg(&output_path, &format!("--- Starting backup session for device: {} ---", request.device_id));

    for package in &request.apps {
        progress_callback(BackupProgress {
            package_name: package.clone(),
            status: "Starting backup...".to_string(),
            percentage: 0,
        });

        let app_dir = output_path.join(package);
        if !app_dir.exists() {
            fs::create_dir(&app_dir)
                .map_err(|e| format!("Failed to create app directory: {}", e))?;
        }

        let mut manifest = BackupManifest {
            package_name: package.clone(),
            apks: Vec::new(),
            data_archives: Vec::new(),
        };

        if request.backup_apk {
            progress_callback(BackupProgress {
                package_name: package.clone(),
                status: "Extracting APK(s)...".to_string(),
                percentage: 20,
            });

            // Get APK paths
            let path_output = execute_adb(&[
                "-s",
                &request.device_id,
                "shell",
                "pm",
                "path",
                package,
            ])?;

            let mut paths = Vec::new();
            for line in path_output.lines() {
                if let Some(path) = line.strip_prefix("package:") {
                    paths.push(path.to_string());
                }
            }

            // Pull each APK
            for path in paths {
                let file_name = PathBuf::from(&path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let dest_path = app_dir.join(&file_name);

                log_msg(&output_path, &format!("[{}] Pulling APK: {}", package, path));
                if let Err(e) = execute_adb(&[
                    "-s",
                    &request.device_id,
                    "pull",
                    &path,
                    dest_path.to_str().unwrap(),
                ]) {
                    let err_msg = format!("Error pulling APK {}: {}", path, e);
                    progress_callback(BackupProgress {
                        package_name: package.clone(),
                        status: err_msg.clone(),
                        percentage: 20, // keep at 20 but show error
                    });
                    // Log error and continue rather than aborting entirely
                    log_msg(&output_path, &format!("[{}] {}", package, err_msg));
                    eprintln!("{}", err_msg);
                } else {
                    manifest.apks.push(ApkRecord {
                        local_filename: file_name,
                        device_path: path.clone(),
                    });
                    log_msg(&output_path, &format!("[{}] Successfully pulled APK to {}", package, dest_path.display()));
                }
            }
        }

        let mut data_pull_targets = Vec::new();

        if request.backup_data {
            data_pull_targets.push((
                format!("/sdcard/Android/data/{}", package),
                "data.tar.gz",
                "Backing up App Data...",
                false // doesn't strictly need root if accessible, though scope storage might restrict. We'll use false.
            ));
        }
        if request.backup_user {
             data_pull_targets.push((
                format!("/data/user/0/{}", package),
                "user.tar.gz",
                "Backing up User Data...",
                true
            ));
        }
        if request.backup_user_de {
            data_pull_targets.push((
                format!("/data/user_de/0/{}", package),
                "user_de.tar.gz",
                "Backing up User DE Data...",
                true
            ));
        }
        if request.backup_obb {
             data_pull_targets.push((
                format!("/sdcard/Android/obb/{}", package),
                "obb.tar.gz",
                "Backing up OBB...",
                false
            ));
        }
        if request.backup_media {
             data_pull_targets.push((
                format!("/sdcard/Android/media/{}", package),
                "media.tar.gz",
                "Backing up Media...",
                false
            ));
        }

        let total_targets = data_pull_targets.len();
        for (i, (source_path, dest_filename, status_msg, needs_root)) in data_pull_targets.into_iter().enumerate() {
            let percentage = 20 + ((i as f32 / total_targets as f32) * 70.0) as u8;

            // Check if directory exists before pulling
            log_msg(&output_path, &format!("[{}] Checking if directory exists: {}", package, source_path));
            if !check_dir_exists(&request.device_id, &source_path, needs_root) {
                log_msg(&output_path, &format!("[{}] Directory does not exist, skipping: {}", package, source_path));
                continue;
            }

            progress_callback(BackupProgress {
                package_name: package.clone(),
                status: status_msg.to_string(),
                percentage,
            });

            let dest_file = app_dir.join(dest_filename);
            log_msg(&output_path, &format!("[{}] Executing tar for {} to {}", package, source_path, dest_file.display()));

            if needs_root {
                let tar_cmd = format!("tar -czf - -C {} . 2>/dev/null", source_path);
                if let Err(e) = execute_adb_to_file(&[
                    "-s",
                    &request.device_id,
                    "exec-out",
                    "su",
                    "-c",
                    &tar_cmd,
                ], &dest_file) {
                    log_msg(&output_path, &format!("[{}] Failed to backup {}: {}", package, source_path, e));
                } else {
                    manifest.data_archives.push(DataRecord {
                        local_filename: dest_filename.to_string(),
                        device_path: source_path.clone(),
                        requires_root: true,
                    });
                    log_msg(&output_path, &format!("[{}] Successfully backed up {}", package, source_path));
                }
            } else {
                let tar_cmd = format!("tar -czf - -C {} . 2>/dev/null", source_path);
                if let Err(e) = execute_adb_to_file(&[
                    "-s",
                    &request.device_id,
                    "exec-out",
                    &tar_cmd,
                ], &dest_file) {
                    log_msg(&output_path, &format!("[{}] Failed to backup {}: {}", package, source_path, e));
                } else {
                    manifest.data_archives.push(DataRecord {
                        local_filename: dest_filename.to_string(),
                        device_path: source_path.clone(),
                        requires_root: false,
                    });
                    log_msg(&output_path, &format!("[{}] Successfully backed up {}", package, source_path));
                }
            }
        }

        let manifest_path = app_dir.join("backup_manifest.json");
        if let Ok(manifest_json) = serde_json::to_string_pretty(&manifest) {
            let _ = fs::write(manifest_path, manifest_json);
        }

        progress_callback(BackupProgress {
            package_name: package.clone(),
            status: "Done".to_string(),
            percentage: 100,
        });
    }

    Ok(())
}

pub fn restore_backup(
    request: BackupRequest,
    progress_callback: impl Fn(BackupProgress),
) -> Result<(), String> {
    let output_path = PathBuf::from(&request.output_dir);

    for package in &request.apps {
        progress_callback(BackupProgress {
            package_name: package.clone(),
            status: "Starting restore...".to_string(),
            percentage: 0,
        });

        let app_dir = output_path.join(package);
        if !app_dir.exists() {
            log_msg(&output_path, &format!("[{}] Restore skipped: Backup directory does not exist", package));
            continue;
        }

        let manifest_path = app_dir.join("backup_manifest.json");
        let manifest: Option<BackupManifest> = if manifest_path.exists() {
            if let Ok(content) = fs::read_to_string(&manifest_path) {
                serde_json::from_str(&content).ok()
            } else { None }
        } else { None };

        if let Some(man) = manifest {
            if request.backup_apk {
                progress_callback(BackupProgress {
                    package_name: package.clone(),
                    status: "Installing APK(s)...".to_string(),
                    percentage: 30,
                });

                // For V1 we still just look for base.apk if it exists, or the first APK in manifest
                // Proper split APK installation requires `pm install-create` session.
                let base_apk = app_dir.join("base.apk");
                let apk_to_install = if base_apk.exists() {
                    Some(base_apk)
                } else if !man.apks.is_empty() {
                    Some(app_dir.join(&man.apks[0].local_filename))
                } else {
                    None
                };

                if let Some(apk_path) = apk_to_install {
                    log_msg(&output_path, &format!("[{}] Installing APK: {}", package, apk_path.display()));
                    let _ = execute_adb(&[
                        "-s",
                        &request.device_id,
                        "install",
                        "-r",
                        apk_path.to_str().unwrap(),
                    ]);
                }
            }

            if request.backup_data {
                progress_callback(BackupProgress {
                    package_name: package.clone(),
                    status: "Restoring Data...".to_string(),
                    percentage: 70,
                });


                for data_record in man.data_archives.iter() {
                    let local_file = app_dir.join(&data_record.local_filename);
                    if !local_file.exists() {
                        log_msg(&output_path, &format!("[{}] Missing local archive for restore: {}", package, local_file.display()));
                        continue;
                    }

                    log_msg(&output_path, &format!("[{}] Restoring archive {} to {}", package, data_record.local_filename, data_record.device_path));

                    // Create directory first
                    let mkdir_cmd = format!("mkdir -p {}", data_record.device_path);
                    if data_record.requires_root {
                        let _ = execute_adb(&["-s", &request.device_id, "shell", "su", "-c", &mkdir_cmd]);
                    } else {
                        let _ = execute_adb(&["-s", &request.device_id, "shell", &mkdir_cmd]);
                    }

                    // Extract tar via exec-in
                    let tar_cmd = format!("tar -xzf - -C {}", data_record.device_path);
                    if data_record.requires_root {
                        if let Err(e) = execute_adb_from_file(&[
                            "-s",
                            &request.device_id,
                            "exec-in",
                            "su",
                            "-c",
                            &tar_cmd
                        ], &local_file) {
                            log_msg(&output_path, &format!("[{}] Failed to restore {}: {}", package, data_record.local_filename, e));
                        }
                    } else {
                         if let Err(e) = execute_adb_from_file(&[
                            "-s",
                            &request.device_id,
                            "exec-in",
                            &tar_cmd
                        ], &local_file) {
                            log_msg(&output_path, &format!("[{}] Failed to restore {}: {}", package, data_record.local_filename, e));
                        }
                    }
                }
            }
        } else {
            log_msg(&output_path, &format!("[{}] Restore skipped: No backup_manifest.json found", package));
        }

        progress_callback(BackupProgress {
            package_name: package.clone(),
            status: "Restore Complete".to_string(),
            percentage: 100,
        });
    }

    Ok(())
}
