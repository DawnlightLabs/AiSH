import { resolve } from 'node:path';
import { defineConfig } from 'vite';

export default defineConfig({
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'index.html'),
        downloads: resolve(__dirname, 'downloads/index.html')
      }
    }
  },
  plugins: [
    {
      name: 'aish-html-shell',
      transformIndexHtml() {
        return '<!doctype html><html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><meta name="theme-color" content="#090807"><meta name="description" content="AiSH is an AI-native shell and desktop terminal from Dawnlight Labs."><title>AiSH by Dawnlight Labs</title></head><body><div id="root"></div><script type="module" src="/src/main.jsx"></script></body></html>';
      }
    }
  ]
});
