<!--
	FR-ER: Entity resolution.
	Scans the entity store for likely-duplicates (across name variations,
	punctuation, capitalization) and lets the user confirm an alias mapping.
-->
<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { PageHeader, StatCard, FilterBar } from '$lib/components';

	interface EntityMatchSuggestion {
		canonical_id: number;
		canonical_value: string;
		alias_id: number;
		alias_value: string;
		entity_type: string;
		similarity: number;
		reason: string;
	}

	let suggestions = $state<EntityMatchSuggestion[]>([]);
	let dismissed = $state(new Set<string>());
	let confirmed = $state(new Set<string>());
	let running = $state(false);
	let ranOnce = $state(false);
	let error = $state('');
	let search = $state('');

	let threshold = $state(0.8);

	function key(s: EntityMatchSuggestion): string {
		return `${s.entity_type}:${s.canonical_id}:${s.alias_id}`;
	}

	const visible = $derived(
		suggestions.filter((s) => {
			const k = key(s);
			if (dismissed.has(k) || confirmed.has(k)) return false;
			if (!search.trim()) return true;
			const q = search.toLowerCase();
			return (
				s.canonical_value.toLowerCase().includes(q) ||
				s.alias_value.toLowerCase().includes(q) ||
				s.entity_type.toLowerCase().includes(q)
			);
		})
	);

	const stats = $derived.by(() => {
		const total = suggestions.length;
		const exact = suggestions.filter((s) => s.similarity >= 0.999).length;
		return { total, exact, pending: visible.length };
	});

	async function scan() {
		running = true;
		error = '';
		try {
			suggestions = await invoke<EntityMatchSuggestion[]>('suggest_entity_matches', {
				threshold,
				perTypeLimit: 1000,
				scanLimit: 5000
			});
			dismissed = new Set();
			confirmed = new Set();
			ranOnce = true;
		} catch (e) {
			console.error('Scan failed:', e);
			error = String(e);
		} finally {
			running = false;
		}
	}

	async function confirmAlias(s: EntityMatchSuggestion) {
		try {
			await invoke('add_entity_alias', {
				canonicalId: s.canonical_id,
				alias: s.alias_value,
				aliasType: 'manual',
				confidence: s.similarity
			});
			confirmed.add(key(s));
			confirmed = new Set(confirmed); // trigger reactivity on Set
		} catch (e) {
			console.error('Add alias failed:', e);
			error = String(e);
		}
	}

	function dismiss(s: EntityMatchSuggestion) {
		dismissed.add(key(s));
		dismissed = new Set(dismissed);
	}
</script>

<div class="page">
	<PageHeader
		title="Entity Resolution"
		subtitle="Find and merge duplicate entities (e.g. 'John Smith' vs 'J. Smith')"
	>
		{#snippet actions()}
			<button class="btn primary" onclick={scan} disabled={running}>
				{running ? 'Scanning...' : ranOnce ? 'Rescan' : 'Scan for matches'}
			</button>
		{/snippet}
	</PageHeader>

	{#if error}
		<div class="error-banner">{error}</div>
	{/if}

	<div class="stat-grid">
		<StatCard value={stats.total} label="Total suggestions" />
		<StatCard value={stats.exact} label="Exact normalizations" variant="success" />
		<StatCard value={stats.pending} label="Pending review" variant="info" />
	</div>

	<FilterBar bind:search placeholder="Filter by entity value or type...">
		{#snippet extras()}
			<label class="inline-control">
				Threshold
				<input
					type="range"
					min="0.5"
					max="1"
					step="0.05"
					bind:value={threshold}
					disabled={running}
				/>
				<span class="range-value">{threshold.toFixed(2)}</span>
			</label>
		{/snippet}
	</FilterBar>

	{#if !ranOnce}
		<div class="empty-state">
			Click "Scan for matches" to surface duplicate-entity candidates from your data.
		</div>
	{:else if visible.length === 0}
		<div class="empty-state">
			No remaining suggestions. {confirmed.size > 0 ? `Confirmed ${confirmed.size} alias(es).` : ''}
		</div>
	{:else}
		<div class="match-list">
			{#each visible as s (key(s))}
				<div class="match-row">
					<div class="match-main">
						<div class="match-pair">
							<span class="entity-name canonical">{s.canonical_value}</span>
							<span class="arrow">←</span>
							<span class="entity-name alias">{s.alias_value}</span>
						</div>
						<div class="match-meta">
							<span class="entity-type">{s.entity_type}</span>
							<span class="muted">similarity {(s.similarity * 100).toFixed(0)}%</span>
							<span class="muted">{s.reason}</span>
						</div>
					</div>
					<div class="actions">
						<button class="btn sm" onclick={() => dismiss(s)}>Skip</button>
						<button class="btn sm primary" onclick={() => confirmAlias(s)}> Make alias </button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.match-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.match-row {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		padding: var(--space-4);
		background: var(--color-bg-card);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
	}

	.match-main {
		flex: 1;
		min-width: 0;
	}

	.match-pair {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		font-size: var(--text-md);
		margin-bottom: var(--space-1);
	}

	.entity-name {
		color: var(--color-text-primary);
		font-weight: 500;
	}

	.entity-name.alias {
		color: var(--color-text-secondary);
	}

	.arrow {
		color: var(--color-text-muted);
		font-weight: 700;
	}

	.match-meta {
		display: flex;
		gap: var(--space-3);
		flex-wrap: wrap;
		font-size: var(--text-sm);
	}

	.entity-type {
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-size: var(--text-xs);
		color: var(--color-status-info);
	}

	.muted {
		color: var(--color-text-muted);
	}

	.actions {
		display: flex;
		gap: var(--space-2);
		flex-shrink: 0;
	}

	.inline-control {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-sm);
		color: var(--color-text-secondary);
	}

	.inline-control input[type='range'] {
		width: 130px;
	}

	.range-value {
		font-variant-numeric: tabular-nums;
		min-width: 2.5rem;
	}
</style>
