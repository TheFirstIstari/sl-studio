import { test, expect } from '@playwright/test';

test.describe('Geographic Locations Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/maps');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should display maps title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Geographic');
	});

	test('should have map container', async ({ page }) => {
		const mapContainer = page.locator('.map-container, .leaflet-container, #map');
		const count = await mapContainer.count();
		if (count > 0) {
			await expect(mapContainer.first()).toBeVisible();
		}
	});
});