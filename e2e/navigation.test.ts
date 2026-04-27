import { test, expect } from '@playwright/test';

test.describe('Main Navigation Routes', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	// Dashboard
	test('should load dashboard page', async ({ page }) => {
		await page.goto('/');
		await expect(page.locator('h1')).toContainText('Dashboard');
	});

	// Analysis page navigation
	test('should navigate to analysis page', async ({ page }) => {
		await page.click('a[href="/analysis"]');
		await expect(page).toHaveURL('/analysis');
		await expect(page.locator('h1')).toContainText('Analysis');
	});

	// Results page navigation
	test('should navigate to results page', async ({ page }) => {
		await page.click('a[href="/results"]');
		await expect(page).toHaveURL('/results');
		await expect(page.locator('h1')).toContainText('Results');
	});

	// Timeline page navigation
	test('should navigate to timeline page', async ({ page }) => {
		await page.click('a[href="/timeline"]');
		await expect(page).toHaveURL('/timeline');
		await expect(page.locator('h1')).toContainText('Timeline');
	});

	// Statistics page navigation
	test('should navigate to statistics page', async ({ page }) => {
		await page.click('a[href="/stats"]');
		await expect(page).toHaveURL('/stats');
		await expect(page.locator('h1')).toContainText('Statistics');
	});

	// Network page navigation
	test('should navigate to network page', async ({ page }) => {
		await page.click('a[href="/network"]');
		await expect(page).toHaveURL('/network');
		await expect(page.locator('h1')).toContainText('Network');
	});

	// Maps page navigation
	test('should navigate to maps page', async ({ page }) => {
		await page.click('a[href="/maps"]');
		await expect(page).toHaveURL('/maps');
		await expect(page.locator('h1')).toContainText('Geographic Locations');
	});

	// Anomalies page navigation
	test('should navigate to anomalies page', async ({ page }) => {
		await page.click('a[href="/anomalies"]');
		await expect(page).toHaveURL('/anomalies');
		await expect(page.locator('h1')).toContainText('Anomaly Detection');
	});

	// Export page navigation
	test('should navigate to export page', async ({ page }) => {
		await page.click('a[href="/export"]');
		await expect(page).toHaveURL('/export');
		await expect(page.locator('h1')).toContainText('Export');
	});

	// Compare page navigation
	test('should navigate to compare page', async ({ page }) => {
		await page.click('a[href="/compare"]');
		await expect(page).toHaveURL('/compare');
		await expect(page.locator('h1')).toContainText('Compare Projects');
	});

	// Backup page navigation
	test('should navigate to backup page', async ({ page }) => {
		await page.click('a[href="/backup"]');
		await expect(page).toHaveURL('/backup');
		await expect(page.locator('h1')).toContainText('Backup');
	});

	// Settings page navigation
	test('should navigate to settings page', async ({ page }) => {
		await page.click('a[href="/settings"]');
		await expect(page).toHaveURL('/settings');
		await expect(page.locator('h1')).toContainText('Settings');
	});
});

test.describe('Direct URL Access', () => {
	test('should load analysis page directly via URL', async ({ page }) => {
		await page.goto('/analysis');
		await expect(page.locator('h1')).toContainText('Analysis');
	});

	test('should load results page directly via URL', async ({ page }) => {
		await page.goto('/results');
		await expect(page.locator('h1')).toContainText('Results');
	});

	test('should load timeline page directly via URL', async ({ page }) => {
		await page.goto('/timeline');
		await expect(page.locator('h1')).toContainText('Timeline');
	});

	test('should load statistics page directly via URL', async ({ page }) => {
		await page.goto('/stats');
		await expect(page.locator('h1')).toContainText('Statistics');
	});

	test('should load network page directly via URL', async ({ page }) => {
		await page.goto('/network');
		await expect(page.locator('h1')).toContainText('Network');
	});

	test('should load maps page directly via URL', async ({ page }) => {
		await page.goto('/maps');
		await expect(page.locator('h1')).toContainText('Geographic Locations');
	});

	test('should load anomalies page directly via URL', async ({ page }) => {
		await page.goto('/anomalies');
		await expect(page.locator('h1')).toContainText('Anomaly Detection');
	});

	test('should load export page directly via URL', async ({ page }) => {
		await page.goto('/export');
		await expect(page.locator('h1')).toContainText('Export Data');
	});

	test('should load compare page directly via URL', async ({ page }) => {
		await page.goto('/compare');
		await expect(page.locator('h1')).toContainText('Compare Projects');
	});

	test('should load backup page directly via URL', async ({ page }) => {
		await page.goto('/backup');
		await expect(page.locator('h1')).toContainText('Backup & Restore');
	});

	test('should load settings page directly via URL', async ({ page }) => {
		await page.goto('/settings');
		await expect(page.locator('h1')).toContainText('Settings');
	});

	test('should load dashboard page directly via URL', async ({ page }) => {
		await page.goto('/');
		await expect(page.locator('h1')).toContainText('Dashboard');
	});
});

