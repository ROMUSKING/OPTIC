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
- `GradedTraversal` get/put (v0 surface; book traverse/update deferred but Phase1 syntax skeleton added): HIR summaries → `TraversalLeaf` (`m7_reserved=false`) → entity loop codegen + `// optic(traversal):` + optional `// simd-eligible` for homogeneous `SoA<f32>`
- `verify` allows properly lowered `PrismLeaf` / `TraversalLeaf` / `Tap` / `Record` (`m7_reserved=false`); rejects stubs (**CGI-006**)
- Compose+prism/traversal rejected at CGIR (**CGI-003** `prism_in_compose` / `traversal_in_compose`)
- Acceptance: `examples/alive_filter.opt`, `examples/all_healths.opt` (tokens/ast/hir/cgir/rust/bench + `run` execution)

## M7+ deferred

- traverse/update surface syntax (Phase 1 basic skeleton support added per approved plan §Phase1 + reuse parse_grade_dim etc; v0 get/put still surface for trav, full enforce later; book ch13)
- 2026-06 M7 Phase 1 skeleton (this delta): KwTraverse/KwUpdate + BranchBias + fields + 28 ast goldens + EBNF/PLAN/docs sync; 37 files +205/-28 (core 9f +129i); fmt/clippy/tests/opticc pass; see review.
- Full AVX intrinsics / LLVM SIMD (v0 emits metadata comment only; hardened)
- `unsafe optic` / `extern` host boundaries (**TYP-010**; HIR lowering prep)
- profile/replay observability CLI — see `docs/observability-v0.md` (stubs; full M8)

## Verification

```bash
cargo test --workspace --no-fail-fast
cargo run -p optic-cli -- check examples/*.opt
```

