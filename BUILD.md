Local build notes and troubleshooting

Prerequisites

- Install Rust toolchain (rustup) and cargo: https://rustup.rs/
- Install Node.js (v22 recommended) and npm
- Install Xcode command line tools on macOS: xcode-select --install
- Install GNU make, pkg-config, cmake if not present (brew install pkg-config cmake make)

Important: MuPDF native build may fail if your repository path contains spaces (e.g., "Obsidian Vault"). If you encounter native build failures during mupdf-sys compilation, use one of the workarounds:

1. Create a symlink to a path without spaces and build from there:
   ln -s "$(pwd)" ~/steinline
   cd ~/steinline
   mise run build

2. Move the repository to a path without spaces, e.g. ~/Projects/steinline

3. If you prefer to avoid MuPDF entirely, consider editing Cargo.toml to remove mupdf and use a different pdf renderer (advanced).

Building

- Frontend: npm ci && npm run build
- Backend (tauri): cd src-tauri && cargo build --release
- Full (via mise): mise run build

Troubleshooting

- If you see "No rule to make target '/Users/frobinson/Documents/Obsidian'" or similar during mupdf build, it's caused by unquoted paths in MuPDF's Makefile. Use the symlink workaround above.
- Ensure Xcode CLT and required system packages are installed.

If build still fails, open an issue with logs using the bug template.
