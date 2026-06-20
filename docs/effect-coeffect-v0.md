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

## M7 prism (done)

- **GradedPrism** surface, HIR summaries (preview/review regions), and `PrismLeaf` CGIR lowering (`m7_reserved=false`)
- Rust codegen: preview as `Option<focus>`, review as conditional store in map queries
- Acceptance: `examples/alive_filter.opt` (full golden + execution parity)

## Deferred to M7+

- **GradedTraversal** surface, HIR lowering, and `TraversalLeaf` CGIR codegen
- Coinductive grade dimensions, staging grades, session types (book ch12–14)
- `unsafe optic` and `extern` / foreign host boundaries (rejected **TYP-010** in v0)

CGIR reserves `TraversalLeaf` stubs. Materialized stubs (`m7_reserved=true`) or unstubs traversal/tap/record nodes still fail **CGI-006** in narrow v0.

## Deferred to M8

Observability (`Tap`, `Record`) — see [observability-v0.md](observability-v0.md).

## Diagnostic pointers

| Code | When |
|------|------|
| TYP-010 | Traversal/unsafe/extern on surface (`collect_unsupported_surface`) |
| CGI-006 | Unstubs M7/M8 reserved CGIR node (`TraversalLeaf`, `Tap`, `Record`, or stub `PrismLeaf`) |
| GRA-* / ALI-* | Grade and alias checks on supported lens forms |

```bash
opticc explain TYP-010
opticc check examples/alive_filter.opt
opticc check examples/unsupported_traversal.opt --json
```