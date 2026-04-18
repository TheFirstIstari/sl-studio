<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { open } from '@tauri-apps/plugin-dialog';
	import { listen } from '@tauri-apps/api/event';
	import { onMount, onDestroy } from 'svelte';

	// Types
	interface Config {
		project: { name: string; evidence_root: string; registry_db: string; intelligence_db: string };
		model: { id: string; local_path: string; context_length: number };
		hardware: {
			gpu_backend: string;
			gpu_memory_fraction: number;
			cpu_workers: number;
			ocr_provider: string;
			whisper_size: string;
		};
		processing: { batch_size: number; max_image_resolution: number };
	}

	interface RegistryFile {
		path: string;
		fingerprint: string;
	}

	interface RegistryProgress {
		total: number;
		processed: number;
		current: number;
		current_file: string;
		phase: string;
	}

	interface ExtractionProgress {
		phase: string;
		current_file: string;
		processed: number;
		total: number;
		success_count: number;
		error_count: number;
	}

	interface AnalysisProgress {
		phase: string;
		current_file: string;
		processed: number;
		total: number;
	}

	interface ExtractionResult {
		fingerprint: string;
		path: string;
		success: boolean;
		char_count: number;
		error: string | null;
	}

	interface ExtractionStats {
		total_files: number;
		total_characters: number;
		average_characters: number;
		average_quality: number;
		partial_count: number;
		files_by_type: Record<string, number>;
	}

	interface WorkflowState {
		files_scanned: number;
		files_extracted: number;
		files_analyzed: number;
		last_scan_time: string | null;
		last_extraction_time: string | null;
		last_analysis_time: string | null;
		current_stage: string;
	}

	// Utility functions
	function formatNumber(n: number): string {
		if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
		if (n >= 1000) return (n / 1000).toFixed(1) + 'K';
		return n.toFixed(0);
	}

	function formatPercent(n: number): string {
		return (n * 100).toFixed(1) + '%';
	}

	function getQualityColor(quality: number): string {
		if (quality >= 0.8) return 'var(--color-success)';
		if (quality >= 0.5) return 'var(--color-warning)';
		return 'var(--color-error)';
	}

	// State
	let config = $state<Config | null>(null);
	let extractionStats = $state<ExtractionStats | null>(null);
	let workflowState = $state<WorkflowState | null>(null);
	let modelLoaded = $state(false);
	let scanning = $state(false);
	let extracting = $state(false);
	let analyzing = $state(false);

	let registryProgress = $state<RegistryProgress>({
		total: 0,
		processed: 0,
		current: 0,
		current_file: '',
		phase: ''
	});
	let extractionProgress = $state<ExtractionProgress>({
		phase: '',
		current_file: '',
		processed: 0,
		total: 0,
		success_count: 0,
		error_count: 0
	});
	let analysisProgress = $state<AnalysisProgress>({
		phase: '',
		current_file: '',
		processed: 0,
		total: 0
	});

	let unlistenProgress: (() => void) | null = null;
	let unlistenComplete: (() => void) | null = null;
	let unlistenExtraction: (() => void) | null = null;

	// Load stats
	async function loadExtractionStats() {
		try {
			extractionStats = await invoke<ExtractionStats>('get_extraction_statistics');
		} catch (e) {
			console.error('Failed to load extraction stats:', e);
		}
	}

	// Initialize
	onMount(async () => {
		try {
			config = await invoke<Config>('load_config');
			modelLoaded = await invoke<boolean>('is_model_loaded');
			if (config) await invoke('init_project', { config });

			// Load workflow state
			try {
				workflowState = await invoke<WorkflowState>('get_workflow_state');
				if (workflowState?.files_scanned > 0) {
					registryProgress.phase = 'complete';
					registryProgress.processed = workflowState.files_scanned;
				}
				if (workflowState?.files_extracted > 0) {
					extractionProgress.phase = 'complete';
					extractionProgress.success_count = workflowState.files_extracted;
				}
			} catch (e) {
				console.error('Failed to load workflow state:', e);
			}
		} catch (e) {
			console.error('Failed to load config:', e);
		}

		// Event listeners
		unlistenProgress = await listen<RegistryProgress>('registry_progress', (e) => {
			registryProgress = e.payload;
		});
		unlistenComplete = await listen<number>('registry_complete', (e) => {
			registryProgress.phase = 'complete';
			registryProgress.processed = e.payload;
			scanning = false;
		});
		unlistenExtraction = await listen<ExtractionProgress>('extraction_progress', (e) => {
			extractionProgress = e.payload;
		});
	});

	onDestroy(() => {
		if (unlistenProgress) unlistenProgress();
		if (unlistenComplete) unlistenComplete();
		if (unlistenExtraction) unlistenExtraction();
	});

	// Actions
	async function configureFolders() {
		try {
			const selected = await open({
				directory: true,
				multiple: false,
				title: 'Select Evidence Folder'
			});
			if (selected && config) {
				config.project.evidence_root = selected as string;
				await invoke('save_config', { config });
				config = await invoke<Config>('load_config');
			}
		} catch (e) {
			console.error('Error selecting folder:', e);
		}
	}

	async function startScan() {
		if (!config?.project.evidence_root) {
			registryProgress.phase = 'error';
			registryProgress.current_file = 'Please configure evidence folder first';
			return;
		}
		scanning = true;
		registryProgress = {
			phase: 'Initializing...',
			current: 0,
			processed: 0,
			total: 0,
			current_file: ''
		};
		const scanTimeout = setTimeout(() => {
			if (scanning) registryProgress.current_file = 'Scan still running...';
		}, 300000);
		try {
			const result = await invoke<number>('start_registry');
			registryProgress.phase = 'complete';
			registryProgress.processed = result;
			scanning = false;
			clearTimeout(scanTimeout);
		} catch (e) {
			registryProgress.phase = 'error';
			registryProgress.current_file = `Error: ${e}`;
			scanning = false;
		}
	}

	async function extractAllFiles() {
		extracting = true;
		extractionProgress = {
			phase: 'Loading...',
			current_file: '',
			processed: 0,
			total: 0,
			success_count: 0,
			error_count: 0
		};
		try {
			const queue = await invoke<RegistryFile[]>('get_extraction_queue', { limit: 10000 });
			if (queue.length === 0) {
				extractionProgress.phase = 'complete';
				extractionProgress.current_file = 'No files need extraction';
				extracting = false;
				return;
			}
			extractionProgress.total = queue.length;
			extractionProgress.phase = 'Extracting text...';
			const fingerprints = queue.map((f) => f.fingerprint);
			const results = await invoke<ExtractionResult[]>('extract_batch', {
				fingerprints,
				cpuWorkers: config?.hardware?.cpu_workers || 6
			});
			extractionProgress.success_count = results.filter((r) => r.success).length;
			extractionProgress.error_count = results.filter((r) => !r.success).length;
			extractionProgress.processed = results.length;
			extractionProgress.phase = 'complete';
			extractionProgress.current_file = `Extracted ${extractionProgress.success_count}/${results.length} files`;
		} catch (e) {
			extractionProgress.phase = 'error';
			extractionProgress.current_file = `Error: ${e}`;
		} finally {
			extracting = false;
			loadExtractionStats();
		}
	}

	async function analyzeExtractedFiles() {
		if (!config?.model.local_path) {
			analysisProgress.phase = 'error';
			analysisProgress.current_file = 'No model configured. Please download a model in Settings.';
			return;
		}
		analyzing = true;
		analysisProgress = { phase: 'Loading model...', current_file: '', processed: 0, total: 0 };
		try {
			if (!modelLoaded) {
				const models = await invoke<Array<{ path: string }>>('list_downloaded_models');
				const modelPath = models.length > 0 ? models[0].path : null;
				if (!modelPath)
					throw new Error('No model file found. Please download a model in Settings.');
				await invoke('init_reasoner', {
					modelPath,
					contextSize: config.model.context_length || 8192,
					gpuLayers: 32
				});
				modelLoaded = true;
			}
			const queue = await invoke<RegistryFile[]>('get_analysis_queue', { limit: 10 });
			if (queue.length === 0) {
				analysisProgress.phase = 'complete';
				analysisProgress.current_file = 'No files need analysis';
				analyzing = false;
				return;
			}
			analysisProgress.total = queue.length;
			analysisProgress.phase = 'Analyzing files...';
			const fingerprints = queue.map((f) => f.fingerprint);
			await invoke('analyze_batch', { fingerprints });
			analysisProgress.processed = queue.length;
			analysisProgress.phase = 'complete';
			analysisProgress.current_file = `Analyzed ${queue.length} files`;
		} catch (e) {
			analysisProgress.phase = 'error';
			analysisProgress.current_file = `Error: ${e}`;
		} finally {
			analyzing = false;
		}
	}

	async function stopExtraction() {
		await invoke('set_cancel_flag', { cancel: true });
		extractionProgress.current_file = 'Cancelling...';
		extractionProgress.phase = 'Cancelling';
	}

	async function stopAnalysis() {
		await invoke('set_cancel_flag', { cancel: true });
		analysisProgress.current_file = 'Cancelling...';
	}
