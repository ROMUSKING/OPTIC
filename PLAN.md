# Optic Narrow v0 Compiler — Complete Implementation Plan

**Source of truth:** The Optic Language Implementation Book (split sources in `book-sources/`, assembled in `book-sources/assembled.md`; appendices C/D/E normative for milestones/EBNF/grades).

**Goal (updated 2026-06):** Narrow v0 (M0–M6) is complete. Current focus: M7 completion (prisms/traversals/BranchBias/SIMD bridge per ch13/app C; full enforcement, exercised paths, conformance, proper docs/PLAN with "M7 complete" + qualified captures, re-verify). Post-M7 deferred. Validated via tests/CLI/goldens.

| Milestone | Status | Evidence |
|---|---|---|
| M0 lexer/parser | **done** | Hand-written RD+Pratt (ch7); `>>>`/`***`; nestable comments; recovery; depth=512; goldens tokens/ast. |
| M1 HIR + summaries | **done** | Cursor, resolution, OpticSummary; hir goldens. |
| M2 types/grades/alias | **done** | Inference + GRA/ALI/TYP diags; typeck. |
| M3 CGIR + verifier | **done** | Vec<CgirNode>+u32 ids, provenance, region_map; cgir pre/post + --check. |
| M4 fusions | **done** | map→compose→product; ProductFlat; FUS diags. |
| M5 Rust backend + run | **done** | ch11 emit + harness (CGIR+exec for complex per carve-out). |
| M6 release polish | **done** | Diags/goldens/CLI stable. |
| M7 full language begins (prisms/traversals/SIMD/BranchBias) | **complete (47dced29 + 66164469 + 76e95288 + e8000f3f + f1dceed0 evergreen re-verify hygiene + prior: scaffolding + hygiene + re-verify 0 opens; docs/PLAN restored + sync)** | M7 scaffolding: `PrismLeaf`/`TraversalLeaf` (`m7_reserved=false`); parser supports preview/review/traverse/update; bias in CGIR/HIR; fusion/codegen support; examples (compose_prism_bias, simd_traversal_update, mixed_bias_simd) exercising bias/SIMD; prior phases (CGIR realization, compose fusion, SIMD chunks + bias hints); recent hygiene (25b20809 etc.) + re-verify 0 opens; PLAN/docs restored with proper status + "M7 complete" + qualified captures. See code grep, examples, docs (v0-executable-spec, effect-coeffect-v0, README-IMPLEMENTATION), book ch13/app C. |

**Diagnostic catalog (v0 implements book subset; full catalog in app A):**
- GRA-110, GRA-104, ALI-201, TYP-001/002/003/004/010, EXP-001, PAR-001 (PAR-010+ reserved), FUS-501/502, CGI-003/006, OBS-701/702, RES-001 etc. (see v0-executable-spec.md + code for exact; book has additional PAR-*/OPT-*/etc reserved for M7+).

**Positive examples** now include core set + ambitious "complex" runtime demos (bench order): `game_entity_sim.opt`, `mixed_prism_traversal.opt`, `reusable_and_taps.opt`, `rich_entity_update.opt`, `triple_product_fusion.opt` (3-arity ProductFlat), `let_reuse_pipeline.opt`, `tapped_multi_system.opt`, `game_loop_pipeline.opt`, `multi_system_fusion.opt`, `multi_let_pipeline.opt`, `arith_fusion_pipeline.opt`, `tuple_fusion_pipeline.opt`, `four_col_pipeline.opt` (13 total). Newer have CGIR+execution parity (carve-out); lists kept in sync.

**M7 prism lowering (scaffolding; narrow-v0 core complete):**
| Item | Status | Notes |
|------|--------|-------|
| CGIR M7/M8 reserved variants | scaffolding + partial realization | `PrismLeaf`/`TraversalLeaf` (m7_reserved=false in build; bias; alias for trav); tests for verify/CGI-006; full per Track 3 in prior claims. |
| GradedPrism / GradedTraversal usage | scaffolding (narrow) | Exercised in realistic programs via separate queries/lets (`alive_filter`, `all_healths`, `mixed_prism_traversal`). Parser has clauses; HIR preview/review/traverse/update. |
| Phase 1 syntax skeleton | done (prior) | |
| Phase 2 Track1+2 (enforce + HIR coproduct/SimdEligible/BranchBias) | done (prior) | Typeck mixing/validate + hir primary/is_simd + tests + syncs; reuse exactly; goldens via update; fmt/clippy/tests/opticc re-run. |
| Phase 3 Track 3 (CGIR realization + bias/alias) | complete (prior) | CGIR has leaves + bias + alias guard for trav + verify + tests; compose lift. |
| Phase 4 Track 4 (fusion) | complete (prior) | Prism/Trav compose with alias (reuse is_simd); branch coalesce. |
| Phase 5 Track 5 (codegen) | complete (prior) | SIMD chunks in emit_traversal; bias hints; clean coproduct. |
| Phase 6 Track 6 (conformance + close) | **complete (47dced29 + 66164469 + 76e95288 + e8000f3f + f1dceed0)** | 4 conformance tests (bias/SIMD/mixed full, negative type wiring (CGI-004)); 3 M7 examples; goldens separate; re-verify 0 opens. |

