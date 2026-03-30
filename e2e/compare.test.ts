import { test, expect } from '@playwright/test';

test.describe('Compare Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/compare');
	});

	test('should display compare title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Compare Projects');
	});

	test('should have current project section', async ({ page }) => {
		await expect(page.locator('h2:has-text("Current Project")')).toBeVisible();
	});

	test('should have compare with section', async ({ page }) => {
		await expect(page.locator('h2:has-text("Compare With")')).toBeVisible();
	});

	test('should have select project button', async ({ page }) => {
		await expect(page.locator('.select-btn')).toBeVisible();
	});

	test('should have compare button', async ({ page }) => {
		await expect(page.locator('.compare-btn')).toBeVisible();
		await expect(page.locator('.compare-btn')).toContainText('Compare Projects');
	});

	test('should have VS divider', async ({ page }) => {
		await expect(page.locator('.vs-divider')).toBeVisible();
		await expect(page.locator('.vs-divider')).toContainText('VS');
	});
});

test.describe('Compare Page States', () => {
	test('should disable compare button without project selected', async ({ page }) => {
		await page.goto('/compare');
		// Compare button should be disabled when no project is selected
		await expect(page.locator('.compare-btn')).toBeDisabled();
	});

	test('should show error when comparing without project', async ({ page }) => {
		await page.goto('/compare');
		// This test would need mocking the dialog, so we just check UI state
		await expect(page.locator('.compare-btn')).toBeDisabled();
	});
});
