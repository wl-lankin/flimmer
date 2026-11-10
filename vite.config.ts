import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1460, strictPort: true, hmr: { protocol: 'ws', host: 'localhost', port: 1461 }, watch: { ignored: ['**/src-tauri/**'] } },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: { outDir: 'dist', emptyOutDir: true, target: 'chrome120', modulePreload: { polyfill: false } },
  resolve: process.env.VITEST ? { conditions: ['browser'] } : undefined,
  test: {
    environment: 'jsdom',
    include: ['src/**/*.spec.ts'],
  },
});
