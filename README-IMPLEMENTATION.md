# Optic Narrow v0 — Implementation Guide

This repository implements the **narrow v0** compiler described in the Optic Language Implementation Book (`book-sources/`, assembled in `book-sources/assembled.md`).

## Quick start

```bash
cargo build -p optic-cli
cargo run -p optic-cli -- check examples/health_get.opt
cargo test --workspace --no-fail-fast
```

The CLI binary is **`opticc`** (appendix B names it `optic`).

## Architecture

| Crate | Milestone | Role |
|-------|-----------|------|
| `optic-syntax` | M0 | Lexer, parser, AST |
| `optic-hir` | M1 | Lowering, `OpticSummary`, region map |
| `optic-typeck` | M2 | Types, grades, alias check |
| `optic-cgir` | M3 | CGIR graph + verifier |
| `optic-opt` | M4 | Map / compose / product fusion |
| `optic-codegen-rust` | M5 | Rust emission |
| `optic-runtime` | M5 | `Cursor` + SoA harness |
| `optic-diagnostics` | M6 | Structured diagnostic catalog |
| `optic` | M6 | Stable library facade |
| `optic-cli` | M6 | `opticc` command surface |

Pipeline: `.opt → parse → lower → typeck → CGIR → optimize → verify → emit Rust`

## Key documents

- **[PLAN.md](PLAN.md)** — milestone status, diagnostic catalog, gate checklist
- **[docs/v0-executable-spec.md](docs/v0-executable-spec.md)** — API, CLI, fixtures, verification
- **[fixtures/README.md](fixtures/README.md)** — golden update workflow

## Library API (`crates/optic`)

Embedding helpers: `compile_check`, `compile_check_from_path`, `compile_emit`, `explain_grade_from_src`, `explain_focus_from_src`, `parse_src`, `lower_src`, `dump_ast_src`, `dump_hir_src`, `collect_unsupported_surface`.

`dump-summary --node` and `dump-cgir --node` both resolve optic/let **names first** (via HIR summaries / `resolved_optics`), then fall back to numeric CGIR node ids.

M7 scaffolding: `PrismLeaf` and `TraversalLeaf` lowered (`m7_reserved=false`). M8 scaffolding: `.tap`/`.record` → `Tap`/`Record` (`m7_reserved=false`) + hooks; profile/replay → **OBS-701** (stubs). See docs. (Narrow v0 core M0-M6 complete.)

2026-06-20 continuation: ... ; PLAN/docs exact match (this task: plan updated vs book/app C/EBNF + actual Vec impl + wontfix for full traverse/profile in narrow).
2026-06-21: hard guards + shared helpers + boundary carry prep + test coverage (build guard via small+helper, TYP-010, harness match) + precise sync; see PLAN.
2026-06-21 further: match (not expect) on compile_emit (surface gate) + build(&TypedHir) (guard checks) for decision paths; continued doc/plan sync.
2026-06-21 continuation: explicit build match decision on real TypedHir from nested/records example; plan/docs synced same pass.
2026-06-21 continuation (facade): added explicit build_cgir match decision coverage in automated facade check_positive test (earlier facade build match work) using real TypedHir from record_health.opt (Entities data decl + Record hook + region_map paths exercised for guard Ok/non-exceed); goldens parity zero-change; same-pass sync to all docs.
2026-06-21 continuation (before_fusion + emit): explicit `match` on compile_cgir before_fusion early-return (record_health: Entities+Record+region_map) + compile_emit Ok (nested_position: Transform/Entities/region); real data; goldens zero-drift; same-pass sync. (See PLAN for full exercised details.)
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

