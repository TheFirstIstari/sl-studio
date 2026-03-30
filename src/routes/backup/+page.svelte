<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { open } from '@tauri-apps/plugin-dialog';

	let includeEvidence = $state(false);
	let isBackingUp = $state(false);
	let isRestoring = $state(false);
	let status = $state('');
	let lastBackup = $state<string | null>(null);

	async function handleBackup() {
		isBackingUp = true;
		status = 'Creating backup...';
		try {
			const result = await invoke<{ backup_path: string; size_bytes: number; created_at: string }>(
				'create_backup',
				{
					includeEvidence
				}
			);
			lastBackup = result.backup_path;
			status = `Backup created: ${(result.size_bytes / 1024 / 1024).toFixed(2)} MB`;
		} catch (e) {
			status = `Error: ${e}`;
		} finally {
			isBackingUp = false;
		}
	}

	async function handleRestore() {
		const selected = await open({
			filters: [{ name: 'ZIP Files', extensions: ['zip'] }],
			title: 'Select Backup File'
		});

		if (!selected) return;

		isRestoring = true;
		status = 'Restoring backup...';
		try {
			await invoke('restore_backup', { backupPath: selected as string });
			status = 'Backup restored successfully!';
		} catch (e) {
			status = `Error: ${e}`;
		} finally {
			isRestoring = false;
		}
	}
</script>

<div class="backup-page">
	<h1>Backup & Restore</h1>

	<div class="backup-sections">
		<div class="backup-card">
			<h2>Create Backup</h2>
			<p>Create a backup of your project data including databases and configuration.</p>

			<div class="form-group">
				<label class="checkbox-label">
					<input type="checkbox" bind:checked={includeEvidence} />
					<span>Include evidence files</span>
				</label>
				<p class="hint">Warning: Including evidence may result in large backup files</p>
			</div>

			<button class="backup-btn" onclick={handleBackup} disabled={isBackingUp}>
				{isBackingUp ? 'Creating Backup...' : 'Create Backup'}
			</button>

			{#if lastBackup}
				<div class="last-backup">
					<p>Last backup: {lastBackup.split('/').pop()}</p>
				</div>
			{/if}
		</div>

		<div class="backup-card">
			<h2>Restore Backup</h2>
			<p>Restore your project from a previously created backup file.</p>

			<div class="warning-box">
				<p>
					<strong>Warning:</strong> Restoring will overwrite your current databases. Make sure to create
					a backup first!
				</p>
			</div>

			<button class="restore-btn" onclick={handleRestore} disabled={isRestoring}>
				{isRestoring ? 'Restoring...' : 'Restore from Backup'}
			</button>
		</div>
	</div>

	{#if status}
		<div class="status-message" class:error={status.startsWith('Error')}>
			{status}
		</div>
	{/if}

	<div class="backup-info">
		<h2>Backup Information</h2>
		<ul>
			<li>Databases (registry.db, intelligence.db) are always included</li>
			<li>Configuration settings are included</li>
			<li>Evidence files can optionally be included</li>
			<li>Backups are stored in the application data directory</li>
		</ul>
	</div>
</div>

<style>
	.backup-page {
		padding: 2rem;
		max-width: 1000px;
	}

	h1 {
		margin-bottom: 1.5rem;
		font-size: 1.75rem;
	}

	h2 {
		font-size: 1.25rem;
		margin-bottom: 0.75rem;
	}

	.backup-sections {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(350px, 1fr));
		gap: 1.5rem;
		margin-bottom: 2rem;
	}

	.backup-card {
		background: var(--card-bg, #1e1e1e);
		border-radius: 8px;
		padding: 1.5rem;
	}

	.backup-card p {
		color: #888;
		margin-bottom: 1rem;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
	}

	.checkbox-label input {
		width: auto;
	}

	.hint {
		font-size: 0.875rem;
		color: #666;
		margin-top: 0.5rem;
	}

	.backup-btn,
	.restore-btn {
		width: 100%;
		padding: 0.75rem;
		border: none;
		border-radius: 4px;
		font-size: 1rem;
		cursor: pointer;
	}

	.backup-btn {
		background: #4a9eff;
		color: white;
	}

	.backup-btn:hover:not(:disabled) {
		background: #3a8eef;
	}

	.restore-btn {
		background: #ff9800;
		color: white;
	}

	.restore-btn:hover:not(:disabled) {
		background: #f57c00;
	}

	button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.last-backup {
		margin-top: 1rem;
		padding: 0.75rem;
		background: #252525;
		border-radius: 4px;
		font-size: 0.875rem;
		color: #4caf50;
	}

	.warning-box {
		background: #fff3e0;
		border-left: 4px solid #ff9800;
		padding: 1rem;
		margin-bottom: 1rem;
		border-radius: 0 4px 4px 0;
	}

	.warning-box p {
		color: #333;
		margin: 0;
	}

	.status-message {
		padding: 1rem;
		background: #252525;
		border-radius: 4px;
		margin-bottom: 1.5rem;
	}

	.status-message.error {
		background: #f4433620;
		color: #f44336;
	}

	.backup-info {
		background: var(--card-bg, #1e1e1e);
		border-radius: 8px;
		padding: 1.5rem;
	}

	.backup-info ul {
		color: #888;
		padding-left: 1.5rem;
	}

	.backup-info li {
		margin-bottom: 0.5rem;
	}
</style>
