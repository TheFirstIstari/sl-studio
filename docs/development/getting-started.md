# Getting Started

## Prerequisites

| Tool    | Version | Purpose              |
| ------- | ------- | -------------------- |
| Rust    | 1.75+   | Backend development  |
| Node.js | 22.x    | Frontend development |
| mise    | Latest  | Task runner          |

### Platform-Specific Requirements

| Platform | Requirements                                                                                |
| -------- | ------------------------------------------------------------------------------------------- |
| macOS    | Xcode Command Line Tools                                                                    |
| Windows  | Visual Studio Build Tools                                                                   |
| Linux    | `libwebkit2gtk-4.0-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`, `libgtk-3-dev` |

## Setup

### 1. Clone the Repository

```bash
git clone https://github.com/TheFirstIstari/sl-studio.git
cd sl-studio
```

### 2. Install Dependencies

```bash
# Install Node.js dependencies
npm install

# Rust dependencies are managed by mise
mise install
```

### 3. Install System Dependencies (Linux)

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.0-dev libappindicator3-dev librsvg2-dev patchelf libgtk-3-dev
```

Or use the provided script:

```bash
bash scripts/install_deps.sh
```

### 4. Run Development Server

```bash
mise run dev
```

This starts both the Vite dev server (port 1420) and the Tauri application.

## First Use Workflow

### 1. Configure Project

1. Open **Settings** page
2. Set **Evidence Root Folder** to your evidence directory (or network share)
3. Set **Registry DB** and **Intelligence DB** paths (can be left as defaults)
4. Set **Export Directory** for export reports

### 2. Download LLM Model

1. In **Settings**, go to Model section
2. Select a model from the dropdown (Mistral 7B or Llama 2 7B)
3. Click **Download Model** (~4GB, may take a while)
4. Or manually download and set "Model Path" to the GGUF file

### 3. Scan Evidence

1. Go to **Analysis** page
2. Click **Start Fingerprinting** to scan and fingerprint all files
3. Watch progress as files are hashed and registered

### 4. Run Analysis

1. Click **Start Analysis** to run the neural reasoner on files
2. The LLM will extract facts from each file
3. Progress shows extracted facts count

### 5. View Results

- **Results** - Browse and filter extracted facts
- **Timeline** - Chronological view of events
- **Statistics** - Charts and distributions
- **Network** - Entity relationship graphs
- **Anomalies** - Detect outliers

## Project Structure

```
sl-studio/
├── src-tauri/           # Rust backend
│   ├── src/             # Source code
│   ├── benches/         # Criterion benchmarks
│   └── Cargo.toml       # Rust dependencies
├── src/                 # SvelteKit frontend
│   ├── routes/          # Pages
│   └── lib/             # Shared components
├── e2e/                 # Playwright E2E tests
├── docs/                # Documentation
├── scripts/             # Shell scripts
├── mise.toml            # Task definitions
└── package.json         # Node.js dependencies
```

## First Run

1. Run `mise run dev` to start the application
2. The Tauri window should open automatically
3. Navigate to Settings to configure your project
4. Select an evidence directory to begin scanning

## Next Steps

- Read the [Development Workflow](workflow.md) guide
- Explore the [Backend Overview](../backend/overview.md)
- Review the [API Reference](../api/tauri-commands.md)
