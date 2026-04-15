<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount, onDestroy } from 'svelte';
	import cytoscape from 'cytoscape';

	interface EntityRelationship {
		entity1_id: number;
		entity1_type: string;
		entity1_value: string;
		entity2_id: number;
		entity2_type: string;
		entity2_value: string;
		cooccurrence: number;
		avg_confidence: number | null;
	}

	interface ConnectedEntity {
		entity_id: number;
		entity_type: string;
		value: string;
		confidence: number | null;
		source_file: string;
		distance: number;
	}

	let cyContainer: HTMLDivElement;
	let cy: cytoscape.Core | null = null;
	let relationships = $state<EntityRelationship[]>([]);
	let connectedEntities = $state<ConnectedEntity[]>([]);
	let loading = $state(true);
	let selectedNode = $state<string | null>(null);
	let minConfidence = $state<number>(0.5);

	onMount(async () => {
		await loadRelationships();
		initGraph();
	});

	onDestroy(() => {
		if (cy) {
			cy.destroy();
		}
	});

	async function loadRelationships() {
		loading = true;
		try {
			relationships = await invoke<EntityRelationship[]>('get_entity_relationships', {
				entityId: null,
				minConfidence: minConfidence
			});
		} catch (e) {
			console.error('Error loading relationships:', e);
			relationships = [];
		} finally {
			loading = false;
		}
	}

	async function loadConnectedEntities(entityId: number) {
		try {
			connectedEntities = await invoke<ConnectedEntity[]>('get_connected_entities', {
				entityId: entityId,
				minConfidence: minConfidence
			});
		} catch (e) {
			console.error('Error loading connected entities:', e);
			connectedEntities = [];
		}
	}

	function initGraph() {
		if (!cyContainer) return;

		const elements: cytoscape.ElementDefinition[] = [];

		const nodeMap = new Map<string, { id: string; type: string; value: string }>();

		for (const rel of relationships) {
			const id1 = `node-${rel.entity1_id}`;
			const id2 = `node-${rel.entity2_id}`;

			if (!nodeMap.has(id1)) {
				nodeMap.set(id1, { id: id1, type: rel.entity1_type, value: rel.entity1_value });
			}
			if (!nodeMap.has(id2)) {
				nodeMap.set(id2, { id: id2, type: rel.entity2_type, value: rel.entity2_value });
			}

			elements.push({
				data: {
					id: `edge-${rel.entity1_id}-${rel.entity2_id}`,
					source: id1,
					target: id2,
					weight: rel.cooccurrence
				}
			});
		}

		for (const node of nodeMap.values()) {
			elements.push({
				data: {
					id: node.id,
					label: node.value.length > 15 ? node.value.substring(0, 15) + '...' : node.value,
					fullLabel: node.value,
					type: node.type
				}
			});
		}

		cy = cytoscape({
			container: cyContainer,
			elements: elements,
			style: [
				{
					selector: 'node',
					style: {
						label: 'data(label)',
						'background-color': '#e94560',
						color: '#eaeaea',
						'font-size': '10px',
						'text-valign': 'bottom',
						'text-margin-y': 5,
						width: 30,
						height: 30
					}
				},
				{
					selector: 'node[type = "PERSON"]',
					style: {
						'background-color': '#3b82f6'
					}
				},
				{
					selector: 'node[type = "ORGANIZATION"]',
					style: {
						'background-color': '#10b981'
					}
				},
				{
					selector: 'node[type = "LOCATION"]',
					style: {
						'background-color': '#f59e0b'
					}
				},
				{
					selector: 'node[type = "DATE"]',
					style: {
						'background-color': '#8b5cf6'
					}
				},
				{
					selector: 'edge',
					style: {
						width: 'data(weight)',
						'line-color': '#0f3460',
						opacity: 0.6
					}
				},
				{
					selector: ':selected',
					style: {
						'border-width': 2,
						'border-color': '#ffffff'
					}
				}
			],
			layout: {
				name: 'cose',
				animate: true,
				animationDuration: 500
			},
			minZoom: 0.5,
			maxZoom: 3,
			wheelSensitivity: 0.3
		});

		cy.on('tap', 'node', async (evt) => {
			const nodeId = evt.target.id();
			selectedNode = nodeId;
			const nodeIdNum = parseInt(nodeId.replace('node-', ''));
			await loadConnectedEntities(nodeIdNum);
		});

		cy.on('tap', 'edge', (evt) => {
			const edge = evt.target;
			const source = edge.source().id();
			const target = edge.target().id();
			selectedNode = `${source} → ${target}`;
		});

		cy.on('tap', (evt) => {
			if (evt.target === cy) {
				selectedNode = null;
				connectedEntities = [];
			}
		});
	}

	function getTypeColor(typeName: string): string {
		switch (typeName) {
			case 'PERSON':
				return '#3b82f6';
			case 'ORGANIZATION':
				return '#10b981';
			case 'LOCATION':
				return '#f59e0b';
			case 'DATE':
				return '#8b5cf6';
			default:
				return '#e94560';
		}
	}

	function zoomIn() {
		if (cy) cy.zoom(cy.zoom() * 1.2);
	}

	function zoomOut() {
		if (cy) cy.zoom(cy.zoom() / 1.2);
	}

	function fitView() {
		if (cy) cy.fit();
	}

	function reload() {
		if (cy) cy.destroy();
		loadRelationships().then(initGraph);
	}
