<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';
	import { PageHeader, StatCard } from '$lib/components';

	interface Anomaly {
		id: number;
		fingerprint: string;
		filename: string;
		summary: string;
		metric: string;
		value: number;
		expected_value: number;
		deviation: number;
		associated_date: string | null;
	}

	let anomalies = $state<Anomaly[]>([]);
	let loading = $state(true);
	let selectedMetric = $state<string>('severity');
	let threshold = $state<number>(2.0);
	let selectedAnomaly = $state<Anomaly | null>(null);

	onMount(async () => {
		await loadAnomalies();
	});

	async function loadAnomalies() {
		loading = true;
		try {
			anomalies = await invoke<Anomaly[]>('detect_anomalies', {
				metric: selectedMetric,
				thresholdStd: threshold
			});
		} catch (e) {
			console.error('Error loading anomalies:', e);
			anomalies = [];
		} finally {
			loading = false;
		}
	}

	async function changeMetric(metric: string) {
		selectedMetric = metric;
		await loadAnomalies();
	}

	function cssVar(name: string): string {
		return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
	}

	function getDeviationColor(deviation: number): string {
		const abs = Math.abs(deviation);
		if (abs > 3) return cssVar('--color-severity-high');
		if (abs > 2) return cssVar('--color-severity-medium-high');
		return cssVar('--color-severity-medium');
	}

	function getMetricLabel(metric: string): string {
		switch (metric) {
			case 'severity':
				return 'Severity Score';
			case 'confidence':
				return 'Confidence';
			case 'quality':
				return 'Quality Score';
			default:
				return metric;
		}
	}

	function formatDate(dateStr: string | null): string {
		if (!dateStr) return 'N/A';
		return new Date(dateStr).toLocaleDateString();
	}
</script>

