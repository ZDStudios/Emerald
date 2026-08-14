import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';
import { resolve } from 'node:path';

export default defineConfig({
  plugins: [solid()],
  // Tauri serves the chrome from a fixed port in dev and from disk in release.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  build: {
    // WebKitGTK 2.44+ / WKWebView / WebView2 all handle this comfortably, and
    // it keeps the chrome bundle small — which matters, because this bundle is
    // parsed on every cold start and shows up directly in the startup number.
    target: 'es2022',
    minify: 'esbuild',
    sourcemap: false,
    rollupOptions: {
      input: {
        // The browser chrome.
        main: resolve(__dirname, 'index.html'),
        // The new-tab page, rendered in a *page* webview like any site, so it
        // gets the same discard policy and the same content script as the web.
        newtab: resolve(__dirname, 'newtab.html'),
      },
    },
  },
});