</script>

<div class="network-page">
	<div class="page-header">
		<h1>Entity Network</h1>

		<div class="controls">
			<div class="control-group">
				<label for="min-conf">Min Confidence:</label>
				<input
					id="min-conf"
					type="number"
					min="0"
					max="1"
					step="0.1"
					bind:value={minConfidence}
					onchange={reload}
				/>
			</div>
			<button class="icon-btn" onclick={zoomIn} title="Zoom In">
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<circle cx="11" cy="11" r="8" />
					<line x1="21" y1="21" x2="16.65" y2="16.65" />
					<line x1="11" y1="8" x2="11" y2="14" />
					<line x1="8" y1="11" x2="14" y2="11" />
				</svg>
			</button>
			<button class="icon-btn" onclick={zoomOut} title="Zoom Out">
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<circle cx="11" cy="11" r="8" />
					<line x1="21" y1="21" x2="16.65" y2="16.65" />
					<line x1="8" y1="11" x2="14" y2="11" />
				</svg>
			</button>
			<button class="icon-btn" onclick={fitView} title="Fit View">
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path
						d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3"
					/>
				</svg>
			</button>
		</div>
	</div>

	{#if loading}
		<div class="loading">Loading network...</div>
	{:else if relationships.length === 0}
		<div class="empty">
			<svg
				class="empty-icon"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
			>
				<circle cx="12" cy="12" r="10" />
				<line x1="4.93" y1="4.93" x2="19.07" y2="19.07" />
			</svg>
			<p>No entity relationships found.</p>
			<p class="empty-hint">Run analysis to extract entities and their relationships.</p>
		</div>
	{:else}
		<div class="network-container">
			<div class="graph-container" bind:this={cyContainer}></div>

			{#if selectedNode}
				<div class="side-panel">
					<div class="panel-header">
						<h2>Selection</h2>
						<button class="close-btn" onclick={() => (selectedNode = null)}>×</button>
					</div>

					<div class="selection-info">
						<div class="selection-label">Selected:</div>
						<div class="selection-value">{selectedNode}</div>
					</div>

					{#if connectedEntities.length > 0}
						<div class="connected-list">
							<h3>Connected Entities</h3>
							{#each connectedEntities as entity}
								<div class="connected-item">
									<div
										class="entity-dot"
										style="background-color: {getTypeColor(entity.entity_type)}"
									></div>
									<div class="entity-info">
										<div class="entity-value">{entity.value}</div>
										<div class="entity-meta">
											<span class="entity-type">{entity.entity_type}</span>
											{#if entity.confidence}
												<span class="entity-confidence">{Math.round(entity.confidence * 100)}%</span
												>
											{/if}
										</div>
									</div>
								</div>
							{/each}
						</div>
					{/if}

					<div class="legend">
						<h3>Legend</h3>
						<div class="legend-item">
							<div class="legend-dot" style="background-color: #3b82f6"></div>
							<span>Person</span>
						</div>
						<div class="legend-item">
							<div class="legend-dot" style="background-color: #10b981"></div>
							<span>Organization</span>
						</div>
						<div class="legend-item">
							<div class="legend-dot" style="background-color: #f59e0b"></div>
							<span>Location</span>
						</div>
						<div class="legend-item">
							<div class="legend-dot" style="background-color: #8b5cf6"></div>
							<span>Date</span>
						</div>
						<div class="legend-item">
							<div class="legend-dot" style="background-color: #e94560"></div>
							<span>Other</span>
						</div>
					</div>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.network-page {
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
		flex-wrap: wrap;
		gap: 1rem;
	}

	h1 {
		font-size: 1.75rem;
		color: #eaeaea;
	}

	.controls {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.control-group {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.control-group label {
		font-size: 0.875rem;
		color: #9ca3af;
	}

	.control-group input {
		width: 60px;
		padding: 0.5rem;
		background-color: #16213e;
		border: 1px solid #0f3460;
		border-radius: 6px;
		color: #eaeaea;
		font-size: 0.875rem;
	}

	.icon-btn {
		width: 36px;
		height: 36px;
		display: flex;
		align-items: center;
		justify-content: center;
		background-color: #16213e;
		border: 1px solid #0f3460;
		border-radius: 6px;
		color: #9ca3af;
		cursor: pointer;
		transition: all 0.2s;
	}

	.icon-btn:hover {
		border-color: #e94560;
		color: #eaeaea;
	}

	.icon-btn svg {
		width: 18px;
		height: 18px;
	}

	.loading,
	.empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		text-align: center;
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

	.network-container {
		flex: 1;
		display: flex;
		position: relative;
		min-height: 0;
	}

	.graph-container {
		flex: 1;
		background-color: #16213e;
		border-radius: 8px;
		border: 1px solid #0f3460;
	}

	.side-panel {
		width: 280px;
		background-color: #16213e;
		border-left: 1px solid #0f3460;
		padding: 1rem;
		overflow-y: auto;
	}

	.panel-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.panel-header h2 {
		font-size: 1rem;
		color: #e94560;
	}

	.close-btn {
		width: 24px;
		height: 24px;
		background: none;
		border: none;
		color: #9ca3af;
		font-size: 1.25rem;
		cursor: pointer;
		border-radius: 4px;
	}

	.close-btn:hover {
		background-color: #0f3460;
		color: #eaeaea;
	}

	.selection-info {
		padding: 0.75rem;
		background-color: #1a1a2e;
		border-radius: 6px;
		margin-bottom: 1rem;
	}

	.selection-label {
		font-size: 0.75rem;
		color: #9ca3af;
		margin-bottom: 0.25rem;
	}

	.selection-value {
		font-size: 0.875rem;
		color: #eaeaea;
		word-break: break-all;
	}

	.connected-list h3,
	.legend h3 {
		font-size: 0.875rem;
		color: #9ca3af;
		margin-bottom: 0.75rem;
	}

	.connected-list {
		margin-bottom: 1.5rem;
	}

	.connected-item {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
		padding: 0.5rem 0;
		border-bottom: 1px solid #0f3460;
	}

	.entity-dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		flex-shrink: 0;
		margin-top: 4px;
	}

	.entity-info {
		flex: 1;
		min-width: 0;
	}

	.entity-value {
		font-size: 0.875rem;
		color: #eaeaea;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.entity-meta {
		display: flex;
		gap: 0.5rem;
		font-size: 0.75rem;
	}

	.entity-type {
		color: #e94560;
	}

	.entity-confidence {
		color: #9ca3af;
	}

	.legend-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.75rem;
		color: #9ca3af;
		margin-bottom: 0.5rem;
	}

	.legend-dot {
		width: 12px;
		height: 12px;
		border-radius: 50%;
	}
</style>
