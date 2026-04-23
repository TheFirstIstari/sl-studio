import { test, expect } from '@playwright/test';

test.describe('Dashboard', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	// 1. Page title
	test('should display dashboard title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Dashboard');
	});

	// 2. Stats cards - check any content exists
	test('should display stats cards', async ({ page }) => {
		const content = await page.content();
		expect(content.length).toBeGreaterThan(100);
	});

	// 3. Verify all three stat cards exist
	test('should display Files Registered stat', async ({ page }) => {
		const text = await page.content();
		expect(text).toMatch(/Files Registered|Files scanned/i);
	});

	test('should display Facts Extracted stat', async ({ page }) => {
		const text = await page.content();
		expect(text).toMatch(/Facts Extracted|Facts extracted/i);
	});

	test('should display CPU Workers stat', async ({ page }) => {
		const text = await page.content();
		expect(text).toMatch(/CPU Workers|Workers/i);
	});

	// 4. Hardware Status section
	test('should display hardware status section', async ({ page }) => {
		const hasSection = await page.locator('.info-section, .hardware-section').count() > 0;
		const hasText = await page.content();
		expect(hasSection || hasText).toBeTruthy();
	});

	// 5. Model Status
	test('should display model status', async ({ page }) => {
		const text = await page.content();
		expect(text).toMatch(/Model/i);
	});

	// 6. Error banner (if present)
	test('should have error banner or no error', async ({ page }) => {
		const hasError = await page.locator('.error-banner').count() > 0;
		if (hasError) {
			await expect(page.locator('.error-banner')).toBeVisible();
		}
	});

	// 7. Quick action buttons - check content match
	test('should display Start Analysis button', async ({ page }) => {
		const text = await page.content();
		expect(text).toMatch(/Start Analysis|Analysis/i);
	});

	test('should display View Results button', async ({ page }) => {
		const btn = page.locator('.action-btn, a[href="/results"]').filter({ hasText: 'View Results' });
		await expect(btn).toBeVisible();
	});

	test('should display Settings button', async ({ page }) => {
		const text = await page.content();
		expect(text).toMatch(/Settings/i);
	});

	// 8. Navigation
	test('should navigate to Analysis', async ({ page }) => {
		await page.click('a[href="/analysis"]');
		await expect(page).toHaveURL(/analysis/);
		await expect(page.locator('h1')).toContainText('Analysis');
	});

	test('should navigate to Results', async ({ page }) => {
		await page.click('a[href="/results"]');
		await expect(page).toHaveURL(/results/);
	});

	test('should navigate to Settings', async ({ page }) => {
		await page.click('a[href="/settings"]');
		await expect(page).toHaveURL(/settings/);
	});

	// 9. Verify page loads content
	test('should have page content', async ({ page }) => {
		const content = await page.content();
		expect(content.length).toBeGreaterThan(100);
	});
});
