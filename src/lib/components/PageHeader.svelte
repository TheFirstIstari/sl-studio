<!--
	Standardized page header. Use as the first child of a route's outer container.

	<PageHeader title="Quality Review" subtitle="Review low-confidence facts">
		{#snippet actions()}
			<button class="btn primary">Export</button>
		{/snippet}
	</PageHeader>
-->
<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		title: string;
		subtitle?: string;
		actions?: Snippet;
	}

	let { title, subtitle = '', actions }: Props = $props();
</script>

<header class="page-header">
	<div class="title-block">
		<h1>{title}</h1>
		{#if subtitle}
			<p class="subtitle">{subtitle}</p>
		{/if}
	</div>
	{#if actions}
		<div class="actions">
			{@render actions()}
		</div>
	{/if}
</header>

<style>
	.page-header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-4);
		margin-bottom: var(--space-6);
	}

	.title-block {
		flex: 1;
		min-width: 0;
	}

	.page-header h1 {
		margin: 0;
		font-size: var(--text-2xl);
		font-weight: 600;
		color: var(--color-text-primary);
	}

	.subtitle {
		margin: var(--space-1) 0 0;
		color: var(--color-text-secondary);
		font-size: var(--text-base);
	}

	.actions {
		display: flex;
		gap: var(--space-2);
		flex-shrink: 0;
	}
</style>
