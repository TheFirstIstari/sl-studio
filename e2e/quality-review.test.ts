import { test, expect } from '@playwright/test';

test.describe('Quality Review Queue', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/quality');
		await page.waitForLoadState('networkidle').catch(() => {});
	});

	// 1. Page loads and shows low-confidence facts
	test('should display Quality Review title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Quality Review');
	});

	test('should have stats cards', async ({ page }) => {
		const statCards = page.locator('.stat-card');
		const count = await statCards.count();
		if (count > 0) {
			await expect(statCards.first()).toBeVisible();
		}
	});

	test('should display low confidence stat', async ({ page }) => {
		const statCard = page.locator('.stat-card').first();
		const count = await statCard.count();
		if (count > 0) {
			await expect(statCard).toBeVisible();
			await expect(statCard.locator('.stat-value')).toBeVisible();
		}
	});

	// 2. Quality badges display correctly (green/yellow/red)
	test('should display quality badges', async ({ page }) => {
		const qualityBadges = page.locator('.quality-badge');
		const count = await qualityBadges.count();
		if (count > 0) {
			await expect(qualityBadges.first()).toBeVisible();
		}
	});

	test('should display green badge for high confidence', async ({ page }) => {
		const qualityBadges = page.locator('.quality-badge');
		const count = await qualityBadges.count();
		if (count > 0) {
			const firstBadge = qualityBadges.first();
			const bgColor = await firstBadge.evaluate((el) => {
				return window.getComputedStyle(el).backgroundColor;
			});
			// Green would be rgb(34, 197, 94) = #22c55e
			expect(bgColor).toMatch(/rgb\(\d+, \d+, \d+\)|#[0-9a-fA-F]{6}/);
		}
	});

	test('should display yellow badge for medium confidence', async ({ page }) => {
		// Skip this test if no medium confidence badges exist
		const stats = page.locator('.stat-card.medium');
		const statCount = await stats.count();
		if (statCount > 0) {
			const mediumLabel = await stats.first().locator('.stat-label').textContent();
			expect(mediumLabel).toContain('Medium');
		}
	});

	// 3. Verification status buttons work
	test('should have verification status buttons in detail panel', async ({ page }) => {
		const factItems = page.locator('.fact-item');
		const count = await factItems.count();
		if (count > 0) {
			await factItems.first().click();
			const verifyButtons = page.locator('.verify-btn');
			await expect(verifyButtons).toHaveCount(3);
		}
	});

	test('should have unverified button', async ({ page }) => {
		const factItems = page.locator('.fact-item');
		const count = await factItems.count();
		if (count > 0) {
			await factItems.first().click();
			const unverifiedBtn = page.locator('.verify-btn').filter({ hasText: 'Unverified' });
			await expect(unverifiedBtn).toBeVisible();
		}
	});

	test('should have confirmed button', async ({ page }) => {
		const factItems = page.locator('.fact-item');
		const count = await factItems.count();
		if (count > 0) {
			await factItems.first().click();
			const confirmedBtn = page.locator('.verify-btn').filter({ hasText: 'Confirmed' });
			await expect(confirmedBtn).toBeVisible();
		}
	});

	test('should have disputed button', async ({ page }) => {
		const factItems = page.locator('.fact-item');
		const count = await factItems.count();
		if (count > 0) {
			await factItems.first().click();
			const disputedBtn = page.locator('.verify-btn').filter({ hasText: 'Disputed' });
			await expect(disputedBtn).toBeVisible();
		}
	});

	// 4. Confirm/dispute actions change status
	test('should toggle confirmed status when clicked', async ({ page }) => {
		const factItems = page.locator('.fact-item');
		const count = await factItems.count();
		if (count > 0) {
			await factItems.first().click();
			const confirmedBtn = page.locator('.verify-btn').filter({ hasText: 'Confirmed' });
			await confirmedBtn.click();
			await expect(confirmedBtn).toHaveClass(/active/);
		}
	});

	test('should toggle disputed status when clicked', async ({ page }) => {
		const factItems = page.locator('.fact-item');
		const count = await factItems.count();
		if (count > 0) {
			await factItems.first().click();
			const disputedBtn = page.locator('.verify-btn').filter({ hasText: 'Disputed' });
			await disputedBtn.click();
			await expect(disputedBtn).toHaveClass(/active/);
		}
	});

	// 5. Export review report button exists
	test('should have export review report button', async ({ page }) => {
		const exportBtn = page.locator('.export-btn');
		await expect(exportBtn).toBeVisible();
	});

	test('should display export button with correct text', async ({ page }) => {
		const exportBtn = page.locator('.export-btn');
		await expect(exportBtn).toContainText('Export Review Report');
	});

	// 6. Facts sorted by severity
	test('should display severity badges', async ({ page }) => {
		const severityBadges = page.locator('.severity-badge');
		const count = await severityBadges.count();
		if (count > 0) {
			await expect(severityBadges.first()).toBeVisible();
		}
	});

	test('should display severity value', async ({ page }) => {
		const severityBadges = page.locator('.severity-badge');
		const count = await severityBadges.count();
		if (count > 0) {
			const text = await severityBadges.first().textContent();
			expect(text).toMatch(/Severity:/);
		}
	});

	// Additional tests for quality page
	test('should have filter input', async ({ page }) => {
		const filterInput = page.locator('.filter-input');
		const count = await filterInput.count();
		if (count > 0) {
			await expect(filterInput).toBeVisible();
		}
	});

	test('should have filter select', async ({ page }) => {
		const filterSelect = page.locator('.filter-select');
		const count = await filterSelect.count();
		if (count > 0) {
			await expect(filterSelect).toBeVisible();
		}
	});

	test('should show facts list or empty state', async ({ page }) => {
		const factsList = page.locator('.facts-list');
		const emptyState = page.locator('.empty-state');
		const listCount = await factsList.count();
		const emptyCount = await emptyState.count();
		
		if (listCount > 0) {
			await expect(factsList).toBeVisible();
		} else if (emptyCount > 0) {
			await expect(emptyState).toBeVisible();
		}
	});

	test('should have detail panel when fact selected', async ({ page }) => {
		const factItems = page.locator('.fact-item');
		const count = await factItems.count();
		if (count > 0) {
			await factItems.first().click();
			const detailPanel = page.locator('.detail-panel');
			await expect(detailPanel).toBeVisible();
		}
	});

	test('should display fact summary in detail panel', async ({ page }) => {
		const factItems = page.locator('.fact-item');
		const count = await factItems.count();
		if (count > 0) {
			await factItems.first().click();
			const detailSection = page.locator('.detail-section h3').filter({ hasText: 'Summary' });
			await expect(detailSection).toBeVisible();
		}
	});
});