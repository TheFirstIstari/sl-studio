<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';

	interface TimelineEvent {
		id: number;
		fingerprint: string;
		filename: string;
		summary: string;
		category: string | null;
		date: string;
		severity: number;
		confidence: number | null;
	}

	let events = $state<TimelineEvent[]>([]);
	let loading = $state(true);
	let viewMode = $state<'timeline' | 'list'>('timeline');
	let selectedEvent = $state<TimelineEvent | null>(null);

	onMount(async () => {
		await loadTimeline();
	});

	async function loadTimeline() {
		loading = true;
		try {
			events = await invoke<TimelineEvent[]>('get_timeline_events', {
				startDate: null,
				endDate: null,
				limit: 500
			});
		} catch (e) {
			console.error('Error loading timeline:', e);
			events = [];
		} finally {
			loading = false;
		}
	}

	function getSeverityColor(severity: number): string {
		if (severity >= 8) return '#ef4444';
		if (severity >= 6) return '#f97316';
		if (severity >= 4) return '#eab308';
		return '#4ade80';
	}

	function formatDate(dateStr: string): string {
		const date = new Date(dateStr);
		return date.toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: 'numeric'
		});
	}

	let groupedByMonth = $derived(() => {
		const groups: Record<string, TimelineEvent[]> = {};
		for (const event of events) {
			const month = event.date.substring(0, 7);
			if (!groups[month]) {
				groups[month] = [];
			}
			groups[month].push(event);
		}
		return Object.entries(groups).sort((a, b) => b[0].localeCompare(a[0]));
	});
</script>

