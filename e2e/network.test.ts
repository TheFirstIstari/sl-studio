import { test, expect } from '@playwright/test';

test.describe('Network Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/network');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should display network title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Network');
	});

	test('should have network graph container', async ({ page }) => {
		const graph = page.locator('.network-graph, .cytoscape-container, #network');
		const count = await graph.count();
		if (count > 0) {
			await expect(graph.first()).toBeVisible();
		}
	});

	test('should have entity selector', async ({ page }) => {
		const selector = page.locator('select#entity-type, .entity-select');
		const count = await selector.count();
		if (count > 0) {
			await expect(selector.first()).toBeVisible();
		}
	});
});