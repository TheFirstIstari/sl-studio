<script lang="ts">
	import '$lib/styles/theme.css';
	import { page } from '$app/stores';
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		initializeApp,
		initTheme,
		toggleTheme,
		cleanupEventListeners,
		workflow,
		projectInitialized,
		isLoading,
		error,
		refreshStats,
		refreshWorkflow,
		theme
	} from '$lib/stores/app';

	// ---------------------------------------------------------------------------
	// Navigation — 15 items (stats merged into dashboard, compare merged into results)
	// ---------------------------------------------------------------------------
	const navItems = [
		{ href: '/', label: 'Dashboard', shortcut: 'G D' },
		{ href: '/analysis', label: 'Analysis', shortcut: 'G A' },
		{ href: '/results', label: 'Results', shortcut: 'G R' },
		{ href: '/quality', label: 'Quality', shortcut: 'G Q' },
		{ href: '/chains', label: 'Chains', shortcut: 'G H' },
		{ href: '/entities', label: 'Entities', shortcut: 'G I' },
		{ href: '/pipelines', label: 'Pipelines', shortcut: 'G P' },
		{ href: '/timeline', label: 'Timeline', shortcut: 'G T' },
		{ href: '/network', label: 'Network', shortcut: 'G N' },
		{ href: '/maps', label: 'Maps', shortcut: 'G M' },
		{ href: '/anomalies', label: 'Anomalies', shortcut: 'G L' },
		{ href: '/metadata', label: 'Metadata', shortcut: 'G F' },
		{ href: '/export', label: 'Export', shortcut: 'G E' },
		{ href: '/backup', label: 'Backup', shortcut: 'G B' },
		{ href: '/settings', label: 'Settings', shortcut: 'G ,' }
	];

	let showShortcuts = $state(false);
	let pressedKeys = $state<string[]>([]);
	let initialized = $state(false);
	let refreshInterval: ReturnType<typeof setInterval> | null = null;

	// Single active backend operation (scan/extract/analyze). null when
	// idle. Used to decorate the sidebar status and Analysis nav item so
	// users can tell at a glance that work is in progress even if they
	// have navigated away from the Analysis page.
	const busyOp = $derived.by(() => {
		const w = $workflow;
		if (!w) return null;
		if (w.is_scanning) return 'Scanning';
		if (w.is_extracting) return 'Extracting';
		if (w.is_analyzing) return 'Analyzing';
		return null;
	});

	const globalShortcuts: Record<string, () => void> = {
		'?': () => (showShortcuts = !showShortcuts),
		Escape: () => (showShortcuts = false)
	};

	function handleKeydown(event: KeyboardEvent) {
		const target = event.target as HTMLElement;
		// Don't fire nav shortcuts when typing in inputs
		if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
			if (event.key === 'Escape') globalShortcuts['Escape']();
			return;
		}

		if (event.metaKey || event.ctrlKey) return;

		const key = event.key;

		if (pressedKeys.length > 0 && pressedKeys[0] === 'g') {
			const nav = navItems.find(
				(n) => n.shortcut.toLowerCase().replace('g ', '').replace(',', '') === key.toLowerCase()
			);
			if (nav) {
				goto(nav.href);
				pressedKeys = [];
				return;
			}
		}

		if (key.toLowerCase() === 'g') {
			pressedKeys = ['g'];
		} else if (pressedKeys.includes('g')) {
			pressedKeys = [];
		}

		if (globalShortcuts[key]) {
			event.preventDefault();
			globalShortcuts[key]();
		}
	}

	onMount(async () => {
		initTheme();
		window.addEventListener('keydown', handleKeydown);

		// Initialize app stores ONCE
		await initializeApp();
		initialized = true;

		// Periodic refresh every 10 s
		refreshInterval = setInterval(async () => {
			await refreshWorkflow();
			await refreshStats();
		}, 10000);
	});

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown);
		if (refreshInterval) clearInterval(refreshInterval);
		cleanupEventListeners();
	});

	let { children } = $props();
</script>

