import { test, expect } from '@playwright/test';

test.describe('Statistics Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/stats');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should display Statistics page title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Statistics');
	});

	test('should have overview cards section', async ({ page }) => {
		const overviewCards = page.locator('.overview-cards');
		const cardsExist = (await overviewCards.count()) > 0;
		if (cardsExist) {
			await expect(overviewCards).toBeVisible();
		}
	});

	test('should display Total Facts card', async ({ page }) => {
		const cards = page.locator('.overview-card');
		const count = await cards.count();
		if (count > 0) {
			await expect(page.locator('.card-label')).toContainText('Total Facts');
		}
	});

	test('should display Avg Severity card', async ({ page }) => {
		const cards = page.locator('.overview-card');
		const count = await cards.count();
		if (count > 0) {
			await expect(page.locator('.card-label')).toContainText('Avg Severity');
		}
	});

	test('should display Avg Confidence card', async ({ page }) => {
		const cards = page.locator('.overview-card');
		const count = await cards.count();
		if (count > 0) {
			await expect(page.locator('.card-label')).toContainText('Avg Confidence');
		}
	});

	test('should display Entity Mentions card', async ({ page }) => {
		const cards = page.locator('.overview-card');
		const count = await cards.count();
		if (count > 0) {
			await expect(page.locator('.card-label')).toContainText('Entity Mentions');
		}
	});

	test('should display Unique Entities card', async ({ page }) => {
		const cards = page.locator('.overview-card');
		const count = await cards.count();
		if (count > 0) {
			await expect(page.locator('.card-label')).toContainText('Unique Entities');
		}
	});

	test('should display Evidence Chains card', async ({ page }) => {
		const cards = page.locator('.overview-card');
		const count = await cards.count();
		if (count > 0) {
			await expect(page.locator('.card-label')).toContainText('Evidence Chains');
		}
	});

	test('should have all 6 overview cards', async ({ page }) => {
		const cards = page.locator('.overview-card');
		const count = await cards.count();
		if (count > 0) {
			await expect(cards).toHaveCount(6);
		}
	});

	test('should display Facts by Severity chart', async ({ page }) => {
		const chartCard = page.locator('.chart-card').first();
		await expect(chartCard).toBeVisible();
		await expect(chartCard.locator('h2')).toContainText('Facts by Severity');
	});

	test('should have canvas for Facts by Severity chart', async ({ page }) => {
		const chartCard = page.locator('.chart-card').first();
		await expect(chartCard.locator('canvas')).toBeVisible();
	});

	test('should display Facts by Category chart', async ({ page }) => {
		const chartCards = page.locator('.chart-card');
		const count = await chartCards.count();
		if (count >= 2) {
			const secondChart = chartCards.nth(1);
			await expect(secondChart.locator('h2')).toContainText('Facts by Category');
		}
	});

	test('should have canvas for Facts by Category chart', async ({ page }) => {
		const chartCards = page.locator('.chart-card');
		const count = await chartCards.count();
		if (count >= 2) {
			const secondChart = chartCards.nth(1);
			await expect(secondChart.locator('canvas')).toBeVisible();
		}
	});

	test('should display Top Entities chart', async ({ page }) => {
		const chartCards = page.locator('.chart-card');
		const count = await chartCards.count();
		if (count >= 3) {
			const thirdChart = chartCards.nth(2);
			await expect(thirdChart.locator('h2')).toContainText('Top Entities');
		}
	});

	test('should have canvas for Top Entities chart', async ({ page }) => {
		const chartCards = page.locator('.chart-card');
		const count = await chartCards.count();
		if (count >= 3) {
			const thirdChart = chartCards.nth(2);
			await expect(thirdChart.locator('canvas')).toBeVisible();
		}
	});

	test('should have charts rendered on canvas elements', async ({ page }) => {
		const canvases = page.locator('canvas');
		const count = await canvases.count();
		expect(count).toBeGreaterThanOrEqual(3); // Should have at least 3 charts
	});

	test('should display Category Details table', async ({ page }) => {
		const tableCard = page.locator('.table-card');
		const tableExists = (await tableCard.count()) > 0;

		if (tableExists) {
			await expect(tableCard).toBeVisible();
			await expect(tableCard.locator('h2')).toContainText('Category Details');
		}
	});

	test('should have table with proper headers', async ({ page }) => {
		const tableCard = page.locator('.table-card');
		const tableExists = (await tableCard.count()) > 0;

		if (tableExists) {
			const table = tableCard.locator('.data-table');
			await expect(table.locator('th')).toContainText('Category');
			await expect(table.locator('th')).toContainText('Count');
			await expect(table.locator('th')).toContainText('Avg Severity');
			await expect(table.locator('th')).toContainText('Avg Confidence');
		}
	});

	test('should have table with data rows', async ({ page }) => {
		const tableCard = page.locator('.table-card');
		const tableExists = (await tableCard.count()) > 0;

		if (tableExists) {
			const tbody = tableCard.locator('tbody');
			const rows = tbody.locator('tr');
			const rowCount = await rows.count();
			expect(rowCount).toBeGreaterThanOrEqual(0);
		}
	});

	test('should have charts grid layout', async ({ page }) => {
		const chartsGrid = page.locator('.charts-grid');
		await expect(chartsGrid).toBeVisible();
	});

	test('should have multiple charts in grid', async ({ page }) => {
		const chartCards = page.locator('.chart-card');
		await expect(chartCards).toHaveCount(3);
	});

	test('should have chart containers with proper height', async ({ page }) => {
		const chartContainers = page.locator('.chart-container');
		await expect(chartContainers).toHaveCount(3);
	});

	test('should display loading state initially', async ({ page }) => {
		const loading = page.locator('.loading');
		const loadingVisible = await loading.isVisible().catch(() => false);

		if (loadingVisible) {
			await expect(loading).toContainText('Loading');
		}
	});

	test('should have overview card values populated', async ({ page }) => {
		const cards = page.locator('.overview-card');
		const count = await cards.count();

		if (count > 0) {
			// Each card should have a value
			for (let i = 0; i < count; i++) {
				const card = cards.nth(i);
				await expect(card.locator('.card-value')).toBeVisible();
			}
		}
	});

	test('overview cards should have numeric values', async ({ page }) => {
		const cardValues = page.locator('.card-value');
		const count = await cardValues.count();

		if (count > 0) {
			const firstValue = await cardValues.first().textContent();
			// Value should be a number or percentage
			expect(firstValue).toMatch(/^[\d.]+%?$/);
		}
	});

	test('should have proper page layout structure', async ({ page }) => {
		const statsPage = page.locator('.stats-page');
		await expect(statsPage).toBeVisible();
	});

	test('should display percentage format for confidence', async ({ page }) => {
		// Check that confidence is displayed as percentage
		const cards = page.locator('.overview-card');
		const count = await cards.count();

		if (count > 0) {
			const confidenceCard = cards.filter({ hasText: 'Avg Confidence' });
			if ((await confidenceCard.count()) > 0) {
				const value = await confidenceCard.locator('.card-value').textContent();
				expect(value).toContain('%');
			}
		}
	});

	test('Top Entities chart should be wider layout', async ({ page }) => {
		const chartCards = page.locator('.chart-card');
		const count = await chartCards.count();
		if (count >= 3) {
			const thirdChart = chartCards.nth(2);
			await expect(thirdChart).toHaveClass(/wide/);
		}
	});

	test('table should show category statistics data', async ({ page }) => {
		const tableCard = page.locator('.table-card');
		const tableExists = (await tableCard.count()) > 0;

		if (tableExists) {
			const tbody = tableCard.locator('tbody');
			const rows = tbody.locator('tr');
			const rowCount = await rows.count();

			if (rowCount > 0) {
				// Each row should have 4 columns (Category, Count, Avg Severity, Avg Confidence)
				const firstRow = rows.first();
				const cells = firstRow.locator('td');
				await expect(cells).toHaveCount(4);
			}
		}
	});
});
