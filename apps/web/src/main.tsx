/**
 * React entry — the "hands & eyes" app-shell mount point (ADR 0005). It only wires the root; the
 * shell chrome lives in `App`. No geometry or engine wiring yet — those land in later phases on top
 * of the worker/protocol/wasm plumbing kept alongside this file.
 */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app";
import "./app.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("web: #root mount point missing from index.html");
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>
);
