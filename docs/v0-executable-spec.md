# Optic Narrow v0 — Executable Spec

Concise reference for the working compiler in this repository. Normative semantics live in `book-sources/` (assembled in `book-sources/assembled.md`).

## Milestone gates (M0–M6)

| Milestone | Book | Implementation evidence |
|-----------|------|-------------------------|
| M0 | ch.7, app. D | `optic-syntax` lexer/parser; `fixtures/tokens/`, `fixtures/ast/` |
| M1 | ch.8 | `optic-hir` summaries + cursors; `fixtures/hir/` |
| M2 | ch.9 | `optic-typeck` grades/alias/types; `fixtures/diagnostics/` |
| M3 | ch.10 | `optic-cgir` + verifier; `fixtures/cgir/pre|post/` |
| M4 | ch.10 | `optic-opt` three fusions; FUS-501/FUS-502 notes |
| M5 | ch.11 | `optic-codegen-rust` + `optic-runtime`; `fixtures/rust/`, `fixtures/bench/` |
| M6 | ch.11, app. B | Stable diagnostics, frozen fixtures, library API (`crates/optic`) |

## Pipeline

```
.opt → parse → lower → check → build_cgir → optimize → verify → emit_rust
```

Library entrypoints (`crates/optic`):

| API | Purpose |
|-----|---------|
| `parse`, `lower`, `check`, `typeck_pass` | Front-end + M2 typeck |
| `build_cgir`, `optimize`, `emit_rust` | CGIR through Rust emission |
| `compile_check`, `compile_check_from_path`, `compile_cgir`, `compile_emit` | Full pipeline helpers (post-fusion verify) |
| `compile_*_with_limit` | Same with explicit source byte cap |
| `explain_grade_from_src` | Declared vs inferred grade (fails on target TYP-*; lenient for other items) |
| `explain_focus_from_src` | PathLift / root-path report (per-node lenience for sibling errors) |
| `collect_unsupported_surface` | Early surface gate: TYP-010 (unsafe/extern), OBS-701 (profile/replay), OBS-702 (trailing tap/record), OBS-703 (invalid hook label defense-in-depth) |
| `has_unsupported_observability` | True when diagnostics include OBS-701 or OBS-702 |
| `parse_src`, `lower_src`, `dump_ast_src`, `dump_hir_src` | Front-end helpers returning `Diagnostic` |
| `Diagnostic` | Structured diagnostic from `optic-diagnostics` |
| `DEFAULT_MAX_SOURCE_BYTES` | 4 MiB cap (matches CLI) |
| `CheckOutcome.typed_hir` | Typed HIR retained on successful `compile_check` |
| `source_id_from_path` | Stable `SourceId` for file-based parsing |

Appendix B names the binary `optic`; this repo ships **`opticc`** as the CLI crate binary.

## CLI (`opticc`)

| Command | Purpose |
|---------|---------|
| `check file.opt [--json]` | Full pipeline through codegen dry-run |
| `transpile file.opt [-o out.rs]` | Emit Rust |
| `dump-tokens` / `dump-ast` / `dump-hir` | M0–M1 snapshots |
| `dump-cgir [--before-fusion] [--check] [--node NAME\|N]` | CGIR inspection: **optic/let name via `resolved_optics` first**, then numeric NodeId. Unknown names: EXP-001 with `resolved_optics` candidates (dump-summary uses HIR binding names). |
| `dump-summary [--node NAME\|N]` | OpticSummary: **HIR name lookup first**, then numeric CGIR node id fallback |
| `explain-grade file.opt --node NAME [--json]` | Declared vs inferred grade + regions |
| `explain-focus file.opt --node NAME [--json]` | PathLift prefix + root-path for optic/let |
| `explain CODE` | Diagnostic catalog entry |
| `run file.opt` | Transpile + sandboxed `cargo run` harness |
| `bench [file.opt] [--update]` | Compare timing baselines (all examples or one file) |
| `doctor [file.opt]` | Toolchain + runtime path; optional file runs `check` |
| `profile file.opt`, `replay file.opt` | OBS-701 surface (stubs + CLI arms; defer real per observability-v0) |
| `snapshot-update --confirm` | Regenerate goldens |

## Diagnostic catalog (v0 core)

