<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount, onDestroy } from 'svelte';
	import L from 'leaflet';
	import 'leaflet/dist/leaflet.css';

	interface LocationEntity {
		id: number;
		name: string;
		normalized_name: string | null;
		latitude: number | null;
		longitude: number | null;
		confidence: number | null;
		fingerprint: string;
		source_file: string;
		fact_summary: string;
		severity: number;
	}

	let mapContainer: HTMLDivElement;
	let map: L.Map | null = null;
	let locations = $state<LocationEntity[]>([]);
	let loading = $state(true);
	let selectedLocation = $state<LocationEntity | null>(null);
	let minConfidence = $state(0.5);

	onMount(async () => {
		await loadLocations();
		initMap();
	});

	onDestroy(() => {
		if (map) {
			map.remove();
		}
	});

	async function loadLocations() {
		loading = true;
		try {
			locations = await invoke<LocationEntity[]>('get_location_entities', {
				minConfidence: minConfidence
			});
		} catch (e) {
			console.error('Error loading locations:', e);
			locations = [];
		} finally {
			loading = false;
		}
	}

	function initMap() {
		if (!mapContainer) return;

		map = L.map(mapContainer).setView([20, 0], 2);

		L.tileLayer('https://{s}.basemaps.cartocdn.com/dark_all/{z}/{x}/{y}{r}.png', {
			attribution:
				'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors &copy; <a href="https://carto.com/attributions">CARTO</a>',
			subdomains: 'abcd',
			maxZoom: 19
		}).addTo(map);

		addMarkers();
	}

	function addMarkers() {
		if (!map) return;

		locations.forEach((loc) => {
			if (loc.latitude !== null && loc.longitude !== null) {
				const color = getSeverityColor(loc.severity);

				const icon = L.divIcon({
					className: 'custom-marker',
					html: `<div style="background-color: ${color}; width: 12px; height: 12px; border-radius: 50%; border: 2px solid white; box-shadow: 0 0 4px rgba(0,0,0,0.5);"></div>`,
					iconSize: [16, 16],
					iconAnchor: [8, 8]
				});

				const marker = L.marker([loc.latitude, loc.longitude], { icon }).addTo(map!);

				marker.on('click', () => {
					selectedLocation = loc;
				});
			}
		});

		if (locations.filter((l) => l.latitude !== null).length > 0) {
			map.fitBounds(
				locations
					.filter((l) => l.latitude !== null)
					.map((l) => [l.latitude!, l.longitude!] as L.LatLngTupleExpression),
				{ padding: [50, 50] }
			);
		}
	}

	function getSeverityColor(severity: number): string {
		if (severity >= 8) return '#ef4444';
		if (severity >= 6) return '#f97316';
		if (severity >= 4) return '#eab308';
		return '#4ade80';
	}

	function reloadMap() {
		if (map) {
			map.remove();
		}
		loadLocations().then(initMap);
	}
</script>

