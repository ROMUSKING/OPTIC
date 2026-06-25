# Optic Narrow v0 Compiler — Complete Implementation Plan

**Source of truth:** The Optic Language Implementation Book (split sources in `book-sources/`, assembled in `book-sources/assembled.md`; appendices C/D/E normative for milestones/EBNF/grades).

**Goal (updated 2026-06):** Narrow v0 (M0–M6) is complete. Current focus: grow the suite of complex acceptance examples and functional (runtime) tests using the implemented functionality, while keeping goldens/docs/PLAN in sync. M7+ remains deferred per book (app C). Core output still includes:
- A usable `opticc` CLI (and library API).
- End-to-end: `.opt` source → parse → HIR + summaries → type/grade/alias check (with good diagnostics) → CGIR (with provenance) → the three fusion rewrites → readable, correct Rust emission.
- Tiny `optic-runtime` (Cursor + SoA support).
- Acceptance examples that **compile to Rust, the emitted Rust compiles and runs, and performs the exact mutations described**.
- Golden fixtures, diagnostics, and benchmark baselines (per appendix B).
- All per the normative EBNF (appendix D), grade rules (ch. 6/9 + appendix E), OpticSummary / Cursor / CGIR shapes (ch. 8/10), codegen shape (ch. 11), and milestone ladder (appendix C).

**Scope for "complete" (this task):** Narrow v0 only per book ch7/app D (GradedOptic + get/put, lens-like, in-memory SoA, CacheGrade + OwnershipGrade v0, three fusions). Select M7/M8 scaffolding for prism/traversal/tap/record examples present (preview/review or get/put lowering, m7_reserved=false, comment hooks); full traverse/update syntax, profile/replay real, richer grades out of scope. Architecture reserves nodes for later.

**Non-goals for first delivery:** Full project graph / `optic init`, LSP, multicore, native LLVM (ch16), staging (ch14), rich experimental lanes. We implement the *semantic microscope* (Rust backend) described in ch11.

## 1. Key Contracts (narrow v0)

See the book (ch. 7–11 + app. C/D/E) for full rationale. Core artifacts that must remain stable:

- `OpticSummary` (costate/focus, `PathLift`, read/write regions, `ConcreteGrade`, provenance).
- `ConcreteGrade` + `Rational` (CacheGrade sat_add/max; Ownership stronger/disjoint rules per app E).
- `Cursor<'a, S>` (the runtime heart).
- **CGIR**: hand-rolled `Vec<CgirNode>` + u32 ids, `ProductFlat`, provenance (`original_ids`), `resolved_optics` (name-first), `region_map`.
- Three fusions (map → compose → product) with provenance preservation.
- Regions: conservative field roots only.
- Parser: hand-written, `>>>` > `***`, nestable comments, spans everywhere.
- Emission: direct `for id { Cursor; arena.field[id] }` loops + provenance comments (ch11 shape).

**Supported surface (app D + scaffolding):** `data { SoA<...> }`, `optic Name: GradedOptic<...> { get/put }` (plus preview/review and get/put for prism/traversal), `let`, `fn main`, `>>>` / `***`, `query(optic).get()/set(v)/map(...)`, simple expr + blocks + tuples + arith. No traverse/update syntax, no full profile/replay. 

Single `main` costate per program for now. No modules.

**Examples to support (from book + app B layout + post-M6 growth):**
- Core: `health_get.opt`, `health_set.opt`, `health_decay.opt`, `health_position.opt` (product + map), `nested_*`, `compose_*`.
- Complex runtime demos (2026-06-25 + later): `game_entity_sim.opt` (fused multi-col product + let reuse + tap hook), `rich_entity_update.opt`, `reusable_and_taps.opt` (prefix hooks), `mixed_prism_traversal.opt` (scaffolding usage), plus triple/let/tapped/game/pipeline variants, `multi_let_pipeline.opt`, `arith_fusion_pipeline.opt`.
- Negative: `invalid_grade.opt`, `invalid_alias.opt` (stable diagnostics with evidence).
- Direction: more ambitious multi-system "complex applications" exercising fusion, reuse, and M7/M8 scaffolding in realistic patterns.

