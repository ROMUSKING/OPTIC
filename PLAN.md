# Optic Narrow v0 Compiler — Complete Implementation Plan

**Source of truth:** The Optic Language Implementation Book (split sources in `book-sources/`, assembled in `book-sources/assembled.md`; appendices C/D/E normative for milestones/EBNF/grades).

**Goal:** Deliver a *fully working*, executable narrow-v0 compiler (and supporting runtime) that meets the book's M0–M6 milestone gates for the prelude (app C). M7+ (prisms/traversals full syntax, observability profile/replay real) deferred per book. Output includes:
- A usable `opticc` CLI (and library API).
- End-to-end: `.opt` source → parse → HIR + summaries → type/grade/alias check (with good diagnostics) → CGIR (with provenance) → the three fusion rewrites → readable, correct Rust emission.
- Tiny `optic-runtime` (Cursor + SoA support).
- Acceptance examples that **compile to Rust, the emitted Rust compiles and runs, and performs the exact mutations described**.
- Golden fixtures, diagnostics, and benchmark baselines (per appendix B).
- All per the normative EBNF (appendix D), grade rules (ch. 6/9 + appendix E), OpticSummary / Cursor / CGIR shapes (ch. 8/10), codegen shape (ch. 11), and milestone ladder (appendix C).

**Scope for "complete" (this task):** Narrow v0 only per book ch7/app D (GradedOptic + get/put, lens-like, in-memory SoA, CacheGrade + OwnershipGrade v0, three fusions). Select M7/M8 scaffolding for prism/traversal/tap/record examples present (preview/review or get/put lowering, m7_reserved=false, comment hooks); full traverse/update syntax, profile/replay real, richer grades out of scope. Architecture reserves nodes for later.

**Non-goals for first delivery:** Full project graph / `optic init`, LSP, multicore, native LLVM (ch16), staging (ch14), rich experimental lanes. We implement the *semantic microscope* (Rust backend) described in ch11.

## 1. Analysis Summary (from book-sources/)

**Core artifacts (must exist and be stable):**
- `OpticSummary` (ch8): costate, focus, `lift: PathLift`, `get_reads / put_reads / put_writes: Set<Region>`, `get_grade / put_grade: ConcreteGrade`, determinism, serializable, provenance, (later boundary).
- `ConcreteGrade { cache: u8 (255=∞), ownership: OwnershipDim { share: Rational, read_only: bool, must_use: bool } }`.
- Surface aliases: `LinearGrade`, `AffineGrade`, `SharedGrade`, `CacheGrade<N>`, `OwnershipGrade<r>`, `_` (infer).
- `Cursor<'a, S> { arena: &'a mut S, id: usize }` — the operational heart (ch5/8/11).
- **CGIR** (ch10): `CgirGraph { nodes: Vec<CgirNode>, roots, provenance_index, resolved_optics, region_map }` (hand-rolled u32 NodeId indexing for determinism; no external IndexVec crate).
  Nodes (core v0 + select M7/M8 scaffolding): `OpticLeaf`, `Compose`, `Product`/`ProductFlat`, `Query*`, `FusedLoop`, `PrismLeaf`/`TraversalLeaf` (m7_reserved=false for supported), `Tap`/`Record` (m7_reserved=false for tap/record).
- **Three fusions** (fixed-point driver, provenance-preserving):
  1. Map fusion (chained pure `.map` collapse).
  2. Compose fusion (intermediate focus does not escape → single loop body + temps).
  3. Product flattening (normalize `A *** B` for codegen).
- **Grade rules** (ch6/9, app E):
  - Cache seq: `sat_add` (u8).
  - Cache prod: `max`.
  - Ownership seq: stronger (max share, or must_use, and read_only?).
  - Ownership prod: structural disjoint OR (read-only and shares ≤1) OR same partition family + shares ≤1. Else reject.
- Regions for v0: conservative field roots (e.g. "healths", "positions") normalized from `s.foo[s.id]` etc. No index-symbolic analysis.
- **Alias checker** (ch9): the hardest M2 gate. Must reject `invalid_alias.opt` cases involving `put_reads` hazards even when no direct write overlap.
- **Parser** (ch7 + app D): Hand-written recursive-descent + binding-power (Pratt-style) for optics. Longest-match for `>>>` / `***`. Nestable `{- -}` comments. Recovery to sync points. Spans on everything. Precedence: `>>>` tighter than `***`.
- **Codegen target shape** (ch11): 
  ```rust
  for id_0 in 0..entities.len() {
      let cursor_0 = optic_runtime::Cursor::new(&mut entities, id_0);
      // direct field accesses via cursor_0.id
      // provenance comments: // optic(fused): [HealthView, PositionView]
  }
  ```
  No iterators/adapters in hot path, deterministic names, readable.