<div class="timeline-page">
	<div class="page-header">
		<h1>Timeline</h1>

		<div class="controls">
			<button
				class="view-btn"
				class:active={viewMode === 'timeline'}
				onclick={() => (viewMode = 'timeline')}
			>
				Timeline
			</button>
			<button
				class="view-btn"
				class:active={viewMode === 'list'}
				onclick={() => (viewMode = 'list')}
			>
				List
			</button>
		</div>
	</div>

	{#if loading}
		<div class="loading">Loading timeline...</div>
	{:else if events.length === 0}
		<div class="empty">
			<svg class="empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<circle cx="12" cy="12" r="10" />
				<polyline points="12 6 12 12 16 14" />
			</svg>
			<p>No timeline events found.</p>
			<p class="empty-hint">Run analysis to generate timeline data.</p>
		</div>
	{:else if viewMode === 'timeline'}
		<div class="timeline-container">
			{#each groupedByMonth() as [month, monthEvents]}
				<div class="month-group">
					<div class="month-header">
						<span class="month-label">{month}</span>
						<span class="month-count">{monthEvents.length} events</span>
					</div>

					<div class="timeline">
						{#each monthEvents as event}
							<button
								class="timeline-item"
								class:selected={selectedEvent?.id === event.id}
								onclick={() => (selectedEvent = event)}
							>
								<div class="timeline-marker" style="background-color: {getSeverityColor(event.severity)}"></div>
								<div class="timeline-content">
									<div class="timeline-date">{formatDate(event.date)}</div>
									<div class="timeline-summary">{event.summary}</div>
									<div class="timeline-meta">
										<span class="timeline-filename">{event.filename}</span>
										{#if event.category}
											<span class="timeline-category">{event.category}</span>
										{/if}
										<span class="timeline-severity" style="background-color: {getSeverityColor(event.severity)}">
											{event.severity}
										</span>
									</div>
								</div>
							</button>
						{/each}
					</div>
				</div>
			{/each}
		</div>
	{:else}
		<div class="list-view">
			{#each events as event}
				<button
					class="list-item"
					class:selected={selectedEvent?.id === event.id}
					onclick={() => (selectedEvent = event)}
				>
					<div class="list-date">{formatDate(event.date)}</div>
					<div class="list-summary">{event.summary}</div>
					<div class="list-meta">
						<span class="list-filename">{event.filename}</span>
						<span class="list-severity" style="background-color: {getSeverityColor(event.severity)}">
							{event.severity}
						</span>
					</div>
				</button>
			{/each}
		</div>
	{/if}

	{#if selectedEvent}
		<div class="detail-panel">
			<div class="detail-header">
				<h2>Event Details</h2>
				<button class="close-btn" onclick={() => (selectedEvent = null)}>×</button>
			</div>
			<div class="detail-content">
				<div class="detail-row">
					<span class="detail-label">Date:</span>
					<span class="detail-value">{formatDate(selectedEvent.date)}</span>
				</div>
				<div class="detail-row">
					<span class="detail-label">Filename:</span>
					<span class="detail-value">{selectedEvent.filename}</span>
				</div>
				<div class="detail-row">
					<span class="detail-label">Category:</span>
					<span class="detail-value">{selectedEvent.category || 'Unknown'}</span>
				</div>
				<div class="detail-row">
					<span class="detail-label">Severity:</span>
					<span class="detail-value">
						<span class="severity-badge" style="background-color: {getSeverityColor(selectedEvent.severity)}">
							{selectedEvent.severity}/10
						</span>
					</span>
				</div>
				<div class="detail-row">
					<span class="detail-label">Confidence:</span>
					<span class="detail-value">
						{selectedEvent.confidence ? Math.round(selectedEvent.confidence * 100) : 'N/A'}%
					</span>
				</div>
				<div class="detail-section">
					<h3>Summary</h3>
					<p>{selectedEvent.summary}</p>
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.timeline-page {
		height: 100%;
		display: flex;
		flex-direction: column;
		position: relative;
	}

	.page-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.5rem;
	}

	h1 {
		font-size: 1.75rem;
		color: #eaeaea;
	}

	.controls {
		display: flex;
		gap: 0.5rem;
	}

	.view-btn {
		padding: 0.5rem 1rem;
		background-color: #16213e;
		border: 1px solid #0f3460;
		border-radius: 6px;
		color: #9ca3af;
		cursor: pointer;
		transition: all 0.2s;
	}

	.view-btn:hover {
		border-color: #e94560;
		color: #eaeaea;
	}

	.view-btn.active {
		background-color: #e94560;
		border-color: #e94560;
		color: #ffffff;
	}

	.loading,
	.empty {
		text-align: center;
		padding: 3rem;
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

	.timeline-container {
		flex: 1;
		overflow-y: auto;
		padding-right: 350px;
	}

	.month-group {
		margin-bottom: 2rem;
	}

	.month-header {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 0.75rem 1rem;
		background-color: #16213e;
		border-radius: 8px;
		margin-bottom: 1rem;
	}

	.month-label {
		font-size: 1rem;
		font-weight: 600;
		color: #e94560;
	}

	.month-count {
		font-size: 0.875rem;
		color: #9ca3af;
	}

	.timeline {
		position: relative;
		padding-left: 2rem;
	}

	.timeline::before {
		content: '';
		position: absolute;
		left: 6px;
		top: 0;
		bottom: 0;
		width: 2px;
		background-color: #0f3460;
	}

	.timeline-item {
		display: flex;
		gap: 1rem;
		padding: 1rem;
		background-color: #16213e;
		border: 1px solid #0f3460;
		border-radius: 8px;
		margin-bottom: 0.75rem;
		cursor: pointer;
		transition: all 0.2s;
		width: 100%;
		text-align: left;
	}

	.timeline-item:hover {
		border-color: #e94560;
	}

	.timeline-item.selected {
		border-color: #e94560;
		background-color: #0f3460;
	}

	.timeline-marker {
		width: 14px;
		height: 14px;
		border-radius: 50%;
		flex-shrink: 0;
		margin-top: 4px;
	}

	.timeline-content {
		flex: 1;
		min-width: 0;
	}

	.timeline-date {
		font-size: 0.875rem;
		color: #9ca3af;
		margin-bottom: 0.25rem;
	}

	.timeline-summary {
		font-size: 0.875rem;
		color: #eaeaea;
		margin-bottom: 0.5rem;
		line-height: 1.4;
	}

	.timeline-meta {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-size: 0.75rem;
	}

	.timeline-filename {
		color: #6b7280;
	}

	.timeline-category {
		color: #e94560;
		padding: 0.125rem 0.5rem;
		background-color: rgba(233, 69, 96, 0.2);
		border-radius: 4px;
	}

	.timeline-severity {
		padding: 0.125rem 0.5rem;
		border-radius: 4px;
		color: #ffffff;
		font-weight: 600;
	}

	.list-view {
		flex: 1;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding-right: 350px;
	}

	.list-item {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 1rem;
		background-color: #16213e;
		border: 1px solid #0f3460;
		border-radius: 8px;
		cursor: pointer;
		transition: all 0.2s;
		width: 100%;
		text-align: left;
	}

	.list-item:hover {
		border-color: #e94560;
	}

	.list-item.selected {
		border-color: #e94560;
		background-color: #0f3460;
	}

	.list-date {
		width: 100px;
		font-size: 0.875rem;
		color: #9ca3af;
		flex-shrink: 0;
	}

	.list-summary {
		flex: 1;
		font-size: 0.875rem;
		color: #eaeaea;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.list-meta {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.list-filename {
		font-size: 0.75rem;
		color: #6b7280;
	}

	.list-severity {
		padding: 0.125rem 0.5rem;
		border-radius: 4px;
		color: #ffffff;
		font-size: 0.75rem;
		font-weight: 600;
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
</style>
