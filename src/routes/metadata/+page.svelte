<!--
	FR-META: File metadata viewer.
	Loads files from the registry, lets the investigator select one, then calls
	extract_metadata (live parse) or get_cached_metadata (DB lookup).  Results
	are shown in a two-column split: file list on the left, metadata detail on
	the right.  The investigator can cache metadata for any file so later
	lookups are instant.
-->
<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { onMount } from 'svelte';
	import { PageHeader, StatCard, FilterBar } from '$lib/components';

	// ── Types ──────────────────────────────────────────────────────────────────

	interface RegistryEntry {
		id: number;
		fingerprint: string;
		path: string;
		file_name: string;
		file_type: string | null;
		file_size: number;
	}

	interface DocumentMetadata {
		source: string; // "exif" | "pdf" | "none"
		title: string | null;
		author: string | null;
		subject: string | null;
		creator: string | null;
		producer: string | null;
		created_at: string | null;
		modified_at: string | null;
		keywords: string | null;
		camera_model: string | null;
		gps_latitude: number | null;
		gps_longitude: number | null;
		raw: Record<string, string>;
	}

	// ── State ──────────────────────────────────────────────────────────────────

	let files = $state<RegistryEntry[]>([]);
	let selected = $state<RegistryEntry | null>(null);
	let metadata = $state<DocumentMetadata | null>(null);
	let loading = $state(true);
	let extracting = $state(false);
	let caching = $state(false);
	let error = $state('');
	let search = $state('');
	let showRaw = $state(false);
	let cachedFingerprints = $state(new Set<string>());

	// ── Derived ────────────────────────────────────────────────────────────────

	let filteredFiles = $derived(
		files.filter((f) => {
			if (!search.trim()) return true;
			const q = search.toLowerCase();
			return (
				f.file_name.toLowerCase().includes(q) ||
				(f.file_type ?? '').toLowerCase().includes(q) ||
				f.path.toLowerCase().includes(q)
			);
		})
	);

	// Count files with supported metadata formats
	let supportedCount = $derived(
		files.filter((f) => {
			const ext = f.file_name.split('.').pop()?.toLowerCase() ?? '';
			return ['jpg', 'jpeg', 'png', 'tiff', 'tif', 'heic', 'heif', 'webp', 'pdf'].includes(ext);
		}).length
	);

	let cachedCount = $derived(cachedFingerprints.size);

	let hasGps = $derived(
		metadata !== null && metadata.gps_latitude !== null && metadata.gps_longitude !== null
	);

	// ── Lifecycle ──────────────────────────────────────────────────────────────

	onMount(async () => {
		await loadFiles();
	});

	// ── Helpers ────────────────────────────────────────────────────────────────

	async function loadFiles() {
		loading = true;
		error = '';
		try {
			files = await invoke<RegistryEntry[]>('get_registry_files', { limit: 2000 });
		} catch (e) {
			console.error('Failed to load registry:', e);
			error = `Failed to load files: ${e}`;
			files = [];
		} finally {
			loading = false;
		}
	}

	async function selectFile(file: RegistryEntry) {
		selected = file;
		metadata = null;
		error = '';
		showRaw = false;

		// Try cache first, fall back to live extraction.
		try {
			const cached = await invoke<DocumentMetadata | null>('get_cached_metadata', {
				fingerprint: file.fingerprint,
				metadataType: 'auto'
			});
			if (cached) {
				metadata = cached;
				cachedFingerprints = new Set([...cachedFingerprints, file.fingerprint]);
				return;
			}
		} catch {
			// Cache miss is fine – fall through to live extraction.
		}

		await extractLive(file);
	}

	async function extractLive(file: RegistryEntry) {
		extracting = true;
		error = '';
		try {
			metadata = await invoke<DocumentMetadata>('extract_metadata', { path: file.path });
		} catch (e) {
			console.error('Metadata extraction failed:', e);
			error = `Extraction failed: ${e}`;
			metadata = null;
		} finally {
			extracting = false;
		}
	}

	async function cacheMetadata() {
		if (!selected) return;
		caching = true;
		error = '';
		try {
			metadata = await invoke<DocumentMetadata>('cache_metadata', {
				fingerprint: selected.fingerprint,
				path: selected.path
			});
			cachedFingerprints = new Set([...cachedFingerprints, selected.fingerprint]);
		} catch (e) {
			console.error('Cache metadata failed:', e);
			error = `Failed to cache: ${e}`;
		} finally {
			caching = false;
		}
	}

	async function refreshMetadata() {
		if (!selected) return;
		await extractLive(selected);
	}

	function formatFileSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	function sourceLabel(source: string): string {
		if (source === 'exif') return 'EXIF';
		if (source === 'pdf') return 'PDF Info';
		return 'None';
	}

	function sourceBadgeClass(source: string): string {
		if (source === 'exif') return 'badge-exif';
		if (source === 'pdf') return 'badge-pdf';
		return 'badge-none';
	}

	async function openInMaps() {
		if (!hasGps || !metadata) return;
		const lat = metadata.gps_latitude!;
		const lon = metadata.gps_longitude!;
		// Open coordinates in OpenStreetMap (privacy-preserving, no API key).
		// Use the Tauri opener plugin so the URL is launched in the user's
		// default browser — window.open is blocked by the webview CSP.
		const url = `https://www.openstreetmap.org/?mlat=${lat}&mlon=${lon}&zoom=15`;
		try {
			await openUrl(url);
		} catch (e) {
			console.error('Failed to open URL:', e);
			error = `Failed to open browser: ${e}`;
		}
	}

	function rawEntries(raw: Record<string, string>): [string, string][] {
		return Object.entries(raw).sort(([a], [b]) => a.localeCompare(b));
	}