**Milestone ladder (app C) — gates we will hit with evidence (fixtures, tests, committed baselines):**
- M0: lexer/parser deterministic, recovery, tokens/AST fixtures.
- M1: names resolve, cursors, summaries.
- M2: types + grades + alias (alias checker is "the hardest").
- M3: CGIR + provenance + verifier.
- M4: the three fusions sound + provenance-preserving.
- M5: Rust backend emits compiling Rust; acceptance examples run; runtime complete; **bench baselines committed**.
- M6: prelude "release" (diagnostics stable, fixtures frozen, agent-repair loop conceptually validated via our test harness).

**Repository layout (app B):** We follow it closely at the root of this workspace (the book lives alongside as `book-sources/` + root assembled + LICENSE; the impl is the executable realization of the book).

## 2. Architecture & Technology Choices

- **Host language:** Rust (1.95+ present in env). Natural for systems work, excellent for the "Rust transpiler as semantic microscope".
- **Project structure:** Cargo workspace exactly as appendix B recommends (`crates/optic-*` split). Thin crates early; merge only if it hurts velocity. Top-level `Cargo.toml` workspace + `opticc` bin in `crates/optic-cli`.
- **Parser:** 100% hand-written per ch7 (lexer with longest-match scan for >>>/***, depth-counted nested block comments; recursive descent + binding-power for optic_expr). No generated parser (pest etc. avoided for fidelity to "deterministic recovery" story). Spans: byte offsets + source map or (line, col, len).
- **IR containers:** hand-rolled `Vec<CgirNode>` + u32 `NodeId` (manual indexing in CgirGraph { nodes: Vec<...>, resolved_optics, region_map }) for deterministic ids, easy snapshots. No external IndexVec or graph crate. Keeps provenance trivial. (See updated CGIR desc + actual optic-cgir.)
- **Grades / Rational:** Pure-Rust first (`struct Rational { num: i64, den: u64 }` + gcd reduce + ops). `ConcreteGrade` with sat_add / max. Z3 (`z3` crate) feature-gated for future symbolic / harder inference (libs already installed in env; we can `cargo add z3` later if pure Rust inference proves insufficient for the acceptance suite). Book emphasizes Z3 experience from Granule; we keep the door open.
- **Diagnostics:** Structured `Diagnostic { code: "GRA-104", phase, primary_span, rule, evidence: serde_json::Value, minimal_fix_options, next_commands }`. Human pretty-print + `--json`. Use simple emitter (or ariadne-like if dep added; start with hand-rolled for zero surprise).
- **CLI:** `clap`. Subcommands: `check`, `transpile`, `dump-ast/hir/cgir`, `run` (transpile + compile + execute harness for verification). Also support the explain / doctor style from app B.
- **Testing strategy (golden + execution):**
  - Unit tests inside crates.
  - Integration: `optic-tests` or `tests/` that feed `.opt` from `examples/` / `fixtures/`, compare `dump-*` output to committed `.txt` or `.json` fixtures (update with `optic snapshot-update --confirm` style).
  - Execution tests: for positive examples, transpile → generate temp Cargo project or direct rustc command that links `optic-runtime` (path dep), run the binary, assert side-effects (array state) or stdout.
  - Negative tests: specific diagnostics codes + messages for `invalid_*`.
  - Property: roundtrip or "summary of body matches manual region/grade calc".
- **Runtime crate (`optic-runtime`):** Tiny, readable in <5 min.
  - `Cursor<'a, S>` (new, id accessor, perhaps debug bounds).
  - Re-export or helpers for SoA patterns (but in v0 many examples use raw Vec fields after data lowering).
  - No hidden semantics.
- **Codegen:** String building or `quote`/`proc-macro2` (prefer minimal; start with `write!` + a small pretty emitter for deterministic names + comments). Emit complete `fn main` test harnesses for examples or pure mutation fns + driver.
- **Deps (minimal, justified):**
  - Core: `thiserror`, `serde` + `serde_json` for dumps/diagnostics/fixtures. (No index_vec; hand-rolled Vec<u32 ids> used in CGIR.)
  - CLI: `clap`, `anyhow`.
  - Optional: `z3` (feature `solver`), `num-rational` (or hand-rolled), `insta` or file-compare for golden.
  - Build: keep dev-deps light.
- **Error handling & recovery:** Parser recovers; later phases collect multiple errors where safe (esp. M0/M1).
- **Spans & provenance:** Threaded from day 0 (critical per book — "provenance must be designed into M3, not retrofitted").
- **Release discipline (ch11):** Stable diagnostic codes, deterministic dump output (sorted ids, canonical names), golden fixtures frozen at M5/M6, benchmark baselines (simple wall-time or instruction counts via `criterion` later; start with "example ran successfully + produced expected mutations").

## 3. Risks & Mitigations (from book critical path notes)

- Alias checker (the hard M2 gate): continue exercising `put_reads` hazards and complex products with `invalid_alias.opt`.
- Golden vs runtime parity policy: new "complex" / runtime-focused examples use CGIR + execution harness as the documented baseline; full multi-layer goldens are added only when they provide high value.
- Keeping docs/PLAN in sync on every addition (historical source of drift).

## 4. Tooling & Environment

- Host: Linux. Rust (current stable), cargo.
- Golden updates: `OPTIC_UPDATE_GOLDEN=1 cargo test ...` or `opticc snapshot-update --confirm`.
- For Z3 (future, if needed): feature-gated; pure-Rust arithmetic is sufficient for current acceptance set.
- Keep synthetic N caps modest in tests (N≤12 in hir, N≤8 in cgir integration)
- Optional low-mem probes (manual): `ulimit -v 2000000 cargo test -p optic-syntax -- --ignored` and `ulimit -v 2000000 cargo test -p optic-hir -- --ignored`
- Run harness uses `tempfile::tempdir()` (auto-cleaned per run); `optic-runtime` path is canonicalized and must stay inside the workspace root (symlinks escaping the repo are rejected)
- **Trust boundary:** `opticc run` / `optic bench` invoke `cargo` in an isolated env (temp `HOME`/`CARGO_HOME`, fixed tool `PATH`) but **do not** use OS-level sandboxing (namespaces/seccomp). Intentional for trusted-local dev/CI — treat untrusted `.opt` as `check` only
- Cargo stderr from the run harness is redacted by default (`--verbose` for full output)

**Implementation notes (ch8–ch11):** `Arc<OpticSummary>` for cheap sharing; `dedup_regions` O(n) on small region sets; compose produces new summary data per ch8.9.5.1 (unavoidable clone once per composition).

## 6. Execution Order & Verification

We will implement bottom-up with continuous verification:
1. Scaffolding + lexer/parser (M0 gate) + first examples parse.
2. HIR + summaries (M1) + dumps.
3. Typeck + grades + alias (M2) + negative tests pass with evidence.
4. CGIR + verifier (M3).
5. Fusions (M4) + post-fusion dumps.
6. Codegen + runtime + execution harness (M5) — **this is the "fully working code" milestone**.
7. Polish to M6.

At each phase: `cargo check`, relevant unit/golden tests, manual run of CLI on examples, commit fixtures when stable.

After M5/M6: workspace is self-contained runnable per narrow v0; `opticc run` on examples shows fused execution.

## 7. Open Questions for User (will be resolved conservatively or asked)

- Exact set of primitive types / Vec2 etc. in v0 surface (book uses f32, Vec2 — we will introduce a small prelude or hardcode a couple for examples).
- How much of "fn" bodies and full expr language (we will implement a sufficient subset for the acceptance examples + simple arith/blocks).
- Whether to make the emitted Rust always a complete bin with `main` that hardcodes example data + runs + prints (easiest for "run" verification) or pure fns.
- Initial diagnostic code catalog (we will seed GRA-*, ALI-*, PAR-*, TYP-* from the book examples and extend).
- Any preference on extra crates (e.g. `ariadne` for beautiful errors vs hand-rolled)?

This plan is derived directly from the book (specific chapter/appendix references above). Implementation will cross-reference the book in code comments and the final `docs/`.

## 8. Current progress (as of 2026-06-25 + this delta)

Narrow v0 (M0–M6) complete. Realizes book ch.7-11 + M7/M8 scaffolding per app C/D/E. Validated via tests/CLI/goldens.

| Milestone | Status | Evidence |
|---|---|---|
| M0 lexer/parser | **done** | Hand-written RD+Pratt (ch7); `>>>`/`***`; nestable comments; recovery; depth=512; goldens tokens/ast. |
| M1 HIR + summaries | **done** | Cursor, resolution, OpticSummary; hir goldens. |
| M2 types/grades/alias | **done** | Inference + GRA/ALI/TYP diags; typeck. |
| M3 CGIR + verifier | **done** | Vec<CgirNode>+u32 ids, provenance, region_map; cgir pre/post + --check. |
| M4 fusions | **done** | map→compose→product; ProductFlat; FUS diags. |
| M5 Rust backend + run | **done** | ch11 emit + harness (CGIR+exec for complex per carve-out). |
| M6 release polish | **done** | Diags/goldens/CLI stable. |

**Diagnostic catalog (v0 implements book subset; full catalog in app A):**
- GRA-110, GRA-104, ALI-201, TYP-001/002/003/004/010, EXP-001, PAR-001 (PAR-010+ reserved), FUS-501/502, CGI-003/006, OBS-701/702, RES-001 etc. (see v0-executable-spec.md + code for exact; book has additional PAR-*/OPT-*/etc reserved for M7+)

