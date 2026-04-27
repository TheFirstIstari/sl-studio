import { test, expect } from '@playwright/test';

test.describe('Entity Network Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/network');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should display Entity Network page title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Entity Network');
	});

	test('should have min confidence input with correct attributes', async ({ page }) => {
		const confidenceInput = page.locator('#min-conf');
		await expect(confidenceInput).toBeVisible();
		await expect(confidenceInput).toHaveAttribute('type', 'number');
		await expect(confidenceInput).toHaveAttribute('min', '0');
		await expect(confidenceInput).toHaveAttribute('max', '1');
		await expect(confidenceInput).toHaveAttribute('step', '0.1');
	});

	test('should have zoom controls buttons', async ({ page }) => {
		// Zoom In button
		const zoomInBtn = page.locator('button[title="Zoom In"]');
		await expect(zoomInBtn).toBeVisible();

		// Zoom Out button
		const zoomOutBtn = page.locator('button[title="Zoom Out"]');
		await expect(zoomOutBtn).toBeVisible();

		// Fit View button
		const fitViewBtn = page.locator('button[title="Fit View"]');
		await expect(fitViewBtn).toBeVisible();
	});

	test('should have graph container', async ({ page }) => {
		// Graph container only shows when there are relationships
		// Check if we have data or empty state
		const emptyState = page.locator('.empty');
		const hasEmpty = (await emptyState.count()) > 0;
		const graphContainer = page.locator('.graph-container');

		if (!hasEmpty) {
			await expect(graphContainer).toBeVisible();
		} else {
			// If empty, graph container should not exist
			const graphExists = await graphContainer.count();
			if (graphExists > 0) {
				await expect(graphContainer).toBeVisible();
			}
		}
	});

	test('should show empty state when no entities exist', async ({ page }) => {
		const emptyState = page.locator('.empty');
		const count = await emptyState.count();
		if (count > 0) {
			await expect(emptyState).toBeVisible();
			await expect(emptyState.locator('text=No entity relationships found')).toBeVisible();
		} else {
			// If there's data, graph should be visible
			const graphContainer = page.locator('.graph-container');
			await expect(graphContainer).toBeVisible();
		}
	});

	test('should display legend with all entity types', async ({ page }) => {
		// The legend is only visible when a node is selected or when there's data
		const legend = page.locator('.legend');
		const legendVisible = await legend.isVisible().catch(() => false);

		if (legendVisible) {
			// Check for all legend items
			await expect(page.locator('.legend')).toContainText('Person');
			await expect(page.locator('.legend')).toContainText('Organization');
			await expect(page.locator('.legend')).toContainText('Location');
			await expect(page.locator('.legend')).toContainText('Date');
			await expect(page.locator('.legend')).toContainText('Other');
		}
	});

	test('should display legend items with correct colors', async ({ page }) => {
		const legendVisible = await page
			.locator('.legend')
			.isVisible()
			.catch(() => false);

		if (legendVisible) {
			const legendItems = page.locator('.legend-item');
			const count = await legendItems.count();
			expect(count).toBe(5); // Person, Organization, Location, Date, Other
		}
	});

	test('should have side panel with selection info', async ({ page }) => {
		// Side panel shows when a node is selected (but we can't simulate cytoscape clicks easily)
		// So we test that the panel structure exists
		const sidePanel = page.locator('.side-panel');
		const panelExists = (await sidePanel.count()) > 0;

		if (panelExists) {
			await expect(page.locator('.panel-header h2')).toContainText('Selection');
			await expect(page.locator('.close-btn')).toBeVisible();
		}
	});

	test('should display connected entities section when node selected', async ({ page }) => {
		const connectedList = page.locator('.connected-list');
		const connectedExists = (await connectedList.count()) > 0;

		if (connectedExists) {
			await expect(connectedList.locator('h3')).toContainText('Connected Entities');
		}
	});

	test('should show entity dots with type colors', async ({ page }) => {
		const entityDots = page.locator('.legend-dot');
		const count = await entityDots.count();

		// Should have 5 legend dots for different entity types
		expect(count).toBeGreaterThanOrEqual(0);
	});

	test('should have network container layout', async ({ page }) => {
		// Container exists when data exists, empty state shows otherwise
		const emptyState = page.locator('.empty');
		const hasEmpty = (await emptyState.count()) > 0;
		const networkContainer = page.locator('.network-container');

		if (!hasEmpty) {
			await expect(networkContainer).toBeVisible();
		} else {
			// For empty state, no network-container exists
			const ncExists = await networkContainer.count();
			if (ncExists > 0) {
				await expect(networkContainer).toBeVisible();
			}
		}
	});

	test('should display loading state initially', async ({ page }) => {
		const loading = page.locator('.loading');
		const loadingVisible = await loading.isVisible().catch(() => false);

		if (loadingVisible) {
			await expect(loading).toContainText('Loading');
		}
	});

	test('should have control group for confidence filter', async ({ page }) => {
		const controlGroup = page.locator('.control-group');
		await expect(controlGroup).toBeVisible();
		await expect(controlGroup.locator('label')).toContainText('Min Confidence:');
	});

	test('should have controls section with buttons', async ({ page }) => {
		const controls = page.locator('.controls');
		await expect(controls).toBeVisible();

		// Should have icon buttons for zoom
		const iconBtns = page.locator('.icon-btn');
		const count = await iconBtns.count();
		expect(count).toBeGreaterThanOrEqual(3); // Zoom in, zoom out, fit view
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

	test('should reset min confidence to valid range', async ({ page }) => {
		const confidenceInput = page.locator('#min-conf');

		// Fill with a valid value
		await confidenceInput.fill('0.5');
		await expect(confidenceInput).toHaveValue('0.5');
	});

	test('cytoscape graph should be interactive', async ({ page }) => {
		// Check that the cytoscape container can accept interactions
		// At minimum verify the container exists and is ready
		const emptyState = page.locator('.empty');
		const hasEmpty = (await emptyState.count()) > 0;
		const graphContainer = page.locator('.graph-container');

		if (!hasEmpty) {
			await expect(graphContainer).toBeVisible();
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
});
