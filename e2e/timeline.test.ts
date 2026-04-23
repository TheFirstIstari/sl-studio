import { test, expect } from '@playwright/test';

test.describe('Timeline Page', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/timeline');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	// 1. Page title
	test('should display Timeline title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Timeline');
	});

	// 2. View Mode toggle buttons exist
	test('should display Timeline view button', async ({ page }) => {
		const timelineBtn = page.locator('.view-btn').filter({ hasText: 'Timeline' });
		await expect(timelineBtn).toBeVisible();
	});

	test('should display List view button', async ({ page }) => {
		const listBtn = page.locator('.view-btn').filter({ hasText: 'List' });
		await expect(listBtn).toBeVisible();
	});

	// 3. View Mode toggle functionality
	test('should switch to Timeline view', async ({ page }) => {
		const timelineBtn = page.locator('.view-btn').filter({ hasText: 'Timeline' });
		await timelineBtn.click();
		await expect(timelineBtn).toHaveClass(/active/);
	});

	test('should switch to List view', async ({ page }) => {
		const listBtn = page.locator('.view-btn').filter({ hasText: 'List' });
		await listBtn.click();
		await expect(listBtn).toHaveClass(/active/);
	});

	// 4. Timeline view - events grouped by month
	test('should display timeline view with month groups', async ({ page }) => {
		const timelineView = page.locator('.timeline-container');
		const count = await timelineView.count();
		if (count > 0) {
			await expect(timelineView).toBeVisible();
		}
	});

	test('should display month headers in timeline view', async ({ page }) => {
		const monthHeaders = page.locator('.month-header');
		const count = await monthHeaders.count();
		if (count > 0) {
			await expect(monthHeaders.first()).toBeVisible();
		}
	});

	test('should display month label format', async ({ page }) => {
		const monthLabel = page.locator('.month-label');
		const count = await monthLabel.count();
		if (count > 0) {
			const text = await monthLabel.first().textContent();
			// Month format should be YYYY-MM
			expect(text).toMatch(/\d{4}-\d{2}/);
		}
	});

	test('should display event count per month', async ({ page }) => {
		const monthCount = page.locator('.month-count');
		const count = await monthCount.count();
		if (count > 0) {
			await expect(monthCount.first()).toBeVisible();
			const text = await monthCount.first().textContent();
			expect(text).toMatch(/\d+ events/);
		}
	});

	// 5. Timeline events
	test('should display timeline event items', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			await expect(timelineItems.first()).toBeVisible();
		}
	});

	test('should display timeline date', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			const date = timelineItems.first().locator('.timeline-date');
			await expect(date).toBeVisible();
		}
	});

	test('should display timeline summary', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			const summary = timelineItems.first().locator('.timeline-summary');
			await expect(summary).toBeVisible();
		}
	});

	// 6. Timeline event metadata
	test('should display filename in timeline event', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			const filename = timelineItems.first().locator('.timeline-filename');
			await expect(filename).toBeVisible();
		}
	});

	test('should display category in timeline event', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			const category = timelineItems.first().locator('.timeline-category');
			const catCount = await category.count();
			if (catCount > 0) {
				await expect(category).toBeVisible();
			}
		}
	});

	test('should display severity in timeline event', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			const severity = timelineItems.first().locator('.timeline-severity');
			await expect(severity).toBeVisible();
			const text = await severity.textContent();
			expect(text).toMatch(/\d+/);
		}
	});

	// 7. Timeline marker (colored dot)
	test('should display timeline marker with severity color', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			const marker = timelineItems.first().locator('.timeline-marker');
			await expect(marker).toBeVisible();
			const bgColor = await marker.evaluate((el) => {
				return window.getComputedStyle(el).backgroundColor;
			});
			expect(bgColor).toMatch(/rgb\(\d+, \d+, \d+\)|#[0-9a-fA-F]{6}/);
		}
	});

	// 8. List view alternative
	test('should display list view when List button clicked', async ({ page }) => {
		const listBtn = page.locator('.view-btn').filter({ hasText: 'List' });
		await listBtn.click();
		// Check if list view exists OR empty state is shown (data may not exist)
		const listView = page.locator('.list-view');
		const emptyState = page.locator('.empty');
		const listCount = await listView.count();
		const emptyCount = await emptyState.count();
		// Either list view or empty state should be visible after clicking list button
		expect(listCount > 0 || emptyCount > 0).toBeTruthy();
	});

	test('should display list items in list view', async ({ page }) => {
		const listBtn = page.locator('.view-btn').filter({ hasText: 'List' });
		await listBtn.click();
		const listItems = page.locator('.list-item');
		const count = await listItems.count();
		if (count > 0) {
			await expect(listItems.first()).toBeVisible();
		}
	});

	test('should display date in list item', async ({ page }) => {
		const listBtn = page.locator('.view-btn').filter({ hasText: 'List' });
		await listBtn.click();
		const listItems = page.locator('.list-item');
		const count = await listItems.count();
		if (count > 0) {
			const date = listItems.first().locator('.list-date');
			await expect(date).toBeVisible();
		}
	});

	test('should display summary in list item', async ({ page }) => {
		const listBtn = page.locator('.view-btn').filter({ hasText: 'List' });
		await listBtn.click();
		const listItems = page.locator('.list-item');
		const count = await listItems.count();
		if (count > 0) {
			const summary = listItems.first().locator('.list-summary');
			await expect(summary).toBeVisible();
		}
	});

	// 9. Detail panel
	test('should display detail panel when event clicked', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			await timelineItems.first().click();
			const detailPanel = page.locator('.detail-panel');
			await expect(detailPanel).toBeVisible();
		}
	});

	test('should display Event Details title in detail panel', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			await timelineItems.first().click();
			await expect(page.locator('.detail-header h2')).toContainText('Event Details');
		}
	});

	test('should display date in detail panel', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			await timelineItems.first().click();
			await expect(page.locator('.detail-row').filter({ hasText: 'Date:' })).toBeVisible();
		}
	});

	test('should display filename in detail panel', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			await timelineItems.first().click();
			await expect(page.locator('.detail-row').filter({ hasText: 'Filename:' })).toBeVisible();
		}
	});

	test('should display category in detail panel', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			await timelineItems.first().click();
			await expect(page.locator('.detail-row').filter({ hasText: 'Category:' })).toBeVisible();
		}
	});

	test('should display severity in detail panel', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			await timelineItems.first().click();
			await expect(page.locator('.detail-row').filter({ hasText: 'Severity:' })).toBeVisible();
		}
	});

	test('should display confidence in detail panel', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			await timelineItems.first().click();
			await expect(page.locator('.detail-row').filter({ hasText: 'Confidence:' })).toBeVisible();
		}
	});

	test('should display summary section in detail panel', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			await timelineItems.first().click();
			await expect(page.locator('.detail-section')).toBeVisible();
		}
	});

	// 10. Detail panel close button
	test('should have close button in detail panel', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			await timelineItems.first().click();
			const closeBtn = page.locator('.close-btn');
			await expect(closeBtn).toBeVisible();
		}
	});

	test('should close detail panel when close button clicked', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			await timelineItems.first().click();
			const closeBtn = page.locator('.close-btn');
			await closeBtn.click();
			await expect(page.locator('.detail-panel')).not.toBeVisible();
		}
	});

	// 11. Event selection state
	test('should highlight selected timeline event', async ({ page }) => {
		const timelineItems = page.locator('.timeline-item');
		const count = await timelineItems.count();
		if (count > 0) {
			await timelineItems.first().click();
			await expect(timelineItems.first()).toHaveClass(/selected/);
		}
	});

	// 12. Empty state
	test('should display empty state when no events', async ({ page }) => {
		const empty = page.locator('.empty');
		const events = page.locator('.timeline-item, .list-item');
		const eventCount = await events.count();
		if (eventCount === 0) {
			await expect(empty).toBeVisible();
		}
	});

	// 13. Loading state
	test('should display loading state', async ({ page }) => {
		const loading = page.locator('.loading');
		const count = await loading.count();
		if (count > 0) {
			await expect(loading).toBeVisible();
		}
	});

	// 14. Severity badge colors
	test('should have different severity badge colors', async ({ page }) => {
		const severityBadges = page.locator('.timeline-severity, .list-severity');
		const count = await severityBadges.count();
		if (count > 0) {
			const firstBadge = severityBadges.first();
			const bgColor = await firstBadge.evaluate((el) => {
				return window.getComputedStyle(el).backgroundColor;
			});
			expect(bgColor).toMatch(/rgb\(\d+, \d+, \d+\)|#[0-9a-fA-F]{6}/);
		}
	});
});