<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';

	interface Fact {
		id: number;
		fingerprint: string;
		filename: string;
		fact_summary: string;
		category: string | null;
		identified_crime: string | null;
		severity_score: number;
		confidence: number | null;
		created_at: string;
	}

	let facts = $state<Fact[]>([]);
	let loading = $state(true);
	let filter = $state('');
	let sortBy = $state<'severity' | 'date'>('severity');
	let selectedFact = $state<Fact | null>(null);
	let selectedIds = $state<Set<number>>(new Set());
	let selectAll = $state(false);

	onMount(async () => {
		await loadFacts();
	});

	async function loadFacts() {
		loading = true;
		try {
			facts = await invoke<Fact[]>('search_facts', { query: '*', limit: 500 });
		} catch (e) {
			console.error('Error loading facts:', e);
			facts = [];
		} finally {
			loading = false;
		}
	}

	function toggleSelect(id: number) {
		const newSet = new Set(selectedIds);
		if (newSet.has(id)) {
			newSet.delete(id);
		} else {
			newSet.add(id);
		}
		selectedIds = newSet;
	}

	function toggleSelectAll() {
		if (selectAll) {
			selectedIds = new Set();
			selectAll = false;
		} else {
			selectedIds = new Set(filteredFacts.map(f => f.id));
			selectAll = true;
		}
	}

	function getSeverityColor(score: number): string {
		if (score >= 8) return '#ef4444';
		if (score >= 6) return '#f97316';
		if (score >= 4) return '#eab308';
		return '#4ade80';
	}

	function getCategoryIcon(category: string | null): string {
		switch (category) {
			case 'Financial':
				return 'dollar';
			case 'Legal':
				return 'scale';
			case 'Digital':
				return 'laptop';
			case 'Physical':
				return 'map-pin';
			case 'Verbal':
				return 'mic';
			default:
				return 'file';
		}
	}

	let filteredFacts = $derived(
		facts
			.filter(
				(f) =>
					!filter ||
					f.fact_summary.toLowerCase().includes(filter.toLowerCase()) ||
					f.category?.toLowerCase().includes(filter.toLowerCase()) ||
					f.identified_crime?.toLowerCase().includes(filter.toLowerCase())
			)
			.sort((a, b) => {
				if (sortBy === 'severity') {
					return b.severity_score - a.severity_score;
				}
				return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
			})
	);
</script>

