<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount, onDestroy } from 'svelte';
	import { listen } from '@tauri-apps/api/event';

	// Use shared stores
	import { stats, workflow, refreshStats, refreshWorkflow } from '$lib/stores/app';
	import { PageHeader, FilterBar, Modal } from '$lib/components';

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

	interface HistoryState {
		filter: string;
		sortBy: 'severity' | 'date';
		selectedIds: number[];
		// Facet filters
		categories: string[];
		minSeverity: number;
		maxSeverity: number;
		startDate: string;
		endDate: string;
		minConfidence: number;
	}

	const CATEGORIES = ['Financial', 'Legal', 'Digital', 'Physical', 'Verbal'];

	let facts = $state<Fact[]>([]);
	let loading = $state(true);
	let error = $state('');
	let filter = $state('');
	let sortBy = $state<'severity' | 'date'>('severity');
	let selectedFact = $state<Fact | null>(null);
	let selectedIds = $state(new Set<number>());
	let selectAll = $state(false);

	// Facet filter state
	let selectedCategories = $state<string[]>([]);
	let minSeverity = $state(0);
	let maxSeverity = $state(10);
	let startDate = $state('');
	let endDate = $state('');
	let minConfidence = $state(0);

	let history = $state<HistoryState[]>([]);
	let historyIndex = $state(-1);
	let canUndo = $derived(historyIndex > 0);
	let canRedo = $derived(historyIndex < history.length - 1);

	// FR-FACET-004: saved facet presets
	interface FacetPreset {
		id: number;
		page: string;
		name: string;
		state_json: string;
		updated_at: string | null;
	}
	let presets = $state<FacetPreset[]>([]);
	let showSavePreset = $state(false);
	let newPresetName = $state('');

	async function loadPresets() {
		try {
			presets = await invoke<FacetPreset[]>('list_facet_presets', { page: 'results' });
		} catch (e) {
			console.error('Failed to load presets:', e);
		}
	}

	async function savePreset() {
		const name = newPresetName.trim();
		if (!name) return;
		try {
			const state: HistoryState = {
				filter,
				sortBy,
				selectedIds: [],
				categories: selectedCategories,
				minSeverity,
				maxSeverity,
				startDate,
				endDate,
				minConfidence
			};
			await invoke('save_facet_preset', {
				page: 'results',
				name,
				stateJson: JSON.stringify(state)
			});
			newPresetName = '';
			showSavePreset = false;
			await loadPresets();
		} catch (e) {
			console.error('Save preset failed:', e);
			error = String(e);
		}
	}

	function applyPreset(p: FacetPreset) {
		try {
			const s: HistoryState = JSON.parse(p.state_json);
			filter = s.filter ?? '';
			sortBy = s.sortBy ?? 'severity';
			selectedCategories = s.categories ?? [];
			minSeverity = s.minSeverity ?? 0;
			maxSeverity = s.maxSeverity ?? 10;
			startDate = s.startDate ?? '';
			endDate = s.endDate ?? '';
			minConfidence = s.minConfidence ?? 0;
			saveToHistory();
		} catch (e) {
			console.error('Apply preset failed:', e);
			error = String(e);
		}
	}

	async function deletePreset(id: number) {
		if (!confirm('Delete this preset?')) return;
		try {
			await invoke('delete_facet_preset', { presetId: id });
			await loadPresets();
		} catch (e) {
			console.error('Delete preset failed:', e);
			error = String(e);
		}
	}

	let activeFilterCount = $derived.by(() => {
		let count = 0;
		if (filter) count++;
		if (selectedCategories.length > 0) count++;
		if (minSeverity > 0) count++;
		if (maxSeverity < 10) count++;
		if (startDate) count++;
		if (endDate) count++;
		if (minConfidence > 0) count++;
		return count;
	});

	// Calculate facet counts
	let categoryCounts = $derived.by(() => {
		const counts: Record<string, number> = {};
		CATEGORIES.forEach((cat) => (counts[cat] = 0));
		facts.forEach((f) => {
			if (f.category && counts[f.category] !== undefined) {
				counts[f.category]++;
			}
		});
		return counts;
	});

	let severityCounts = $derived.by(() => {
		const counts: Record<string, number> = { low: 0, medium: 0, high: 0 };
		facts.forEach((f) => {
			if (f.severity_score >= 8) counts.high++;
			else if (f.severity_score >= 4) counts.medium++;
			else counts.low++;
		});
		return counts;
	});

	let confidenceRanges = $derived.by(() => {
		return {
			high: facts.filter((f) => (f.confidence ?? 0) >= 80).length,
			medium: facts.filter((f) => (f.confidence ?? 0) >= 50 && (f.confidence ?? 0) < 80).length,
			low: facts.filter((f) => (f.confidence ?? 0) < 50).length
		};
	});

	function saveToHistory() {
		const state: HistoryState = {
			filter,
			sortBy,
			selectedIds: Array.from(selectedIds),
			categories: selectedCategories,
			minSeverity,
			maxSeverity,
			startDate,
			endDate,
			minConfidence
		};
		history = [...history.slice(0, historyIndex + 1), state];
		historyIndex = history.length - 1;
	}

	function undo() {
		if (canUndo) {
			historyIndex--;
			const state = history[historyIndex];
			filter = state.filter;
			sortBy = state.sortBy;
			selectedIds = new Set(state.selectedIds);
			selectedCategories = state.categories;
			minSeverity = state.minSeverity;
			maxSeverity = state.maxSeverity;
			startDate = state.startDate;
			endDate = state.endDate;
			minConfidence = state.minConfidence;
		}
	}

	function redo() {
		if (canRedo) {
			historyIndex++;
			const state = history[historyIndex];
			filter = state.filter;
			sortBy = state.sortBy;
			selectedIds = new Set(state.selectedIds);
			selectedCategories = state.categories;
			minSeverity = state.minSeverity;
			maxSeverity = state.maxSeverity;
			startDate = state.startDate;
			endDate = state.endDate;
			minConfidence = state.minConfidence;
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if ((event.metaKey || event.ctrlKey) && event.key === 'z') {
			event.preventDefault();
			if (event.shiftKey) {
				redo();
			} else {
				undo();
			}
		}
		if ((event.metaKey || event.ctrlKey) && event.key === 'y') {
			event.preventDefault();
			redo();
		}
	}

	// Initialize - use stores for state (already initialized in +layout.svelte)
	async function initialize() {
		await loadFacts();
		saveToHistory();
	}

	let pollInterval: ReturnType<typeof setInterval>;
	let unlistenAnalysis: (() => void) | null = null;

	onMount(async () => {
		initialize();
		await loadPresets();
		window.addEventListener('keydown', handleKeydown);

		// Use stores for workflow - refresh periodically
		pollInterval = setInterval(refreshWorkflow, 2000);

		// Listen for analysis progress updates
		unlistenAnalysis = await listen('analysis_progress', () => {
			refreshWorkflow();
		});
	});

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown);
		if (pollInterval) clearInterval(pollInterval);
		if (unlistenAnalysis) unlistenAnalysis();
	});

	async function loadFacts() {
		loading = true;
		error = '';
		try {
			facts = await invoke<Fact[]>('search_facts', { query: 'a', limit: 500 });
		} catch (e) {
			console.error('Error loading facts:', e);
			error = `Failed to load facts: ${e}`;
			facts = [];
		} finally {
			loading = false;
		}
	}

	function toggleSelect(id: number) {
		// Svelte 5 reactive state on a Set tracks mutations directly,
		// so no need to allocate a new Set on every toggle.
		if (selectedIds.has(id)) {
			selectedIds.delete(id);
		} else {
			selectedIds.add(id);
		}
		saveToHistory();
	}

	function toggleSelectAll() {
		if (selectAll) {
			selectedIds = new Set();
			selectAll = false;
		} else {
			selectedIds = new Set(filteredFacts.map((f) => f.id));
			selectAll = true;
		}
		saveToHistory();
	}

	function onFilterChange() {
		saveToHistory();
	}

	function onSortChange() {
		saveToHistory();
	}

	function getSeverityColor(score: number): string {
		if (score >= 8) return '#ef4444';
		if (score >= 6) return '#f97316';
		if (score >= 4) return '#eab308';
		return '#4ade80';
	}

	function getCategoryIcon(category: string | null): string {
		if (category === 'Financial') return 'dollar';
		if (category === 'Legal') return 'scale';
		if (category === 'Digital') return 'laptop';
		if (category === 'Physical') return 'map-pin';
		if (category === 'Verbal') return 'mic';
		return 'file';
	}

	let filteredFacts = $derived(
		facts
			.filter((f) => {
				// Text filter
				if (
					filter &&
					!f.fact_summary.toLowerCase().includes(filter.toLowerCase()) &&
					!f.category?.toLowerCase().includes(filter.toLowerCase()) &&
					!f.identified_crime?.toLowerCase().includes(filter.toLowerCase())
				) {
					return false;
				}
				// Category filter (AND logic - must match any selected)
				if (selectedCategories.length > 0) {
					if (!f.category || !selectedCategories.includes(f.category)) {
						return false;
					}
				}
				// Severity range filter
				if (f.severity_score < minSeverity || f.severity_score > maxSeverity) {
					return false;
				}
				// Date range filter
				if (startDate) {
					const factDate = new Date(f.created_at);
					const start = new Date(startDate);
					if (factDate < start) return false;
				}
				if (endDate) {
					const factDate = new Date(f.created_at);
					const end = new Date(endDate);
					if (factDate > end) return false;
				}
				// Confidence filter
				if (minConfidence > 0 && (f.confidence ?? 0) < minConfidence / 100) {
					return false;
				}
				return true;
			})
			.sort((a, b) => {
				if (sortBy === 'severity') {
					return b.severity_score - a.severity_score;
				}
				return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
			})
	);

	function toggleCategory(category: string) {
		if (selectedCategories.includes(category)) {
			selectedCategories = selectedCategories.filter((c) => c !== category);
		} else {
			selectedCategories = [...selectedCategories, category];
		}
		saveToHistory();
	}

	function clearAllFilters() {
		filter = '';
		selectedCategories = [];
		minSeverity = 0;
		maxSeverity = 10;
		startDate = '';
		endDate = '';
		minConfidence = 0;
		saveToHistory();
	}
