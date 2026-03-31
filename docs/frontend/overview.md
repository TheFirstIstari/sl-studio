# Frontend Overview

## Architecture

The frontend is a **SvelteKit 5 Single Page Application (SPA)** served via Tauri's embedded webview. It communicates with the Rust backend through Tauri's IPC mechanism using `invoke` commands.

## Structure

```
src/
├── app.html                    # HTML shell
├── routes/
│   ├── +layout.ts              # Disables SSR for Tauri SPA mode
│   ├── +layout.svelte          # Root layout (header, sidebar, footer)
│   ├── +page.svelte            # Dashboard
│   ├── analysis/+page.svelte   # Analysis pipeline
│   ├── results/+page.svelte    # Results viewer
│   ├── timeline/+page.svelte   # Timeline visualization
│   ├── stats/+page.svelte      # Statistics
│   ├── network/+page.svelte    # Entity network graph
│   ├── maps/+page.svelte       # Geographic map
│   ├── anomalies/+page.svelte  # Anomaly detection
│   ├── compare/+page.svelte    # Project comparison
│   ├── export/+page.svelte     # Export
│   ├── backup/+page.svelte     # Backup & Restore
│   └── settings/+page.svelte   # Settings
└── lib/
    └── components/
        └── PerformanceMonitor.svelte
```

## Root Layout

`+layout.svelte` provides:

- **Header**: Logo, status indicator, system info
- **Sidebar**: Navigation to all 12 pages
- **Footer**: Console output area
- **Keyboard shortcuts**: G+letter navigation, ? for help modal
- **Dark theme**: #1a1a2e background optimized for forensic work

## Key Libraries

| Library         | Purpose                               |
| --------------- | ------------------------------------- |
| Chart.js 4      | Bar, doughnut, horizontal bar charts  |
| Cytoscape.js 3  | Interactive network graphs            |
| Leaflet.js 1    | Geographic maps with CARTO dark tiles |
| @tauri-apps/api | Tauri IPC communication               |

## State Management

- Component-level reactive state via Svelte stores
- Tauri `invoke()` calls for backend communication
- No global state manager - each page manages its own state

## Theme

Dark theme throughout:

- Background: #1a1a2e
- Text: Light colors for readability
- Severity colors: Red (Critical), Orange (High), Yellow (Medium), Green (Low)
- Optimized for extended forensic analysis sessions