<div class="app">
	<!-- ------------------------------------------------------------------ -->
	<!-- Left sidebar -->
	<!-- ------------------------------------------------------------------ -->
	<aside class="sidebar">
		<div class="sidebar-top">
			<!-- Logo -->
			<div class="logo">
				<svg
					class="logo-icon"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					aria-hidden="true"
				>
					<circle cx="12" cy="12" r="10" />
					<path d="M12 2v4M12 18v4M2 12h4M18 12h4" />
					<circle cx="12" cy="12" r="4" />
				</svg>
				<span class="logo-text">SL Studio</span>
			</div>

			<!-- Navigation -->
			<nav aria-label="Main navigation">
				{#each navItems as item}
					<a
						href={item.href}
						class="nav-item"
						class:active={$page.url.pathname === item.href}
						aria-current={$page.url.pathname === item.href ? 'page' : undefined}
						title={item.href === '/analysis' && busyOp
							? `${busyOp} in progress`
							: `${item.label} (${item.shortcut})`}
					>
						<span class="nav-label">{item.label}</span>
						{#if item.href === '/analysis' && busyOp}
							<span class="nav-busy-dot" aria-label="{busyOp} in progress"></span>
						{/if}
					</a>
				{/each}
			</nav>
		</div>

		<!-- Sidebar footer: status + theme toggle -->
		<div class="sidebar-bottom">
			<div class="status-row">
				<span
					class="status-dot"
					class:ready={$projectInitialized && !$error && !busyOp}
					class:loading={$isLoading || !!busyOp}
					class:error={!!$error}
					aria-hidden="true"
				></span>
				<span class="status-text">
					{#if $isLoading}
						Loading…
					{:else if $error}
						Error
					{:else if busyOp}
						{busyOp}…
					{:else if $projectInitialized}
						Ready
					{:else}
						Not initialized
					{/if}
				</span>
			</div>

			<button
				class="theme-toggle"
				onclick={toggleTheme}
				title="Toggle {$theme === 'dark' ? 'light' : 'dark'} mode"
				aria-label="Toggle colour theme"
			>
				{#if $theme === 'dark'}
					<!-- Sun icon -->
					<svg
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						aria-hidden="true"
					>
						<circle cx="12" cy="12" r="5" />
						<path
							d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"
						/>
					</svg>
				{:else}
					<!-- Moon icon -->
					<svg
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						aria-hidden="true"
					>
						<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
					</svg>
				{/if}
			</button>

			<button
				class="shortcuts-btn"
				onclick={() => (showShortcuts = true)}
				title="Keyboard shortcuts (?)"
				aria-label="Show keyboard shortcuts">?</button
			>
		</div>
	</aside>

	<!-- ------------------------------------------------------------------ -->
	<!-- Main content area -->
	<!-- ------------------------------------------------------------------ -->
	<div class="content">
		<main class="main" id="main-content">
			{#if !initialized || $isLoading}
				<div class="loading-overlay">
					<div class="loading-spinner" aria-label="Loading" role="status"></div>
					<p>Initialising SL Studio…</p>
					{#if $error}
						<p class="load-error">{$error}</p>
					{/if}
				</div>
			{:else}
				{@render children()}
			{/if}
		</main>

		<!-- Workflow footer bar — only shown when a project is active -->
		{#if $projectInitialized && $workflow}
			<footer class="workflow-bar" aria-label="Workflow status">
				<span class="stage" class:active={$workflow.is_scanning}>
					Scan: {$workflow.files_scanned}
				</span>
				<span class="stage" class:active={$workflow.is_extracting}>
					Extract: {$workflow.files_extracted}
				</span>
				<span class="stage" class:active={$workflow.is_analyzing}>
					Analyze: {$workflow.files_analyzed}
				</span>
				<span class="current-file">{$workflow.current_file || 'Idle'}</span>
			</footer>
		{/if}
	</div>
</div>

<!-- -------------------------------------------------------------------- -->
<!-- Keyboard shortcuts overlay -->
<!-- -------------------------------------------------------------------- -->
{#if showShortcuts}
	<div
		class="shortcuts-overlay"
		role="dialog"
		aria-modal="true"
		aria-label="Keyboard shortcuts"
		tabindex="-1"
		onclick={() => (showShortcuts = false)}
		onkeydown={(e) => e.key === 'Escape' && (showShortcuts = false)}
	>
		<div
			class="shortcuts-panel"
			role="dialog"
			aria-label="Shortcuts list"
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
		>
			<div class="shortcuts-header">
				<h2>Keyboard Shortcuts</h2>
				<button
					class="close-btn"
					onclick={() => (showShortcuts = false)}
					aria-label="Close shortcuts panel"
				>
					<svg
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						aria-hidden="true"><path d="M18 6 6 18M6 6l12 12" /></svg
					>
				</button>
			</div>
			<ul>
				{#each navItems as item}
					<li>
						<span class="shortcut-keys">
							<kbd>G</kbd> <kbd>{item.shortcut.replace('G ', '').replace(',', ',')}</kbd>
						</span>
						<span class="shortcut-label">{item.label}</span>
					</li>
				{/each}
				<li>
					<span class="shortcut-keys"><kbd>?</kbd></span>
					<span class="shortcut-label">Toggle shortcuts</span>
				</li>
				<li>
					<span class="shortcut-keys"><kbd>Esc</kbd></span>
					<span class="shortcut-label">Close overlays</span>
				</li>
			</ul>
		</div>
	</div>
{/if}

<style>
	/* ------------------------------------------------------------------ */
	/* App shell */
	/* ------------------------------------------------------------------ */
	.app {
		display: flex;
		height: 100vh;
		overflow: hidden;
		background-color: var(--color-bg-app);
		color: var(--color-text-primary);
	}

	/* ------------------------------------------------------------------ */
	/* Sidebar */
	/* ------------------------------------------------------------------ */
	.sidebar {
		width: var(--sidebar-width);
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		background-color: var(--color-bg-sidebar);
		border-right: 1px solid var(--color-border);
		overflow: hidden;
	}

	.sidebar-top {
		flex: 1;
		overflow-y: auto;
		padding: var(--space-3) 0 var(--space-2);
	}

	/* Logo */
	.logo {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-4) var(--space-4);
	}

	.logo-icon {
		width: 22px;
		height: 22px;
		color: var(--color-accent);
		flex-shrink: 0;
	}

	.logo-text {
		font-size: var(--text-base);
		font-weight: 700;
		color: #fff;
		letter-spacing: -0.01em;
	}

	/* Nav items */
	.nav-item {
		display: flex;
		align-items: center;
		padding: var(--space-2) var(--space-4);
		text-decoration: none;
		color: rgba(255, 255, 255, 0.65);
		font-size: var(--text-sm);
		font-weight: 450;
		border-radius: 0;
		transition:
			background-color var(--transition-fast),
			color var(--transition-fast);
		border-left: 3px solid transparent;
	}

	.nav-item:hover {
		background-color: var(--color-bg-sidebar-hover);
		color: #fff;
	}

	.nav-item.active {
		background-color: var(--color-bg-sidebar-active);
		color: var(--color-accent);
		border-left-color: var(--color-accent);
		font-weight: 600;
	}

	.nav-label {
		flex: 1;
	}

	.nav-busy-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--color-accent);
		animation: pulse 1s infinite;
		margin-left: var(--space-2);
		flex-shrink: 0;
	}

	/* Sidebar bottom: status + controls */
	.sidebar-bottom {
		padding: var(--space-3) var(--space-3) var(--space-3);
		border-top: 1px solid rgba(255, 255, 255, 0.08);
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.status-row {
		flex: 1;
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-width: 0;
	}

	.status-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--color-status-unverified);
		flex-shrink: 0;
	}

	.status-dot.ready {
		background: var(--color-status-ready);
	}

	.status-dot.loading {
		background: var(--color-status-loading);
		animation: pulse 1s infinite;
	}

	.status-dot.error {
		background: var(--color-severity-high);
	}

	.status-text {
		font-size: var(--text-xs);
		color: rgba(255, 255, 255, 0.5);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.theme-toggle,
	.shortcuts-btn {
		background: none;
		border: 1px solid rgba(255, 255, 255, 0.12);
		color: rgba(255, 255, 255, 0.55);
		border-radius: var(--radius-sm);
		width: 26px;
		height: 26px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: var(--text-xs);
		flex-shrink: 0;
		transition:
			color var(--transition-fast),
			border-color var(--transition-fast),
			background-color var(--transition-fast);
	}

	.theme-toggle:hover,
	.shortcuts-btn:hover {
		color: #fff;
		border-color: rgba(255, 255, 255, 0.3);
		background-color: rgba(255, 255, 255, 0.06);
	}

	.theme-toggle svg {
		width: 14px;
		height: 14px;
	}

	/* ------------------------------------------------------------------ */
	/* Content column */
	/* ------------------------------------------------------------------ */
	.content {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-width: 0;
		overflow: hidden;
	}

	.main {
		flex: 1;
		overflow-y: auto;
		background-color: var(--color-bg-page);
		position: relative;
	}

	/* ------------------------------------------------------------------ */
	/* Loading state */
	/* ------------------------------------------------------------------ */
	.loading-overlay {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		background-color: var(--color-bg-page);
		gap: var(--space-4);
		color: var(--color-text-secondary);
	}

	.loading-spinner {
		width: 36px;
		height: 36px;
		border: 3px solid var(--color-border);
		border-top-color: var(--color-accent);
		border-radius: 50%;
		animation: spin 0.9s linear infinite;
	}

	.load-error {
		color: var(--color-severity-high);
		font-size: var(--text-sm);
	}

	/* ------------------------------------------------------------------ */
	/* Workflow footer bar */
	/* ------------------------------------------------------------------ */
	.workflow-bar {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: 0 var(--space-4);
		height: var(--footer-height);
		background-color: var(--color-bg-footer);
		border-top: 1px solid var(--color-border);
		font-size: var(--text-xs);
		color: rgba(255, 255, 255, 0.55);
		flex-shrink: 0;
	}

	.stage {
		padding: 2px var(--space-2);
		background: rgba(255, 255, 255, 0.07);
		border-radius: var(--radius-sm);
		white-space: nowrap;
	}

	.stage.active {
		background: var(--color-status-ready);
		color: #fff;
	}

	.current-file {
		flex: 1;
		text-align: right;
		opacity: 0.6;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* ------------------------------------------------------------------ */
	/* Shortcuts overlay */
	/* ------------------------------------------------------------------ */
	.shortcuts-overlay {
		position: fixed;
		inset: 0;
		background: var(--color-bg-overlay);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 200;
		backdrop-filter: blur(2px);
	}

	.shortcuts-panel {
		background: var(--color-bg-card);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-xl);
		padding: var(--space-6);
		max-width: 420px;
		width: calc(100% - var(--space-8));
		box-shadow: var(--shadow-lg);
		max-height: 80vh;
		overflow-y: auto;
	}

	.shortcuts-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: var(--space-4);
	}

	.shortcuts-header h2 {
		margin: 0;
		font-size: var(--text-lg);
		font-weight: 600;
		color: var(--color-text-primary);
	}

	.close-btn {
		background: none;
		border: none;
		color: var(--color-text-muted);
		cursor: pointer;
		padding: var(--space-1);
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-sm);
		transition: color var(--transition-fast);
	}

	.close-btn svg {
		width: 18px;
		height: 18px;
	}

	.close-btn:hover {
		color: var(--color-text-primary);
	}

	.shortcuts-panel ul {
		list-style: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.shortcuts-panel li {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-1) 0;
	}

	.shortcut-keys {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		min-width: 80px;
	}

	.shortcut-label {
		color: var(--color-text-secondary);
		font-size: var(--text-sm);
	}

	kbd {
		background-color: var(--color-bg-elevated);
		border: 1px solid var(--color-border);
		border-radius: var(--radius-sm);
		padding: 1px 6px;
		font-family: ui-monospace, 'SFMono-Regular', Menlo, Consolas, monospace;
		font-size: var(--text-xs);
		color: var(--color-text-primary);
		white-space: nowrap;
	}

	/* ------------------------------------------------------------------ */
	/* Animations */
	/* ------------------------------------------------------------------ */
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.4;
		}
	}
</style>
