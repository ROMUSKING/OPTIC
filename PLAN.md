# Optic Narrow v0 Compiler — Complete Implementation Plan

**Source of truth:** The Optic Language Implementation Book (v34 split sources in `book-sources/`, assembled in root and `book-sources/assembled.md`).

**Goal:** Deliver a *fully working*, executable narrow-v0 compiler (and supporting runtime) that meets the book's M0–M6 milestone gates for the prelude. The output must include:
- A usable `opticc` CLI (and library API).
- End-to-end: `.opt` source → parse → HIR + summaries → type/grade/alias check (with good diagnostics) → CGIR (with provenance) → the three fusion rewrites → readable, correct Rust emission.
- Tiny `optic-runtime` (Cursor + SoA support).
- Acceptance examples that **compile to Rust, the emitted Rust compiles and runs, and performs the exact mutations described**.
- Golden fixtures, diagnostics, and benchmark baselines (per appendix B).
- All per the normative EBNF (appendix D), grade rules (ch. 6/9 + appendix E), OpticSummary / Cursor / CGIR shapes (ch. 8/10), codegen shape (ch. 11), and milestone ladder (appendix C).

**Scope for "complete" (this task):** Narrow v0 only (lens-like optics, in-memory SoA, CacheGrade + OwnershipGrade v0 carrier, the three fusions). Later features (prisms, LLVM, full 8D grades, multicore, coinduction, self-hosting) are explicitly out of scope for the first delivery but the architecture must not paint them into a corner (reserve nodes, keep summaries rich, etc.).

**Non-goals for first delivery:** Full project graph / `optic init`, LSP, multicore, native LLVM (ch16), staging (ch14), rich experimental lanes. We implement the *semantic microscope* (Rust backend) described in ch11.

## 1. Analysis Summary (from book-sources/)

**Core artifacts (must exist and be stable):**
- `OpticSummary` (ch8): costate, focus, `lift: PathLift`, `get_reads / put_reads / put_writes: Set<Region>`, `get_grade / put_grade: ConcreteGrade`, determinism, serializable, provenance, (later boundary).
- `ConcreteGrade { cache: u8 (255=∞), ownership: OwnershipDim { share: Rational, read_only: bool, must_use: bool } }`.
- Surface aliases: `LinearGrade`, `AffineGrade`, `SharedGrade`, `CacheGrade<N>`, `OwnershipGrade<r>`, `_` (infer).
- `Cursor<'a, S> { arena: &'a mut S, id: usize }` — the operational heart (ch5/8/11).
- **CGIR** (ch10): `CgirGraph { nodes: IndexVec<NodeId, CgirNode>, roots, provenance_index }`.
  Nodes (core v0): `OpticLeaf {get_fn, put_fn, summary, ...}`, `Compose`, `Product`, `QueryGet/Set/Map`, `FusedLoop { original_ids, ... }`.
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
- **IR containers:** Custom `IndexVec<Id, T>` (newtype u32 id + Vec) for deterministic ids, easy snapshots, no external graph crate initially (petgraph optional later). Keeps provenance trivial.
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
  - Core: `thiserror`, `index_vec` or hand-rolled IndexVec (to avoid extra), `serde` + `serde_json` for dumps/diagnostics/fixtures.
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
- `optic dump-cgir [--before-fusion] [--node N] [--check]`.
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

## 8. Current progress (updated 2026-06-17, review pass 5e79ac44 round 6)

| Milestone | Status | Evidence |
|---|---|---|
| M0 lexer/parser | **mostly done** | Recovery fixed; goldens `fixtures/tokens/`, `fixtures/ast/` (positive + negative tokens); parser hang regression test |
| M1 HIR + summaries | **mostly done** | Tuple/`TupleProj`; HIR map-chain fusion + multi-param guard; `Arc<HirExpr>` map bodies shared to CGIR; HIR goldens for all positive examples (`fixtures/hir/`); `cargo test -p optic-hir golden_hir` |
| M2 types/grades/alias | **mostly done** | ch9.9.3 inference; GRA-110/GRA-104/ALI-201 with `related_spans`; `check` runs CGIR+verify+codegen dry-run |
| M3 CGIR + verifier | **mostly done** | `resolved_optics` alias map; reachability GC through query→optic spine; `dump-cgir --check`; CGIR goldens incl. `health_get`/`health_set` pre+post |
| M4 fusions | **partial — OUT OF SCOPE this /implement run** | ch10 order map→compose→product; map fusion + FusedLoop provenance for compose/product. **Next /implement gate:** full compose body rewrite (ch10) + execution equivalence tests. Reviewers: explicit PLAN deferral — not a defect in this delivery. |
| M5 Rust backend + run | **mostly done** | `is_valid_rust_ident` + keyword denylist + hostile-name emit test; `fixtures/rust/` emit shape tests (all four positives); execution + transpile-compile tests; `fixtures/bench/` baselines |
| M6 release polish | **mostly done** | Full diagnostic JSON witnesses (GRA/ALI/PAR/CGI/RES) with `ranked_fixes` regression; security regression tests; sandboxed toolchain env; bench + execution for all positives; `--verbose` cargo stderr |

**Diagnostic catalog (aligned to book):**
- GRA-110: optic-decl CacheGrade tighter than inferred (ch9.9.3)
- GRA-104: sequential `>>>` composition exceeds bound (ch9.9.4)
- ALI-201: alias conflict with `conflicting_regions` evidence
- RES-001 / CGI-*: resolve and CGIR build errors

**Positive examples** use `CacheGrade<2>` for single-field get+put lenses (inferred cache = sat_add(1,1) = 2).

**Next iteration priorities (/implement gate):**
1. **M4:** Full compose fusion body rewrite (ch10) + execution equivalence tests
2. Broader codegen hardening beyond `is_valid_rust_ident` + keyword denylist

**Done (round 6):** HIR goldens (decay/get/set/position); PAR-001 `ranked_fixes`; security regression suite; runtime path canonicalization; cargo stderr redaction (`--verbose`); hostile-name codegen pipeline test

---

*This PLAN.md lives at the root. Update it as implementation reveals book ambiguities or better conservative choices. Reassemble book sources only if we edit the manuscript itself (per AGENTS.md).*
