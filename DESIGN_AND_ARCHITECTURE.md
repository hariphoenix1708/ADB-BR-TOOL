# Android App Backup & Restore: Windows Desktop Application

## 1. Platform and Technology Comparison & Recommendation

### Comparison of Options
* **WinUI 3 / C++ or C#:** The latest native UI framework for Windows. Excellent integration with Windows APIs, modern Fluent design. However, it can have a steeper learning curve, less robust community compared to web stacks, and is strictly Windows-only.
* **WPF / .NET (C#):** Highly mature, huge ecosystem, excellent performance for enterprise apps. C# is fantastic for handling complex logic, multithreading, and file I/O. UI can look slightly dated unless heavily customized.
* **Rust:** Unmatched performance and memory safety. Great for backend operations (hashing, file IO, ADB communication), but building UI directly in pure Rust (e.g., egui, slint) is still evolving and may lack the polish of mature UI frameworks.
* **Electron (Node.js/Web):** Extremely easy to build beautiful, modern UIs using web tech (React/Vue). Huge ecosystem. However, it is resource-heavy, has a large binary size, and Node.js might not be as performant for heavy file operations (like huge backup compressions and hashing) as compiled languages.
* **Tauri (Rust/Web):** Combines the best of both worlds. You build the UI with web technologies (React, Vue, etc.) ensuring a polished, modern, and easily maintainable frontend. The backend runs on Rust, providing near-native performance for heavy tasks like compression, encryption, hashing, and ADB communication. It results in a tiny binary and uses low RAM compared to Electron.
* **Flutter:** Great for cross-platform, but desktop support, while stable, can sometimes feel non-native on Windows, and handling very heavy system-level integration (like custom ADB wrappers and root file system manipulation) might require extensive platform channels to C++/Rust anyway.

### Recommendation: **Tauri (Rust Backend + React/TypeScript Frontend)**
**Why?**
- **Performance & Safety:** Rust is ideal for the backend engine. We need to handle large file streams (backups), calculate checksums (SHA-256), encrypt streams (AES-256), and compress data. Rust does this safely and blazingly fast.
- **UI/UX:** React/TypeScript allows building a highly responsive, modern, "polished" UI, supporting complex state management for features like manual app selection, drag-and-drop, and real-time backup progress tracking.
- **System Integration:** Rust can easily interface with system processes (calling ADB) and handle raw byte streams securely.

---

## 2. Architecture Requirements

### 2.1 Module Structure
* **`frontend/` (React/TS):**
  * `components/`: UI components (App list, progress bars, settings).
  * `views/`: Main screens (Dashboard, Backup, Restore, Settings).
  * `state/`: State management (Zustand or Redux) for device status, selected apps, backup progress.
  * `services/`: Tauri IPC bindings to communicate with the Rust backend.
* **`src-tauri/` (Rust Backend):**
  * `main.rs`: Entry point, Tauri setup.
  * `adb/`: ADB communication layer (device detection, command execution, shell).
  * `backup/`: Backup engine (fetching APKs, Data, OBB, compression, encryption).
  * `restore/`: Restore engine (pushing files, setting permissions, installing APKs).
  * `storage/`: Indexing, manifest creation, history management.
  * `security/`: Encryption (AES-GCM), key management, and checksum validation (SHA-256).
  * `device/`: Device capability detection (Android version, root status, users/profiles).

### 2.2 Data Flow (Windows <-> Android)
1. **Discovery:** App polls `adb devices` or uses mDNS for Wireless ADB.
2. **Analysis:** App runs `adb shell pm list packages` and `adb shell dumpsys package` to gather app details.
3. **Backup Flow:**
   - Android -> ADB -> Rust Backend -> Stream to Disk.
   - Rust compresses and encrypts the stream on-the-fly.
   - Rust calculates hashes during the stream.
   - Rust reports progress to Frontend via Tauri events.
4. **Restore Flow:**
   - Disk -> Rust Backend -> Decrypt & Decompress stream -> ADB -> Android.
   - Rust verifies hash before restoring.
   - App uses `pm install` for APKs and pushes data via `adb push` or `tar` pipe.

### 2.3 ADB Communication Flow
- The backend will bundle a known good version of `adb.exe` to ensure compatibility.
- Rust will use `std::process::Command` to spawn ADB processes.
- For large data transfers, we will use ADB's `exec-out` and `exec-in` to stream data directly (e.g., `adb exec-out "tar -cz /data/data/com.example.app"`), avoiding intermediate files on the device where possible.

### 2.4 Root Command Execution
- Safe execution involves prefixing commands with `su -c`.
- Example: `adb shell su -c "tar -cz /data/data/com.example.app"`
- The app will test root availability during device connection: `adb shell su -c "id"` and parse for `uid=0`.

### 2.5 File Operations (Enumeration, Copy, Compress, Encrypt, Verify)
- **Enumeration:** `find` and `stat` via ADB shell.
- **Copy:** Streaming via `tar` over `adb exec-out`.
- **Compress:** `flate2` or `zstd` crate in Rust.
- **Encrypt:** `aes-gcm` crate in Rust. Stream encryption to handle large files.
- **Verify:** `sha2` crate (SHA-256).

### 2.6 Storage & Indexing
- Backups stored in user-defined directories.
- Each backup session generates a `manifest.json` containing:
  - Device info, Android version.
  - App metadata (package, version code, signatures).
  - File index with relative paths, sizes, and SHA-256 hashes.
- SQLite database (`sqlx` in Rust) to keep a local history and index of all backups for fast search and diffing.

### 2.7 Safe & Reversible Restore
- **Dry-run:** Simulate the restore by verifying backup integrity and checking device space.
- **Merge/Replace:** Option to clear existing app data (`pm clear`) before restore, or simply overwrite files.
- **Permissions:** Crucial for root restore. The app will record the original UID/GID of the app data from `packages.xml` or `stat`, and explicitly `chown` and `restorecon` the restored data to ensure Android's SELinux and permission models are respected.

---

## 3. Security Requirements

- **Encryption at Rest:** All backups can be AES-256-GCM encrypted. Keys derived from a user password using PBKDF2 or Argon2.
- **Key Protection:** Key derivation happens in memory. Keys are not stored unless requested (via Windows Credential Manager).
- **Validation:** Sanitize all paths to prevent Path Traversal attacks (e.g., escaping the backup directory).
- **Accidental Overwrite:** Frontend will warn users before restoring over an existing, newer version of an app.
- **Integrity Checks:** Before any restore begins, the `manifest.json` checksums are verified against the backup archive.
- **Logging:** All ADB commands and file operations are logged to a local, rotated log file (sensitive data redacted).

---

## 4. Advanced Features Implementation Strategy

- **Incremental Backups:** Achieved by comparing file modification times (`mtime`) and hashes against the previous backup's manifest. We use `rsync` style logic over ADB or basic diffing.
- **Split APKs:** Extracted using `adb shell pm path <package>`, parsing the outputs, and pulling all `.apk` files (base + config splits). They must be restored together via `pm install-create`, `pm install-write`, and `pm install-commit`.
- **Per-user / Per-profile:** Handled by adding the `--user <ID>` flag to `pm` commands and targeting `/data/user/<ID>/` instead of the default `/data/data/`.
