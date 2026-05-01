<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount, onDestroy } from 'svelte';
	import { Chart, registerables } from 'chart.js';
	import { PageHeader, StatCard } from '$lib/components';

	Chart.register(...registerables);

	// Read a CSS custom property at runtime so Chart.js uses themed colours.
	function cssVar(name: string): string {
		return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
	}

	interface OverallStats {
		total_facts: number;
		avg_severity: number;
		avg_confidence: number;
		avg_quality: number;
		total_entities: number;
		unique_entities: number;
		total_chains: number;
		total_chain_links: number;
	}

	interface CategoryStat {
		category: string;
		count: number;
		avg_severity: number | null;
		avg_confidence: number | null;
	}

	interface SeverityStat {
		severity: number;
		count: number;
	}

	interface EntityCentrality {
		entity_id: number;
		entity_type: string;
		value: string;
		document_count: number;
		occurrence_count: number;
		avg_confidence: number | null;
		centrality_score: number;
	}

	let overallStats = $state<OverallStats | null>(null);
	let categoryStats = $state<CategoryStat[]>([]);
	let severityStats = $state<SeverityStat[]>([]);
	let topEntities = $state<EntityCentrality[]>([]);
	let loading = $state(true);

	let severityChartEl: HTMLCanvasElement | undefined = $state();
	let categoryChartEl: HTMLCanvasElement | undefined = $state();
	let entityChartEl: HTMLCanvasElement | undefined = $state();

	let severityChart: Chart | null = null;
	let categoryChart: Chart | null = null;
	let entityChart: Chart | null = null;

	onMount(async () => {
		await loadStats();
		initCharts();
	});

	onDestroy(() => {
		if (severityChart) {
			severityChart.destroy();
			severityChart = null;
		}
		if (categoryChart) {
			categoryChart.destroy();
			categoryChart = null;
		}
		if (entityChart) {
			entityChart.destroy();
			entityChart = null;
		}
	});

	async function loadStats() {
		loading = true;
		try {
			overallStats = await invoke<OverallStats>('get_overall_statistics');
			categoryStats = await invoke<CategoryStat[]>('get_category_distribution');
			severityStats = await invoke<SeverityStat[]>('get_severity_distribution');
			topEntities = await invoke<EntityCentrality[]>('get_entity_centrality', {
				entityType: null,
				minConfidence: 0.0
			});
			topEntities = topEntities.slice(0, 20);
		} catch (e) {
			console.error('Error loading stats:', e);
		} finally {
			loading = false;
		}
	}

	function initCharts() {
		if (severityChartEl) {
			severityChart = new Chart(severityChartEl, {
				type: 'bar',
				data: {
					labels: [],
					datasets: [
						{
							label: 'Facts by Severity',
							data: [],
							backgroundColor: [
								cssVar('--color-severity-low'),
								cssVar('--color-severity-medium'),
								cssVar('--color-severity-medium-high'),
								cssVar('--color-severity-high'),
								cssVar('--color-severity-high')
							]
						}
					]
				},
				options: {
					responsive: true,
					maintainAspectRatio: false,
					plugins: {
						legend: { display: false }
					},
					scales: {
						y: {
							beginAtZero: true,
							ticks: { color: cssVar('--color-text-secondary') },
							grid: { color: cssVar('--color-border') }
						},
						x: {
							ticks: { color: cssVar('--color-text-secondary') },
							grid: { color: cssVar('--color-border') }
						}
					}
				}
			});
		}

		if (categoryChartEl) {
			categoryChart = new Chart(categoryChartEl, {
				type: 'doughnut',
				data: {
					labels: [],
					datasets: [
						{
							data: [],
							backgroundColor: [cssVar('--color-accent'), cssVar('--color-entity-person'), cssVar('--color-entity-org'), cssVar('--color-entity-location'), cssVar('--color-entity-date'), cssVar('--color-severity-medium-high')]
						}
					]
				},
				options: {
					responsive: true,
					maintainAspectRatio: false,
					plugins: {
						legend: {
							position: 'right',
							labels: { color: cssVar('--color-text-secondary') }
						}
					}
				}
			});
		}

		if (entityChartEl) {
			entityChart = new Chart(entityChartEl, {
				type: 'bar',
				data: {
					labels: [],
					datasets: [
						{
							label: 'Entity Occurrences',
							data: [],
							backgroundColor: cssVar('--color-accent')
						}
					]
				},
				options: {
					indexAxis: 'y',
					responsive: true,
					maintainAspectRatio: false,
					plugins: {
						legend: { display: false }
					},
					scales: {
						x: {
							beginAtZero: true,
							ticks: { color: cssVar('--color-text-secondary') },
							grid: { color: cssVar('--color-border') }
						},
						y: {
							ticks: { color: cssVar('--color-text-secondary') },
							grid: { color: cssVar('--color-border') }
						}
					}
				}
			});
		}
	}

	$effect(() => {
		if (severityStats.length > 0 && severityChart) {
			severityChart.data.labels = severityStats.map((s) => `Severity ${s.severity}`);
			severityChart.data.datasets[0].data = severityStats.map((s) => s.count);
			severityChart.update();
		}

		if (categoryStats.length > 0 && categoryChart) {
			categoryChart.data.labels = categoryStats.map((c) => c.category);
			categoryChart.data.datasets[0].data = categoryStats.map((c) => c.count);
			categoryChart.update();
		}

		if (topEntities.length > 0 && entityChart) {
			entityChart.data.labels = topEntities.map((e) => e.value.substring(0, 20));
			entityChart.data.datasets[0].data = topEntities.map((e) => e.occurrence_count);
			entityChart.update();
		}
	});
