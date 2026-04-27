<script lang="ts">
	// Import shared stores - already initialized in +layout.svelte
	import { config, hardware, stats, modelLoaded, isLoading, error } from '$lib/stores/app';
	import { PageHeader, StatCard } from '$lib/components';

	// Get model path from config store
	let modelPath = $derived($config?.model?.local_path || '');

	function dismissError() {
		error.set('');
	}

	function gb(bytes: number | undefined): string {
		if (!bytes) return '0';
		return (bytes / 1024 / 1024 / 1024).toFixed(1);
	}
</script>

<div class="page">
	<PageHeader title="Dashboard" subtitle="Project status and quick actions" />

	{#if $error}
		<div class="error-banner" role="alert">
			<span>{$error}</span>
			<button class="dismiss" onclick={dismissError} aria-label="Dismiss error">×</button>
		</div>
	{/if}

	<div class="stat-grid">
		<StatCard
			value={$isLoading ? '...' : ($stats?.registry_count ?? 0)}
			label="Files registered"
			variant="info"
		/>
		<StatCard
			value={$isLoading ? '...' : ($stats?.intelligence_count ?? 0)}
			label="Facts extracted"
			variant="success"
		/>
		<StatCard value={$isLoading ? '...' : ($hardware?.cpu_cores ?? '...')} label="CPU cores" />
	</div>

	{#if $hardware}
		<section class="dashboard-section">
			<h2>Hardware</h2>
			<div class="info-grid">
				<div class="info-card">
					<span class="info-label">CPU</span>
					<span class="info-value">{$hardware.cpu_cores} cores</span>
				</div>
				<div class="info-card">
					<span class="info-label">Memory</span>
					<span class="info-value">{gb($hardware.total_memory)} GB total</span>
				</div>
				<div class="info-card">
					<span class="info-label">Available</span>
					<span class="info-value">{gb($hardware.available_memory)} GB</span>
				</div>
				<div class="info-card">
					<span class="info-label">Backend</span>
					<span class="info-value">{$hardware.gpu_backend || 'CPU'}</span>
				</div>
				{#if $hardware.gpu_name}
					<div class="info-card full-width">
						<span class="info-label">GPU</span>
						<span class="info-value">
							{$hardware.gpu_name} ({($hardware.gpu_memory / 1024 / 1024).toFixed(0)} MB)
						</span>
					</div>
				{/if}
			</div>
		</section>
	{/if}

	<section class="dashboard-section">
		<h2>Model</h2>
		<div class="model-status">
			<div class="status-row">
				<span class="info-label">Status</span>
				<span class="info-value" class:loaded={$modelLoaded}>
					{$modelLoaded ? 'Loaded' : modelPath ? 'Not loaded' : 'No model configured'}
				</span>
			</div>
			{#if modelPath}
				<div class="status-row">
					<span class="info-label">Path</span>
					<span class="info-value mono">{modelPath}</span>
				</div>
			{/if}
		</div>
	</section>

	<section class="dashboard-section">
		<h2>Quick actions</h2>
		<div class="action-buttons">
			<a href="/analysis" class="btn">Start analysis</a>
			<a href="/results" class="btn">View results</a>
			<a href="/quality" class="btn">Quality review</a>
			<a href="/chains" class="btn">Evidence chains</a>
			<a href="/entities" class="btn">Entity resolution</a>
			<a href="/pipelines" class="btn">Pipelines</a>
			<a href="/settings" class="btn ghost">Settings</a>
		</div>
	</section>
</div>

<style>
	.error-banner {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.error-banner .dismiss {
		background: none;
		border: none;
		color: inherit;
		font-size: 1.25rem;
		cursor: pointer;
		padding: 0 var(--space-1);
	}

	.dashboard-section {
		margin-top: var(--space-6);
	}

	.dashboard-section h2 {
		font-size: var(--text-lg);
		font-weight: 600;
		margin: 0 0 var(--space-3);
		color: var(--color-text-secondary);
	}

	.info-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
		gap: var(--space-3);
	}

	.info-card {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-4);
		background-color: var(--color-bg-card);
		border-radius: var(--radius-lg);
		border: 1px solid var(--color-border);
	}

	.info-card.full-width {
		grid-column: 1 / -1;
	}

	.info-label {
		font-size: var(--text-xs);
		color: var(--color-text-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.info-value {
		font-size: var(--text-base);
		color: var(--color-text-primary);
		font-weight: 500;
	}

	.info-value.loaded {
		color: var(--color-status-confirmed);
	}

	.info-value.mono {
		font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
		font-size: var(--text-sm);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.model-status {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-4);
		background-color: var(--color-bg-card);
		border-radius: var(--radius-lg);
		border: 1px solid var(--color-border);
	}

	.status-row {
		display: flex;
		gap: var(--space-3);
		align-items: center;
		min-width: 0;
	}

	.status-row .info-label {
		min-width: 80px;
	}

	.action-buttons {
		display: flex;
		gap: var(--space-2);
		flex-wrap: wrap;
	}

	/* Override .btn to have a tinted background for the action shortcuts. */
	.action-buttons :global(a.btn) {
		text-decoration: none;
		background-color: var(--color-bg-elevated);
	}

	.action-buttons :global(a.btn:hover) {
		background-color: var(--color-accent);
		color: white;
		border-color: var(--color-accent);
	}
</style>