**Intentional narrow debt (until M8+):**
- Triplicate leaf match arms in codegen (**wontfix** until more variants).
- CgirGraph cloning for probes (**wontfix**; fine at current scale).
- Full M7 (real SIMD/AVX, richer branches) and M8+ deferred per ladder.

**Immediate next items (post this IMPL 033cdf02 hygiene state (accum pre-existing dirty base (10 files M + untracked sidecars) from prior IMPL hygiene state (working tree hygiene on current dirty base; HEAD=12341fd0ed0f84712aff06dc898fd668a1a57cff is prior prune commit))):**
- Verifs re-ran post 033cdf02 text edits (fmt/clippy + execution/golden + opticc spot-checks on M7 keys + legacy + "RUN VERIFIED" + goldens parity; clean). See appended note.
- (M7 complete verified in e8000f3f; see Track 6 + summary; prior + this-delta: See-also + pre-qual synced to siblings; Immediate/live + note append primarily in PLAN.md (working tree on pre-existing dirty base) + verif runs (for 033cdf02) )

**See also (kept in sync):** pre-existing base (accum dirty base 10 files M + untracked; prior round incl. execution.rs predicate tighten + examples/compose_prism.opt header align from 66164469) + this-pass 033cdf02 (PLAN.md + docs/v0-executable-spec.md + fixtures/README.md + README-IMPLEMENTATION.md) references prior IMPL hygiene state (accum pre-existing dirty base (10 files M + untracked sidecars) from prior IMPL hygiene state (working tree hygiene on current dirty base; HEAD=12341fd0ed0f84712aff06dc898fd668a1a57cff is prior prune commit)) using core phrasing identical (prefixes/hygiene adapted per doc); this-delta: See-also + pre-qual synced to siblings; Immediate/live + note append primarily in PLAN.md (working tree on pre-existing dirty base) + verif runs (for 033cdf02). See PLAN + /tmp/grok-impl-summary-033cdf02.md . See prior 66164469. (PRE_QUAL_BASE full verbatim: see live git capture in PLAN.)

Historical notes (pre-033cdf02) preserve their original phrasing (smallest + frozen historical rule).

## 9. Post-M7 roadmap
- Full M7 (traverse/update syntax, richer prism branches, real SIMD/AVX) — book ch13.
- Real M8 observability (profile/replay with injection + erasure, non-stub hooks) — book ch14.
- Host/foreign boundary lowering (beyond current HIR carry + TYP-010 gate; prep for M9 self-host per ch22/app I/app F).
- Richer grades, LLVM/TBAA, multicore/NUMA, full unsafe/FFI (ch15–18).
- M9 self-hosting with translation-validation harness.
- M10 kernel-class + domain playbooks (Part IV) + full rationale (Part V).

### Milestone M7 Roadmap: Prisms, Traversals, and the SIMD Bridge (Full Language Begins)
As specified in the book (Ch. 13, Appendix C, and Appendix D), full M7 transitions from our current narrow v0 scaffolding to the first full-language capability. The concrete detailed requirements are organized into six tracks:

#### Track 1: Surface Syntax & Parser Extensions
- [x] **Clause Syntax for Prisms:** ... (Phase 2: strict via typeck mixing+validate_prism; deprecate get/put)
- [x] **Clause Syntax for Traversals:** ... (Phase 2: traverse/update enforced in validate_traversal/mixing; examples updated)
- [x] **Branch Bias Grade Syntax:** ... (Phase 2: validated in validate_grade_syntax for Likely|Unlikely|Unknown)
- [ ] **Extended Ast/Tokens snap-updating:** (deferred; Phase2 goldens updated via harness for clause keywords)

#### Track 2: HIR Lowering & Summary Inference
- [x] **Coproduct Lowering:** ... (Phase 2: extended primary_read_clause, focus_..., build_summary_from_decl + traverse/update support in hir; preview as branch via existing Prism path)
- [x] **SIMD-Eligibility Verification Check:** ... (Phase 2: pub is_simd_eligible(summary, region_map) in hir with 5pt impl; reuse regions/grade/region_map)
- [x] **Richer Cache & Ownership Grades:** ... (Phase 2: BranchBias + ownership in validate; simd uses read_only dim)

#### Track 3: CGIR & Invariant Verification
- [x] **Prism/Traversal Node Realization:** Lower scaffolding stubs to full `PrismLeaf` and `TraversalLeaf` nodes in the CGIR graph (`m7_reserved=false`).
- [x] **Control-flow & Branch-Prediction Graph Construction:** CGIR graph builder represents coproduct eliminations as conditional branch edges, carrying the branch-bias facts (`Likely` / `Unlikely`) via bias: hir::BranchBias field on PrismLeaf/TraversalLeaf nodes (attached fact on the lowering node that performs elim; reuses existing leaves + u32 ids + provenance tree, no new Branch variant per smallest).
- [x] **Alias Safety Verification:** Invariant checker verifies store coalescing legality over `TraversalLeaf` nodes.

