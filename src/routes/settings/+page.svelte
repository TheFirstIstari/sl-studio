<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { open } from '@tauri-apps/plugin-dialog';
	import { listen } from '@tauri-apps/api/event';
	import { onMount, onDestroy } from 'svelte';
	import type { AppConfig } from '$lib/stores/app';
	import { PageHeader, StatCard } from '$lib/components';

	interface ModelInfo {
		id: string;
		filename: string;
		size: number;
		path: string;
	}

	interface DownloadProgress {
		bytes_downloaded: number;
		total_bytes: number;
		filename: string;
		status: string;
	}

	interface HardwareInfo {
		cpu_threads: number;
		total_memory_gb: number;
		available_memory_gb: number;
		recommended_workers: number;
		recommended_batch_size: number;
		cpu_workers: number;
	}

	interface HardwareStatus {
		cpu_threads: number;
		total_memory_gb: number;
		recommended_backend: string;
		scaling: {
			batch_size: number;
			cpu_workers: number;
		};
	}

	const RECOMMENDED_MODELS = [
		{
			id: 'ggml-org/gemma-3-4b-it-GGUF',
			name: 'Gemma 3 4B Instruct',
			size: '~2.5GB',
			quantization: 'Q4_K_M',
			note: 'Recommended for 16GB Macs - 131K context, QAT quantization'
		},
		{
			id: 'TheBloke/Mistral-7B-Instruct-v0.2-GGUF',
			name: 'Mistral 7B Instruct',
			size: '~4.1GB',
			quantization: 'Q4_K_M'
		},
		{
			id: 'TheBloke/Llama-2-7B-Chat-GGUF',
			name: 'Llama 2 7B Chat',
			size: '~3.8GB',
			quantization: 'Q4_K_M'
		}
	];

	let config = $state({
		projectName: 'New Investigation',
		evidenceRoot: '',
		registryDb: '',
		intelligenceDb: '',
		modelPath: '',
		contextSize: 8192,
		cpuWorkers: 6,
		vramAllocation: 0.4,
		batchSize: 6,
		gpuBackend: 'cpu'
	});

	let recommendedBackend = $state('cpu');
	let loading = $state(true);
	let saving = $state(false);
	let statusMessage = $state('');

	let downloadedModels = $state<ModelInfo[]>([]);
	let downloading = $state(false);
	let downloadProgress = $state<DownloadProgress | null>(null);
	let selectedModel = $state('');
	let downloadError = $state('');

	let hardwareInfo = $state<{
		cpu_threads: number;
		total_memory_gb: number;
		available_memory_gb: number;
		recommended_workers: number;
		recommended_batch_size: number;
	} | null>(null);

	interface SystemMonitor {
		cpu_usage_percent: number;
		memory_used_gb: number;
		memory_available_gb: number;
		memory_percent: number;
	}

	let systemMonitor = $state<SystemMonitor | null>(null);
	let monitorInterval: ReturnType<typeof setInterval> | null = null;

	let unlisten: (() => void) | null = null;

	async function updateSystemMonitor() {
		try {
			const next = await invoke<SystemMonitor>('get_system_monitor');
			// Skip the reactive update when nothing has changed so consumers of
			// `systemMonitor` don't re-run every 2s for identical values.
			if (
				systemMonitor &&
				next.cpu_usage_percent === systemMonitor.cpu_usage_percent &&
				next.memory_percent === systemMonitor.memory_percent &&
				next.memory_used_gb === systemMonitor.memory_used_gb &&
				next.memory_available_gb === systemMonitor.memory_available_gb
			) {
				return;
			}
			systemMonitor = next;
		} catch (e) {
			console.error('Failed to get system monitor:', e);
		}
	}

	onMount(async () => {
		try {
			const loaded = await invoke<Partial<AppConfig>>('load_config');
			if (loaded) {
				config = {
					projectName: loaded.project?.name || 'New Investigation',
					evidenceRoot: loaded.project?.evidence_root || '',
					registryDb: loaded.project?.registry_db || '',
					intelligenceDb: loaded.project?.intelligence_db || '',
					modelPath: loaded.model?.local_path || '',
					contextSize: loaded.model?.context_length || 8192,
					cpuWorkers: loaded.hardware?.cpu_workers || 6,
					vramAllocation: loaded.hardware?.gpu_memory_fraction ?? 0.4,
					batchSize: loaded.processing?.batch_size || 6,
					gpuBackend: loaded.hardware?.gpu_backend || 'cpu'
				};
			}

			const hwStatus = await invoke<HardwareStatus>('detect_hardware');
			if (hwStatus) {
				recommendedBackend = hwStatus.recommended_backend || 'cpu';
				// If config doesn't have gpu_backend set yet, use recommended
				if (!config.gpuBackend || config.gpuBackend === 'cpu') {
					config.gpuBackend = recommendedBackend;
				}
				config.cpuWorkers = hwStatus.cpu_threads || 8;
				config.batchSize = hwStatus.scaling?.batch_size || 24;
			}

			hardwareInfo = await invoke<typeof hardwareInfo>('get_hardware_info');

			downloadedModels = await invoke<ModelInfo[]>('list_downloaded_models');

			unlisten = await listen<DownloadProgress>('download_status', (event) => {
				downloadProgress = event.payload;
			});

			await updateSystemMonitor();
			monitorInterval = setInterval(updateSystemMonitor, 2000);
		} catch (e) {
			console.error('Failed to load config:', e);
		} finally {
			loading = false;
		}
	});

	onDestroy(() => {
		if (unlisten) unlisten();
		if (monitorInterval) clearInterval(monitorInterval);
	});

	async function saveConfig() {
		saving = true;
		statusMessage = '';
		try {
			// Get auto-detected hardware settings
			const hwInfo = await invoke<HardwareInfo>('get_hardware_info');

			const configData = {
				version: '0.2.0',
				project: {
					name: config.projectName,
					evidence_root: config.evidenceRoot,
					registry_db: config.registryDb,
					intelligence_db: config.intelligenceDb
				},
				model: {
					source: 'local',
					id: 'qwen-2.5-7b',
					quantization: 'awq',
					context_length: config.contextSize,
					downloaded: false,
					local_path: config.modelPath
				},
				hardware: {
					gpu_backend: 'metal', // Auto-detected
					gpu_memory_fraction: 0.8, // Auto-detected (80%)
					cpu_workers: hwInfo.cpu_workers, // Auto-detected
					ocr_provider: 'onnx',
					whisper_size: 'base'
				},
				processing: {
					batch_size: hwInfo.recommended_batch_size, // Auto-detected
					max_image_resolution: 2048
				}
			};

			await invoke('save_config', { config: configData });
			statusMessage = 'Configuration saved successfully!';
		} catch (e) {
			statusMessage = `Error: ${e}`;
		} finally {
			saving = false;
		}
	}

	async function selectFolder(field: 'evidenceRoot' | 'registryDb' | 'intelligenceDb') {
		try {
			const selected = await open({
				directory: field === 'evidenceRoot',
				multiple: false,
				title: `Select ${field.replace(/([A-Z])/g, ' $1').trim()}`
			});

			if (selected) {
				config[field] = selected as string;
			}
		} catch (e) {
			console.error('Error selecting folder:', e);
		}
	}

	async function downloadSelectedModel() {
		if (!selectedModel) return;

		const model = RECOMMENDED_MODELS.find((m) => m.id === selectedModel);
		if (!model) return;

		downloading = true;
		downloadError = '';
		downloadProgress = null;

		try {
			const result = await invoke<ModelInfo>('download_model', {
				repoId: model.id,
				filename: ''
			});

			config.modelPath = result.path;
			downloadedModels = await invoke<ModelInfo[]>('list_downloaded_models');
			statusMessage = `Model downloaded: ${result.filename}`;
		} catch (e) {
			downloadError = `Download failed: ${e}`;
		} finally {
			downloading = false;
			downloadProgress = null;
		}
	}

	async function selectModelFile() {
		try {
			const selected = await open({
				directory: false,
				multiple: false,
				title: 'Select GGUF Model File',
				filters: [{ name: 'GGUF Models', extensions: ['gguf'] }]
			});

			if (selected) {
				config.modelPath = selected as string;
			}
		} catch (e) {
			console.error('Error selecting model:', e);
		}
	}

	function formatBytes(bytes: number): string {
		if (bytes === 0) return '0 B';
		const k = 1024;
		const sizes = ['B', 'KB', 'MB', 'GB'];
		const i = Math.floor(Math.log(bytes) / Math.log(k));
		return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
	}

	function getProgressPercent(): number {
		if (!downloadProgress || downloadProgress.total_bytes === 0) return 0;
		return Math.round((downloadProgress.bytes_downloaded / downloadProgress.total_bytes) * 100);
	}
