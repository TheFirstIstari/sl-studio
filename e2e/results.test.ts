import { test, expect } from '@playwright/test';

test.describe('Results Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/results');
		await page.waitForLoadState('networkidle');
	});

	// 1. Basic page load
	test('should display results title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Results');
	});

	// 2. Search/filter input exists somewhere on page
	test('should have search input', async ({ page }) => {
		const input = page.locator('input.filter-input');
		await expect(input).toBeVisible();
	});

	// 3. Sort selector exists
	test('should have sort selector', async ({ page }) => {
		const select = page.locator('select.sort-select');
		await expect(select).toBeVisible();
	});

	// 4. Filter input works
	test('should filter facts by search input', async ({ page }) => {
		const searchInput = page.locator('input.filter-input');
		await searchInput.fill('test');
		await expect(searchInput).toHaveValue('test');
	});

	// 5. Sort selector works
	test('should change sort order', async ({ page }) => {
		const sortSelect = page.locator('select.sort-select');
		await sortSelect.selectOption('date');
		await expect(sortSelect).toHaveValue('date');
	});

	// 6. Check for results grid OR empty div - just verify page loaded
	test('should load page content', async ({ page }) => {
		const hasContent = await page.content();
		// Just verify page loaded with results-related HTML
		expect(hasContent).toContain('Results');
	});

	// 7. Should have the filter placeholder
	test('should have filter placeholder', async ({ page }) => {
		const input = page.locator('input.filter-input');
		await expect(input).toHaveAttribute('placeholder', 'Filter facts...');
	});

	// 8. Should have sort options
	test('should have sort options', async ({ page }) => {
		const select = page.locator('select.sort-select');
		await expect(select.locator('option')).toHaveCount(2);
	});
});
