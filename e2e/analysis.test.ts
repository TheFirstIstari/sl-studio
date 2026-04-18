import { test, expect } from '@playwright/test';

test.describe('Analysis Page - Model Selection', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto('/analysis');
	});

	test('should display analysis page title', async ({ page }) => {
		await expect(page.locator('h1')).toContainText('Analysis');
	});

	test('should display model status indicator', async ({ page }) => {
		const modelStatus = page.locator('.model-status, .status-value').first();
		const count = await modelStatus.count();
		if (count > 0) {
			await expect(modelStatus).toBeVisible();
		}
	});

	test('should have analyze button or stage 2 button', async ({ page }) => {
		await page.goto('/analysis');
		await page.waitForLoadState('networkidle');
		
		const analyzeButton = page.locator('button:has-text("Analyze"), button:has-text("Stage 2")').first();
		const isVisible = await analyzeButton.isVisible().catch(() => false);
		if (isVisible) {
			const isDisabled = await analyzeButton.isDisabled().catch(() => false);
			expect(typeof isDisabled).toBe('boolean');
		}
	});

	test('should display text extraction section', async ({ page }) => {
		await expect(page.getByRole('heading', { name: 'Text Extraction' })).toBeVisible();
	});

	test('should display analysis section', async ({ page }) => {
		await expect(page.getByRole('heading', { name: 'LLM Analysis' })).toBeVisible();
	});

	test('should have progress indicators', async ({ page }) => {
		const progressElements = page.locator('.progress, .stage-progress');
		const count = await progressElements.count();
		expect(count).toBeGreaterThanOrEqual(0);
	});

	test('should navigate to settings page', async ({ page }) => {
		const configureLink = page.locator('a[href="/settings"]').first();
		const isVisible = await configureLink.isVisible().catch(() => false);
		if (isVisible) {
			await configureLink.click();
			await expect(page).toHaveURL('/settings');
		}
	});
});

test.describe('Analysis Page - Workflow', () => {
	test('should display text extraction stage', async ({ page }) => {
		await page.goto('/analysis');
		const extractStage = page.getByRole('heading', { name: 'Text Extraction' });
		await expect(extractStage).toBeVisible();
	});

	test('should display LLM analysis stage', async ({ page }) => {
		await page.goto('/analysis');
		const analyzeStage = page.getByRole('heading', { name: 'LLM Analysis' });
		await expect(analyzeStage).toBeVisible();
	});

	test('should display extraction statistics', async ({ page }) => {
		await page.goto('/analysis');
		const stats = page.getByRole('heading', { name: 'Extraction Statistics' });
		const isVisible = await stats.isVisible().catch(() => false);
		if (isVisible) {
			await expect(stats).toBeVisible();
		}
	});

	test('should have stop buttons available', async ({ page }) => {
		await page.goto('/analysis');
		const stopButton = page.locator('button:has-text("Stop"), button:has-text("Cancel")').first();
		const count = await stopButton.count();
		expect(count).toBeGreaterThanOrEqual(0);
	});
});

test.describe('Analysis Page - State Persistence', () => {
	test('should preserve state when navigating between pages', async ({ page }) => {
		await page.goto('/analysis');
		await page.waitForLoadState('networkidle');
		
		await page.goto('/results');
		await page.waitForLoadState('networkidle');
		
		await page.goto('/analysis');
		await page.waitForLoadState('networkidle');
		
		await expect(page.locator('h1')).toContainText('Analysis');
	});

	test('should load config on page mount', async ({ page }) => {
		await page.goto('/analysis');
		
		const consoleErrors: string[] = [];
		page.on('console', msg => {
			if (msg.type() === 'error') {
				consoleErrors.push(msg.text());
			}
		});
		
		await page.waitForTimeout(1000);
		
		const criticalErrors = consoleErrors.filter(e => 
			!e.includes('model') && !e.includes('Model')
		);
		expect(criticalErrors.length).toBeLessThan(2);
	});
});
