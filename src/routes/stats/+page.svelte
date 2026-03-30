<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';
	import { Chart, registerables } from 'chart.js';

	Chart.register(...registerables);

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

	let severityChartEl: HTMLCanvasElement;
	let categoryChartEl: HTMLCanvasElement;
	let entityChartEl: HTMLCanvasElement;

	let severityChart: Chart | null = null;
	let categoryChart: Chart | null = null;
	let entityChart: Chart | null = null;

	onMount(async () => {
		await loadStats();
		initCharts();
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
					datasets: [{
						label: 'Facts by Severity',
						data: [],
						backgroundColor: [
							'#4ade80',
							'#eab308',
							'#f97316',
							'#ef4444',
							'#dc2626'
						]
					}]
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
							ticks: { color: '#9ca3af' },
							grid: { color: '#0f3460' }
						},
						x: {
							ticks: { color: '#9ca3af' },
							grid: { color: '#0f3460' }
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
					datasets: [{
						data: [],
						backgroundColor: [
							'#e94560',
							'#3b82f6',
							'#10b981',
							'#f59e0b',
							'#8b5cf6',
							'#ec4899'
						]
					}]
				},
				options: {
					responsive: true,
					maintainAspectRatio: false,
					plugins: {
						legend: {
							position: 'right',
							labels: { color: '#9ca3af' }
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
					datasets: [{
						label: 'Entity Occurrences',
						data: [],
						backgroundColor: '#e94560'
					}]
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
							ticks: { color: '#9ca3af' },
							grid: { color: '#0f3460' }
						},
						y: {
							ticks: { color: '#9ca3af' },
							grid: { color: '#0f3460' }
						}
					}
				}
			});
		}
	}

	$effect(() => {
		if (severityStats.length > 0 && severityChart) {
			severityChart.data.labels = severityStats.map(s => `Severity ${s.severity}`);
			severityChart.data.datasets[0].data = severityStats.map(s => s.count);
			severityChart.update();
		}

		if (categoryStats.length > 0 && categoryChart) {
			categoryChart.data.labels = categoryStats.map(c => c.category);
			categoryChart.data.datasets[0].data = categoryStats.map(c => c.count);
			categoryChart.update();
		}

		if (topEntities.length > 0 && entityChart) {
			entityChart.data.labels = topEntities.map(e => e.value.substring(0, 20));
			entityChart.data.datasets[0].data = topEntities.map(e => e.occurrence_count);
			entityChart.update();
		}
	});
</script>

<div class="stats-page">
	<h1>Statistics</h1>

	{#if loading}
		<div class="loading">Loading statistics...</div>
	{:else}
		{#if overallStats}
			<div class="overview-cards">
				<div class="overview-card">
					<div class="card-value">{overallStats.total_facts}</div>
					<div class="card-label">Total Facts</div>
				</div>
				<div class="overview-card">
					<div class="card-value">{overallStats.avg_severity?.toFixed(1) || '0'}</div>
					<div class="card-label">Avg Severity</div>
				</div>
				<div class="overview-card">
					<div class="card-value">{overallStats.avg_confidence ? (overallStats.avg_confidence * 100).toFixed(0) + '%' : 'N/A'}</div>
					<div class="card-label">Avg Confidence</div>
				</div>
				<div class="overview-card">
					<div class="card-value">{overallStats.total_entities}</div>
					<div class="card-label">Entity Mentions</div>
				</div>
				<div class="overview-card">
					<div class="card-value">{overallStats.unique_entities}</div>
					<div class="card-label">Unique Entities</div>
				</div>
				<div class="overview-card">
					<div class="card-value">{overallStats.total_chains}</div>
					<div class="card-label">Evidence Chains</div>
				</div>
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

	h1 {
		font-size: 1.75rem;
		color: #eaeaea;
		margin-bottom: 1.5rem;
	}

	h2 {
		font-size: 1.125rem;
		color: #eaeaea;
		margin-bottom: 1rem;
	}

	.loading {
		text-align: center;
		padding: 3rem;
		color: #9ca3af;
	}

	.overview-cards {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
		gap: 1rem;
		margin-bottom: 2rem;
	}

	.overview-card {
		padding: 1.25rem;
		background-color: #16213e;
		border-radius: 8px;
		border: 1px solid #0f3460;
		text-align: center;
	}

	.card-value {
		font-size: 2rem;
		font-weight: 700;
		color: #e94560;
	}

	.card-label {
		font-size: 0.875rem;
		color: #9ca3af;
		margin-top: 0.25rem;
	}

	.charts-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1.5rem;
		margin-bottom: 2rem;
	}

	.chart-card {
		background-color: #16213e;
		border-radius: 8px;
		border: 1px solid #0f3460;
		padding: 1.5rem;
	}

	.chart-card.wide {
		grid-column: 1 / -1;
	}

	.chart-container {
		height: 250px;
	}

	.table-card {
		background-color: #16213e;
		border-radius: 8px;
		border: 1px solid #0f3460;
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
		border-bottom: 1px solid #0f3460;
	}

	.data-table th {
		color: #9ca3af;
		font-weight: 500;
		font-size: 0.875rem;
	}

	.data-table td {
		color: #eaeaea;
		font-size: 0.875rem;
	}

	.data-table tbody tr:hover {
		background-color: #0f3460;
	}
</style>
