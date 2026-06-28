/**
 * Tiny dev static server (Bun runtime). Bundles on start via the `dev` script's `bun build`, then
 * serves `index.html`, the bundled `dist/`, and the wasm artifact under `src/wasm/pkg/`.
 *
 * Not part of the typechecked app surface — it is a Bun script, run with `bun ./serve.ts`.
 */
const ROOT = new URL(".", import.meta.url).pathname;
const PORT = Number(process.env.PORT ?? 5173);

Bun.serve({
  port: PORT,
  async fetch(request) {
    const url = new URL(request.url);
    const path = url.pathname === "/" ? "/index.html" : url.pathname;
    const file = Bun.file(ROOT + path.replace(/^\//, ""));
    if (await file.exists()) return new Response(file);
    return new Response("not found", { status: 404 });
  },
});

console.log(`web: serving http://localhost:${PORT}`);
