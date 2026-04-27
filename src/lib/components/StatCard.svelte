<!--
	Compact KPI/stat card. Use inside a `.stat-grid` (defined in this file's
	companion utility CSS, see app.html / global) or any flex/grid container.

	<StatCard value={42} label="Total facts" variant="info" />
-->
<script lang="ts">
	type Variant = 'default' | 'success' | 'warning' | 'danger' | 'info';

	interface Props {
		value: string | number;
		label: string;
		variant?: Variant;
		hint?: string;
	}

	let { value, label, variant = 'default', hint }: Props = $props();
</script>

<div class="stat-card variant-{variant}">
	<div class="value">{value}</div>
	<div class="label">{label}</div>
	{#if hint}
		<div class="hint">{hint}</div>
	{/if}
</div>

<style>
	.stat-card {
		background: var(--color-bg-card);
		border: 1px solid var(--color-border);
		border-left: 3px solid var(--color-border);
		border-radius: var(--radius-lg);
		padding: var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		min-width: 0;
	}

	.value {
		font-size: var(--text-2xl);
		font-weight: 600;
		color: var(--color-text-primary);
		font-variant-numeric: tabular-nums;
		line-height: 1.1;
	}

	.label {
		font-size: var(--text-sm);
		color: var(--color-text-secondary);
	}

	.hint {
		font-size: var(--text-xs);
		color: var(--color-text-muted);
	}

	.variant-success {
		border-left-color: var(--color-status-confirmed);
	}
	.variant-warning {
		border-left-color: var(--color-severity-medium);
	}
	.variant-danger {
		border-left-color: var(--color-severity-high);
	}
	.variant-info {
		border-left-color: var(--color-status-info);
	}
</style>
