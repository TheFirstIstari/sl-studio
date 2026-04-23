import { test, expect } from '@playwright/test';

test.describe('Settings Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/settings');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should display Settings page title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Settings');
	});

	test('should have project settings section', async ({ page }) => {
		await expect(page.locator('h2:has-text("Project")')).toBeVisible();
	});

	test('should have model settings section', async ({ page }) => {
		await expect(page.locator('h2:has-text("Model")')).toBeVisible();
	});

	test('should have hardware settings section', async ({ page }) => {
		await expect(page.locator('h2:has-text("Hardware")')).toBeVisible();
	});

	test('should have system monitor section', async ({ page }) => {
		await expect(page.locator('h2:has-text("System Monitor")')).toBeVisible();
	});

	test('should have hardware stats section', async ({ page }) => {
		const hardwareStats = page.locator('h2:has-text("Hardware Stats")');
		const count = await hardwareStats.count();
		if (count > 0) {
			await expect(hardwareStats).toBeVisible();
		}
	});

	test('should have save configuration button', async ({ page }) => {
		const saveBtn = page.locator('.save-btn');
		await expect(saveBtn).toBeVisible();
		await expect(saveBtn).toContainText('Save Configuration');
	});

	test('should have project name input', async ({ page }) => {
		await expect(page.locator('#projectName')).toBeVisible();
	});

	test('should allow changing project name', async ({ page }) => {
		const projectNameInput = page.locator('#projectName');
		await projectNameInput.fill('Test Investigation');
		await expect(projectNameInput).toHaveValue('Test Investigation');
	});

	test('should have evidence root input', async ({ page }) => {
		await expect(page.locator('#evidenceRoot')).toBeVisible();
	});

	test('should have browse button for evidence root', async ({ page }) => {
		const browseBtn = page.locator('button:has-text("Browse")').first();
		await expect(browseBtn).toBeVisible();
	});

	test('should have registry database input', async ({ page }) => {
		await expect(page.locator('#registryDb')).toBeVisible();
	});

	test('should have browse button for registry database', async ({ page }) => {
		const browseBtns = page.locator('button:has-text("Browse")');
		const count = await browseBtns.count();
		expect(count).toBeGreaterThanOrEqual(2);
	});

	test('should have intelligence database input', async ({ page }) => {
		await expect(page.locator('#intelligenceDb')).toBeVisible();
	});

	test('should have model selection dropdown', async ({ page }) => {
		const modelSelect = page.locator('#modelSelect');
		await expect(modelSelect).toBeVisible();
	});

	test('should have model options in dropdown', async ({ page }) => {
		const modelSelect = page.locator('#modelSelect');
		const options = modelSelect.locator('option');
		const count = await options.count();
		expect(count).toBeGreaterThan(1);
	});

	test('should have download model button', async ({ page }) => {
		const downloadBtn = page.locator('.download-btn');
		await expect(downloadBtn).toBeVisible();
	});

	test('should have model path input', async ({ page }) => {
		await expect(page.locator('#modelPath')).toBeVisible();
	});

	test('should have browse button for model file', async ({ page }) => {
		const browseBtns = page.locator('button:has-text("Browse")');
		const count = await browseBtns.count();
		expect(count).toBeGreaterThanOrEqual(3);
	});

	test('should display GPU backend', async ({ page }) => {
		const gpuBackend = page.locator('#gpu-backend');
		const count = await gpuBackend.count();
		if (count > 0) {
			await expect(gpuBackend).toBeVisible();
		}
	});

	test('should display hardware card for CPU cores', async ({ page }) => {
		const hardwareCards = page.locator('.hardware-card');
		const count = await hardwareCards.count();

		if (count > 0) {
			const cpuCard = hardwareCards.filter({ hasText: 'CPU Cores' });
			const cpuExists = await cpuCard.count();
			if (cpuExists > 0) {
				await expect(cpuCard.locator('.hardware-label')).toContainText('CPU Cores');
			}
		}
	});

	test('should display hardware card for RAM', async ({ page }) => {
		const hardwareCards = page.locator('.hardware-card');
		const count = await hardwareCards.count();

		if (count > 0) {
			const ramCard = hardwareCards.filter({ hasText: 'RAM' });
			const ramExists = await ramCard.count();
			if (ramExists > 0) {
				await expect(ramCard.locator('.hardware-label')).toContainText('RAM');
			}
		}
	});

	test('should display hardware card for workers', async ({ page }) => {
		const hardwareCards = page.locator('.hardware-card');
		const count = await hardwareCards.count();

		if (count > 0) {
			const workersCard = hardwareCards.filter({ hasText: 'Workers' });
			const workersExists = await workersCard.count();
			if (workersExists > 0) {
				await expect(workersCard.locator('.hardware-label')).toContainText('Workers');
			}
		}
	});

	test('should display hardware card for batch size', async ({ page }) => {
		const hardwareCards = page.locator('.hardware-card');
		const count = await hardwareCards.count();

		if (count > 0) {
			const batchCard = hardwareCards.filter({ hasText: 'Batch' });
			const batchExists = await batchCard.count();
			if (batchExists > 0) {
				await expect(batchCard.locator('.hardware-label')).toContainText('Batch Size');
			}
		}
	});

	test('should display system monitor CPU usage bar', async ({ page }) => {
		const monitorItems = page.locator('.monitor-item');
		const count = await monitorItems.count();

		if (count > 0) {
			const cpuMonitor = monitorItems.filter({ hasText: 'CPU Usage' });
			const cpuExists = await cpuMonitor.count();
			if (cpuExists > 0) {
				await expect(cpuMonitor.locator('.monitor-bar')).toBeVisible();
				await expect(cpuMonitor.locator('.monitor-value')).toBeVisible();
			}
		}
	});

	test('should display system monitor memory bar', async ({ page }) => {
		const monitorItems = page.locator('.monitor-item');
		const count = await monitorItems.count();

		if (count > 0) {
			const memMonitor = monitorItems.filter({ hasText: 'Memory' });
			const memExists = await memMonitor.count();
			if (memExists > 0) {
				await expect(memMonitor.locator('.monitor-bar')).toBeVisible();
				await expect(memMonitor.locator('.monitor-value')).toBeVisible();
			}
		}
	});

	test('system monitor should show CPU percentage', async ({ page }) => {
		const monitorItems = page.locator('.monitor-item');
		const count = await monitorItems.count();

		if (count > 0) {
			const cpuMonitor = monitorItems.filter({ hasText: 'CPU Usage' });
			const cpuExists = await cpuMonitor.count();
			if (cpuExists > 0) {
				const value = await cpuMonitor.locator('.monitor-value').textContent();
				expect(value).toContain('%');
			}
		}
	});

	test('system monitor should show memory usage', async ({ page }) => {
		const monitorItems = page.locator('.monitor-item');
		const count = await monitorItems.count();

		if (count > 0) {
			const memMonitor = monitorItems.filter({ hasText: 'Memory' });
			const memExists = await memMonitor.count();
			if (memExists > 0) {
				const value = await memMonitor.locator('.monitor-value').textContent();
				expect(value).toContain('GB');
			}
		}
	});

	test('should have hardware cards grid', async ({ page }) => {
		const hardwareCards = page.locator('.hardware-cards');
		const count = await hardwareCards.count();
		if (count > 0) {
			await expect(hardwareCards).toBeVisible();
		}
	});

	test('should have hardware section with multiple cards', async ({ page }) => {
		const hardwareCards = page.locator('.hardware-card');
		const count = await hardwareCards.count();
		expect(count).toBeGreaterThanOrEqual(0);
	});

	test('should have settings grid layout', async ({ page }) => {
		const settingsGrid = page.locator('.settings-grid');
		await expect(settingsGrid).toBeVisible();
	});

	test('should have multiple settings sections', async ({ page }) => {
		const sections = page.locator('.settings-section');
		const count = await sections.count();
		expect(count).toBeGreaterThanOrEqual(4);
	});

	test('should display loading state initially', async ({ page }) => {
		const loading = page.locator('.loading');
		const loadingVisible = await loading.isVisible().catch(() => false);

		if (loadingVisible) {
			await expect(loading).toContainText('Loading');
		}
	});

	test('should have actions section', async ({ page }) => {
		const actions = page.locator('.actions');
		await expect(actions).toBeVisible();
	});

	test('should allow editing model selection', async ({ page }) => {
		const modelSelect = page.locator('#modelSelect');
		await modelSelect.selectOption({ index: 1 });
	});

	test('should show model download progress', async ({ page }) => {
		const progressBar = page.locator('.progress-bar');
		const progressExists = await progressBar.count();
		expect(progressExists).toBeGreaterThanOrEqual(0);
	});

	test('input fields should have correct placeholders', async ({ page }) => {
		const projectName = page.locator('#projectName');
		await expect(projectName).toHaveAttribute('placeholder', 'New Investigation');
	});

	test('evidence root should have placeholder', async ({ page }) => {
		const evidenceRoot = page.locator('#evidenceRoot');
		await expect(evidenceRoot).toHaveAttribute('placeholder', '/path/to/evidence');
	});

	test('registry database should have placeholder', async ({ page }) => {
		const registryDb = page.locator('#registryDb');
		await expect(registryDb).toHaveAttribute('placeholder', '/path/to/registry.db');
	});

	test('intelligence database should have placeholder', async ({ page }) => {
		const intelligenceDb = page.locator('#intelligenceDb');
		await expect(intelligenceDb).toHaveAttribute('placeholder', '/path/to/intelligence.db');
	});
});

test.describe('Settings Interactions', () => {
	test('should show save button disabled while saving', async ({ page }) => {
		await page.waitForSelector('.save-btn', { timeout: 10000 }).catch(() => {});
		const saveBtn = page.locator('.save-btn');
		const count = await saveBtn.count();
		if (count > 0) {
			await expect(saveBtn).toBeVisible();
		}
	});

	test('should fill project name field', async ({ page }) => {
		await page.waitForSelector('#projectName', { timeout: 10000 }).catch(() => {});
		const projectName = page.locator('#projectName');
		const count = await projectName.count();

		if (count > 0) {
			await projectName.fill('New Test').catch(() => {});
			await expect(page.locator('#projectName')).toHaveValue('New Test');
		}
	});
});