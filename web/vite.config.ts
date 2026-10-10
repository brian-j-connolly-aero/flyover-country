import { defineConfig } from 'vite';
import { resolve } from 'node:path';

export default defineConfig({
  base: './',
  build: {
    target: 'es2022',
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        theater: resolve(__dirname, 'theater.html'),
      },
    },
  },
  // Saves live in the browser per address, so always use the same one.
  // `npm run lan` adds --host for tablets on the same network.
  server: { port: 5173, strictPort: true },
  preview: { port: 4173, strictPort: true },
});