# IMPL 35a48c49 hygiene note (high-level M5/M6 compact + sync): PLAN M5/M6 parentheticals pruned (exec order/table/After/immediate) + re-verify + sync hygiene on dirty tree; exactly 4 .md files edited (PLAN.md + docs/v0-executable-spec.md + fixtures/README.md + README-IMPLEMENTATION.md); M7 Phase 6 / "M7 complete (Track 6 this delta)" blocks kept byte-identical (core phrasing preserved, prefixes adapted); pre-existing qualified (base dirty tree); 0 drift. See PLAN + /tmp/grok-impl-summary-35a48c49.md .
- M7 Phase 3 (Track 3 this delta + supporting HIR): CGIR Prism/Traversal node realization (m7_reserved=false full paths); bias: hir::BranchBias carried on leaves for coproduct conditional branch edges (extract_branch_bias from decl + HIR validate); alias store coalesc verify over TraversalLeaf in verify (guard reuses is_simd 5pt#5 + ownership/put_writes); added minimal unit tests for extract + alias error; reused lower_get_put_leaf / build_region_fn / extract patterns / region_map / same Vec id + provenance tree (no new Branch variant per smallest); 11 files changed, 310 insertions(+), 40 deletions(-); goldens parity; fmt/clippy/tests/opticc/run on keys; PLAN/docs synced identical phrasing.
- M7 Phase 4 (Track 4 this delta): Prism/Traversal Compose Fusion + branch-coalescing via compose; lifted CGI-003 prism/trav under alias/ownership invariants (reuse is_simd_eligible for alias dec; bias field from Phase 3 present on leaves); extended compose_chain_forbidden_leaf + compose_fusion_block_note conditionally (no wholesale rewrite); removed codegen chain guard; FusedLoop+ComposeFusedBody reused for prism/trav; legal cases (e.g. Affine) now fuse/succeed, illegal keep CGI/FUS; updated examples/docs/tests/PLAN with live capture; some compose_prism/trav now succeed; kept CGI for bad. (Smallest; fmt/clippy/tests pass.)
- M7 Phase 5 (Track 5 this delta): SIMD Loop Emission (chunked portable 4-wide vector nests + Cursor/SoA in emit_traversal_query_* / emit_traversal_map_decay when is_simd_eligible_region); Branch Bias Hint Emission (inside per-el/fused loop after cursor via extract_leaf_bias on Prism/TraversalLeaf); Coproduct clean (if let Some or direct let= per wrap_some/returns_option without Some() double-wrap in emit_prism_preview_rust + callers; reuse helpers); reused emit_prism_preview_rust/emit_* /FusedLoop/is_simd_region/resolved_* inside current paths; goldens updated via OPTIC + unit bias cov + fmt/clippy/tests/opticc cli on keys; PLAN/docs synced *identical* phrasing; legacy OpticLeaf paths unchanged. (Smallest; fmt/clippy/tests pass.)
- M7 Phase 6 (Track 6 this delta): Conformance & Baselines (positive/negative harness for branch bias, SIMD eligibility, prism compose; execution parity + goldens; 4 harness conformance tests (pre-existing in prior delta, documented here; reused run_*/parse_entities/contains for hints/shapes; goldens separate (via golden_rust_* + assert_rust_golden in optic-codegen-rust; these 4 use manual transpile+temp+contains, not golden asserts)); full docs/PLAN sync *byte-identical* phrasing + M7 complete mark + live git capture with base dirty tree qualifier; re-verify all passed 0 opens). (Smallest precise; fmt/clippy/tests pass; b62dd648).
- M7 complete (Track 6 this delta): conformance suite solid (pos/neg for bias, SIMD, prism compose; exec parity+goldens via harness); examples exercise full; baselines via run parity + PLAN note; full docs/PLAN sync *byte-identical* phrasing + M7 complete mark; re-verify passed, 0 opens after fixes (fmt/clippy/tests/opticc/cli on all M7+new+legacy keys, goldens no drift; b62dd648). See live capture below.

`CheckOutcome` includes `typed_hir` for downstream tooling.

## Fixtures

Frozen evidence under `fixtures/` (tokens, ast, hir, cgir, rust, diagnostics, bench). Regenerate with `OPTIC_UPDATE_GOLDEN=1` or `opticc snapshot-update --confirm` — see `fixtures/README.md`.

## Book ↔ v0 mapping notes

- Book `TYP-201` (compose focus mismatch) is not separately emitted in v0; optic **body** mismatches use **`TYP-002`**.
- Parser recursion is capped at **`MAX_PARSE_DEPTH = 512`** (security; emits `PAR-001`).

Robustness assertions (CGIR wiring/ProductFlat/grades/regions/codegen/parser, error propagation) + clippy allows (required) added 2026-06-20. Parser depth complete on all decl paths; fusion updated for obs nodes; harness env exact match. Keep in sync with PLAN + v0-executable-spec + fixtures/README. Stray root + empty src cleaned. All asserts have msgs.

**See also (kept in sync):** exactly 4 .md files (PLAN.md + docs/v0-executable-spec.md + fixtures/README.md + README-IMPLEMENTATION.md) touched this delta for sync hygiene (M5/M6 high-level compact only).

Pre-existing qualified (base dirty tree); this-delta exhaustive: PLAN prunes on goals list + hygiene in 4 md; see IMPL 99f72455 note below.

# IMPL 99f72455 hygiene sync marker
# IMPL 99f72455 hygiene note (high-level M5/M6 compact + sync + re-verify): PLAN M5/M6 parentheticals pruned (high-level only); re-ran full fmt/clippy + execution/golden + opticc spot-checks + "RUN VERIFIED" on M7; exactly 4 .md files touched this delta for sync hygiene (M5/M6 high-level compact only); pre-existing qualified (base dirty tree); qualified captures; re-verify matrix clean. See PLAN + /tmp/grok-impl-summary-99f72455.md .
# IMPL 99f72455 hygiene note (uniform): re-run full fmt/clippy + execution/golden + opticc spot-checks + "RUN VERIFIED" on M7; residual high-level PLAN compact (M5/M6 verbose parentheticals pruned); exactly 4 .md files touched this delta for sync hygiene; pre-existing qualified (base dirty tree); qualified captures; re-verify matrix clean. See PLAN + /tmp/grok-impl-summary-99f72455.md .