**Positive examples** now include core set + ambitious "complex" runtime demos (bench order): `game_entity_sim.opt`, `mixed_prism_traversal.opt`, `reusable_and_taps.opt`, `rich_entity_update.opt`, `triple_product_fusion.opt` (3-arity ProductFlat), `let_reuse_pipeline.opt`, `tapped_multi_system.opt`, `game_loop_pipeline.opt`, `multi_system_fusion.opt`, `multi_let_pipeline.opt`, `arith_fusion_pipeline.opt`, `tuple_fusion_pipeline.opt` (12 total). Newer have CGIR+execution parity (carve-out); lists kept in sync.

**M7 prism lowering (scaffolding; narrow-v0 core complete):**

| Item | Status | Notes |
|------|--------|-------|
| CGIR M7/M8 reserved variants | scaffolding | `PrismLeaf`/`TraversalLeaf` (M7, `m7_reserved=false`); `Tap`/`Record` (M8); **CGI-006** only for true stubs. Full traverse/update syntax deferred. |
| GradedPrism / GradedTraversal usage | scaffolding (narrow) | Exercised in realistic programs via separate queries/lets (`alive_filter`, `all_healths`, `mixed_prism_traversal`). |

**M8 observability scaffolding (stubs + surface gates for narrow; full M8 deferred):**

