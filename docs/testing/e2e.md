# E2E Tests

## Overview

End-to-end tests use Playwright to test the frontend application across multiple browsers.

## Test Files

Located in `e2e/`:

| File                 | Coverage                                                                |
| -------------------- | ----------------------------------------------------------------------- |
| `dashboard.test.ts`  | Dashboard title, status, stats cards, quick actions, keyboard shortcuts |
| `navigation.test.ts` | Navigation to all 12 pages                                              |
| `export.test.ts`     | Export functionality                                                    |
| `settings.test.ts`   | Settings page                                                           |
| `backup.test.ts`     | Backup/restore workflow                                                 |
| `compare.test.ts`    | Project comparison                                                      |

## Running Tests

### Using mise (recommended)

```bash
mise run e2e              # Run all E2E tests
mise run e2e_ui           # Run with UI mode
```

### Using Playwright directly

```bash
npx playwright test
npx playwright test --ui  # UI mode
npx playwright test --project=chromium  # Single browser
```

## Browsers Tested

| Browser  | Status  |
| -------- | ------- |
| Chromium | Enabled |
| Firefox  | Enabled |
| WebKit   | Enabled |

## Configuration

`playwright.config.ts`:

- Base URL: `http://localhost:1420`
- Test directory: `e2e/`
- Output directory: `test-results/`
- Retries: Configurable for CI

## Test Structure

```typescript
test('description', async ({ page }) => {
	await page.goto('/');
	// Assertions
	await expect(page.locator('selector')).toBeVisible();
});
```