**Surface (normative EBNF appendix D, ch7):**
- `data Foo { f: SoA<T>, g: SoA<U> }` (costate layouts; SoA<T> is surface marker → Vec<T> columns in Rust).
- `optic Name: GradedOptic<S, A, G> { get s => ...; put (s,v) => ... }`
- Composition `A >>> B`, `A *** B`.
- `let x = ...;` , `fn ...`.
- `costate.query(optic).get() / .set(v) / .map(|x| e)`.
- Simple expr: field/index, assign, block, binary arith, lits, idents. (Enough for the prelude examples.)

**Examples to support (from book + app B layout):**
- `health_get.opt`, `health_set.opt`, `health_decay.opt`, `health_position.opt` (product + map), `nested_...`.
- Negative: `invalid_grade.opt`, `invalid_alias.opt` (must produce stable diagnostics with evidence).
- Simple game-loop style SoA update.

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

**Gaps / conservative choices we will document in code + PLAN updates:**
- Regions are field-root strings (conservative `[*]` normalization). No dependent index analysis.
- No full symbolic solver in v0 (pure arith + simple fraction check for alias).
- Single `main` costate per program for simplicity in first examples (multiple data decls supported but queries target one).
- No module system (all in one file for narrow prelude).
- `SoA<T>` lowers to `Vec<T>` column in the struct (user data decl becomes `struct Entities { healths: Vec<f32>, ... }`).
- Grade inference: bottom-up from body regions + composition rules (support `_` and partial).
- For "run": we will provide a test harness generator so emitted code is self-contained enough to `rustc` directly against the runtime source, or we build a temp crate. (Avoids complex linking.)

## 3. Phased Implementation Plan (aligned to milestones)

**Phase 0 — Scaffolding & Tooling (M0 prep) — 1-2 "days"**
- Top-level `Cargo.toml` (workspace) + per-crate `Cargo.toml` (libs + cli bin).
- `optic-runtime` skeleton (Cursor + basic SoA example struct).
- `optic-diagnostics` (Diagnostic struct + emitter + codes catalog started).
- `optic-syntax`: Span, Token, TokenKind, SourceMap. Hand-written lexer (exact longest-match per ch7.9). Basic tests.
- `examples/`: seed 4-6 `.opt` files (positive + 2 invalid) transcribed from book snippets + app B.
- `fixtures/`: initial empty golden dirs + a README describing update process.
- `optic-cli`: skeleton with clap "check" / "transpile" / "dump-tokens".
- CI-like: `cargo check`, `cargo test --workspace` runnable immediately.
- Run `cargo build` verification.

**Phase 1 — M0: Lexer + Parser + AST (deterministic)**
- Full lexer (all tokens from ch7 table, nested comments with depth counter, longest-match >>> / ***).
- Recursive descent parser + `parse_optic_expr` binding power (>>> lbp higher).
- AST types mirroring EBNF (DataDecl, OpticDecl with GradedOpticType, OpticExpr with Seq/Par, QueryChain, etc.).
- Error recovery to sync points (top-level items, optic get/put, etc.).
- `optic dump-tokens` / `dump-ast` (or via CLI) + committed fixtures.
- Acceptance: parse every example + the normative grammar cases; one-pass error collection.
- Test: golden token streams and pretty AST for `health_position.opt` etc.

**Phase 2 — M1: HIR + Names + Summaries + Cursors**
- HIR lowering: `HirOptic` (Named/Compose/Product), `HirQuery` (Get/Set/Map), expr lowering with explicit cursors.
- Name resolution (locals > optics > data > builtins). Deterministic order (ch8).
- Cursor lowering table (s.field[s.id] → cursor.arena.field[cursor.id]; s → cursor.arena, etc.).
- `OpticSummary` construction from optic bodies (extract reads/writes via a simple region walker on the lowered get/put bodies).
- `PathLift` (simple for field projections in v0).
- Summary composition for `>>>` and `***` (union + lift rules from ch8).
- HIR dump stable. Golden fixtures.
- `optic dump-hir file.opt`.

