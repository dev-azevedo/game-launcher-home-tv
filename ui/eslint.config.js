import js from '@eslint/js';
import globals from 'globals';
import tseslint from 'typescript-eslint';
import react from 'eslint-plugin-react';
import reactHooks from 'eslint-plugin-react-hooks';

const htmlSinks = ['innerHTML', 'outerHTML', 'insertAdjacentHTML'];

export default tseslint.config(
  { ignores: ['dist', 'node_modules', 'coverage'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ['**/*.{ts,tsx}'],
    languageOptions: { globals: { ...globals.browser } },
    plugins: { react, 'react-hooks': reactHooks },
    settings: { react: { version: 'detect' } },
    rules: {
      ...reactHooks.configs.recommended.rules,
      // SEC-07: dado externo so como texto.
      'react/no-danger': 'error',
      'react/no-danger-with-children': 'error',
      'no-restricted-properties': [
        'error',
        ...htmlSinks.map((property) => ({
          property,
          message: 'SEC-07: proibido inserir HTML. Renderize dado como texto por JSX.',
        })),
        { object: 'document', property: 'write', message: 'SEC-07: proibido document.write.' },
        { object: 'document', property: 'writeln', message: 'SEC-07: proibido document.writeln.' },
      ],
      'no-restricted-syntax': [
        'error',
        {
          selector: "AssignmentExpression[left.property.name=/^(innerHTML|outerHTML)$/]",
          message: 'SEC-07: proibido atribuir innerHTML/outerHTML.',
        },
        {
          selector: "Property[key.name=/^(innerHTML|outerHTML)$/]",
          message: 'SEC-07: proibido innerHTML/outerHTML.',
        },
      ],
      'no-eval': 'error',
      'no-implied-eval': 'error',
      'no-new-func': 'error',
      // INV-02 / SEC-03: IPC so em shared/ipc.
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            { group: ['@tauri-apps/*'], message: 'Importe @tauri-apps/* apenas em src/shared/ipc.' },
          ],
        },
      ],
    },
  },
  {
    files: ['src/shared/ipc/**/*.{ts,tsx}'],
    rules: { 'no-restricted-imports': 'off' },
  },
  {
    files: ['**/*.test.{ts,tsx}', 'src/test/**'],
    languageOptions: { globals: { ...globals.browser, ...globals.node } },
  },
  { files: ['*.config.{js,ts}'], languageOptions: { globals: globals.node } },
);
