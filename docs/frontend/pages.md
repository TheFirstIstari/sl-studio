# Pages

## Dashboard (`/`)

Stats cards showing:

- Files registered
- Facts extracted
- CPU workers
- Hardware status grid
- Model status
- Quick action buttons

## Analysis (`/analysis`)

Project setup and processing pipeline:

- Project selection panel
- Registry scanner with SHA-256 fingerprinting progress
- Neural reasoner with model loading
- File-by-file analysis progress

## Results (`/results`)

Fact viewer with:

- Fact list with filtering
- Sorting by severity/date
- Bulk selection
- Undo/redo history
- Severity color-coding
- Detail panel

## Timeline (`/timeline`)

Chronological visualization:

- Events grouped by month
- Timeline view and list view toggle
- Severity-colored markers
- Detail panel

## Statistics (`/stats`)

Data analysis with Chart.js:

- Overview cards
- Bar charts for severity distribution
- Doughnut charts for category distribution
- Top entities
- Data table

## Network (`/network`)

Entity relationship graph:

- Cytoscape.js interactive graph
- Node types: Person, Organization, Location, Date
- Zoom controls
- Connected entity side panel
- Legend

## Maps (`/maps`)

Geographic visualization:

- Leaflet.js with CARTO dark basemap
- Severity-colored markers
- Locations side panel
- Coordinate detail panel

## Anomalies (`/anomalies`)

Outlier detection:

- Z-score based anomaly detection
- Severity/confidence/quality metrics
- Configurable threshold
- Deviation visualization

## Compare (`/compare`)

Project comparison:

- Compare two projects
- Entity overlap table
- Timeline correlation score
- Fact similarity percentage

## Export (`/export`)

Data export:

- Export facts/entities/timeline
- Formats: JSON, CSV, PDF
- Min weight filter
- Export history log

## Backup (`/backup`)

Backup and restore:

- Create ZIP backups (with optional evidence files)
- Restore from backup
- Warning dialogs

## Settings (`/settings`)

Configuration:

- Project config
- HuggingFace model download with progress bar
- Local model selection
- Hardware tuning (CPU workers, VRAM, batch size)
- Real-time system monitor
