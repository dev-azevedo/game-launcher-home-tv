import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('.', import.meta.url));
// Fonte unica da marca (arch 3.5): importada por alias, sem copia.
const brand = fileURLToPath(new URL('../docs/brand/sofa', import.meta.url));

export default defineConfig({
  base: './',
  plugins: [react()],
  resolve: { alias: { '@brand': brand, '@': `${root}src` } },
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    fs: { allow: [root, brand] },
  },
  build: {
    target: 'es2022',
    // CSP (SEC-01): font-src/img-src 'self' admitem data: so em img. Nada de asset inline.
    assetsInlineLimit: 0,
    cssCodeSplit: true,
    sourcemap: false,
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    css: false,
    testTimeout: 30000,
    include: ['src/**/*.test.{ts,tsx}'],
  },
});
