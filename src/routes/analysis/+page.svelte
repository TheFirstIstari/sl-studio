<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { open } from '@tauri-apps/plugin-dialog';
	import { listen } from '@tauri-apps/api/event';
	import { onMount, onDestroy } from 'svelte';

	interface Config {
		project: {
			name: string;
			evidence_root: string;
			registry_db: string;
			intelligence_db: string;
		};
		model: {
			id: string;
			local_path: string;
			context_length: number;
		};
		hardware: {
			gpu_backend: string;
			gpu_memory_fraction: number;
			cpu_workers: number;
			ocr_provider: string;
			whisper_size: string;
		};
		processing: {
			batch_size: number;
			max_image_resolution: number;
		};
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

	function formatNumber(n: number): string {
		if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
		if (n >= 1000) return (n / 1000).toFixed(1) + 'K';
		return n.toFixed(0);
	}

	async function loadExtractionStats() {
		try {
			extractionStats = await invoke<ExtractionStats>('get_extraction_statistics');
		} catch (e) {
			console.error('Failed to load extraction stats:', e);
		}
	}

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

	onMount(async () => {
		try {
			config = await invoke<Config>('load_config');
			modelLoaded = await invoke<boolean>('is_model_loaded');

			// Initialize the project with config to set up database
			if (config) {
				await invoke('init_project', { config });
			}

			// Load workflow state from database to restore progress
			try {
				workflowState = await invoke<WorkflowState>('get_workflow_state');

				// Restore progress states based on DB state
				if (workflowState) {
					if (workflowState.files_scanned > 0) {
						registryProgress.phase = 'complete';
						registryProgress.processed = workflowState.files_scanned;
					}
					if (workflowState.files_extracted > 0) {
						extractionProgress.phase = 'complete';
						extractionProgress.success_count = workflowState.files_extracted;
					}
				}
			} catch (e) {
				console.error('Failed to load workflow state:', e);
			}
		} catch (e) {
			console.error('Failed to load config:', e);
		}

		unlistenProgress = await listen<RegistryProgress>('registry_progress', (event) => {
			registryProgress = event.payload;
		});

		unlistenComplete = await listen<number>('registry_complete', (event) => {
			registryProgress.phase = 'complete';
			registryProgress.processed = event.payload;
			scanning = false;
		});

		unlistenExtraction = await listen<ExtractionProgress>('extraction_progress', (event) => {
			extractionProgress = event.payload;
		});
	});

	onDestroy(() => {
		if (unlistenProgress) unlistenProgress();
		if (unlistenComplete) unlistenComplete();
		if (unlistenExtraction) unlistenExtraction();
	});

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

		const scanComplete = (processed: number) => {
			registryProgress.phase = 'complete';
			registryProgress.processed = processed;
			scanning = false;
			clearTimeout(scanTimeout);
		};

		const scanTimeout = setTimeout(() => {
			if (scanning) {
				console.warn('Scan timeout - backend still running');
				registryProgress.current_file = 'Scan still running...';
			}
		}, 300000);

		try {
			const result = await invoke<number>('start_registry');
			scanComplete(result);
		} catch (e) {
			console.error('Scan error:', e);
			registryProgress.phase = 'error';
			registryProgress.current_file = `Error: ${e}`;
			scanning = false;
		}
	}

	// NEW: Separate extraction function
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
			// Get files that need extraction
			const extractionQueue = await invoke<RegistryFile[]>('get_extraction_queue', {
				limit: 10000
			});

			if (extractionQueue.length === 0) {
				extractionProgress.phase = 'complete';
				extractionProgress.current_file = 'No files need extraction';
				extracting = false;
				return;
			}

			extractionProgress.total = extractionQueue.length;
			extractionProgress.phase = 'Extracting text...';

			const fingerprints = extractionQueue.map((f) => f.fingerprint);

			// Call the batch extraction
			const results = await invoke<ExtractionResult[]>('extract_batch', {
				fingerprints: fingerprints,
				cpuWorkers: config?.hardware?.cpu_workers || 6
			});

			// Update progress
			extractionProgress.success_count = results.filter((r) => r.success).length;
			extractionProgress.error_count = results.filter((r) => !r.success).length;
			extractionProgress.processed = results.length;
			extractionProgress.phase = 'complete';
			extractionProgress.current_file = `Extracted ${extractionProgress.success_count}/${results.length} files`;
		} catch (e) {
			console.error('Extraction error:', e);
			extractionProgress.phase = 'error';
			extractionProgress.current_file = `Error: ${e}`;
		} finally {
			extracting = false;
			loadExtractionStats();
		}
	}

	// NEW: Separate analysis function that uses pre-extracted text
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

				if (!modelPath) {
					throw new Error('No model file found. Please download a model in Settings.');
				}

				await invoke('init_reasoner', {
					modelPath: modelPath,
					contextSize: config.model.context_length || 8192,
					gpuLayers: 32 // Full GPU acceleration on M4
				});
				modelLoaded = true;
			}

			analysisProgress.phase = 'Getting analysis queue...';

			// Get files that have extracted text but haven't been analyzed
			const analysisQueue = await invoke<RegistryFile[]>('get_analysis_queue', { limit: 10 });

			if (analysisQueue.length === 0) {
				analysisProgress.phase = 'complete';
				analysisProgress.current_file = 'No files need analysis';
				analyzing = false;
				return;
			}

			analysisProgress.total = analysisQueue.length;
			analysisProgress.phase = 'Analyzing files...';

			const fingerprints = analysisQueue.map((f) => f.fingerprint);

			// Call batch analysis
			await invoke('analyze_batch', { fingerprints });

			analysisProgress.processed = analysisQueue.length;
			analysisProgress.phase = 'complete';
			analysisProgress.current_file = `Analyzed ${analysisQueue.length} files`;
		} catch (e) {
			console.error('Analysis error:', e);
			analysisProgress.phase = 'error';
			analysisProgress.current_file = `Error: ${e}`;
		} finally {
			analyzing = false;
		}
	}
