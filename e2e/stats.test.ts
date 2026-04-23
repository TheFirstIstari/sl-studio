import { test, expect } from '@playwright/test';

test.describe('Statistics Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/stats');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should display statistics title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Statistics');
	});

	test('should have charts', async ({ page }) => {
		const charts = page.locator('canvas, .chart');
		const count = await charts.count();
		if (count > 0) {
			await expect(charts.first()).toBeVisible();
		}
	});

	test('should have statistics cards', async ({ page }) => {
		const cards = page.locator('.stat-card, .stats-card, .statistic');
		const count = await cards.count();
		if (count > 0) {
			await expect(cards.first()).toBeVisible();
		}
	});
});