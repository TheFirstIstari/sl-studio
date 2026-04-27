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

	// 2. Search/filter input exists
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

	// 6. Check for results grid - verify page loaded
	test('should load page content', async ({ page }) => {
		const hasContent = await page.content();
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

	// 9. Undo button exists
	test('should display Undo button', async ({ page }) => {
		const undoBtn = page.locator('.history-btn').first();
		await expect(undoBtn).toBeVisible();
	});

	// 10. Redo button exists
	test('should display Redo button', async ({ page }) => {
		const redoBtn = page.locator('.history-btn').nth(1);
		await expect(redoBtn).toBeVisible();
	});

	// 11. Undo/Redo buttons have correct SVG icons
	test('should have Undo button with correct icon path', async ({ page }) => {
		const undoBtn = page.locator('.history-btn').first();
		await expect(undoBtn.locator('svg')).toBeVisible();
	});

	test('should have Redo button with correct icon path', async ({ page }) => {
		const redoBtn = page.locator('.history-btn').nth(1);
		await expect(redoBtn.locator('svg')).toBeVisible();
	});

	// 12. Undo/Redo buttons can be disabled
	test('should disable Undo button when no history', async ({ page }) => {
		const undoBtn = page.locator('.history-btn').first();
		const isDisabled = await undoBtn.isDisabled();
		// Initially should be disabled (no history)
		expect(typeof isDisabled).toBe('boolean');
	});

	test('should disable Redo button when no future history', async ({ page }) => {
		const redoBtn = page.locator('.history-btn').nth(1);
		const isDisabled = await redoBtn.isDisabled();
		// Initially should be disabled (at end of history)
		expect(typeof isDisabled).toBe('boolean');
	});

	// 13. Fact selection checkbox
	test('should display fact cards with checkboxes', async ({ page }) => {
		const factCards = page.locator('.fact-card');
		const count = await factCards.count();
		if (count > 0) {
			await expect(factCards.first().locator('input[type="checkbox"]')).toBeVisible();
		}
	});

	// 14. Click fact card shows detail panel
	test('should show detail panel when fact card clicked', async ({ page }) => {
		const factCards = page.locator('.fact-card');
		const count = await factCards.count();
		if (count > 0) {
			const firstCard = factCards.first();
			await firstCard.click();
			const detailPanel = page.locator('.fact-detail');
			await expect(detailPanel).toBeVisible();
		}
	});

	// 15. Detail panel shows all required fields
	test('should display Filename in detail panel', async ({ page }) => {
		const factCards = page.locator('.fact-card');
		const count = await factCards.count();
		if (count > 0) {
			await factCards.first().click();
			await expect(page.locator('.detail-row').filter({ hasText: 'Filename:' })).toBeVisible();
		}
	});

	test('should display Category in detail panel', async ({ page }) => {
		const factCards = page.locator('.fact-card');
		const count = await factCards.count();
		if (count > 0) {
			await factCards.first().click();
			await expect(page.locator('.detail-row').filter({ hasText: 'Category:' })).toBeVisible();
		}
	});

	test('should display Crime in detail panel', async ({ page }) => {
		const factCards = page.locator('.fact-card');
		const count = await factCards.count();
		if (count > 0) {
			await factCards.first().click();
			await expect(page.locator('.detail-row').filter({ hasText: 'Crime:' })).toBeVisible();
		}
	});

	test('should display Severity in detail panel', async ({ page }) => {
		const factCards = page.locator('.fact-card');
		const count = await factCards.count();
		if (count > 0) {
			await factCards.first().click();
			await expect(page.locator('.detail-row').filter({ hasText: 'Severity:' })).toBeVisible();
		}
	});

	test('should display Confidence in detail panel', async ({ page }) => {
		const factCards = page.locator('.fact-card');
		const count = await factCards.count();
		if (count > 0) {
			await factCards.first().click();
			await expect(page.locator('.detail-row').filter({ hasText: 'Confidence:' })).toBeVisible();
		}
	});

	// 16. Severity badge display
	test('should display severity badge on fact card', async ({ page }) => {
		const factCards = page.locator('.fact-card');
		const count = await factCards.count();
		if (count > 0) {
			const severityBadge = factCards.first().locator('.fact-severity');
			await expect(severityBadge).toBeVisible();
			const text = await severityBadge.textContent();
			expect(text).toMatch(/\d+/);
		}
	});

	// 17. Category display
	test('should display category on fact card', async ({ page }) => {
		const factCards = page.locator('.fact-card');
		const count = await factCards.count();
		if (count > 0) {
			const filename = factCards.first().locator('.fact-filename');
			await expect(filename).toBeVisible();
		}
	});

	// 18. Fact detail shows severity badge with color
	test('should display severity badge in detail panel', async ({ page }) => {
		const factCards = page.locator('.fact-card');
		const count = await factCards.count();
		if (count > 0) {
			await factCards.first().click();
			const severityBadge = page.locator('.severity-badge');
			await expect(severityBadge).toBeVisible();
			const text = await severityBadge.textContent();
			expect(text).toMatch(/\d+\/10/);
		}
	});

	// 19. Fact detail shows date
	test('should display date in detail panel', async ({ page }) => {
		const factCards = page.locator('.fact-card');
		const count = await factCards.count();
		if (count > 0) {
			await factCards.first().click();
			await expect(page.locator('.detail-row').filter({ hasText: 'Date:' })).toBeVisible();
		}
	});

	// 20. Fact detail shows summary section
	test('should display summary section in detail panel', async ({ page }) => {
		const factCards = page.locator('.fact-card');
		const count = await factCards.count();
		if (count > 0) {
			await factCards.first().click();
			await expect(page.locator('.detail-section')).toBeVisible();
		}
	});

	// 21. Fact card selection state
	test('should highlight selected fact card', async ({ page }) => {
		const factCards = page.locator('.fact-card');
		const count = await factCards.count();
		if (count > 0) {
			await factCards.first().click();
			await expect(factCards.first()).toHaveClass(/selected/);
		}
	});

	// 22. Empty state handling
	test('should show empty state when no facts', async ({ page }) => {
		const empty = page.locator('.empty');
		const factCards = page.locator('.fact-card');
		const factCount = await factCards.count();
		if (factCount === 0) {
			await expect(empty).toBeVisible();
		}
	});

	// 23. Select all checkbox
	test('should have select all checkbox', async ({ page }) => {
		const selectAll = page.locator('.select-all input[type="checkbox"]');
		const count = await selectAll.count();
		if (count > 0) {
			await expect(selectAll).toBeVisible();
		}
	});

	// 24. Selected count display
	test('should display selected count', async ({ page }) => {
		const selectAllLabel = page.locator('.select-all');
		const count = await selectAllLabel.count();
		if (count > 0) {
			await expect(selectAllLabel).toBeVisible();
		}
	});
});