</script>

<div class="stats-page page">
	<PageHeader title="Statistics" />

	{#if loading}
		<div class="loading">Loading statistics...</div>
	{:else}
		{#if overallStats}
			<div class="stat-grid">
				<StatCard value={overallStats.total_facts} label="Total Facts" variant="info" />
				<StatCard value={overallStats.avg_severity?.toFixed(1) || '0'} label="Avg Severity" />
				<StatCard
					value={overallStats.avg_confidence
						? (overallStats.avg_confidence * 100).toFixed(0) + '%'
						: 'N/A'}
					label="Avg Confidence"
				/>
				<StatCard value={overallStats.total_entities} label="Entity Mentions" />
				<StatCard value={overallStats.unique_entities} label="Unique Entities" />
				<StatCard value={overallStats.total_chains} label="Evidence Chains" />
			</div>
		{/if}

		<div class="charts-grid">
			<div class="chart-card">
				<h2>Facts by Severity</h2>
				<div class="chart-container">
					<canvas bind:this={severityChartEl}></canvas>
				</div>
			</div>

			<div class="chart-card">
				<h2>Facts by Category</h2>
				<div class="chart-container">
					<canvas bind:this={categoryChartEl}></canvas>
				</div>
			</div>

			<div class="chart-card wide">
				<h2>Top Entities</h2>
				<div class="chart-container">
					<canvas bind:this={entityChartEl}></canvas>
				</div>
			</div>
		</div>

		{#if categoryStats.length > 0}
			<div class="table-card">
				<h2>Category Details</h2>
				<table class="data-table">
					<thead>
						<tr>
							<th>Category</th>
							<th>Count</th>
							<th>Avg Severity</th>
							<th>Avg Confidence</th>
						</tr>
					</thead>
					<tbody>
						{#each categoryStats as cat}
							<tr>
								<td>{cat.category}</td>
								<td>{cat.count}</td>
								<td>{cat.avg_severity?.toFixed(1) || 'N/A'}</td>
								<td>{cat.avg_confidence ? (cat.avg_confidence * 100).toFixed(0) + '%' : 'N/A'}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{/if}
	{/if}
</div>

<style>
	.stats-page {
		max-width: 1400px;
	}

	h2 {
		font-size: 1.125rem;
		color: var(--color-text-primary);
		margin-bottom: 1rem;
	}

	.loading {
		text-align: center;
		padding: 3rem;
		color: var(--color-text-secondary);
	}

	.charts-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1.5rem;
		margin-bottom: 2rem;
	}

	.chart-card {
		background-color: var(--color-bg-card);
		border-radius: 8px;
		border: 1px solid var(--color-border);
		padding: 1.5rem;
	}

	.chart-card.wide {
		grid-column: 1 / -1;
	}

	.chart-container {
		height: 250px;
	}

	.table-card {
		background-color: var(--color-bg-card);
		border-radius: 8px;
		border: 1px solid var(--color-border);
		padding: 1.5rem;
		overflow-x: auto;
	}

	.data-table {
		width: 100%;
		border-collapse: collapse;
	}

	.data-table th,
	.data-table td {
		padding: 0.75rem;
		text-align: left;
		border-bottom: 1px solid var(--color-border);
	}

	.data-table th {
		color: var(--color-text-secondary);
		font-weight: 500;
		font-size: 0.875rem;
	}

	.data-table td {
		color: var(--color-text-primary);
		font-size: 0.875rem;
	}

	.data-table tbody tr:hover {
		background-color: var(--color-bg-elevated);
	}
</style>