| Item | Status | Notes |
|------|--------|-------|
| `.tap` / `.record` surface | scaffolding (narrow) | Lowered to comment hooks. Prefix-only. |
| OBS-701/702/703 | scaffolding (narrow) | `.profile`/`.replay` + trailing hooks rejected. |

**Bench baselines** exist in `fixtures/bench/`. Runtime examples intentionally lack full timing baselines (carve-out).

**New / Next Goals (post 2026-06-25 complex-examples task)**
- [done this delta: prior] Added 3 new ambitious runtime examples (triple/let_reuse/tapped); CGIR+run+bench+verify+golden+harness+sync.
- [done this delta] Added multi_let_pipeline.opt (chained let + tap + tuple arith + untouched col + marker); CGIR+run_*+verify+bench+golden+lists synced.
- [done this delta] Added arith_fusion_pipeline.opt (next game-loop/pipeline style exercising more tuple/arith/3-arity/untouched cols/let/fusion + marker) + run_* (richer asserts + boundary) + self-host markers to 2 more .opt + evidence + verify + golden_cgir + bench/PLAN/docs syncs (11 total; order from bench_examples).
- [done this delta] Added tuple_fusion_pipeline.opt + run test + CGIR + registrations + lists bump to 12; updated docs/PLAN with automation note for sync.
- [done this delta] Compacted M5 text (milestone table + section 8 high-level trim); extended parse asserts for arity; synced all lists (fixtures/README "12 total", docs, PLAN).
- Grow suite further with game-loop/pipeline apps stressing fusion/let/tuple/scaffolding (use only narrow surface).
- Expand execution coverage + harness (full runtime N=0 entities exec, more arity edges, richer asserts).
- Add more self-host prep markers/comments using extern in examples (comment only).
- Keep PLAN/docs/goldens/code in sync (no drift) for future additions; accurate .opt comments; runtime-focused CGIR+exec policy.

