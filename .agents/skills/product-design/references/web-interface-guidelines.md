# Web interface guidelines

Adapted from Vercel's Web Interface Guidelines. **This file is the single source** for the
`/web-interface-guidelines` reviewer command (run in-session via `/ci-local`; ADR 0011) — the
command reads these rules, it doesn't carry its own copy (DRY, the same one-source discipline as the
MODEL). Load it for the keyboard/focus/forms/animation/touch/performance dimension of a review or
implementation.

## Enforcement tiers

Each rule is tagged with where it's enforced, so we don't ask an agent to re-check what a linter
already gates, and don't pretend an unbuilt check exists:

- **`[biome]`** — already a **required** gate via the Ultracite/Biome preset (`bun run lint`). Don't
  re-report; it can't merge if violated.
- **`[agent]`** — needs judgment/context; the `/web-interface-guidelines` reviewer's real job.
- **`[gap]`** — a reliable deterministic check that **is not built yet** (candidate for a custom
  Biome/GritQL rule). Tracked in `coverage-gaps.md`; the agent flags it meanwhile.
- **`[n/a]`** — no surface in Jose today (no forms, `<img>`, SSR/hydration, i18n, virtualized
  lists, Tailwind). Listed for completeness; **don't run it as a finding** — it's noise until the
  surface exists. When it does, flip the tier.

> **Scope first.** Jose's web surface is two `<canvas>`/SVG **viewports** + a **toolbar** +
> a **status bar**, dark-only, client-rendered (Vite SPA). The applicable set is small (the
> `[agent]`/`[gap]` rows below). Everything `[n/a]` is a future `coverage-gaps.md` entry, not a review
> comment. Re-read `apps/web/CONTEXT.md` for the canonical surface nouns before writing a finding.

---

## Accessibility

- `[agent]` Icon-only buttons need `aria-label` (Biome won't infer intent; the tool buttons are
  text today — guard if an icon-only control is added). Tie to `rule/control-accessible-name`.
- `[biome]` Form controls need `<label>`/`aria-label` — `noLabelWithoutControl`.
- `[biome]` Interactive elements need keyboard handlers — `useKeyWithClickEvents`.
- `[biome]` `<button>` for actions, `<a>`/`<Link>` for nav, never `<div onClick>` —
  `noStaticElementInteractions`, `useButtonType`.
- `[biome]` Images need `alt` (or `alt=""`) — `useAltText`. (`[n/a]` surface today.)
- `[agent]` Decorative icons/SVGs need `aria-hidden="true"`.
- `[agent]` Async updates (status changes, future toasts/validation) need `aria-live="polite"` — the
  **status bar** is a live region candidate; it has no `aria-live` today (real gap).
- `[biome]` Prefer semantic HTML before ARIA — `useSemanticElements`.
- `[agent]` Headings hierarchical `<h1>`–`<h6>`; skip-link to main. (Minimal surface; the shell has
  no headings yet.)
- `[biome]` `autoFocus` used sparingly — `noAutofocus` flags it.
- `[gap]` `scroll-margin-top` on heading anchors (CSS; no anchored headings yet).

## Focus states

- `[agent]` Interactive elements need a **visible focus** state. The tool buttons rely on the UA
  default outline today — verify it survives any restyle (real risk if a custom button style lands).
- `[gap]` Never `outline: none`/`outline-none` without a focus replacement — reliable CSS check,
  not built.
- `[agent]` Prefer `:focus-visible` over `:focus`.
- `[agent]` `:focus-within` for compound controls.

## Animation

- `[agent]` Honor `prefers-reduced-motion`. The 3D view's OrbitControls damping + the camera
  re-frame are motion; a reduced-motion variant isn't handled (gap to flag).
- `[agent]` Animate `transform`/`opacity` only.
- `[gap]` Never `transition: all` — list properties. Reliable CSS check, not built. (`app.css` has
  no transitions today — near-zero surface.)
- `[agent]` Correct `transform-origin`; SVG transforms on a `<g>` with `transform-box: fill-box`.
- `[agent]` Animations interruptible (orbit/drag already are — `three-view.tsx`).

## Typography

- `[gap]` `…` not `...`; loading/progress strings end with `…` (status text already does —
  "Loading engine…"). A literal-`...` check is a reliable candidate.
