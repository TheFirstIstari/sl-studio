<script lang="ts">
	import { page } from '$app/stores';
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { 
		initializeApp, 
		cleanupEventListeners,
		config, 
		hardware, 
		stats, 
		workflow, 
		modelLoaded,
		projectInitialized,
		isLoading,
		error,
		refreshStats,
		refreshWorkflow
	} from '$lib/stores/app';

	const navItems = [
		{ href: '/', label: 'Dashboard', icon: 'dashboard', shortcut: 'G D' },
		{ href: '/analysis', label: 'Analysis', icon: 'search', shortcut: 'G A' },
		{ href: '/results', label: 'Results', icon: 'list', shortcut: 'G R' },
		{ href: '/timeline', label: 'Timeline', icon: 'timeline', shortcut: 'G T' },
		{ href: '/stats', label: 'Statistics', icon: 'chart', shortcut: 'G S' },
		{ href: '/network', label: 'Network', icon: 'network', shortcut: 'G N' },
		{ href: '/maps', label: 'Maps', icon: 'map', shortcut: 'G M' },
		{ href: '/anomalies', label: 'Anomalies', icon: 'alert', shortcut: 'G L' },
		{ href: '/export', label: 'Export', icon: 'download', shortcut: 'G E' },
		{ href: '/compare', label: 'Compare', icon: 'compare', shortcut: 'G C' },
		{ href: '/backup', label: 'Backup', icon: 'backup', shortcut: 'G B' },
		{ href: '/settings', label: 'Settings', icon: 'settings', shortcut: 'G ,' }
	];

	let showShortcuts = $state(false);
	let pressedKeys = $state<string[]>([]);
	let initialized = $state(false);
	let refreshInterval: ReturnType<typeof setInterval> | null = null;

	const globalShortcuts: Record<string, () => void> = {
		'?': () => (showShortcuts = !showShortcuts),
		Escape: () => (showShortcuts = false)
	};

	function handleKeydown(event: KeyboardEvent) {
		const key = event.key;

		if (event.metaKey || event.ctrlKey) {
			return;
		}

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
		window.addEventListener('keydown', handleKeydown);
		
		// Initialize app stores ONCE
		await initializeApp();
		initialized = true;
		
		// Setup periodic refresh for stats (every 10 seconds)
		refreshInterval = setInterval(async () => {
			await refreshWorkflow();
			await refreshStats();
		}, 10000);
	});

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown);
		if (refreshInterval) {
			clearInterval(refreshInterval);
		}
		cleanupEventListeners();
	});

	let { children } = $props();
</script>

<div class="app">
	<header class="header">
		<div class="logo">
			<svg class="logo-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<circle cx="12" cy="12" r="10" />
				<path d="M12 2v4M12 18v4M2 12h4M18 12h4" />
				<circle cx="12" cy="12" r="4" />
			</svg>
			<span class="logo-text">SL Studio</span>
		</div>
		<div class="status">
			{#if $isLoading}
				<span class="status-dot loading"></span>
				<span class="status-text">Loading...</span>
			{:else if $error}
				<span class="status-dot error"></span>
				<span class="status-text">Error</span>
			{:else if $projectInitialized}
				<span class="status-dot ready"></span>
				<span class="status-text">Ready</span>
			{:else}
				<span class="status-dot"></span>
				<span class="status-text">Not initialized</span>
			{/if}
		</div>
	</header>

	<nav class="nav">
		{#each navItems as item}
			<a 
				href={item.href} 
				class="nav-item"
				class:active={$page.url.pathname === item.href}
				title="{item.label} ({item.shortcut})"
			>
				<span class="nav-label">{item.label}</span>
			</a>
		{/each}
	</nav>

	<main class="main">
		{#if !initialized || $isLoading}
			<div class="loading-overlay">
				<div class="loading-spinner"></div>
				<p>Initializing SL Studio...</p>
				{#if $error}
					<p class="error">{$error}</p>
				{/if}
			</div>
		{:else}
			{@render children()}
		{/if}
	</main>

	{#if showShortcuts}
		<div class="shortcuts-overlay" onclick={() => (showShortcuts = false)}>
			<div class="shortcuts-panel" onclick={(e) => e.stopPropagation()}>
				<h2>Keyboard Shortcuts</h2>
				<ul>
					{#each navItems as item}
						<li>
							<kbd>G</kbd> + <kbd>{item.shortcut.replace('G ', '').replace(',', '')}</kbd>
							<span>{item.label}</span>
						</li>
					{/each}
					<li><kbd>?</kbd><span>Toggle shortcuts</span></li>
					<li><kbd>Esc</kbd><span>Close overlays</span></li>
				</ul>
			</div>
		</div>
	{/if}

	<!-- Shared state display in footer -->
	{#if $projectInitialized && $workflow}
		<footer class="workflow-bar">
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

<style>
	.app {
		display: flex;
		flex-direction: column;
		height: 100vh;
		background: #f5f5f5;
	}

	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem 1rem;
		background: #1a1a2e;
		color: white;
	}

	.logo {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.logo-icon {
		width: 24px;
		height: 24px;
	}

	.logo-text {
		font-size: 1.125rem;
		font-weight: 600;
	}

	.status {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.875rem;
	}

	.status-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #6b7280;
	}

	.status-dot.ready {
		background: #22c55e;
	}

	.status-dot.loading {
		background: #eab308;
		animation: pulse 1s infinite;
	}

	.status-dot.error {
		background: #ef4444;
	}

	.nav {
		display: flex;
		gap: 0.25rem;
		padding: 0.5rem 1rem;
		background: white;
		border-bottom: 1px solid #e5e7eb;
		overflow-x: auto;
	}

	.nav-item {
		padding: 0.5rem 0.75rem;
		border-radius: 0.375rem;
		text-decoration: none;
		color: #374151;
		font-size: 0.875rem;
		transition: all 0.15s;
	}

	.nav-item:hover {
		background: #f3f4f6;
	}

	.nav-item.active {
		background: #1a1a2e;
		color: white;
	}

	.main {
		flex: 1;
		overflow: auto;
		position: relative;
	}

	.loading-overlay {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		background: white;
		gap: 1rem;
	}

	.loading-spinner {
		width: 40px;
		height: 40px;
		border: 3px solid #e5e7eb;
		border-top-color: #1a1a2e;
		border-radius: 50%;
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.5; }
	}

	.error {
		color: #ef4444;
	}

	.shortcuts-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0,0,0,0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 100;
	}

	.shortcuts-panel {
		background: white;
		padding: 1.5rem;
		border-radius: 0.5rem;
		max-width: 400px;
		width: 100%;
	}

	.shortcuts-panel h2 {
		margin: 0 0 1rem;
		font-size: 1.125rem;
	}

	.shortcuts-panel ul {
		list-style: none;
		padding: 0;
		margin: 0;
	}

	.shortcuts-panel li {
		display: flex;
		gap: 0.5rem;
		padding: 0.25rem 0;
	}

	.shortcuts-panel kbd {
		background: #f3f4f6;
		padding: 0.125rem 0.375rem;
		border-radius: 0.25rem;
		font-family: monospace;
		font-size: 0.75rem;
	}

	.workflow-bar {
		display: flex;
		gap: 1rem;
		padding: 0.5rem 1rem;
		background: #1a1a2e;
		color: white;
		font-size: 0.75rem;
	}

	.stage {
		padding: 0.25rem 0.5rem;
		background: rgba(255,255,255,0.1);
		border-radius: 0.25rem;
	}

	.stage.active {
		background: #22c55e;
	}

	.current-file {
		flex: 1;
		text-align: right;
		opacity: 0.7;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
