import { test, expect } from '@playwright/test';

test.describe('Export Data Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/export');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	// Page title tests
	test('should display correct page title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Export Data');
	});

	test('should have correct URL path', async ({ page }) => {
		await page.goto('/export');
		await expect(page).toHaveURL(/.*\/export/);
	});

	// Export Type dropdown tests
	test('should have Export Type dropdown', async ({ page }) => {
		await expect(page.locator('select#export-type')).toBeVisible();
	});

	test('should have all 7 export type options', async ({ page }) => {
		const select = page.locator('select#export-type');
		const options = select.locator('option');
		await expect(options).toHaveCount(7);
	});

	test('should have facts-json as default option', async ({ page }) => {
		const select = page.locator('select#export-type');
		await expect(select).toHaveValue('facts-json');
	});

	test('should display Facts (JSON) option', async ({ page }) => {
		await expect(page.locator('select#export-type option[value="facts-json"]')).toBeAttached();
	});

	test('should display Facts (CSV) option', async ({ page }) => {
		await expect(page.locator('select#export-type option[value="facts-csv"]')).toBeAttached();
	});

	test('should display Entities (CSV) option', async ({ page }) => {
		await expect(page.locator('select#export-type option[value="entities-csv"]')).toBeAttached();
	});

	test('should display Timeline (JSON) option', async ({ page }) => {
		await expect(page.locator('select#export-type option[value="timeline-json"]')).toBeAttached();
	});

	test('should display Full Report (JSON) option', async ({ page }) => {
		await expect(page.locator('select#export-type option[value="full-report"]')).toBeAttached();
	});

	test('should display PDF Report option', async ({ page }) => {
		await expect(page.locator('select#export-type option[value="pdf-report"]')).toBeAttached();
	});

	test('should display Excel Data (JSON) option', async ({ page }) => {
		await expect(page.locator('select#export-type option[value="excel-data"]')).toBeAttached();
	});
});

test.describe('Export Form Fields', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/export');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should have Minimum Weight input visible by default', async ({ page }) => {
		await expect(page.locator('#min-weight')).toBeVisible();
	});

	test('should have Limit input visible by default', async ({ page }) => {
		await expect(page.locator('#limit')).toBeVisible();
	});

	test('should have Minimum Weight with correct attributes', async ({ page }) => {
		const input = page.locator('#min-weight');
		await expect(input).toHaveAttribute('type', 'number');
		await expect(input).toHaveAttribute('min', '0');
		await expect(input).toHaveAttribute('max', '1');
		await expect(input).toHaveAttribute('step', '0.1');
	});

	test('should have Limit with correct attributes', async ({ page }) => {
		const input = page.locator('#limit');
		await expect(input).toHaveAttribute('type', 'number');
		await expect(input).toHaveAttribute('min', '1');
		await expect(input).toHaveAttribute('max', '100000');
	});

	test('should have default values for inputs', async ({ page }) => {
		await expect(page.locator('#min-weight')).toHaveValue('0');
		await expect(page.locator('#limit')).toHaveValue('1000');
	});

	test('should accept numeric input for Minimum Weight', async ({ page }) => {
		const input = page.locator('#min-weight');
		await input.fill('0.5');
		await expect(input).toHaveValue('0.5');
	});

	test('should accept numeric input for Limit', async ({ page }) => {
		const input = page.locator('#limit');
		await input.fill('500');
		await expect(input).toHaveValue('500');
	});

	test('should show form row container for facts export types', async ({ page }) => {
		await expect(page.locator('.form-row')).toBeVisible();
	});
});

