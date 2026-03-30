import { test, expect } from '@playwright/test';

test.describe('Backup Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/backup');
	});

	test('should display backup title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Backup & Restore');
	});

	test('should have create backup section', async ({ page }) => {
		await expect(page.locator('h2:has-text("Create Backup")')).toBeVisible();
	});

	test('should have restore backup section', async ({ page }) => {
		await expect(page.locator('h2:has-text("Restore Backup")')).toBeVisible();
	});

	test('should have include evidence checkbox', async ({ page }) => {
		await expect(page.locator('input[type="checkbox"]')).toBeVisible();
		await expect(page.locator('.checkbox-label')).toContainText('Include evidence files');
	});

	test('should have create backup button', async ({ page }) => {
		await expect(page.locator('.backup-btn')).toBeVisible();
		await expect(page.locator('.backup-btn')).toContainText('Create Backup');
	});

	test('should have restore button', async ({ page }) => {
		await expect(page.locator('.restore-btn')).toBeVisible();
		await expect(page.locator('.restore-btn')).toContainText('Restore from Backup');
	});

	test('should have warning box for restore', async ({ page }) => {
		await expect(page.locator('.warning-box')).toBeVisible();
		await expect(page.locator('.warning-box')).toContainText('Warning');
	});

	test('should have backup info section', async ({ page }) => {
		await expect(page.locator('.backup-info')).toBeVisible();
		await expect(page.locator('.backup-info h2')).toContainText('Backup Information');
	});
});
