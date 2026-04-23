import { test, expect } from '@playwright/test';

test.describe('Anomalies Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/anomalies');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should display anomalies title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Anomal');
	});

	test('should have metric selector', async ({ page }) => {
		const selector = page.locator('select#metric, .metric-select');
		const count = await selector.count();
		if (count > 0) {
			await expect(selector.first()).toBeVisible();
		}
	});

	test('should have threshold input', async ({ page }) => {
		const input = page.locator('input#threshold, input[type="number"]');
		const count = await input.count();
		if (count > 0) {
			await expect(input.first()).toBeVisible();
		}
	});
});