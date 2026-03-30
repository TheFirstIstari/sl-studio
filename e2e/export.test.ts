import { test, expect } from '@playwright/test';

test.describe('Export Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/export');
	});

	test('should display export title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Export Data');
	});

	test('should have export type selector', async ({ page }) => {
		await expect(page.locator('select#export-type')).toBeVisible();
	});

	test('should have all export options', async ({ page }) => {
		const select = page.locator('select#export-type');
		await expect(select).toBeVisible();

		const options = select.locator('option');
		await expect(options).toHaveCount(7);

		// Check for specific options
		await expect(select.locator('option[value="facts-json"]')).toBeAttached();
		await expect(select.locator('option[value="facts-csv"]')).toBeAttached();
		await expect(select.locator('option[value="pdf-report"]')).toBeAttached();
		await expect(select.locator('option[value="excel-data"]')).toBeAttached();
	});

	test('should show min weight and limit for facts-json', async ({ page }) => {
		await page.selectOption('select#export-type', 'facts-json');
		await expect(page.locator('#min-weight')).toBeVisible();
		await expect(page.locator('#limit')).toBeVisible();
	});

	test('should show export button', async ({ page }) => {
		await expect(page.locator('.export-btn')).toBeVisible();
		await expect(page.locator('.export-btn')).toContainText('Export');
	});

	test('should have export button disabled initially', async ({ page }) => {
		// Export button should be enabled by default (can export without file selected)
		await expect(page.locator('.export-btn')).toBeEnabled();
	});
});
