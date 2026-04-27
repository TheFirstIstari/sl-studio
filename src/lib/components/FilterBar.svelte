<!--
	Standardized filter/search bar.

	<FilterBar bind:search placeholder="Filter facts...">
		{#snippet extras()}
			<select bind:value={category} class="filter-select">...</select>
		{/snippet}
	</FilterBar>

	The `extras` snippet is rendered to the right of the search input. Pass
	any combination of selects, range sliders, etc.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		search?: string;
		placeholder?: string;
		extras?: Snippet;
	}

	let { search = $bindable(''), placeholder = 'Filter...', extras }: Props = $props();
</script>

<div class="filter-bar">
	<input class="filter-input" type="text" {placeholder} bind:value={search} />
	{#if extras}
		<div class="extras">
			{@render extras()}
		</div>
	{/if}
</div>

<style>
	.filter-bar {
		display: flex;
		gap: var(--space-3);
		align-items: center;
		background: var(--color-bg-card);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		padding: var(--space-3);
		margin-bottom: var(--space-4);
		flex-wrap: wrap;
	}

	.filter-input {
		flex: 1;
		min-width: 200px;
		background: var(--color-bg-input);
		border: 1px solid var(--color-border);
		color: var(--color-text-primary);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-md);
		font-size: var(--text-sm);
	}

	.filter-input:focus {
		outline: none;
		border-color: var(--color-accent);
	}

	.extras {
		display: flex;
		gap: var(--space-2);
		align-items: center;
		flex-wrap: wrap;
	}

	.extras :global(select),
	.extras :global(.filter-select) {
		background: var(--color-bg-input);
		border: 1px solid var(--color-border);
		color: var(--color-text-primary);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-md);
		font-size: var(--text-sm);
	}
</style>