</script>

<div class="analysis">
	<h1>Analysis</h1>

	<div class="analysis-grid">
		<section class="panel">
			<h2>Project Setup</h2>

			<div class="setup-info">
				<div class="info-row">
					<span class="info-label">Evidence Root:</span>
					<span class="info-value">{config?.project.evidence_root || 'Not configured'}</span>
				</div>
				<div class="info-row">
					<span class="info-label">Registry DB:</span>
					<span class="info-value"
						>{config?.project.registry_db ? 'Configured' : 'Not configured'}</span
					>
				</div>
				<div class="info-row">
					<span class="info-label">Intelligence DB:</span>
					<span class="info-value"
						>{config?.project.intelligence_db ? 'Configured' : 'Not configured'}</span
					>
				</div>
			</div>

			<button class="action-btn secondary" onclick={configureFolders}> Configure Folders </button>
		</section>

		<section class="panel">
			<h2>Registry Scanner</h2>
			<p class="description">
				Scan the evidence folder and create fingerprints of all files using SHA-256 hashing.
				Duplicate files are automatically skipped.
			</p>

			<div class="progress-section">
				{#if scanning}
					<div class="progress-bar">
						<div
							class="progress-fill"
							style="width: {registryProgress.total > 0
								? (registryProgress.processed / registryProgress.total) * 100
								: 50}%"
						></div>
					</div>
					<div class="progress-text">
						{registryProgress.phase} - {registryProgress.processed}/{registryProgress.total ||
							'...'}
					</div>
					{#if registryProgress.current_file}
						<div class="current-file">{registryProgress.current_file}</div>
					{/if}
				{:else if registryProgress.phase === 'complete'}
					<div class="idle-text">Scan complete - {registryProgress.processed} files processed</div>
				{:else if registryProgress.phase === 'error'}
					<div class="error-text">{registryProgress.current_file}</div>
				{:else}
					<div class="idle-text">Ready to scan</div>
				{/if}
			</div>

			<button
				class="action-btn primary"
				onclick={startScan}
				disabled={scanning || extracting || analyzing}
			>
				{scanning ? 'Scanning...' : 'Start Fingerprinting'}
			</button>
		</section>

		<!-- Quick Status Header -->
		{#if workflowState}
			<div class="workflow-header">
				<div
					class="workflow-step"
					class:active={workflowState.files_scanned > 0}
					class:complete={workflowState.files_scanned > 0}
				>
					<span class="step-icon">1</span>
					<span class="step-label">Scanned</span>
					<span class="step-count">{workflowState.files_scanned}</span>
				</div>
				<div class="workflow-arrow">→</div>
				<div
					class="workflow-step"
					class:active={workflowState.files_extracted > 0}
					class:complete={workflowState.files_extracted > 0}
				>
					<span class="step-icon">2</span>
					<span class="step-label">Extracted</span>
					<span class="step-count">{workflowState.files_extracted}</span>
				</div>
				<div class="workflow-arrow">→</div>
				<div
					class="workflow-step"
					class:active={workflowState.files_analyzed > 0}
					class:complete={workflowState.files_analyzed > 0}
				>
					<span class="step-icon">3</span>
					<span class="step-label">Analyzed</span>
					<span class="step-count">{workflowState.files_analyzed}</span>
				</div>
			</div>
		{/if}

		<!-- NEW: Stage 1: Extraction Panel -->
		<section class="panel">
			<h2>Stage 1: Text Extraction</h2>
			<p class="description">
				Extract text from all files using CPU parallelism. Run this first to cache extracted text.
				Files can be processed in parallel for maximum throughput.
			</p>

			<div class="progress-section">
				{#if extracting}
					<div class="progress-bar">
						<div class="progress-fill indeterminate"></div>
					</div>
					<div class="progress-text">{extractionProgress.phase}</div>
					{#if extractionProgress.current_file}
						<div class="current-file">{extractionProgress.current_file}</div>
					{/if}
					<div class="progress-stats">
						<span>Processed: {extractionProgress.processed}/{extractionProgress.total}</span>
					</div>
				{:else if extractionProgress.phase === 'complete'}
					<div class="idle-text">
						{#if extractionProgress.success_count > 0}
							✓ {extractionProgress.success_count} extracted
						{/if}
						{#if extractionProgress.error_count > 0}
							<br />✗ {extractionProgress.error_count} failed
						{/if}
					</div>
				{:else if extractionProgress.phase === 'error'}
					<div class="error-text">{extractionProgress.current_file}</div>
				{:else}
					<div class="idle-text">Ready to extract</div>
				{/if}
			</div>

			<button
				class="action-btn primary"
				onclick={extractAllFiles}
				disabled={scanning || extracting || analyzing}
			>
				{extracting ? 'Extracting...' : 'Extract All Files'}
			</button>
		</section>

		<!-- Extraction Statistics Panel -->
		<section class="panel stats-panel">
			<h2>Extraction Statistics</h2>
			{#if extractionStats}
				<div class="stats-grid">
					<div class="stat-item">
						<span class="stat-value">{extractionStats.total_files}</span>
						<span class="stat-label">Files Extracted</span>
					</div>
					<div class="stat-item">
						<span class="stat-value">{extractionStats.average_quality.toFixed(1)}%</span>
						<span class="stat-label">Avg Quality</span>
					</div>
					<div class="stat-item">
						<span class="stat-value">{formatNumber(extractionStats.average_characters)}</span>
						<span class="stat-label">Avg Chars</span>
					</div>
					<div class="stat-item">
						<span class="stat-value">{formatNumber(extractionStats.total_characters)}</span>
						<span class="stat-label">Total Characters</span>
					</div>
					<div class="stat-item" class:warning={extractionStats.partial_count > 0}>
						<span class="stat-value">{extractionStats.partial_count}</span>
						<span class="stat-label">Partial (Warnings)</span>
					</div>
				</div>
				{#if extractionStats.files_by_type && Object.keys(extractionStats.files_by_type).length > 0}
					<div class="file-type-dist">
						<span class="dist-label">By File Type:</span>
						<div class="dist-items">
							{#each Object.entries(extractionStats.files_by_type) as [type, count]}
								<span class="dist-item">{type}: {count}</span>
							{/each}
						</div>
					</div>
				{/if}
			{:else}
				<div class="idle-text">Run extraction to see statistics</div>
			{/if}
			<button class="action-btn secondary" onclick={loadExtractionStats} disabled={extracting}>
				Refresh Stats
			</button>
		</section>

		<!-- NEW: Stage 2: LLM Analysis Panel -->
		<section class="panel">
			<h2>Stage 2: LLM Analysis</h2>
			<p class="description">
				Run LLM inference on extracted text to extract structured facts. Uses GPU acceleration for
				maximum performance.
			</p>

			<div class="model-info">
				<div class="info-row">
					<span class="info-label">Model:</span>
					<span class="info-value" class:loaded={modelLoaded}>
						{modelLoaded ? 'Loaded' : config?.model.local_path ? 'Not loaded' : 'No model'}
					</span>
				</div>
				<div class="info-row">
					<span class="info-label">Model Path:</span>
					<span class="info-value path">{config?.model.local_path || 'Not set'}</span>
				</div>
				<div class="info-row">
					<span class="info-label">Context:</span>
					<span class="info-value">{config?.model.context_length || 8192} tokens</span>
				</div>
				<div class="info-row">
					<span class="info-label">GPU:</span>
					<span class="info-value">{config?.hardware?.gpu_backend || 'metal'}</span>
				</div>
			</div>

			<div class="progress-section">
				{#if analyzing}
					<div class="progress-bar">
						<div class="progress-fill indeterminate"></div>
					</div>
					<div class="progress-text">{analysisProgress.phase}</div>
					{#if analysisProgress.current_file}
						<div class="current-file">{analysisProgress.current_file}</div>
					{/if}
				{:else if analysisProgress.phase === 'complete'}
					<div class="idle-text">{analysisProgress.current_file}</div>
				{:else if analysisProgress.phase === 'error'}
					<div class="error-text">{analysisProgress.current_file}</div>
				{:else}
					<div class="idle-text">Ready to analyze (run extraction first)</div>
				{/if}
			</div>

			<button
				class="action-btn primary"
				onclick={analyzeExtractedFiles}
				disabled={scanning || extracting || analyzing || !config?.model.local_path}
			>
				{analyzing ? 'Analyzing...' : 'Analyze Extracted Files'}
			</button>
		</section>
	</div>
</div>

<style>
	.analysis {
		max-width: 1200px;
	}

	h1 {
		font-size: 1.75rem;
		margin-bottom: 1.5rem;
		color: #eaeaea;
	}

	h2 {
		font-size: 1.25rem;
		margin-bottom: 0.75rem;
		color: #e94560;
	}

	.analysis-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(350px, 1fr));
		gap: 1.5rem;
	}

	.panel {
		background-color: #16213e;
		padding: 1.5rem;
		border-radius: 8px;
		border: 1px solid #0f3460;
	}

	.description {
		color: #9ca3af;
		font-size: 0.875rem;
		margin-bottom: 1rem;
		line-height: 1.5;
	}

	.setup-info,
	.model-info {
		margin-bottom: 1rem;
	}

	.info-row {
		display: flex;
		justify-content: space-between;
		padding: 0.5rem 0;
		border-bottom: 1px solid #0f3460;
	}

	.info-label {
		color: #9ca3af;
		font-size: 0.875rem;
	}

	.info-value {
		color: #eaeaea;
		font-size: 0.875rem;
		font-weight: 500;
	}

	.info-value.loaded {
		color: #4ade80;
	}

	.info-value.path {
		max-width: 200px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.progress-section {
		margin: 1.5rem 0;
		min-height: 80px;
	}

	.progress-bar {
		height: 8px;
		background-color: #1a1a2e;
		border-radius: 4px;
		overflow: hidden;
		margin-bottom: 0.5rem;
	}

	.progress-fill {
		height: 100%;
		background-color: #e94560;
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

	.progress-text {
		font-size: 0.875rem;
		color: #9ca3af;
	}

	.progress-stats {
		font-size: 0.75rem;
		color: #6b7280;
		margin-top: 0.25rem;
	}

	.current-file {
		font-size: 0.75rem;
		color: #6b7280;
		margin-top: 0.25rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.idle-text {
		color: #4ade80;
		font-size: 0.875rem;
	}

	.error-text {
		color: #ef4444;
		font-size: 0.875rem;
	}

	.action-btn {
		width: 100%;
		padding: 0.875rem 1rem;
		border: none;
		border-radius: 6px;
		font-size: 1rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.action-btn.primary {
		background-color: #e94560;
		color: #ffffff;
	}

	.action-btn.primary:hover:not(:disabled) {
		background-color: #d13650;
	}

	.action-btn.secondary {
		background-color: #0f3460;
		color: #eaeaea;
	}

	.action-btn.secondary:hover:not(:disabled) {
		background-color: #1a4a7a;
	}

	.action-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.stats-panel {
		grid-column: 1 / -1;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(5, 1fr);
		gap: 1rem;
		margin-bottom: 1rem;
	}

	.stat-item {
		text-align: center;
		padding: 0.75rem;
		background-color: #1a1a2e;
		border-radius: 6px;
	}

	.stat-item.warning {
		border: 1px solid #f59e0b;
	}

	.stat-value {
		display: block;
		font-size: 1.25rem;
		font-weight: 600;
		color: #eaeaea;
	}

	.stat-label {
		display: block;
		font-size: 0.75rem;
		color: #9ca3af;
		margin-top: 0.25rem;
	}

	.file-type-dist {
		margin-bottom: 1rem;
		padding: 0.75rem;
		background-color: #1a1a2e;
		border-radius: 6px;
	}

	.dist-label {
		font-size: 0.875rem;
		color: #9ca3af;
		display: block;
		margin-bottom: 0.5rem;
	}

	.dist-items {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.dist-item {
		font-size: 0.75rem;
		padding: 0.25rem 0.5rem;
		background-color: #0f3460;
		border-radius: 4px;
		color: #eaeaea;
	}

	.workflow-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem;
		background: #1e1e1e;
		border-radius: 8px;
		margin-bottom: 1rem;
	}

	.workflow-step {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 1rem;
		border-radius: 6px;
		background: #2a2a2a;
		opacity: 0.5;
	}

	.workflow-step.active {
		opacity: 1;
		background: #3a3a3a;
	}

	.workflow-step.complete {
		background: #1a3a1a;
	}

	.step-icon {
		width: 24px;
		height: 24px;
		border-radius: 50%;
		background: #444;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.75rem;
	}

	.workflow-step.complete .step-icon {
		background: #2a7a2a;
	}

	.workflow-arrow {
		color: #666;
	}

	.step-label {
		font-size: 0.875rem;
		color: #9ca3af;
	}

	.step-count {
		font-size: 0.875rem;
		color: #eaeaea;
		font-weight: 600;
	}
</style>