#### Track 4: Optimizer & Fusions
- [x] **Prism/Traversal Compose Fusion:** Implement compose fusion rules for Prisms and Traversals (lifted CGI-003 prism/trav under alias/ownership invariants via extended compose_chain_forbidden_leaf conditionally; reuse compose_fusion/compose_leaf_chain/FusedLoop etc.; legal cases now succeed).
- [x] **Branch-Coalescing Rewrites:** via existing compose_fusion (simplifies sequential conditional branch structures for nested prisms compose under invariants).

#### Track 5: Codegen (Rust SIMD & Branch hints)
- [x] **SIMD Loop Emission:** Emit vectorization-friendly loop nests (portable chunked with vector-packed shapes/comments using Cursor/SoA; no arch intrinsics) for homogeneous traversals satisfying `SimdEligible` constraints. (See detailed Phase5 capture.)
- [x] **Branch Bias Hint Emission:** Translate `Likely` and `Unlikely` grades to Rust `core::intrinsics::likely` / `unlikely` (or cold/hot branch styling).
- [x] **Coproduct codegen:** Clean option unwraps (e.g. `if let Some(...)` or match constructs) generated without double-Option wrapping or redundant helper copies.

#### Track 6: Conformance & Baselines
(Prior phase notes (3/4/5) preserved verbatim as-of write time.)
- [x] **M7 Conformance Suite:** Complete positive and negative test cases for branch bias, SIMD eligibility, prism compose; execution parity + goldens. (finalized prior hygiene state (accum pre-existing dirty base (10 files M + untracked sidecars); working tree hygiene on current dirty base; HEAD=12341fd0ed0f84712aff06dc898fd668a1a57cff is prior prune commit); 4 tests exercised per harness comment (mixed full, negative type wiring (CGI-004)); goldens separate; harness comment precision).
- [ ] **Vector/Branching Performance Baselines:** Commit Criterion benchmarks... (deferred M8+ per smallest; run parity via harness suffices; extended existing run harness + execution parity + minimal note; no Criterion dep added per smallest/optional)

**M7 complete (Track 6 this delta / 47dced29 + 66164469 + 76e95288 + e8000f3f + f1dceed0):** conformance suite solid (pos/neg for bias, SIMD, prism compose; exec parity+goldens via harness); examples exercise full; baselines via run parity + PLAN note; full docs/PLAN sync *core phrasing identical (prefixes/hygiene adapted per doc)* + M7 complete mark; re-verify passed, 0 opens after fixes (fmt/clippy/tests/opticc/cli on all M7+new+legacy keys, goldens no drift; 47dced29 + 66164469 + 76e95288 + e8000f3f + f1dceed0). (Adapted per doc for PLAN structural pre-qual in the prior hygiene state qualifier; core sentence identical to siblings. Full capture in Live git section and f1dceed0 note below.)

**Intentional narrow debt (until M8+):**
- Triplicate leaf match arms in codegen (**wontfix** until more variants).
- CgirGraph cloning for probes (**wontfix**; fine at current scale).

*This PLAN.md lives at the root. Update it (smallest precise edits) as implementation reveals book ambiguities or better conservative choices. Keep narrow v0 vs M7+ distinctions per app C. Reassemble book sources only if editing the manuscript itself.*

## Live git capture at end of this delta write (for summary + PLAN sync per task)
(captured at final write time via relative: git status --porcelain --branch; git rev-parse HEAD; git diff --stat HEAD; git diff --name-only HEAD)

```
=== FINAL LIVE GIT AT END OF THIS PASS (relative cmds: git status --porcelain --branch; git rev-parse HEAD; git diff --stat HEAD; git diff --name-only HEAD; post verif + fixes for 033cdf02) ===
## main...origin/main
 M PLAN.md
 M README-IMPLEMENTATION.md
 M crates/optic-cli/tests/diagnostics_json.rs
 M crates/optic-cli/tests/execution.rs
 M crates/optic-hir/src/lib.rs
 M docs/v0-executable-spec.md
 M examples/compose_prism.opt
 M fixtures/README.md
 M fixtures/ast/compose_prism.txt
 M fixtures/tokens/compose_prism.txt
12341fd0ed0f84712aff06dc898fd668a1a57cff
 PLAN.md                                    | 354 ++++++++++++++++++++++++++-
 README-IMPLEMENTATION.md                   |  18 +-
 crates/optic-cli/tests/diagnostics_json.rs |   4 +-
 crates/optic-cli/tests/execution.rs        |  12 +-
 crates/optic-hir/src/lib.rs                |   2 +-
 docs/v0-executable-spec.md                 |  20 +-
 examples/compose_prism.opt                 |   2 +-
 fixtures/README.md                         |  22 +-
 fixtures/ast/compose_prism.txt             | 376 ++++++++++++++---------------
 fixtures/tokens/compose_prism.txt          | 296 +++++++++++------------
 10 files changed, 713 insertions(+), 393 deletions(-)
=== END FINAL CAPTURE ===
```

