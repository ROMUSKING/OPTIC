# Observability — Narrow v0 Stance

Cross-reference: [v0 executable spec](v0-executable-spec.md), book ch14.5 (tap / record / profile / replay).

## v0 (M0–M6 prelude)

Narrow v0 **does not** lower or codegen observability hooks. The CGIR enum reserves stub variants for later milestones:

| CGIR variant | Book concept | v0 behavior |
|--------------|--------------|-------------|
| `Tap` | Side-channel tap on an optic spine | Stub only; `verify` rejects if materialized (**CGI-006**) |
| `Record` | Structured event record on query/optic | Stub only; `verify` rejects if materialized (**CGI-006**) |

Surface syntax for tap/record/profile/replay is **not** accepted in v0. No `OBS-*` diagnostic codes are emitted yet; reserved for M8 when observability lowering is wired.

## Deferred to M8

- Tap/record insertion passes on CGIR
- Profile counters and replay checkpoints
- CLI: `optic profile`, `optic replay` (appendix B placeholders)
- Diagnostic catalog entries `OBS-*` with JSON witnesses

## Related rejections (M7 prep)

Traversal and host features are rejected earlier via **TYP-010** (type phase), before CGIR build. GradedPrism is lowered in M7; see [effect-coeffect-v0.md](effect-coeffect-v0.md) for grade/effect posture.

## Verification today

```bash
opticc check examples/health_get.opt --json   # no observability nodes in graph
opticc dump-cgir examples/health_get.opt --check
```

M7 reserved nodes (`PrismLeaf`, `TraversalLeaf`, `Tap`, `Record`) must never appear in post-build graphs for positive examples. Table-driven unit tests in `optic-cgir` assert **CGI-006** (structured diagnostic) for all four variants on hand-built graphs.