**Phase 3 — M2: Type Checking, Grade Inference, Alias Safety (the hard gate)**
- Type universe: primitives (i32/u32/f32 etc for v0), tuples, `SoA<T>` (surface), user data types (from data decl), `GradedOptic<S,A,G>`.
- Simple expr typeck in bodies + query contexts.
- Grade representation + ops (sat_add, max, "stronger", fraction add/check ≤1).
- Grade inference: bottom-up from body region counts (cache = |distinct reads ∪ writes| sat), ownership from annotations or default Affine-ish.
- Support `_` elision (infer exactly) and partial.
- Alias checker: exact sketch from ch9 (effective effects, overlapping regions via structural field-root match, read-only share sum ≤1 or partition (v0: we treat same-costate product as potential partition; start conservative)).
- Must pass `invalid_alias.opt` (put_reads hazard etc.) and produce `ALI-*` or `GRA-*` diagnostics with evidence (regions involved).
- Type/grade/alias errors collected; `optic check` reports them nicely.
- Stable diagnostic codes (GRA-*, ALI-*, TYP-*).
- Golden `fixtures/diagnostics/`.

**Phase 4 — M3: CGIR + Provenance + Verifier**
- CGIR builder from typed HIR (bottom-up).
- `CgirGraph`, `NodeId`, `CgirNode` variants (OpticLeaf with lowered get_fn/put_fn as CgirExpr using cursor forms, Compose/Product, Query*, FusedLoop reserved).
- Every node carries provenance (span + original source optic ids).
- `CgirGraph` invariants (unique ids, focus/costate wiring for compose, shared costate for product, alias_safe on Product before codegen, etc.).
- `optic dump-cgir [--before-fusion] [--node NAME|N] [--check]`.
- Verifier pass that fails loudly on violation.
- Golden pre-fusion CGIR fixtures.

**Phase 5 — M4: The Three Fusions (sound + provenance)**
- Fixed-point driver (≤8 iters): map_fusion, compose_fusion, product_flatten. Re-verify after each.
- Map fusion: pure QueryMap chains over same root → single map.
- Compose fusion: sequential where intermediate does not escape the combined put/get → fused body expr with temps; provenance union.
- Product flattening: `(A *** B) *** C` etc. canonicalized; multi-child products for codegen.
- `FusedLoop` nodes carry `original_ids` + reason.
- Soundness: we will write a short comment/doc in code + test that the rewrite preserves get/put semantics (or use the emitted Rust + before/after state comparison as evidence).
- Post-fusion CGIR golden + `--check` must pass.
- `optic dump-cgir` (post) .

**Phase 6 — M5: Rust Backend + Runtime + Acceptance Execution**
- `optic-codegen-rust`: walk (fused) CGIR, emit the exact loop shape from ch11 (one `for id_N`, `Cursor::new`, direct `arena.field[id]`, provenance `//` comments, deterministic temps `_h`, `_p_new` etc.).
- Handle single lenses, seq composition (intermediates or fused), products (multiple loads/stores in one iteration).
- Lower data decls to `struct Name { col: Vec<T>, ... }` (SoA columns).
- Support simple bodies (arith in maps, blocks in put).
- `optic-runtime` complete: `Cursor`, perhaps a tiny `Soa` view or just docs that raw Vecs are used. Make emitted code `use optic_runtime::Cursor;`.
- CLI `transpile file.opt -o out.rs`.
- **Execution verification harness** (in tests or `optic run`):
  - For each positive example, generate or use a driver that constructs the SoA data, "calls" the logic (via the emitted main or by having the transpile also emit a `pub fn run_example(world: &mut Entities)`), runs it, asserts the mutation happened exactly as the optic body described (e.g. healths updated by -10, positions shifted, no alias corruption).
  - Compile step: use `std::process::Command` with `rustc` (or temp `cargo` project with `[dependencies] optic-runtime = { path = "..." }` + the emitted .rs as bin). Capture success + output.
- Commit benchmark baselines (even if trivial "example X completed in Y wall ms" or just "ran successfully"; simple timing of the harness).
- `optic check` / `doctor` basic.
- All M0–M4 fixtures + new rust/ + bench/ fixtures green.

**Phase 7 — M6 Polish + "Release" Artifacts**
- Stabilize all diagnostic output (codes, phrasing, evidence shape).
- Freeze fixtures (document the snapshot process).
- More examples (at least the full set from app B).
- CLI polish for the commands in app B (explain-grade skeleton, dump-summary, etc.).
- `optic bench` stub that runs the harnesses and compares to baselines (print "within tolerance" or diff).
- Documentation in `docs/v0-executable-spec.md` (cross-ref book chapters) or inline.
- Workspace `cargo test --workspace` + `cargo run -p optic-cli -- check examples/*.opt` all pass.
- Update root README or add `README-IMPLEMENTATION.md` summarizing how the code realizes the book.
- (Optional but nice) A single "hello" end-to-end demo script: `cargo run -p optic-cli -- run examples/health_position.opt` that shows before/after + "fused loop emitted".

