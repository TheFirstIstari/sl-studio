// Shared application state stores
// These are initialized once in layout and shared across all pages

import { writable, derived, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// Types matching Rust backend
export interface AppConfig {
	project_name: string;
	evidence_root: string;
	registry_db: string;
	intelligence_db: string;
	worker_count: number;
	batch_size: number;
	auto_scale_workers: boolean;
	auto_scale_batch: boolean;
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
	recommended_gpu_layers: number;
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
let unlistenWorkflow: UnlistenFn | null = null;

// Initialize all stores - called once in layout
export async function initializeApp() {
	if (typeof window === 'undefined') return; // SSR guard

	isLoading.set(true);
	error.set(null);

	try {
		// Load config
		const cfg = await invoke<AppConfig>('load_config');
		config.set(cfg);

		// Detect hardware
		const hw = await invoke<HardwareStatus>('detect_hardware');
		hardware.set(hw);

		const hwInfo = await invoke<HardwareInfo>('get_recommended_settings');
		hardwareInfo.set(hwInfo);

		// Get project stats
		const projectStats = await invoke<ProjectStats>('get_stats');
		stats.set(projectStats);

		// Check model loaded
		const loaded = await invoke<boolean>('is_model_loaded');
		modelLoaded.set(loaded);

		// Initialize project if config exists
		if (cfg && cfg.evidence_root) {
			await invoke('init_project', { config: cfg });
			projectInitialized.set(true);

			// Get workflow state
			const wf = await invoke<WorkflowState>('get_workflow_state');
			workflow.set(wf);
		}

		// Setup event listeners for background tasks
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

		// Listen for workflow state changes
		unlistenWorkflow = await listen<WorkflowState>('workflow_state', (event) => {
			workflow.set(event.payload);
		});
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
	if (unlistenWorkflow) {
		unlistenWorkflow();
		unlistenWorkflow = null;
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
	($config) => $config?.evidence_root !== null && $config?.evidence_root !== ''
);