</script>

<div class="analysis-container">
	<!-- Header -->
	<header class="page-header">
		<h1>Analysis Pipeline</h1>
		<p class="subtitle">Process evidence files through extraction and LLM analysis stages</p>
	</header>

	<!-- Workflow Status Bar -->
	{#if workflowState}
		<div class="workflow-bar">
			<div class="workflow-stage" class:done={workflowState.files_scanned > 0}>
				<div class="stage-indicator">{workflowState.files_scanned > 0 ? '✓' : '1'}</div>
				<div class="stage-info">
					<span class="stage-label">Scanned</span>
					<span class="stage-count">{workflowState.files_scanned} files</span>
				</div>
			</div>
			<div class="workflow-connector"></div>
			<div class="workflow-stage" class:done={workflowState.files_extracted > 0}>
				<div class="stage-indicator">{workflowState.files_extracted > 0 ? '✓' : '2'}</div>
				<div class="stage-info">
					<span class="stage-label">Extracted</span>
					<span class="stage-count">{workflowState.files_extracted} files</span>
				</div>
			</div>
			<div class="workflow-connector"></div>
			<div class="workflow-stage" class:done={workflowState.files_analyzed > 0}>
				<div class="stage-indicator">{workflowState.files_analyzed > 0 ? '✓' : '3'}</div>
				<div class="stage-info">
					<span class="stage-label">Analyzed</span>
					<span class="stage-count">{workflowState.files_analyzed} facts</span>
				</div>
			</div>
			<div class="workflow-spacer"></div>
			{#if extractionStats}
				<div class="quick-stats">
					<span class="quick-stat">
						<span class="qs-value">{extractionStats.total_files}</span>
						<span class="qs-label">extracted</span>
					</span>
					<span class="quick-stat">
						<span class="qs-value">{formatNumber(extractionStats.total_characters)}</span>
						<span class="qs-label">chars</span>
					</span>
				</div>
			{/if}
		</div>
	{/if}

	<!-- Main Grid -->
	<div class="analysis-grid">
		<!-- Stage 1: Scanner -->
		<section class="panel scanner-panel">
			<div class="panel-header">
				<span class="panel-number">01</span>
				<h2>Registry Scanner</h2>
			</div>
			<p class="panel-description">
				Scan evidence folder and create SHA-256 fingerprints. Duplicates are auto-skipped.
			</p>

			<div class="progress-display">
				{#if scanning}
					<div class="progress-track">
						<div
							class="progress-fill"
							style="width: {registryProgress.total > 0
								? (registryProgress.processed / registryProgress.total) * 100
								: 50}%"
						></div>
					</div>
					<div class="progress-label">{registryProgress.phase}</div>
					<div class="progress-detail">
						{registryProgress.processed}/{registryProgress.total || '...'} files
					</div>
				{:else if registryProgress.phase === 'complete'}
					<div class="status-badge success">
						<span class="badge-icon">✓</span>
						<span class="badge-text">{registryProgress.processed} files scanned</span>
					</div>
				{:else if registryProgress.phase === 'error'}
					<div class="status-badge error">{registryProgress.current_file}</div>
				{:else}
					<div class="status-badge idle">Ready to scan</div>
				{/if}
			</div>

			<div class="panel-actions">
				<button
					class="btn btn-primary"
					onclick={startScan}
					disabled={scanning || extracting || analyzing}
				>
					{scanning ? 'Scanning...' : 'Start Scan'}
				</button>
			</div>
		</section>

		<!-- Stage 2: Extraction -->
		<section class="panel extraction-panel">
			<div class="panel-header">
				<span class="panel-number">02</span>
				<h2>Text Extraction</h2>
			</div>
			<p class="panel-description">
				Extract text from PDFs, images, and audio using CPU parallelism.
			</p>

			<div class="progress-display">
				{#if extracting}
					<div class="progress-track">
						<div class="progress-fill indeterminate"></div>
					</div>
					<div class="progress-label">{extractionProgress.phase}</div>
					<div class="progress-detail">
						Processed: {extractionProgress.processed}/{extractionProgress.total}
					</div>
				{:else if extractionProgress.phase === 'complete'}
					<div class="status-badge success">
						<span class="badge-icon">✓</span>
						<span class="badge-text">{extractionProgress.success_count} extracted</span>
						{#if extractionProgress.error_count > 0}
							<span class="badge-error">• {extractionProgress.error_count} failed</span>
						{/if}
					</div>
				{:else if extractionProgress.phase === 'error'}
					<div class="status-badge error">{extractionProgress.current_file}</div>
				{:else}
					<div class="status-badge idle">Ready to extract</div>
				{/if}
			</div>

			<div class="panel-actions">
				{#if extracting}
					<button class="btn btn-danger" onclick={stopExtraction}>Stop</button>
				{:else}
					<button
						class="btn btn-primary"
						onclick={extractAllFiles}
						disabled={scanning || extracting || analyzing}
					>
						Extract All
					</button>
				{/if}
			</div>
		</section>

		<!-- Stage 3: Analysis -->
		<section class="panel analysis-panel">
			<div class="panel-header">
				<span class="panel-number">03</span>
				<h2>LLM Analysis</h2>
			</div>
			<p class="panel-description">Extract structured facts from text using local LLM inference.</p>

			<div class="model-status">
				<div class="model-badge" class:loaded={modelLoaded}>
					{modelLoaded ? 'Model Loaded' : config?.model.local_path ? 'Model Ready' : 'No Model'}
				</div>
			</div>

			<div class="progress-display">
				{#if analyzing}
					<div class="progress-track">
						<div class="progress-fill indeterminate"></div>
					</div>
					<div class="progress-label">{analysisProgress.phase}</div>
					<div class="progress-detail">{analysisProgress.processed}/{analysisProgress.total}</div>
				{:else if analysisProgress.phase === 'complete'}
					<div class="status-badge success">
						<span class="badge-icon">✓</span>
						<span class="badge-text">{analysisProgress.current_file}</span>
					</div>
				{:else if analysisProgress.phase === 'error'}
					<div class="status-badge error">{analysisProgress.current_file}</div>
				{:else}
					<div class="status-badge idle">Run extraction first</div>
				{/if}
			</div>

			<div class="panel-actions">
				{#if analyzing}
					<button class="btn btn-danger" onclick={stopAnalysis}>Stop</button>
				{:else}
					<button
						class="btn btn-primary"
						onclick={analyzeExtractedFiles}
						disabled={scanning || extracting || analyzing || !config?.model.local_path}
					>
						Analyze Files
					</button>
				{/if}
			</div>
		</section>

		<!-- Extraction Statistics -->
		<section class="panel stats-panel">
			<div class="panel-header">
				<h2>Extraction Statistics</h2>
				<button class="btn-icon" onclick={loadExtractionStats} title="Refresh">↻</button>
			</div>

			{#if extractionStats}
				<div class="stats-grid">
					<div class="stat-card">
						<span class="stat-value">{extractionStats.total_files}</span>
						<span class="stat-label">Files</span>
					</div>
					<div class="stat-card">
						<span class="stat-value">{formatPercent(extractionStats.average_quality)}</span>
						<span class="stat-label">Quality</span>
						<div class="quality-bar" style="--quality: {extractionStats.average_quality}"></div>
					</div>
					<div class="stat-card">
						<span class="stat-value">{formatNumber(extractionStats.total_characters)}</span>
						<span class="stat-label">Characters</span>
					</div>
					<div class="stat-card">
						<span class="stat-value">{formatNumber(extractionStats.average_characters)}</span>
						<span class="stat-label">Avg/File</span>
					</div>
					{#if extractionStats.partial_count > 0}
						<div class="stat-card warning">
							<span class="stat-value">{extractionStats.partial_count}</span>
							<span class="stat-label">Partial</span>
						</div>
					{/if}
				</div>
				{#if extractionStats.files_by_type && Object.keys(extractionStats.files_by_type).length > 0}
					<div class="file-types">
						<span class="ft-label">By type:</span>
						<div class="ft-items">
							{#each Object.entries(extractionStats.files_by_type) as [type, count]}
								<span class="ft-badge">{type} {count}</span>
							{/each}
						</div>
					</div>
				{/if}
			{:else}
				<div class="empty-stats">Run extraction to see statistics</div>
			{/if}
		</section>
	</div>
</div>

<style>
	/* CSS Variables - Design System */
	.analysis-container {
		--color-primary: #e94560;
		--color-primary-hover: #d13650;
		--color-bg-main: #1a1a2e;
		--color-bg-panel: #16213e;
		--color-bg-card: #1e1e2e;
		--color-border: #0f3460;
		--color-text: #eaeaea;
		--color-text-muted: #9ca3af;
		--color-success: #4ade80;
		--color-warning: #f59e0b;
		--color-error: #ef4444;
		--radius-sm: 4px;
		--radius-md: 8px;
		--radius-lg: 12px;
	}

	.analysis-container {
		max-width: 1400px;
		padding: 1.5rem;
	}

	/* Header */
	.page-header {
		margin-bottom: 1.5rem;
	}

	.page-header h1 {
		font-size: 1.75rem;
		font-weight: 600;
		color: var(--color-text);
		margin-bottom: 0.25rem;
	}

	.subtitle {
		font-size: 0.875rem;
		color: var(--color-text-muted);
	}

	/* Workflow Bar */
	.workflow-bar {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 1rem 1.25rem;
		background: var(--color-bg-panel);
		border-radius: var(--radius-lg);
		border: 1px solid var(--color-border);
		margin-bottom: 1.5rem;
	}

	.workflow-stage {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.5rem 1rem;
		border-radius: var(--radius-md);
		background: var(--color-bg-card);
		opacity: 0.6;
		transition: all 0.3s;
	}

	.workflow-stage.done {
		opacity: 1;
		background: rgba(74, 222, 128, 0.15);
	}

	.stage-indicator {
		width: 28px;
		height: 28px;
		border-radius: 50%;
		background: var(--color-border);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--color-text-muted);
	}

	.workflow-stage.done .stage-indicator {
		background: var(--color-success);
		color: var(--color-bg-main);
	}

	.stage-info {
		display: flex;
		flex-direction: column;
	}

	.stage-label {
		font-size: 0.75rem;
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.stage-count {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--color-text);
	}

	.workflow-connector {
		width: 24px;
		height: 2px;
		background: var(--color-border);
	}

	.workflow-spacer {
		flex: 1;
	}

	.quick-stats {
		display: flex;
		gap: 1.5rem;
	}

	.quick-stat {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
	}

	.qs-value {
		font-size: 1.25rem;
		font-weight: 700;
		color: var(--color-success);
	}

	.qs-label {
		font-size: 0.625rem;
		text-transform: uppercase;
		color: var(--color-text-muted);
	}

	/* Main Grid */
	.analysis-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 1rem;
	}

	.panel {
		background: var(--color-bg-panel);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		padding: 1.25rem;
	}

	.stats-panel {
		grid-column: span 3;
	}

	.panel-header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.75rem;
	}

	.panel-number {
		font-size: 0.625rem;
		font-weight: 700;
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.1em;
	}

	.panel h2 {
		font-size: 1rem;
		font-weight: 600;
		color: var(--color-text);
		margin: 0;
	}

	.panel-description {
		font-size: 0.8125rem;
		color: var(--color-text-muted);
		line-height: 1.5;
		margin-bottom: 1rem;
	}

	/* Progress Display */
	.progress-display {
		margin: 1rem 0;
		min-height: 60px;
	}

	.progress-track {
		height: 6px;
		background: var(--color-bg-card);
		border-radius: 3px;
		overflow: hidden;
		margin-bottom: 0.5rem;
	}

	.progress-fill {
		height: 100%;
		background: var(--color-primary);
		transition: width 0.3s ease;
	}

	.progress-fill.indeterminate {
		width: 30%;
		animation: indeterminate 1.5s infinite linear;
	}

	@keyframes indeterminate {
		0% {
			transform: translateX(-100%);
		}
		100% {
			transform: translateX(400%);
		}
	}

	.progress-label {
		font-size: 0.8125rem;
		color: var(--color-text);
	}

	.progress-detail {
		font-size: 0.75rem;
		color: var(--color-text-muted);
	}

	/* Status Badges */
	.status-badge {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 0.75rem;
		border-radius: var(--radius-md);
		font-size: 0.8125rem;
	}

	.status-badge.success {
		background: rgba(74, 222, 128, 0.15);
		color: var(--color-success);
	}

	.status-badge.error {
		background: rgba(239, 68, 68, 0.15);
		color: var(--color-error);
	}

	.status-badge.idle {
		background: var(--color-bg-card);
		color: var(--color-text-muted);
	}

	.badge-icon {
		font-weight: 700;
	}

	.badge-error {
		opacity: 0.7;
	}

	/* Model Status */
	.model-status {
		margin-bottom: 1rem;
	}

	.model-badge {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		background: var(--color-bg-card);
		border-radius: var(--radius-sm);
		font-size: 0.75rem;
		color: var(--color-text-muted);
	}

	.model-badge.loaded {
		background: rgba(74, 222, 128, 0.15);
		color: var(--color-success);
	}

	/* Buttons */
	.panel-actions {
		margin-top: auto;
	}

	.btn {
		width: 100%;
		padding: 0.75rem 1rem;
		border: none;
		border-radius: var(--radius-md);
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-primary {
		background: var(--color-primary);
		color: white;
	}

	.btn-primary:hover:not(:disabled) {
		background: var(--color-primary-hover);
	}

	.btn-danger {
		background: var(--color-error);
		color: white;
	}

	.btn-danger:hover:not(:disabled) {
		background: #dc2626;
	}

	.btn-icon {
		background: none;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
		padding: 0.25rem;
		font-size: 1rem;
	}

	/* Stats Grid */
	.stats-grid {
		display: flex;
		gap: 0.75rem;
		flex-wrap: wrap;
	}

	.stat-card {
		flex: 1;
		min-width: 100px;
		padding: 0.875rem 1rem;
		background: var(--color-bg-card);
		border-radius: var(--radius-md);
		text-align: center;
	}

	.stat-card.warning {
		border: 1px solid var(--color-warning);
	}

	.stat-value {
		display: block;
		font-size: 1.375rem;
		font-weight: 700;
		color: var(--color-text);
	}

	.stat-label {
		display: block;
		font-size: 0.6875rem;
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-top: 0.25rem;
	}

	.quality-bar {
		height: 3px;
		background: var(--color-border);
		border-radius: 2px;
		margin-top: 0.5rem;
		overflow: hidden;
	}

	.quality-bar::before {
		content: '';
		display: block;
		height: 100%;
		width: calc(var(--quality, 0) * 100%);
		background: var(--color-success);
	}

	/* File Types */
	.file-types {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid var(--color-border);
	}

	.ft-label {
		font-size: 0.75rem;
		color: var(--color-text-muted);
		display: block;
		margin-bottom: 0.5rem;
	}

	.ft-items {
		display: flex;
		gap: 0.375rem;
		flex-wrap: wrap;
	}

	.ft-badge {
		padding: 0.25rem 0.5rem;
		background: var(--color-border);
		border-radius: var(--radius-sm);
		font-size: 0.6875rem;
		color: var(--color-text);
	}

	.empty-stats {
		text-align: center;
		padding: 2rem;
		color: var(--color-text-muted);
		font-size: 0.875rem;
	}
</style>
