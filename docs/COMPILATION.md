# Building MAMESET Cleaner

This document explains how to build MAMESET Cleaner from source, on Windows 10/11 (x64).

## Prerequisites

- **Rust** (2021 edition) via [rustup](https://rustup.rs), with the `x86_64-pc-windows-msvc` target
- **Visual Studio Build Tools** ("C++ build tools" workload) — required by Rust's `msvc` toolchain on Windows
- **Inno Setup 7** — only needed to build the installer (`installer/inno_setup_script.iss`), not to build the software itself

Check your installation:

```powershell
rustc --version
cargo --version
```

## Development build

```powershell
cargo build
```

The binary is generated in `target\debug\mameset_cleaner.exe`. In development mode, logs (`tracing`) remain visible in the terminal.

## Release build (optimized)

```powershell
cargo build --release
```

The final binary is generated in `target\release\mameset_cleaner.exe`. This mode:
- enables full optimizations (`opt-level = 3`, LTO, a single codegen unit);
- strips debug symbols (`strip`) to reduce the file size;
- hides the console window at startup (no console appears on double-click);
- embeds the application icon (`assets/icons/app.ico`) into the executable.

## Running the tests

```powershell
cargo test
```

Includes unit tests (in each module under `src/`) and integration tests (`test/*.rs`), for a total of more than 60 tests covering parsing, scanning, deduplication, filtering and cleanup.

## Building the Windows installer

1. Build the binary in release mode (`cargo build --release`).
2. Open `installer/inno_setup_script.iss` with Inno Setup 7 (or run `ISCC.exe installer\inno_setup_script.iss` from the command line).
3. The generated installer (`MAMESET-Cleaner-Setup-vX.Y.Z.exe`) is placed in `installer/output/`.

The installer targets Windows 10 and Windows 11 (x64), uses the application icon, and supports silent installation and uninstallation (`/VERYSILENT`).

## Project structure

```
MAMESET-Cleaner/
├── src/            Rust source code (core logic + interface)
├── ui/             Slint user interface (.slint)
├── assets/         Icons and translation files (i18n)
├── test/           Integration tests
├── installer/      Inno Setup 7 script
└── docs/           Documentation (including this file)
```