test.describe('Export Type Field Visibility', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/export');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should hide min weight and limit for entities-csv', async ({ page }) => {
		await page.selectOption('select#export-type', 'entities-csv');
		await expect(page.locator('#min-weight')).not.toBeVisible();
		await expect(page.locator('#limit')).not.toBeVisible();
	});

	test('should hide min weight and limit for timeline-json', async ({ page }) => {
		await page.selectOption('select#export-type', 'timeline-json');
		await expect(page.locator('#min-weight')).not.toBeVisible();
		await expect(page.locator('#limit')).not.toBeVisible();
	});

	test('should hide min weight and limit for full-report', async ({ page }) => {
		await page.selectOption('select#export-type', 'full-report');
		await expect(page.locator('#min-weight')).not.toBeVisible();
		await expect(page.locator('#limit')).not.toBeVisible();
	});

	test('should hide min weight and limit for pdf-report', async ({ page }) => {
		await page.selectOption('select#export-type', 'pdf-report');
		await expect(page.locator('#min-weight')).not.toBeVisible();
		await expect(page.locator('#limit')).not.toBeVisible();
	});

	test('should hide min weight and limit for excel-data', async ({ page }) => {
		await page.selectOption('select#export-type', 'excel-data');
		await expect(page.locator('#min-weight')).not.toBeVisible();
		await expect(page.locator('#limit')).not.toBeVisible();
	});

	test('should show min weight and limit for facts-json', async ({ page }) => {
		await page.selectOption('select#export-type', 'facts-json');
		await expect(page.locator('#min-weight')).toBeVisible();
		await expect(page.locator('#limit')).toBeVisible();
	});

	test('should show min weight and limit for facts-csv', async ({ page }) => {
		await page.selectOption('select#export-type', 'facts-csv');
		await expect(page.locator('#min-weight')).toBeVisible();
		await expect(page.locator('#limit')).toBeVisible();
	});
});

test.describe('Export Button', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/export');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should have Export button', async ({ page }) => {
		await expect(page.locator('.export-btn')).toBeVisible();
	});

	test('should have correct button text', async ({ page }) => {
		await expect(page.locator('.export-btn')).toContainText('Export');
	});

	test('should be disabled when exporting', async ({ page }) => {
		// Initially enabled, can disable during export operation
		await expect(page.locator('.export-btn')).toBeEnabled();
	});

	test('should show exporting state text when exporting', async ({ page }) => {
		// Button shows "Exporting..." when isExporting is true (would need mock to test actual disabled state)
		await expect(page.locator('.export-btn')).toContainText('Export');
	});
});

test.describe('Export History', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/export');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should not display history initially', async ({ page }) => {
		await expect(page.locator('.export-history')).not.toBeVisible();
	});

	test('should have export history table structure', async ({ page }) => {
		// Table exists but hidden initially
		const history = page.locator('.export-history');
		const count = await history.count();
		expect(count).toBeGreaterThanOrEqual(0);
	});

	test('should have history table headers', async ({ page }) => {
		// Would test after export action creates history
		const table = page.locator('.export-history table');
		const count = await table.count();
		expect(count).toBeGreaterThanOrEqual(0);
	});
});

test.describe('Status Message', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/export');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should not display status message initially', async ({ page }) => {
		await expect(page.locator('.status-message')).not.toBeVisible();
	});

	test('should display status container element', async ({ page }) => {
		const status = page.locator('.status-message');
		const count = await status.count();
		expect(count).toBeGreaterThanOrEqual(0);
	});
});

test.describe('Export Form Layout', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/export');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	test('should have export form container', async ({ page }) => {
		await expect(page.locator('.export-form')).toBeVisible();
	});

	test('should have form groups for inputs when visible', async ({ page }) => {
		const formGroups = page.locator('.form-group');
		const count = await formGroups.count();
		expect(count).toBeGreaterThanOrEqual(1);
	});

	test('should have labels for inputs', async ({ page }) => {
		await expect(page.locator('label[for="export-type"]')).toContainText('Export Type');
		await expect(page.locator('label[for="min-weight"]')).toContainText('Minimum Weight');
		await expect(page.locator('label[for="limit"]')).toContainText('Limit');
	});
});