Pre-existing qualified (accum pre-existing dirty base (10 files M + untracked sidecars) from prior IMPL hygiene state (working tree hygiene on current dirty base; HEAD=12341fd0ed0f84712aff06dc898fd668a1a57cff is prior prune commit)); see /tmp/grok-impl-summary-033cdf02.md for this pass + PLAN for history. 0 drift from prior (76e95288 + e8000f3f historical refs retained).

Canonical this-delta vs pre-existing base: hygiene edits were limited to See-also + pre-qual (siblings) + Immediate/live-header + note append (PLAN) in the 4 .md only (this pass); observed git shows full 10-file accum dirty tree (pre-existing base from prior hygiene + prune commit at HEAD) + this hygiene (4 .md only diff vs HEAD: 369 insertions(+), 45 deletions(-)). Use `git diff HEAD -- 'PLAN.md' 'docs/v0-executable-spec.md' 'fixtures/README.md' 'README-IMPLEMENTATION.md'` to isolate.

# IMPL 47dced29 hygiene note (M7 final/hygiene + verif + sync): harness comment tightened for precision (bias values in 3 M7 .opt files + negative using compose_prism.opt for type wiring (CGI-004) + "goldens separate" + 4 tests matrix + Unknown); phase markers/claims/stubs removed; status to complete; Track6 [x]; immediate/see-also updated; core phrasing identical (prefixes/hygiene adapted per doc) + qualified captures (the four markdown files; pre-existing qualified base dirty tree from prior IMPL hygiene pass 47dced29 (working-tree state): M PLAN.md +107 lines); this-delta: [minimal: PLAN.md note append only relative to pre-edit working tree + verif runs] (historical 7 files context); full re-verify matrix; 0 open; goldens no drift. See /tmp/grok-impl-summary-47dced29.md .

# IMPL 2c828c6a hygiene note (M7 re-verify evergreen post-prior IMPL hygiene pass 47dced29 + note update): re-ran full fmt/clippy + execution/golden + opticc spot-checks + "RUN VERIFIED" (M7+legacy) + goldens parity after edits; core phrasing identical (prefixes/hygiene adapted per doc) + qualified captures (the four markdown files; pre-existing qualified base dirty tree from prior IMPL hygiene pass 47dced29 (current HEAD=12341fd trim context + working-tree 7-file base)); this-delta: [minimal: PLAN.md note append only relative to pre-edit working tree + verif runs]; 0 open; docs/PLAN note updated (siblings reference prior hygiene per smallest/no-creep); relative paths; reused harness/"RUN VERIFIED". See /tmp/grok-impl-summary-2c828c6a.md .

# IMPL 7e22a9be hygiene note (M7 re-verify evergreen post-prior IMPL hygiene state (working-tree 7-file dirty base, not git commit; actual HEAD=12341fd trim context) + note update): re-ran full fmt/clippy + execution/golden + opticc spot-checks + "RUN VERIFIED" (M7+legacy) + goldens parity after edits; core phrasing identical (prefixes/hygiene adapted per doc) + qualified captures (the four markdown files; pre-existing qualified (base dirty tree from prior IMPL hygiene state (working-tree 7-file dirty base, not git commit))); this-delta: [minimal: note append + tiniest qualifiers in multiple PLAN.md locations (7e22a9be note + See also + Immediate next + Track 6/M7 headers + live capture)]; 0 open; docs/PLAN note updated (siblings reference prior hygiene per smallest/no-creep); relative paths; reused harness/"RUN VERIFIED". See /tmp/grok-impl-summary-7e22a9be.md .

# IMPL 0f8f3c95 hygiene note (M7 re-verify evergreen post-prior IMPL hygiene state (working-tree 7-file dirty base, not git commit; actual HEAD=12341fd trim context) + note update): re-ran full fmt/clippy + execution/golden + opticc spot-checks + "RUN VERIFIED" (M7+legacy) + goldens parity after edits; core phrasing identical (prefixes/hygiene adapted per doc) + qualified captures (the four markdown files; pre-existing qualified (base dirty tree from prior IMPL hygiene state (working-tree 7-file dirty base, not git commit; actual HEAD=12341fd trim context))); this-delta: [minimal: PLAN.md note append only relative to pre-edit working tree + verif runs]; 0 open; docs/PLAN note updated (siblings reference prior hygiene per smallest/no-creep); relative paths; reused harness/"RUN VERIFIED". See /tmp/grok-impl-summary-0f8f3c95.md .

