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

## M7 prism + traversal (scaffolding for narrow v0; book M7 begins full)

- `GradedPrism` preview/review: HIR summaries → `PrismLeaf` (`m7_reserved=false`) → Rust codegen
- `GradedTraversal` get/put (v0 surface; book traverse/update deferred): HIR summaries → `TraversalLeaf` (`m7_reserved=false`) → entity loop codegen + `// optic(traversal):` + optional `// simd-eligible` for homogeneous `SoA<f32>`
- `verify` allows properly lowered `PrismLeaf` / `TraversalLeaf` / `Tap` / `Record` (`m7_reserved=false`); rejects stubs (**CGI-006**)
- Compose+prism/traversal rejected at CGIR (**CGI-003** `prism_in_compose` / `traversal_in_compose`)
- Acceptance: `examples/alive_filter.opt`, `examples/all_healths.opt` (tokens/ast/hir/cgir/rust/bench + `run` execution)

## M7+ deferred

- traverse/update surface syntax (v0 uses get/put clauses for `GradedTraversal`; full deferred per book)
- Full AVX intrinsics / LLVM SIMD (v0 emits metadata comment only; hardened)
- `unsafe optic` / `extern` host boundaries (**TYP-010**; HIR lowering prep)
- profile/replay observability CLI — see `docs/observability-v0.md` (stubs; full M8)

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
- 2026-06-21: more hard guards (verify/emit using shared scale helper), boundary flag carried+invariants in codegen, build guard exercised (small+helper/decision; notes precise vs empty case), match-not-expect + TYP-010 emit path explicit test, harness/doctor match; full verification; plan/docs synced; goldens untouched.
- 2026-06-21 further: explicit `match` (compile_emit error return/surface gate + build(&TypedHir) non-exceed guard checks) for decision explicitness; PLAN+docs appended same-pass; smallest; full verification pass.
- 2026-06-21 continuation: explicit match on build(&real TypedHir) for decision (nested_position/records exercised in cgir build path); doc/plan sync; no goldens/behavior change; full fmt/clippy/test/CLI (incl records/nested/host_boundary).
- 2026-06-21 continuation (facade): explicit `match build_cgir(&TypedHir)` Ok decision arm + guard non-exceed exercised in automated facade check_positive test (earlier facade build match work) using real TypedHir from record_health.opt (Entities data decl + Record hook + region_map paths exercised for guard Ok/non-exceed); goldens checked zero-drift; appended precise notes same-pass to docs/PLAN/fixtures/README.
- 2026-06-21 continuation (before_fusion + emit): explicit `match` on compile_cgir before_fusion early return (facade_compile_cgir_before_fusion_positive + record_health) + compile_emit Ok (facade_compile_emit_positive + nested_position) using real data/region paths; docs/PLAN synced same-pass (verbatim exercised-path language).
- 2026-06-21 continuation (scale): explicit `match` for additional cgir scale guard decision coverage + .expect conversions; see PLAN. goldens zero-drift.
- 2026-06-21 continuation (cgir/facade remaining build+compile .expect conversions + real fixtures + comment tighten; this run): converted remaining build .expect + outer record_health compile_check; cgir query health_get/set + tap_record_chain; facade alive/all_healths/from_path; corrected fixture lists; standardized panics/headers; see PLAN. (canonical parenthetical; real this-run fixtures only)
- 2026-06-21 continuation (codegen helper + cgir integration build .expect conversions + golden fixture coverage + doc/plan sync; this run): codegen helper assert_rust_golden + cgir integ large-N + optic selection comment to explicit match (real golden fixtures for build guard decisions); same-pass sync with identical one-liners to PLAN+docs; git 7f 122+/25- (final observed after review nits); see PLAN.
- 2026-06-21 continuation (remove heuristic default_summary fallback in HIR prod path + explicit Err + unified RES-001 + minimal coverage + doc/plan sync; this run): hir Named compute (unified RES msg + minimal inline test coverage + annotated case) + catch-up one-liners + base summary sync; see PLAN.
- 2026-06-21 continuation (lift_region unwrap removal + [0] indexing + doc/plan sync; this run): isolated lift hunk + smallest direct len1 test + appends (hir lift_region for ch8 Seq); see PLAN.
- 2026-06-21 continuation (test error handling consistency .unwrap->.expect + doc/plan sync; this run): consolidated redundant any/find + terse expect + hardened evidence + sibling real-fixture find witness added in typeck GRA-110 (inline + invalid_grade sibling); same-pass sync + summary refresh; see PLAN.
- 2026-06-21 continuation (test error handling consistency remaining GRA terse find.expect harness style + doc/plan sync; this run): remaining loose .any() for GRA-110 in typeck despite test -> direct find.expect("CODE") (real invalid_grade.opt, terse harness style); smallest; same-pass sync; see PLAN.
- 2026-06-21 continuation (review issues addressing: counts/placeholders/phrasing/var/comment/bookrefs + doc/plan sync; this run): fixed placeholders/accum counts (concrete hunk net + grep), added sibling comment parity, aligned phrasing to "terse find.expect("CODE") harness style", corrected book ch9/app refs, aligned var name to err family, wontfix outside-smallest (5/7); same-pass sync + summary refresh; see PLAN.
- 2026-06-21 continuation (residual stale quals update in PLAN/summary + doc/plan sync; this run): updated stale numbers/phrases (94->115+/8- final observed, hunk qual) in PLAN + summary; new PLAN subsection + verbatim one-liners; smallest text; see PLAN.
- 2026-06-21 continuation (final number accuracy in residual sync subsection + doc/plan sync; this run): updated 115->127 final observed (incl append) in PLAN residual subsection + this sync append; one-liners; see PLAN.

