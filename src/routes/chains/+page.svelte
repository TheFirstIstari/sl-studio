<!--
	FR-CHAIN: Evidence chains.
	Lists all chains, lets users create new ones, drill into a chain to see
	its linked facts, edit metadata, or delete the chain.
-->
<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';
	import { PageHeader, StatCard, FilterBar, Modal } from '$lib/components';

	interface ChainSummary {
		id: number;
		chain_name: string;
		chain_type: string;
		description: string | null;
		created_by: string | null;
		created_at: string | null;
		updated_at: string | null;
		item_count: number;
		avg_strength: number | null;
	}

	interface ChainItem {
		link_id: number;
		intelligence_id: number;
		relationship_type: string;
		relationship_strength: number;
		notes: string | null;
		linked_by: string | null;
		linked_at: string | null;
		filename: string;
		fact_summary: string;
		category: string | null;
	}

	interface EvidenceChain {
		id: number;
		chain_name: string;
		chain_type: string;
		description: string | null;
		created_by: string | null;
		created_at: string | null;
		updated_at: string | null;
		items: ChainItem[];
	}

	let chains = $state<ChainSummary[]>([]);
	let loading = $state(true);
	let error = $state('');
	let search = $state('');

	// Create modal
	let showCreate = $state(false);
	let newName = $state('');
	let newType = $state('temporal');
	let newDescription = $state('');
	let creating = $state(false);

	// Detail view
	let activeChain = $state<EvidenceChain | null>(null);
	let detailLoading = $state(false);

	const CHAIN_TYPES = [
		{ value: 'temporal', label: 'Temporal' },
		{ value: 'transactional', label: 'Transactional' },
		{ value: 'communications', label: 'Communications' },
		{ value: 'custodial', label: 'Custodial' },
		{ value: 'other', label: 'Other' }
	];

	const filtered = $derived(
		chains.filter((c) => {
			if (!search.trim()) return true;
			const q = search.toLowerCase();
			return (
				c.chain_name.toLowerCase().includes(q) ||
				c.chain_type.toLowerCase().includes(q) ||
				(c.description ?? '').toLowerCase().includes(q)
			);
		})
	);

	const stats = $derived.by(() => {
		const totalLinks = chains.reduce((sum, c) => sum + c.item_count, 0);
		const avgStrength =
			chains.length === 0
				? 0
				: chains.reduce((s, c) => s + (c.avg_strength ?? 0), 0) / chains.length;
		return { totalLinks, avgStrength };
	});

	onMount(loadChains);

	async function loadChains() {
		loading = true;
		error = '';
		try {
			chains = await invoke<ChainSummary[]>('list_evidence_chains', {
				limit: 200,
				offset: 0
			});
		} catch (e) {
			console.error('Failed to load chains:', e);
			error = String(e);
		} finally {
			loading = false;
		}
	}

	async function createChain() {
		if (!newName.trim()) return;
		creating = true;
		try {
			await invoke<number>('create_evidence_chain', {
				name: newName.trim(),
				chainType: newType,
				description: newDescription.trim() || null,
				createdBy: null
			});
			newName = '';
			newDescription = '';
			newType = 'temporal';
			showCreate = false;
			await loadChains();
		} catch (e) {
			console.error('Create failed:', e);
			error = String(e);
		} finally {
			creating = false;
		}
	}

	async function openChain(chainId: number) {
		detailLoading = true;
		try {
			activeChain = await invoke<EvidenceChain | null>('get_evidence_chain', {
				chainId
			});
		} catch (e) {
			console.error('Open chain failed:', e);
			error = String(e);
		} finally {
			detailLoading = false;
		}
	}

	async function deleteChain(chainId: number) {
		if (!confirm('Delete this chain? Linked facts are not deleted.')) return;
		try {
			await invoke('delete_evidence_chain', { chainId });
			activeChain = null;
			await loadChains();
		} catch (e) {
			console.error('Delete failed:', e);
			error = String(e);
		}
	}

	async function unlinkItem(chainId: number, intelligenceId: number) {
		try {
			await invoke('remove_from_evidence_chain', {
				chainId,
				intelligenceId
			});
			await openChain(chainId);
			await loadChains();
		} catch (e) {
			console.error('Unlink failed:', e);
			error = String(e);
		}
	}

	function formatDate(s: string | null): string {
		if (!s) return '—';
		try {
			return new Date(s).toLocaleDateString();
		} catch {
			return s;
		}
	}