**Immediate next items**
- Grow suite with yet another complex pipeline/game-loop .opt exercising additional tuple/arith/untouched patterns via narrow surface only (tuple_fusion_pipeline added this delta).
- Complete explicit runtime arity-mismatch positive + negative test coverage (build on cgi005 + new 3-arity run_* ; add dedicated mismatch harness cases; indirect via 3-arity + cgi005 this delta).
- Implement full runtime N=0 entities exec (harness update + boundary test; see PLAN immediate next + fixtures/README carve-out (N=0 codegen deferred; synthetic + boundary only; no NOTE yet); full N=0 .opt runtime open).
- Further expand self-host prep markers (commented extern) to additional .opt + add evidence asserts in run_* (markers on let/rich + new this delta).
- Extend verify_example_stdout + parse_*_boundary for additional arity edges (e.g. 4-col products; parse extended with 3/4-col note + arith sample this delta).
- Add note/automation hint for keeping bench_examples / verify / golden_cgir / docs lists in sync on future adds (added in main.rs comment + const-like; see also test list consistency).
- Re-run full fmt/clippy + execution/golden + opticc spot-checks after all changes (always required; done).
- Further compact PLAN (high-level only; prune any residual M5/M6 verbose parentheticals in next pass).
- (list order unification + 12 total sync done this delta + doc syncs; see bench_examples order as source of truth; automation note added).

**See also (kept in sync):** `README-IMPLEMENTATION.md`, `docs/v0-executable-spec.md`, `fixtures/README.md`.

## 9. Post-M7 roadmap

- Full M7 (traverse/update syntax, richer prism branches, real SIMD/AVX) — book ch13.
- Real M8 observability (profile/replay with injection + erasure, non-stub hooks) — book ch14.
- Host/foreign boundary lowering (beyond current HIR carry + TYP-010 gate; prep for M9 self-host per ch22/app I/app F).
- Richer grades, LLVM/TBAA, multicore/NUMA, full unsafe/FFI (ch15–18).
- M9 self-hosting with translation-validation harness.
- M10 kernel-class + domain playbooks (Part IV) + full rationale (Part V).

### M7 notes (scaffolding state)

Current scaffolding gives working preview/review + get/put paths for the prelude examples. When doing full M7:
- Add traverse/update surface + HIR lowering (per book/app D update).
- Extend codegen for branches + real intrinsics; lift m7_reserved where appropriate.
- Update verifier/fusions as needed while preserving narrow compatibility.
- Keep "wontfix for narrow" distinctions explicit.

**Intentional narrow debt (until M8+):**
- Triplicate leaf match arms in codegen (**wontfix** until more variants).
- CgirGraph cloning for probes (**wontfix**; fine at current scale).

*This PLAN.md lives at the root. Update it (smallest precise edits) as implementation reveals book ambiguities or better conservative choices. Keep narrow v0 vs M7+ distinctions per app C. Reassemble book sources only if editing the manuscript itself.*

## Live git capture at end of this delta write (for summary + PLAN sync per task)
# (re-captured final via `git status --porcelain`, `git diff --stat HEAD -- [this-delta files]`, `git ls-files --others` post checkout unrelated)
# This delta tracked (limited): edits to PLAN.md (compact/sync/[done]), main.rs (bench+verify), execution.rs (run_*+marker asserts+parse extend+notes), golden_cgir.rs (regs+comment), docs/v0-executable-spec.md, fixtures/README.md, examples/let_reuse_pipeline.opt + rich_entity_update.opt (markers). Untracked (per policy): arith_fusion_pipeline.opt + cgir pre/post.
# Accum/prior: M game_loop+triple (prior markers vs HEAD; this-delta only touched let/rich), untracked multi_let* + its cgir (prior policy), large PLAN stat (base content vs old HEAD). See summary for verbatim captures.