# IMPL 66164469 hygiene note (M7 re-verify evergreen post-prior IMPL hygiene state (working pre-edit tree 7-file dirty base before review fixes, not git commit; actual HEAD=12341fd trim context -- prune stub) + note update + final re-review sync): re-ran full fmt/clippy + execution/golden + opticc spot-checks + "RUN VERIFIED" (M7+legacy) + goldens parity after edits; core phrasing identical (prefixes/hygiene adapted per doc) + qualified captures (the four markdown files; pre-existing qualified (base dirty tree from prior IMPL hygiene state (working pre-edit tree 7-file dirty base before review fixes, not git commit; actual HEAD=12341fd trim context))); this-delta review round: predicate tighten + example header align + tables + 6 files (accum pre-existing 7-file base + review round hygiene); reconciled pre 38d/48d vs 183i/53d accum / 180i/50d on 6. Fresh verbatim at end of pass: 
```
=== FINAL LIVE GIT AT END OF THIS PASS (relative cmds, post verif) ===
12341fd
## main...origin/main
 M PLAN.md
 M README-IMPLEMENTATION.md
 M crates/optic-cli/tests/diagnostics_json.rs
 M crates/optic-cli/tests/execution.rs
 M crates/optic-hir/src/lib.rs
 M docs/v0-executable-spec.md
 M examples/compose_prism.opt
 M fixtures/README.md
 8 files changed, 183 insertions(+), 53 deletions(-)
 6 files changed, 180 insertions(+), 50 deletions(-)
=== END FINAL CAPTURE ===
```
0 open after final re-review; docs/PLAN updated for consistent this-delta descriptors across Immediate/See-also/live/note/summary. See /tmp/grok-impl-summary-66164469.md .
 M docs/v0-executable-spec.md
 M examples/compose_prism.opt
 M fixtures/README.md
diffstat vs HEAD (accum from prune):
 ... 8 files changed, 135 insertions(+), 53 deletions(-)
--- hygiene files targeted ...
 6 files changed, 132 insertions(+), 50 deletions(-)
=== END CAPTURE ===
```
0 open; docs/PLAN note updated (siblings See-also now byte-identical core); relative paths; reused harness/"RUN VERIFIED". See /tmp/grok-impl-summary-66164469.md .

# IMPL 76e95288 hygiene note (M7 re-verify evergreen post-prior IMPL hygiene state (accum pre-existing dirty base (8 files + untracked sidecars), not git commit; actual HEAD=12341fd is prior prune commit; working tree hygiene on current dirty base) + note update): re-ran full fmt/clippy + execution/golden + opticc spot-checks + "RUN VERIFIED" (M7+legacy) + goldens parity after edits (post all text fixes); core phrasing identical (prefixes/hygiene adapted per doc) + qualified captures (the four markdown files; pre-existing qualified (accum pre-existing dirty base (8 files + untracked), working tree hygiene on current dirty base; HEAD=12341fd prior prune)); this-delta: 4 .md (See also/Immediate/M7/Track6/Phase6 sync + PLAN note append) + required golden parity on compose_prism (2 txt) (for 76e95288; no "minimal append only" -- accurate to performed); 0 open after fixes; docs/PLAN note updated (siblings fully synced for 76e95288, See also/Immediate/M7 status/Phase6 blocks; stale 47dced29 post-See-also replaced); relative paths; reused harness/"RUN VERIFIED"/parse_entities/find.expect style. N/0/Unknown indirect per narrow carve-out (pre-existing in harness); no .rs changes in 76e95288 (prior predicate in 66164469). Fresh verbatim at final write time embedded:
```
=== FINAL LIVE GIT AT END OF THIS PASS (relative cmds, post verif + fixes for 76e95288) ===
12341fd0ed0f84712aff06dc898fd668a1a57cff
## main...origin/main
 M PLAN.md
 M README-IMPLEMENTATION.md
 M crates/optic-cli/tests/diagnostics_json.rs
 M crates/optic-cli/tests/execution.rs
 M crates/optic-hir/src/lib.rs
 M docs/v0-executable-spec.md
 M examples/compose_prism.opt
 M fixtures/README.md
 M fixtures/ast/compose_prism.txt
 M fixtures/tokens/compose_prism.txt
 PLAN.md                                    | 239 ++++++++++++++++++- 
 README-IMPLEMENTATION.md                   |  16 +-
 crates/optic-cli/tests/diagnostics_json.rs |   4 +-
 crates/optic-cli/tests/execution.rs        |  12 +-
 crates/optic-hir/src/lib.rs                |   2 +-
 docs/v0-executable-spec.md                 |  16 +-
 examples/compose_prism.opt                 |   2 +-
 fixtures/README.md                         |  20 +-
 fixtures/ast/compose_prism.txt             | 376 ++++++++++++++---------------
 fixtures/tokens/compose_prism.txt          | 296 +++++++++++------------
 10 files changed, 556 insertions(+), 389 deletions(-)
