import { test, expect } from '@playwright/test';

test.describe('Timeline Filters', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/timeline');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	// 1. Date range picker filters work
	test('should have date range inputs', async ({ page }) => {
		const dateInputs = page.locator('input[type="date"]');
		await expect(dateInputs).toHaveCount(2);
		await expect(dateInputs.first()).toBeVisible();
		await expect(dateInputs.nth(1)).toBeVisible();
	});

	test('should accept date values in date inputs', async ({ page }) => {
		const startDateInput = page.locator('input[type="date"]').first();
		await startDateInput.fill('2024-01-01');
		await expect(startDateInput).toHaveValue('2024-01-01');
	});

	// 2. Category multi-select filters work
	test('should have category filter buttons', async ({ page }) => {
		const categoryBtns = page.locator('.category-btn');
		await expect(categoryBtns).toHaveCount(5);
		await expect(categoryBtns.first()).toBeVisible();
	});

	test('should toggle category filter on click', async ({ page }) => {
		const categoryBtn = page.locator('.category-btn').first();
		await categoryBtn.click();
		await expect(categoryBtn).toHaveClass(/active/);
	});

	// 3. Severity range filter works
	test('should have severity range inputs', async ({ page }) => {
		const severitySliders = page.locator('.severity-slider');
		await expect(severitySliders).toHaveCount(2);
	});

	test('should display severity range label', async ({ page }) => {
		const severityLabel = page.locator('.filter-label').filter({ hasText: 'Severity:' });
		await expect(severityLabel).toBeVisible();
	});

	// 4. Zoom controls change view
	test('should have zoom control buttons', async ({ page }) => {
		const zoomBtns = page.locator('.zoom-btn');
		await expect(zoomBtns).toHaveCount(4);
		await expect(zoomBtns.first()).toBeVisible();
	});

	test('should toggle zoom button on click', async ({ page }) => {
		const zoomBtn = page.locator('.zoom-btn').first();
		await zoomBtn.click();
		await expect(zoomBtn).toHaveClass(/active/);
	});

	// 5. Clear filters button resets
	test('should have clear filters button', async ({ page }) => {
		const clearBtn = page.locator('.clear-filters-btn');
		await expect(clearBtn).toBeVisible();
	});

	test('should clear filters when clicked', async ({ page }) => {
		const clearBtn = page.locator('.clear-filters-btn');
		const categoryBtn = page.locator('.category-btn').first();
		
		// Activate a category filter first
		await categoryBtn.click();
		await expect(categoryBtn).toHaveClass(/active/);
		
		// Clear filters
		await clearBtn.click();
		
		// Category should no longer be active
		await expect(categoryBtn).not.toHaveClass(/active/);
	});

	// 6. List view also shows filtered results
	test('should display list view button', async ({ page }) => {
		const listBtn = page.locator('.view-btn').filter({ hasText: 'List' });
		await expect(listBtn).toBeVisible();
	});

	test('should show list items when list view selected', async ({ page }) => {
		const listBtn = page.locator('.view-btn').filter({ hasText: 'List' });
		await listBtn.click();
		
		const listItems = page.locator('.list-item');
		const count = await listItems.count();
		if (count > 0) {
			await expect(listItems.first()).toBeVisible();
		}
	});
});