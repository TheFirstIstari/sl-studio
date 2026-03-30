<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { open } from '@tauri-apps/plugin-dialog';

	interface ProjectSummary {
		name: string;
		path: string;
		fact_count: number;
		entity_count: number;
		timeline_count: number;
	}

	interface EntityOverlap {
		entity_value: string;
		entity_type: string;
		count_project1: number;
		count_project2: number;
	}

	interface TimelineCorrelation {
		correlation_score: number;
		aligned_events: number;
		project1_date_range: [string, string];
		project2_date_range: [string, string];
	}

	interface ProjectComparison {
		project1_name: string;
		project2_name: string;
		entity_overlap: EntityOverlap[];
		common_entities: Array<{
			entity_id: number;
			entity_type: string;
			value: string;
			occurrence_count: number;
		}>;
		timeline_correlation: TimelineCorrelation;
		fact_similarity: number;
	}

	let currentProject = $state<ProjectSummary | null>(null);
	let project2Path = $state('');
	let comparison = $state<ProjectComparison | null>(null);
	let isLoading = $state(false);
	let error = $state('');

	async function loadCurrentProject() {
		try {
			currentProject = await invoke<ProjectSummary>('get_project_summary');
		} catch (e) {
			error = `Error loading project: ${e}`;
		}
	}

	async function selectProject2() {
		const selected = await open({
			directory: true,
			multiple: false,
			title: 'Select Project to Compare'
		});
		if (selected) {
			project2Path = selected as string;
		}
	}

	async function runComparison() {
		if (!project2Path) {
			error = 'Please select a project to compare';
			return;
		}

		isLoading = true;
		error = '';

		try {
			comparison = await invoke<ProjectComparison>('compare_projects', {
				project2Path
			});
		} catch (e) {
			error = `Error comparing projects: ${e}`;
		} finally {
			isLoading = false;
		}
	}

	$effect(() => {
		loadCurrentProject();
	});
</script>

