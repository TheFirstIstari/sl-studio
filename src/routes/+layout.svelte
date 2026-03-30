<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';

	const navItems = [
		{ href: '/', label: 'Dashboard', icon: 'dashboard', shortcut: 'G D' },
		{ href: '/analysis', label: 'Analysis', icon: 'search', shortcut: 'G A' },
		{ href: '/results', label: 'Results', icon: 'list', shortcut: 'G R' },
		{ href: '/timeline', label: 'Timeline', icon: 'timeline', shortcut: 'G T' },
		{ href: '/stats', label: 'Statistics', icon: 'chart', shortcut: 'G S' },
		{ href: '/network', label: 'Network', icon: 'network', shortcut: 'G N' },
		{ href: '/anomalies', label: 'Anomalies', icon: 'alert', shortcut: 'G L' },
		{ href: '/settings', label: 'Settings', icon: 'settings', shortcut: 'G S,' }
	];

	let showShortcuts = $state(false);
	let pressedKeys = $state<string[]>([]);

	const globalShortcuts: Record<string, () => void> = {
		'?': () => showShortcuts = !showShortcuts,
		'Escape': () => showShortcuts = false,
	};

	function handleKeydown(event: KeyboardEvent) {
		const key = event.key;
		
		if (event.metaKey || event.ctrlKey) {
			return;
		}

		if (pressedKeys.length > 0 && pressedKeys[0] === 'g') {
			const nav = navItems.find(n => n.shortcut.toLowerCase().replace('g ', '').replace(',', '') === key.toLowerCase());
			if (nav) {
				window.location.href = nav.href;
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

	onMount(() => {
		window.addEventListener('keydown', handleKeydown);
		return () => window.removeEventListener('keydown', handleKeydown);
	});
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
			<span class="status-dot"></span>
			<span class="status-text">Ready</span>
		</div>
	</header>

	<div class="main-layout">
		<nav class="sidebar">
			<ul class="nav-list">
				{#each navItems as item (item.href)}
					<li>
						<a href={item.href} class="nav-item" class:active={$page.url.pathname === item.href}>
							<svg
								class="nav-icon"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
							>
								{#if item.icon === 'dashboard'}
									<rect x="3" y="3" width="7" height="7" rx="1" />
									<rect x="14" y="3" width="7" height="7" rx="1" />
									<rect x="3" y="14" width="7" height="7" rx="1" />
									<rect x="14" y="14" width="7" height="7" rx="1" />
								{:else if item.icon === 'search'}
									<circle cx="11" cy="11" r="8" />
									<path d="M21 21l-4.35-4.35" />
							{:else if item.icon === 'list'}
								<line x1="8" y1="6" x2="21" y2="6" />
								<line x1="8" y1="12" x2="21" y2="12" />
								<line x1="8" y1="18" x2="21" y2="18" />
								<circle cx="4" cy="6" r="1" fill="currentColor" />
								<circle cx="4" cy="12" r="1" fill="currentColor" />
								<circle cx="4" cy="18" r="1" fill="currentColor" />
							{:else if item.icon === 'timeline'}
								<circle cx="12" cy="12" r="10" />
								<polyline points="12 6 12 12 16 14" />
							{:else if item.icon === 'chart'}
								<line x1="18" y1="20" x2="18" y2="10" />
								<line x1="12" y1="20" x2="12" y2="4" />
								<line x1="6" y1="20" x2="6" y2="14" />
							{:else if item.icon === 'alert'}
								<path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
								<line x1="12" y1="9" x2="12" y2="13" />
								<line x1="12" y1="17" x2="12.01" y2="17" />
							{:else if item.icon === 'network'}
								<circle cx="12" cy="5" r="3" />
								<circle cx="5" cy="19" r="3" />
								<circle cx="19" cy="19" r="3" />
								<line x1="12" y1="8" x2="5" y2="16" />
								<line x1="12" y1="8" x2="19" y2="16" />
							{:else if item.icon === 'settings'}
									<circle cx="12" cy="12" r="3" />
									<path
										d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
									/>
								{/if}
							</svg>
							<span class="nav-label">{item.label}</span>
						</a>
					</li>
				{/each}
			</ul>
		</nav>

		<main class="content">
			<slot />
		</main>
	</div>

	<footer class="footer">
		<div class="console">
			<span class="console-prompt">$</span>
			<span class="console-text">SL Studio v0.1.0</span>
		</div>
	</footer>
</div>

<style>
	:global(*) {
		box-sizing: border-box;
		margin: 0;
		padding: 0;
	}

	:global(body) {
		font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
		background-color: #1a1a2e;
		color: #eaeaea;
		overflow: hidden;
	}

	.app {
		display: flex;
		flex-direction: column;
		height: 100vh;
		background-color: #1a1a2e;
	}

	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem 1rem;
		background-color: #16213e;
		border-bottom: 1px solid #0f3460;
		-webkit-app-region: drag;
	}

	.logo {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		-webkit-app-region: no-drag;
	}

	.logo-icon {
		width: 24px;
		height: 24px;
		color: #e94560;
	}

	.logo-text {
		font-size: 1.25rem;
		font-weight: 600;
		color: #e94560;
	}

	.status {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		-webkit-app-region: no-drag;
	}

	.status-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background-color: #4ade80;
	}

	.status-text {
		font-size: 0.875rem;
		color: #9ca3af;
	}

	.main-layout {
		display: flex;
		flex: 1;
		overflow: hidden;
	}

	.sidebar {
		width: 200px;
		background-color: #16213e;
		border-right: 1px solid #0f3460;
		padding: 1rem 0;
	}

	.nav-list {
		list-style: none;
	}

	.nav-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem 1rem;
		color: #9ca3af;
		text-decoration: none;
		transition: all 0.2s;
	}

	.nav-item:hover {
		background-color: #0f3460;
		color: #eaeaea;
	}

	.nav-item.active {
		background-color: #e94560;
		color: #ffffff;
	}

	.nav-icon {
		width: 20px;
		height: 20px;
	}

	.nav-label {
		font-size: 0.875rem;
	}

	.content {
		flex: 1;
		padding: 1.5rem;
		overflow-y: auto;
		background-color: #1a1a2e;
	}

	.footer {
		padding: 0.5rem 1rem;
		background-color: #16213e;
		border-top: 1px solid #0f3460;
	}

	.console {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-family: 'SF Mono', Monaco, 'Courier New', monospace;
		font-size: 0.75rem;
	}

	.console-prompt {
		color: #4ade80;
	}

	.console-text {
		color: #9ca3af;
	}

	.shortcut-hint {
		position: fixed;
		bottom: 2rem;
		right: 2rem;
		padding: 0.5rem 1rem;
		background-color: #16213e;
		border: 1px solid #0f3460;
		border-radius: 6px;
		font-size: 0.75rem;
		color: #9ca3af;
	}

	.modal-overlay {
		position: fixed;
		inset: 0;
		background-color: rgba(0, 0, 0, 0.7);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background-color: #16213e;
		border: 1px solid #0f3460;
		border-radius: 12px;
		padding: 1.5rem;
		max-width: 500px;
		width: 90%;
		max-height: 80vh;
		overflow-y: auto;
	}

	.modal h2 {
		font-size: 1.25rem;
		color: #eaeaea;
		margin-bottom: 1rem;
	}

	.modal-section {
		margin-bottom: 1.5rem;
	}

	.modal-section h3 {
		font-size: 0.875rem;
		color: #9ca3af;
		margin-bottom: 0.75rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.shortcut-row {
		display: flex;
		justify-content: space-between;
		padding: 0.5rem 0;
		border-bottom: 1px solid #0f3460;
	}

	.shortcut-label {
		color: #eaeaea;
		font-size: 0.875rem;
	}

	.shortcut-key {
		display: flex;
		gap: 0.25rem;
	}

	.shortcut-key kbd {
		padding: 0.25rem 0.5rem;
		background-color: #1a1a2e;
		border: 1px solid #0f3460;
		border-radius: 4px;
		font-family: 'SF Mono', Monaco, monospace;
		font-size: 0.75rem;
		color: #e94560;
	}
</style>

{#if showShortcuts}
	<div class="modal-overlay" onclick={() => showShortcuts = false}>
		<div class="modal" onclick={(e) => e.stopPropagation()}>
			<h2>Keyboard Shortcuts</h2>
			
			<div class="modal-section">
				<h3>Navigation</h3>
				{#each navItems as item}
					<div class="shortcut-row">
						<span class="shortcut-label">{item.label}</span>
						<div class="shortcut-key">
							{#each item.shortcut.split(' ') as key}
								<kbd>{key}</kbd>
							{/each}
						</div>
					</div>
				{/each}
			</div>

			<div class="modal-section">
				<h3>Global</h3>
				<div class="shortcut-row">
					<span class="shortcut-label">Show Shortcuts</span>
					<div class="shortcut-key"><kbd>?</kbd></div>
				</div>
				<div class="shortcut-row">
					<span class="shortcut-label">Close Modal</span>
					<div class="shortcut-key"><kbd>Esc</kbd></div>
				</div>
			</div>
		</div>
	</div>
{:else}
	<div class="shortcut-hint">Press ? for shortcuts</div>
{/if}