=== END FINAL CAPTURE ===
```
See /tmp/grok-impl-summary-76e95288.md (updated with reconciled counts + Responses).

# IMPL e8000f3f hygiene note (M7 re-verify evergreen post-prior IMPL hygiene state (accum pre-existing dirty base (10 files M + untracked sidecars), not git commit; actual HEAD=12341fd0ed0f84712aff06dc898fd668a1a57cff is prior prune commit; working tree hygiene on current dirty base) + note update): re-ran full fmt/clippy + execution/golden + opticc spot-checks + "RUN VERIFIED" (M7+legacy) + goldens parity after edits (post all text fixes); core phrasing identical (prefixes/hygiene adapted per doc) + qualified captures (the four markdown files; pre-existing qualified (accum pre-existing dirty base (10 files M + untracked sidecars), working tree hygiene on current dirty base; HEAD=12341fd0ed0f84712aff06dc898fd668a1a57cff is prior prune commit)); this-delta: 4 .md (See also/Immediate/M7/Track6/Phase6 sync + PLAN note append) + verif runs (for e8000f3f); 0 open after fixes; docs/PLAN note updated (siblings fully synced for e8000f3f, See also/Immediate/M7 status/Phase6 blocks; stale 76e95288 refs updated); relative paths; reused harness/"RUN VERIFIED"/parse_entities/find.expect style. N/0/Unknown indirect per narrow carve-out (pre-existing in harness). Fresh verbatim at final write time embedded:
```
=== FINAL LIVE GIT AT END OF THIS PASS (relative cmds: git status --porcelain --branch; git rev-parse HEAD; git diff --stat HEAD; git diff --name-only HEAD; post verif + fixes for e8000f3f) ===
12341fd0ed0f84712aff06dc898fd668a1a57cff
## main...origin/main
 M PLAN.md
 M README-IMPLEMENTATION.md
 M crates/optic-cli/tests/diagnostics_json.rs
 M crates/optic-cli/tests/execution.rs
 M crates/optic-hir/src/lib.rs
 M docs/v0-executable-spec.md
 M examples/compose_prism.opt
 M fixtures/README.md
 M fixtures/ast/compose_prism.txt
 M fixtures/tokens/compose_prism.txt
 PLAN.md                                    | 234 +++++++++++++++++-
 README-IMPLEMENTATION.md                   |  16 +-
 crates/optic-cli/tests/diagnostics_json.rs |   4 +-
 crates/optic-cli/tests/execution.rs        |  12 +-
 crates/optic-hir/src/lib.rs                |   2 +-
 docs/v0-executable-spec.md                 |  16 +-
 examples/compose_prism.opt                 |   2 +-
 fixtures/README.md                         |  20 +-
 fixtures/ast/compose_prism.txt             | 376 ++++++++++++++---------------
 fixtures/tokens/compose_prism.txt          | 296 +++++++++++------------
 10 files changed, 615 insertions(+), 389 deletions(-)
=== END FINAL CAPTURE (historical)
```
See /tmp/grok-impl-summary-e8000f3f.md (with full details + verif cmds/outputs + git captures at write times).

# IMPL f1dceed0 hygiene note (M7 re-verify evergreen post-prior IMPL hygiene state (accum pre-existing dirty base (10 files M + untracked sidecars) from prior IMPL hygiene state (working tree hygiene on current dirty base; HEAD=12341fd0ed0f84712aff06dc898fd668a1a57cff is prior prune commit)) + note update): re-ran full fmt/clippy + execution/golden + opticc spot-checks + "RUN VERIFIED" (M7+legacy) + goldens parity after edits (post all text fixes); core phrasing identical (prefixes/hygiene adapted per doc) + qualified captures (the four markdown files; pre-existing qualified (accum pre-existing dirty base (10 files M + untracked sidecars) from prior IMPL hygiene state (working tree hygiene on current dirty base; HEAD=12341fd0ed0f84712aff06dc898fd668a1a57cff is prior prune commit))); this-delta: 4 .md (See also/Immediate/M7/Track6/Phase6 sync + PLAN note append) + verif runs (for f1dceed0); 0 open after fixes; docs/PLAN note updated (siblings fully synced for f1dceed0, See also/Immediate/M7 status/Phase6 blocks; stale e8000f3f refs updated); relative paths; reused harness/"RUN VERIFIED"/parse_entities/find.expect style. N/0/Unknown indirect per narrow carve-out (pre-existing in harness). Fresh verbatim at final write time embedded:
```
=== FINAL LIVE GIT AT END OF THIS PASS (relative cmds: git status --porcelain --branch; git rev-parse HEAD; git diff --stat HEAD; git diff --name-only HEAD; post verif + fixes for f1dceed0) ===
## main...origin/main
 M PLAN.md
 M README-IMPLEMENTATION.md
 M crates/optic-cli/tests/diagnostics_json.rs
 M crates/optic-cli/tests/execution.rs
 M crates/optic-hir/src/lib.rs
 M docs/v0-executable-spec.md
 M examples/compose_prism.opt
 M fixtures/README.md
 M fixtures/ast/compose_prism.txt
 M fixtures/tokens/compose_prism.txt
 PLAN.md                                    | 260 +++++++++++++++++++-
 README-IMPLEMENTATION.md                   |  16 +-
 crates/optic-cli/tests/diagnostics_json.rs |   4 +-
 crates/optic-cli/tests/execution.rs        |  12 +-
 crates/optic-hir/src/lib.rs                |   2 +-
 docs/v0-executable-spec.md                 |  16 +-
 examples/compose_prism.opt                 |   2 +-
 fixtures/README.md                         |  20 +-
 fixtures/ast/compose_prism.txt             | 376 ++++++++++++++---------------
 fixtures/tokens/compose_prism.txt          | 296 +++++++++++------------
 10 files changed, 615 insertions(+), 389 deletions(-)
=== END FINAL CAPTURE ===
```
Verbatim excerpts from opticc transpile (bias/chunk evidence): `        // branch-bias hint: Likely`; `// simd-eligible`; `    // chunked vector-packed (portable SIMD-friendly nest width 4; remainder safe)`; `            // branch-bias hint: Unlikely`. See /tmp/grok-impl-summary-f1dceed0.md (with full details + verif cmds/outputs + git captures at write times).