<div class="maps-page">
	<div class="page-header">
		<h1>Geographic Locations</h1>

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
					onchange={reloadMap}
				/>
			</div>
			<button class="refresh-btn" onclick={reloadMap}>
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M23 4v6h-6" />
					<path d="M1 20v-6h6" />
					<path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
				</svg>
			</button>
		</div>
	</div>

	{#if loading}
		<div class="loading">Loading locations...</div>
	{:else if locations.length === 0}
		<div class="empty">
			<svg
				class="empty-icon"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
			>
				<path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z" />
				<circle cx="12" cy="10" r="3" />
			</svg>
			<p>No location entities found.</p>
			<p class="empty-hint">Run analysis to extract location entities from your evidence.</p>
		</div>
	{:else}
		<div class="map-container">
			<div class="map" bind:this={mapContainer}></div>

			{#if locations.length > 0}
				<div class="locations-panel">
					<h2>Locations ({locations.length})</h2>
					<div class="locations-list">
						{#each locations as loc}
							<button
								class="location-item"
								class:selected={selectedLocation?.id === loc.id}
								onclick={() => (selectedLocation = loc)}
							>
								<div
									class="location-dot"
									style="background-color: {getSeverityColor(loc.severity)}"
								></div>
								<div class="location-info">
									<div class="location-name">{loc.name}</div>
									<div class="location-meta">
										<span>{loc.source_file}</span>
										{#if loc.confidence}
											<span>{Math.round(loc.confidence * 100)}%</span>
										{/if}
									</div>
								</div>
							</button>
						{/each}
					</div>
				</div>
			{/if}
		</div>
	{/if}

	{#if selectedLocation}
		<div class="detail-panel">
			<div class="detail-header">
				<h2>Location Details</h2>
				<button class="close-btn" onclick={() => (selectedLocation = null)}>×</button>
			</div>
			<div class="detail-content">
				<div class="detail-row">
					<span class="detail-label">Name:</span>
					<span class="detail-value">{selectedLocation.name}</span>
				</div>

				{#if selectedLocation.normalized_name && selectedLocation.normalized_name !== selectedLocation.name}
					<div class="detail-row">
						<span class="detail-label">Normalized:</span>
						<span class="detail-value">{selectedLocation.normalized_name}</span>
					</div>
				{/if}

				<div class="detail-row">
					<span class="detail-label">Coordinates:</span>
					<span class="detail-value">
						{selectedLocation.latitude?.toFixed(4)}, {selectedLocation.longitude?.toFixed(4)}
					</span>
				</div>

				<div class="detail-row">
					<span class="detail-label">Severity:</span>
					<span class="detail-value">
						<span
							class="severity-badge"
							style="background-color: {getSeverityColor(selectedLocation.severity)}"
						>
							{selectedLocation.severity}/10
						</span>
					</span>
				</div>

				<div class="detail-row">
					<span class="detail-label">Confidence:</span>
					<span class="detail-value">
						{selectedLocation.confidence ? Math.round(selectedLocation.confidence * 100) : 'N/A'}%
					</span>
				</div>

				<div class="detail-row">
					<span class="detail-label">Source:</span>
					<span class="detail-value">{selectedLocation.source_file}</span>
				</div>

				<div class="detail-section">
					<h3>Fact Summary</h3>
					<p>{selectedLocation.fact_summary}</p>
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.maps-page {
		height: 100%;
		display: flex;
		flex-direction: column;
		position: relative;
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

	.refresh-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		background-color: #16213e;
		border: 1px solid #0f3460;
		border-radius: 6px;
		color: #9ca3af;
		cursor: pointer;
		transition: all 0.2s;
	}

	.refresh-btn:hover {
		border-color: #e94560;
		color: #eaeaea;
	}

	.refresh-btn svg {
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

	.map-container {
		flex: 1;
		display: flex;
		position: relative;
		min-height: 0;
		border-radius: 8px;
		overflow: hidden;
	}

	.map {
		flex: 1;
		background-color: #16213e;
	}

	.locations-panel {
		width: 280px;
		background-color: #16213e;
		border-left: 1px solid #0f3460;
		padding: 1rem;
		overflow-y: auto;
	}

	.locations-panel h2 {
		font-size: 1rem;
		color: #eaeaea;
		margin-bottom: 1rem;
	}

	.locations-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.location-item {
		display: flex;
		align-items: flex-start;
		gap: 0.75rem;
		padding: 0.75rem;
		background-color: #1a1a2e;
		border: 1px solid #0f3460;
		border-radius: 6px;
		cursor: pointer;
		transition: all 0.2s;
		text-align: left;
		width: 100%;
	}

	.location-item:hover {
		border-color: #e94560;
	}

	.location-item.selected {
		border-color: #e94560;
		background-color: #0f3460;
	}

	.location-dot {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		flex-shrink: 0;
		margin-top: 4px;
	}

	.location-info {
		flex: 1;
		min-width: 0;
	}

	.location-name {
		font-size: 0.875rem;
		color: #eaeaea;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.location-meta {
		display: flex;
		gap: 0.5rem;
		font-size: 0.75rem;
		color: #6b7280;
	}

	.detail-panel {
		position: fixed;
		right: 0;
		top: 0;
		bottom: 0;
		width: 350px;
		background-color: #16213e;
		border-left: 1px solid #0f3460;
		padding: 1.5rem;
		overflow-y: auto;
	}

	.detail-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.5rem;
	}

	.detail-header h2 {
		font-size: 1.25rem;
		color: #e94560;
	}

	.close-btn {
		width: 32px;
		height: 32px;
		background: none;
		border: none;
		color: #9ca3af;
		font-size: 1.5rem;
		cursor: pointer;
		border-radius: 4px;
	}

	.close-btn:hover {
		background-color: #0f3460;
		color: #eaeaea;
	}

	.detail-row {
		display: flex;
		justify-content: space-between;
		padding: 0.75rem 0;
		border-bottom: 1px solid #0f3460;
	}

	.detail-label {
		color: #9ca3af;
		font-size: 0.875rem;
	}

	.detail-value {
		color: #eaeaea;
		font-size: 0.875rem;
		font-weight: 500;
	}

	.severity-badge {
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
		font-size: 0.75rem;
		color: #ffffff;
	}

	.detail-section {
		margin-top: 1.5rem;
	}

	.detail-section h3 {
		font-size: 0.875rem;
		color: #9ca3af;
		margin-bottom: 0.5rem;
	}

	.detail-section p {
		color: #eaeaea;
		font-size: 0.875rem;
		line-height: 1.5;
	}

	:global(.custom-marker) {
		background: transparent;
		border: none;
	}

	:global(.leaflet-container) {
		background-color: #16213e;
	}
</style>
