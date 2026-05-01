<!--
	Lightweight modal: backdrop click + Escape close, content slot.

	<Modal open={showModal} title="Edit pipeline" onclose={() => showModal = false}>
		{#snippet body()}
			<p>Modal contents</p>
		{/snippet}
		{#snippet footer()}
			<button class="btn ghost" onclick={() => showModal = false}>Cancel</button>
			<button class="btn primary" onclick={save}>Save</button>
		{/snippet}
	</Modal>
-->
<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		open: boolean;
		title?: string;
		size?: 'sm' | 'md' | 'lg';
		onclose?: () => void;
		body?: Snippet;
		footer?: Snippet;
	}

	let { open, title = '', size = 'md', onclose, body, footer }: Props = $props();

	function handleBackdrop() {
		onclose?.();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && open) {
			e.preventDefault();
			onclose?.();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if open}
	<button type="button" class="modal-backdrop" onclick={handleBackdrop} aria-label="Close modal"
	></button>
	<div class="modal-shell" role="dialog" aria-modal="true" aria-label={title}>
		<div class="modal size-{size}">
			{#if title}
				<header class="modal-header">
					<h2>{title}</h2>
					<button class="close-btn" aria-label="Close" onclick={() => onclose?.()}>
					<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M18 6 6 18M6 6l12 12"/></svg>
				</button>
				</header>
			{/if}
			{#if body}
				<div class="modal-body">
					{@render body()}
				</div>
			{/if}
			{#if footer}
				<footer class="modal-footer">
					{@render footer()}
				</footer>
			{/if}
		</div>
	</div>
{/if}

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.55);
		z-index: 100;
		border: none;
		padding: 0;
		cursor: default;
	}

	.modal-shell {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-4);
		z-index: 101;
		pointer-events: none;
	}

	.modal {
		pointer-events: auto;
		background: var(--color-bg-card);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-xl);
		display: flex;
		flex-direction: column;
		max-height: calc(100vh - 4rem);
		width: 100%;
	}

	.modal.size-sm {
		max-width: 420px;
	}
	.modal.size-md {
		max-width: 640px;
	}
	.modal.size-lg {
		max-width: 960px;
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--space-4) var(--space-5);
		border-bottom: 1px solid var(--color-border);
	}

	.modal-header h2 {
		margin: 0;
		font-size: var(--text-lg);
		font-weight: 600;
	}

	.close-btn {
		background: transparent;
		border: none;
		color: var(--color-text-secondary);
		cursor: pointer;
		padding: var(--space-1);
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-sm);
	}

	.close-btn svg {
		width: 18px;
		height: 18px;
	}

	.close-btn:hover {
		color: var(--color-text-primary);
	}

	.modal-body {
		padding: var(--space-5);
		overflow-y: auto;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
		padding: var(--space-4) var(--space-5);
		border-top: 1px solid var(--color-border);
	}
</style>
