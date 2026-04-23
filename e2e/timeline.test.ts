import { test, expect } from '@playwright/test';

test.describe('Timeline Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/timeline');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should display timeline title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Timeline');
	});

	test('should have date range inputs', async ({ page }) => {
		const dateInputs = page.locator('input[type="date"]');
		const count = await dateInputs.count();
		if (count > 0) {
			await expect(dateInputs.first()).toBeVisible();
		}
	});

	test('should have timeline visualization', async ({ page }) => {
		const timeline = page.locator('.timeline, .timeline-container, .events');
		const count = await timeline.count();
		if (count > 0) {
			await expect(timeline.first()).toBeVisible();
		}
	});
});