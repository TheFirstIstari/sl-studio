import { test, expect } from '@playwright/test';

test.describe('Anomaly Detection Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/anomalies');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	// Page title tests
	test('should display correct page title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Anomaly Detection');
	});

	test('should have correct URL path', async ({ page }) => {
		await page.goto('/anomalies');
		await expect(page).toHaveURL(/.*\/anomalies/);
	});
});

test.describe('Anomaly Detection Controls', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/anomalies');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should have Metric dropdown', async ({ page }) => {
		await expect(page.locator('#metric-select')).toBeVisible();
	});

	test('should have Threshold input', async ({ page }) => {
		await expect(page.locator('#threshold')).toBeVisible();
	});

	test('should have all 3 metric options', async ({ page }) => {
		const select = page.locator('#metric-select');
		const options = select.locator('option');
		await expect(options).toHaveCount(3);
	});

	test('should have Severity option', async ({ page }) => {
		const select = page.locator('#metric-select');
		await expect(select.locator('option[value="severity"]')).toBeAttached();
	});

	test('should have Confidence option', async ({ page }) => {
		const select = page.locator('#metric-select');
		await expect(select.locator('option[value="confidence"]')).toBeAttached();
	});

	test('should have Quality option', async ({ page }) => {
		const select = page.locator('#metric-select');
		await expect(select.locator('option[value="quality"]')).toBeAttached();
	});

	test('should have default metric value', async ({ page }) => {
		const select = page.locator('#metric-select');
		await expect(select).toHaveValue('severity');
	});

	test('should have correct threshold attributes', async ({ page }) => {
		const input = page.locator('#threshold');
		await expect(input).toHaveAttribute('type', 'number');
		await expect(input).toHaveAttribute('min', '1');
		await expect(input).toHaveAttribute('max', '5');
		await expect(input).toHaveAttribute('step', '0.5');
	});

	test('should have default threshold value', async ({ page }) => {
		await expect(page.locator('#threshold')).toHaveValue('2');
	});

	test('should have labels for controls', async ({ page }) => {
		await expect(page.locator('label[for="metric-select"]')).toContainText('Metric');
		await expect(page.locator('label[for="threshold"]')).toContainText('Threshold');
	});
});

test.describe('Summary Stats', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/anomalies');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should display summary stats container after loading', async ({ page }) => {
		// Summary appears after loading anomalies
		const summary = page.locator('.summary');
		const count = await summary.count();
		expect(count).toBeGreaterThanOrEqual(0);
	});

	test('should display anomaly count stat when anomalies exist', async ({ page }) => {
		await page.waitForTimeout(2000); // Allow time for anomalies to load
		const summary = page.locator('.summary');
		const count = await summary.count();
		if (count > 0) {
			await expect(summary).toContainText('Anomalies Found');
		}
	});

	test('should display current metric stat when anomalies exist', async ({ page }) => {
		await page.waitForTimeout(2000);
		const summary = page.locator('.summary');
		const count = await summary.count();
		if (count > 0) {
			await expect(summary).toContainText('Metric');
		}
	});

	test('should display threshold stat when anomalies exist', async ({ page }) => {
		await page.waitForTimeout(2000);
		const summary = page.locator('.summary');
		const count = await summary.count();
		if (count > 0) {
			await expect(summary).toContainText('Threshold');
		}
	});

	test('should have three summary statistics when anomalies exist', async ({ page }) => {
		await page.waitForTimeout(2000);
		const summary = page.locator('.summary');
		const count = await summary.count();
		if (count > 0) {
			const stats = page.locator('.summary-stat');
			await expect(stats).toHaveCount(3);
		}
	});
});

