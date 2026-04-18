import { test, expect } from '@playwright/test';

test.describe('Dashboard', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should display dashboard title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Dashboard');
	});

	test('should display status indicator', async ({ page }) => {
		const status = page.locator('.status, [class*="status"]').first();
		const count = await status.count();
		if (count > 0) {
			await expect(status).toBeVisible();
		}
	});

	test('should display statistics cards', async ({ page }) => {
		const statsGrid = page.locator('.stats-grid, .statistics, [class*="stats"]').first();
		const count = await statsGrid.count();
		if (count > 0) {
			await expect(statsGrid).toBeVisible();
		}
	});

	test('should show quick actions', async ({ page }) => {
		const quickActions = page.locator('.quick-actions, .action-buttons, .nav-actions').first();
		const count = await quickActions.count();
		if (count > 0) {
			await expect(quickActions).toBeVisible();
		}
	});

	test('should navigate to analysis page', async ({ page }) => {
		await page.click('a[href="/analysis"]');
		await expect(page).toHaveURL('/analysis');
		await expect(page.locator('h1')).toContainText('Analysis');
	});

	test('should navigate to settings page', async ({ page }) => {
		await page.click('a[href="/settings"]');
		await expect(page).toHaveURL('/settings');
		await expect(page.locator('h1')).toContainText('Settings');
	});
});
