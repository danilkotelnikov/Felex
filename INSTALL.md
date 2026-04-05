# Installation Guide

Three ways to get Felex running: download a ready-made installer, build from source, or run in development mode.

---

## Option 1: Windows Installer

The simplest path. Download the latest release from the [Releases page](https://github.com/danilkotelnikov/Felex/releases).

Two installer formats are available:
- **`.msi`** — standard Windows Installer (recommended for managed environments)
- **`.exe`** — NSIS installer with language selection (English / Russian)

Double-click, follow the wizard, launch Felex from Start Menu or desktop shortcut.

**System requirements:**
- Windows 10/11 (64-bit)
- 4 GB RAM minimum (8 GB recommended if using AI features)
- ~200 MB disk space

---

## Option 2: Build from Source

### Prerequisites

| Tool | Version | Why |
|------|---------|-----|
| [Node.js](https://nodejs.org/) | 18+ | Frontend build (React, Vite) |
| [Rust](https://rustup.rs/) | 1.70+ | Backend compilation |
| [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) | 2019+ | C++ compiler for Rust on Windows |
| [Python](https://www.python.org/) | 3.10+ | Data pipeline (optional, only for feed DB regeneration) |

### Automated Setup (Windows)

The setup script checks for all dependencies and installs what's missing:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/setup.ps1
```

This will:
1. Check Node.js, npm, Rust, Cargo, and VS Build Tools
2. Install missing tools via `winget` or `rustup`
3. Install the Tauri CLI
4. Run `npm install`

### Manual Setup

```bash
# 1. Clone the repository
git clone https://github.com/danilkotelnikov/Felex.git
cd Felex

# 2. Install JavaScript dependencies
npm install

# 3. Generate feed database artifacts for the frontend
npm run build:feed-runtime

# 4. Build the desktop application
npm run tauri:build
```

The build produces installers in the Cargo target directory (default: `C:\FelexBuild\tauri-target\release\bundle\`). Both `.msi` and `.exe` formats are created automatically.

If `C:\FelexBuild` is not writable, set a custom target directory:
```bash
set CARGO_TARGET_DIR=.\tmp-cargo-target
npm run tauri:build
```

### Build time expectations

First build compiles all Rust dependencies from source — expect 5–15 minutes depending on your machine. Subsequent builds are much faster thanks to incremental compilation.

---

## Option 3: Development Mode

For working on the code or just exploring:

```bash
# Start both backend and frontend with hot-reload
npm run dev:full
```

This launches:
- **Rust API server** on `http://localhost:7432`
- **Vite dev server** on `http://localhost:5173` (with proxy to the API)

Open `http://localhost:5173` in your browser.

You can also run the layers separately:

```bash
# Backend only
cargo run --bin felex-server

# Frontend only (needs backend running)
npm run dev
```

### Database setup

On first run, the backend creates and migrates the SQLite database automatically. To run migrations manually:

```bash
cargo run --bin migrate
```

To import feed data from the Python pipeline output:

```bash
cargo run --bin import-feeds
```

---

## AI Assistant Setup (Optional)

Felex includes an AI advisor that uses a local language model to comment on rations. This feature is entirely optional — everything else works without it.

### Install Ollama

Download from [ollama.ai/download](https://ollama.ai/download) and install.

### Pull a model

```bash
# Recommended for most PCs (2-3 GB download)
ollama pull qwen3.5:4b

# Better quality, needs more RAM (5-6 GB download)
ollama pull qwen3.5:9b
```

### Verify it works

```bash
ollama run qwen3.5:4b "Hello"
```

Felex auto-detects Ollama running on `localhost:11434`. No additional configuration needed.

### Using a different model backend

Felex also supports any OpenAI-compatible API. Configure via environment variables or `src/agent/config.rs`.

---

## Regenerating the Feed Database

The feed database is pre-built and ships with the repo in `database/output/`. You only need to regenerate it if you're modifying feed data.

```bash
cd database
pip install -r requirements.txt
python -m pytest                  # verify tests pass
cd ..
npm run build:feed-runtime        # rebuild frontend artifacts
```

---

## Troubleshooting

### `rc.exe` not found during Rust build

The Windows resource compiler is needed for Tauri builds. The build script tries to locate it automatically, but if it fails:

1. Open Visual Studio Installer
2. Modify your Build Tools installation
3. Ensure "Windows SDK" is checked under "Desktop development with C++"

### Build fails with OneDrive path errors

OneDrive can interfere with Rust compilation due to file locking. Set a build directory outside OneDrive:

```bash
set CARGO_TARGET_DIR=C:\FelexBuild\target
npm run tauri:build
```

### `npm run build:feed-runtime` fails

Make sure Python 3 is on your PATH and the database dependencies are installed:

```bash
python --version
cd database && pip install -r requirements.txt
```

### Ollama not connecting

Check that Ollama is running (`ollama serve`) and accessible on port 11434:

```bash
curl http://localhost:11434/api/tags
```