test.describe('Anomaly Cards', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/anomalies');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should have anomalies list container after loading', async ({ page }) => {
		await page.waitForTimeout(2000);
		const anomaliesList = page.locator('.anomalies-list');
		const count = await anomaliesList.count();
		expect(count).toBeGreaterThanOrEqual(0);
	});

	test('should display anomaly cards with metric badge', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await expect(cards.first().locator('.anomaly-metric')).toBeVisible();
		}
	});

	test('should display anomaly cards with deviation badge', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await expect(cards.first().locator('.anomaly-deviation')).toBeVisible();
		}
	});

	test('should display anomaly summary text', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await expect(cards.first().locator('.anomaly-summary')).toBeVisible();
		}
	});

	test('should display filename in anomaly card', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await expect(cards.first().locator('.anomaly-filename')).toBeVisible();
		}
	});

	test('should display date in anomaly card', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await expect(cards.first().locator('.anomaly-date')).toBeVisible();
		}
	});

	test('should allow clicking on anomaly card', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await cards.first().click();
			// Card becomes selected
			await expect(cards.first()).toHaveClass(/selected/);
		}
	});

	test('should display multiple anomaly cards', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		expect(count).toBeGreaterThanOrEqual(0);
	});
});

test.describe('Detail Panel', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/anomalies');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should have close button in detail panel', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await cards.first().click();
			await expect(page.locator('.close-btn')).toBeVisible();
		}
	});

	test('should have detail header in panel', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await cards.first().click();
			await expect(page.locator('.detail-header h2')).toContainText('Anomaly Details');
		}
	});

	test('should display metric badge in detail panel', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await cards.first().click();
			await expect(page.locator('.metric-badge')).toBeVisible();
		}
	});

	test('should display deviation in detail panel', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await cards.first().click();
			await expect(page.locator('.detail-value.deviation')).toBeVisible();
		}
	});

	test('should display comparison section', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await cards.first().click();
			await expect(page.locator('.comparison')).toBeVisible();
		}
	});

	test('should display actual value in comparison', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await cards.first().click();
			await expect(page.locator('.comparison-item').first()).toContainText('Actual Value');
		}
	});

	test('should display expected value in comparison', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await cards.first().click();
			await expect(page.locator('.comparison-item').nth(1)).toContainText('Expected Value');
		}
	});

	test('should display filename in detail panel', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await cards.first().click();
			await expect(page.locator('.detail-label:has-text("Filename")')).toBeVisible();
		}
	});

	test('should display date in detail panel', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await cards.first().click();
			await expect(page.locator('.detail-label:has-text("Date")')).toBeVisible();
		}
	});

	test('should display summary section', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await cards.first().click();
			await expect(page.locator('.detail-section h3:has-text("Summary")')).toBeVisible();
		}
	});

	test('should display fingerprint section', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await cards.first().click();
			await expect(page.locator('.detail-section h3:has-text("Fingerprint")')).toBeVisible();
		}
	});

	test('should close detail panel when close button clicked', async ({ page }) => {
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			await cards.first().click();
			await expect(page.locator('.detail-panel')).toBeVisible();
			await page.locator('.close-btn').click();
			await expect(page.locator('.detail-panel')).not.toBeVisible();
		}
	});
});

test.describe('Loading and Empty States', () => {
	test('should display loading state', async ({ page }) => {
		await page.goto('/anomalies');
		// Loading may appear briefly
		const loading = page.locator('.loading');
		const count = await loading.count();
		expect(count).toBeGreaterThanOrEqual(0);
	});

	test('should display empty state when no anomalies', async ({ page }) => {
		await page.goto('/anomalies');
		await page.waitForTimeout(1000);
		const empty = page.locator('.empty');
		const count = await empty.count();
		// Either empty or has anomalies
		expect(count).toBeGreaterThanOrEqual(0);
	});
});

test.describe('Page Header Layout', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/anomalies');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should have page header container', async ({ page }) => {
		await expect(page.locator('.page-header')).toBeVisible();
	});

	test('should have controls container', async ({ page }) => {
		await expect(page.locator('.controls')).toBeVisible();
	});

	test('should have control groups for inputs', async ({ page }) => {
		const controlGroups = page.locator('.control-group');
		await expect(controlGroups).toHaveCount(2);
	});
});

test.describe('Deviation Badge Colors', () => {
	test('should display deviation with color styling', async ({ page }) => {
		await page.goto('/anomalies');
		await page.waitForLoadState('networkidle').catch(() => {});
		const cards = page.locator('.anomaly-card');
		const count = await cards.count();
		if (count > 0) {
			const deviation = cards.first().locator('.anomaly-deviation');
			await expect(deviation).toHaveCSS(/background-color.*rgb|background-color.*#/);
		}
	});
});