| Code | Phase | Meaning |
|------|-------|---------|
| PAR-001 | parse | Surface syntax error (includes `MAX_PARSE_DEPTH = 512` recursion cap) |
| PAR-010+ | parse | Reserved for future parse subcodes (v0 uses PAR-001 for syntax + depth) |
| RES-001 | resolve | Unknown binding/optic |
| HIR-101 | resolve | Duplicate SoA costate |
| EXP-001 | type | Unknown `--node` name (`explain-grade`, `explain-focus`, `dump-summary`, `dump-cgir`; numeric id misses use plain `node id N not found`) |
| TYP-001 | type | Unknown costate/focus type |
| TYP-002 | type | Optic body type ≠ declared focus |
| TYP-003 | type | Invalid grade syntax or optic clause mix (lens/prism/traversal) |
| TYP-004 | type | Cannot infer optic body type (v0) |
| TYP-010 | type | `unsafe optic` / `extern` host boundary syntax deferred to M7+ |
| GRA-110 | grade | Declared CacheGrade tighter than inferred |
| GRA-104 | grade | Sequential `>>>` exceeds cache bound |
| ALI-201 | alias | Product alias conflict |
| CGI-001–005 | cgir/codegen | Graph/build/codegen failures |
| CGI-006 | cgir | Stub M7/M8 reserved node (`m7_reserved=true` on PrismLeaf/TraversalLeaf/Tap/Record) |
| OBS-701 | type | Unsupported observability query method (profile/replay in v0) |
| OBS-702 | type | Trailing `.tap`/`.record` after `.get`/`.set`/`.map` (prefix-only in v0) |
| OBS-703 | type | Invalid observability hook label (typeck defense-in-depth; parse normally rejects) |
| FUS-501 | fusion | Compose blocked — intermediate escapes |
| FUS-502 | fusion | Compose blocked — legality precondition |

**Book remap:** appendix A `TYP-201` (compose focus/costate mismatch) is not a separate v0 code; optic **body** type mismatches map to **`TYP-002`** with `evidence.expected_type` / `evidence.actual_type`.

Witness JSON: `fixtures/diagnostics/*.json` from `opticc check --json` (includes `typ*.json`, `unsupported_*.json`, `explain_grade_*.json`, `explain_focus_*.json`).

Agent-repair smoke: `cargo test -p optic agent_repair_smoke` validates `ranked_fixes` + evidence fields on frozen GRA/ALI/TYP witnesses. Policy simulation: `cargo test -p optic-cli agent_repair_policy`.

## Fixture update process

```bash
cargo run -p optic-cli -- snapshot-update --confirm
# or per-layer:
OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-syntax golden_
OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-hir golden_hir
OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-cli golden_cgir
OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-cli diagnostics_json
OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-codegen-rust golden_rust
cargo run -p optic-cli -- bench --update
```

Review diffs before commit. See `fixtures/README.md`.

## M7 prism + traversal (done)

- `GradedPrism` preview/review: HIR summaries → `PrismLeaf` (`m7_reserved=false`) → Rust codegen
- `GradedTraversal` get/put (v0 surface; book traverse/update deferred): HIR summaries → `TraversalLeaf` (`m7_reserved=false`) → entity loop codegen + `// optic(traversal):` + optional `// simd-eligible` for homogeneous `SoA<f32>`
- `verify` allows properly lowered `PrismLeaf` / `TraversalLeaf` / `Tap` / `Record` (`m7_reserved=false`); rejects stubs (**CGI-006**)
- Compose+prism/traversal rejected at CGIR (**CGI-003** `prism_in_compose` / `traversal_in_compose`)
- Acceptance: `examples/alive_filter.opt`, `examples/all_healths.opt` (tokens/ast/hir/cgir/rust/bench + `run` execution)

## M7+ deferred

- traverse/update surface syntax (v0 uses get/put clauses for `GradedTraversal`)
- Full AVX intrinsics / LLVM SIMD (v0 emits metadata comment only; 2026-06-20 metadata bridge hardened)
- `unsafe optic` / `extern` host boundaries (**TYP-010**; HIR lowering prep 2026-06-20)
- profile/replay observability CLI — see `docs/observability-v0.md` (tap/record scaffolding done; stubs+CLI 2026-06-20)

## Verification

```bash
cargo test --workspace --no-fail-fast
cargo run -p optic-cli -- check examples/*.opt
```

Positive examples must transpile, compile, and match harness predicates in `optic-cli/tests/execution.rs`.

## Robustness (2026-06-20 + continuation)
- debug_assert! + guards for CGIR (incl unsafe boundary), simd, hir prep, parser (depth on decls+ all bodies listed in PLAN), emit.
- Sanit/enforce: costate + boundary names validated; harness now full env_clear+PATH match to cli.
- HIR unsafe lower prep exercised by explicit bypass test (delta vs prior documented; gates preserve).
- No golden change; coverage added for CLI profile arms, prep paths, hardened errs, body depth.
- Docs match code: depth on fn/let/optic/getput bodies, harness full clear, prep delta noted, "metadata/stubs/prep only".
- 2026 impl pass: parser depth + fusion Tap/Record + harness sync + more asserts; fully matches docs.