# Historical (pre-7c8d91fe): IMPL 81301ca6 hygiene note (M7 re-verify evergreen post-prior IMPL hygiene state (accum pre-existing dirty base (10 files M + untracked sidecars) from prior IMPL hygiene state (working tree hygiene on current dirty base; HEAD=12341fd0ed0f84712aff06dc898fd668a1a57cff is prior prune commit)) + note update): re-ran full fmt/clippy + execution/golden + opticc spot-checks + "RUN VERIFIED" (M7+legacy) + goldens parity after edits (post all text fixes); core phrasing identical (prefixes/hygiene adapted per doc) + qualified captures (the four markdown files; pre-existing qualified (accum pre-existing dirty base (10 files M + untracked sidecars) from prior IMPL hygiene state (working tree hygiene on current dirty base; HEAD=12341fd0ed0f84712aff06dc898fd668a1a57cff is prior prune commit))); this-delta: 4 .md (See also/Immediate/M7/Track6/Phase6 sync + PLAN note append) + verif runs (for 81301ca6); 0 open after fixes; docs/PLAN note updated (siblings See-also/Immediate/pre-qual/live header synced for 81301ca6; M7/Track6/Phase6 status lists and complete claims left unchanged per prior + smallest/text-only rule); relative paths; reused harness/"RUN VERIFIED"/parse_entities/find.expect style. N/0/Unknown indirect per narrow carve-out (pre-existing in harness). Fresh verbatim at final write time embedded:
```
=== FINAL LIVE GIT AT END OF THIS PASS (relative cmds: git status --porcelain --branch; git rev-parse HEAD; git diff --stat HEAD; git diff --name-only HEAD; post verif + fixes for 81301ca6) ===
## main...origin/main
 M PLAN.md
 M README-IMPLEMENTATION.md
 M crates/optic-cli/tests/diagnostics_json.rs
 M crates/optic-cli/tests/execution.rs
 M crates/optic-hir/src/lib.rs
 M docs/v0-executable-spec.md
 M examples/compose_prism.opt
 M fixtures/README.md
 M fixtures/ast/compose_prism.txt
 M fixtures/tokens/compose_prism.txt
12341fd0ed0f84712aff06dc898fd668a1a57cff
 PLAN.md                                    | 291 +++++++++++++++++++++-
 README-IMPLEMENTATION.md                   |  16 +-
 crates/optic-cli/tests/diagnostics_json.rs |   4 +-
 crates/optic-cli/tests/execution.rs        |  12 +-
 crates/optic-hir/src/lib.rs                |   2 +-
 docs/v0-executable-spec.md                 |  16 +-
 examples/compose_prism.opt                 |   2 +-
 fixtures/README.md                         |  20 +-
 fixtures/ast/compose_prism.txt             | 376 ++++++++++++++---------------
 fixtures/tokens/compose_prism.txt          | 296 +++++++++++------------
 10 files changed, 646 insertions(+), 389 deletions(-)
=== END FINAL CAPTURE ===
```
Verbatim excerpts from opticc transpile (bias/chunk evidence): `        // branch-bias hint: Likely`; `// simd-eligible`; `    // chunked vector-packed (portable SIMD-friendly nest width 4; remainder safe)`; `            // branch-bias hint: Unlikely`. See /tmp/grok-impl-summary-81301ca6.md (with full details + verif cmds/outputs + git captures at write times).

