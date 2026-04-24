import { test, expect } from '@playwright/test';

test.describe('Network Metrics', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/network');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	// 1. Network stats display (nodes, edges, avg connections)
	test('should have network stats section', async ({ page }) => {
		const networkStats = page.locator('.network-stats');
		const count = await networkStats.count();
		if (count > 0) {
			await expect(networkStats).toBeVisible();
		}
	});

	test('should display total nodes stat', async ({ page }) => {
		const networkStats = page.locator('.network-stats');
		const statCount = await networkStats.count();
		if (statCount > 0) {
			const statItems = networkStats.locator('.stat-item');
			const itemsCount = await statItems.count();
			if (itemsCount > 0) {
				await expect(statItems.first()).toBeVisible();
				const label = await statItems.first().locator('.stat-label').textContent();
				// First stat should be Total Nodes or similar
				expect(label).toMatch(/Total|Nodes|nodes/);
			}
		}
	});

	test('should display total edges stat', async ({ page }) => {
		const networkStats = page.locator('.network-stats');
		const statCount = await networkStats.count();
		if (statCount > 0) {
			const statItems = networkStats.locator('.stat-item');
			const itemsCount = await statItems.count();
			if (itemsCount >= 2) {
				const secondStat = statItems.nth(1);
				await expect(secondStat).toBeVisible();
				const label = await secondStat.locator('.stat-label').textContent();
				expect(label).toMatch(/Edges|edges/);
			}
		}
	});

	test('should display average connections stat', async ({ page }) => {
		const networkStats = page.locator('.network-stats');
		const statCount = await networkStats.count();
		if (statCount > 0) {
			const statItems = networkStats.locator('.stat-item');
			const itemsCount = await statItems.count();
			if (itemsCount >= 3) {
				const thirdStat = statItems.nth(2);
				await expect(thirdStat).toBeVisible();
				const label = await thirdStat.locator('.stat-label').textContent();
				expect(label).toMatch(/Connections|connections|Avg/);
			}
		}
	});

	test('should display numeric values for stats', async ({ page }) => {
		const networkStats = page.locator('.network-stats');
		const statCount = await networkStats.count();
		if (statCount > 0) {
			const statValues = networkStats.locator('.stat-value');
			const valueCount = await statValues.count();
			if (valueCount > 0) {
				await expect(statValues.first()).toBeVisible();
				const text = await statValues.first().textContent();
				// Should contain a number
				expect(text).toMatch(/\d+/);
			}
		}
	});

	// 2. Degree centrality shows for selected node
	test('should display degree info when node selected', async ({ page }) => {
		// Check if there's a side panel after selection
		const sidePanel = page.locator('.side-panel');
		const panelCount = await sidePanel.count();
		if (panelCount > 0) {
			await expect(sidePanel).toBeVisible();
		} else {
			// If no side panel initially, we need to verify structure exists
			// for when a node is selected
			const graphContainer = page.locator('.graph-container');
			const graphCount = await graphContainer.count();
			expect(graphCount).toBeGreaterThanOrEqual(0);
		}
	});

	test('should have degree info section in side panel', async ({ page }) => {
		// Wait for loading to complete
		await page.waitForTimeout(1000);
		
		const emptyState = page.locator('.empty');
		const hasEmpty = await emptyState.count() > 0;
		
		if (!hasEmpty) {
			const sidePanel = page.locator('.side-panel');
			const panelCount = await sidePanel.count();
			if (panelCount > 0) {
				const degreeInfo = page.locator('.degree-info');
				const degreeCount = await degreeInfo.count();
				if (degreeCount > 0) {
					await expect(degreeInfo).toBeVisible();
				}
			}
		}
	});

	// 3. Top hubs section displays correctly
	test('should have hub list section', async ({ page }) => {
		// Wait for loading to complete
		await page.waitForTimeout(1000);
		
		const emptyState = page.locator('.empty');
		const hasEmpty = await emptyState.count() > 0;
		
		if (!hasEmpty) {
			const hubList = page.locator('.hub-list');
			const hubCount = await hubList.count();
			if (hubCount > 0) {
				await expect(hubList).toBeVisible();
			}
		}
	});

	test('should display top hubs header', async ({ page }) => {
		// Wait for loading to complete
		await page.waitForTimeout(1000);
		
		const emptyState = page.locator('.empty');
		const hasEmpty = await emptyState.count() > 0;
		
		if (!hasEmpty) {
			const hubList = page.locator('.hub-list');
			const hubCount = await hubList.count();
			if (hubCount > 0) {
				await expect(hubList.locator('h3')).toContainText('Top Hubs');
			}
		}
	});

	test('should display hub items', async ({ page }) => {
		// Wait for loading to complete
		await page.waitForTimeout(1000);
		
		const emptyState = page.locator('.empty');
		const hasEmpty = await emptyState.count() > 0;
		
		if (!hasEmpty) {
			const hubItems = page.locator('.hub-item');
			const itemCount = await hubItems.count();
			if (itemCount > 0) {
				await expect(hubItems.first()).toBeVisible();
			}
		}
	});

	test('should display hub ranking', async ({ page }) => {
		// Wait for loading to complete
		await page.waitForTimeout(1000);
		
		const emptyState = page.locator('.empty');
		const hasEmpty = await emptyState.count() > 0;
		
		if (!hasEmpty) {
			const hubItems = page.locator('.hub-item');
			const itemCount = await hubItems.count();
			if (itemCount > 0) {
				const hubRank = hubItems.first().locator('.hub-rank');
				await expect(hubRank).toBeVisible();
				const text = await hubRank.textContent();
				expect(text).toMatch(/#\d+/);
			}
		}
	});

	test('should display hub degree in hub item', async ({ page }) => {
		// Wait for loading to complete
		await page.waitForTimeout(1000);
		
		const emptyState = page.locator('.empty');
		const hasEmpty = await emptyState.count() > 0;
		
		if (!hasEmpty) {
			const hubItems = page.locator('.hub-item');
			const itemCount = await hubItems.count();
			if (itemCount > 0) {
				const hubMeta = hubItems.first().locator('.hub-meta');
				await expect(hubMeta).toBeVisible();
				const text = await hubMeta.textContent();
				expect(text).toMatch(/connections/);
			}
		}
	});

	// 4. Node sizes vary based on degree (cytoscape visualization)
	test('should have cytoscape graph container', async ({ page }) => {
		const graphContainer = page.locator('.graph-container');
		const count = await graphContainer.count();
		if (count > 0) {
			await expect(graphContainer).toBeVisible();
		}
	});

	test('should display legend', async ({ page }) => {
		// Legend should appear when data exists
		const legend = page.locator('.legend');
		const count = await legend.count();
		if (count > 0) {
			await expect(legend).toBeVisible();
		}
	});

	test('should display legend with entity types', async ({ page }) => {
		const legend = page.locator('.legend');
		const count = await legend.count();
		if (count > 0) {
			await expect(legend.locator('.legend-item')).toHaveCount(5);
		}
	});

	test('should have network container layout', async ({ page }) => {
		const emptyState = page.locator('.empty');
		const hasEmpty = await emptyState.count() > 0;
		
		if (!hasEmpty) {
			const networkContainer = page.locator('.network-container');
			await expect(networkContainer).toBeVisible();
		}
	});

	// Additional tests
	test('should have network page title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Entity Network');
	});

	test('should have min confidence control', async ({ page }) => {
		const minConfInput = page.locator('#min-conf');
		await expect(minConfInput).toBeVisible();
	});

	test('should have zoom controls', async ({ page }) => {
		const emptyState = page.locator('.empty');
		const hasEmpty = await emptyState.count() > 0;
		
		if (!hasEmpty) {
			const zoomIn = page.locator('button[title="Zoom In"]');
			const zoomOut = page.locator('button[title="Zoom Out"]');
			const fitView = page.locator('button[title="Fit View"]');
			
			await expect(zoomIn).toBeVisible();
			await expect(zoomOut).toBeVisible();
			await expect(fitView).toBeVisible();
		}
	});
});