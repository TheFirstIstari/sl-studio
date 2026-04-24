<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';

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

	// Quality badge colors per SPEC
	function getQualityBadgeColor(confidence: number | null): string {
		const conf = confidence ?? 0;
		if (conf >= 0.7) return '#22c55e'; // green
		if (conf >= 0.5) return '#eab308'; // yellow
		return '#ef4444'; // red
	}

	// Severity color from existing pattern
	function getSeverityColor(score: number): string {
		if (score >= 8) return '#ef4444';
		if (score >= 6) return '#f97316';
		if (score >= 4) return '#eab308';
		return '#4ade80';
	}

	function getCategoryIcon(category: string | null): string {
		if (category === 'Financial') return '$';
		if (category === 'Legal') return '§';
		if (category === 'Digital') return '§';
		if (category === 'Physical') return '¶';
		if (category === 'Verbal') return '§';
		return '•';
	}

	let facts = $state<Fact[]>([]);
	let loading = $state(true);
	let error = $state('');
	let selectedFact = $state<Fact | null>(null);
	let filter = $state('');
	let showVerified = $state('low'); // 'low' | 'all' | 'verified'

	// Stats
	let stats = $derived.by(() => ({
		total: facts.length,
		lowConfidence: facts.filter(f => (f.confidence ?? 0) < 0.5).length,
		mediumConfidence: facts.filter(f => (f.confidence ?? 0) >= 0.5 && (f.confidence ?? 0) < 0.7).length,
		highConfidence: facts.filter(f => (f.confidence ?? 0) >= 0.7).length,
		unverified: facts.filter(f => f.verification_status === 'unverified').length,
		confirmed: facts.filter(f => f.verification_status === 'confirmed').length,
		disputed: facts.filter(f => f.verification_status === 'disputed').length,
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
			const results = await invoke<{ id: number; fingerprint: string; filename: string; summary: string; category: string | null; severity: number; confidence: number | null; }[]>('search_facts', {
				query: '',
				limit: 1000
			});

			facts = results.map(r => ({
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
			facts = facts.map(f =>
				f.id === id ? { ...f, verification_status: status } : f
			);

			if (selectedFact?.id === id) {
				selectedFact = { ...selectedFact, verification_status: status };
			}
		} catch (e) {
			console.error('Error updating verification:', e);
			error = String(e);
		}
	}

	function exportReviewReport() {
		const reviewed = facts.filter(f => f.verification_status !== 'unverified');
		const report = {
			generated_at: new Date().toISOString(),
			summary: {
				total_facts: facts.length,
				reviewed: reviewed.length,
				confirmed: stats.confirmed,
				disputed: stats.disputed,
				unverified: stats.unverified
			},
			facts: reviewed.map(f => ({
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

<div class="quality-page">
	<header class="page-header">
		<h1>Quality Review Queue</h1>
		<p class="subtitle">Review low-confidence extractions and verify facts</p>
	</header>

	{#if error}
		<div class="error-banner">
			{error}
		</div>
	{/if}

	<!-- Stats Cards -->
	<div class="stats-grid">
		<div class="stat-card">
			<div class="stat-value">{stats.total}</div>
			<div class="stat-label">Total Facts</div>
		</div>
		<div class="stat-card low">
			<div class="stat-value">{stats.lowConfidence}</div>
			<div class="stat-label">Low Confidence (&lt;50%)</div>
		</div>
		<div class="stat-card medium">
			<div class="stat-value">{stats.mediumConfidence}</div>
			<div class="stat-label">Medium (50-70%)</div>
		</div>
		<div class="stat-card high">
			<div class="stat-value">{stats.highConfidence}</div>
			<div class="stat-label">High (≥70%)</div>
		</div>
		<div class="stat-card">
			<div class="stat-value">{stats.unverified}</div>
			<div class="stat-label">Unverified</div>
		</div>
		<div class="stat-card confirmed">
			<div class="stat-value">{stats.confirmed}</div>
			<div class="stat-label">Confirmed</div>
		</div>
		<div class="stat-card disputed">
			<div class="stat-value">{stats.disputed}</div>
			<div class="stat-label">Disputed</div>
		</div>
	</div>

	<!-- Filters -->
	<div class="filters">
		<input
			type="text"
			placeholder="Filter facts..."
			bind:value={filter}
			class="filter-input"
		/>
		<select bind:value={showVerified} class="filter-select">
			<option value="low">Low Confidence First</option>
			<option value="all">All Facts</option>
			<option value="verified">Verified Only</option>
		</select>
	</div>

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
						onclick={() => selectedFact = fact}
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
							<span class="category">{getCategoryIcon(fact.category)} {fact.category ?? 'Unknown'}</span>
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
							onclick={() => updateVerificationStatus(selectedFact.id, 'unverified')}
						>
							Unverified
						</button>
						<button
							class="verify-btn confirmed"
							class:active={selectedFact.verification_status === 'confirmed'}
							onclick={() => updateVerificationStatus(selectedFact.id, 'confirmed')}
						>
							Confirmed
						</button>
						<button
							class="verify-btn disputed"
							class:active={selectedFact.verification_status === 'disputed'}
							onclick={() => updateVerificationStatus(selectedFact.id, 'disputed')}
						>
							Disputed
						</button>
					</div>
				</div>

				<!-- Review Actions -->
				<div class="review-actions">
					<button class="action-btn" onclick={() => updateVerificationStatus(selectedFact.id, 'confirmed')}>
						Mark as Reviewed
					</button>
					<button class="action-btn flag" onclick={() => updateVerificationStatus(selectedFact.id, 'disputed')}>
						Flag for Follow-up
					</button>
				</div>
			</div>
		{/if}
	</div>

	<!-- Export Button -->
	<div class="export-bar">
		<button class="export-btn" onclick={exportReviewReport}>
			Export Review Report
		</button>
	</div>
</div>

<style>
	.quality-page {
		padding: 1.5rem;
		max-width: 1400px;
		margin: 0 auto;
	}

	.page-header {
		margin-bottom: 1.5rem;
	}

	.page-header h1 {
		font-size: 1.75rem;
		font-weight: 600;
		color: #f1f5f9;
		margin: 0;
	}

	.subtitle {
		color: #94a3b8;
		margin: 0.25rem 0 0;
	}

	.error-banner {
		background: #7f1d1d;
		color: #fecaca;
		padding: 0.75rem 1rem;
		border-radius: 0.5rem;
		margin-bottom: 1rem;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
		gap: 0.75rem;
		margin-bottom: 1.5rem;
	}

	.stat-card {
		background: #1e293b;
		border-radius: 0.5rem;
		padding: 1rem;
		text-align: center;
	}

	.stat-card.low {
		border: 1px solid #ef4444;
	}

	.stat-card.medium {
		border: 1px solid #eab308;
	}

	.stat-card.high {
		border: 1px solid #22c55e;
	}

	.stat-card.confirmed {
		border: 1px solid #22c55e;
	}

	.stat-card.disputed {
		border: 1px solid #ef4444;
	}

	.stat-value {
		font-size: 1.5rem;
		font-weight: 600;
		color: #f1f5f9;
	}

	.stat-label {
		font-size: 0.75rem;
		color: #94a3b8;
		margin-top: 0.25rem;
	}

	.filters {
		display: flex;
		gap: 0.75rem;
		margin-bottom: 1rem;
	}

	.filter-input,
	.filter-select {
		background: #1e293b;
		border: 1px solid #334155;
		border-radius: 0.375rem;
		color: #f1f5f9;
		padding: 0.5rem 0.75rem;
		font-size: 0.875rem;
	}

	.filter-input {
		flex: 1;
		max-width: 300px;
	}

	.filter-select {
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
		background: #1e293b;
		border: 1px solid #334155;
		border-radius: 0.5rem;
		padding: 0.75rem;
		text-align: left;
		cursor: pointer;
		transition: border-color 0.15s;
	}

	.fact-item:hover {
		border-color: #475569;
	}

	.fact-item.selected {
		border-color: #3b82f6;
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
		color: #0f172a;
		font-weight: 500;
	}

	.quality-badge.large,
	.severity-badge.large {
		font-size: 0.875rem;
		padding: 0.25rem 0.5rem;
	}

	.status-badge {
		background: #64748b;
		color: white;
	}

	.status-badge.confirmed {
		background: #22c55e;
	}

	.status-badge.disputed {
		background: #ef4444;
		color: white;
	}

	.fact-summary {
		color: #e2e8f0;
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
		color: #94a3b8;
	}

	.category {
		color: #818cf8;
	}

	.loading,
	.empty-state {
		background: #1e293b;
		border-radius: 0.5rem;
		padding: 2rem;
		text-align: center;
		color: #94a3b8;
	}

	.detail-panel {
		background: #1e293b;
		border-radius: 0.5rem;
		padding: 1rem;
		height: fit-content;
	}

	.detail-panel h2 {
		font-size: 1.125rem;
		font-weight: 600;
		color: #f1f5f9;
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
		color: #94a3b8;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin: 0 0 0.25rem;
	}

	.detail-section p {
		color: #e2e8f0;
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
		border: 1px solid #334155;
		border-radius: 0.375rem;
		background: transparent;
		color: #94a3b8;
		font-size: 0.8rem;
		cursor: pointer;
		transition: all 0.15s;
	}

	.verify-btn:hover {
		border-color: #475569;
		color: #e2e8f0;
	}

	.verify-btn.active.unverified {
		background: #64748b;
		border-color: #64748b;
		color: white;
	}

	.verify-btn.active.confirmed {
		background: #22c55e;
		border-color: #22c55e;
		color: white;
	}

	.verify-btn.active.disputed {
		background: #ef4444;
		border-color: #ef4444;
		color: white;
	}

	.review-actions {
		display: flex;
		gap: 0.5rem;
		margin-top: 1rem;
	}

	.action-btn {
		flex: 1;
		padding: 0.5rem;
		background: #334155;
		border: none;
		border-radius: 0.375rem;
		color: #e2e8f0;
		font-size: 0.875rem;
		cursor: pointer;
		transition: background 0.15s;
	}

	.action-btn:hover {
		background: #475569;
	}

	.action-btn.flag {
		background: #7f1d1d;
	}

	.action-btn.flag:hover {
		background: #991b1b;
	}

	.export-bar {
		margin-top: 1.5rem;
		text-align: right;
	}

	.export-btn {
		padding: 0.625rem 1.25rem;
		background: #3b82f6;
		border: none;
		border-radius: 0.375rem;
		color: white;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		transition: background 0.15s;
	}

	.export-btn:hover {
		background: #2563eb;
	}
</style>