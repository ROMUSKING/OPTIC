# Observability — Narrow v0 Stance

Cross-reference: [v0 executable spec](v0-executable-spec.md), book ch14.5 (tap / record / profile / replay).

## v0 (M0–M8 scaffolding)

Narrow v0 lowers **tap** and **record** query methods to CGIR comment hooks; profile/replay remain deferred.

| CGIR variant | Book concept | v0 behavior |
|--------------|--------------|-------------|
| `Tap` | Side-channel tap on an optic spine | Lowered when `m7_reserved=false`; codegen emits `// optic(tap):` comment hook |
| `Record` | Structured event record on query/optic | Lowered when `m7_reserved=false`; codegen emits `// optic(record):` comment hook |
| stub `Tap`/`Record` | Placeholder graphs | `m7_reserved=true`; **CGI-006** via `verify_to_diagnostic` |

### Surface syntax (M8)

| Query method | v0 |
|--------------|-----|
| `.tap("label")` | **supported** — `examples/tap_health.opt` |
| `.record("event")` | **supported** — `examples/record_health.opt` |
| `.profile(...)` / `.replay(...)` | **OBS-701** — `examples/unsupported_profile.opt`, `examples/unsupported_replay.opt` |
| Trailing `.tap`/`.record` after query methods | **OBS-702** — `examples/trailing_tap.opt`, `examples/trailing_record.opt` |

Inner optic/query codegen still runs; observability hooks are comment metadata only (book ch14.5 deferral stance).

### Hook string policy

- Single-line ASCII labels: `[A-Za-z0-9_.-]` (max 128 bytes)
- Only `\"` escape supported in source literals; multiline/control chars rejected at parse
- Codegen sanitizes labels again before `// optic(tap|record):` emission

### v0 structural limitations

| Limitation | Behavior |
|------------|----------|
| **Prefix-only hooks** | `.tap`/`.record` must precede `.get`/`.set`/`.map`; trailing hooks → **OBS-702** |
| **Orphan CGIR nodes** | `Tap`/`Record` are sibling nodes before the query root; not wired into compose/query reachability |
| **FusedLoop hook skip** | When `query_optic_name` cannot resolve from a `FusedLoop` root, hooks are skipped (no optic name) |
| **Single query** | v0 allows one query root; `emit_observability_hooks` scans only nodes before the query root index |
| **Surface gate symmetry** | `dump-ast` / `dump-hir` reject OBS-701/702 examples (same gate as `compile_check`) |

## Deferred beyond v0 narrow

- Profile counters and replay checkpoints (real runtime hooks)
- CLI: `optic profile`, `optic replay` (appendix B placeholders; v0 stubs + subcommands only; real hooks deferred)
- Grade-controlled erasure passes on observer nodes

(2026-06-20: runtime no-op + CLI arms added (metadata/stubs); still OBS-701 gate)

## Related rejections

GradedPrism and GradedTraversal are lowered in M7 (`PrismLeaf` / `TraversalLeaf` with `m7_reserved=false`); see [effect-coeffect-v0.md](effect-coeffect-v0.md). Host/foreign boundaries remain rejected via **TYP-010** (type phase), before CGIR build.

Stub M7/M8 reserved nodes (`m7_reserved=true`) still emit **CGI-006**. Properly lowered `Tap`/`Record` (`m7_reserved=false`) pass `verify` and do not hit CGI-006.

## Verification today

```bash
opticc check examples/tap_health.opt --json
opticc check examples/record_health.opt --json
opticc check examples/unsupported_replay.opt --json
opticc check examples/trailing_tap.opt --json
opticc check examples/trailing_record.opt --json
opticc dump-cgir examples/tap_health.opt --check
opticc dump-cgir examples/tap_record_chain.opt --check
opticc explain OBS-701
opticc explain OBS-702
```

Table-driven unit tests in `optic-cgir` assert **CGI-006** for stub `Tap`/`Record` (`m7_reserved=true`) and allow lowered nodes (`m7_reserved=false`).