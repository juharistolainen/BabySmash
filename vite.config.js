import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";
// @ts-expect-error type error without @types/node package
import { readFileSync } from "node:fs";
const host = process.env.TAURI_DEV_HOST;

/** @param {string} dir */
const pkgVersion = (dir) =>
  JSON.parse(readFileSync(new URL(`${dir}/package.json`, import.meta.url), "utf8")).version;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [sveltekit()],

  // Installed (not semver-range) versions for the About dialog; see src/lib/buildInfo.ts.
  define: {
    __FRONTEND_VERSIONS__: JSON.stringify({
      app: pkgVersion("."),
      svelte: pkgVersion("./node_modules/svelte"),
      kit: pkgVersion("./node_modules/@sveltejs/kit"),
      vite: pkgVersion("./node_modules/vite"),
      tauriApi: pkgVersion("./node_modules/@tauri-apps/api"),
    }),
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
