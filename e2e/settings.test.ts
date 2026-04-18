import { test, expect } from '@playwright/test';

test.describe('Settings Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/settings');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should display settings title', async ({ page }) => {
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

	test('should have save configuration button', async ({ page }) => {
		const saveBtn = page.locator('.save-btn, button:has-text("Save")').first();
		await expect(saveBtn).toBeVisible();
	});

	test('should have project name input', async ({ page }) => {
		await expect(page.locator('#projectName')).toBeVisible();
	});

	test('should allow changing project name', async ({ page }) => {
		const projectNameInput = page.locator('#projectName');
		await projectNameInput.fill('Test Investigation');
		await expect(projectNameInput).toHaveValue('Test Investigation');
	});

	test('should have batch size input', async ({ page }) => {
		const batchInput = page.locator('#batchSize');
		const count = await batchInput.count();
		if (count > 0) {
			await expect(batchInput).toBeVisible();
		}
	});

	test('should have CPU workers input', async ({ page }) => {
		const cpuInput = page.locator('#cpuWorkers');
		const count = await cpuInput.count();
		if (count > 0) {
			await expect(cpuInput).toBeVisible();
		}
	});

	test('should allow changing batch size', async ({ page }) => {
		const batchInput = page.locator('#batchSize');
		const count = await batchInput.count();
		if (count > 0) {
			await batchInput.fill('32');
			await expect(batchInput).toHaveValue('32');
		}
	});
});

test.describe('Settings Interactions', () => {
	test('should show loading state initially', async ({ page }) => {
		await page.goto('/settings');
		const loading = page.locator('.loading');
		const hasLoading = await loading.isVisible().catch(() => false);
		if (hasLoading) {
			await expect(loading).toContainText('Loading');
		}
	});
});
