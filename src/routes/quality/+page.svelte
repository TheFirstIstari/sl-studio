<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';
	import { PageHeader, StatCard, FilterBar } from '$lib/components';
	import { getSeverityColor, getCategoryIcon, getQualityBadgeColor } from '$lib/utils';

	interface Fact {
		id: number;
		fingerprint: string;
		filename: string;
		fact_summary: string;
		category: string | null;
		severity_score: number;
		confidence: number | null;
		verification_status: string | null;
		review_notes: string | null;
		created_at: string;
	}

	let facts = $state<Fact[]>([]);
	let loading = $state(true);
	let error = $state('');
	let selectedFact = $state<Fact | null>(null);
	let filter = $state('');
	let showVerified = $state('low'); // 'low' | 'all' | 'verified'

	// FR-DEDUP: near-duplicate detection
	interface DuplicateGroup {
		keeper_id: number;
		member_ids: number[];
		similarity: number;
	}
	let dedupGroups = $state<DuplicateGroup[]>([]);
	let dedupThreshold = $state(0.85);
	let dedupRequireSameCategory = $state(true);
	let dedupRunning = $state(false);
	let dedupRanOnce = $state(false);

	async function findDuplicates() {
		dedupRunning = true;
		error = '';
		try {
			dedupGroups = await invoke<DuplicateGroup[]>('find_duplicate_facts', {
				threshold: dedupThreshold,
				requireSameCategory: dedupRequireSameCategory,
				requireSameDate: false
			});
			dedupRanOnce = true;
		} catch (e) {
			console.error('Find duplicates failed:', e);
			error = String(e);
		} finally {
			dedupRunning = false;
		}
	}

	async function mergeGroup(group: DuplicateGroup) {
		try {
			const deleted = await invoke<number>('merge_duplicate_facts', {
				keeperId: group.keeper_id,
				memberIds: group.member_ids
			});
			// Drop the merged group from the panel and reload facts so the
			// list reflects the soft-deletes.
			dedupGroups = dedupGroups.filter((g) => g.keeper_id !== group.keeper_id);
			await loadFacts();
			error = '';
			console.info(`Merged ${deleted} duplicates into fact ${group.keeper_id}`);
		} catch (e) {
			console.error('Merge failed:', e);
			error = String(e);
		}
	}

	function factSummaryById(id: number): string {
		return facts.find((f) => f.id === id)?.fact_summary ?? `(fact #${id})`;
	}

	// FR-VERIF: cross-validation
	interface CorroborationMatch {
		intelligence_id: number;
		filename: string;
		fact_summary: string;
		similarity: number;
		agreement: 'agree' | 'partial' | 'conflict';
	}
	interface CrossValidationResult {
		intelligence_id: number;
		source_filename: string;
		matches: CorroborationMatch[];
		consensus_score: number;
	}
	let crossValidation = $state<CrossValidationResult | null>(null);
	let crossValidating = $state(false);

	async function runCrossValidation(factId: number) {
		crossValidating = true;
		try {
			crossValidation = await invoke<CrossValidationResult>('cross_validate_fact', {
				intelligenceId: factId,
				threshold: 0.5
			});
		} catch (e) {
			console.error('Cross-validation failed:', e);
			error = String(e);
			crossValidation = null;
		} finally {
			crossValidating = false;
		}
	}

	// FR-WEIGHT: per-fact evidence weight (auto-loaded on selection)
	let evidenceWeight = $state<number | null>(null);
	let weightForFactId = $state<number | null>(null);

	async function loadEvidenceWeight(factId: number) {
		try {
			evidenceWeight = await invoke<number>('get_evidence_weight', {
				intelligenceId: factId
			});
			weightForFactId = factId;
		} catch (e) {
			console.error('Load evidence weight failed:', e);
			evidenceWeight = null;
		}
	}

	// Reset cross-validation + auto-load weight when selectedFact changes.
	$effect(() => {
		if (!selectedFact) {
			crossValidation = null;
			evidenceWeight = null;
			weightForFactId = null;
			return;
		}
		if (crossValidation && crossValidation.intelligence_id !== selectedFact.id) {
			crossValidation = null;
		}
		if (weightForFactId !== selectedFact.id) {
			loadEvidenceWeight(selectedFact.id);
		}
	});

	// Stats
	let stats = $derived.by(() => ({
		total: facts.length,
		lowConfidence: facts.filter((f) => (f.confidence ?? 0) < 0.5).length,
		mediumConfidence: facts.filter((f) => (f.confidence ?? 0) >= 0.5 && (f.confidence ?? 0) < 0.7)
			.length,
		highConfidence: facts.filter((f) => (f.confidence ?? 0) >= 0.7).length,
		unverified: facts.filter((f) => f.verification_status === 'unverified').length,
		confirmed: facts.filter((f) => f.verification_status === 'confirmed').length,
		disputed: facts.filter((f) => f.verification_status === 'disputed').length
	}));

	let filteredFacts = $derived(
		facts
			.filter((f) => {
				// Text filter
				if (
					filter &&
					!f.fact_summary.toLowerCase().includes(filter.toLowerCase()) &&
					!f.filename.toLowerCase().includes(filter.toLowerCase()) &&
					!f.category?.toLowerCase().includes(filter.toLowerCase())
				) {
					return false;
				}

				// Confidence filter
				const conf = f.confidence ?? 0;
				if (showVerified === 'low' && conf >= 0.5) {
					return false;
				}

				return true;
			})
			.sort((a, b) => {
				// Sort by severity (highest first), then by confidence (lowest first for review priority)
				if (b.severity_score !== a.severity_score) {
					return b.severity_score - a.severity_score;
				}
				return (a.confidence ?? 0) - (b.confidence ?? 0);
			})
	);

	onMount(async () => {
		await loadFacts();
	});

	async function loadFacts() {
		loading = true;
		error = '';
		try {
			// Get facts with low confidence for review
			const results = await invoke<
				{
					id: number;
					fingerprint: string;
					filename: string;
					summary: string;
					category: string | null;
					severity: number;
					confidence: number | null;
				}[]
			>('search_facts', {
				query: '',
				limit: 1000
			});

			facts = results.map((r) => ({
				id: r.id,
				fingerprint: r.fingerprint,
				filename: r.filename,
				fact_summary: r.summary,
				category: r.category,
				severity_score: r.severity,
				confidence: r.confidence,
				verification_status: 'unverified',
				review_notes: null,
				created_at: new Date().toISOString()
			}));
		} catch (e) {
			console.error('Error loading facts:', e);
			error = String(e);
			facts = [];
		} finally {
			loading = false;
		}
	}

	async function updateVerificationStatus(id: number, status: string) {
		try {
			await invoke('update_fact_verification', {
				id,
				status,
				reviewNotes: null
			});

			// Update local state
			facts = facts.map((f) => (f.id === id ? { ...f, verification_status: status } : f));

			if (selectedFact?.id === id) {
				selectedFact = { ...selectedFact, verification_status: status };
			}
		} catch (e) {
			console.error('Error updating verification:', e);
			error = String(e);
		}
	}

	function exportReviewReport() {
		const reviewed = facts.filter((f) => f.verification_status !== 'unverified');
		const report = {
			generated_at: new Date().toISOString(),
			summary: {
				total_facts: facts.length,
				reviewed: reviewed.length,
				confirmed: stats.confirmed,
				disputed: stats.disputed,
				unverified: stats.unverified
			},
			facts: reviewed.map((f) => ({
				id: f.id,
				filename: f.filename,
				summary: f.fact_summary,
				category: f.category,
				severity: f.severity_score,
				confidence: f.confidence,
				verification_status: f.verification_status,
				review_notes: f.review_notes
			}))
		};

		const blob = new Blob([JSON.stringify(report, null, 2)], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `quality-review-${new Date().toISOString().split('T')[0]}.json`;
		a.click();
		URL.revokeObjectURL(url);
	}
</script>

<div class="quality-page page">
	<PageHeader
		title="Quality Review Queue"
		subtitle="Review low-confidence extractions and verify facts"
	/>

	{#if error}
		<div class="error-banner">
			{error}
		</div>
	{/if}

	<!-- FR-DEDUP: Duplicate detection panel -->
	<section class="dedup-panel">
		<header class="dedup-header">
			<h2>Find Duplicate Facts</h2>
			<p class="subtitle">
				Detect near-duplicate fact summaries so you can merge them. Soft-deletes losers; the keeper
				retains its provenance.
			</p>
		</header>
		<div class="dedup-controls">
			<label>
				Similarity threshold
				<input
					type="range"
					min="0.5"
					max="1"
					step="0.05"
					bind:value={dedupThreshold}
					disabled={dedupRunning}
				/>
				<span class="range-value">{dedupThreshold.toFixed(2)}</span>
			</label>
			<label class="checkbox-label">
				<input type="checkbox" bind:checked={dedupRequireSameCategory} disabled={dedupRunning} />
				Require same category
			</label>
			<button class="btn primary" onclick={findDuplicates} disabled={dedupRunning}>
				{dedupRunning ? 'Scanning...' : 'Find Duplicates'}
			</button>
		</div>

		{#if dedupRanOnce && dedupGroups.length === 0 && !dedupRunning}
			<p class="dedup-empty">No duplicate groups found at threshold {dedupThreshold.toFixed(2)}.</p>
		{/if}

		{#if dedupGroups.length > 0}
			<div class="dedup-groups">
				{#each dedupGroups as group (group.keeper_id)}
					<div class="dedup-group">
						<div class="dedup-group-header">
							<span class="dedup-count">{group.member_ids.length} facts</span>
							<span class="dedup-similarity">avg sim {group.similarity.toFixed(2)}</span>
							<button class="merge-btn" onclick={() => mergeGroup(group)}>
								Merge into #{group.keeper_id}
							</button>
						</div>
						<ul class="dedup-members">
							{#each group.member_ids as id (id)}
								<li class:keeper={id === group.keeper_id}>
									<strong>#{id}{id === group.keeper_id ? ' (keeper)' : ''}</strong>
									<span>{factSummaryById(id)}</span>
								</li>
							{/each}
						</ul>
					</div>
				{/each}
			</div>
		{/if}
	</section>

	<!-- Stats Cards -->
	<div class="stat-grid">
		<StatCard value={stats.total} label="Total facts" />
		<StatCard value={stats.lowConfidence} label="Low confidence (<50%)" variant="danger" />
		<StatCard value={stats.mediumConfidence} label="Medium (50-70%)" variant="warning" />
		<StatCard value={stats.highConfidence} label="High (≥70%)" variant="success" />
		<StatCard value={stats.unverified} label="Unverified" />
		<StatCard value={stats.confirmed} label="Confirmed" variant="success" />
		<StatCard value={stats.disputed} label="Disputed" variant="danger" />
	</div>

	<FilterBar bind:search={filter} placeholder="Filter facts...">
		{#snippet extras()}
			<select bind:value={showVerified} class="filter-select">
				<option value="low">Low confidence first</option>
				<option value="all">All facts</option>
				<option value="verified">Verified only</option>
			</select>
		{/snippet}
	</FilterBar>

	<!-- Facts List -->
	<div class="facts-container">
		{#if loading}
			<div class="loading">Loading facts...</div>
		{:else if filteredFacts.length === 0}
			<div class="empty-state">
				{#if facts.length === 0}
					<p>No facts found. Process some documents first.</p>
				{:else}
					<p>No facts match the current filters.</p>
				{/if}
			</div>
		{:else}
			<div class="facts-list">
				{#each filteredFacts as fact (fact.id)}
					<button
						class="fact-item"
						class:selected={selectedFact?.id === fact.id}
						onclick={() => (selectedFact = fact)}
					>
						<div class="fact-header">
							<span
								class="quality-badge"
								style="background-color: {getQualityBadgeColor(fact.confidence)}"
							>
								{Math.round((fact.confidence ?? 0) * 100)}%
							</span>
							<span
								class="severity-badge"
								style="background-color: {getSeverityColor(fact.severity_score)}"
							>
								Severity: {fact.severity_score}
							</span>
							{#if fact.verification_status !== 'unverified'}
								<span
									class="status-badge"
									class:confirmed={fact.verification_status === 'confirmed'}
									class:disputed={fact.verification_status === 'disputed'}
								>
									{fact.verification_status}
								</span>
							{/if}
						</div>
						<div class="fact-summary">{fact.fact_summary}</div>
						<div class="fact-meta">
							<span class="category"
								>{getCategoryIcon(fact.category)} {fact.category ?? 'Unknown'}</span
							>
							<span class="filename">{fact.filename}</span>
						</div>
					</button>
				{/each}
			</div>
		{/if}

		<!-- Detail Panel -->
		{#if selectedFact}
			<div class="detail-panel">
				<h2>Fact Details</h2>

				<div class="detail-badges">
					<span
						class="quality-badge large"
						style="background-color: {getQualityBadgeColor(selectedFact.confidence)}"
					>
						Confidence: {Math.round((selectedFact.confidence ?? 0) * 100)}%
					</span>
					<span
						class="severity-badge large"
						style="background-color: {getSeverityColor(selectedFact.severity_score)}"
					>
						Severity: {selectedFact.severity_score}
					</span>
				</div>

				<div class="detail-section">
					<h3>Summary</h3>
					<p>{selectedFact.fact_summary}</p>
				</div>

				<div class="detail-section">
					<h3>Source</h3>
					<p>{selectedFact.filename}</p>
				</div>

				{#if evidenceWeight !== null}
					<div class="detail-section">
						<h3>Evidence weight</h3>
						<div class="weight-row">
							<div class="weight-bar">
								<div
									class="weight-fill"
									class:strong={evidenceWeight >= 0.7}
									class:weak={evidenceWeight < 0.4}
									style="width: {Math.round(Math.min(evidenceWeight, 1) * 100)}%"
								></div>
							</div>
							<span class="weight-value">{(evidenceWeight * 100).toFixed(0)}%</span>
						</div>
						<p class="weight-hint">
							Combines severity, confidence, source reliability, and corroboration.
						</p>
					</div>
				{/if}

				<div class="detail-section">
					<h3>Category</h3>
					<p>{selectedFact.category ?? 'Unknown'}</p>
				</div>

				<!-- Verification Workflow -->
				<div class="detail-section">
					<h3>Verification Status</h3>
					<div class="verification-buttons">
						<button
							class="verify-btn unverified"
							class:active={selectedFact.verification_status === 'unverified'}
							onclick={() =>
								selectedFact && updateVerificationStatus(selectedFact.id, 'unverified')}
						>
							Unverified
						</button>
						<button
							class="verify-btn confirmed"
							class:active={selectedFact.verification_status === 'confirmed'}
							onclick={() => selectedFact && updateVerificationStatus(selectedFact.id, 'confirmed')}
						>
							Confirmed
						</button>
						<button
							class="verify-btn disputed"
							class:active={selectedFact.verification_status === 'disputed'}
							onclick={() => selectedFact && updateVerificationStatus(selectedFact.id, 'disputed')}
						>
							Disputed
						</button>
					</div>
				</div>

				<!-- FR-VERIF: cross-validation across sources -->
				<div class="detail-section">
					<div class="cross-header">
						<h3>Cross-validation</h3>
						<button
							class="btn sm"
							onclick={() => selectedFact && runCrossValidation(selectedFact.id)}
							disabled={crossValidating}
						>
							{crossValidating ? 'Checking...' : 'Check sources'}
						</button>
					</div>
					{#if crossValidation}
						<div class="consensus">
							<span class="consensus-label">Consensus score</span>
							<div class="consensus-bar" title="Average similarity × source diversity">
								<div
									class="consensus-fill"
									class:strong={crossValidation.consensus_score >= 0.6}
									class:weak={crossValidation.consensus_score < 0.3}
									style="width: {Math.round(crossValidation.consensus_score * 100)}%"
								></div>
							</div>
							<span class="consensus-value">
								{(crossValidation.consensus_score * 100).toFixed(0)}%
							</span>
						</div>
						{#if crossValidation.matches.length === 0}
							<p class="cross-empty">
								No corroborating facts found in other sources at similarity ≥ 0.5.
							</p>
						{:else}
							<ul class="cross-matches">
								{#each crossValidation.matches.slice(0, 5) as m (m.intelligence_id)}
									<li>
										<span class="agreement agreement-{m.agreement}">{m.agreement}</span>
										<span class="cross-summary">{m.fact_summary}</span>
										<span class="cross-source">{m.filename}</span>
										<span class="cross-sim">{(m.similarity * 100).toFixed(0)}%</span>
									</li>
								{/each}
							</ul>
						{/if}
					{:else}
						<p class="cross-hint">
							Find matching facts in other source files to validate or contradict this one.
						</p>
					{/if}
				</div>

				<!-- Review Actions -->
				<div class="review-actions">
					<button
						class="action-btn"
						onclick={() => selectedFact && updateVerificationStatus(selectedFact.id, 'confirmed')}
					>
						Mark as Reviewed
					</button>
					<button
						class="action-btn flag"
						onclick={() => selectedFact && updateVerificationStatus(selectedFact.id, 'disputed')}
					>
						Flag for Follow-up
					</button>
				</div>
			</div>
		{/if}
	</div>

	<!-- Export Button -->
	<div class="export-bar">
		<button class="export-btn" onclick={exportReviewReport}> Export Review Report </button>
	</div>
</div>

<style>
	.quality-page {
		padding: 1.5rem;
		max-width: 1400px;
		margin: 0 auto;
	}

	/* page-header, stat cards, filter input come from shared components and
	   theme.css; this page only styles its bespoke pieces below. */

	.filter-select {
		background: var(--color-bg-input);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		color: var(--color-text-primary);
		padding: var(--space-2) var(--space-3);
		font-size: var(--text-sm);
		min-width: 150px;
	}

	.facts-container {
		display: grid;
		grid-template-columns: 1fr 350px;
		gap: 1rem;
		min-height: 400px;
	}

	.facts-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-height: 600px;
		overflow-y: auto;
	}

	.fact-item {
		background: var(--color-bg-card);
		border: 1px solid var(--color-border);
		border-radius: 0.5rem;
		padding: 0.75rem;
		text-align: left;
		cursor: pointer;
		transition: border-color 0.15s;
	}

	.fact-item:hover {
		border-color: var(--color-text-muted);
	}

	.fact-item.selected {
		border-color: var(--color-accent);
	}

	.fact-header {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 0.5rem;
		flex-wrap: wrap;
	}

	.quality-badge,
	.severity-badge,
	.status-badge {
		font-size: 0.7rem;
		padding: 0.15rem 0.4rem;
		border-radius: 0.25rem;
		color: var(--color-text-inverse);
		font-weight: 500;
	}

	.quality-badge.large,
	.severity-badge.large {
		font-size: 0.875rem;
		padding: 0.25rem 0.5rem;
	}

	.status-badge {
		background: var(--color-text-muted);
		color: var(--color-text-inverse);
	}

	.status-badge.confirmed {
		background: var(--color-status-confirmed);
	}

	.status-badge.disputed {
		background: var(--color-severity-high);
		color: var(--color-text-inverse);
	}

	.fact-summary {
		color: var(--color-text-primary);
		font-size: 0.875rem;
		margin-bottom: 0.5rem;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.fact-meta {
		display: flex;
		gap: 1rem;
		font-size: 0.75rem;
		color: var(--color-text-secondary);
	}

	.category {
		color: var(--color-entity-date);
	}

	.loading,
	.empty-state {
		background: var(--color-bg-card);
		border-radius: 0.5rem;
		padding: 2rem;
		text-align: center;
		color: var(--color-text-secondary);
	}

	.detail-panel {
		background: var(--color-bg-card);
		border-radius: 0.5rem;
		padding: 1rem;
		height: fit-content;
	}

	.detail-panel h2 {
		font-size: 1.125rem;
		font-weight: 600;
		color: var(--color-text-primary);
		margin: 0 0 1rem;
	}

	.detail-badges {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1rem;
		flex-wrap: wrap;
	}

	.detail-section {
		margin-bottom: 1rem;
	}

	.detail-section h3 {
		font-size: 0.75rem;
		font-weight: 500;
		color: var(--color-text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin: 0 0 0.25rem;
	}

	.detail-section p {
		color: var(--color-text-primary);
		font-size: 0.875rem;
		margin: 0;
	}

	.verification-buttons {
		display: flex;
		gap: 0.5rem;
	}

	.verify-btn {
		flex: 1;
		padding: 0.5rem;
		border: 1px solid var(--color-border);
		border-radius: 0.375rem;
		background: transparent;
		color: var(--color-text-secondary);
		font-size: 0.8rem;
		cursor: pointer;
		transition: all 0.15s;
	}

	.verify-btn:hover {
		border-color: var(--color-text-muted);
		color: var(--color-text-primary);
	}

	.verify-btn.active.unverified {
		background: var(--color-text-muted);
		border-color: var(--color-text-muted);
		color: var(--color-text-inverse);
	}

	.verify-btn.active.confirmed {
		background: var(--color-status-confirmed);
		border-color: var(--color-status-confirmed);
		color: var(--color-text-inverse);
	}

	.verify-btn.active.disputed {
		background: var(--color-severity-high);
		border-color: var(--color-severity-high);
		color: var(--color-text-inverse);
	}

	.review-actions {
		display: flex;
		gap: 0.5rem;
		margin-top: 1rem;
	}

	.action-btn {
		flex: 1;
		padding: 0.5rem;
		background: var(--color-bg-elevated);
		border: none;
		border-radius: 0.375rem;
		color: var(--color-text-primary);
		font-size: 0.875rem;
		cursor: pointer;
		transition: background 0.15s;
	}

	.action-btn:hover {
		background: var(--color-text-muted);
	}

	.action-btn.flag {
		background: var(--color-severity-high);
	}

	.action-btn.flag:hover {
		background: var(--color-severity-high);
	}

	.export-bar {
		margin-top: 1.5rem;
		text-align: right;
	}

	.export-btn {
		padding: 0.625rem 1.25rem;
		background: var(--color-accent);
		border: none;
		border-radius: 0.375rem;
		color: var(--color-text-inverse);
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: background 0.15s;
	}

	.export-btn:hover {
		background: var(--color-accent-hover);
	}

	/* FR-DEDUP panel */
	.dedup-panel {
		background: var(--color-bg-card);
		border: 1px solid var(--color-border);
		border-radius: 0.5rem;
		padding: 1.25rem;
		margin-bottom: 1.5rem;
	}

	.dedup-header h2 {
		margin: 0 0 0.25rem;
		font-size: 1.15rem;
	}

	.dedup-header .subtitle {
		margin: 0 0 0.75rem;
		color: var(--color-text-muted);
		font-size: 0.85rem;
	}

	.dedup-controls {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 1rem;
		margin: 0.5rem 0 1rem;
	}

	.dedup-controls label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.85rem;
	}

	.dedup-controls input[type='range'] {
		width: 160px;
	}

	.dedup-controls .range-value {
		font-variant-numeric: tabular-nums;
		min-width: 2.5rem;
	}

	.dedup-controls button.primary {
		background: var(--color-accent);
		color: var(--color-text-inverse);
		border: none;
		padding: 0.5rem 1rem;
		border-radius: 0.375rem;
		font-weight: 500;
		cursor: pointer;
	}

	.dedup-controls button.primary:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.dedup-empty {
		color: var(--color-text-muted);
		font-style: italic;
		margin: 0.5rem 0 0;
	}

	.dedup-groups {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.dedup-group {
		border: 1px solid var(--color-border);
		border-radius: 0.375rem;
		padding: 0.75rem;
		background: rgba(255, 255, 255, 0.02);
	}

	.dedup-group-header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.5rem;
	}

	.dedup-count {
		font-weight: 600;
	}

	.dedup-similarity {
		color: var(--color-text-muted);
		font-size: 0.85rem;
	}

	.merge-btn {
		margin-left: auto;
		background: transparent;
		color: var(--color-accent);
		border: 1px solid var(--color-accent);
		padding: 0.35rem 0.75rem;
		border-radius: 0.375rem;
		font-size: 0.85rem;
		cursor: pointer;
	}

	.merge-btn:hover {
		background: var(--color-accent);
		color: var(--color-text-inverse);
	}

	.dedup-members {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.dedup-members li {
		display: flex;
		gap: 0.5rem;
		font-size: 0.85rem;
	}

	.dedup-members li.keeper {
		color: var(--color-status-confirmed);
	}

	/* FR-VERIF cross-validation panel */
	.cross-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: var(--space-2);
	}

	.cross-header h3 {
		margin: 0;
	}

	.cross-hint,
	.cross-empty {
		font-size: var(--text-sm);
		color: var(--color-text-muted);
		margin: var(--space-1) 0 0;
	}

	.consensus {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin-bottom: var(--space-3);
	}

	.consensus-label {
		font-size: var(--text-xs);
		color: var(--color-text-muted);
	}

	.consensus-bar {
		flex: 1;
		height: 8px;
		background: var(--color-bg-elevated);
		border-radius: 999px;
		overflow: hidden;
	}

	.consensus-fill {
		height: 100%;
		background: var(--color-status-info);
		transition: width 0.2s;
	}

	.consensus-fill.strong {
		background: var(--color-status-confirmed);
	}

	.consensus-fill.weak {
		background: var(--color-severity-high);
	}

	.consensus-value {
		font-variant-numeric: tabular-nums;
		font-size: var(--text-sm);
		min-width: 3rem;
		text-align: right;
	}

	.cross-matches {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.cross-matches li {
		display: grid;
		grid-template-columns: auto 1fr auto auto;
		gap: var(--space-2);
		align-items: center;
		font-size: var(--text-sm);
	}

	.agreement {
		text-transform: uppercase;
		font-size: var(--text-xs);
		font-weight: 600;
		padding: 2px 6px;
		border-radius: var(--radius-sm);
		letter-spacing: 0.05em;
	}

	.agreement-agree {
		background: rgba(74, 222, 128, 0.15);
		color: var(--color-status-confirmed);
	}

	.agreement-partial {
		background: rgba(234, 179, 8, 0.15);
		color: var(--color-severity-medium);
	}

	.agreement-conflict {
		background: rgba(239, 68, 68, 0.15);
		color: var(--color-severity-high);
	}

	.cross-summary {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.cross-source {
		color: var(--color-text-muted);
		font-size: var(--text-xs);
		max-width: 120px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.cross-sim {
		font-variant-numeric: tabular-nums;
		color: var(--color-text-muted);
		font-size: var(--text-xs);
	}

	/* FR-WEIGHT inline display */
	.weight-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.weight-bar {
		flex: 1;
		height: 8px;
		background: var(--color-bg-elevated);
		border-radius: 999px;
		overflow: hidden;
	}

	.weight-fill {
		height: 100%;
		background: var(--color-status-info);
		transition: width 0.2s;
	}

	.weight-fill.strong {
		background: var(--color-status-confirmed);
	}

	.weight-fill.weak {
		background: var(--color-severity-high);
	}

	.weight-value {
		font-variant-numeric: tabular-nums;
		font-size: var(--text-sm);
		min-width: 3rem;
		text-align: right;
	}

	.weight-hint {
		margin: var(--space-1) 0 0;
		font-size: var(--text-xs);
		color: var(--color-text-muted);
	}
</style>
