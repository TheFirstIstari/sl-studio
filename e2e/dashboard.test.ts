import { test, expect } from '@playwright/test';

test.describe('Dashboard', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/');
	});

	test('should display dashboard title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Dashboard');
	});

	test('should display status indicator', async ({ page }) => {
		await expect(page.locator('.status')).toBeVisible();
	});

	test('should display statistics cards', async ({ page }) => {
		await expect(page.locator('.stats-grid')).toBeVisible();
	});

	test('should show quick actions', async ({ page }) => {
		const quickActions = page.locator('.quick-actions, .action-buttons');
		await expect(quickActions.first()).toBeVisible();
	});
});

test.describe('Dashboard Interactions', () => {
	test('should open keyboard shortcuts modal', async ({ page }) => {
		await page.goto('/');
		await page.keyboard.press('?');
		await expect(page.locator('.modal')).toBeVisible();
		await expect(page.locator('.modal h2')).toContainText('Keyboard Shortcuts');
	});

	test('should close keyboard shortcuts modal with Escape', async ({ page }) => {
		await page.goto('/');
		await page.keyboard.press('?');
		await expect(page.locator('.modal')).toBeVisible();
		await page.keyboard.press('Escape');
		await expect(page.locator('.modal')).not.toBeVisible();
	});
});
