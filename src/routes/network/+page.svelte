<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount, onDestroy } from 'svelte';
	import cytoscape from 'cytoscape';
	import { PageHeader } from '$lib/components';

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

	interface NodeDegree {
		id: string;
		value: string;
		type: string;
		degree: number;
	}

	let cyContainer: HTMLDivElement | undefined = $state();
	let cy: cytoscape.Core | null;
	let relationships = $state<EntityRelationship[]>([]);
	let connectedEntities = $state<ConnectedEntity[]>([]);
	let loading = $state(true);
	let selectedNode = $state<string | null>(null);
	let minConfidence = $state<number>(0.5);
	let nodeDegrees = $state<NodeDegree[]>([]);
	let totalNodes = $state(0);
	let totalEdges = $state(0);
	let avgConnections = $state(0);
	let selectedNodeDegree = $state(0);

	// FR-NET-005: communities + betweenness
	interface EntityCommunity {
		community_id: number;
		entity_ids: number[];
		size: number;
	}
	interface EntityBetweenness {
		entity_id: number;
		betweenness: number;
	}
	let communities = $state<EntityCommunity[]>([]);
	let betweenness = $state<EntityBetweenness[]>([]);
	let analyzingNet = $state(false);

	// Read a CSS custom property value at runtime so Cytoscape uses themed colours.
	function cssVar(name: string): string {
		return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
	}

	// Community palette — read from theme CSS variables so dark/light mode both work.
	// Computed lazily (on first graph render) via getCommunityPalette().
	function getCommunityPalette(): string[] {
		return [
			cssVar('--color-accent'),
			cssVar('--color-entity-person'),
			cssVar('--color-entity-org'),
			cssVar('--color-entity-location'),
			cssVar('--color-entity-date'),
			cssVar('--color-severity-medium-high'),
			cssVar('--color-status-info'),
			cssVar('--color-status-confirmed'),
			cssVar('--color-severity-high'),
			cssVar('--color-entity-other')
		];
	}

	function communityColor(communityId: number): string {
		// communityId is 1-indexed.
		const palette = getCommunityPalette();
		return palette[(communityId - 1) % palette.length];
	}

	function applyCommunityColors() {
		if (!cy || communities.length === 0) return;
		// Build entity_id -> community_id map.
		const map = new Map<number, number>();
		for (const c of communities) {
			for (const eid of c.entity_ids) {
				map.set(eid, c.community_id);
			}
		}
		cy.batch(() => {
			cy!.nodes().forEach((node) => {
				const idStr = node.id();
				const eid = parseInt(idStr.replace('node-', ''), 10);
				const communityId = map.get(eid);
				if (communityId !== undefined) {
					node.style('background-color', communityColor(communityId));
				}
			});
		});
	}

	async function analyzeNetwork() {
		analyzingNet = true;
		try {
			const [c, b] = await Promise.all([
				invoke<EntityCommunity[]>('detect_entity_communities', { minCooccurrence: 2 }),
				invoke<EntityBetweenness[]>('compute_betweenness_centrality', {
					minCooccurrence: 2,
					topK: 10
				})
			]);
			communities = c;
			betweenness = b;
			applyCommunityColors();
		} catch (e) {
			console.error('Network analysis failed:', e);
		} finally {
			analyzingNet = false;
		}
	}

	function entityLabel(id: number): string {
		// Build a label from the relationships data we already have.
		const rel = relationships.find((r) => r.entity1_id === id || r.entity2_id === id);
		if (!rel) return `#${id}`;
		return rel.entity1_id === id ? rel.entity1_value : rel.entity2_value;
	}

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

			// Calculate degree centrality metrics
			const degreeMap = new Map<
				string,
				{ id: string; value: string; type: string; degree: number }
			>();

			for (const rel of relationships) {
				const id1 = `node-${rel.entity1_id}`;
				const id2 = `node-${rel.entity2_id}`;

				// Initialize or increment degree for entity1
				if (!degreeMap.has(id1)) {
					degreeMap.set(id1, {
						id: id1,
						value: rel.entity1_value,
						type: rel.entity1_type,
						degree: 0
					});
				}
				const node1 = degreeMap.get(id1)!;
				node1.degree++;

				// Initialize or increment degree for entity2
				if (!degreeMap.has(id2)) {
					degreeMap.set(id2, {
						id: id2,
						value: rel.entity2_value,
						type: rel.entity2_type,
						degree: 0
					});
				}
				const node2 = degreeMap.get(id2)!;
				node2.degree++;
			}

			// Convert to array and sort by degree (descending)
			nodeDegrees = Array.from(degreeMap.values()).sort((a, b) => b.degree - a.degree);

			// Calculate network summary stats
			totalNodes = degreeMap.size;
			totalEdges = relationships.length;
			avgConnections = totalNodes > 0 ? (totalEdges * 2) / totalNodes : 0;
		} catch (e) {
			console.error('Error loading relationships:', e);
			relationships = [];
			nodeDegrees = [];
			totalNodes = 0;
			totalEdges = 0;
			avgConnections = 0;
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

		const nodeMap = new Map<string, { id: string; type: string; value: string; degree: number }>();

		// First pass: build node map with degree calculation
		for (const rel of relationships) {
			const id1 = `node-${rel.entity1_id}`;
			const id2 = `node-${rel.entity2_id}`;

			if (!nodeMap.has(id1)) {
				nodeMap.set(id1, { id: id1, type: rel.entity1_type, value: rel.entity1_value, degree: 0 });
			}
			if (!nodeMap.has(id2)) {
				nodeMap.set(id2, { id: id2, type: rel.entity2_type, value: rel.entity2_value, degree: 0 });
			}

			// Increment degrees
			const node1 = nodeMap.get(id1)!;
			const node2 = nodeMap.get(id2)!;
			node1.degree++;
			node2.degree++;
		}

		// Calculate max degree for normalization
		let maxDegree = 0;
		for (const node of nodeMap.values()) {
			if (node.degree > maxDegree) maxDegree = node.degree;
		}

		// Add edges
		for (const rel of relationships) {
			elements.push({
				data: {
					id: `edge-${rel.entity1_id}-${rel.entity2_id}`,
					source: `node-${rel.entity1_id}`,
					target: `node-${rel.entity2_id}`,
					weight: rel.cooccurrence
				}
			});
		}

		// Add nodes with degree-based sizing
		for (const node of nodeMap.values()) {
			// Scale node size: min 30, max 60 based on degree
			const sizeRatio = maxDegree > 0 ? node.degree / maxDegree : 0;
			const nodeSize = 30 + sizeRatio * 30;

			elements.push({
				data: {
					id: node.id,
					label: node.value.length > 15 ? node.value.substring(0, 15) + '...' : node.value,
					fullLabel: node.value,
					type: node.type,
					degree: node.degree,
					width: nodeSize,
					height: nodeSize
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
						'background-color': cssVar('--color-accent'),
						color: cssVar('--color-text-primary'),
						'font-size': '10px',
						'text-valign': 'bottom',
						'text-margin-y': 5,
						width: 'data(width)',
						height: 'data(height)'
					}
				},
				{
					selector: 'node[type = "PERSON"]',
					style: {
						'background-color': cssVar('--color-entity-person')
					}
				},
				{
					selector: 'node[type = "ORGANIZATION"]',
					style: {
						'background-color': cssVar('--color-entity-org')
					}
				},
				{
					selector: 'node[type = "LOCATION"]',
					style: {
						'background-color': cssVar('--color-entity-location')
					}
				},
				{
					selector: 'node[type = "DATE"]',
					style: {
						'background-color': cssVar('--color-entity-date')
					}
				},
				{
					selector: 'edge',
					style: {
						width: 'data(weight)',
						'line-color': cssVar('--color-border'),
						opacity: 0.6
					}
				},
				{
					selector: ':selected',
					style: {
						'border-width': 2,
						'border-color': cssVar('--color-text-inverse')
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
			const nodeIdNum = parseInt(nodeId.replace('node-', ''));

			// Find degree for selected node
			const nodeDegreeData = nodeDegrees.find((n) => n.id === nodeId);
			selectedNodeDegree = nodeDegreeData ? nodeDegreeData.degree : 0;

			selectedNode = nodeId;
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
				return cssVar('--color-entity-person');
			case 'ORGANIZATION':
				return cssVar('--color-entity-org');
			case 'LOCATION':
				return cssVar('--color-entity-location');
			case 'DATE':
				return cssVar('--color-entity-date');
			default:
				return cssVar('--color-accent');
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

<div class="network-page page">
	<PageHeader title="Entity Network">
		{#snippet actions()}
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
			<button class="btn sm primary" onclick={analyzeNetwork} disabled={analyzingNet}>
				{analyzingNet ? 'Analyzing...' : 'Analyze structure'}
			</button>
		{/snippet}
	</PageHeader>

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
			<div class="network-stats">
				<div class="stat-item">
					<span class="stat-value">{totalNodes}</span>
					<span class="stat-label">Total Nodes</span>
				</div>
				<div class="stat-item">
					<span class="stat-value">{totalEdges}</span>
					<span class="stat-label">Total Edges</span>
				</div>
				<div class="stat-item">
					<span class="stat-value">{avgConnections.toFixed(1)}</span>
					<span class="stat-label">Avg Connections</span>
				</div>
				{#if communities.length > 0}
					<div class="stat-item">
						<span class="stat-value">{communities.length}</span>
						<span class="stat-label">Communities</span>
					</div>
				{/if}
			</div>

			{#if communities.length > 0 || betweenness.length > 0}
				<div class="analysis-grid">
					{#if communities.length > 0}
						<div class="analysis-card">
							<h3>Communities</h3>
							<p class="analysis-hint">Connected subgraphs in the entity network.</p>
							<ul class="analysis-list">
								{#each communities.slice(0, 8) as c (c.community_id)}
									<li>
										<span class="rank">#{c.community_id}</span>
										<span>{c.size} entities</span>
										<span class="muted">
											{c.entity_ids.slice(0, 3).map(entityLabel).join(', ')}{c.entity_ids.length > 3
												? '…'
												: ''}
										</span>
									</li>
								{/each}
							</ul>
						</div>
					{/if}
					{#if betweenness.length > 0}
						<div class="analysis-card">
							<h3>Top betweenness centrality</h3>
							<p class="analysis-hint">
								Entities that bridge otherwise-disconnected parts of the network.
							</p>
							<ul class="analysis-list">
								{#each betweenness as b (b.entity_id)}
									<li>
										<span>{entityLabel(b.entity_id)}</span>
										<span class="muted">{b.betweenness.toFixed(2)}</span>
									</li>
								{/each}
							</ul>
						</div>
					{/if}
				</div>
			{/if}

			<div class="graph-container" bind:this={cyContainer}></div>

			{#if selectedNode}
				<div class="side-panel">
					<div class="panel-header">
						<h2>Selection</h2>
						<button class="close-btn" onclick={() => (selectedNode = null)} aria-label="Close">
							<svg
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
								aria-hidden="true"><path d="M18 6 6 18M6 6l12 12" /></svg
							>
						</button>
					</div>

					<div class="selection-info">
						<div class="selection-label">Selected:</div>
						<div class="selection-value">{selectedNode}</div>
					</div>

					{#if selectedNodeDegree > 0}
						<div class="degree-info">
							<div class="selection-label">Degree (Connections):</div>
							<div class="degree-value">{selectedNodeDegree}</div>
						</div>
					{/if}

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

					{#if nodeDegrees.length > 0}
						<div class="hub-list">
							<h3>Top Hubs</h3>
							{#each nodeDegrees.slice(0, 5) as hub}
								<div class="hub-item">
									<div class="hub-rank">#{nodeDegrees.indexOf(hub) + 1}</div>
									<div class="hub-info">
										<div class="hub-value">{hub.value}</div>
										<div class="hub-meta">
											<span class="hub-degree">{hub.degree} connections</span>
										</div>
									</div>
								</div>
							{/each}
						</div>
					{/if}

					<div class="legend">
						<h3>Legend</h3>
						<div class="legend-item">
							<div class="legend-dot" style="background-color: var(--color-entity-person)"></div>
							<span>Person</span>
						</div>
						<div class="legend-item">
							<div class="legend-dot" style="background-color: var(--color-entity-org)"></div>
							<span>Organization</span>
						</div>
						<div class="legend-item">
							<div class="legend-dot" style="background-color: var(--color-entity-location)"></div>
							<span>Location</span>
						</div>
						<div class="legend-item">
							<div class="legend-dot" style="background-color: var(--color-entity-date)"></div>
							<span>Date</span>
						</div>
						<div class="legend-item">
							<div class="legend-dot" style="background-color: var(--color-accent)"></div>
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

	.control-group {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.control-group label {
		font-size: 0.875rem;
		color: var(--color-text-secondary);
	}

	.control-group input {
		width: 60px;
		padding: 0.5rem;
		background-color: var(--color-bg-card);
		border: 1px solid var(--color-border);
		border-radius: 6px;
		color: var(--color-text-primary);
		font-size: 0.875rem;
	}

	.icon-btn {
		width: 36px;
		height: 36px;
		display: flex;
		align-items: center;
		justify-content: center;
		background-color: var(--color-bg-card);
		border: 1px solid var(--color-border);
		border-radius: 6px;
		color: var(--color-text-secondary);
		cursor: pointer;
		transition: all 0.2s;
	}

	.icon-btn:hover {
		border-color: var(--color-accent);
		color: var(--color-text-primary);
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
		color: var(--color-text-secondary);
	}

	.empty-icon {
		width: 48px;
		height: 48px;
		color: var(--color-text-muted);
		margin-bottom: 1rem;
	}

	.empty-hint {
		font-size: 0.875rem;
		color: var(--color-text-muted);
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
		background-color: var(--color-bg-card);
		border-radius: 8px;
		border: 1px solid var(--color-border);
	}

	.side-panel {
		width: 280px;
		background-color: var(--color-bg-card);
		border-left: 1px solid var(--color-border);
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
		color: var(--color-accent);
	}

	.close-btn {
		width: 24px;
		height: 24px;
		background: none;
		border: none;
		color: var(--color-text-secondary);
		cursor: pointer;
		border-radius: 4px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.close-btn svg {
		width: 16px;
		height: 16px;
	}

	.close-btn:hover {
		background-color: var(--color-bg-elevated);
		color: var(--color-text-primary);
	}

	.selection-info {
		padding: 0.75rem;
		background-color: var(--color-bg-input);
		border-radius: 6px;
		margin-bottom: 1rem;
	}

	.selection-label {
		font-size: 0.75rem;
		color: var(--color-text-secondary);
		margin-bottom: 0.25rem;
	}

	.selection-value {
		font-size: 0.875rem;
		color: var(--color-text-primary);
		word-break: break-all;
	}

	.connected-list h3,
	.legend h3 {
		font-size: 0.875rem;
		color: var(--color-text-secondary);
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
		border-bottom: 1px solid var(--color-border);
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
		color: var(--color-text-primary);
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
		color: var(--color-accent);
	}

	.entity-confidence {
		color: var(--color-text-secondary);
	}

	.legend-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.75rem;
		color: var(--color-text-secondary);
		margin-bottom: 0.5rem;
	}

	.legend-dot {
		width: 12px;
		height: 12px;
		border-radius: 50%;
	}

	.network-stats {
		display: flex;
		gap: 1.5rem;
		padding: 0.75rem 1rem;
		background-color: var(--color-bg-card);
		border-bottom: 1px solid var(--color-border);
	}

	.stat-item {
		display: flex;
		flex-direction: column;
		align-items: center;
	}

	.stat-value {
		font-size: 1.25rem;
		font-weight: 600;
		color: var(--color-text-primary);
	}

	.stat-label {
		font-size: 0.75rem;
		color: var(--color-text-secondary);
	}

	/* FR-NET-005 analysis grid */
	.analysis-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4);
		background: var(--color-bg-card);
		border-bottom: 1px solid var(--color-border);
	}

	.analysis-card h3 {
		margin: 0 0 var(--space-1);
		font-size: var(--text-base);
		color: var(--color-text-primary);
	}

	.analysis-hint {
		margin: 0 0 var(--space-2);
		font-size: var(--text-xs);
		color: var(--color-text-muted);
	}

	.analysis-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.analysis-list li {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-sm);
	}

	.analysis-list .rank {
		font-family: ui-monospace, SFMono-Regular, monospace;
		font-size: var(--text-xs);
		color: var(--color-status-info);
		min-width: 2rem;
	}

	.analysis-list .muted {
		color: var(--color-text-muted);
		font-size: var(--text-xs);
		flex: 1;
		text-align: right;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.degree-info {
		padding: 0.75rem;
		background-color: var(--color-bg-input);
		border-radius: 6px;
		margin-bottom: 1rem;
	}

	.degree-value {
		font-size: 1.5rem;
		font-weight: 600;
		color: var(--color-accent);
	}

	.hub-list {
		margin-bottom: 1.5rem;
	}

	.hub-item {
		display: flex;
		align-items: flex-start;
		gap: 0.5rem;
		padding: 0.5rem 0;
		border-bottom: 1px solid var(--color-border);
	}

	.hub-rank {
		font-size: 0.75rem;
		color: var(--color-text-secondary);
		min-width: 20px;
	}

	.hub-info {
		flex: 1;
		min-width: 0;
	}

	.hub-value {
		font-size: 0.875rem;
		color: var(--color-text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.hub-meta {
		font-size: 0.75rem;
	}

	.hub-degree {
		color: var(--color-entity-location);
	}
</style>
