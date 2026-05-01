<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { onMount, onDestroy } from 'svelte';

	// Use shared stores for config, hardware, workflow, and model state
	import {
		config,
		hardware,
		hardwareInfo,
		modelLoaded,
		workflow,
		refreshWorkflow,
		refreshStats
	} from '$lib/stores/app';
	import { PageHeader } from '$lib/components';

	// Types
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

	// Utility functions
	function formatNumber(n: number): string {
		if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
		if (n >= 1000) return (n / 1000).toFixed(1) + 'K';
		return n.toFixed(0);
	}

	function formatPercent(n: number): string {
		return (n * 100).toFixed(1) + '%';
	}

	// Local state for progress tracking (not in stores)
	let extractionStats = $state<ExtractionStats | null>(null);
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
	let unlistenAnalysis: (() => void) | null = null;

	// Load stats
	async function loadExtractionStats() {
		try {
			extractionStats = await invoke<ExtractionStats>('get_extraction_statistics');
		} catch (e) {
			console.error('Failed to load extraction stats:', e);
		}
	}

	// Initialize - use stores for config, modelLoaded, workflow
	onMount(async () => {
		// Use stores for config, modelLoaded - already initialized in +layout.svelte
		// The workflow store is also already populated

		// Sync workflow state to local progress for display
		const wf = $workflow;
		if (wf && wf.files_scanned > 0) {
			registryProgress.phase = 'complete';
			registryProgress.processed = wf.files_scanned;
		}
		if (wf && wf.files_extracted > 0) {
			extractionProgress.phase = 'complete';
			extractionProgress.success_count = wf.files_extracted;
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
		unlistenAnalysis = await listen<AnalysisProgress>('analysis_progress', (e) => {
			analysisProgress = e.payload;
		});
	});

	onDestroy(() => {
		if (unlistenProgress) unlistenProgress();
		if (unlistenComplete) unlistenComplete();
		if (unlistenExtraction) unlistenExtraction();
		if (unlistenAnalysis) unlistenAnalysis();
	});

	// Actions

	// Busy state is sourced from the backend workflow via the global
	// store. The backend BusyGuard is the authoritative mutex — it sets
	// is_scanning / is_extracting / is_analyzing on entry and clears on
	// drop (including on error or panic). We only use local booleans
	// for the duration of each await so we can disable our own button
	// before the next poll tick.
	const busy = $derived(
		!!($workflow?.is_scanning || $workflow?.is_extracting || $workflow?.is_analyzing) ||
			scanning ||
			extracting ||
			analyzing
	);

	async function startScan() {
		if (!$config?.project?.evidence_root) {
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
		try {
			const result = await invoke<number>('start_registry');
			registryProgress.phase = 'complete';
			registryProgress.processed = result;
		} catch (e) {
			registryProgress.phase = 'error';
			registryProgress.current_file = `Error: ${e}`;
		} finally {
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
				return;
			}
			extractionProgress.total = queue.length;
			extractionProgress.phase = 'Extracting text...';
			const fingerprints = queue.map((f) => f.fingerprint);
			const results = await invoke<ExtractionResult[]>('extract_batch', {
				fingerprints,
				cpuWorkers: $config?.hardware?.cpu_workers || 6
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
		if (!$config?.model?.local_path) {
			analysisProgress.phase = 'error';
			analysisProgress.current_file = 'No model configured. Please download a model in Settings.';
			return;
		}
		analyzing = true;
		analysisProgress = { phase: 'Loading model...', current_file: '', processed: 0, total: 0 };
		try {
			if (!$modelLoaded) {
				const models = await invoke<Array<{ path: string }>>('list_downloaded_models');
				const modelPath = $config?.model?.local_path || (models.length > 0 ? models[0].path : null);
				if (!modelPath)
					throw new Error('No model file found. Please download a model in Settings.');

				// Validate model can be loaded before trying
				try {
					await invoke('validate_model', { modelPath });
				} catch (e) {
					modelLoaded.set(false);
					throw new Error(
						'Model not supported: ' + e + '. Please select a different model in Settings.'
					);
				}

				await invoke('init_reasoner', {
					modelPath,
					contextSize: $config?.model?.context_length || 8192,
					gpuLayers: 32
				});
				modelLoaded.set(true);
			}
			const queue = await invoke<RegistryFile[]>('get_analysis_queue', { limit: 10 });
			if (queue.length === 0) {
				analysisProgress.phase = 'complete';
				analysisProgress.current_file = 'No files need analysis';
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

<div class="analysis-container page">
	<PageHeader
		title="Analysis Pipeline"
		subtitle="Process evidence files through extraction and LLM analysis stages"
	/>

	<!-- Workflow Status Bar -->
	{#if $workflow}
		<div class="workflow-bar">
			<div class="workflow-stage" class:done={$workflow.files_scanned > 0}>
				<div class="stage-indicator">
					{#if $workflow.files_scanned > 0}
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg>
					{:else}1{/if}
				</div>
				<div class="stage-info">
					<span class="stage-label">Scanned</span>
					<span class="stage-count">{$workflow.files_scanned} files</span>
				</div>
			</div>
			<div class="workflow-connector"></div>
			<div class="workflow-stage" class:done={$workflow.files_extracted > 0}>
				<div class="stage-indicator">
					{#if $workflow.files_extracted > 0}
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg>
					{:else}2{/if}
				</div>
				<div class="stage-info">
					<span class="stage-label">Extracted</span>
					<span class="stage-count">{$workflow.files_extracted} files</span>
				</div>
			</div>
			<div class="workflow-connector"></div>
			<div class="workflow-stage" class:done={$workflow.files_analyzed > 0}>
				<div class="stage-indicator">
					{#if $workflow.files_analyzed > 0}
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg>
					{:else}3{/if}
				</div>
				<div class="stage-info">
					<span class="stage-label">Analyzed</span>
					<span class="stage-count">{$workflow.files_analyzed} facts</span>
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
						<span class="badge-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg></span>
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
					disabled={busy}
					title={busy && !scanning ? 'Another operation is in progress' : undefined}
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
						<span class="badge-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg></span>
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
						disabled={busy}
						title={busy && !extracting ? 'Another operation is in progress' : undefined}
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
				<div class="model-badge" class:loaded={$modelLoaded}>
					{$modelLoaded ? 'Model Loaded' : $config?.model?.local_path ? 'Model Ready' : 'No Model'}
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
						<span class="badge-icon"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg></span>
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
						disabled={busy || !$config?.model?.local_path}
						title={busy && !analyzing ? 'Another operation is in progress' : undefined}
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
	

	.analysis-container {
		max-width: 1400px;
		padding: 1.5rem;
	}

	/* Workflow Bar */
	.workflow-bar {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 1rem 1.25rem;
		background: var(--color-bg-card);
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

	.stage-indicator svg {
		width: 14px;
		height: 14px;
	}

	.workflow-stage.done .stage-indicator {
		background: var(--color-status-confirmed);
		color: var(--color-bg-input);
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
		color: var(--color-text-primary);
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
		color: var(--color-status-confirmed);
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
		background: var(--color-bg-card);
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
		color: var(--color-text-primary);
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
		background: var(--color-accent);
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
		color: var(--color-text-primary);
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
		color: var(--color-status-confirmed);
	}

	.status-badge.error {
		background: rgba(239, 68, 68, 0.15);
		color: var(--color-severity-high);
	}

	.status-badge.idle {
		background: var(--color-bg-card);
		color: var(--color-text-muted);
	}

	.badge-icon {
		display: flex;
		align-items: center;
	}

	.badge-icon svg {
		width: 14px;
		height: 14px;
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
		color: var(--color-status-confirmed);
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
		background: var(--color-accent);
		color: var(--color-text-inverse);
	}

	.btn-primary:hover:not(:disabled) {
		background: var(--color-accent-hover);
	}

	.btn-danger {
		background: var(--color-severity-high);
		color: var(--color-text-inverse);
	}

	.btn-danger:hover:not(:disabled) {
		background: var(--color-severity-high);
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
		border: 1px solid var(--color-severity-medium);
	}

	.stat-value {
		display: block;
		font-size: 1.375rem;
		font-weight: 700;
		color: var(--color-text-primary);
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
		background: var(--color-status-confirmed);
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
		color: var(--color-text-primary);
	}

	.empty-stats {
		text-align: center;
		padding: 2rem;
		color: var(--color-text-muted);
		font-size: 0.875rem;
	}
</style>
