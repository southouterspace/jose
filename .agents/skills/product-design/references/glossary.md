# Glossary

**This file does not define the terms — it routes to their owner.** The ubiquitous language of the
drawing UX is owned by [`apps/web/CONTEXT.md`](../../../apps/web/CONTEXT.md), pinned in a
domain-modeling session per the repo's `CONTEXT.md` convention (`CONTEXT-MAP.md`). When the two
disagree, **`CONTEXT.md` is right and this file is stale.** Do not fork a definition here.

Load this to get the canonical noun fast, then read the owner entry for the full meaning and the
"avoid" list before writing copy or naming a new element.

## Quick index → owner

| Term | One-line | Avoid | Owner entry |
| ---- | -------- | ----- | ----------- |
| **App shell** | The toolbar + status + viewport layout; chrome, no geometry. | layout, frame, chrome | `CONTEXT.md` "App shell" |
| **Viewport** | One rendering surface for a fixed projection; here, both are also input surfaces. | pane, canvas, window | "Viewport" |
| **Plan view** | Top-down orthographic 2D; draws/edits the footprint. | 2D view, top view, floorplan | "Plan view" |
| **3D view** | Perspective, orbitable; shows the mass, interactive for push/pull. | model view, scene | "3D view" |
| **Elevation view** | A viewport looking at one face; deferred in the drawing UX. | section, side view | "Elevation view" |
| **Footprint** | The closed 2D profile drawn in plan; the interior face of later framing. | outline, polygon, sketch, perimeter | "Footprint" |
| **Push/pull** | The 3D gesture extruding the top cap to set height; vertical, top-cap only in the MVP. | extrude, drag, pull-up | "Push/pull" |
| **Mass** | The 3D solid produced by extruding a footprint. | block, box, model, solid | "Mass" |
| **Space** | The enclosed region the user draws (footprint + mass); the canonical input. | room, zone, area | "Space" |
| **Outward framing** | Derived framing grows outward; the footprint is the interior face. | wall offset, inset framing | "Outward framing" |
| **World space** | The shared coordinate system, in ticks (1/32in). | global/scene/model space | "World space" |
| **Mirror** | A read-only, zero-copy view over the engine's SoA buffer; the only way the client reads geometry. | model, store, cache, snapshot | "Mirror" |
| **Tool** | A picking state machine turning gestures into a command; the active tool receives picks. | mode, gesture handler | "Tool" |

## Why this routing exists

Duplicating the glossary would create a second source of truth — exactly what the architecture
forbids. The skill's job is to *route* an agent to the owner at the moment it's naming something, not
to copy the owner. If you find yourself wanting to add a definition here, add it to `CONTEXT.md`
instead and leave a one-line index row here.
