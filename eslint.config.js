import svelte from 'eslint-plugin-svelte';
import ts from '@typescript-eslint/eslint-plugin';
import tsParser from '@typescript-eslint/parser';
import svelteParser from 'svelte-eslint-parser';

const svelteConfig = svelte.configs['recommended'];

const svelteTsConfig = {
	files: ['**/*.svelte'],
	languageOptions: {
		parser: svelteParser,
		parserOptions: {
			parser: tsParser
		}
	},
	plugins: {
		'@typescript-eslint': ts
	},
	rules: {
		'@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
		'@typescript-eslint/no-explicit-any': 'warn'
	}
};

const tsConfig = {
	files: ['**/*.ts', '**/*.tsx'],
	languageOptions: {
		parser: tsParser
	},
	plugins: {
		'@typescript-eslint': ts
	},
	rules: {
		'@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
		'@typescript-eslint/no-explicit-any': 'warn'
	}
};

export default [
	...svelteConfig,
	{
		ignores: [
			'node_modules/**',
			'src-tauri/target/**',
			'.svelte-kit/**',
			'build/**',
			// Prevent ESLint from scanning the literal '$HOME' directory that mise
			// creates in the project root when env vars expand to a literal path segment.
			'\$HOME/**',
			'**/registry/src/**',
			'**/toolchains/**'
		]
	},
	svelteTsConfig,
	tsConfig,
	{
		rules: {
			'svelte/no-at-html-tags': 'error',
			'svelte/prefer-svelte-reactivity': 'off',
			'svelte/no-navigation-without-resolve': 'off',
			'svelte/require-each-key': 'off'
		}
	}
];