**Later phases (out of first delivery but prepared for):** M7+ (prisms etc.), self-hosting (M9), kernel (M10). The split crates + rich summaries + provenance + CGIR reservation make extension mechanical.

## 4. Risks & Mitigations (from book critical path notes)

- Alias checker false negatives (esp. put_reads): Prioritize `invalid_alias.opt` + exhaustive structural cases early in M2. Conservative rejection is acceptable.
- Provenance retrofits: Thread `SourceSpan` + original optic ids on *every* node from the first CGIR builder.
- Benchmark baselines: Commit even trivial ones at M5 (before any "green" claims).
- Grade arithmetic off-by-one at boundaries: Unit-test the sat_add, fraction ≤1, and composition tables exhaustively. Use the book's "sat_add" and "stronger" prose exactly.
- Parser tokenization drift: Golden token fixtures + property "every pretty-printed AST re-parses identically".
- Emitted Rust not zero-cost / readable: The codegen phase will be reviewed against the exact shapes in ch11. If it uses iterators or extra allocs, fix upstream (HIR/CGIR) not the emitter.
- "Working code" verification: Every positive example must have an execution test that actually invokes rustc (or cargo) on the output and observes the mutation. No "it typechecks" hand-waving.

## 5. Tooling & Environment

- Host: Linux (current). Rust 1.95, cargo, llvm-config, libz3-dev already present (from initial `run_terminal_command`).
- No python for core (only book assembly if we touch `book-sources/`).
- To add crates: `cargo add ...` inside the relevant crate dir or from workspace root with `-p`.
- For Z3 later (if chosen): `cargo add z3 --features=static-link-z3` or system (libs present).
- Verification commands will be added to the plan execution (e.g. `cargo test -p optic-syntax --test parser_golden`).

## Hardware limits (constrained environments)

Minimum **2 GB RAM** recommended for `rustc` and the `opticc run` verification harness.

**Verification discipline:**
- Single-crate checks: `cargo check -p <crate> --quiet`
- Sequential tests: `cargo test -p <crate> -- --quiet` (one crate at a time; avoid full-workspace parallel builds)
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

After full M5/M6: the workspace contains a self-contained, runnable, tested realization of the narrow v0 spec. Users (and future agents) can `cargo run -p optic-cli -- check examples/health_position.opt` and `... run ...` and see real fused Rust execute the optic semantics.

## 7. Open Questions for User (will be resolved conservatively or asked)

- Exact set of primitive types / Vec2 etc. in v0 surface (book uses f32, Vec2 — we will introduce a small prelude or hardcode a couple for examples).
- How much of "fn" bodies and full expr language (we will implement a sufficient subset for the acceptance examples + simple arith/blocks).
- Whether to make the emitted Rust always a complete bin with `main` that hardcodes example data + runs + prints (easiest for "run" verification) or pure fns.
- Initial diagnostic code catalog (we will seed GRA-*, ALI-*, PAR-*, TYP-* from the book examples and extend).
- Any preference on extra crates (e.g. `ariadne` for beautiful errors vs hand-rolled)?

This plan is derived directly from the book (specific chapter/appendix references above). Implementation will cross-reference the book in code comments and the final `docs/`.

## 8. Current progress (updated 2026-06-19, prelude closure)

| Milestone | Status | Evidence |
|---|---|---|
| M0 lexer/parser | **done** | Recovery fixed; goldens `fixtures/tokens/`, `fixtures/ast/` (positive + negative incl. `unsupported_prism`, `unsupported_traversal`, `host_boundary`, `compose_triple`); `MAX_PARSE_DEPTH=512`; parser hang regression test; prism/traversal/unsafe/extern surface parsed (`GradedPrism` + `GradedTraversal` lowered in M7; TYP-010 for unsafe/extern only) |
| M1 HIR + summaries | **done** | Tuple/`TupleProj`; HIR map-chain fusion + multi-param guard; `Arc<HirExpr>` map bodies shared to CGIR; **OpticSummary costate/focus from decl**; HIR goldens for all positive examples incl. `compose_triple` (`fixtures/hir/`); `cargo test -p optic-hir golden_hir` |
| M2 types/grades/alias | **done** | ch9.9.3 inference; GRA-110/GRA-104/ALI-201 with `related_spans`; **TYP-010** for unsafe/extern; prism/traversal typeck via preview/review or get/put; `check` runs CGIR+verify+codegen dry-run |
| M3 CGIR + verifier | **done** | `resolved_optics` alias map; reachability GC through query→optic spine; **`dump-cgir --node NAME\|N`** (name via `resolved_optics`, then numeric id); early **CGI-003** for unsupported optic bodies in compose chains; compose wiring uses **summary** focus/costate; unreachable materialized `FusedLoop` flagged; `dump-cgir --check`; CGIR goldens incl. `health_get`/`health_set` pre+post |
| M4 fusions | **done** | ch10 order map→compose→product; map fusion; compose body rewrite; nested compose chain fusion; **`ProductFlat` materialization** (nested+leaf products rewritten in-place; provenance `ProductFlattening`; verify invariants); `intermediate_escapes_query`; FUS-501/FUS-502 |
| M5 Rust backend + run | **done** | `RegionMap` from data decls threaded via `CgirGraph`; nested compose with `FocusField` put spine; `nested_position.opt` end-to-end; `fixtures/rust/` + `fixtures/bench/` incl. nested_position; codegen returns `Err` for unknown regions; `region_bind`/`column_init` derive from `ColumnInfo` (custom record defaults remain fixture-driven for harness init only) |
| M6 release polish | **done** | ... ; 2026-06-20: added debug_assert/guards + sync (see § robustness pass); no behavior change on valid paths. M0-M6 core complete per app C. |

