import { test, expect } from '@playwright/test';

test.describe('Settings Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/settings');
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

	test('should have processing settings section', async ({ page }) => {
		await expect(page.locator('h2:has-text("Processing")')).toBeVisible();
	});

	test('should have system monitor section', async ({ page }) => {
		await expect(page.locator('h2:has-text("System Monitor")')).toBeVisible();
	});

	test('should have save configuration button', async ({ page }) => {
		await expect(page.locator('.save-btn')).toBeVisible();
		await expect(page.locator('.save-btn')).toContainText('Save Configuration');
	});

	test('should have project name input', async ({ page }) => {
		await expect(page.locator('#projectName')).toBeVisible();
	});

	test('should have batch size input', async ({ page }) => {
		await expect(page.locator('#batchSize')).toBeVisible();
	});

	test('should have CPU workers input', async ({ page }) => {
		await expect(page.locator('#cpuWorkers')).toBeVisible();
	});
});

test.describe('Settings Interactions', () => {
	test('should show loading state initially', async ({ page }) => {
		await page.goto('/settings');
		// Loading should be brief
		const loading = page.locator('.loading');
		// Either loading is visible or content is loaded
		const hasLoading = await loading.isVisible().catch(() => false);
		if (hasLoading) {
			await expect(loading).toContainText('Loading configuration');
		}
	});

	test('should allow changing batch size', async ({ page }) => {
		await page.goto('/settings');
		await page.waitForSelector('#batchSize');
		const batchSizeInput = page.locator('#batchSize');
		await batchSizeInput.fill('32');
		await expect(batchSizeInput).toHaveValue('32');
	});
});
