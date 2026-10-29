import js from '@eslint/js';
import prettier from 'eslint-config-prettier';
import security from 'eslint-plugin-security';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';
import ts from 'typescript-eslint';
import svelteConfig from './svelte.config.js';

export default ts.config(
  { ignores: ['dist/', 'src-tauri/', 'node_modules/', 'e2e/.work/', 'e2e/node_modules/'] },
  js.configs.recommended,
  ...ts.configs.strictTypeChecked,
  ...svelte.configs.recommended,
  security.configs.recommended,
  prettier,
  ...svelte.configs.prettier,
  {
    languageOptions: {
      globals: globals.browser,
      parserOptions: {
        projectService: true,
        extraFileExtensions: ['.svelte'],
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
  {
    files: ['**/*.svelte', '**/*.svelte.ts'],
    languageOptions: {
      parserOptions: { parser: ts.parser, svelteConfig },
    },
    rules: {
      // `let { x = $bindable() } = $props()` is required Svelte syntax, not a redundant default.
      '@typescript-eslint/no-useless-default-assignment': 'off',
    },
  },
  {
    // Build scripts open paths the developer passes on their own command line.
    files: ['scripts/**/*.ts'],
    rules: { 'security/detect-non-literal-fs-filename': 'off' },
  },
  {
    // The e2e suites are Node scripts that drive a test build: they start processes, read and
    // write files under the paths they create, and index objects and build patterns from their own
    // constants, never from outside input.
    files: ['e2e/**/*.mjs'],
    ...ts.configs.disableTypeChecked,
    languageOptions: {
      ...ts.configs.disableTypeChecked.languageOptions,
      globals: globals.node,
    },
    rules: {
      ...ts.configs.disableTypeChecked.rules,
      'security/detect-child-process': 'off',
      'security/detect-non-literal-fs-filename': 'off',
      'security/detect-non-literal-regexp': 'off',
      'security/detect-object-injection': 'off',
      // `const { id, ...content } = saved` drops fields on purpose.
      '@typescript-eslint/no-unused-vars': ['error', { ignoreRestSiblings: true }],
    },
  },
  {
    files: ['*.config.js', '*.config.ts'],
    ...ts.configs.disableTypeChecked,
    languageOptions: {
      ...ts.configs.disableTypeChecked.languageOptions,
      globals: globals.node,
    },
  },
);