<div class="compare-page">
	<h1>Compare Projects</h1>

	{#if error}
		<div class="error-message">{error}</div>
	{/if}

	<div class="project-selection">
		<div class="project-card">
			<h2>Current Project</h2>
			{#if currentProject}
				<div class="project-info">
					<p><strong>Name:</strong> {currentProject.name}</p>
					<p><strong>Facts:</strong> {currentProject.fact_count}</p>
					<p><strong>Entities:</strong> {currentProject.entity_count}</p>
					<p><strong>Timeline Events:</strong> {currentProject.timeline_count}</p>
				</div>
			{:else}
				<p>Loading...</p>
			{/if}
		</div>

		<div class="vs-divider">VS</div>

		<div class="project-card">
			<h2>Compare With</h2>
			<div class="project-info">
				<button class="select-btn" onclick={selectProject2}>
					{project2Path ? 'Change Project' : 'Select Project'}
				</button>
				{#if project2Path}
					<p class="selected-path">{project2Path}</p>
				{/if}
			</div>
		</div>
	</div>

	<button class="compare-btn" onclick={runComparison} disabled={isLoading || !project2Path}>
		{isLoading ? 'Comparing...' : 'Compare Projects'}
	</button>

	{#if comparison}
		<div class="comparison-results">
			<h2>Comparison Results</h2>

			<div class="similarity-score">
				<h3>Similarity Score</h3>
				<div class="score-bar">
					<div class="score-fill" style="width: {comparison.fact_similarity * 100}%"></div>
				</div>
				<p>{(comparison.fact_similarity * 100).toFixed(1)}%</p>
			</div>

			<div class="timeline-correlation">
				<h3>Timeline Correlation</h3>
				<p>
					Correlation Score: <strong
						>{(comparison.timeline_correlation.correlation_score * 100).toFixed(1)}%</strong
					>
				</p>
				<p>Aligned Events: <strong>{comparison.timeline_correlation.aligned_events}</strong></p>
				<p class="date-range">
					Project 1: {comparison.timeline_correlation.project1_date_range[0]} to {comparison
						.timeline_correlation.project1_date_range[1]}
				</p>
				<p class="date-range">
					Project 2: {comparison.timeline_correlation.project2_date_range[0]} to {comparison
						.timeline_correlation.project2_date_range[1]}
				</p>
			</div>

			{#if comparison.entity_overlap.length > 0}
				<div class="entity-overlap">
					<h3>Entity Overlap ({comparison.entity_overlap.length} shared)</h3>
					<table>
						<thead>
							<tr>
								<th>Entity</th>
								<th>Type</th>
								<th>Project 1</th>
								<th>Project 2</th>
							</tr>
						</thead>
						<tbody>
							{#each comparison.entity_overlap.slice(0, 20) as entity}
								<tr>
									<td>{entity.entity_value}</td>
									<td>{entity.entity_type}</td>
									<td>{entity.count_project1}</td>
									<td>{entity.count_project2}</td>
								</tr>
							{/each}
						</tbody>
					</table>
					{#if comparison.entity_overlap.length > 20}
						<p class="more">...and {comparison.entity_overlap.length - 20} more</p>
					{/if}
				</div>
			{:else}
				<p class="no-overlap">No shared entities found</p>
			{/if}
		</div>
	{/if}
</div>

<style>
	.compare-page {
		padding: 2rem;
		max-width: 1000px;
	}

	h1 {
		margin-bottom: 1.5rem;
		font-size: 1.75rem;
	}

	h2 {
		font-size: 1.25rem;
		margin-bottom: 1rem;
	}

	h3 {
		font-size: 1.1rem;
		margin-bottom: 0.75rem;
	}

	.error-message {
		background: #f44336;
		color: white;
		padding: 0.75rem;
		border-radius: 4px;
		margin-bottom: 1rem;
	}

	.project-selection {
		display: flex;
		gap: 2rem;
		align-items: stretch;
		margin-bottom: 2rem;
	}

	.project-card {
		flex: 1;
		background: var(--card-bg, #1e1e1e);
		border-radius: 8px;
		padding: 1.5rem;
	}

	.project-info p {
		margin: 0.5rem 0;
	}

	.vs-divider {
		display: flex;
		align-items: center;
		font-weight: bold;
		color: #666;
	}

	.select-btn {
		padding: 0.75rem 1.5rem;
		background: #4a9eff;
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		margin-bottom: 1rem;
	}

	.select-btn:hover {
		background: #3a8eef;
	}

	.selected-path {
		font-size: 0.875rem;
		color: #888;
		word-break: break-all;
	}

	.compare-btn {
		width: 100%;
		padding: 1rem;
		background: #4caf50;
		color: white;
		border: none;
		border-radius: 4px;
		font-size: 1.1rem;
		cursor: pointer;
		margin-bottom: 2rem;
	}

	.compare-btn:hover:not(:disabled) {
		background: #43a047;
	}

	.compare-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.comparison-results {
		background: var(--card-bg, #1e1e1e);
		border-radius: 8px;
		padding: 1.5rem;
	}

	.similarity-score {
		margin-bottom: 1.5rem;
	}

	.score-bar {
		height: 24px;
		background: #333;
		border-radius: 12px;
		overflow: hidden;
		margin: 0.5rem 0;
	}

	.score-fill {
		height: 100%;
		background: linear-gradient(90deg, #4a9eff, #4caf50);
		transition: width 0.3s ease;
	}

	.timeline-correlation {
		margin-bottom: 1.5rem;
		padding: 1rem;
		background: #252525;
		border-radius: 4px;
	}

	.date-range {
		font-size: 0.875rem;
		color: #888;
	}

	.entity-overlap {
		margin-top: 1rem;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		margin-top: 0.5rem;
	}

	th,
	td {
		padding: 0.75rem;
		text-align: left;
		border-bottom: 1px solid #444;
	}

	th {
		background: #252525;
		font-weight: 600;
	}

	.no-overlap {
		color: #888;
		font-style: italic;
	}

	.more {
		margin-top: 0.5rem;
		color: #888;
		font-size: 0.875rem;
	}
</style>
