<!--
	FR-PLP: Custom multi-pass LLM pipelines.
	Lists builtin + user pipelines, lets users edit pass prompts/temperatures
	and save as a custom pipeline.
-->
<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';
	import { PageHeader, StatCard, FilterBar, Modal } from '$lib/components';

	interface PipelinePass {
		name: string;
		description: string;
		prompt_template: string;
		output_schema: string | null;
		max_tokens: number;
		temperature: number;
		sample_size: number | null;
	}

	interface Pipeline {
		id: string;
		name: string;
		description: string;
		passes: PipelinePass[];
		is_builtin: boolean;
	}

	let pipelines = $state<Pipeline[]>([]);
	let loading = $state(true);
	let error = $state('');
	let search = $state('');

	let editing = $state<Pipeline | null>(null);
	let editingId = $state(''); // for "save as" of a builtin
	let saving = $state(false);

	const filtered = $derived(
		pipelines.filter((p) => {
			if (!search.trim()) return true;
			const q = search.toLowerCase();
			return (
				p.name.toLowerCase().includes(q) ||
				p.description.toLowerCase().includes(q) ||
				p.id.toLowerCase().includes(q)
			);
		})
	);

	const stats = $derived.by(() => ({
		total: pipelines.length,
		custom: pipelines.filter((p) => !p.is_builtin).length,
		builtin: pipelines.filter((p) => p.is_builtin).length
	}));

	onMount(loadPipelines);

	async function loadPipelines() {
		loading = true;
		error = '';
		try {
			pipelines = await invoke<Pipeline[]>('list_pipelines');
		} catch (e) {
			console.error('Load pipelines failed:', e);
			error = String(e);
		} finally {
			loading = false;
		}
	}

	function startNew() {
		editing = {
			id: `custom-${Date.now()}`,
			name: 'New custom pipeline',
			description: '',
			is_builtin: false,
			passes: [
				{
					name: 'extract',
					description: 'Extract structured facts',
					prompt_template:
						'You are extracting facts from forensic evidence. Return JSON with a `facts` array.\n\nDocument:\n{{text}}',
					output_schema: null,
					max_tokens: 2000,
					temperature: 0.1,
					sample_size: null
				}
			]
		};
		editingId = editing.id;
	}

	function startEdit(p: Pipeline) {
		// Builtins are forked into a new custom pipeline.
		if (p.is_builtin) {
			editing = {
				id: `custom-${p.id}-${Date.now()}`,
				name: `${p.name} (copy)`,
				description: p.description,
				is_builtin: false,
				passes: p.passes.map((pass) => ({ ...pass }))
			};
		} else {
			editing = JSON.parse(JSON.stringify(p));
		}
		editingId = editing!.id;
	}

	function addPass() {
		if (!editing) return;
		editing.passes = [
			...editing.passes,
			{
				name: `pass_${editing.passes.length + 1}`,
				description: '',
				prompt_template: '',
				output_schema: null,
				max_tokens: 1000,
				temperature: 0.1,
				sample_size: null
			}
		];
	}

	function removePass(i: number) {
		if (!editing) return;
		editing.passes = editing.passes.filter((_, idx) => idx !== i);
	}

	async function saveEditing() {
		if (!editing) return;
		saving = true;
		error = '';
		try {
			await invoke('save_pipeline', { pipeline: editing });
			editing = null;
			await loadPipelines();
		} catch (e) {
			console.error('Save failed:', e);
			error = String(e);
		} finally {
			saving = false;
		}
	}

	async function deletePipeline(id: string) {
		if (!confirm('Delete this custom pipeline?')) return;
		try {
			await invoke('delete_pipeline', { pipelineId: id });
			await loadPipelines();
		} catch (e) {
			console.error('Delete failed:', e);
			error = String(e);
		}
	}
</script>

