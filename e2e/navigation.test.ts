import { test, expect } from '@playwright/test';

test.describe('Navigation', () => {
	test('should load dashboard page', async ({ page }) => {
		await page.goto('/');
		await expect(page.locator('h1')).toContainText('Dashboard');
	});

	test('should navigate to analysis page', async ({ page }) => {
		await page.goto('/');
		await page.click('a[href="/analysis"]');
		await expect(page).toHaveURL('/analysis');
		await expect(page.locator('h1')).toContainText('Analysis');
	});

	test('should navigate to results page', async ({ page }) => {
		await page.goto('/');
		await page.click('a[href="/results"]');
		await expect(page).toHaveURL('/results');
		await expect(page.locator('h1')).toContainText('Results');
	});

	test('should navigate to timeline page', async ({ page }) => {
		await page.goto('/');
		await page.click('a[href="/timeline"]');
		await expect(page).toHaveURL('/timeline');
		await expect(page.locator('h1')).toContainText('Timeline');
	});

	test('should navigate to statistics page', async ({ page }) => {
		await page.goto('/');
		await page.click('a[href="/stats"]');
		await expect(page).toHaveURL('/stats');
		await expect(page.locator('h1')).toContainText('Statistics');
	});

	test('should navigate to network page', async ({ page }) => {
		await page.goto('/');
		await page.click('a[href="/network"]');
		await expect(page).toHaveURL('/network');
		await expect(page.locator('h1')).toContainText('Network');
	});

	test('should navigate to maps page', async ({ page }) => {
		await page.goto('/');
		await page.click('a[href="/maps"]');
		await expect(page).toHaveURL('/maps');
		await expect(page.locator('h1')).toContainText('Geographic');
	});

	test('should navigate to anomalies page', async ({ page }) => {
		await page.goto('/');
		await page.click('a[href="/anomalies"]');
		await expect(page).toHaveURL('/anomalies');
		await expect(page.locator('h1')).toContainText('Anomaly');
	});

	test('should navigate to export page', async ({ page }) => {
		await page.goto('/');
		await page.click('a[href="/export"]');
		await expect(page).toHaveURL('/export');
		await expect(page.locator('h1')).toContainText('Export');
	});

	test('should navigate to compare page', async ({ page }) => {
		await page.goto('/');
		await page.click('a[href="/compare"]');
		await expect(page).toHaveURL('/compare');
		await expect(page.locator('h1')).toContainText('Compare');
	});

	test('should navigate to backup page', async ({ page }) => {
		await page.goto('/');
		await page.click('a[href="/backup"]');
		await expect(page).toHaveURL('/backup');
		await expect(page.locator('h1')).toContainText('Backup');
	});

	test('should navigate to settings page', async ({ page }) => {
		await page.goto('/');
		await page.click('a[href="/settings"]');
		await expect(page).toHaveURL('/settings');
		await expect(page.locator('h1')).toContainText('Settings');
	});
});
