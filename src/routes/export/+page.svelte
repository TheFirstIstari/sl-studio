<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { save } from '@tauri-apps/plugin-dialog';

	let exportType = $state('facts-json');
	let minWeight = $state(0.0);
	let limit = $state(1000);
	let isExporting = $state(false);
	let status = $state('');
	let exportHistory = $state<Array<{ type: string; time: string; status: string }>>([]);

	async function handleExport() {
		isExporting = true;
		status = 'Exporting...';
		try {
			let data: string | number[];
			let filename: string;
			let filters: Array<{ name: string; extensions: string[] }>;
			let isBinary = false;

			switch (exportType) {
				case 'facts-json':
					data = await invoke<string>('export_facts_json', {
						minWeight,
						limit,
						categories: null,
						startDate: null,
						endDate: null
					});
					filename = 'facts-export.json';
					filters = [{ name: 'JSON', extensions: ['json'] }];
					break;
				case 'facts-csv':
					data = await invoke<string>('export_facts_csv', { minWeight, limit });
					filename = 'facts-export.csv';
					filters = [{ name: 'CSV', extensions: ['csv'] }];
					break;
				case 'entities-csv':
					data = await invoke<string>('export_entities_csv', {
						entityType: null,
						minConfidence: 0.0
					});
					filename = 'entities-export.csv';
					filters = [{ name: 'CSV', extensions: ['csv'] }];
					break;
				case 'timeline-json':
					data = await invoke<string>('export_timeline_json', {
						startDate: null,
						endDate: null
					});
					filename = 'timeline-export.json';
					filters = [{ name: 'JSON', extensions: ['json'] }];
					break;
				case 'full-report':
					data = await invoke<string>('export_full_report_json', {});
					filename = 'full-report.json';
					filters = [{ name: 'JSON', extensions: ['json'] }];
					break;
				case 'pdf-report':
					data = await invoke<number[]>('export_pdf_report', {});
					filename = 'report.pdf';
					filters = [{ name: 'PDF', extensions: ['pdf'] }];
					isBinary = true;
					break;
				case 'excel-data':
					data = await invoke<string>('export_excel_data', {});
					filename = 'excel-data.json';
					filters = [{ name: 'JSON', extensions: ['json'] }];
					break;
				default:
					throw new Error('Unknown export type');
			}

			const filePath = await save({
				defaultPath: filename,
				filters
			});

			if (filePath) {
				let contents: number[];
				if (isBinary) {
					contents = data as number[];
				} else {
					const encoder = new TextEncoder();
					contents = Array.from(encoder.encode(data as string));
				}
				await invoke('write_file', { path: filePath, contents });
				status = `Exported to ${filePath}`;
				exportHistory = [
					{ type: exportType, time: new Date().toLocaleTimeString(), status: 'Success' },
					...exportHistory
				];
			} else {
				status = 'Export cancelled';
			}
		} catch (e) {
			status = `Error: ${e}`;
			exportHistory = [
				{ type: exportType, time: new Date().toLocaleTimeString(), status: 'Failed' },
				...exportHistory
			];
		} finally {
			isExporting = false;
		}
	}
</script>

<div class="export-page">
	<h1>Export Data</h1>

	<div class="export-form">
		<div class="form-group">
			<label for="export-type">Export Type</label>
			<select id="export-type" bind:value={exportType}>
				<option value="facts-json">Facts (JSON)</option>
				<option value="facts-csv">Facts (CSV)</option>
				<option value="entities-csv">Entities (CSV)</option>
				<option value="timeline-json">Timeline (JSON)</option>
				<option value="full-report">Full Report (JSON)</option>
				<option value="pdf-report">PDF Report</option>
				<option value="excel-data">Excel Data (JSON)</option>
			</select>
		</div>

		{#if exportType === 'facts-json' || exportType === 'facts-csv'}
			<div class="form-row">
				<div class="form-group">
					<label for="min-weight">Minimum Weight</label>
					<input id="min-weight" type="number" min="0" max="1" step="0.1" bind:value={minWeight} />
				</div>
				<div class="form-group">
					<label for="limit">Limit</label>
					<input id="limit" type="number" min="1" max="100000" bind:value={limit} />
				</div>
			</div>
		{/if}

		<button class="export-btn" onclick={handleExport} disabled={isExporting}>
			{isExporting ? 'Exporting...' : 'Export'}
		</button>

		{#if status}
			<div class="status-message">{status}</div>
		{/if}
	</div>

	{#if exportHistory.length > 0}
		<div class="export-history">
			<h2>Recent Exports</h2>
			<table>
				<thead>
					<tr>
						<th>Type</th>
						<th>Time</th>
						<th>Status</th>
					</tr>
				</thead>
				<tbody>
					{#each exportHistory as item}
						<tr>
							<td>{item.type}</td>
							<td>{item.time}</td>
							<td class={item.status.toLowerCase()}>{item.status}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>

<style>
	.export-page {
		padding: 2rem;
		max-width: 800px;
	}

	h1 {
		margin-bottom: 1.5rem;
		font-size: 1.75rem;
	}

	.export-form {
		background: var(--card-bg, #1e1e1e);
		border-radius: 8px;
		padding: 1.5rem;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	.form-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
	}

	label {
		display: block;
		margin-bottom: 0.5rem;
		font-weight: 500;
	}

	select,
	input {
		width: 100%;
		padding: 0.5rem;
		border: 1px solid #444;
		border-radius: 4px;
		background: #2a2a2a;
		color: #fff;
	}

	.export-btn {
		width: 100%;
		padding: 0.75rem;
		background: #4a9eff;
		color: white;
		border: none;
		border-radius: 4px;
		font-size: 1rem;
		cursor: pointer;
		margin-top: 1rem;
	}

	.export-btn:hover:not(:disabled) {
		background: #3a8eef;
	}

	.export-btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.status-message {
		margin-top: 1rem;
		padding: 0.75rem;
		background: #2a2a2a;
		border-radius: 4px;
		text-align: center;
	}

	.export-history {
		margin-top: 2rem;
	}

	.export-history h2 {
		font-size: 1.25rem;
		margin-bottom: 1rem;
	}

	table {
		width: 100%;
		border-collapse: collapse;
	}

	th,
	td {
		padding: 0.75rem;
		text-align: left;
		border-bottom: 1px solid #444;
	}

	th {
		background: var(--card-bg, #1e1e1e);
	}

	.success {
		color: #4caf50;
	}

	.failed {
		color: #f44336;
	}
</style>
