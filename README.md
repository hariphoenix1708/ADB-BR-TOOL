# Android App Backup & Restore (Windows)

A professional-grade Windows desktop application for managing Android backups. Connects via USB or Wireless ADB, supporting non-root and root devices, Android 14+ support.

**Note:** This is a v1 prototype implementing the core Tauri (Rust + React/TS) stack, UI scaffolding, and basic ADB communication (device listing, app discovery, APK pulling). Advanced data streaming, encryption, compression, and robust restore functionality require a more extensive implementation and are stubbed for the current iteration.

## Architecture & Design
Please see [DESIGN_AND_ARCHITECTURE.md](./DESIGN_AND_ARCHITECTURE.md) for detailed technical decisions, technology stack recommendations, and architectural plans.

## Development Setup

Prerequisites:
- Node.js (v20+)
- Rust (Stable)
- System dependencies (Linux: `libglib2.0-dev`, `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`)

```bash
npm install
npm run tauri dev
```