<div class="results">
	<div class="results-header">
		<h1>Results</h1>

		<div class="controls">
			<input type="text" placeholder="Filter facts..." bind:value={filter} class="filter-input" />

			<select bind:value={sortBy} class="sort-select">
				<option value="severity">Sort by Severity</option>
				<option value="date">Sort by Date</option>
			</select>
		</div>
	</div>

	{#if loading}
		<div class="loading">Loading facts...</div>
	{:else if facts.length === 0}
		<div class="empty">
			<svg
				class="empty-icon"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
			>
				<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
				<line x1="9" y1="14" x2="15" y2="14" />
			</svg>
			<p>No facts extracted yet.</p>
			<p class="empty-hint">Run the analysis pipeline to extract facts from your evidence.</p>
		</div>
	{:else}
		<div class="results-grid">
			<div class="facts-list">
				<div class="facts-toolbar">
					<label class="select-all">
						<input
							type="checkbox"
							checked={selectAll}
							onchange={toggleSelectAll}
						/>
						<span>{selectedIds.size} selected</span>
					</label>
					{#if selectedIds.size > 0}
						<div class="bulk-actions">
							<button class="bulk-btn" onclick={() => console.log('Export selected')}>
								Export
							</button>
							<button class="bulk-btn danger" onclick={() => console.log('Delete selected')}>
								Delete
							</button>
						</div>
					{/if}
				</div>

				<div class="facts-count">
					{filteredFacts.length} of {facts.length} facts
				</div>

				{#each filteredFacts as fact}
					<div
						class="fact-card"
						class:selected={selectedFact?.id === fact.id}
					>
						<label class="fact-checkbox" onclick={(e) => e.stopPropagation()}>
							<input
								type="checkbox"
								checked={selectedIds.has(fact.id)}
								onchange={() => toggleSelect(fact.id)}
							/>
						</label>
						<button
							class="fact-content"
							onclick={() => (selectedFact = fact)}
						>
						<div class="fact-header">
							<svg
								class="fact-icon"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
							>
								{#if getCategoryIcon(fact.category) === 'dollar'}
									<line x1="12" y1="1" x2="12" y2="23" /><path
										d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"
									/>
								{:else if getCategoryIcon(fact.category) === 'scale'}
									<path d="M16 3l5 5-5 5" /><path d="M21 8H3" /><path d="M21 16l-5 5-5-5" /><path
										d="M16 21H3"
									/>
								{:else if getCategoryIcon(fact.category) === 'laptop'}
									<rect x="2" y="3" width="20" height="14" rx="2" ry="2" /><line
										x1="2"
										y1="20"
										x2="22"
										y2="20"
									/>
								{:else if getCategoryIcon(fact.category) === 'map-pin'}
									<path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z" /><circle
										cx="12"
										cy="10"
										r="3"
									/>
								{:else if getCategoryIcon(fact.category) === 'mic'}
									<path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" /><path
										d="M19 10v2a7 7 0 0 1-14 0v-2"
									/><line x1="12" y1="19" x2="12" y2="23" /><line x1="8" y1="23" x2="16" y2="23" />
								{:else}
									<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" /><polyline
										points="14 2 14 8 20 8"
									/><line x1="16" y1="13" x2="8" y2="13" /><line x1="16" y1="17" x2="8" y2="17" />
								{/if}
							</svg>
							<span class="fact-filename">{fact.filename}</span>
							<span
								class="fact-severity"
								style="background-color: {getSeverityColor(fact.severity_score)}"
							>
								{fact.severity_score}
							</span>
						</div>
						<div class="fact-summary">{fact.fact_summary}</div>
						{#if fact.identified_crime}
							<div class="fact-crime">{fact.identified_crime}</div>
						{/if}
					</button>
				</div>
				{/each}
			</div>

			{#if selectedFact}
				<div class="fact-detail">
					<h2>Fact Details</h2>

					<div class="detail-row">
						<span class="detail-label">Filename:</span>
						<span class="detail-value">{selectedFact.filename}</span>
					</div>

					<div class="detail-row">
						<span class="detail-label">Category:</span>
						<span class="detail-value">{selectedFact.category || 'Unknown'}</span>
					</div>

					<div class="detail-row">
						<span class="detail-label">Crime:</span>
						<span class="detail-value crime"
							>{selectedFact.identified_crime || 'None identified'}</span
						>
					</div>

					<div class="detail-row">
						<span class="detail-label">Severity:</span>
						<span class="detail-value">
							<span
								class="severity-badge"
								style="background-color: {getSeverityColor(selectedFact.severity_score)}"
							>
								{selectedFact.severity_score}/10
							</span>
						</span>
					</div>

					<div class="detail-row">
						<span class="detail-label">Confidence:</span>
						<span class="detail-value">
							{selectedFact.confidence ? Math.round(selectedFact.confidence * 100) : 'N/A'}%
						</span>
					</div>

					<div class="detail-row">
						<span class="detail-label">Date:</span>
						<span class="detail-value">{selectedFact.created_at}</span>
					</div>

					<div class="detail-section">
						<h3>Summary</h3>
						<p>{selectedFact.fact_summary}</p>
					</div>

					<div class="detail-section">
						<h3>Fingerprint</h3>
						<code class="fingerprint">{selectedFact.fingerprint}</code>
					</div>
				</div>
			{:else}
				<div class="no-selection">
					<p>Select a fact to view details</p>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.results {
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	.results-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.5rem;
	}

	h1 {
		font-size: 1.75rem;
		color: #eaeaea;
	}

	.controls {
		display: flex;
		gap: 0.75rem;
	}

	.filter-input {
		padding: 0.5rem 0.75rem;
		background-color: #16213e;
		border: 1px solid #0f3460;
		border-radius: 6px;
		color: #eaeaea;
		font-size: 0.875rem;
		width: 200px;
	}

	.filter-input:focus {
		outline: none;
		border-color: #e94560;
	}

	.sort-select {
		padding: 0.5rem 0.75rem;
		background-color: #16213e;
		border: 1px solid #0f3460;
		border-radius: 6px;
		color: #eaeaea;
		font-size: 0.875rem;
	}

	.loading,
	.empty {
		text-align: center;
		padding: 3rem;
		color: #9ca3af;
	}

	.empty-icon {
		width: 48px;
		height: 48px;
		color: #6b7280;
		margin-bottom: 1rem;
	}

	.empty-hint {
		font-size: 0.875rem;
		color: #6b7280;
		margin-top: 0.5rem;
	}

	.results-grid {
		display: grid;
		grid-template-columns: 1fr 350px;
		gap: 1.5rem;
		flex: 1;
		min-height: 0;
	}

	.facts-list {
		overflow-y: auto;
		background-color: #16213e;
		border-radius: 8px;
		border: 1px solid #0f3460;
		padding: 1rem;
	}

	.facts-toolbar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem;
		background-color: #1a1a2e;
		border-radius: 6px;
		margin-bottom: 1rem;
	}

	.select-all {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.875rem;
		color: #9ca3af;
		cursor: pointer;
	}

	.select-all input {
		width: 16px;
		height: 16px;
	}

	.bulk-actions {
		display: flex;
		gap: 0.5rem;
	}

	.bulk-btn {
		padding: 0.5rem 1rem;
		background-color: #0f3460;
		border: none;
		border-radius: 4px;
		color: #eaeaea;
		font-size: 0.75rem;
		cursor: pointer;
		transition: all 0.2s;
	}

	.bulk-btn:hover {
		background-color: #e94560;
	}

	.bulk-btn.danger:hover {
		background-color: #ef4444;
	}

	.facts-count {
		font-size: 0.75rem;
		color: #6b7280;
		margin-bottom: 1rem;
	}

	.fact-card {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
		width: 100%;
		text-align: left;
		padding: 1rem;
		background-color: #1a1a2e;
		border: 1px solid #0f3460;
		border-radius: 6px;
		margin-bottom: 0.75rem;
		cursor: pointer;
		transition: all 0.2s;
	}

	.fact-card:hover {
		border-color: #e94560;
	}

	.fact-card.selected {
		border-color: #e94560;
		background-color: #0f3460;
	}

	.fact-checkbox {
		flex-shrink: 0;
		margin-top: 4px;
	}

	.fact-checkbox input {
		width: 18px;
		height: 18px;
		cursor: pointer;
	}

	.fact-content {
		flex: 1;
		background: none;
		border: none;
		padding: 0;
		margin: 0;
		text-align: left;
		cursor: pointer;
	}

	.fact-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.5rem;
	}

	.fact-icon {
		width: 16px;
		height: 16px;
		color: #e94560;
	}

	.fact-filename {
		flex: 1;
		font-size: 0.875rem;
		color: #9ca3af;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.fact-severity {
		font-size: 0.75rem;
		font-weight: 600;
		color: #ffffff;
		padding: 0.125rem 0.5rem;
		border-radius: 4px;
	}

	.fact-summary {
		font-size: 0.875rem;
		color: #eaeaea;
		line-height: 1.4;
		margin-bottom: 0.25rem;
	}

	.fact-crime {
		font-size: 0.75rem;
		color: #e94560;
	}

	.fact-detail {
		background-color: #16213e;
		border-radius: 8px;
		border: 1px solid #0f3460;
		padding: 1.5rem;
		overflow-y: auto;
	}

	.fact-detail h2 {
		font-size: 1.25rem;
		color: #e94560;
		margin-bottom: 1.5rem;
	}

	.detail-row {
		display: flex;
		justify-content: space-between;
		padding: 0.75rem 0;
		border-bottom: 1px solid #0f3460;
	}

	.detail-label {
		color: #9ca3af;
		font-size: 0.875rem;
	}

	.detail-value {
		color: #eaeaea;
		font-size: 0.875rem;
		font-weight: 500;
	}

	.detail-value.crime {
		color: #e94560;
	}

	.severity-badge {
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
		font-size: 0.75rem;
		color: #ffffff;
	}

	.detail-section {
		margin-top: 1.5rem;
	}

	.detail-section h3 {
		font-size: 0.875rem;
		color: #9ca3af;
		margin-bottom: 0.5rem;
	}

	.detail-section p {
		color: #eaeaea;
		font-size: 0.875rem;
		line-height: 1.5;
	}

	.fingerprint {
		display: block;
		padding: 0.75rem;
		background-color: #1a1a2e;
		border-radius: 4px;
		font-family: 'SF Mono', Monaco, monospace;
		font-size: 0.75rem;
		color: #9ca3af;
		word-break: break-all;
	}

	.no-selection {
		display: flex;
		align-items: center;
		justify-content: center;
		background-color: #16213e;
		border-radius: 8px;
		border: 1px solid #0f3460;
		color: #6b7280;
	}
</style>