</script>

<div class="settings page">
	<PageHeader title="Settings" />

	{#if loading}
		<div class="loading">Loading configuration...</div>
	{:else}
		<div class="settings-grid">
			<section class="settings-section">
				<h2>Project</h2>

				<div class="form-group">
					<label for="projectName">Project Name</label>
					<input
						type="text"
						id="projectName"
						bind:value={config.projectName}
						placeholder="New Investigation"
					/>
				</div>

				<div class="form-group">
					<label for="evidenceRoot">Evidence Root Folder</label>
					<div class="input-with-button">
						<input
							type="text"
							id="evidenceRoot"
							bind:value={config.evidenceRoot}
							placeholder="/path/to/evidence"
						/>
						<button class="browse-btn" onclick={() => selectFolder('evidenceRoot')}>Browse</button>
					</div>
				</div>

				<div class="form-group">
					<label for="registryDb">Registry Database</label>
					<div class="input-with-button">
						<input
							type="text"
							id="registryDb"
							bind:value={config.registryDb}
							placeholder="/path/to/registry.db"
						/>
						<button class="browse-btn" onclick={() => selectFolder('registryDb')}>Browse</button>
					</div>
				</div>

				<div class="form-group">
					<label for="intelligenceDb">Intelligence Database</label>
					<div class="input-with-button">
						<input
							type="text"
							id="intelligenceDb"
							bind:value={config.intelligenceDb}
							placeholder="/path/to/intelligence.db"
						/>
						<button class="browse-btn" onclick={() => selectFolder('intelligenceDb')}>Browse</button
						>
					</div>
				</div>
			</section>

			<section class="settings-section">
				<h2>Model</h2>

				<div class="form-group">
					<label for="modelSelect">Download Model from HuggingFace</label>
					<select
						id="modelSelect"
						bind:value={selectedModel}
						class="model-select"
						disabled={downloading}
					>
						<option value="">Select a model...</option>
						{#each RECOMMENDED_MODELS as model}
							<option value={model.id}>
								{model.name} ({model.size})
							</option>
						{/each}
					</select>

					<button
						class="download-btn"
						onclick={downloadSelectedModel}
						disabled={!selectedModel || downloading}
					>
						{#if downloading}
							Downloading...
						{:else}
							Download Model
						{/if}
					</button>

					{#if downloadProgress}
						<div class="progress-bar">
							<div class="progress-fill" style="width: {getProgressPercent()}%"></div>
						</div>
						<div class="progress-text">
							{formatBytes(downloadProgress.bytes_downloaded)} / {formatBytes(
								downloadProgress.total_bytes
							)}
							({getProgressPercent()}%)
						</div>
					{/if}

					{#if downloadError}
						<div class="error-text">{downloadError}</div>
					{/if}
				</div>

				<div class="form-group">
					<label for="modelPath">Or Select Local Model</label>
					<div class="input-with-button">
						<input
							type="text"
							id="modelPath"
							bind:value={config.modelPath}
							placeholder="/path/to/model.gguf"
							readonly
						/>
						<button class="browse-btn" onclick={selectModelFile}>Browse</button>
					</div>
					<p class="hint">Currently selected: {config.modelPath || 'None'}</p>
				</div>

				{#if downloadedModels.length > 0}
					<div class="form-group">
						<span class="label-text">Downloaded Models</span>
						<div class="models-list">
							{#each downloadedModels as model}
								<button
									class="model-item"
									class:selected={config.modelPath === model.path}
									onclick={() => (config.modelPath = model.path)}
								>
									<span class="model-name">{model.filename}</span>
									<span class="model-size">{formatBytes(model.size)}</span>
								</button>
							{/each}
						</div>
					</div>
				{/if}
			</section>

			<section class="settings-section">
				<h2>Hardware</h2>

				<div class="form-group">
					<label for="gpu-backend">GPU Backend</label>
					<div class="display-value" id="gpu-backend">{recommendedBackend}</div>
					<p class="hint">Automatically detected</p>
				</div>
			</section>

			<section class="settings-section">
				<h2>System Monitor</h2>

				{#if systemMonitor}
					<div class="monitor-grid">
						<div class="monitor-item">
							<span class="monitor-label">CPU Usage</span>
							<div class="monitor-bar">
								<div class="monitor-fill" style="width: {systemMonitor.cpu_usage_percent}%"></div>
							</div>
							<span class="monitor-value">{systemMonitor.cpu_usage_percent.toFixed(1)}%</span>
						</div>

						<div class="monitor-item">
							<span class="monitor-label">Memory</span>
							<div class="monitor-bar">
								<div class="monitor-fill" style="width: {systemMonitor.memory_percent}%"></div>
							</div>
							<span class="monitor-value"
								>{systemMonitor.memory_used_gb.toFixed(1)}GB / {(
									systemMonitor.memory_used_gb + systemMonitor.memory_available_gb
								).toFixed(1)}GB</span
							>
						</div>
					</div>
				{:else}
					<p class="loading-text">Loading system monitor...</p>
				{/if}
			</section>

			{#if hardwareInfo}
				<section class="settings-section hardware-section">
					<h2>Hardware Stats</h2>
					<div class="stat-grid">
						<StatCard value={hardwareInfo.cpu_threads} label="CPU Cores" variant="info" />
						<StatCard
							value="{hardwareInfo.total_memory_gb.toFixed(1)} GB"
							label="Total RAM"
							variant="info"
						/>
						<StatCard value={hardwareInfo.recommended_workers} label="Recommended Workers" />
						<StatCard value={hardwareInfo.recommended_batch_size} label="Recommended Batch Size" />
					</div>
				</section>
			{/if}
		</div>

		<div class="actions">
			<button class="save-btn" onclick={saveConfig} disabled={saving}>
				{saving ? 'Saving...' : 'Save Configuration'}
			</button>
			{#if statusMessage}
				<span class="status-message" class:error={statusMessage.startsWith('Error')}>
					{statusMessage}
				</span>
			{/if}
		</div>
	{/if}
</div>

<style>
	.settings {
		max-width: 1000px;
	}

	h2 {
		font-size: 1.25rem;
		margin-bottom: 1rem;
		color: var(--color-accent);
		border-bottom: 1px solid var(--color-border);
		padding-bottom: 0.5rem;
	}

	.loading {
		text-align: center;
		padding: 2rem;
		color: var(--color-text-secondary);
	}

	.settings-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
		gap: 1.5rem;
	}

	.settings-section {
		background-color: var(--color-bg-card);
		padding: 1.5rem;
		border-radius: 8px;
		border: 1px solid var(--color-border);
	}

	.form-group {
		margin-bottom: 1.25rem;
	}

	label {
		display: block;
		font-size: 0.875rem;
		color: var(--color-text-secondary);
		margin-bottom: 0.5rem;
	}

	.label-text {
		display: block;
		font-size: 0.875rem;
		color: var(--color-text-secondary);
		margin-bottom: 0.5rem;
	}

	input[type='text'],
	input[type='number'],
	select {
		width: 100%;
		padding: 0.625rem 0.875rem;
		background-color: var(--color-bg-input);
		border: 1px solid var(--color-border);
		border-radius: 6px;
		color: var(--color-text-primary);
		font-size: 0.875rem;
	}

	select {
		cursor: pointer;
	}

	input:focus,
	select:focus {
		outline: none;
		border-color: var(--color-accent);
	}

	input[type='range'] {
		width: 100%;
		accent-color: var(--color-accent);
	}

	.range-value {
		display: inline-block;
		margin-left: 0.5rem;
		font-size: 0.875rem;
		color: var(--color-accent);
		font-weight: 600;
	}

	.display-value {
		background: var(--color-bg-input);
		padding: 0.625rem 0.875rem;
		border-radius: 6px;
		color: var(--color-accent);
		font-weight: 600;
		font-size: 1rem;
	}

	.input-with-button {
		display: flex;
		gap: 0.5rem;
	}

	.input-with-button input {
		flex: 1;
	}

	.browse-btn,
	.download-btn {
		padding: 0.625rem 1rem;
		background-color: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 6px;
		color: var(--color-text-primary);
		cursor: pointer;
		font-size: 0.875rem;
		transition: all 0.2s;
		white-space: nowrap;
	}

	.download-btn {
		margin-top: 0.75rem;
		width: 100%;
		background-color: var(--color-accent);
		border-color: var(--color-accent);
	}

	.browse-btn:hover,
	.download-btn:hover:not(:disabled) {
		background-color: var(--color-accent);
		border-color: var(--color-accent);
	}

	.download-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.hint {
		font-size: 0.75rem;
		color: var(--color-text-muted);
		margin-top: 0.25rem;
	}

	.model-select {
		width: 100%;
		padding: 0.625rem 0.875rem;
		background-color: var(--color-bg-input);
		border: 1px solid var(--color-border);
		border-radius: 6px;
		color: var(--color-text-primary);
		font-size: 0.875rem;
		cursor: pointer;
	}

	.progress-bar {
		height: 8px;
		background-color: var(--color-bg-input);
		border-radius: 4px;
		overflow: hidden;
		margin-top: 0.75rem;
	}

	.progress-fill {
		height: 100%;
		background-color: var(--color-accent);
		transition: width 0.3s ease;
	}

	.progress-text {
		font-size: 0.75rem;
		color: var(--color-text-secondary);
		margin-top: 0.25rem;
		text-align: center;
	}

	.error-text {
		font-size: 0.75rem;
		color: var(--color-severity-high);
		margin-top: 0.5rem;
	}

	.models-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin-top: 0.5rem;
	}

	.model-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem;
		background-color: var(--color-bg-input);
		border: 1px solid var(--color-border);
		border-radius: 6px;
		cursor: pointer;
		transition: all 0.2s;
		text-align: left;
	}

	.model-item:hover {
		border-color: var(--color-accent);
	}

	.model-item.selected {
		border-color: var(--color-accent);
		background-color: var(--color-bg-elevated);
	}

	.model-name {
		font-size: 0.875rem;
		color: var(--color-text-primary);
	}

	.model-size {
		font-size: 0.75rem;
		color: var(--color-text-secondary);
	}

	.actions {
		margin-top: 2rem;
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.save-btn {
		padding: 0.75rem 1.5rem;
		background-color: var(--color-accent);
		border: none;
		border-radius: 6px;
		color: var(--color-text-inverse);
		font-size: 1rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.save-btn:hover:not(:disabled) {
		background-color: var(--color-accent-hover);
	}

	.save-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.status-message {
		font-size: 0.875rem;
		color: var(--color-status-confirmed);
	}

	.status-message.error {
		color: var(--color-severity-high);
	}

	.monitor-grid {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.monitor-item {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.monitor-label {
		font-size: 0.875rem;
		color: var(--color-text-secondary);
	}

	.monitor-bar {
		height: 8px;
		background: var(--color-bg-elevated);
		border-radius: 4px;
		overflow: hidden;
	}

	.monitor-fill {
		height: 100%;
		background: linear-gradient(90deg, var(--color-status-info), var(--color-status-confirmed));
		transition: width 0.3s ease;
	}

	.monitor-value {
		font-size: 0.75rem;
		color: var(--color-text-primary);
	}

	.loading-text {
		color: var(--color-text-secondary);
		font-size: 0.875rem;
	}

	.hardware-section {
		grid-column: 1 / -1;
	}
</style>
