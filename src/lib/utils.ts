/**
 * Shared UI utility functions for SL Studio.
 *
 * Import from routes like:
 *   import { getSeverityColor, getCategoryIcon, getQualityBadgeColor, formatFileSize } from '$lib/utils';
 *
 * All colour functions return CSS custom property references so that theme
 * changes in theme.css propagate automatically.
 */

/**
 * Returns the CSS variable name (as a `var(--...)` string) for a severity
 * score 1–10.  Uses the --color-severity-* tokens defined in theme.css.
 */
export function getSeverityColor(score: number): string {
	if (score >= 8) return 'var(--color-severity-high)';
	if (score >= 6) return 'var(--color-severity-medium-high)';
	if (score >= 4) return 'var(--color-severity-medium)';
	return 'var(--color-severity-low)';
}

/**
 * Returns a simple icon identifier string for a fact category.
 * Used by results and quality pages to pick the right icon branch.
 */
export function getCategoryIcon(category: string | null): string {
	if (category === 'Financial') return 'dollar';
	if (category === 'Legal') return 'scale';
	if (category === 'Digital') return 'laptop';
	if (category === 'Physical') return 'map-pin';
	if (category === 'Verbal') return 'mic';
	return 'file';
}

/**
 * Returns the CSS variable name for a confidence/quality score 0.0–1.0.
 * Uses the --color-quality-* tokens defined in theme.css.
 */
export function getQualityBadgeColor(confidence: number | null): string {
	const conf = confidence ?? 0;
	if (conf >= 0.7) return 'var(--color-quality-high)';
	if (conf >= 0.5) return 'var(--color-quality-medium)';
	return 'var(--color-quality-low)';
}

/**
 * Human-readable file size string (B / KB / MB).
 */
export function formatFileSize(bytes: number): string {
	if (bytes < 1024) return `${bytes} B`;
	if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
	return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * Format an ISO date string for display in timeline / results.
 */
export function formatDate(dateStr: string): string {
	const date = new Date(dateStr);
	return date.toLocaleDateString('en-US', {
		month: 'short',
		day: 'numeric',
		year: 'numeric'
	});
}