- `[agent]` Curly quotes `“ ”` not straight `"`.
- `[agent]` Non-breaking spaces in units/shortcuts/brand (`8&nbsp;ft`).
- `[agent]` `font-variant-numeric: tabular-nums` for numeric readouts — the status bar's height
  value is a candidate.
- `[agent]` `text-wrap: balance`/`pretty` on headings.

## Content handling

- `[agent]` Text containers handle long content (`truncate`/`line-clamp`/`break-words`); flex
  children need `min-w-0`. The status bar is a single line — a very long footprint readout could
  overflow (ties to `resilience.md`).
- `[agent]` Handle empty states — don't render broken UI for empty arrays (the empty/ready state is
  designed; keep it so).

## Navigation & state

- `[agent]` URL reflects state (active tool, future selection) — Jose stores tool state in React,
  **not** the URL (`ADR 0005` allows TanStack/URL "selectively"); deep-linking is a deliberate
  non-goal for the MVP, so flag only if a *new* shareable state lands. Note in `coverage-gaps.md`.
- `[biome]` Links use `<a>`/`<Link>` for navigation — `noStaticElementInteractions`. (`[n/a]` — no
  navigation surface yet.)
- `[agent]` Destructive actions need confirmation or an undo window — ties to
  `rule/destructive-names-action`; **no undo exists** (`coverage-gaps.md`), so the bar is high.

## Touch & interaction

- `[agent]` `touch-action`: **Jose intentionally uses `touch-action: none`** on both drawing
  surfaces (pointer gestures own the canvas) — this is a deliberate **exception** to the
  "`manipulation`" guideline; do not flag it. Apply `manipulation` only to non-canvas controls.
- `[gap]` `-webkit-tap-highlight-color` set intentionally.
- `[agent]` `overscroll-behavior: contain` in any future modal/drawer/sheet (none today).
- `[agent]` During a drag: disable text selection / `inert` dragged elements (orbit freeze already
  done; selection-disable is a finish item).

## Dark mode & theming

- `[done]` `color-scheme: dark` on the root — `app.css` sets it. Keep it.
- `[gap]` `<meta name="theme-color">` matching the page background — **`apps/web/index.html` lacks
  it** (a real, concrete finding; trivial fix).
- `[n/a]` Native `<select>` dark-mode styling — no `<select>` in the app.

## Performance

- `[agent]` No layout reads (`getBoundingClientRect`/`offsetHeight`) in render — the imperative
  Three.js renderer reads in effects/handlers, not render (`three-view.tsx`); keep it that way.
- `[n/a]` Virtualize large lists, preconnect/preload fonts, batch DOM reads/writes — no lists, no
  custom fonts (`system-ui`), no asset CDN today.

## Content & copy

- `[agent]` Routes to **`copy.md`** — active voice, specific labels, errors include a next step,
  second person, numerals for counts. The canonical-noun rule (`rule/canonical-noun`) takes
  precedence over generic copy advice.

## Forms · Images · Locale/i18n · Hydration

- `[n/a]` **Entire sections.** Jose has no forms, no `<img>`, no i18n, and no SSR/hydration (Vite
  client SPA). Do not run these as findings. Each becomes a `coverage-gaps.md` entry if/when the
  surface appears (e.g. a settings form, a thumbnail, localized dimensions).

---

## Anti-patterns (flag when present)

`[biome]` `<div>`/`<span>` with click handlers · inline `onClick` nav without `<a>` · inputs without
labels · `autoFocus` without justification. **`[gap]`** `user-scalable=no`/`maximum-scale=1` ·
`onPaste`+`preventDefault` · `transition: all` · `outline: none` without `:focus-visible` · images
without dimensions · literal `...`. **`[n/a]`** large `.map()` without virtualization · hardcoded
date/number formats (no dates/i18n yet). The viewport meta in `index.html` is currently clean
(`width=device-width, initial-scale=1`) — keep it.

## How a finding is written

Match the reviewer command's format: group by file, `file:line`, terse, issue + location, fix only
if non-obvious. **Skip `[biome]` and `[n/a]` rows** — report `[agent]` and `[gap]` only. A `[gap]`
finding should name the rule so it can later graduate to a deterministic check.