**Diagnostic catalog (v0 implements book subset; full catalog in app A):**
- GRA-110, GRA-104, ALI-201, TYP-001/002/003/004/010, EXP-001, PAR-001 (PAR-010+ reserved), FUS-501/502, CGI-003/006, OBS-701/702, RES-001 etc. (see v0-executable-spec.md + code for exact; book has additional PAR-*/OPT-*/etc reserved for M7+)

**Positive examples** use `CacheGrade<2>` for single-field get+put lenses (inferred cache = sat_add(1,1) = 2).

**Completed gates (2026-06-19):**
1. **Product flatten materialization** (BUG-001): `ProductFlat` CGIR node; `product_flatten` rewrites nested/leaf products; codegen `collect_regions_from_node` + verify invariants; `health_position` post-fusion golden updated (no provenance-only `FusedLoop`)
2. **PathLift + nested field paths** (SUG-004): `PathLift.prefix` + `is_subregion` dotted lattice; seq/par summary lift; `HirExpr::FocusField`; `nested_position.opt` + full golden parity (tokens/ast/hir/cgir/rust/bench)
3. **Region→field mapping** (SUG-003): `RegionMap` from `data` decls on `CgirGraph`; codegen uses structured `lookup_region_*` returning `Err`; record types emitted for nested structs

**Completed M6 (2026-06-19; narrow v0):**
1. `opticc explain-grade file.opt --node NAME [--json]` — declared vs inferred cache/ownership + regions
2. `docs/v0-executable-spec.md` — executable spec cross-referencing M0–M6, CLI, diagnostics, fixture workflow
3. `crates/optic` facade — `parse`, `lower`, `check`, `build_cgir`, `optimize`, `emit_rust`, `compile_*`, `Diagnostic`
4. TYP-001/002/003 catalog + `examples/typ*.opt` + `fixtures/diagnostics/typ*.json`
5. Property/smoke tests: parse→lower idempotence + summary regions ⊆ declared columns (`crates/optic`)

### Prelude-complete summary (M0-M6 narrow v0; 2026-06-20)

Appendix B (extended examples for scaffolding): **`alive_filter.opt`** (M7 prism e2e via preview), **`all_healths.opt`** (M7 traversal e2e via get/put), **`tap_health.opt`** etc. (M8 obs hooks); negatives for OBS/TYP/ GRA. (Note: narrow v0 EBNF per app D is GradedOptic get/put only; prism/traversal surface is extension scaffolding.)

New CLI / facade commands (M6 + scaffolding):
- `opticc explain-focus file.opt --node NAME [--json]` — PathLift prefix, root-path, focus_fields
- `opticc dump-summary file.opt --node NAME|N` — optic/let **name** lookup (name before numeric id)
- `opticc dump-cgir file.opt --node NAME|N` — optic/let **name** lookup via `resolved_optics` (name before numeric id)
- `opticc doctor [file.opt]` — toolchain check; optional per-file `check`
- `opticc bench [file.opt] [--update]` — all (core) examples or single-file harness
- `opticc profile` / `replay` — OBS-701 surface stubs (narrow)

### M7 prism lowering (status: scaffolding; narrow-v0 core complete)

