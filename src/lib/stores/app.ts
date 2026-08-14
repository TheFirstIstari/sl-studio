// Shared application state stores
// These are initialized once in layout and shared across all pages

import { writable, derived, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// Types matching Rust backend (src-tauri/src/lib.rs::AppConfig).
// This mirrors the nested shape that `load_config` actually returns.
export interface ProjectConfig {
	name: string;
	evidence_root: string;
	registry_db: string;
	intelligence_db: string;
}

export interface ModelConfig {
	source: 'huggingface' | 'local';
	id: string;
	mlx_model_name: string;
	dtype: string;
	context_length: number;
	downloaded: boolean;
}

export interface HardwareConfig {
	gpu_backend: string;
	gpu_memory_fraction: number;
	cpu_workers: number;
	auto_scale_workers: boolean;
	batch_size: number;
	auto_scale_batch: boolean;
	ocr_provider: string;
	whisper_size: string;
	whisper_model_path: string | null;
}

export interface ProcessingConfig {
	batch_size: number;
	max_image_resolution: number;
}

export interface AppConfig {
	version: string;
	project: ProjectConfig;
	model: ModelConfig;
	hardware: HardwareConfig;
	processing: ProcessingConfig;
}

export interface HardwareStatus {
	cpu_cores: number;
	total_memory: number;
	available_memory: number;
	gpu_backend: string;
	gpu_name: string;
	gpu_memory: number;
}

export interface HardwareInfo {
	recommended_context: number;
	recommended_batch_size: number;
	worker_count: number;
	backend: string;
}

export interface WorkflowState {
	files_scanned: number;
	files_extracted: number;
	files_analyzed: number;
	current_stage: string;
	is_scanning: boolean;
	is_extracting: boolean;
	is_analyzing: boolean;
	scan_progress: number;
	extract_progress: number;
	analyze_progress: number;
	current_file: string;
	processed_count: number;
	total_count: number;
}

export interface ProjectStats {
	total_files: number;
	files_scanned: number;
	files_extracted: number;
	files_analyzed: number;
	total_facts: number;
	total_entities: number;
	registry_count: number;
	intelligence_count: number;
	total_characters: number;
	average_characters: number;
	average_quality: number;
	partial_count: number;
	files_by_type: Record<string, number>;
	files_scanned_at: string | null;
	files_extracted_at: string | null;
	files_analyzed_at: string | null;
}

export interface ProcessingProgress {
	total: number;
	processed: number;
	current_file: string;
	phase: string;
	success_count: number;
	error_count: number;
}

// Create writable stores
export const config = writable<AppConfig | null>(null);
export const hardware = writable<HardwareStatus | null>(null);
export const hardwareInfo = writable<HardwareInfo | null>(null);
export const stats = writable<ProjectStats | null>(null);
export const workflow = writable<WorkflowState | null>(null);
export const modelLoaded = writable<boolean>(false);
export const projectInitialized = writable<boolean>(false);
export const isLoading = writable<boolean>(false);
export const error = writable<string | null>(null);

// Event listeners for background tasks
let unlistenExtraction: UnlistenFn | null = null;
let unlistenAnalysis: UnlistenFn | null = null;

// Initialize all stores - called once in layout
export async function initializeApp() {
	if (typeof window === 'undefined') return; // SSR guard

	isLoading.set(true);
	error.set(null);

	try {
		// Step 1: load config first — it gates project init.
		const cfg = await invoke<AppConfig>('load_config');
		config.set(cfg);

		// Step 2: initialize the project if we have an evidence root.
		// Done before stats / workflow because both depend on it.
		const hasProject = !!cfg?.project?.evidence_root;
		if (hasProject) {
			await invoke('init_project', { config: cfg });
			projectInitialized.set(true);
		}

		// Step 3: fan out the remaining independent queries in parallel.
		// Each is a separate Tauri IPC round-trip; running them
		// concurrently cuts startup latency roughly to the slowest one.
		const [hw, hwInfo, projectStats, loaded, wf] = await Promise.all([
			invoke<HardwareStatus>('detect_hardware'),
			invoke<HardwareInfo>('get_recommended_settings'),
			invoke<ProjectStats>('get_stats'),
			invoke<boolean>('is_model_loaded'),
			hasProject ? invoke<WorkflowState>('get_workflow_state') : Promise.resolve(null)
		]);
		hardware.set(hw);
		hardwareInfo.set(hwInfo);
		stats.set(projectStats);
		modelLoaded.set(loaded);
		if (wf) workflow.set(wf);

		// Step 4: setup event listeners for background tasks.
		await setupEventListeners();
	} catch (e) {
		console.error('Failed to initialize app:', e);
		error.set(String(e));
	} finally {
		isLoading.set(false);
	}
}

// Setup event listeners for background processing
export async function setupEventListeners() {
	if (typeof window === 'undefined') return;

	// Cleanup existing listeners
	await cleanupEventListeners();

	try {
		// Listen for extraction progress
		unlistenExtraction = await listen<ProcessingProgress>('extraction_progress', (event) => {
			const prog = event.payload;
			// Update workflow with progress
			workflow.update((w) => {
				if (!w) return createDefaultWorkflow();
				return {
					...w,
					current_stage: prog.phase,
					current_file: prog.current_file,
					extract_progress: prog.total > 0 ? (prog.processed / prog.total) * 100 : 0
				};
			});

			// Update stats
			stats.update((s) => {
				if (!s) return null;
				return {
					...s,
					files_extracted_at: new Date().toISOString()
				};
			});
		});

		// Listen for analysis progress
		unlistenAnalysis = await listen<ProcessingProgress>('analysis_progress', (event) => {
			const prog = event.payload;
			workflow.update((w) => {
				if (!w) return createDefaultWorkflow();
				return {
					...w,
					current_stage: prog.phase,
					current_file: prog.current_file,
					analyze_progress: prog.total > 0 ? (prog.processed / prog.total) * 100 : 0
				};
			});
		});

		// Workflow state is polled every 10 s by the layout (see
		// refreshWorkflow) because the backend updates state atomically in
		// commands rather than broadcasting an event. Polling is sufficient
		// for a UI that only needs best-effort freshness.
	} catch (e) {
		console.error('Failed to setup event listeners:', e);
	}
}

// Cleanup event listeners
export async function cleanupEventListeners() {
	if (unlistenExtraction) {
		unlistenExtraction();
		unlistenExtraction = null;
	}
	if (unlistenAnalysis) {
		unlistenAnalysis();
		unlistenAnalysis = null;
	}
}

// Helper to create default workflow
function createDefaultWorkflow(): WorkflowState {
	return {
		files_scanned: 0,
		files_extracted: 0,
		files_analyzed: 0,
		current_stage: 'idle',
		is_scanning: false,
		is_extracting: false,
		is_analyzing: false,
		scan_progress: 0,
		extract_progress: 0,
		analyze_progress: 0,
		current_file: '',
		processed_count: 0,
		total_count: 0
	};
}

// Refresh stats from backend
export async function refreshStats() {
	try {
		const projectStats = await invoke<ProjectStats>('get_stats');
		stats.set(projectStats);
	} catch (e) {
		console.error('Failed to refresh stats:', e);
	}
}

// Refresh workflow state
export async function refreshWorkflow() {
	try {
		const wf = await invoke<WorkflowState>('get_workflow_state');
		workflow.set(wf);
	} catch (e) {
		console.error('Failed to refresh workflow:', e);
	}
}

// Derived stores for convenience
export const isProcessing: Readable<boolean> = derived(
	workflow,
	($workflow) =>
		$workflow?.is_scanning === true ||
		$workflow?.is_extracting === true ||
		$workflow?.is_analyzing === true
);

export const hasProject: Readable<boolean> = derived(
	config,
	($config) => !!$config?.project?.evidence_root
);

// ---------------------------------------------------------------------------
// Theme store — persisted in localStorage, applied to document.documentElement
// ---------------------------------------------------------------------------
type Theme = 'dark' | 'light';

const THEME_KEY = 'sl-studio-theme';

function getInitialTheme(): Theme {
	if (typeof window === 'undefined') return 'dark';
	const stored = localStorage.getItem(THEME_KEY) as Theme | null;
	if (stored === 'light' || stored === 'dark') return stored;
	return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
}

function applyTheme(t: Theme) {
	if (typeof document !== 'undefined') {
		document.documentElement.dataset.theme = t;
	}
}

export const theme = writable<Theme>('dark');

export function initTheme() {
	const initial = getInitialTheme();
	theme.set(initial);
	applyTheme(initial);
	theme.subscribe((t) => {
		applyTheme(t);
		if (typeof window !== 'undefined') {
			localStorage.setItem(THEME_KEY, t);
		}
	});
}

export function toggleTheme() {
	theme.update((t) => (t === 'dark' ? 'light' : 'dark'));
}
