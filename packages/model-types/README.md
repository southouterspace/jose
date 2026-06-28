# @jose/model-types

The generated TypeScript view of the domain MODEL — id unions (`LayerId`, `TypeId`,
`Stereotype`, `PipelineStage`), a structural `MODEL_MANIFEST`, and `MODEL_VERSION`.

**Generated, not authored.** Source of truth is [`schema/`](../../schema); the generator
is [`tooling/codegen`](../../tooling/codegen). Regenerate with `bun run codegen` from the
repo root. Files under `src/generated/` carry a `@generated` header and are off-limits to
hand edits — CI (`bun run codegen:check`) fails on any drift.