| Item | Status | Notes |
|------|--------|-------|
| CGIR M7/M8 reserved variants | scaffolding | `PrismLeaf`, `TraversalLeaf` (M7, m7_reserved=false for e2e like alive/all); `Tap`/`Record` (M8, m7_reserved=false for tap/record); **CGI-006** for true stubs; **OBS-701** for profile/replay. Full M7 per book (incl. traverse/update syntax) deferred beyond narrow. |
| `dump-cgir --node NAME\|N` | **done** | `resolve_cgir_node`; name-before-numeric; unknown name → EXP-001; unknown id → `node id N not found` |
| Appendix B doc stubs | scaffolding | `docs/observability-v0.md`, `docs/effect-coeffect-v0.md` (M7/M8) |
| `opticc explain TYP-010` / `CGI-006` | **done** | Enriched catalogs; prism no longer TYP-010 |
| Structured CGI-006 wiring | scaffolding (narrow) | `verify_to_diagnostic` on optimize + verify + `dump-cgir --check`; stub `PrismLeaf` still CGI-006 |
| GradedPrism HIR → CGIR → codegen | scaffolding (narrow) | `alive_filter.opt` e2e via preview; `PrismLeaf` with `m7_reserved=false` passes verify (full M7 syntax deferred per book) |
| GradedTraversal lowering | scaffolding (narrow) | `all_healths.opt` e2e; v0 surface uses get/put (book traverse/update deferred per app D); `TraversalLeaf` with `m7_reserved=false` passes verify; `// simd-eligible` metadata bridge |
| `compose_field_access.opt` | **wontfix** | whole-column `s.healths` get body rejected at typeck (**TYP-004**); CGI-003 preserved for CGIR compose-chain bodies (per book narrow EBNF + ch7) |

**Next iteration priorities (narrow v0 scope only):**
1. traverse/update surface syntax + full AVX intrinsics bridge — **wontfix for narrow v0** (book ch13, app D EBNF: narrow uses get/put for GradedTraversal; full syntax deferred to M7+)
2. profile/replay observability CLI + grade-controlled erasure passes — stubs present (OBS-701 gate, no-op fns, CLI delegates to check); full per book M8, outside narrow v0 scope (wontfix here; see docs/observability-v0.md)

### M8 observability scaffolding (status: stubs + surface gate for narrow v0; full M8 deferred)

| Item | Status | Notes |
|------|--------|-------|
| `.tap("label")` / `.record("event")` surface | scaffolding (narrow) | Query-chain methods; parser + AST `QueryMethod` variants; lowered to comment hooks only (full runtime M8) |
| HIR → CGIR lowering | scaffolding (narrow) | `ObsHook` on `HirQuery` → `Tap`/`Record` with `m7_reserved=false` |
| `verify` / `is_allowed_m7_node` | scaffolding (narrow) | Lowered Tap/Record pass verify; stubs still **CGI-006** |
| Codegen comment hooks | scaffolding (narrow) | `// optic(tap):` / `// optic(record):`; inner optic still runs |
| **OBS-701** diagnostics | scaffolding (narrow) | `.profile`/`.replay` rejected; `unsupported_profile.opt` / `unsupported_replay.opt` + JSON witnesses |
| **OBS-702** diagnostics | scaffolding (narrow) | Trailing `.tap`/`.record` rejected; `trailing_tap.opt` / `trailing_record.opt` + JSON witnesses |
| Hook-string policy | scaffolding | Single-line ASCII labels; parse-time validation — see `docs/observability-v0.md` §Hook string policy |
| Structural limitations | scaffolding | Prefix-only hooks, orphan CGIR nodes, FusedLoop skip — see `docs/observability-v0.md` §v0 structural limitations |
| Examples + goldens | partial (per design) | Per-example inventory: `tap_health.opt` + `record_health.opt` (full tokens/ast/hir/cgir/rust/bench); `tap_record_chain.opt` (tokens/ast/hir/cgir/rust, no bench); `compose_tap.opt` (cgir/rust only) — intentional per fixtures/README (narrow scope) |

**Done (prior rounds + gates):** compose body rewrite + equivalence; nested compose chain fusion/codegen; FUS-501/FUS-502; whole-column reject preserved (TYP-004 at typeck for `compose_field_access.opt`, CGI-003 at CGIR for compose-chain bodies); `original_ids` superset documented in `fixtures/README.md`

## 9. Post-M7 roadmap

