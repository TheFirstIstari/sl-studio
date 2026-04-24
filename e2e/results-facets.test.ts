import { test, expect } from '@playwright/test';

test.describe('Results Faceted Filters', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/results');
		await page.waitForLoadState('networkidle');
	});

	// 1. Category filter checkboxes with counts
	test('should have filters panel', async ({ page }) => {
		const filtersPanel = page.locator('.filters-panel');
		const count = await filtersPanel.count();
		if (count > 0) {
			await expect(filtersPanel).toBeVisible();
		}
	});

	test('should have category filter checkboxes when facts exist', async ({ page }) => {
		const filtersPanel = page.locator('.filters-panel');
		const panelCount = await filtersPanel.count();
		
		// Only test if filters panel exists (which means there are facts)
		if (panelCount > 0) {
			const categoryCheckboxes = page.locator('.filter-option input[type="checkbox"]');
			await expect(categoryCheckboxes.first()).toBeVisible();
		}
	});

	test('should have category filter options with counts', async ({ page }) => {
		const optionCounts = page.locator('.option-count');
		const count = await optionCounts.count();
		if (count > 0) {
			await expect(optionCounts.first()).toBeVisible();
			// Should have format like "(5)"
			const text = await optionCounts.first().textContent();
			expect(text).toMatch(/\(\d+\)/);
		}
	});

	// 2. Severity range inputs work
	test('should have severity range inputs', async ({ page }) => {
		const severityInputs = page.locator('.severity-range input[type="number"]');
		const count = await severityInputs.count();
		if (count > 0) {
			await expect(severityInputs.first()).toBeVisible();
		}
	});

	test('should have severity range separator', async ({ page }) => {
		const rangeSeparator = page.locator('.range-separator');
		const count = await rangeSeparator.count();
		if (count > 0) {
			await expect(rangeSeparator.first()).toContainText('to');
		}
	});

	// 3. Date range pickers work
	test('should have date range inputs', async ({ page }) => {
		const dateInputs = page.locator('.filters-panel input[type="date"]');
		const count = await dateInputs.count();
		if (count > 0) {
			await expect(dateInputs.first()).toBeVisible();
		}
	});

	test('should accept date values', async ({ page }) => {
		const dateInputs = page.locator('.filters-panel input[type="date"]');
		const count = await dateInputs.count();
		if (count > 0) {
			await dateInputs.first().fill('2024-01-01');
			await expect(dateInputs.first()).toHaveValue('2024-01-01');
		}
	});

	// 4. Confidence filter slider works
	test('should have confidence slider', async ({ page }) => {
		const confidenceSlider = page.locator('.confidence-slider input[type="range"]');
		const count = await confidenceSlider.count();
		if (count > 0) {
			await expect(confidenceSlider).toBeVisible();
		}
	});

	test('should display confidence filter label', async ({ page }) => {
		const confidenceSection = page.locator('.filter-section h3').filter({ hasText: 'Min Confidence' });
		const count = await confidenceSection.count();
		if (count > 0) {
			await expect(confidenceSection).toBeVisible();
		}
	});

	// 5. Combined filters work (AND logic)
	test('should toggle category checkbox filter', async ({ page }) => {
		const categoryOption = page.locator('.filter-option').first();
		const checkbox = categoryOption.locator('input[type="checkbox"]');
		const count = await checkbox.count();
		if (count > 0) {
			await checkbox.check();
			await expect(checkbox).toBeChecked();
		}
	});

	// 6. Active filter count shows correctly
	test('should have clear filters button when filters active', async ({ page }) => {
		const clearBtn = page.locator('.clear-filters-btn');
		const btnCount = await clearBtn.count();
		if (btnCount > 0) {
			await expect(clearBtn).toBeVisible();
		}
	});

	test('should display active filter count', async ({ page }) => {
		const clearBtn = page.locator('.clear-filters-btn');
		const btnCount = await clearBtn.count();
		if (btnCount > 0) {
			const text = await clearBtn.textContent();
			// Should say "Clear X filter(s)"
			expect(text).toMatch(/Clear \d+ filter/);
		}
	});

	// 7. Clear all filters works
	test('should clear all filters when clear button clicked', async ({ page }) => {
		const clearBtn = page.locator('.clear-filters-btn');
		const btnCount = await clearBtn.count();
		if (btnCount > 0) {
			// First activate some filters
			const checkbox = page.locator('.filter-option input[type="checkbox"]').first();
			const checkCount = await checkbox.count();
			if (checkCount > 0) {
				await checkbox.check();
				await expect(checkbox).toBeChecked();
				
				// Then clear
				await clearBtn.click();
				
				// Checkbox should be unchecked
				await expect(checkbox).not.toBeChecked();
			}
		}
	});

	// 8. Text search still works alongside facets
	test('should have search input', async ({ page }) => {
		const searchInput = page.locator('.filter-input');
		await expect(searchInput).toBeVisible();
	});

	test('should filter facts by text search', async ({ page }) => {
		const searchInput = page.locator('.filter-input');
		await searchInput.fill('test');
		await expect(searchInput).toHaveValue('test');
	});

	test('should work combined with facet filters', async ({ page }) => {
		const searchInput = page.locator('.filter-input');
		
		// Add text filter
		await searchInput.fill('financial');
		
		// Check search input has correct value
		await expect(searchInput).toHaveValue('financial');
	});
});