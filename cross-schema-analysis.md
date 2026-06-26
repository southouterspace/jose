# Cross-Schema Architecture Analysis
**Parametric Residential Framing Tool — constrained BIM engine**
Scope: four domain schemas (materials, solver, drawing-workspace, design-standard) + two context artifacts (architecture draft, reference library). Analysis and registry only — no schema edits. Conventions and visual language left intact for later.

Companion machine contract: [`type-registry.json`](type-registry.json) — 74 types indexed, 11 collisions, 13 dangling refs.

---

## 0. What was parsed

| Artifact | Schema | Types | Notes |
|---|---|---|---|
| `lumber-schema_1.html` | `parametric-lumber@0.2` | 14 | 5 layers. The tick-invariant reference implementation. |
| `solver-schema_1.html` | `framing-solver@0.1.0` | 20 (+2 re-carded) | 6 layers. Re-cards `Piece` and `SupplierSku` as cross-refs. |
| `drawing-workspace-schema_1.html` | `drawing-workspace@0.1.0` | **23** | 6 groups. ⚠ Prior session recalled "22"; the MODEL actually has 23 (A5·B4·C6·D2·E4·F2). |
| `design-standard-schema.html` | `design-standard@0.1` | 13 | Strategy pattern, 3 tiers. Proposes the `LumberStock→Stock` rename. |
| `architecture.html` | `system-architecture@0.1` | — | Prose + diagram. **No machine-readable MODEL object.** |
| `reference-library.html` | `reference-library` | 23 books / 16 subjects | Has a `#library-data` MODEL; `CitationKey` target. |

The four domain MODELs are internally consistent and share the stereotype color coding exactly (entity `#E2682C` / value `#5B9BD5` / reference `#6FBF73` / service `#C792EA` / render `#E0B341`). The findings below are about how they fit **together**.

---

## Severity 1 — Architectural (resolve before more schemas land)

### S1-A. The wood design-value model is built three times
`MechanicalProperties` (materials, value) · `NDSDesignValues` (solver, reference) · `NDS_Wood.designValues` (design-standard, leaf) all hold the same intrinsic set — `Fb, Ft, Fc, FcPerp, Fv, E, Emin`. The solver card even admits it: *"Lives near MechanicalProperties in the materials schema."*

The flyweight mechanics are correct in each place (looked up by key, never copied per member) — but the **type itself is not single-sourced**. Three schemas will drift the moment one species table is corrected.

> **Direction:** one wood design-value flyweight, canonical in materials (`MechanicalProperties`). `solver.NDSDesignValues` and `design-standard.NDS_Wood` *reference* it; they do not restate its fields. `NDS_Wood` becomes the leaf that points at the materials flyweight behind the seam.

### S1-B. The adjustment-factor stack is built 3–4 times — and split wrong
`materials.MechanicalProperties.adjustmentFactors` (a `map`) · `solver.AdjustmentFactors` (value) · `design-standard.modificationFactors` · `design-standard.NDS_Wood.factors`.

Worse than duplication, this is the one real **intrinsic/contextual (flyweight) violation** in the set. Materials folds the *whole* factor stack into the shared `MechanicalProperties` flyweight. But most NDS factors are **contextual**, not intrinsic:

- `CD` (load duration) — depends on the load case, not the stick.
- `Cr` (repetitive member) — depends on spacing/use (studs 16" OC).
- `Cp` / `CL` (column / beam stability) — depend on bracing and end conditions, i.e. *placement*.

Only `CF` (size) is genuinely intrinsic to the section. The solver gets this right — *"Role selects the stack,"* derived from placement context. Materials does not.

> **Direction:** base reference values stay intrinsic (flyweight); the *applied* factor stack moves to per-`MemberPlacement` (contextual). Remove the factors map from `materials.MechanicalProperties`.

### S1-C. The structural-check stage has two competing architectures
The same pipeline stage is modeled twice and the two have not been reconciled:

- **solver** — flat, NDS-specific: `NDSDesignValues + AdjustmentFactors + LoadCase + CapacityCheck`.
- **design-standard** — Strategy seam: `DesignStandard⟨interface⟩ + SizingArbiter + LimitStateCheck + BeamStatics + LoadContext`, material-blind, with `NDS_Wood / AISI_CFS / ACI_Concrete` leaves.

design-standard is clearly the evolution ("Already built — this is what the sizing-engine layer implements") but solver still ships the older flat model. Anyone reading solver implements the wrong thing.

> **Direction:** design-standard's seam **subsumes** solver's structural layer. `solver.CapacityCheck` → `design-standard.SizingArbiter`; `solver.{NDSDesignValues, AdjustmentFactors}` → the `NDS_Wood` leaf behind the seam. Mark solver's structural layer as superseded.

### S1-D. `Plane` — known collision #1 — is defined in the wrong place
`Plane` is defined in **drawing** (group B, value object) but consumed by **materials** (`CutOperation.plane`) and **solver** (`Face.plane`). A cut plane, a face plane, and a sketch plane are the same math object: `origin + basisAxes + normal`. Materials already flags it: *"Shared Plane primitive … see cross-schema unify."* A domain schema (materials) should not have to reach *up* into the drawing workspace for a geometry primitive.

> **Direction:** extract `Plane` to a shared **geometry-kernel** layer (the lowest shared module — where `truck`/BREP lives per the architecture draft). drawing keeps `SketchPlane` (which *wraps* a `Plane` + U/V); the bare `Plane` moves down. All three schemas reference the one home. See §2.

---

## Severity 2 — Significant (structural debt, not yet on fire)

### S2-A. The render layer lives inside a domain schema — known collision #2
`RenderMesh` / `MeshInstance` / `MaterialRef` sit in **materials** layer 05, directly contradicting that schema's own stated rule: *"Presentation lives outside the domain. The domain never imports render code."* The render adapter *is* the domain schema's last layer.

Render concepts also scatter: drawing's `Camera`/`Viewport` are **view/navigation** (not mesh render — different concern), and architecture defines the real presentation home as the JS-side **render mirror** (typed-array views over Rust buffers, read-only).

> **Direction:** extract `RenderMesh`/`MeshInstance`/`MaterialRef` into a dedicated render-adapter artifact — which is the **render mirror** in the system-architecture spec (§5b). The flyweight split itself is correct (intrinsic geometry in `RenderMesh`, per-instance `Transform` in `MeshInstance`); only its *home* is wrong.

### S2-B. Geometry primitives are scattered across domain schemas
`Transform` and `BoundingBox` live in **materials**; `CoordinateFrame` lives in **drawing**. All three are pure geometry with no material or domain content, and all three are consumed broadly (`Transform` by materials + solver + design-standard). Same class of error as `Plane`.

> **Direction:** move `Transform`, `BoundingBox`, `CoordinateFrame` — together with `Plane`, `Path2D`, `Segment` — into the shared geometry-kernel layer. See §2.

### S2-C. `LumberStock/Stock` and `LumberSpec/StockSpec` — same entity, two names
design-standard proposes renaming `LumberStock/LumberSpec → Stock/StockSpec` + a `material` discriminator, and asserts `Piece, Cut, Transform, BoundingBox, SupplierSku, PriceQuote` carry over unchanged. Until applied, the same entity has two names across schemas and any reader has to know the alias.

> **Direction:** either apply the rename everywhere, or record `Stock ≡ LumberStock` as a documented alias in the registry so cross-refs resolve. (Registry currently records it as `renamePending`.)

### S2-D. Base-unit violations: area typed as `ticks²`
The canonical rule — linear geometry as int ticks, area/volume/weight/price as **real** — is followed beautifully in materials (it's the reference implementation). Solver breaks it for area:

- `solver.Face.area : ticks²`
- `solver.LoadCase.tributaryArea : ticks²`

Materials is explicit that area is *"derived (not stored as tick²)"*, and design-standard's `SectionProperties` correctly uses real `in²/in³/in⁴`. Solver's `ticks²` is the outlier and will silently misfeed any section/weight calc that assumes real area. Full per-file audit in §3.

---

## Severity 3 — Hygiene (cheap fixes, prevent future drift)

### S3-A. Dangling type references (defined nowhere)
Used as field types but never given a card or primitive definition:

| Ref | Used in | Fix |
|---|---|---|
| `MeshRef` | `materials.Piece.geometry` | should ref `RenderMesh.geometryId` |
| `ProvenanceLink` | `materials.Piece.provenance` | define / unify with shared provenance vocab |
| `PriceTier` | `materials.PriceQuote.bulkTiers` | promote inline `{minQty,pricePer}` to a named VO |
| `MaterialRef` | `materials.RenderMesh.material` | define in render-adapter artifact |
| `Path2D`, `Segment` | solver geometry, `Wall.baseline` | geometry-kernel primitives (tick coords) |
| `DesignValues`, `Factor`, `Connection`, `PrescriptiveTable`, `SizingQuery`, `Result` | design-standard | define the Strategy I/O types |

### S3-B. Primitive vocabulary drift
`UUID` vs `uuid`; `ticks` vs `tick`; `vec3` vs `vec3<int>`. Materials distinguishes tick-positions (`vec3<int>`) from directions (`vec3`); solver and drawing use bare `vec3` for both. A one-page shared primitive glossary (`tick`, `TickVec3`, `UnitVec3`, `quat`) fixes this and the base-unit typing at once.

### S3-C. Price representation drift
`materials.PriceQuote` is float USD; `solver.StockOption.price` is integer `cents` and doesn't reference `PriceQuote` at all. Pick one; solver should reference the value object.

### S3-D. "Provenance" means three things
`materials.ProvenanceLink`, `drawing.SnapProvenance`, `design-standard.Provenance` — three distinct, legitimately-different provenance concepts with no shared naming. Keep them distinct but align the vocabulary.

### S3-E. Stereotype vocabulary grew quietly
design-standard adds a 6th «interface» stereotype (`DesignStandard`) that reuses the render color slot; «render» is present in materials + drawing but absent in solver. Either adopt «interface» formally in the shared legend or recolor, and note render's absence in solver is intentional.

### S3-F. design-standard declares no base unit
Its `meta` has no `units`/base-unit field; it relies entirely on carried-over typed primitives. One line restating the tick invariant keeps it self-describing like the others.

---

## §2. Shared-primitive unification — the recommended home

Every misplaced primitive points at the same missing module. The architecture draft already names it: the **geometry kernel** (`truck`/BREP) + the **canonical store** (SoA tick arrays), both Rust-resident, the floor everything stands on.

**Proposed canonical home — `geometry-kernel` (shared, lowest layer):**

| Primitive | Current home | Consumers that should point at the new home |
|---|---|---|
| `tick` (int, 1/32") | nowhere (implicit) | all four schemas |
| `TickVec3` (`vec3<int>`) | materials only | solver, drawing world-points |
| `UnitVec3` (`vec3` dir) | implicit | normals/axes everywhere |
| `quat` | implicit | `Transform`, `NavigationState` |
| **`Plane`** | drawing | `materials.CutOperation.plane`, `solver.Face.plane`, `drawing.SketchPlane.plane` |
| **`Transform`** | materials | materials, `solver.MemberPlacement`, design-standard |
| **`BoundingBox`** | materials | `materials.RenderMesh`, design-standard |
| **`CoordinateFrame`** | drawing | drawing inference / world basis |
| `Path2D`, `Segment` | undefined | `solver.Volume.profile`, `Face.boundary`, `Wall.baseline` |
| `Volume`, `Face`, `PushPullOp` | solver layer 1 | drawing crossRefs already call these "geometry-kernel@0.x" |

That last row is the tell: drawing's `crossRefs` already reference `geometry-kernel@0.x` for `Volume`/`Face`/`PushPullOp`, even though they're currently *defined* inside solver's layer 1. The kernel wants to be its own artifact. Promoting it gives `Plane`/`Transform`/`Path2D`/`Segment` a natural home and resolves S1-D, S2-B, and the `Volume`/`Face` ambiguous-home flag in one move.

**Two shared primitives to keep distinct, not merged:**
- `Plane` (drawing's own note: *"a noun for a thing in the world"*) vs an **axis** you snap *along* — drawing is right to keep these as different words; only `Plane` moves down.
- `SketchPlane` stays in drawing — it's a *wrapper* (Plane + local U/V + source face), genuinely workspace-specific.

**The render "primitive":** `RenderMesh`/`MeshInstance` are *not* geometry-kernel material — they're the presentation mirror. Their home is the render-adapter / system-architecture spec (§5b), not the kernel and not materials.

---

## §3. Base-unit invariant audit (per file)

Rule: **linear geometry → int ticks @ 1/32"; area/volume/weight/price → real.** `1in = 32 ticks`.

### materials — ✅ COMPLIANT (reference implementation)
Every linear field is int ticks (`length`, `actualWidth/Thickness`, `Transform.origin` `vec3<int>`, `BoundingBox.min/max`, `ConnectionPoint.position`, `CutOperation.position/kerf`, `SupplierSku.stockLength`). Area/volume/weight/price all float. Normals are plain `vec3`. `crossSectionArea` explicitly *"not stored as tick²."* This is the model the others should conform to. **0 violations.**

### solver — ⚠ VIOLATIONS
| Field | Typed | Should be | Sev |
|---|---|---|---|
| `Face.area` | `ticks²` | float `in²` (derived) | S2 |
| `LoadCase.tributaryArea` | `ticks²` | float `in²` (derived) | S2 |
| `StockOption.price` / `SupplierSku.price` | `cents` (int) | reference `PriceQuote` (float USD) — drift | S3 |
| positional `vec3` / `Segment` | bare `vec3` | tick-typed where world position | S3 |

Linear length fields (`Wall.*`, `Opening.*`, `MemberPlacement.length`, `Demand/StockOption.length`, etc.) are correctly ticks. The `StockOption.length` note examples (`92.625, 96, 120…`) read as **inches**, not ticks — misleading annotation on a correctly-typed field.

### drawing — ⚠ SOFT VIOLATIONS
World-space committed points are typed bare `vec3` and only *prose*-promised to ticks: `SnapTarget.point : "vec3 (resolved to tick)"`, `Plane.origin`, `ReferenceAnchor.point`, `DrawOp.payload : "world ticks"`. Type them `vec3<int>`. **Legitimately float** (no fix): `Camera.eye/target/up` (view math), `region`/`tolerancePx` (screen space) — the schema's own line *"Pixel-defined, not world-defined; result resolves to world ticks"* is the correct boundary.

### design-standard — ✅ NO LINEAR VIOLATIONS
`SectionProperties` uses real `in²/in³/in⁴` (the counter-example to solver's `ticks²`). Analysis layer carries little raw geometry. Only nit: `meta` declares no base unit (S3-F).

---

## §4. Pipeline coherence

Canonical: `drawing → geometry kernel → framing model → placement → structural check → cut optimization → supplier → drawings export`.

| Artifact | Pipeline shown | Verdict |
|---|---|---|
| materials | none (upstream data) | n/a |
| **solver** | geometry → building → placement → structural → cut → supplier | covers the middle; `Building Model` vs `framing model`; no export handoff; loads hidden inside structural |
| **drawing** | viewport+camera → cursor+inference → tool → SketchPlane lift → kernel op → framing solver | coherent — this is the *expansion* of the `drawing` stage; correctly defers `drawings` **output** to a separate downstream schema |
| **design-standard** | drawing → geometry → framing model → placement → **structural sizing** → cut optimization → supplier → drawings export | **exact 8-stage canonical match** (only `structural sizing` vs `structural check`) |
| **architecture** | Geometry edit (JS) → Building model → Placement → Structural check → Cut → Supplier → Drawings export | 7 stages, ownership-colored; see drift |

**Drift found:**
1. **Label drift.** `building model` (solver, arch) vs `framing model` (design-standard, canonical); `structural check` vs `structural sizing`; `supplier constraint` vs `supplier`. Pick canonical labels and apply across strips.
2. **Geometry-kernel folded into JS edit.** architecture's single `Geometry edit / JS` stage conflates JS edit-capture with the Rust BREP kernel — yet its own body puts the kernel in Rust. The strip blurs the very boundary the document argues for.
3. **Loads is never a pipeline stage — anywhere.** It's a sub-component in solver (`LoadCase` inside structural), in core in design-standard (`LoadContext`), a machine-list item in architecture (`Loads & analysis`), and absent from every strip. Direct confirmation of Missing Layer (a) below.
4. **Export endpoint.** Terminal in design-standard + architecture; deferred-as-separate-schema in drawing; **absent in solver** (ends at supplier). Solver should at least acknowledge the handoff.

design-standard's strip is the one to treat as canonical — it's the only complete 8-stage version.

---

## §5. Missing layers

### (a) Loads / analysis — propose a dedicated artifact
**Fragments today:** `materials.Weight` (self-weight only) · `solver.LoadCase` (buried in structural) · `design-standard.LoadContext`+`LoadCombination` (in core) · `architecture` "Loads & analysis" (Rust machine list, no schema). Nothing owns the *traversal*: dead-load rollup → live load → tributary area → load path.

**Propose `loads-analysis-schema.html`**, pipeline position **between Placement and Structural Check** (the stage that's missing from every strip).

```
Layer 1 · Load Sources
  DeadLoad      (self-weight rollup: Σ Weight × members + assembly weights)
  LiveLoad      (occupancy psf by room use — IRC R301.5)
  SnowLoad / WindLoad / SeismicLoad   (ASCE 7)        «value»
Layer 2 · Distribution
  TributaryArea (geometry → area each member carries)  «value»
  LoadPath      (graph walk: sheathing→stud→plate→header→foundation) «service»
  LoadRollup    (accumulate demand down the path)       «service»
Layer 3 · Combination
  LoadCombination  (ASCE 7 combos — CANONICAL home; pull out of design-standard) «value»
Layer 4 · Output
  MemberDemand  (per-member {axial, moment, shear} → structural check) «value»
  LoadSolver    (orchestrates 1→4)                      «service»
```
- **Consumes:** `materials.Weight`, `ConnectionPoint`/`ConnectionGraph`, `solver.MemberPlacement`.
- **Produces:** `MemberDemand[]` → `CapacityCheck`/`SizingArbiter`.
- **Cites:** `asce7`, `irc` (loads), `wfcm` (reference library already maps `loads-lateral`).
- **Why its own artifact:** it's a distinct concern (a graph traversal producing demand) with ~10 types and a clear pipeline slot. It also becomes the single home for the currently-duplicated `LoadCombination` and the misplaced `LoadCase`/`LoadContext`.

### (b) System-architecture / compute-locality — **mostly exists; needs formalization**
Good news: `architecture.html` **already is** this spec. It covers the frontend-worker boundary (JS main / Rust worker), SoA typed arrays (canonical store), running totals (the incremental dirty-propagation engine), and the front/back split (two channels). The gap is that it's **prose + diagram with no machine-readable MODEL object** — unlike every other artifact, it can't be parsed into the type registry, and there's no formal contract for buffer layouts or the command protocol.

**Propose: formalize `architecture.html` into `system-architecture-schema.html`** with a MODEL object:

```
Layer 1 · Machines
  RustWorker (brain) · JSMainThread (hands+eyes) · Seam (boundary)   «entity»
Layer 2 · Channels
  CommandChannel  (Channel A: Intent/Command via wasm-bindgen, low volume) «service»
  StateChannel    (Channel B: shared linear memory, zero-copy)             «service»
Layer 3 · Canonical store
  SoABuffer    (typed-array layout per domain type — int tick columns)     «entity»
  BufferLayout (offsets GENERATED from the MODEL source — the contract that
                ties every domain type to its memory layout; can't drift)   «reference»
  DirtyRange   (changed buffer spans returned to JS)                        «value»
  RunningTotal (incremental aggregates: stud count, board-ft, cost, weight) «value»
Layer 4 · Protocol
  Intent/Command · DirtyPropagation · RenderMirror (JS read-only views)
Layer 5 · Locality rules
  LocalityRule[]  (the 5 review verdicts: snap=split, undo=Rust,
                   export=split, preview=JS-dumb, check=split)              «reference»
```
- **Why its own artifact (not crammed into a domain schema):** compute-locality is orthogonal — it describes *how* the domain runs, not *what* it models. Putting SoA/worker-boundary into a domain schema repeats exactly the mistake that S2-A (render-in-materials) is. `BufferLayout` is the keystone: generated from the same MODEL source the domain schemas already use, so the Rust and JS sides provably can't drift.

### (c) Drawings / document export — a third, already-acknowledged gap
Not on your list of two, but flagging it: **drawing** explicitly parks `drawings@0.x` as *"the next deliverable fork,"* and **architecture** review #3 splits it (Rust HLR + JS sheet composition). Propose `drawings-export-schema.html` (`Projection`, `HiddenLineRemoval`, `Sheet`, `Viewport2D`, `Dimension`, `TitleBlock`, `LineWeight`, `DrawingSet`; cites `graphic-standards`, `nationalcad`, `ching`). It's the terminal pipeline stage and currently has no model at all.

---

## §6. DRY + DDD consistency pass

**Flyweight pattern — uniform within schemas, duplicated across them.**
Correctly applied: `LumberSpec`, `SupplierSku` (materials); `ToolDefinition`, `MeasurementGrammar` (drawing); `NDSDesignValues` (solver mechanics); `RenderMesh` (intrinsic geometry + per-instance transform). `ToolDefinition` even names the lineage: *"Same pattern as LumberSpec/SupplierSku — a shared catalog looked up by key, never instantiated per use."* The pattern is healthy. The failure is **cross-schema**: the wood design-value flyweight exists three times (S1-A) and the factor stack 3–4 times (S1-B). Each instance is a correct flyweight; the *type* is just not single-sourced.

**Intrinsic / contextual split.**
- *Exemplary:* `solver.MemberPlacement` (shared `specRef` + per-instance install context — orientation, bracing, ends); `materials.RenderMesh`/`MeshInstance`; `materials.LumberStock.gradeStamp` (per-instance mill stamp correctly kept out of the flyweight).
- *Violation (S1-B):* `materials.MechanicalProperties.adjustmentFactors` folds context-dependent factors (`CD`, `Cr`, `Cp`, `CL`) into the intrinsic flyweight. Solver does it right; materials should too.
- *Minor (S3-C):* `solver.StockOption.price` inlines `cents` instead of referencing the `PriceQuote` value object.

**Stereotype consistency (S3-E):** a 6th «interface» stereotype appeared in design-standard (reusing render's color); «render» is absent in solver (intentional — no presentation types). Worth a one-line note in the shared legend so it's deliberate, not accidental.

---

## Recommended order of operations (when you move to edits)

1. **Stand up the `geometry-kernel` shared layer** and move `Plane`, `Transform`, `BoundingBox`, `CoordinateFrame`, `Path2D`, `Segment`, `Volume`/`Face`/`PushPullOp` into it (resolves S1-D, S2-B, the ambiguous-home flag, and most dangling primitives). *Highest leverage — one move, many fixes.*
2. **Single-source the wood design-value flyweight + factor stack** and split intrinsic vs contextual (S1-A, S1-B).
3. **Reconcile the structural stage** onto design-standard's seam; mark solver's flat model superseded (S1-C).
4. **Extract the render layer** out of materials into the render-mirror artifact (S2-A).
5. **Author the loads-analysis schema** and move `LoadCase`/`LoadContext`/`LoadCombination` into it (S5a) — this also fills the missing pipeline stage.
6. **Formalize `architecture.html`** into a MODEL-bearing system-architecture schema with `BufferLayout` as the cross-schema contract (S5b).
7. Sweep the S3 hygiene items (dangling refs, vocabulary, base-unit typing, labels) — cheap, and they stop the next schema from inheriting the drift.

Items 1–4 are pure consolidation of things already designed; nothing new is invented. Items 5–6 are the two genuinely new artifacts you flagged.