- ~~Lower `GradedPrism` from typed HIR into `PrismLeaf` + Rust codegen~~ (scaffolding — `alive_filter.opt`; full syntax M7+)
- ~~Lower `GradedTraversal` from typed HIR into `TraversalLeaf` + entity-loop codegen~~ (scaffolding — `all_healths.opt`; v0 get/put only per app D — not traverse/update)
- Host/foreign boundary lowering for `unsafe optic` / `extern` (HIR prep done 2026-06-20; gate+diags+sanit kept; no golden)
- traverse/update surface syntax + full SIMD intrinsics bridge (beyond v0 comment metadata) (SIMD metadata+debug hardened; syntax **wontfix narrow** per book ch13/app-D — narrow uses get/put clauses for traversals)
- ~~Observability tap/record scaffolding (M8)~~ (tap/record comment hooks in narrow; profile/replay **OBS-701** stubs per M8 deferral)
- profile/replay CLI + runtime hooks — see `docs/observability-v0.md` (stubs+CLI; full M8 outside narrow v0)

### M7 codegen touch list (`optic-codegen-rust`) — scaffolding for extension examples (narrow core M5 complete)

- `collect_regions_from_node` — `PrismLeaf` / `TraversalLeaf` summaries
- `detect_query_mode` — query-wrapped M7 leaves (map/get/set)
- `emit_leaf_get` / `emit_leaf_put_value` / `emit_leaf_put_stores` — prism preview/review + traversal get/set
- `emit_prism_query_*` / `emit_traversal_query_*` / `emit_traversal_map_decay`
- `emit_compose_chain_loop` — rejects prism/traversal in compose spines
- `emit()` root driver — emits `// optic(tap|record):` hooks for lowered Tap/Record

### M7 structural debt (intentional until M8)

Round-3 dedup review — items fixed where trivial; remainder documented here:

| Item | Status | Rationale |
|------|--------|-----------|
| OpticLeaf / TraversalLeaf CGIR lowering | **fixed** | Shared `lower_get_put_leaf` + `build_region_fn` in `optic-cgir` |
| Region `*_fn` builders (3×) | **fixed** | `build_region_fn` with `Read` / `Write` / `PreviewOption` styles |
| `emit_prism_map_decay` / `emit_traversal_map_decay` scaffold | **fixed** | `leaf_map_decay_region` + `emit_entity_loop_prelude/postlude` |
| Triplicate leaf `match` arms in codegen emit paths | **wontfix** | Prism preview/review vs optic/traversal get/put field names differ; a `LeafKind` enum dispatch pays off when M8 adds `Tap`/`Record` — premature before observability variants land |
| Compose probe `CgirGraph` clone during incremental build | **wontfix** | `compose_build_probe` documents intent; full clone is acceptable under v0 N caps (≤512 parse depth, appendix-B-sized examples). Sharing a borrowed probe would thread lifetimes through `build()` with no measurable win at current scale |
| `seq_parent_column` sort+dedup vs `dedup_regions` | **fixed** | `region_column_roots` helper + doc: column-identity (sorted) vs region-path collection (first-seen order) |

**Hardware / scale note:** compose-chain checks clone the in-progress `nodes` vec plus small side maps (`provenance_index`, `resolved_optics`, `region_map`). At M0–M7 example sizes this is microseconds; revisit only if CGIR graphs exceed low thousands of nodes per compilation unit.

### 2026-06-20 robustness pass (asserts + error handling, keep in sync)
- Added `debug_assert!` (with messages) + guards for key invariants in production lib paths only (no behavior change on valid paths): CGIR post-build/verify/fusion (focus/costate wiring, region consistency, no orphan nodes, provenance integrity, ProductFlat validity >=2 children + alias_safe); grade/alias (sat_add bounds doc, share fraction invariants); region map/dedup capacity; codegen (column presence, ident validity post-validate); parser/lexer (depth state, advance loop guard).
- Hardened error paths: replaced select bare .expect() in optic-codegen-rust emit (non-test) with Result propagation + debug guards; improved String errs in helpers with defensive checks; no new diag codes (conservative, leverage existing cgir_verify_failed_diag / codegen_failed etc.); guard recovery/lexer state.
- Stray root artifacts (generated *.rs, old book copies, prior grok-review, empty src/) removed for canonical sync.
- PLAN, docs/v0-executable-spec.md, README-IMPLEMENTATION.md, fixtures/README.md, code comments remain consistent; no doc drift; added robustness notes to fixtures/README + plan table refs.
- "no behavior change" qualified for valid/goldens (error paths now Err not panic).
- Used #[allow] for clippy instead of new fns like is_empty; only debug_assert! (not assert) + guards.
- Incidental clippy fixes (derives/allows/collapses) documented as required for -D clean verification.
- All new asserts have msgs; some coverage direct calls added. Clones in dedup restructured (no dup clones).
- All per "smallest targeted", exact patterns (Vec+u32 ids, Result<String,String> in emit kept, Arc, dedup order), no new features/M7+.
- Verified: fmt+clippy clean; cargo test relevant (golden, execution, parse_recovery, diagnostics, security); full pipeline on examples; no heuristic fallbacks added.

