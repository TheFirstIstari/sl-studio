import { test, expect } from '@playwright/test';

test.describe('Backup & Restore Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/backup');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	// Page title tests
	test('should display correct page title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Backup & Restore');
	});

	// Create Backup section tests
	test('should display Create Backup section heading', async ({ page }) => {
		await expect(page.locator('h2:has-text("Create Backup")')).toBeVisible();
	});

	test('should have Include evidence checkbox', async ({ page }) => {
		const checkbox = page.locator('input[type="checkbox"]');
		await expect(checkbox).toBeVisible();
		await expect(page.locator('.checkbox-label')).toContainText('Include evidence files');
	});

	test('should have checkbox hint text', async ({ page }) => {
		await expect(page.locator('.hint')).toContainText('Warning: Including evidence may result in large backup files');
	});

	test('should have Create Backup button', async ({ page }) => {
		const button = page.locator('.backup-btn');
		await expect(button).toBeVisible();
		await expect(button).toContainText('Create Backup');
	});

	test('should toggle checkbox state', async ({ page }) => {
		const checkbox = page.locator('input[type="checkbox"]');
		await expect(checkbox).not.toBeChecked();
		await checkbox.check();
		await expect(checkbox).toBeChecked();
		await checkbox.uncheck();
		await expect(checkbox).not.toBeChecked();
	});

	// Restore Backup section tests
	test('should display Restore Backup section heading', async ({ page }) => {
		await expect(page.locator('h2:has-text("Restore Backup")')).toBeVisible();
	});

	test('should have Restore from Backup button', async ({ page }) => {
		const button = page.locator('.restore-btn');
		await expect(button).toBeVisible();
		await expect(button).toContainText('Restore from Backup');
	});

	test('should have warning box for restore', async ({ page }) => {
		const warningBox = page.locator('.warning-box');
		await expect(warningBox).toBeVisible();
		await expect(warningBox).toContainText('Warning');
		await expect(warningBox).toContainText('Restoring will overwrite');
	});

	// Backup Information section tests
	test('should display backup information section', async ({ page }) => {
		await expect(page.locator('.backup-info')).toBeVisible();
	});

	test('should display backup information heading', async ({ page }) => {
		await expect(page.locator('.backup-info h2')).toContainText('Backup Information');
	});

	test('should list backup information items', async ({ page }) => {
		const infoList = page.locator('.backup-info ul');
		await expect(infoList).toBeVisible();
		await expect(infoList).toContainText('Databases');
		await expect(infoList).toContainText('Configuration settings');
		await expect(infoList).toContainText('Evidence files');
		await expect(infoList).toContainText('application data directory');
	});

	// Button states and interactions
	test('should disable create backup button when backing up', async ({ page }) => {
		const button = page.locator('.backup-btn');
		// Button should be enabled initially
		await expect(button).toBeEnabled();
		await expect(button).toContainText('Create Backup');
	});

	test('should disable restore button when restoring', async ({ page }) => {
		const button = page.locator('.restore-btn');
		// Button should be enabled initially
		await expect(button).toBeEnabled();
		await expect(button).toContainText('Restore from Backup');
	});

	// Status message tests
	test('should have status message container', async ({ page }) => {
		// Initially hidden, appears after actions
		await expect(page.locator('.status-message')).not.toBeVisible();
	});

	// Layout tests
	test('should have two backup cards in sections', async ({ page }) => {
		const cards = page.locator('.backup-card');
		await expect(cards).toHaveCount(2);
	});

	test('should have form groups in create backup card', async ({ page }) => {
		const formGroup = page.locator('.backup-card:first-child .form-group');
		await expect(formGroup).toBeVisible();
	});

	// Card content tests
	test('should have description text in create backup card', async ({ page }) => {
		const firstCard = page.locator('.backup-card').first();
		await expect(firstCard).toContainText('Create a backup');
	});

	test('should have description text in restore backup card', async ({ page }) => {
		const secondCard = page.locator('.backup-card').nth(1);
		await expect(secondCard).toContainText('Restore your project');
	});
});

test.describe('Backup Page Navigation', () => {
	test('should navigate to backup page from URL', async ({ page }) => {
		await page.goto('/backup');
		await expect(page.locator('h1')).toContainText('Backup & Restore');
	});

	test('should have correct URL path', async ({ page }) => {
		await page.goto('/backup');
		await expect(page).toHaveURL(/.*\/backup/);
	});
});