</script>

<div class="results page">
	<PageHeader title="Results" subtitle="Browse, filter, and act on extracted facts">
		{#snippet actions()}
			<button class="btn ghost sm" onclick={undo} disabled={!canUndo} title="Undo (Ctrl+Z)">
				← Undo
			</button>
			<button class="btn ghost sm" onclick={redo} disabled={!canRedo} title="Redo (Ctrl+Shift+Z)">
				Redo →
			</button>
			{#if activeFilterCount > 0}
				<button class="btn sm danger" onclick={clearAllFilters}>
					Clear {activeFilterCount} filter{activeFilterCount === 1 ? '' : 's'}
				</button>
			{/if}
		{/snippet}
	</PageHeader>

	{#if error}
		<div class="error-banner" role="alert">
			<span>{error}</span>
		</div>
	{/if}

	<FilterBar bind:search={filter} placeholder="Filter facts...">
		{#snippet extras()}
			<select bind:value={sortBy} onchange={onSortChange} class="filter-select">
				<option value="severity">Sort by severity</option>
				<option value="date">Sort by date</option>
			</select>
			<button
				class="btn sm"
				onclick={() => (showSavePreset = true)}
				disabled={activeFilterCount === 0}
				title="Save current filters as a preset"
			>
				Save preset
			</button>
		{/snippet}
	</FilterBar>

	{#if presets.length > 0}
		<div class="preset-bar">
			<span class="preset-label">Presets:</span>
			{#each presets as p (p.id)}
				<div class="preset-chip">
					<button class="preset-apply" onclick={() => applyPreset(p)} title="Apply preset">
						{p.name}
					</button>
					<button
						class="preset-delete"
						onclick={() => deletePreset(p.id)}
						aria-label="Delete preset"
						title="Delete preset"
					>
						×
					</button>
				</div>
			{/each}
		</div>
	{/if}

	<Modal
		open={showSavePreset}
		title="Save filter preset"
		size="sm"
		onclose={() => (showSavePreset = false)}
	>
		{#snippet body()}
			<label class="form-label">
				Preset name
				<input
					type="text"
					bind:value={newPresetName}
					placeholder="e.g. High-severity financial"
					class="form-input"
					autofocus
				/>
			</label>
			<p class="form-hint">Existing presets with the same name will be overwritten.</p>
		{/snippet}
		{#snippet footer()}
			<button class="btn ghost" onclick={() => (showSavePreset = false)}>Cancel</button>
			<button class="btn primary" onclick={savePreset} disabled={!newPresetName.trim()}>
				Save
			</button>
		{/snippet}
	</Modal>

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
		<!-- Faceted Filters Panel -->
		<div class="filters-panel">
			<div class="filter-section">
				<h3>Category</h3>
				<div class="filter-options">
					{#each CATEGORIES as category}
						<label class="filter-option">
							<input
								type="checkbox"
								checked={selectedCategories.includes(category)}
								onchange={() => toggleCategory(category)}
							/>
							<span class="option-label">{category}</span>
							<span class="option-count">({categoryCounts[category]})</span>
						</label>
					{/each}
				</div>
			</div>

			<div class="filter-section">
				<h3>Severity</h3>
				<div class="severity-range">
					<div class="range-inputs">
						<label>
							<span>Min</span>
							<input
								type="number"
								min="0"
								max="10"
								bind:value={minSeverity}
								onchange={onFilterChange}
								class="range-input"
							/>
						</label>
						<span class="range-separator">to</span>
						<label>
							<span>Max</span>
							<input
								type="number"
								min="0"
								max="10"
								bind:value={maxSeverity}
								onchange={onFilterChange}
								class="range-input"
							/>
						</label>
					</div>
					<div class="severity-badges">
						<span class="severity-badge low">Low ({severityCounts.low})</span>
						<span class="severity-badge medium">Med ({severityCounts.medium})</span>
						<span class="severity-badge high">High ({severityCounts.high})</span>
					</div>
				</div>
			</div>

			<div class="filter-section">
				<h3>Date Range</h3>
				<div class="date-inputs">
					<label class="date-field">
						<span>From</span>
						<input
							type="date"
							bind:value={startDate}
							onchange={onFilterChange}
							class="date-input"
						/>
					</label>
					<label class="date-field">
						<span>To</span>
						<input type="date" bind:value={endDate} onchange={onFilterChange} class="date-input" />
					</label>
				</div>
			</div>

			<div class="filter-section">
				<h3>Min Confidence: {minConfidence}%</h3>
				<div class="confidence-slider">
					<input
						type="range"
						min="0"
						max="100"
						bind:value={minConfidence}
						onchange={onFilterChange}
						class="slider"
					/>
					<div class="confidence-badges">
						<span class="conf-badge low">Low ({confidenceRanges.low})</span>
						<span class="conf-badge medium">Med ({confidenceRanges.medium})</span>
						<span class="conf-badge high">High ({confidenceRanges.high})</span>
					</div>
				</div>
			</div>
		</div>

		<div class="results-grid">
			<div class="facts-list">
				<div class="facts-toolbar">
					<label class="select-all">
						<input type="checkbox" checked={selectAll} onchange={toggleSelectAll} />
						<span>{selectedIds.size} selected</span>
					</label>
					{#if selectedIds.size > 0}
						<div class="bulk-actions">
							<button
								class="bulk-btn"
								onclick={async () => {
									if (selectedIds.size === 0) return;
									try {
										const ids = Array.from(selectedIds);
										const result = await invoke<string>('export_facts_json', {
											minWeight: 0.0,
											limit: ids.length,
											categories: null,
											startDate: null,
											endDate: null
										});
										// Create download
										const blob = new Blob([result], { type: 'application/json' });
										const url = URL.createObjectURL(blob);
										const a = document.createElement('a');
										a.href = url;
										a.download = `facts-export-${Date.now()}.json`;
										a.click();
										URL.revokeObjectURL(url);
									} catch (e) {
										console.error('Export failed:', e);
									}
								}}
							>
								Export
							</button>
							<button
								class="bulk-btn danger"
								onclick={async () => {
									if (selectedIds.size === 0) return;
									if (!confirm(`Delete ${selectedIds.size} selected fact(s)?`)) return;
									try {
										const ids = Array.from(selectedIds);
										const count = await invoke<number>('delete_facts', { ids });
										// Refresh facts
										await loadFacts();
										selectedIds = new Set();
										selectAll = false;
									} catch (e) {
										console.error('Delete failed:', e);
									}
								}}
							>
								Delete
							</button>
						</div>
					{/if}
				</div>

				<div class="facts-count">
					{filteredFacts.length} of {facts.length} facts
				</div>

				{#each filteredFacts as fact}
					<div class="fact-card" class:selected={selectedFact?.id === fact.id}>
						<label class="fact-checkbox" onclick={(e) => e.stopPropagation()}>
							<input
								type="checkbox"
								checked={selectedIds.has(fact.id)}
								onchange={() => toggleSelect(fact.id)}
							/>
						</label>
						<button class="fact-content" onclick={() => (selectedFact = fact)}>
							<div class="fact-header">
								<svg
									class="fact-icon"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
								>
									{#if getCategoryIcon(fact.category) === 'dollar'}
										<line x1="12" y1="1" x2="12" y2="23" />
										<path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6" />
									{:else if getCategoryIcon(fact.category) === 'scale'}
										<path d="M16 3l5 5-5 5" />
										<path d="M21 8H3" />
										<path d="M21 16l-5 5-5-5" />
										<path d="M16 21H3" />
									{:else if getCategoryIcon(fact.category) === 'laptop'}
										<rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
										<line x1="2" y1="20" x2="22" y2="20" />
									{:else if getCategoryIcon(fact.category) === 'map-pin'}
										<path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z" />
										<circle cx="12" cy="10" r="3" />
									{:else if getCategoryIcon(fact.category) === 'mic'}
										<path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" />
										<path d="M19 10v2a7 7 0 0 1-14 0v-2" />
										<line x1="12" y1="19" x2="12" y2="23" />
										<line x1="8" y1="23" x2="16" y2="23" />
									{:else}
										<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
										<polyline points="14 2 14 8 20 8" />
										<line x1="16" y1="13" x2="8" y2="13" />
										<line x1="16" y1="17" x2="8" y2="17" />
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
	.error-banner {
		background: #fee2e2;
		color: #991b1b;
		padding: 0.5rem 1rem;
		border-radius: 6px;
		font-size: 0.875rem;
		width: 100%;
	}

	.results {
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	/* Header, filter input, and sort select now come from PageHeader,
	   FilterBar, and theme.css. Page-specific styles continue below. */

	.filter-select {
		background: var(--color-bg-input);
		border: 1px solid var(--color-border);
		color: var(--color-text-primary);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-md);
		font-size: var(--text-sm);
	}

	/* FR-FACET-004 preset bar */
	.preset-bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
		margin-bottom: var(--space-4);
	}

	.preset-label {
		color: var(--color-text-muted);
		font-size: var(--text-sm);
		margin-right: var(--space-1);
	}

	.preset-chip {
		display: inline-flex;
		align-items: stretch;
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: 999px;
		overflow: hidden;
	}

	.preset-apply {
		background: transparent;
		border: none;
		color: var(--color-text-primary);
		padding: var(--space-1) var(--space-3);
		font-size: var(--text-sm);
		cursor: pointer;
	}

	.preset-apply:hover {
		background: var(--color-accent);
		color: white;
	}

	.preset-delete {
		background: transparent;
		border: none;
		border-left: 1px solid var(--color-border);
		color: var(--color-text-muted);
		padding: var(--space-1) var(--space-3);
		font-size: var(--text-base);
		cursor: pointer;
		line-height: 1;
	}

	.preset-delete:hover {
		color: var(--color-severity-high);
	}

	.form-label {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		font-size: var(--text-sm);
		color: var(--color-text-secondary);
	}

	.form-input {
		background: var(--color-bg-input);
		border: 1px solid var(--color-border);
		color: var(--color-text-primary);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-md);
		font-size: var(--text-sm);
	}

	.form-input:focus {
		outline: none;
		border-color: var(--color-accent);
	}

	.form-hint {
		color: var(--color-text-muted);
		font-size: var(--text-xs);
		margin: var(--space-2) 0 0;
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

	/* Make filters panel full width above results grid */
	.filters-panel {
		width: 100%;
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

	/* Filter panel for facets (header buttons now come from PageHeader). */
	.filters-panel {
		background-color: #16213e;
		border-radius: 8px;
		border: 1px solid #0f3460;
		padding: 1rem;
		margin-bottom: 1.5rem;
	}

	.filter-section {
		margin-bottom: 1.25rem;
	}

	.filter-section:last-child {
		margin-bottom: 0;
	}

	.filter-section h3 {
		font-size: 0.875rem;
		color: #9ca3af;
		margin-bottom: 0.75rem;
		font-weight: 500;
	}

	.filter-options {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.filter-option {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.375rem 0.625rem;
		background-color: #1a1a2e;
		border: 1px solid #0f3460;
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.filter-option:hover {
		border-color: #e94560;
	}

	.filter-option input {
		width: 14px;
		height: 14px;
	}

	.option-label {
		font-size: 0.75rem;
		color: #eaeaea;
	}

	.option-count {
		font-size: 0.75rem;
		color: #6b7280;
	}

	.severity-range {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.range-inputs {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.range-inputs label {
		display: flex;
		align-items: center;
		gap: 0.375rem;
	}

	.range-inputs span {
		font-size: 0.75rem;
		color: #9ca3af;
	}

	.range-input {
		width: 60px;
		padding: 0.375rem;
		background-color: #1a1a2e;
		border: 1px solid #0f3460;
		border-radius: 4px;
		color: #eaeaea;
		font-size: 0.75rem;
	}

	.range-input:focus {
		outline: none;
		border-color: #e94560;
	}

	.range-separator {
		font-size: 0.75rem;
		color: #6b7280;
	}

	.severity-badges {
		display: flex;
		gap: 0.5rem;
	}

	.severity-badge {
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
		font-size: 0.625rem;
		color: white;
	}

	.severity-badge.low {
		background-color: #4ade80;
	}

	.severity-badge.medium {
		background-color: #eab308;
	}

	.severity-badge.high {
		background-color: #ef4444;
	}

	.date-inputs {
		display: flex;
		gap: 0.75rem;
	}

	.date-field {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.date-field span {
		font-size: 0.75rem;
		color: #9ca3af;
	}

	.date-input {
		padding: 0.375rem;
		background-color: #1a1a2e;
		border: 1px solid #0f3460;
		border-radius: 4px;
		color: #eaeaea;
		font-size: 0.75rem;
	}

	.date-input:focus {
		outline: none;
		border-color: #e94560;
	}

	.confidence-slider {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.slider {
		width: 100%;
		height: 6px;
		-webkit-appearance: none;
		appearance: none;
		background: #0f3460;
		border-radius: 3px;
		outline: none;
	}

	.slider::-webkit-slider-thumb {
		-webkit-appearance: none;
		appearance: none;
		width: 16px;
		height: 16px;
		background: #e94560;
		border-radius: 50%;
		cursor: pointer;
	}

	.slider::-moz-range-thumb {
		width: 16px;
		height: 16px;
		background: #e94560;
		border-radius: 50%;
		cursor: pointer;
		border: none;
	}

	.confidence-badges {
		display: flex;
		gap: 0.5rem;
	}

	.conf-badge {
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
		font-size: 0.625rem;
		color: white;
	}

	.conf-badge.low {
		background-color: #ef4444;
	}

	.conf-badge.medium {
		background-color: #eab308;
	}

	.conf-badge.high {
		background-color: #4ade80;
	}
</style>