</script>

<div class="metadata page">
	<PageHeader
		title="Metadata"
		subtitle="Extract and inspect EXIF and document metadata from evidence files"
	>
		{#snippet actions()}
			<button class="btn ghost sm" onclick={loadFiles} disabled={loading}>
				{loading ? 'Loading...' : 'Refresh'}
			</button>
		{/snippet}
	</PageHeader>

	<!-- Summary stats -->
	<div class="stat-grid">
		<StatCard value={files.length} label="Total files" />
		<StatCard value={supportedCount} label="Metadata-capable" variant="info" />
		<StatCard value={cachedCount} label="Cached" variant="success" />
	</div>

	{#if error}
		<div class="error-banner" role="alert">{error}</div>
	{/if}

	<div class="split-layout">
		<!-- ── Left: file list ──────────────────────────────────────────────── -->
		<section class="file-panel">
			<FilterBar bind:search placeholder="Filter files..." />

			{#if loading}
				<div class="list-empty">Loading registry...</div>
			{:else if filteredFiles.length === 0}
				<div class="list-empty">
					{files.length === 0
						? 'No files in registry. Initialize a project first.'
						: 'No files match the current filter.'}
				</div>
			{:else}
				<ul class="file-list" role="listbox" aria-label="Evidence files">
					{#each filteredFiles as file (file.fingerprint)}
						<li
							class="file-item"
							class:active={selected?.fingerprint === file.fingerprint}
							class:cached={cachedFingerprints.has(file.fingerprint)}
							role="option"
							aria-selected={selected?.fingerprint === file.fingerprint}
							onclick={() => selectFile(file)}
							onkeydown={(e) => e.key === 'Enter' && selectFile(file)}
							tabindex="0"
						>
							<div class="file-name">{file.file_name}</div>
							<div class="file-meta">
								<span class="file-type">{file.file_type ?? 'unknown'}</span>
								<span class="file-size">{formatFileSize(file.file_size)}</span>
								{#if cachedFingerprints.has(file.fingerprint)}
									<span class="cached-dot" title="Metadata cached">●</span>
								{/if}
							</div>
						</li>
					{/each}
				</ul>
			{/if}
		</section>

		<!-- ── Right: metadata detail ───────────────────────────────────────── -->
		<section class="detail-panel">
			{#if !selected}
				<div class="detail-empty">
					<div class="detail-empty-icon">
						<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<path
								d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
							/>
						</svg>
					</div>
					<p>Select a file to view its metadata</p>
				</div>
			{:else if extracting}
				<div class="detail-empty">
					<div class="spinner"></div>
					<p>Extracting metadata...</p>
				</div>
			{:else if metadata}
				<!-- Header row -->
				<div class="detail-header">
					<div class="detail-title">
						<span class="detail-filename">{selected.file_name}</span>
						<span class="badge {sourceBadgeClass(metadata.source)}"
							>{sourceLabel(metadata.source)}</span
						>
						{#if cachedFingerprints.has(selected.fingerprint)}
							<span class="badge badge-cached">Cached</span>
						{/if}
					</div>
					<div class="detail-actions">
						{#if !cachedFingerprints.has(selected.fingerprint) && metadata.source !== 'none'}
							<button class="btn sm" onclick={cacheMetadata} disabled={caching}>
								{caching ? 'Caching...' : 'Cache'}
							</button>
						{/if}
						<button class="btn ghost sm" onclick={refreshMetadata} disabled={extracting}>
							Re-extract
						</button>
					</div>
				</div>

				{#if metadata.source === 'none'}
					<p class="no-meta-note">
						This file type does not carry extractable metadata (or the file has none).
					</p>
				{:else}
					<!-- Normalized fields -->
					<div class="meta-section">
						<h3 class="section-title">Normalized Fields</h3>
						<dl class="meta-grid">
							{#if metadata.title}
								<dt>Title</dt>
								<dd>{metadata.title}</dd>
							{/if}
							{#if metadata.author}
								<dt>Author</dt>
								<dd>{metadata.author}</dd>
							{/if}
							{#if metadata.subject}
								<dt>Subject</dt>
								<dd>{metadata.subject}</dd>
							{/if}
							{#if metadata.creator}
								<dt>Creator</dt>
								<dd>{metadata.creator}</dd>
							{/if}
							{#if metadata.producer}
								<dt>Producer</dt>
								<dd>{metadata.producer}</dd>
							{/if}
							{#if metadata.keywords}
								<dt>Keywords</dt>
								<dd>{metadata.keywords}</dd>
							{/if}
							{#if metadata.created_at}
								<dt>Created</dt>
								<dd>{metadata.created_at}</dd>
							{/if}
							{#if metadata.modified_at}
								<dt>Modified</dt>
								<dd>{metadata.modified_at}</dd>
							{/if}
							{#if metadata.camera_model}
								<dt>Camera</dt>
								<dd>{metadata.camera_model}</dd>
							{/if}
							{#if hasGps}
								<dt>GPS</dt>
								<dd class="gps-row">
									<span
										>{metadata.gps_latitude?.toFixed(6)}, {metadata.gps_longitude?.toFixed(6)}</span
									>
									<button class="btn ghost sm" onclick={openInMaps}>View on map</button>
								</dd>
							{/if}
						</dl>

						{#if !metadata.title && !metadata.author && !metadata.created_at && !metadata.camera_model && !hasGps}
							<p class="no-meta-note">No normalized fields found in this file's metadata.</p>
						{/if}
					</div>

					<!-- Raw key/value table -->
					<div class="meta-section">
						<div class="raw-header">
							<h3 class="section-title">Raw Tags ({Object.keys(metadata.raw).length})</h3>
							<button
								class="btn ghost sm"
								onclick={() => (showRaw = !showRaw)}
								aria-expanded={showRaw}
							>
								{showRaw ? 'Hide' : 'Show'}
							</button>
						</div>

						{#if showRaw}
							{#if Object.keys(metadata.raw).length === 0}
								<p class="no-meta-note">No raw tags.</p>
							{:else}
								<div class="raw-table-wrap">
									<table class="raw-table">
										<thead>
											<tr>
												<th>Tag</th>
												<th>Value</th>
											</tr>
										</thead>
										<tbody>
											{#each rawEntries(metadata.raw) as [key, value]}
												<tr>
													<td class="tag-key">{key}</td>
													<td class="tag-value">{value}</td>
												</tr>
											{/each}
										</tbody>
									</table>
								</div>
							{/if}
						{/if}
					</div>
				{/if}
			{:else}
				<div class="detail-empty">
					<p>No metadata could be extracted for this file.</p>
					<button class="btn sm" onclick={refreshMetadata}>Try again</button>
				</div>
			{/if}
		</section>
	</div>
</div>

<style>
	/* ── Layout ─────────────────────────────────────────────────────────────── */

	.split-layout {
		display: grid;
		grid-template-columns: 300px 1fr;
		gap: var(--space-4);
		min-height: 0;
	}

	/* ── File panel ─────────────────────────────────────────────────────────── */

	.file-panel {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		min-height: 0;
	}

	.file-list {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		max-height: calc(100vh - 280px);
		overflow-y: auto;
	}

	.file-item {
		padding: var(--space-3) var(--space-4);
		border-radius: var(--radius-md);
		border: 1px solid var(--color-border);
		background: var(--color-bg-card);
		cursor: pointer;
		transition:
			border-color 0.15s,
			background 0.15s;
	}

	.file-item:hover {
		border-color: var(--color-accent);
	}

	.file-item.active {
		border-color: var(--color-accent);
		background: var(--color-bg-elevated);
	}

	.file-item.cached {
		border-left: 3px solid var(--color-status-confirmed);
	}

	.file-name {
		font-size: var(--text-sm);
		font-weight: 500;
		color: var(--color-text-primary);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.file-meta {
		display: flex;
		gap: var(--space-2);
		margin-top: var(--space-1);
		font-size: var(--text-xs);
		color: var(--color-text-muted);
		align-items: center;
	}

	.file-type {
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.cached-dot {
		color: var(--color-status-confirmed);
		font-size: 0.6rem;
	}

	.list-empty {
		font-size: var(--text-sm);
		color: var(--color-text-muted);
		padding: var(--space-4);
		text-align: center;
	}

	/* ── Detail panel ───────────────────────────────────────────────────────── */

	.detail-panel {
		background: var(--color-bg-card);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-lg);
		padding: var(--space-6);
		min-height: 0;
		overflow-y: auto;
		max-height: calc(100vh - 240px);
	}

	.detail-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-4);
		min-height: 300px;
		color: var(--color-text-muted);
	}

	.detail-empty-icon svg {
		width: 48px;
		height: 48px;
		opacity: 0.4;
	}

	.detail-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: var(--space-4);
		margin-bottom: var(--space-6);
		padding-bottom: var(--space-4);
		border-bottom: 1px solid var(--color-border);
	}

	.detail-title {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
		min-width: 0;
	}

	.detail-filename {
		font-weight: 600;
		font-size: var(--text-lg);
		color: var(--color-text-primary);
		word-break: break-all;
	}

	.detail-actions {
		display: flex;
		gap: var(--space-2);
		flex-shrink: 0;
	}

	/* ── Badges ─────────────────────────────────────────────────────────────── */

	.badge {
		font-size: var(--text-xs);
		font-weight: 600;
		padding: 2px 8px;
		border-radius: 9999px;
		letter-spacing: 0.04em;
		text-transform: uppercase;
	}

	.badge-exif {
		background: rgba(59, 130, 246, 0.15);
		color: var(--color-entity-person);
		border: 1px solid rgba(59, 130, 246, 0.3);
	}

	.badge-pdf {
		background: rgba(233, 69, 96, 0.15);
		color: var(--color-accent);
		border: 1px solid rgba(233, 69, 96, 0.3);
	}

	.badge-none {
		background: rgba(107, 114, 128, 0.15);
		color: var(--color-text-muted);
		border: 1px solid rgba(107, 114, 128, 0.3);
	}

	.badge-cached {
		background: rgba(74, 222, 128, 0.1);
		color: var(--color-status-confirmed);
		border: 1px solid rgba(74, 222, 128, 0.25);
	}

	/* ── Metadata sections ──────────────────────────────────────────────────── */

	.meta-section {
		margin-bottom: var(--space-6);
	}

	.section-title {
		font-size: var(--text-base);
		font-weight: 600;
		color: var(--color-text-secondary);
		margin: 0 0 var(--space-3);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		font-size: var(--text-xs);
	}

	.meta-grid {
		display: grid;
		grid-template-columns: 120px 1fr;
		gap: var(--space-1) var(--space-4);
		margin: 0;
	}

	.meta-grid dt {
		font-size: var(--text-sm);
		color: var(--color-text-muted);
		font-weight: 500;
		padding-top: var(--space-1);
	}

	.meta-grid dd {
		font-size: var(--text-sm);
		color: var(--color-text-primary);
		word-break: break-word;
		margin: 0;
		padding-top: var(--space-1);
	}

	.gps-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.no-meta-note {
		font-size: var(--text-sm);
		color: var(--color-text-muted);
		font-style: italic;
		margin: 0;
	}

	/* ── Raw table ──────────────────────────────────────────────────────────── */

	.raw-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: var(--space-3);
	}

	.raw-header .section-title {
		margin: 0;
	}

	.raw-table-wrap {
		overflow-x: auto;
		border: 1px solid var(--color-border);
		border-radius: var(--radius-md);
	}

	.raw-table {
		width: 100%;
		border-collapse: collapse;
		font-size: var(--text-sm);
	}

	.raw-table th {
		background: var(--color-bg-elevated);
		color: var(--color-text-secondary);
		font-weight: 600;
		text-align: left;
		padding: var(--space-2) var(--space-3);
		font-size: var(--text-xs);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.raw-table td {
		padding: var(--space-2) var(--space-3);
		border-top: 1px solid var(--color-border);
		color: var(--color-text-primary);
		vertical-align: top;
	}

	.raw-table tr:hover td {
		background: var(--color-bg-elevated);
	}

	.tag-key {
		font-family: monospace;
		color: var(--color-text-secondary);
		white-space: nowrap;
		width: 220px;
	}

	.tag-value {
		word-break: break-all;
	}

	/* ── Spinner ────────────────────────────────────────────────────────────── */

	.spinner {
		width: 32px;
		height: 32px;
		border: 3px solid var(--color-border);
		border-top-color: var(--color-accent);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
