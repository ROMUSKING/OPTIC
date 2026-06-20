# Effects, Coeffects, and Grades — Narrow v0 Stance

Cross-reference: [v0 executable spec](v0-executable-spec.md), book ch12–14 (graded boundaries, staging, sessions).

## What v0 already carries

`OpticSummary` (M1) and CGIR `OpticLeaf` nodes materialize a **minimal** effect/coeffect footprint sufficient for lens codegen:

| Field / check | Meaning in v0 |
|---------------|---------------|
| `get_determinism` / `put_determinism` | Pure vs effectful classification on bodies (inference subset) |
| `serializable` | Summary flag for host-boundary posture |
| `CacheGrade` / `OwnershipGrade` | ch9.9.3 inference; **GRA-110**, **GRA-104** |
| `alias_safe` on products | **ALI-201** put-read hazards |

These are enforced through `optic-typeck` and verified on CGIR compose/product wiring.

## M7 prism + traversal (done)

- **GradedPrism** surface, HIR summaries (preview/review regions), and `PrismLeaf` CGIR lowering (`m7_reserved=false`)
- Rust codegen: preview as `Option<focus>`, review as conditional store in map queries
- Acceptance: `examples/alive_filter.opt` (full golden + execution parity)

- **GradedTraversal** surface (v0 get/put clauses), HIR summaries, and `TraversalLeaf` CGIR lowering (`m7_reserved=false`)
- Rust codegen: entity-loop get/put/map with `// optic(traversal):` + optional `// simd-eligible` metadata (no intrinsics in v0)
- Acceptance: `examples/all_healths.opt` (full golden + execution parity)

## Deferred to M7+

- Coinductive grade dimensions, staging grades, session types (book ch12–14)
- `unsafe optic` and `extern` / foreign host boundaries (rejected **TYP-010** in v0; HIR lowering prep + explicit test + sanit 2026-06-20)
- traverse/update surface syntax (book ch13; v0 uses get/put for `GradedTraversal`)
- Full AVX intrinsics / LLVM SIMD bridge (beyond v0 metadata comment only)

CGIR rejects stub `TraversalLeaf` (`m7_reserved=true`) and stub `Tap`/`Record` (`m7_reserved=true`) via **CGI-006**. Lowered observability nodes (`m7_reserved=false`) pass verify.

## M8 observability (scaffolding done)

`.tap`/`.record` query methods → `Tap`/`Record` CGIR + comment hooks. Profile/replay → **OBS-701**; trailing hooks → **OBS-702**. Witnesses: `tap_record_chain.opt`, `compose_tap.opt`, `unsupported_replay.opt`, `trailing_tap.opt`, `trailing_record.opt`. See [observability-v0.md](observability-v0.md).

## Diagnostic pointers

| Code | When |
|------|------|
| TYP-010 | `unsafe optic` / `extern` on surface (`collect_unsupported_surface`) |
| TYP-003 | Clause mix (e.g. GradedTraversal + preview/review) |
| CGI-003 | Compose+prism/traversal (`prism_in_compose`, `traversal_in_compose`) |
| CGI-006 | Stub M7/M8 reserved CGIR node (`m7_reserved=true`) |
| OBS-701 | Unsupported observability query method (profile/replay) |
| OBS-702 | Trailing `.tap`/`.record` after `.get`/`.set`/`.map` (prefix-only in v0) |
| GRA-* / ALI-* | Grade and alias checks on supported lens/prism/traversal forms |

```bash
opticc explain TYP-010
opticc explain TYP-003
opticc explain CGI-003
opticc check examples/all_healths.opt
opticc check examples/unsupported_traversal.opt --json
```