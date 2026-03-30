<script lang="ts">
	import { onMount } from 'svelte';

	interface PerformanceMetric {
		name: string;
		value: number;
		unit: string;
		timestamp: number;
	}

	let metrics = $state<PerformanceMetric[]>([]);
	let isProfiling = $state(false);
	let profilingDuration = $state(5000);
	let profilingInterval = $state<ReturnType<typeof setInterval> | null>(null);

	function collectMetric(name: string, value: number, unit: string = 'ms') {
		metrics = [...metrics.slice(-100), { name, value, unit, timestamp: Date.now() }].slice(-100);
	}

	function measurePageLoad() {
		const perf = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming | undefined;
		if (perf) {
			collectMetric('page_load', perf.loadEventEnd - perf.startTime, 'ms');
			collectMetric('dom_content_loaded', perf.domContentLoadedEventEnd - perf.startTime, 'ms');
			collectMetric('ttfb', perf.responseStart - perf.requestStart, 'ms');
			collectMetric('dom_interactive', perf.domInteractive - perf.startTime, 'ms');
		}
	}

	function measureMemory() {
		const memory = (performance as any).memory;
		if (memory) {
			collectMetric('heap_used', memory.usedJSHeapSize / 1024 / 1024, 'MB');
			collectMetric('heap_total', memory.totalJSHeapSize / 1024 / 1024, 'MB');
		}
	}

	function measureFrameRate() {
		let lastTime = performance.now();
		let frameCount = 0;
		let fps = 0;

		function countFrame() {
			frameCount++;
			const currentTime = performance.now();
			if (currentTime - lastTime >= 1000) {
				fps = frameCount;
				collectMetric('fps', fps, 'fps');
				frameCount = 0;
				lastTime = currentTime;
			}
			if (isProfiling) {
				requestAnimationFrame(countFrame);
			}
		}

		requestAnimationFrame(countFrame);
	}

	function startProfiling() {
		metrics = [];
		isProfiling = true;
		measurePageLoad();
		measureMemory();
		measureFrameRate();

		profilingInterval = setInterval(() => {
			measureMemory();
		}, 1000);

		setTimeout(() => {
			stopProfiling();
		}, profilingDuration);
	}

	function stopProfiling() {
		isProfiling = false;
		if (profilingInterval) {
			clearInterval(profilingInterval);
			profilingInterval = null;
		}
	}

	function getMetricStats(name: string) {
		const filtered = metrics.filter((m) => m.name === name);
		if (filtered.length === 0) return null;

		const values = filtered.map((m) => m.value);
		const avg = values.reduce((a, b) => a + b, 0) / values.length;
		const min = Math.min(...values);
		const max = Math.max(...values);

		return { avg: avg.toFixed(2), min: min.toFixed(2), max: max.toFixed(2), count: values.length };
	}

	function measureComponentRender(componentName: string, fn: () => void) {
		const start = performance.now();
		fn();
		const end = performance.now();
		collectMetric(`render_${componentName}`, end - start);
	}

	function exportMetrics() {
		const data = JSON.stringify(metrics, null, 2);
		const blob = new Blob([data], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `performance-metrics-${Date.now()}.json`;
		a.click();
		URL.revokeObjectURL(url);
	}

	onMount(() => {
		measurePageLoad();
		window.addEventListener('load', measurePageLoad);
		return () => window.removeEventListener('load', measurePageLoad);
	});

	export { measureComponentRender, collectMetric };
</script>

<div class="perf-monitor">
	<h3>Performance Monitor</h3>

	<div class="controls">
		<button onclick={startProfiling} disabled={isProfiling}>
			{isProfiling ? `Profiling... ${Math.ceil(profilingDuration / 1000)}s` : 'Start Profiling'}
		</button>
		<button onclick={() => (metrics = [])}>Clear</button>
		<button onclick={exportMetrics} disabled={metrics.length === 0}>Export</button>
	</div>

	<div class="stats">
		<div class="stat-card">
			<span class="label">Page Load</span>
			<span class="value">{getMetricStats('page_load')?.avg || 'N/A'} ms</span>
		</div>
		<div class="stat-card">
			<span class="label">TTFB</span>
			<span class="value">{getMetricStats('ttfb')?.avg || 'N/A'} ms</span>
		</div>
		<div class="stat-card">
			<span class="label">Avg FPS</span>
			<span class="value">{getMetricStats('fps')?.avg || 'N/A'}</span>
		</div>
		<div class="stat-card">
			<span class="label">Heap Used</span>
			<span class="value">{getMetricStats('heap_used')?.avg || 'N/A'} MB</span>
		</div>
	</div>

	{#if metrics.length > 0}
		<div class="metrics-table">
			<table>
				<thead>
					<tr>
						<th>Metric</th>
						<th>Count</th>
						<th>Avg</th>
						<th>Min</th>
						<th>Max</th>
					</tr>
				</thead>
				<tbody>
					{#each [...new Set(metrics.map((m) => m.name))] as name}
						{@const stats = getMetricStats(name)}
						{#if stats}
							<tr>
								<td>{name}</td>
								<td>{stats.count}</td>
								<td>{stats.avg}</td>
								<td>{stats.min}</td>
								<td>{stats.max}</td>
							</tr>
						{/if}
					{/each}
				</tbody>
			</table>
		</div>
	{/if}

	{#if isProfiling}
		<div class="profiling-indicator">
			<span class="pulse"></span>
			Profiling in progress...
		</div>
	{/if}
</div>

<style>
	.perf-monitor {
		position: fixed;
		bottom: 10px;
		right: 10px;
		background: rgba(0, 0, 0, 0.9);
		color: #fff;
		padding: 1rem;
		border-radius: 8px;
		font-family: monospace;
		font-size: 12px;
		width: 350px;
		z-index: 9999;
	}

	h3 {
		margin: 0 0 0.75rem 0;
		font-size: 14px;
		color: #4a9eff;
	}

	.controls {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}

	.controls button {
		flex: 1;
		padding: 0.5rem;
		background: #333;
		color: #fff;
		border: 1px solid #555;
		border-radius: 4px;
		cursor: pointer;
		font-size: 11px;
	}

	.controls button:hover:not(:disabled) {
		background: #444;
	}

	.controls button:disabled {
		opacity: 0.5;
	}

	.stats {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}

	.stat-card {
		background: #222;
		padding: 0.5rem;
		border-radius: 4px;
		text-align: center;
	}

	.stat-card .label {
		display: block;
		font-size: 10px;
		color: #888;
	}

	.stat-card .value {
		display: block;
		font-size: 14px;
		font-weight: bold;
		color: #4ade80;
	}

	.metrics-table {
		max-height: 200px;
		overflow-y: auto;
	}

	table {
		width: 100%;
		border-collapse: collapse;
	}

	th,
	td {
		padding: 0.25rem 0.5rem;
		text-align: left;
		border-bottom: 1px solid #333;
	}

	th {
		color: #888;
		font-weight: normal;
	}

	.profiling-indicator {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: 0.75rem;
		color: #f97316;
	}

	.pulse {
		width: 8px;
		height: 8px;
		background: #f97316;
		border-radius: 50%;
		animation: pulse 1s infinite;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.3;
		}
	}
</style>
