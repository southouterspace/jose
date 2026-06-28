# `schema/` — the single source of truth

This directory is the **canonical domain MODEL**. Everything downstream — Rust structs,
TypeScript types, and the `BufferLayout` keystone — is **generated** from here by
[`tooling/codegen`](../tooling/codegen). Edit the model here; never hand-edit generated
output.

```
schema/
├── model/      the machine contract — the domain MODEL
│   └── unified-model.json   12 layers · 178 types · 10-stage pipeline (v1.0.1)
└── registry/   type-ownership registry — every type → its one canonical home
    └── type-registry.json
```

## Rules

- **This is an input, not a document.** The human-readable rendering lives at
  [`docs/schema/unified-schema.html`](../docs/schema/unified-schema.html); the files here
  are what the build consumes.
- **Edit the model, regenerate, commit both.** Run `bun run codegen` after any change.
- **Drift is a build failure.** CI runs `bun run codegen:check`; if regenerating produces
  any diff against committed generated files, the build fails. This is what makes the
  Rust-writer / JS-reader contract a mechanical guarantee (see the `BufferLayout` keystone
  in the architecture layer of the model).
- **Versioned.** `meta.version` in `unified-model.json` (currently `1.0.1`) stamps the
  generated artifacts so a buffer mismatch is caught at load, not as silent corruption.

## Provenance

`model/unified-model.json` and `registry/type-registry.json` were promoted from
`docs/schema/` and `docs/analysis/` when the repo gained a build, and the duplicate `docs/`
copies were then removed — **`schema/` is now the sole home** for the machine contract.
The human-readable rendering [`docs/schema/unified-schema.html`](../docs/schema/unified-schema.html)
stays in `docs/` and links back here.