<div class="anomalies-page page">
	<PageHeader title="Anomaly Detection">
		{#snippet actions()}
			<div class="control-group">
				<label for="metric-select">Metric:</label>
				<select
					id="metric-select"
					value={selectedMetric}
					onchange={(e) => changeMetric(e.currentTarget.value)}
				>
					<option value="severity">Severity</option>
					<option value="confidence">Confidence</option>
					<option value="quality">Quality</option>
				</select>
			</div>

			<div class="control-group">
				<label for="threshold">Threshold (σ):</label>
				<input
					id="threshold"
					type="number"
					min="1"
					max="5"
					step="0.5"
					bind:value={threshold}
					onchange={loadAnomalies}
				/>
			</div>
		{/snippet}
	</PageHeader>

	{#if loading}
		<div class="loading">Analyzing for anomalies...</div>
	{:else if anomalies.length === 0}
		<div class="empty">
			<svg
				class="empty-icon"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
			>
				<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
				<polyline points="22 4 12 14.01 9 11.01" />
			</svg>
			<p>No anomalies detected.</p>
			<p class="empty-hint">All values are within the normal range.</p>
		</div>
	{:else}
		<div class="stat-grid">
			<StatCard value={anomalies.length} label="Anomalies Found" variant="warning" />
			<StatCard value={selectedMetric} label="Metric" />
			<StatCard value="±{threshold}σ" label="Threshold" />
		</div>

		<div class="anomalies-list">
			{#each anomalies as anomaly}
				<button
					class="anomaly-card"
					class:selected={selectedAnomaly?.id === anomaly.id}
					onclick={() => (selectedAnomaly = anomaly)}
				>
					<div class="anomaly-header">
						<span class="anomaly-metric">{getMetricLabel(anomaly.metric)}</span>
						<span
							class="anomaly-deviation"
							style="background-color: {getDeviationColor(anomaly.deviation)}"
						>
							{anomaly.deviation > 0 ? '+' : ''}{anomaly.deviation.toFixed(1)}σ
						</span>
					</div>
					<div class="anomaly-summary">{anomaly.summary}</div>
					<div class="anomaly-meta">
						<span class="anomaly-filename">{anomaly.filename}</span>
						<span class="anomaly-date">{formatDate(anomaly.associated_date)}</span>
					</div>
				</button>
			{/each}
		</div>
	{/if}

	{#if selectedAnomaly}
		<div class="detail-panel">
			<div class="detail-header">
				<h2>Anomaly Details</h2>
				<button class="close-btn" onclick={() => (selectedAnomaly = null)}>×</button>
			</div>
			<div class="detail-content">
				<div
					class="metric-badge"
					style="background-color: {getDeviationColor(selectedAnomaly.deviation)}"
				>
					{getMetricLabel(selectedAnomaly.metric)}
				</div>

				<div class="detail-row">
					<span class="detail-label">Deviation:</span>
					<span
						class="detail-value deviation"
						style="color: {getDeviationColor(selectedAnomaly.deviation)}"
					>
						{selectedAnomaly.deviation > 0 ? '+' : ''}{selectedAnomaly.deviation.toFixed(2)}σ
					</span>
				</div>

				<div class="comparison">
					<div class="comparison-item">
						<span class="comparison-label">Actual Value</span>
						<span class="comparison-value">{selectedAnomaly.value.toFixed(2)}</span>
					</div>
					<div class="comparison-arrow">→</div>
					<div class="comparison-item">
						<span class="comparison-label">Expected Value</span>
						<span class="comparison-value">{selectedAnomaly.expected_value.toFixed(2)}</span>
					</div>
				</div>

				<div class="detail-row">
					<span class="detail-label">Filename:</span>
					<span class="detail-value">{selectedAnomaly.filename}</span>
				</div>

				<div class="detail-row">
					<span class="detail-label">Date:</span>
					<span class="detail-value">{formatDate(selectedAnomaly.associated_date)}</span>
				</div>

				<div class="detail-section">
					<h3>Summary</h3>
					<p>{selectedAnomaly.summary}</p>
				</div>

				<div class="detail-section">
					<h3>Fingerprint</h3>
					<code class="fingerprint">{selectedAnomaly.fingerprint}</code>
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.anomalies-page {
		height: 100%;
		display: flex;
		flex-direction: column;
		position: relative;
	}

	.control-group {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.control-group label {
		font-size: 0.875rem;
		color: var(--color-text-secondary);
	}

	.control-group select,
	.control-group input {
		padding: 0.5rem;
		background-color: var(--color-bg-card);
		border: 1px solid var(--color-border);
		border-radius: 6px;
		color: var(--color-text-primary);
		font-size: 0.875rem;
	}

	.control-group select:focus,
	.control-group input:focus {
		outline: none;
		border-color: var(--color-accent);
	}

	.loading,
	.empty {
		text-align: center;
		padding: 3rem;
		color: var(--color-text-secondary);
	}

	.empty-icon {
		width: 48px;
		height: 48px;
		color: var(--color-status-confirmed);
		margin-bottom: 1rem;
	}

	.empty-hint {
		font-size: 0.875rem;
		color: var(--color-text-muted);
		margin-top: 0.5rem;
	}

	.anomalies-list {
		flex: 1;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding-right: 350px;
	}

	.anomaly-card {
		width: 100%;
		text-align: left;
		padding: 1rem;
		background-color: var(--color-bg-card);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.anomaly-card:hover {
		border-color: var(--color-accent);
	}

	.anomaly-card.selected {
		border-color: var(--color-accent);
		background-color: var(--color-bg-elevated);
	}

	.anomaly-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	.anomaly-metric {
		font-size: 0.75rem;
		color: var(--color-text-secondary);
		text-transform: uppercase;
	}

	.anomaly-deviation {
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--color-text-inverse);
	}

	.anomaly-summary {
		font-size: 0.875rem;
		color: var(--color-text-primary);
		margin-bottom: 0.5rem;
		line-height: 1.4;
	}

	.anomaly-meta {
		display: flex;
		gap: 1rem;
		font-size: 0.75rem;
		color: var(--color-text-muted);
	}

	.detail-panel {
		position: fixed;
		right: 0;
		top: 0;
		bottom: 0;
		width: 350px;
		background-color: var(--color-bg-card);
		border-left: 1px solid var(--color-border);
		padding: 1.5rem;
		overflow-y: auto;
	}

	.detail-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.5rem;
	}

	.detail-header h2 {
		font-size: 1.25rem;
		color: var(--color-accent);
	}

	.close-btn {
		width: 32px;
		height: 32px;
		background: none;
		border: none;
		color: var(--color-text-secondary);
		font-size: 1.5rem;
		cursor: pointer;
		border-radius: 4px;
	}

	.close-btn:hover {
		background-color: var(--color-bg-elevated);
		color: var(--color-text-primary);
	}

	.metric-badge {
		display: inline-block;
		padding: 0.5rem 1rem;
		border-radius: 6px;
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--color-text-inverse);
		margin-bottom: 1rem;
	}

	.detail-row {
		display: flex;
		justify-content: space-between;
		padding: 0.75rem 0;
		border-bottom: 1px solid var(--color-border);
	}

	.detail-label {
		color: var(--color-text-secondary);
		font-size: 0.875rem;
	}

	.detail-value {
		color: var(--color-text-primary);
		font-size: 0.875rem;
		font-weight: 500;
	}

	.detail-value.deviation {
		font-weight: 700;
	}

	.comparison {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem 0;
		margin: 1rem 0;
		background-color: var(--color-bg-input);
		border-radius: 8px;
	}

	.comparison-item {
		display: flex;
		flex-direction: column;
		align-items: center;
		flex: 1;
	}

	.comparison-label {
		font-size: 0.75rem;
		color: var(--color-text-secondary);
		margin-bottom: 0.25rem;
	}

	.comparison-value {
		font-size: 1.25rem;
		font-weight: 700;
		color: var(--color-text-primary);
	}

	.comparison-arrow {
		font-size: 1.5rem;
		color: var(--color-text-muted);
	}

	.detail-section {
		margin-top: 1.5rem;
	}

	.detail-section h3 {
		font-size: 0.875rem;
		color: var(--color-text-secondary);
		margin-bottom: 0.5rem;
	}

	.detail-section p {
		color: var(--color-text-primary);
		font-size: 0.875rem;
		line-height: 1.5;
	}

	.fingerprint {
		display: block;
		padding: 0.75rem;
		background-color: var(--color-bg-input);
		border-radius: 4px;
		font-family: 'SF Mono', Monaco, monospace;
		font-size: 0.75rem;
		color: var(--color-text-secondary);
		word-break: break-all;
	}
</style>
