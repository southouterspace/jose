import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import topLevelAwait from "vite-plugin-top-level-await";
import wasm from "vite-plugin-wasm";

/**
 * Vite config for `apps/web` (ADR 0005). React owns the chrome; the wasm engine loads via
 * `vite-plugin-wasm` (+ top-level-await for its async init), and the engine worker continues to
 * spawn through `new Worker(new URL("./engine-worker.ts", import.meta.url), { type: "module" })`,
 * which Vite supports natively. Scope is exactly this one package — Bun stays the workspace PM.
 */
export default defineConfig({
  plugins: [react(), wasm(), topLevelAwait()],
  server: {
    port: 5173,
  },
  worker: {
    format: "es",
    plugins: () => [wasm(), topLevelAwait()],
  },
});