Positive examples must transpile, compile, and match harness predicates in `optic-cli/tests/execution.rs`. Runtime-focused complex set (13 total in bench_examples order: game_entity_sim.opt, mixed_prism_traversal.opt, reusable_and_taps.opt, rich_entity_update.opt, triple_product_fusion.opt, let_reuse_pipeline.opt, tapped_multi_system.opt, game_loop_pipeline.opt, multi_system_fusion.opt, multi_let_pipeline.opt, arith_fusion_pipeline.opt, tuple_fusion_pipeline.opt, four_col_pipeline.opt) uses CGIR+execution parity (see fixtures/README.md carve-out; no full token/ast/hir/rust/bench). Harness expanded with richer N=0/arity asserts per PLAN.

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
- 2026-06-21 continuation (evidence robustness get+and_then for TYP-002 bare indexing in test + doc/plan sync; this run): hardened bare evidence["expected_type"] at TYP-002 site to get+and_then (GRA style; inline exercising keys, real fixture for presence/parity per smallest); smallest; same-pass sync; see PLAN.
- 2026-06-21 continuation (evidence robustness get+and_then for TYP-003 bare indexing in test + doc/plan sync; this run): hardened bare evidence["fragment"] at TYP-003 site to get+and_then (GRA/TYP-002 style; inline src for GradedOptic mix); smallest; same-pass sync; see PLAN.
- 2026-06-21 continuation (review addressing: doc/PLAN precision nits for counts/exercised/accum + wontfix future scope; this run): tightened counts/accum/exercised qual (hunk-isolated grep); kept verbatim one-liners; wontfix scope-violating coverage suggestions; same-pass sync + summary refresh; see PLAN.
- 2026-06-21 continuation (residual addressing sync for final observed stat + doc/plan sync; this run): updated addressing for 63+/3- final stat qual; one-liners; see PLAN.
- 2026-06-21 continuation (re-review residual nits: count/phrasing in addressing+residual subsections + one-liner desc + doc/plan sync; this run): updated stale 37/53 to final 63+/3- + hunk qual in PLAN subsections + one-liners; see PLAN.
- 2026-06-21 continuation (evidence robustness get+and_then for host-boundary TYP-010 bare indexing in test + doc/plan sync; this run): hardened bare evidence filter_map at host_boundary TYP-010 site to get+and_then (self-host prep); real fixture; same-pass sync; see PLAN.
- 2026-06-21 continuation (review addressing: tiniest comment align + doc/PLAN nits (counts/exercised/self-containment) + wontfix bare/coverage (1/2/3/4/5/6/7/8); this run): tiniest comment style align + PLAN/summary precision nits (counts/exercised/self-contain); wontfix remaining bare/None/CLI/coverage per smallest + "other bare left"; see PLAN.
- 2026-06-21 continuation (residual polish: count/phrasing/exercised/self-containment applied directly to primary host subsection (590-598) + doc/plan sync; this run): applied fixes to primary bullets (exercised tighten + counts 98+/4- + self-contain cross-refs); new subsection + one-liners; see PLAN.
- 2026-06-22 continuation (test error handling consistency for TYP-010 host-boundary loose .any -> find.expect("TYP-010") harness style + real fixture direct boundary + doc/plan sync; this run): converted loose any to find.expect on host_boundary TYP-010 paths in facade (real fixture, boundary coverage, harness style); same-pass sync + verif; see PLAN.
- 2026-06-22 continuation (review addressing nits: bare chains/let_ removal + preceding comment style + counts/phrasing accuracy + other bare doc note + doc/plan sync; this run): src style polish to bare find.expect + preceding // + vestigial name remove + "other bare left" note; doc/PLAN count/phrasing fixes (5 find.expect across 3 fns, final observed git/hunk-isolated grep); same-pass appends; see PLAN.
- 2026-06-22 continuation (residual polish: primary canonical 2026-06-22 subsection direct updates for counts/phrasing/self-contain + final observed 155+/19- + doc/plan sync; this run): smallest text-only to primary (~648); appended new residual polish sub + verbatim one-liners; see PLAN.
- 2026-06-22 continuation (evidence robustness get+and_then for host-boundary TYP-010 bare json indexing in cli harness test + doc/plan sync; this run): hardened bare json evidence access in cli host_boundary TYP-010 test to get+and_then (cli harness); real fixture direct; same-pass sync + verif; see PLAN.
- 2026-06-22 continuation (test error handling consistency remaining TYP-002 loose .any -> find.expect("TYP-002") harness style in cli + doc/plan sync; this run): converted loose .any to find.expect("TYP-002") terse harness style in cli test (real typ002 fixture); same-pass sync + verif; see PLAN.
- 2026-06-22 continuation (test error handling consistency TYP-001 loose .any -> find.expect("TYP-001") harness style in facade + doc/plan sync; this run): converted loose .any to find.expect("TYP-001") terse harness style in facade test (real typ001 fixture); same-pass sync + verif; see PLAN.
## 2026-06-23 (TYP/EXP/OBS harness series continuation)
- 2026-06-23 continuation (test error handling consistency TYP-001 loose .any -> find.expect("TYP-001") harness style in typeck + doc/plan sync; this run): converted loose .any to find.expect("TYP-001") terse harness style in typeck test (real typ001 fixture); same-pass sync + verif; see PLAN.
- 2026-06-23 continuation (test error handling consistency TYP-002 loose .any -> find.expect("TYP-002") harness style in typeck + doc/plan sync; this run): converted loose .any to find.expect("TYP-002") terse harness style in typeck test (real typ002 fixture); same-pass sync + verif; see PLAN.
- 2026-06-23 continuation (test error handling consistency TYP-004 loose .any -> find.expect("TYP-004") harness style in typeck + doc/plan sync; this run): converted loose .any to find.expect("TYP-004") terse harness style in typeck test (real typ004 fixture); same-pass sync + verif; see PLAN.
- 2026-06-23 continuation (test error handling consistency EXPLAIN_UNKNOWN_NODE loose .any -> find.expect("EXPLAIN_UNKNOWN_NODE") harness style in typeck + doc/plan sync; this run): converted loose .any to find.expect("EXPLAIN_UNKNOWN_NODE") terse harness style in typeck test (real health_get.opt fixture); same-pass sync + verif; see PLAN.
- 2026-06-23 continuation (test error handling consistency OBS-70x loose .any -> find.expect("OBS-70x") harness style + evidence get+and_then + real fixtures in facade + doc/plan sync; this run): converted loose .any + bare evidence to find.expect/get+and_then (real unsupported_*/trailing_*.opt fixtures, terse harness; see PLAN); same-pass sync + verif; see PLAN.
- 2026-06-23 re-review residual (c17ce56c re-review nits: doc-claim vs code on let-d reformat + stats/excerpts + TYP phrasing + historical qual + sync; this run): corrected claims + stats + excerpts + phrasing + added defense note; src let-d compact defended (short facade); live stats final 4 files changed, 12 insertions(+), 12 deletions(-) (at final write time after all sync addressing); appended residual + one-liners; verif clean; see PLAN. (complete delta applied and verified)
- 2026-06-23 re-review round 2 stats sync (minimal): refreshed all "final live" / 98i/36d etc to exact live 4f 12i/12d, grep=8, raw wc=0 at final write time after all sync addressing in PLAN/summary/residuals (captured right now); no code change; same-pass note sync. (verified)
- 2026-06-23 residual addressing (hygiene: summary stats to 5f 46i/20d live, comments to pure template, PLAN residual boilerplate match; this run): tiniest fixes in /tmp summary + src comments (no review meta) + PLAN; live git 45i/20d at write; appended one-liner; verif clean; see PLAN.
- 2026-06-23 continuation (test error handling consistency OBS-702 trailing_tap loose .any -> find.expect("OBS-702") harness style in typeck + doc/plan sync; this run): converted loose .any to find.expect("OBS-702") terse harness style in typeck test (real trailing_tap fixture); same-pass sync + verif; see PLAN.
- 2026-06-24 continuation (test error handling consistency OBS-702 trailing_* loose .any -> find.expect("OBS-702") harness style in typeck + doc/plan sync; this run): converted loose .any to find.expect("OBS-702") terse harness style in typeck tests (real trailing_* fixtures); same-pass sync + verif; see PLAN.
- 2026-06-24 residual addressing (review f8046ad3: scope/claims/evidence/stale stats fix + both tap+record accuracy; this run): corrected claims to match vs-HEAD (both trailing_*), evidence let-d, note, stats; appended residual + one-liners; verif clean; see PLAN.
- 2026-06-24 continuation (evidence robustness: harden diags[0] bare indexing + direct evidence[""] to let d=find.expect + get+and_then for OBS-701 unsupported_* profile/replay in typeck + update other bare note + doc/plan sync; this run): hardened fragile diags[0] + evidence access to captured find.expect + get+and_then (real unsupported_*.opt fixtures, terse harness; see PLAN); same-pass sync + verif; see PLAN.
- 2026-06-24 continuation (evidence robustness: harden diags[0] bare indexing + direct evidence[""] to let d=find.expect + get+and_then for OBS-703 invalid_hook_label_obs703 in typeck + update other bare note + doc/plan sync; this run): hardened fragile diags[0] + evidence access to captured find.expect + get+and_then (synthetic for invalid hook label, terse harness; see PLAN); same-pass sync + verif; see PLAN.
- 2026-06-24 continuation (evidence robustness: harden bare .any to find.expect("OBS-701") for nested_replay synthetic (test_collect_unsupported_surface_nested_replay_in_binary replay in binary rhs) in typeck + update other bare note + doc/plan sync; this run): hardened fragile .any to find.expect (synthetic for replay in binary rhs, terse harness; see PLAN); same-pass sync + verif; see PLAN.
- 2026-06-24 continuation (test error handling consistency PAR-001 loose .any -> find.expect("PAR-001") harness style in facade + doc/plan sync; this run): converted loose .any to find.expect("PAR-001") terse harness style in facade (oversized synthetic); same-pass sync + verif; see PLAN.
- 2026-06-24 continuation (self-host bootstrap+HIR carry+marker+coverage+verbatim doc sync per ch22/appI/appF/PLAN; live `8 files changed, 77 insertions(+), 18 deletions(-)`): see PLAN sub.
- 2026-06-25 continuation (typeck full Extern carry + passes-as-optics comment + verbatim doc sync per ch22/appI/appF/PLAN; live `5 files changed, 19 insertions(+), 1 deletion(-)`): see PLAN sub.
- 2026-06-25 continuation (cgir full Extern carry + explicit arm + passes-as-optics comment + verbatim doc sync per ch22/appI/appF/PLAN; live `5 files changed, 59 insertions(+), 2 deletions(-)`): see PLAN sub.
- 2026-06-25 continuation (CLI + dump/facade explicit Extern arms in tooling + dump helper comment + verbatim doc sync per ch22/appI/appF/PLAN; live `8 files changed, 26 insertions(+), 1 deletion(-)` (fix round)): see PLAN sub.
- 2026-06-25 continuation (harness expand: richer boundary asserts + 4-col arity edge on real N=0 + parse_entities + verbatim doc sync; execution.rs + PLAN + fixtures + v0-exec + main; live `7 files changed, 230 insertions(+), 31 deletions(-) (5 core + 2 marker restores for pre-existing asserts)`; same-pass sync + verif; see PLAN): see PLAN sub.