test.describe('Navigation URL Format', () => {
	test('should have correct URL format for all routes', async ({ page }) => {
		// Test each route has proper URL
		await page.goto('/analysis');
		await expect(page).toHaveURL(/.*\/analysis/);

		await page.goto('/results');
		await expect(page).toHaveURL(/.*\/results/);

		await page.goto('/timeline');
		await expect(page).toHaveURL(/.*\/timeline/);

		await page.goto('/stats');
		await expect(page).toHaveURL(/.*\/stats/);

		await page.goto('/network');
		await expect(page).toHaveURL(/.*\/network/);

		await page.goto('/maps');
		await expect(page).toHaveURL(/.*\/maps/);

		await page.goto('/anomalies');
		await expect(page).toHaveURL(/.*\/anomalies/);

		await page.goto('/export');
		await expect(page).toHaveURL(/.*\/export/);

		await page.goto('/compare');
		await expect(page).toHaveURL(/.*\/compare/);

		await page.goto('/backup');
		await expect(page).toHaveURL(/.*\/backup/);

		await page.goto('/settings');
		await expect(page).toHaveURL(/.*\/settings/);
	});
});

test.describe('Navigation Links on Pages', () => {
	test('should have accessible navigation links', async ({ page }) => {
		await page.goto('/');
		// Verify nav links exist and have href attributes
		const navLinks = page.locator('nav a');
		await expect(navLinks.first()).toHaveAttribute('href');
	});

	test('should navigate between pages sequentially', async ({ page }) => {
		await page.goto('/');
		await page.click('a[href="/analysis"]');
		await expect(page).toHaveURL('/analysis');
		await page.click('a[href="/results"]');
		await expect(page).toHaveURL('/results');
		await page.click('a[href="/timeline"]');
		await expect(page).toHaveURL('/timeline');
	});
});

test.describe('Page Title Verification', () => {
	test('should display correct title for analysis page', async ({ page }) => {
		await page.goto('/analysis');
		await expect(page.locator('h1')).toContainText('Analysis');
	});

	test('should display correct title for results page', async ({ page }) => {
		await page.goto('/results');
		await expect(page.locator('h1')).toContainText('Results');
	});

	test('should display correct title for timeline page', async ({ page }) => {
		await page.goto('/timeline');
		await expect(page.locator('h1')).toContainText('Timeline');
	});

	test('should display correct title for statistics page', async ({ page }) => {
		await page.goto('/stats');
		await expect(page.locator('h1')).toContainText('Statistics');
	});

	test('should display correct title for network page', async ({ page }) => {
		await page.goto('/network');
		await expect(page.locator('h1')).toContainText('Network');
	});

	test('should display correct title for maps page', async ({ page }) => {
		await page.goto('/maps');
		await expect(page.locator('h1')).toContainText('Geographic Locations');
	});

	test('should display correct title for anomalies page', async ({ page }) => {
		await page.goto('/anomalies');
		await expect(page.locator('h1')).toContainText('Anomaly Detection');
	});

	test('should display correct title for export page', async ({ page }) => {
		await page.goto('/export');
		await expect(page.locator('h1')).toContainText('Export Data');
	});

	test('should display correct title for compare page', async ({ page }) => {
		await page.goto('/compare');
		await expect(page.locator('h1')).toContainText('Compare Projects');
	});

	test('should display correct title for backup page', async ({ page }) => {
		await page.goto('/backup');
		await expect(page.locator('h1')).toContainText('Backup & Restore');
	});

	test('should display correct title for settings page', async ({ page }) => {
		await page.goto('/settings');
		await expect(page.locator('h1')).toContainText('Settings');
	});
});