### 2026-06-20 continuation (narrow v0 + robustness + plan sync)
- Parser depth: threaded + guards + +1 on *all* decl entry+body recursion (parse_data/extern/optic/let/fn, get/put/preview/review clauses, blocks, queries, optic/expr from bodies); test covers decl bodies (safe depth exercises guards).
- Emit hardening: eliminated last bare .unwrap() in prod emit (MapDecay) using structured Result + removed redundant post-check debug_assert.
- Added debug_assert/guards: CGIR build for unsafe_boundary (lowering prep), is_simd_eligible_region, boundary lowering invariants in hir; no new variants.
- Sanit/validation: costate_struct_name now calls+enforces validate_user_ident (Result); typeck conditional sanit on boundary names (no discard); extended to costate + emitted names + comments.
- Subprocess: harness does env_clear() + PATH + full homes (RUSTUP incl) behaviorally matching cli sandbox_command (core locate/dedup replicated in test harness for parity).
- Host prep: HIR lowers unsafe optic to Optic+summary (direct lower delta now documented + explicit test in optic crate); gates/TYP/goldens preserved; CGIR debug; sanit enforced.
- profile/replay: added runtime no-op hooks (profile/replay); added CLI Profile/Replay subcommands (surface OBS-701 via compile gate + note).
- traverse/update + SIMD: no surface syntax change (per narrow v0 get/put for traversal); enhanced SIMD bridge with debug_assert + name sanitization on // optic(traversal) / // simd-eligible emission; metadata only.
- No new CGIR variants (avoid fusion/verify update debt); all per exact patterns; new asserts have direct unit/boundary coverage.
- Sync: updated PLAN, docs/*.md (executable-spec, observability, effect-coeffect), README-IMPLEMENTATION, fixtures/README with continuation notes, status, no drift. (This task: further PLAN sync to book/appC/EBNF + actual code state + wontfix notes.) See also cross-refs in README-IMPLEMENTATION.md and effect-coeffect-v0.md for 2026 notes.
- Verified: cargo fmt, clippy -D, tests (incl new depth/sanit), CLI on examples, full pipeline, no unintended golden changes.
- Addressed past issues proactively: parser depth complete, no bare expect left in emit, no redundant asserts/clones, sanit everywhere for new surfaces, subprocess no inherit, CGIR consistency for leaves, tests for guards, docs/plan in lockstep.
- 2026-06-20 plan update (this task): synced PLAN to book app C (M0-6 narrow vs M7+ begin), app D EBNF (no traverse syntax in narrow), actual code (Vec+id not IndexVec; partial M7/M8 scaffolding; profile/replay OBS stubs), marked traverse/profile full as wontfix for narrow; added next priorities within scope only.

### 2026-06-20 impl pass (full match to docs + avoid past issues)
- Threaded parser depth + guards + +1 into data_decl, extern types/params, let/fn header types + value/body exprs (completes all decl entry+body recursion paths per continuation notes).
- Updated fusion passes (product_flatten child collector) to explicitly name/handle Tap/Record variants (prevents "fusion not updated when CGIR variants added").
- Hardened codegen test harness to behaviorally match CLI sandbox (fixed trusted PATH literal + dedup + compile-time bin; no std::env::var capture of parent PATH; env_clear + homes + RUSTUP; cross-ref parity test).
- Added debug_assert (with msgs) for sanitized hook/traversal names before source embed + CGIR hook id ordering invariants.
- Verified no bare unwraps in prod paths, no heuristic fallbacks, no unnecessary clones, sanit/validation at boundaries; costate validated at emit (per prior), extended guards.
- No new features, no golden changes, followed Vec/u32-ids/Arc/Result/verify_to_diagnostic patterns exactly (no index_vec dep).
- Ran cargo fmt, clippy -D warnings, full tests (golden+exec+depth), CLI on examples + negatives (TYP-010/OBS-*), full pipelines; PLAN/docs/code comments in sync.
- This delivers fully working narrow-v0 per book+PLAN+v0-spec (M0-M6 core + M7/M8 scaffolding as documented; host/prep/profile/simd metadata only; no narrow surface for traverse/update).

---

*This PLAN.md lives at the root. Update it (smallest precise edits) as implementation reveals book ambiguities or better conservative choices. Reassemble book sources only if we edit the manuscript itself (per AGENTS.md). Keep narrow v0 vs M7+ distinctions per app C.*