</script>

<div class="page">
	<PageHeader
		title="Evidence Chains"
		subtitle="Group related facts to document chain of custody and link sequences"
	>
		{#snippet actions()}
			<button class="btn primary" onclick={() => (showCreate = true)}>+ New Chain</button>
		{/snippet}
	</PageHeader>

	{#if error}
		<div class="error-banner">{error}</div>
	{/if}

	<div class="stat-grid">
		<StatCard value={chains.length} label="Total chains" />
		<StatCard value={stats.totalLinks} label="Linked facts" variant="info" />
		<StatCard
			value={stats.avgStrength.toFixed(2)}
			label="Avg link strength"
			variant={stats.avgStrength >= 0.7 ? 'success' : 'warning'}
		/>
	</div>

	<FilterBar bind:search placeholder="Filter chains by name, type, or description..." />

	{#if loading}
		<div class="empty-state">Loading chains...</div>
	{:else if filtered.length === 0}
		<div class="empty-state">
			{chains.length === 0
				? 'No evidence chains yet. Click "New Chain" to create one.'
				: 'No chains match the filter.'}
		</div>
	{:else}
		<div class="chain-list">
			{#each filtered as chain (chain.id)}
				<button class="chain-row" onclick={() => openChain(chain.id)}>
					<div class="chain-main">
						<div class="chain-name">{chain.chain_name}</div>
						<div class="chain-meta">
							<span class="chain-type">{chain.chain_type}</span>
							<span>{chain.item_count} {chain.item_count === 1 ? 'fact' : 'facts'}</span>
							<span class="muted">updated {formatDate(chain.updated_at)}</span>
						</div>
						{#if chain.description}
							<div class="chain-desc">{chain.description}</div>
						{/if}
					</div>
					{#if chain.avg_strength !== null}
						<div
							class="strength-pill"
							class:strong={chain.avg_strength >= 0.7}
							class:weak={chain.avg_strength < 0.4}
						>
							{chain.avg_strength.toFixed(2)}
						</div>
					{/if}
				</button>
			{/each}
		</div>
	{/if}
</div>

<!-- Create chain modal -->
<Modal open={showCreate} title="Create evidence chain" onclose={() => (showCreate = false)}>
	{#snippet body()}
		<div class="form-grid">
			<label>
				Name
				<input type="text" bind:value={newName} placeholder="e.g. Wire transfer sequence" />
			</label>
			<label>
				Type
				<select bind:value={newType}>
					{#each CHAIN_TYPES as t (t.value)}
						<option value={t.value}>{t.label}</option>
					{/each}
				</select>
			</label>
			<label>
				Description (optional)
				<textarea bind:value={newDescription} rows="3" placeholder="Why these facts belong together"
				></textarea>
			</label>
		</div>
	{/snippet}
	{#snippet footer()}
		<button class="btn ghost" onclick={() => (showCreate = false)}>Cancel</button>
		<button class="btn primary" onclick={createChain} disabled={creating || !newName.trim()}
			>{creating ? 'Creating...' : 'Create'}</button
		>
	{/snippet}
</Modal>

<!-- Chain detail modal -->
<Modal
	open={activeChain !== null}
	title={activeChain?.chain_name ?? ''}
	size="lg"
	onclose={() => (activeChain = null)}
>
	{#snippet body()}
		{#if detailLoading || !activeChain}
			<div class="empty-state">Loading...</div>
		{:else}
			<div class="detail-meta">
				<span class="chain-type">{activeChain.chain_type}</span>
				<span class="muted">created {formatDate(activeChain.created_at)}</span>
				<span class="muted">{activeChain.items.length} linked facts</span>
			</div>
			{#if activeChain.description}
				<p class="detail-desc">{activeChain.description}</p>
			{/if}

			{#if activeChain.items.length === 0}
				<p class="empty-state">
					No facts linked yet. From the Results page, use "Link to chain" to attach facts here.
				</p>
			{:else}
				<ul class="link-list">
					{#each activeChain.items as item (item.link_id)}
						<li class="link-item">
							<div class="link-main">
								<div class="link-summary">{item.fact_summary}</div>
								<div class="link-meta">
									<span class="link-type">{item.relationship_type}</span>
									<span class="muted">strength {item.relationship_strength.toFixed(2)}</span>
									<span class="muted">{item.filename}</span>
								</div>
							</div>
							<button
								class="btn sm danger"
								onclick={() => unlinkItem(activeChain!.id, item.intelligence_id)}
							>
								Unlink
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		{/if}
	{/snippet}
	{#snippet footer()}
		<button class="btn danger" onclick={() => activeChain && deleteChain(activeChain.id)}>
			Delete chain
		</button>
		<button class="btn ghost" onclick={() => (activeChain = null)}>Close</button>
	{/snippet}
</Modal>

<style>
	.chain-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.chain-row {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		padding: var(--space-4);
		background: var(--color-bg-card);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		text-align: left;
		cursor: pointer;
		color: inherit;
		font: inherit;
		transition:
			border-color 0.15s,
			background-color 0.15s;
	}

	.chain-row:hover {
		border-color: var(--color-accent);
	}

	.chain-main {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.chain-name {
		font-size: var(--text-md);
		font-weight: 600;
		color: var(--color-text-primary);
	}

	.chain-meta {
		display: flex;
		gap: var(--space-3);
		font-size: var(--text-sm);
		color: var(--color-text-secondary);
		flex-wrap: wrap;
	}

	.chain-type {
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-size: var(--text-xs);
		color: var(--color-status-info);
	}

	.chain-desc {
		font-size: var(--text-sm);
		color: var(--color-text-secondary);
	}

	.muted {
		color: var(--color-text-muted);
	}

	.strength-pill {
		font-variant-numeric: tabular-nums;
		padding: var(--space-1) var(--space-3);
		border-radius: 999px;
		background: var(--color-bg-elevated);
		color: var(--color-text-secondary);
		font-size: var(--text-xs);
		flex-shrink: 0;
	}

	.strength-pill.strong {
		background: rgba(74, 222, 128, 0.15);
		color: var(--color-status-confirmed);
	}

	.strength-pill.weak {
		background: rgba(239, 68, 68, 0.15);
		color: var(--color-severity-high);
	}

	.form-grid {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}

	.form-grid label {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		font-size: var(--text-sm);
		color: var(--color-text-secondary);
	}

	.form-grid input,
	.form-grid select,
	.form-grid textarea {
		background: var(--color-bg-input);
		border: 1px solid var(--color-border);
		color: var(--color-text-primary);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-md);
		font-size: var(--text-sm);
		font-family: inherit;
	}

	.form-grid input:focus,
	.form-grid select:focus,
	.form-grid textarea:focus {
		outline: none;
		border-color: var(--color-accent);
	}

	.detail-meta {
		display: flex;
		gap: var(--space-3);
		flex-wrap: wrap;
		margin-bottom: var(--space-3);
		font-size: var(--text-sm);
	}

	.detail-desc {
		margin: 0 0 var(--space-4);
		color: var(--color-text-secondary);
	}

	.link-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.link-item {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3);
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
	}

	.link-main {
		flex: 1;
		min-width: 0;
	}

	.link-summary {
		color: var(--color-text-primary);
		font-size: var(--text-sm);
		margin-bottom: var(--space-1);
	}

	.link-meta {
		display: flex;
		gap: var(--space-3);
		flex-wrap: wrap;
		font-size: var(--text-xs);
	}

	.link-type {
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-status-info);
	}
</style>
