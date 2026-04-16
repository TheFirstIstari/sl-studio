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
		ignores: ['node_modules', 'src-tauri/target', '.svelte-kit', 'build', '~', 'src-tauri/~']
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