<div class="page">
	<PageHeader title="Pipelines" subtitle="Built-in and custom multi-pass LLM analysis pipelines">
		{#snippet actions()}
			<button class="btn primary" onclick={startNew}>+ New pipeline</button>
		{/snippet}
	</PageHeader>

	{#if error}
		<div class="error-banner">{error}</div>
	{/if}

	<div class="stat-grid">
		<StatCard value={stats.total} label="Total pipelines" />
		<StatCard value={stats.builtin} label="Built-in" variant="info" />
		<StatCard value={stats.custom} label="Custom" variant="success" />
	</div>

	<FilterBar bind:search placeholder="Filter pipelines..." />

	{#if loading}
		<div class="empty-state">Loading pipelines...</div>
	{:else if filtered.length === 0}
		<div class="empty-state">
			{pipelines.length === 0 ? 'No pipelines available.' : 'No pipelines match the filter.'}
		</div>
	{:else}
		<div class="pipeline-list">
			{#each filtered as p (p.id)}
				<div class="pipeline-row">
					<div class="pipeline-main">
						<div class="pipeline-name-row">
							<span class="pipeline-name">{p.name}</span>
							{#if p.is_builtin}
								<span class="badge builtin">built-in</span>
							{:else}
								<span class="badge custom">custom</span>
							{/if}
							<span class="muted mono">{p.id}</span>
						</div>
						{#if p.description}
							<div class="pipeline-desc">{p.description}</div>
						{/if}
						<div class="pipeline-passes">
							{p.passes.length}
							{p.passes.length === 1 ? 'pass' : 'passes'}:
							{p.passes.map((x) => x.name).join(' → ')}
						</div>
					</div>
					<div class="actions">
						<button class="btn sm" onclick={() => startEdit(p)}>
							{p.is_builtin ? 'Fork' : 'Edit'}
						</button>
						{#if !p.is_builtin}
							<button class="btn sm danger" onclick={() => deletePipeline(p.id)}> Delete </button>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<!-- Editor modal -->
<Modal
	open={editing !== null}
	title={editingId.startsWith('custom-') && !pipelines.some((p) => p.id === editingId)
		? 'New pipeline'
		: 'Edit pipeline'}
	size="lg"
	onclose={() => (editing = null)}
>
	{#snippet body()}
		{#if editing}
			<div class="editor-grid">
				<label>
					Name
					<input type="text" bind:value={editing.name} />
				</label>
				<label>
					ID (lowercase, no spaces)
					<input type="text" bind:value={editing.id} />
				</label>
				<label class="full">
					Description
					<textarea rows="2" bind:value={editing.description}></textarea>
				</label>
			</div>

			<div class="passes-header">
				<h3>Passes</h3>
				<button class="btn sm" onclick={addPass}>+ Add pass</button>
			</div>

			{#each editing.passes as pass, i (i)}
				<div class="pass-card">
					<div class="pass-header">
						<span class="pass-index">#{i + 1}</span>
						<input class="pass-name" type="text" bind:value={pass.name} />
						<button
							class="btn sm danger"
							onclick={() => removePass(i)}
							disabled={editing.passes.length === 1}
						>
							Remove
						</button>
					</div>
					<label class="full">
						Description
						<input type="text" bind:value={pass.description} />
					</label>
					<label class="full">
						Prompt template
						<textarea rows="6" bind:value={pass.prompt_template}></textarea>
					</label>
					<div class="pass-controls">
						<label>
							Max tokens
							<input type="number" min="1" bind:value={pass.max_tokens} />
						</label>
						<label>
							Temperature
							<input type="number" min="0" max="2" step="0.05" bind:value={pass.temperature} />
						</label>
					</div>
				</div>
			{/each}
		{/if}
	{/snippet}
	{#snippet footer()}
		<button class="btn ghost" onclick={() => (editing = null)}>Cancel</button>
		<button class="btn primary" onclick={saveEditing} disabled={saving}>
			{saving ? 'Saving...' : 'Save'}
		</button>
	{/snippet}
</Modal>

<style>
	.pipeline-list {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.pipeline-row {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		padding: var(--space-4);
		background: var(--color-bg-card);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
	}

	.pipeline-main {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.pipeline-name-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		flex-wrap: wrap;
	}

	.pipeline-name {
		font-weight: 600;
		color: var(--color-text-primary);
	}

	.badge {
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-size: var(--text-xs);
		padding: 2px 8px;
		border-radius: 999px;
	}

	.badge.builtin {
		background: rgba(59, 130, 246, 0.15);
		color: var(--color-status-info);
	}

	.badge.custom {
		background: rgba(74, 222, 128, 0.15);
		color: var(--color-status-confirmed);
	}

	.mono {
		font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
		font-size: var(--text-xs);
	}

	.muted {
		color: var(--color-text-muted);
	}

	.pipeline-desc {
		color: var(--color-text-secondary);
		font-size: var(--text-sm);
	}

	.pipeline-passes {
		font-size: var(--text-sm);
		color: var(--color-text-secondary);
	}

	.actions {
		display: flex;
		gap: var(--space-2);
		flex-shrink: 0;
	}

	.editor-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-3);
		margin-bottom: var(--space-4);
	}

	.editor-grid label,
	.pass-card label {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		font-size: var(--text-sm);
		color: var(--color-text-secondary);
	}

	.editor-grid label.full,
	.pass-card label.full {
		grid-column: 1 / -1;
	}

	.editor-grid input,
	.editor-grid textarea,
	.pass-card input,
	.pass-card textarea {
		background: var(--color-bg-input);
		border: 1px solid var(--color-border);
		color: var(--color-text-primary);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-md);
		font-size: var(--text-sm);
		font-family: inherit;
	}

	.editor-grid input:focus,
	.editor-grid textarea:focus,
	.pass-card input:focus,
	.pass-card textarea:focus {
		outline: none;
		border-color: var(--color-accent);
	}

	.passes-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin: var(--space-4) 0 var(--space-3);
	}

	.passes-header h3 {
		margin: 0;
		font-size: var(--text-md);
	}

	.pass-card {
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
		padding: var(--space-3);
		margin-bottom: var(--space-3);
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-3);
	}

	.pass-header {
		grid-column: 1 / -1;
		display: flex;
		gap: var(--space-2);
		align-items: center;
	}

	.pass-index {
		font-family: ui-monospace, monospace;
		color: var(--color-text-muted);
		font-size: var(--text-sm);
	}

	.pass-name {
		flex: 1;
		font-weight: 600;
	}

	.pass-controls {
		grid-column: 1 / -1;
		display: flex;
		gap: var(--space-3);
	}

	.pass-controls label {
		flex: 1;
	}
</style>