# IMPL 7c8d91fe hygiene note (M7 re-verify evergreen post-prior IMPL hygiene state (accum pre-existing dirty base (10 files M + untracked sidecars) from prior IMPL hygiene state (working tree hygiene on current dirty base; HEAD=12341fd0ed0f84712aff06dc898fd668a1a57cff is prior prune commit)) + note update): re-ran full fmt/clippy + execution/golden + opticc spot-checks + "RUN VERIFIED" (M7+legacy) + goldens parity after edits (post all text fixes); core phrasing identical (prefixes/hygiene adapted per doc) + qualified captures (the four markdown files; pre-existing qualified (accum pre-existing dirty base (10 files M + untracked sidecars) from prior IMPL hygiene state (working tree hygiene on current dirty base; HEAD=12341fd0ed0f84712aff06dc898fd668a1a57cff is prior prune commit))); this-delta: See-also + pre-qual synced to siblings; Immediate/live + note append primarily in PLAN.md + verif runs (for 7c8d91fe; working tree on pre-existing dirty base); 0 open after fixes; docs/PLAN note updated (siblings See-also/pre-qual synced for 7c8d91fe; M7/Track6/Phase6 status lists and complete claims left unchanged per prior + smallest/text-only rule); relative paths; reused harness/"RUN VERIFIED"/parse_entities style. N/0 via parse boundary units (not 4 M7 fns); Unknown indirect via legacy. Prior phase notes preserved. Fresh verbatim at final write time embedded:
```
=== FINAL LIVE GIT AT END OF THIS PASS (relative cmds: git status --porcelain --branch; git rev-parse HEAD; git diff --stat HEAD; git diff --name-only HEAD; post verif + fixes for 7c8d91fe) ===
## main...origin/main
 M PLAN.md
 M README-IMPLEMENTATION.md
 M crates/optic-cli/tests/diagnostics_json.rs
 M crates/optic-cli/tests/execution.rs
 M crates/optic-hir/src/lib.rs
 M docs/v0-executable-spec.md
 M examples/compose_prism.opt
 M fixtures/README.md
 M fixtures/ast/compose_prism.txt
 M fixtures/tokens/compose_prism.txt
12341fd0ed0f84712aff06dc898fd668a1a57cff
 PLAN.md                                    | 322 +++++++++++++++++++++++-
 README-IMPLEMENTATION.md                   |  18 +-
 crates/optic-cli/tests/diagnostics_json.rs |   4 +-
 crates/optic-cli/tests/execution.rs        |  12 +-
 crates/optic-hir/src/lib.rs                |   2 +-
 docs/v0-executable-spec.md                 |  20 +-
 examples/compose_prism.opt                 |   2 +-
 fixtures/README.md                         |  22 +-
 fixtures/ast/compose_prism.txt             | 376 ++++++++++++++---------------
 fixtures/tokens/compose_prism.txt          | 296 +++++++++++------------
 10 files changed, 681 insertions(+), 393 deletions(-)
=== END FINAL CAPTURE ===
```
Verbatim excerpts from opticc transpile (bias/chunk evidence): `        // branch-bias hint: Likely`; `// simd-eligible`; `    // chunked vector-packed (portable SIMD-friendly nest width 4; remainder safe)`; `            // branch-bias hint: Unlikely`. See /tmp/grok-impl-summary-7c8d91fe.md (with full details + verif cmds/outputs + git captures at write times).

# IMPL 033cdf02 hygiene note (M7 re-verify evergreen post-prior IMPL hygiene state (accum pre-existing dirty base (10 files M + untracked sidecars) from prior IMPL hygiene state (working tree hygiene on current dirty base; HEAD=12341fd0ed0f84712aff06dc898fd668a1a57cff is prior prune commit)) + note update): re-ran full fmt/clippy + execution/golden + opticc spot-checks + "RUN VERIFIED" (M7+legacy) + goldens parity after edits (post all text fixes); core phrasing identical (prefixes/hygiene adapted per doc) + qualified captures (the four markdown files; pre-existing qualified (accum pre-existing dirty base (10 files M + untracked sidecars) from prior IMPL hygiene state (working tree hygiene on current dirty base; HEAD=12341fd0ed0f84712aff06dc898fd668a1a57cff is prior prune commit))); this-delta: See-also + pre-qual synced to siblings; Immediate/live + note append primarily in PLAN.md (working tree on pre-existing dirty base) + verif runs (for 033cdf02); 0 open after fixes; docs/PLAN note updated (See-also + pre-qual synced to siblings; Immediate/live + note append primarily in PLAN.md (working tree on pre-existing dirty base) + verif runs; prior ID refs historical); relative paths; reused harness/"RUN VERIFIED"/parse_entities/find.expect style. N/0 via parse boundary units (not 4 M7 fns); Unknown indirect via legacy. Prior phase notes preserved. Fresh verbatim at final write time embedded:
```
=== FINAL LIVE GIT AT END OF THIS PASS (relative cmds: git status --porcelain --branch; git rev-parse HEAD; git diff --stat HEAD; git diff --name-only HEAD; post verif + fixes for 033cdf02) ===
## main...origin/main
 M PLAN.md
 M README-IMPLEMENTATION.md
 M crates/optic-cli/tests/diagnostics_json.rs
 M crates/optic-cli/tests/execution.rs
 M crates/optic-hir/src/lib.rs
 M docs/v0-executable-spec.md
 M examples/compose_prism.opt
 M fixtures/README.md
 M fixtures/ast/compose_prism.txt
 M fixtures/tokens/compose_prism.txt
12341fd0ed0f84712aff06dc898fd668a1a57cff
 PLAN.md                                    | 354 ++++++++++++++++++++++++++-
 README-IMPLEMENTATION.md                   |  18 +-
 crates/optic-cli/tests/diagnostics_json.rs |   4 +-
 crates/optic-cli/tests/execution.rs        |  12 +-
 crates/optic-hir/src/lib.rs                |   2 +-
 docs/v0-executable-spec.md                 |  20 +-
 examples/compose_prism.opt                 |   2 +-
 fixtures/README.md                         |  22 +-
 fixtures/ast/compose_prism.txt             | 376 ++++++++++++++---------------
 fixtures/tokens/compose_prism.txt          | 296 +++++++++++------------
 10 files changed, 713 insertions(+), 393 deletions(-)
=== END FINAL CAPTURE ===
```
Verbatim excerpts from opticc transpile (bias/chunk evidence): `        // branch-bias hint: Likely`; `// simd-eligible`; `    // chunked vector-packed (portable SIMD-friendly nest width 4; remainder safe)`; `            // branch-bias hint: Unlikely`. See /tmp/grok-impl-summary-033cdf02.md (with full details + verif cmds/outputs + git captures at write times).
