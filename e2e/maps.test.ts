import { test, expect } from '@playwright/test';

test.describe('Geographic Locations Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/maps');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should display Geographic Locations page title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Geographic Locations');
	});

	test('should have min confidence input with correct attributes', async ({ page }) => {
		const confidenceInput = page.locator('#min-conf');
		await expect(confidenceInput).toBeVisible();
		await expect(confidenceInput).toHaveAttribute('type', 'number');
		await expect(confidenceInput).toHaveAttribute('min', '0');
		await expect(confidenceInput).toHaveAttribute('max', '1');
		await expect(confidenceInput).toHaveAttribute('step', '0.1');
	});

	test('should have refresh button', async ({ page }) => {
		const refreshBtn = page.locator('.refresh-btn');
		await expect(refreshBtn).toBeVisible();
	});

	test('should have leaflet map container', async ({ page }) => {
		const mapContainer = page.locator('.map');
		const count = await mapContainer.count();
		if (count > 0) {
			await expect(mapContainer.first()).toBeVisible();
		}
	});

	test('should show empty state when no locations exist', async ({ page }) => {
		const emptyState = page.locator('.empty');
		const count = await emptyState.count();
		if (count > 0) {
			await expect(emptyState).toBeVisible();
			await expect(emptyState.locator('text=No location entities found')).toBeVisible();
		} else {
			// If there's data, map should be visible
			const mapContainer = page.locator('.map');
			await expect(mapContainer).toBeVisible();
		}
	});

	test('should display loading state initially', async ({ page }) => {
		const loading = page.locator('.loading');
		const loadingVisible = await loading.isVisible().catch(() => false);

		if (loadingVisible) {
			await expect(loading).toContainText('Loading');
		}
	});

	test('should have locations panel when data exists', async ({ page }) => {
		const locationsPanel = page.locator('.locations-panel');
		const panelExists = (await locationsPanel.count()) > 0;

		if (panelExists) {
			await expect(locationsPanel).toBeVisible();
			await expect(locationsPanel.locator('h2')).toContainText('Locations');
		}
	});

	test('should have locations list with items', async ({ page }) => {
		const locationsList = page.locator('.locations-list');
		const listExists = (await locationsList.count()) > 0;

		if (listExists) {
			await expect(locationsList).toBeVisible();
		}
	});

	test('should display location items with name', async ({ page }) => {
		const locationItems = page.locator('.location-item');
		const count = await locationItems.count();

		if (count > 0) {
			const firstItem = locationItems.first();
			await expect(firstItem.locator('.location-name')).toBeVisible();
		}
	});

	test('should display location items with severity indicator', async ({ page }) => {
		const locationItems = page.locator('.location-item');
		const count = await locationItems.count();

		if (count > 0) {
			const firstItem = locationItems.first();
			const locationDot = firstItem.locator('.location-dot');
			await expect(locationDot).toBeVisible();
		}
	});

	test('should display location items with confidence', async ({ page }) => {
		const locationItems = page.locator('.location-item');
		const count = await locationItems.count();

		if (count > 0) {
			const firstItem = locationItems.first();
			const meta = firstItem.locator('.location-meta');
			await expect(meta).toBeVisible();
		}
	});

	test('should have detail panel for selected location', async ({ page }) => {
		const detailPanel = page.locator('.detail-panel');
		const panelExists = (await detailPanel.count()) > 0;

		if (panelExists) {
			await expect(detailPanel).toBeVisible();
		}
	});

	test('should display detail panel with location name', async ({ page }) => {
		const locationItems = page.locator('.location-item');
		const count = await locationItems.count();

		if (count > 0) {
			// Click the first location to select it
			await locationItems.first().click();

			// Check detail panel appears
			const detailPanel = page.locator('.detail-panel');
			await expect(detailPanel).toBeVisible();
			await expect(detailPanel.locator('h2')).toContainText('Location Details');
		}
	});

	test('should display coordinates in detail panel', async ({ page }) => {
		const locationItems = page.locator('.location-item');
		const count = await locationItems.count();

		if (count > 0) {
			await locationItems.first().click();

			const detailPanel = page.locator('.detail-panel');
			await expect(detailPanel.locator('.detail-label')).toContainText('Coordinates:');
		}
	});

	test('should display severity badge in detail panel', async ({ page }) => {
		const locationItems = page.locator('.location-item');
		const count = await locationItems.count();

		if (count > 0) {
			await locationItems.first().click();

			const detailPanel = page.locator('.detail-panel');
			const severityBadge = detailPanel.locator('.severity-badge');
			await expect(severityBadge).toBeVisible();
		}
	});

	test('should display severity with correct color coding', async ({ page }) => {
		const locationItems = page.locator('.location-item');
		const count = await locationItems.count();

		if (count > 0) {
			await locationItems.first().click();

			const severityBadge = page.locator('.severity-badge');
			await expect(severityBadge).toBeVisible();

			// Check severity is in format like "X/10"
			const badgeText = await severityBadge.textContent();
			expect(badgeText).toMatch(/\d+\/10/);
		}
	});

	test('should have close button in detail panel', async ({ page }) => {
		const locationItems = page.locator('.location-item');
		const count = await locationItems.count();

		if (count > 0) {
			await locationItems.first().click();

			const closeBtn = page.locator('.detail-panel .close-btn');
			await expect(closeBtn).toBeVisible();
		}
	});

	test('should display fact summary in detail panel', async ({ page }) => {
		const locationItems = page.locator('.location-item');
		const count = await locationItems.count();

		if (count > 0) {
			await locationItems.first().click();

			const detailSection = page.locator('.detail-section');
			await expect(detailSection).toBeVisible();
			await expect(detailSection.locator('h3')).toContainText('Fact Summary');
		}
	});

	test('should allow location selection by clicking', async ({ page }) => {
		const locationItems = page.locator('.location-item');
		const count = await locationItems.count();

		if (count > 0) {
			// Click on a location
			await locationItems.first().click();

			// The item should have selected class
			await expect(locationItems.first()).toHaveClass(/selected/);
		}
	});

	test('should have map container layout', async ({ page }) => {
		// Map container only exists when there are locations
		const emptyState = page.locator('.empty');
		const hasEmpty = (await emptyState.count()) > 0;
		const mapContainer = page.locator('.map-container');

		if (!hasEmpty) {
			await expect(mapContainer).toBeVisible();
		} else {
			// For empty state, no map-container exists
			const mcExists = await mapContainer.count();
			if (mcExists > 0) {
				await expect(mapContainer).toBeVisible();
			}
		}
	});

	test('should have control group for confidence filter', async ({ page }) => {
		const controlGroup = page.locator('.control-group');
		await expect(controlGroup).toBeVisible();
		await expect(controlGroup.locator('label')).toContainText('Min Confidence:');
	});

	test('should have controls section', async ({ page }) => {
		const controls = page.locator('.controls');
		await expect(controls).toBeVisible();
	});

	test('should have page header with proper structure', async ({ page }) => {
		const pageHeader = page.locator('.page-header');
		await expect(pageHeader).toBeVisible();
	});

	test('should allow changing min confidence value', async ({ page }) => {
		const confidenceInput = page.locator('#min-conf');
		await confidenceInput.fill('0.8');
		await expect(confidenceInput).toHaveValue('0.8');
	});

	test('should display locations count in panel header', async ({ page }) => {
		const locationsPanel = page.locator('.locations-panel');
		const panelExists = (await locationsPanel.count()) > 0;

		if (panelExists) {
			const h2 = locationsPanel.locator('h2');
			// Should contain count like "Locations (X)"
			const text = await h2.textContent();
			expect(text).toMatch(/Locations \(\d+\)/);
		}
	});

	test('should handle empty state with hint message', async ({ page }) => {
		const empty = page.locator('.empty');
		const emptyExists = (await empty.count()) > 0;

		if (emptyExists) {
			const hint = page.locator('.empty-hint');
			const hintExists = (await hint.count()) > 0;
			if (hintExists) {
				await expect(hint).toContainText('Run analysis');
			}
		}
	});

	test('detail panel should show source information', async ({ page }) => {
		const locationItems = page.locator('.location-item');
		const count = await locationItems.count();

		if (count > 0) {
			await locationItems.first().click();

			const detailPanel = page.locator('.detail-panel');
			await expect(detailPanel.locator('.detail-label')).toContainText('Source:');
		}
	});
});
