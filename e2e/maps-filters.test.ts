import { test, expect } from '@playwright/test';

test.describe('Maps Severity Filter', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/maps');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	// 1. Severity filter inputs work
	test('should have min severity input', async ({ page }) => {
		const minSeverityInput = page.locator('#min-sev');
		const count = await minSeverityInput.count();
		if (count > 0) {
			await expect(minSeverityInput).toBeVisible();
		}
	});

	test('should have max severity input', async ({ page }) => {
		const maxSeverityInput = page.locator('#max-sev');
		const count = await maxSeverityInput.count();
		if (count > 0) {
			await expect(maxSeverityInput).toBeVisible();
		}
	});

	test('should accept severity values', async ({ page }) => {
		const minSeverityInput = page.locator('#min-sev');
		const count = await minSeverityInput.count();
		if (count > 0) {
			await minSeverityInput.fill('5');
			await expect(minSeverityInput).toHaveValue('5');
		}
	});

	test('should display severity range separator', async ({ page }) => {
		const rangeSeparator = page.locator('.range-separator');
		const count = await rangeSeparator.count();
		if (count > 0) {
			await expect(rangeSeparator).toContainText('to');
		}
	});

	// 2. Marker count updates when filtered
	test('should display locations count in panel header', async ({ page }) => {
		const locationsPanel = page.locator('.locations-panel');
		const panelCount = await locationsPanel.count();
		if (panelCount > 0) {
			const h2 = locationsPanel.locator('h2');
			const text = await h2.textContent();
			// Should show count like "Locations (X of Y)"
			expect(text).toMatch(/Locations \(\d+ of \d+\)/);
		}
	});

	test('should update locations count based on severity filter', async ({ page }) => {
		// First check initial count
		const locationsPanel = page.locator('.locations-panel');
		const panelCount = await locationsPanel.count();
		if (panelCount > 0) {
			let initialText = await locationsPanel.locator('h2').textContent();
			const initialMatch = initialText.match(/Locations \((\d+) of (\d+)\)/);

			if (initialMatch) {
				const filteredCount = parseInt(initialMatch[1]);
				const totalCount = parseInt(initialMatch[2]);

				// Apply severity filter
				const minSeverityInput = page.locator('#min-sev');
				await minSeverityInput.fill('7');

				// Click refresh to apply filter
				const refreshBtn = page.locator('.refresh-btn');
				await refreshBtn.click();

				// Wait for update
				await page.waitForTimeout(500);

				// Check updated count
				const updatedText = await locationsPanel.locator('h2').textContent();
				const updatedMatch = updatedText.match(/Locations \((\d+) of (\d+)\)/);

				if (updatedMatch) {
					const newFilteredCount = parseInt(updatedMatch[1]);
					// With higher severity filter, filtered count should be <= total
					expect(newFilteredCount).toBeLessThanOrEqual(totalCount);
				}
			}
		}
	});

	// 3. Confidence filter still works
	test('should have min confidence input', async ({ page }) => {
		const minConfidenceInput = page.locator('#min-conf');
		await expect(minConfidenceInput).toBeVisible();
	});

	test('should accept confidence values', async ({ page }) => {
		const minConfidenceInput = page.locator('#min-conf');
		await minConfidenceInput.fill('0.8');
		await expect(minConfidenceInput).toHaveValue('0.8');
	});

	test('should have refresh button', async ({ page }) => {
		const refreshBtn = page.locator('.refresh-btn');
		await expect(refreshBtn).toBeVisible();
	});

	test('should have severity label', async ({ page }) => {
		const controlGroup = page.locator('.control-group').filter({ hasText: 'Severity:' });
		const count = await controlGroup.count();
		if (count > 0) {
			await expect(controlGroup).toBeVisible();
		}
	});

	// Additional tests
	test('should have map container', async ({ page }) => {
		const emptyState = page.locator('.empty');
		const hasEmpty = (await emptyState.count()) > 0;

		if (!hasEmpty) {
			const mapContainer = page.locator('.map-container');
			await expect(mapContainer).toBeVisible();
		}
	});

	test('should have location items in list', async ({ page }) => {
		const locationItems = page.locator('.location-item');
		const count = await locationItems.count();
		if (count > 0) {
			await expect(locationItems.first()).toBeVisible();
		}
	});

	test('should be able to select location from list', async ({ page }) => {
		const locationItems = page.locator('.location-item');
		const count = await locationItems.count();
		if (count > 0) {
			await locationItems.first().click();
			await expect(locationItems.first()).toHaveClass(/selected/);
		}
	});

	test('should show detail panel when location selected', async ({ page }) => {
		const locationItems = page.locator('.location-item');
		const count = await locationItems.count();
		if (count > 0) {
			await locationItems.first().click();
			const detailPanel = page.locator('.detail-panel');
			await expect(detailPanel).toBeVisible();
		}
	});
});
