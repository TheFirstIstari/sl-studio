<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { open } from '@tauri-apps/plugin-dialog';
	import { listen } from '@tauri-apps/api/event';
	import { onMount, onDestroy } from 'svelte';

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

	interface HardwareStatus {
		cpu_threads: number;
		total_memory_gb: number;
		scaling: {
			batch_size: number;
			cpu_workers: number;
		};
	}

	const RECOMMENDED_MODELS = [
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
		},
		{
			id: 'TheBloke/Qwen-2.5-7B-Instruct-GGUF',
			name: 'Qwen 2.5 7B Instruct',
			size: '~4.7GB',
			quantization: 'Q4_K_M'
		},
		{
			id: 'TheBloke/Mixtral-8x7B-Instruct-v0.1-GGUF',
			name: 'Mixtral 8x7B',
			size: '~26GB',
			quantization: 'Q4_K_M'
		}
	];

	let config = $state({
		projectName: 'New Investigation',
		evidenceRoot: '',
		registryDb: '',
		intelligenceDb: '',
		modelPath: '',
		contextSize: 16384,
		cpuWorkers: 8,
		vramAllocation: 0.45,
		batchSize: 24
	});

	let loading = $state(true);
	let saving = $state(false);
	let statusMessage = $state('');

	let modelsDir = $state('');
	let downloadedModels = $state<ModelInfo[]>([]);
	let downloading = $state(false);
	let downloadProgress = $state<DownloadProgress | null>(null);
	let selectedModel = $state('');
	let downloadError = $state('');

	let unlisten: (() => void) | null = null;

	onMount(async () => {
		try {
			const loaded = await invoke<any>('load_config');
			if (loaded) {
				config = {
					projectName: loaded.project?.name || 'New Investigation',
					evidenceRoot: loaded.project?.evidence_root || '',
					registryDb: loaded.project?.registry_db || '',
					intelligenceDb: loaded.project?.intelligence_db || '',
					modelPath: loaded.model?.local_path || '',
					contextSize: loaded.model?.context_length || 16384,
					cpuWorkers: loaded.hardware?.cpu_workers || 8,
					vramAllocation: loaded.hardware?.vram_allocation || 0.45,
					batchSize: loaded.processing?.batch_size || 24
				};
			}

			const hwStatus = await invoke<HardwareStatus>('detect_hardware');
			if (hwStatus) {
				config.cpuWorkers = hwStatus.cpu_threads || 8;
				config.batchSize = hwStatus.scaling?.batch_size || 24;
			}

			modelsDir = await invoke<string>('get_models_dir');
			downloadedModels = await invoke<ModelInfo[]>('list_downloaded_models');

			unlisten = await listen<DownloadProgress>('download_status', (event) => {
				downloadProgress = event.payload;
			});
		} catch (e) {
			console.error('Failed to load config:', e);
		} finally {
			loading = false;
		}
	});

	onDestroy(() => {
		if (unlisten) unlisten();
	});

	async function saveConfig() {
		saving = true;
		statusMessage = '';
		try {
			const configData = {
				project: {
					name: config.projectName,
					evidence_root: config.evidenceRoot,
					registry_db: config.registryDb,
					intelligence_db: config.intelligenceDb
				},
				model: {
					id: 'qwen-2.5-7b',
					local_path: config.modelPath,
					context_length: config.contextSize
				},
				hardware: {
					cpu_workers: config.cpuWorkers,
					vram_allocation: config.vramAllocation
				},
				processing: {
					batch_size: config.batchSize
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

<div class="settings">
	<h1>Settings</h1>

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

				<div class="form-group">
					<label for="contextSize">Context Window</label>
					<input
						type="number"
						id="contextSize"
						bind:value={config.contextSize}
						min="2048"
						max="32768"
						step="2048"
					/>
					<p class="hint">LLM context size (2048-32768)</p>
				</div>
			</section>

			<section class="settings-section">
				<h2>Hardware</h2>

				<div class="form-group">
					<label for="cpuWorkers">CPU Workers</label>
					<input type="number" id="cpuWorkers" bind:value={config.cpuWorkers} min="1" max="32" />
					<p class="hint">Number of parallel workers</p>
				</div>

				<div class="form-group">
					<label for="vramAllocation">VRAM Allocation</label>
					<input
						type="range"
						id="vramAllocation"
						bind:value={config.vramAllocation}
						min="0.1"
						max="0.95"
						step="0.05"
					/>
					<span class="range-value">{Math.round(config.vramAllocation * 100)}%</span>
				</div>
			</section>

			<section class="settings-section">
				<h2>Processing</h2>

				<div class="form-group">
					<label for="batchSize">Batch Size</label>
					<input type="number" id="batchSize" bind:value={config.batchSize} min="1" max="128" />
					<p class="hint">Files per inference batch</p>
				</div>
			</section>
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

	h1 {
		font-size: 1.75rem;
		margin-bottom: 1.5rem;
		color: #eaeaea;
	}

	h2 {
		font-size: 1.25rem;
		margin-bottom: 1rem;
		color: #e94560;
		border-bottom: 1px solid #0f3460;
		padding-bottom: 0.5rem;
	}

	.loading {
		text-align: center;
		padding: 2rem;
		color: #9ca3af;
	}

	.settings-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
		gap: 1.5rem;
	}

	.settings-section {
		background-color: #16213e;
		padding: 1.5rem;
		border-radius: 8px;
		border: 1px solid #0f3460;
	}

	.form-group {
		margin-bottom: 1.25rem;
	}

	label {
		display: block;
		font-size: 0.875rem;
		color: #9ca3af;
		margin-bottom: 0.5rem;
	}

	.label-text {
		display: block;
		font-size: 0.875rem;
		color: #9ca3af;
		margin-bottom: 0.5rem;
	}

	input[type='text'],
	input[type='number'],
	select {
		width: 100%;
		padding: 0.625rem 0.875rem;
		background-color: #1a1a2e;
		border: 1px solid #0f3460;
		border-radius: 6px;
		color: #eaeaea;
		font-size: 0.875rem;
	}

	select {
		cursor: pointer;
	}

	input:focus,
	select:focus {
		outline: none;
		border-color: #e94560;
	}

	input[type='range'] {
		width: 100%;
		accent-color: #e94560;
	}

	.range-value {
		display: inline-block;
		margin-left: 0.5rem;
		font-size: 0.875rem;
		color: #e94560;
		font-weight: 600;
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
		background-color: #0f3460;
		border: 1px solid #0f3460;
		border-radius: 6px;
		color: #eaeaea;
		cursor: pointer;
		font-size: 0.875rem;
		transition: all 0.2s;
		white-space: nowrap;
	}

	.download-btn {
		margin-top: 0.75rem;
		width: 100%;
		background-color: #e94560;
		border-color: #e94560;
	}

	.browse-btn:hover,
	.download-btn:hover:not(:disabled) {
		background-color: #e94560;
		border-color: #e94560;
	}

	.download-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.hint {
		font-size: 0.75rem;
		color: #6b7280;
		margin-top: 0.25rem;
	}

	.model-select {
		width: 100%;
		padding: 0.625rem 0.875rem;
		background-color: #1a1a2e;
		border: 1px solid #0f3460;
		border-radius: 6px;
		color: #eaeaea;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.progress-bar {
		height: 8px;
		background-color: #1a1a2e;
		border-radius: 4px;
		overflow: hidden;
		margin-top: 0.75rem;
	}

	.progress-fill {
		height: 100%;
		background-color: #e94560;
		transition: width 0.3s ease;
	}

	.progress-text {
		font-size: 0.75rem;
		color: #9ca3af;
		margin-top: 0.25rem;
		text-align: center;
	}

	.error-text {
		font-size: 0.75rem;
		color: #ef4444;
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
		background-color: #1a1a2e;
		border: 1px solid #0f3460;
		border-radius: 6px;
		cursor: pointer;
		transition: all 0.2s;
		text-align: left;
	}

	.model-item:hover {
		border-color: #e94560;
	}

	.model-item.selected {
		border-color: #e94560;
		background-color: #0f3460;
	}

	.model-name {
		font-size: 0.875rem;
		color: #eaeaea;
	}

	.model-size {
		font-size: 0.75rem;
		color: #9ca3af;
	}

	.actions {
		margin-top: 2rem;
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.save-btn {
		padding: 0.75rem 1.5rem;
		background-color: #e94560;
		border: none;
		border-radius: 6px;
		color: #ffffff;
		font-size: 1rem;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
	}

	.save-btn:hover:not(:disabled) {
		background-color: #d13650;
	}

	.save-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.status-message {
		font-size: 0.875rem;
		color: #4ade80;
	}

	.status-message.error {
		color: #ef4444;
	}
</style>
