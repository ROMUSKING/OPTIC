## 11. Rust Code Generation, Runtime Support, Diagnostics, and Release Discipline

### 11.1 Why the first backend is Rust

The Rust backend is the semantic microscope of the prelude. It is where the project can inspect, line by line, whether the language's core abstractions really lower to the code shape they promised.

That is why the prelude explicitly chooses readability over backend heroics. If the generated Rust is confusing, overly generic, or allocation-heavy, the problem is not that the backend is insufficiently clever. The problem is that the language or the earlier compiler phases have not yet made the right structure explicit.

### 11.2 The target code shape

#### 11.2.1 Representative generated forms

**Single optic map**

```rust
// optic: HealthView
for id_0 in 0..entities.len() {
    let cursor_0 = optic_runtime::Cursor::new(&mut entities, id_0);
    let _h = entities.healths[cursor_0.id];
    let _updated = _h - 10.0;
    entities.healths[cursor_0.id] = _updated;
}
```

**Nested composition**

```rust
// optic(fused): [TransformView, PositionField]
for id_0 in 0..entities.len() {
    let cursor_0 = optic_runtime::Cursor::new(&mut entities, id_0);
    let _intermediate = entities.transforms[cursor_0.id];
    let _p = _intermediate.position;
    let _p_new = shift(_p);
    _intermediate.position = _p_new;
    entities.transforms[cursor_0.id] = _intermediate;
}
```

The prelude wants a very specific family of loop bodies.

- one obvious induction variable,
- direct field indexing,
- no iterator adapters in the hot path,
- no heap allocation inside the loop,
- deterministic temporary names,
- comments carrying optic provenance.

A representative generated loop should look like this.

```rust
// optic(fused): [HealthView, PositionView]
for id_0 in 0..entities.len() {
    let cursor_0 = optic_runtime::Cursor::new(&mut entities, id_0);
    let _h = entities.healths[cursor_0.id];
    let _p = entities.positions[cursor_0.id];
    let (_h_new, _p_new) = update_health(_h, _p);
    entities.healths[cursor_0.id] = _h_new;
    entities.positions[cursor_0.id] = _p_new;
}
```

The point of showing code like this in the book is not cosmetic. It is to keep the backend honest.

### 11.3 The tiny runtime crate

`optic-runtime` in the prelude should be small enough that a reader can understand it in minutes.

- `Cursor<'a, S>`
- a handful of SoA helpers,
- optional debug-mode bounds checks,
- nothing that reimplements language semantics at runtime.

If the runtime becomes a second hidden compiler, the project has already lost the argument that the abstractions are zero-cost.

### 11.4 Diagnostics are part of the architecture

#### 11.4.1 Diagnostic record shape

| Field | Meaning |
|---|---|
| `code` | stable rule identifier |
| `phase` | parse, resolve, type, grade, alias, CGIR, fusion, codegen |
| `primary_span` | main source location |
| `rule` | violated law or invariant |
| `evidence` | structured facts supporting the verdict |
| `minimal_fix_options` | ranked local repairs |
| `next_commands` | smallest useful next inspection step |

#### 11.4.2 Example machine-readable payload

```json
{
  "code": "GRA-104",
  "phase": "grade-check",
  "rule": "cache(seq(a,b)) = sat_add(cache(a), cache(b))",
  "evidence": {
    "lhs": {"node": "RequestParser", "cache": 2},
    "rhs": {"node": "HeavyDbJoin", "cache": 3},
    "actual": 5,
    "bound": 4
  },
  "preferred_fix": "split_pipeline"
}
```

The compiler's diagnostic system is not an outer shell. It is one of the language's critical interfaces, especially for coding-agent workflows.

More strongly: in this architecture, deterministic diagnostics are one of the compiler’s primary proofs that the semantic core has been made executable rather than merely described. If the parser, resolver, checker, fusion engine, or backend cannot emit stable, evidence-bearing diagnostics, then the book’s earlier rigor has not yet become operational reality. The telemetry is not decoration. It is the visible surface of the compiler’s soundness discipline.

The record model should therefore be explicit.

| Field | Purpose |
|---|---|
| stable code | machine-stable reference point |
| phase | where the failure occurred |
| rule | the exact invariant or algebraic law |
| evidence | structured proof facts |
| ranked fixes | localized repair options |
| next commands | deterministic next inspection step |

This is one area where extra prose matters as much as tables do. A good diagnostic system is the operational explanation layer of the language.

The human-readable rendering and the machine-readable JSON should therefore be treated as two views of the same proof object. Neither may contain materially stronger evidence, boundary facts, or repair guidance than the other. Agent-facing richness is required, but it must not come at the cost of making humans second-class readers of the compiler.

#### 11.4.3 Every formal rule must have a machine-facing rendering

The compiler's internal laws may be stated in terms of semirings, coalgebraic structure, phase judgments, and path-lift composition. The default rendered diagnostic should still speak in operational language whenever possible: overlapping write regions, false-sharing risk, cache-budget overflow, runtime-only host reads inside a staged graph, or no zero-cost focused path for the requested context. The theorem-facing rule remains available for precision and cross-referencing, but the everyday rendering should help a systems programmer act without requiring category-theory fluency.

#### 11.4.4 Debuggability after fusion, staging, and native lowering

The optimizer story only remains socially usable if every aggressive rewrite carries a corresponding explanation path. In practical terms the compiler needs a stable identity chain:

```text
source span
  -> HIR item id
  -> CGIR node id
  -> fused-node provenance set
  -> backend location range
  -> profiler / benchmark identity
```

A loop body that came from five optics should still be debuggable as five optics even if it is emitted as one machine-level symbol. That means the toolchain must preserve two related identities from the beginning.

- a **debug identity** for stepping, stack traces, crash locations, and source maps;
- a **performance identity** for profiler samples, benchmark baselines, and regression tracking.

The performance identity should not be only a symbol name. Symbol names change whenever the backend inlines more aggressively, rewrites helper structure, or switches emission strategy. The stable key should be derived from semantic provenance instead:

```text
PerfKey = hash(
  package/module/public-optic-path,
  fused-provenance-set,
  target-profile,
  backend-family,
  edition-line
)
```

That key then supports three later-stage capabilities that many languages discover too late.

1. **Stage-aware crash capsules.** A crash report can include the fused node set, the stage or build artifact that materialized the code, the target profile, and the interface hashes needed to reproduce the failure elsewhere.
2. **Minimal reproducers.** When a crash crosses several fused nodes or staged artifacts, the compiler can materialize a source + interface + artifact bundle instead of asking for an entire workspace archive.
3. **Cross-backend comparability.** The Rust path, LLVM path, and later backends can still be compared because performance data is keyed by semantic provenance rather than by whichever raw symbolization fell out of the emitter.

These are not optional tooling luxuries. They are the operational half of provenance preservation.

### 11.5 Why the book insists on agent-friendly diagnostics

#### 11.5.1 Why diagnostics are a first-class artifact rather than an afterthought

The narrow compiler is trying to prove that front-loaded semantic discipline has direct operational payoffs. Stable diagnostics are one of the clearest places where that claim becomes measurable. If `OpticSummary`, region extraction, grade inference, provenance, and fusion legality are all really as explicit as the architecture says they are, the compiler should be able to explain failures in one pass, with one root cause, with structured evidence that both humans and agents can act on deterministically.

This is why the diagnostics chapter belongs inside the core backend and release discipline rather than in a tooling appendix. The record model is a first-class compiler artifact, and the quality of that artifact is itself evidence about whether the earlier front-end rigor is paying out.

Coding agents amplify both strengths and weaknesses in compiler design. A compiler with noisy, unstable, or weakly evidenced diagnostics becomes hard to automate against. A compiler with explicit codes, structured evidence, and ranked local repairs becomes much easier to integrate into iterative development loops.

That is why the book treats agent-friendly diagnostics as a design goal rather than a later tooling nicety.

### 11.6 Test layers and benchmark discipline

The prelude release discipline is intentionally heavy.

- token snapshots,
- AST snapshots,
- HIR and summary snapshots,
- CGIR snapshots before and after fusion,
- Rust output snapshots,
- diagnostics JSON snapshots,
- benchmark baselines against handwritten loops.

This may look excessive for a first compiler. It is not. The language is making unusually ambitious claims. The only healthy way to manage that ambition is to translate it into repository evidence as early as possible.

### 11.7 What a prelude release actually means

A v0 release is not just "the compiler works on happy paths." It means:

- the examples compile,
- the failures fail with the intended evidence,
- the generated Rust is auditable,
- the optimizer does not erase provenance,
- the benchmarks stay within tolerated drift.

Only after that should the project add prisms, traversals, coinduction, staging, and the native backend.

### 11.8 Three tooling investments that should not wait

These are not milestone gates. They are investments that compound across every subsequent session of development. The cost of establishing them during M5–M6, when the acceptance suite is being built anyway, is low. The cost of retrofitting them at M9, when the compiler is large and has users, is high.

#### 11.8.1 Benchmark harness with committed baselines

The M6 gate says "benchmarks green within tolerance." That gate is only meaningful if baselines were committed before M6 work began.

The benchmark harness should record the following for each benchmark:

```text
PerfBenchmark {
    key:         PerfKey,        // semantic id: optic name + composition shape
    target:      TargetProfile,  // arch, ISA extensions, cache configuration
    backend:     BackendFamily,  // RustTranspiler | LlvmNative
    tolerance:   Tolerance,      // relative band: e.g., ±5%
    baseline_ns: u64,            // committed at M5 acceptance
}
```

The key being semantic (optic name and composition shape) rather than a raw symbol name means benchmarks survive refactoring and renaming without needing manual baseline updates. A benchmark whose `PerfKey` changes because the optic was renamed is a new benchmark, not a broken old one.

Baselines must be committed as part of the M5 acceptance suite. A baseline committed on the same day as the M6 gate check does not count.

#### 11.8.2 Agent-facing diagnostic validation before M6

The diagnostic schema — stable codes, evidence objects, ranked fixes, next-command suggestions — is already specified. Before M6 is declared, it must be validated against a real automated repair loop.

The test: give an automated agent a file with three errors (one grade bound violation, one alias conflict, one type mismatch), allow it to resolve each using only the structured JSON diagnostic output, no additional context. If any of the three requires more than one pass, the evidence field for that diagnostic code is missing something specific.

The standard for M6 should be:

| Error kind | Maximum repair passes | If more passes needed |
|---|---|---|
| `GRA-104` (grade bound exceeded) | 1 | `evidence.decomposition` is missing — which composition step exceeded the bound |
| `ALI-201` (write/write conflict) | 1 | `evidence.conflicting_regions` is missing — which fields in the two optics overlap |
| `TYP-201` (focus type mismatch) | 1 | `evidence.expected_type` and `evidence.actual_type` are both required |

The fix command suggested in `ranked_fixes[0].command` must be syntactically valid and immediately applicable. An agent should not need to guess at the syntax.

#### 11.8.3 Translation-validation harness for the LLVM backend

Before the LLVM backend becomes authoritative (M9), the project needs a way to verify that LLVM-generated code is semantically equivalent to the Rust-transpiled code for the same source program.

The minimum practical harness:

```text
1. compile the acceptance suite with the Rust transpiler backend
2. compile the same suite with the LLVM backend
3. compare diagnostic JSON — both must produce identical codes, spans, and evidence
4. compare generated loop shapes for canonical examples against committed shape fixtures
5. compare benchmark deltas — LLVM result must be within the tolerance band of the Rust baseline
6. only then update the LLVM backend's status to "translation-validated for this revision"
```

Step 3 is the most important. If the two backends disagree on whether a grade or alias violation exists, the semantic claim of the language is violated at the implementation level. The Rust backend is the reference; LLVM must agree with it, not the other way around.

This harness should be built during M7–M8, operational before M9 is declared, and run in CI on every backend change thereafter.

### 11.9 Transition

The narrow compiler is now in place. The next part of the book turns back toward the deferred features and explains their design rationale in the same theory-to-machine style.

---

### 11.10 Detailed implementation reference: deterministic Rust lowering and runtime interface

This section records the exact code-shape witness expected from the v0 backend. It exists so implementers can compare generated output to a precise norm rather than a vague stylistic intuition.

#### 11.10.1 Code shape rules

The Rust backend must produce code in a restricted subset of Rust that is:

- Loop-based (no iterators on the hot path)
- Index-based with explicit `usize` bounds
- Direct field access (no trait dispatch)
- No heap allocation in generated loops
- No `unsafe` in v0

#### 11.10.2 Generated code shapes

**Single optic get:**

```rust
// optic: HealthView
let _result = {
    let cursor_0 = optic_runtime::Cursor::new(&entities, target_id);
    entities.healths[cursor_0.id]
};
```

**Single optic map over all entities:**

```rust
// optic: HealthView
for id_0 in 0..entities.len() {
    let cursor_0 = optic_runtime::Cursor::new(&mut entities, id_0);
    let _h = entities.healths[cursor_0.id];
    let _updated = _h - 10.0;
    entities.healths[cursor_0.id] = _updated;
}
```

**Fused product `HealthView *** PositionView`:**

```rust
// optic(fused): [HealthView, PositionView]
for id_0 in 0..entities.len() {
    let cursor_0 = optic_runtime::Cursor::new(&mut entities, id_0);
    let _h = entities.healths[cursor_0.id];
    let _p = entities.positions[cursor_0.id];
    let (_h_new, _p_new) = update_health(_h, _p);
    entities.healths[cursor_0.id] = _h_new;
    entities.positions[cursor_0.id] = _p_new;
}
```

**Nested composition `TransformView >>> PositionField`:**

```rust
// optic(fused): [TransformView, PositionField]
for id_0 in 0..entities.len() {
    let cursor_0 = optic_runtime::Cursor::new(&mut entities, id_0);
    let _intermediate = entities.transforms[cursor_0.id];  // TransformView.get
    let _p = _intermediate.position;                        // PositionField.get
    let _p_new = shift(_p);
    _intermediate.position = _p_new;                        // PositionField.put
    entities.transforms[cursor_0.id] = _intermediate;       // TransformView.put
}
```

#### 11.10.3 Naming conventions for generated code

| Source construct | Generated name |
|----------------|----------------|
| Cursor variable | `cursor_N` where N is a per-scope counter |
| Loop index | `id_N` |
| Focused value from optic X | `_x` (lowercase of optic name) |
| Updated value | `_x_new` |
| Intermediate (fused) | `_intermediate` |
| Helper calls | `optic_runtime::*` |

Deterministic naming is essential for golden test stability.

#### 11.10.4 `optic-runtime` crate interface (v0)

##### 11.10.4.1 Codegen algorithm by query kind

The Rust backend should use a structured emitter over a small Rust AST, not string concatenation. In pseudocode:

```text
emit_query_get(node):
  emit let cursor_N = Cursor::new(&mut costate, id)
  emit focused_value = emit_get(node.optic, cursor_N)
  return focused_value

emit_query_set(node):
  emit loop over ids
  emit cursor_N
  emit put(node.optic, cursor_N, emit_expr(node.value))

emit_query_map(node):
  emit loop over ids
  emit cursor_N
  emit old = get(node.optic, cursor_N)
  emit new = call(node.map_fn, old)
  emit put(node.optic, cursor_N, new)

emit_fused_loop(node):
  emit one loop
  emit each fused load in source order
  emit transformed temporaries
  emit stores in legality-preserving order
```

The deliberate code-shape goal is that each generated loop can be read as the exact operational witness of a summary judgment.

##### 11.10.4.2 Mechanistic sympathy checklist for generated loops

Every generated hot-path loop should satisfy the following checklist unless the source explicitly prevents it:

- one explicit induction variable,
- contiguous SoA indexing,
- no heap allocation in the loop body,
- no closure allocation,
- temporaries stay in lexical block scope and are eligible for register allocation,
- stores happen after the last dependent load unless proven reorderable,
- comments preserve optic provenance.

This checklist is the low-level acceptance criterion that corresponds to the high-level fusion story.

```rust
// optic-runtime/src/lib.rs

pub struct Cursor<'a, S> {
    pub arena: &'a mut S,
    pub id:    usize,
}

impl<'a, S> Cursor<'a, S> {
    pub fn new(arena: &'a mut S, id: usize) -> Self {
        Cursor { arena, id }
    }
}

// SoA helpers
pub fn soalen<T>(v: &[T]) -> usize { v.len() }

// Debug-mode bounds check (stripped in release)
#[cfg(debug_assertions)]
pub fn check_bounds(id: usize, len: usize, span: &'static str) {
    assert!(id < len, "optic: out-of-bounds access at {}", span);
}
#[cfg(not(debug_assertions))]
#[inline(always)]
pub fn check_bounds(_id: usize, _len: usize, _span: &'static str) {}
```

---

### 11.11 Detailed implementation reference: structured diagnostics and repair-ranking rules

Because the compiler is meant to work with both humans and coding agents, the error model is part of the architecture. The supplement below records the detailed record shape, scoring rules, and state-machine-style repair discipline needed for a stable engineering workflow.

Diagnostics are part of the implementation architecture. They are not a cosmetic layer added after the compiler works.

#### 11.11.1 Diagnostic design goals

- Stable error codes that never depend on wording
- One primary root cause per diagnostic record
- Related evidence attached as structured facts
- Suggested repairs ranked by locality, safety, and likelihood of success
- Machine-readable JSON emission for coding agents
- Human-readable rendering that mirrors the JSON fields
- Clear distinction between proven facts and speculative advice
- Next-command guidance so an agent can continue without guessing

#### 11.11.2 Diagnostic record shape

| Field | Type | Meaning |
|-------|------|---------|
| `code` | `string` | Stable identifier, e.g. `GRA-104` |
| `title` | `string` | Short rule-oriented title |
| `phase` | `enum` | Parse, resolve, type, grade, alias, CGIR, codegen, runtime |
| `severity` | `enum` | Error, Warning, Note |
| `primary_span` | `Span` | The main offending span |
| `related_spans` | `[Span]` | Supporting source locations |
| `rule` | `string` | Exact rule or invariant violated |
| `summary` | `string` | One-sentence human explanation |
| `evidence` | `object` | Structured facts proving the failure |
| `minimal_fix_options` | `[Fix]` | Ordered candidate repairs |
| `preferred_fix` | `string` | The compiler's best current recommendation |
| `next_commands` | `[string]` | The smallest useful follow-up commands |
| `confidence` | `float` | 0.0–1.0 confidence in the preferred fix |
| `provenance` | `[NodeId]` | CGIR nodes involved |

#### 11.11.3 Human-readable format

```text
error[GRA-104]: composed cache grade exceeds declared bound
  --> examples/http_pipeline.opt:42:18
   |
42 | let pipeline = RequestParser >>> HeavyDbJoin;
   |                ^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   | rule: cache(seq(a,b)) = sat_add(cache(a), cache(b))
   | actual: CacheGrade<5>
   | bound:  CacheGrade<4>
   | evidence:
   |   RequestParser = CacheGrade<2>  (declared)
   |   HeavyDbJoin   = CacheGrade<3>  (inferred from 3-field body)
   | preferred fix [1/2]: split the pipeline after RequestParser
   | preferred fix [2/2]: relax the bound to CacheGrade<5>
   | next: optic dump-cgir examples/http_pipeline.opt --node pipeline
```

#### 11.11.4 JSON format for agents

```json
{
  "code": "GRA-104",
  "phase": "grade-check",
  "severity": "error",
  "title": "composed cache grade exceeds declared bound",
  "primary_span": {"file": "examples/http_pipeline.opt", "line": 42, "col": 18, "end_col": 44},
  "rule": "cache(seq(a,b)) = sat_add(cache(a), cache(b))",
  "evidence": {
    "lhs": {"node": "RequestParser", "cache": 2, "source": "declared"},
    "rhs": {"node": "HeavyDbJoin",   "cache": 3, "source": "inferred", "fields_read": ["buffers", "state", "db"]},
    "actual_grade": {"cache": 5},
    "declared_bound": {"cache": 4}
  },
  "minimal_fix_options": [
    {
      "rank": 1,
      "kind": "split_pipeline",
      "edit_scope": "local",
      "suggested_change": "Introduce a named pipeline boundary after RequestParser.",
      "example": "let parse_stage = RequestParser; let db_stage = parse_stage >>> HeavyDbJoin;"
    },
    {
      "rank": 2,
      "kind": "relax_bound",
      "edit_scope": "signature",
      "suggested_change": "Increase the declared bound to CacheGrade<5>.",
      "example": "let pipeline: GradedOptic<..., CacheGrade<5>> = RequestParser >>> HeavyDbJoin;"
    }
  ],
  "preferred_fix": "split_pipeline",
  "next_commands": [
    "optic dump-cgir examples/http_pipeline.opt --node pipeline",
    "optic explain GRA-104"
  ],
  "confidence": 0.93,
  "provenance": ["node:17", "node:23"]
}
```

#### 11.11.5 Diagnostic families

| Prefix | Family | Owner crate |
|--------|--------|-------------|
| `PAR` | Parsing and precedence | `optic-syntax` |
| `RES` | Name and field resolution | `optic-hir` |
| `TYP` | Type mismatch and capability mismatch | `optic-typeck` |
| `GRA` | Grade algebra and bounds | `optic-typeck` |
| `ALI` | Alias and ownership conflicts | `optic-typeck` |
| `OPT` | Unsupported optic kind or feature gate | `optic-hir` / `optic-typeck` |
| `CGI` | CGIR construction or invariant failure | `optic-cgir` |
| `FUS` | Fusion precondition failures or regressions | `optic-opt` |
| `COD` | Rust codegen failure or internal compiler bug | `optic-codegen-rust` |
| `STG` | Staging, compile-time execution, and specialization-cache diagnostics | `optic-hir` / `optic-opt` |
| `OBS` | Observability/replay feature misuse (full language) | reserved |
| `KRN` | Kernel-class target rule violations (full language) | reserved |
| `BTS` | Bootstrapping / self-hosting mismatches (full language) | reserved |
| `GRD` | Gradual grade runtime observations (full language) | reserved |

#### 11.11.6 Repair ranking rules

##### 11.11.6.1 Patch scoring algorithm for agent-facing fixes

A ranked repair list should be generated by an explicit scoring function, not by ad hoc prose order. One workable v0 scoring rule is:

```text
score(fix) =
   5 * locality(fix)
 + 4 * semantic_safety(fix)
 + 3 * compiler_confidence(fix)
 - 3 * churn(fix)
 - 4 * risk_of_masking_root_cause(fix)
```

Suggested normalizations:

- `locality`: 0–3 (single token to multi-file change)
- `semantic_safety`: 0–3 (syntax-only to potentially behavior-changing)
- `compiler_confidence`: 0–3 (based on how direct the evidence is)
- `churn`: 0–3
- `risk_of_masking_root_cause`: 0–3

The ranking must be reproducible for the same evidence. Agents should never see repair options appear in random order from run to run.

##### 11.11.6.2 Diagnostic state machine for coding agents

Every diagnostic family should map cleanly to a next-step state machine:

```text
PAR -> fix syntax -> rerun parse
RES -> inspect declarations / field names -> rerun resolve
TYP -> dump summaries for both sides -> rerun typeck
GRA -> inspect inferred grade evidence -> rerun grade check
ALI -> inspect overlapping regions -> rerun alias check
CGI/FUS/COD -> dump HIR/CGIR -> treat as possible compiler bug if source is clean
```

This is why `next_commands` belongs in the diagnostic record. The compiler should make the next efficient move explicit rather than forcing agents to guess which internal artifact matters.

Coding agents need guidance that reduces churn.

- Prefer syntax-local fixes over type-signature changes.
- Prefer type-signature changes over grade-bound relaxation.
- Prefer structural rewrites that preserve semantics over configuration changes.
- Never suggest adding `unsafe` in the prelude.
- Never suggest suppressing an error without an alternate proof path.
- When two fixes are equally correct, prefer the one that keeps the prelude semantics narrow.
- For `ALI-201` alias conflicts, prefer adding read-only access over splitting the costate.

---

### 11.12 Detailed implementation reference: test layers, benchmark suites, and release gates

The v0 compiler is credible only if each claim has a repository witness. The following material lists those witnesses in the form of snapshot layers, benchmark baselines, and capability gates.

#### 11.12.1 Test layers

Every layer must have its own golden snapshot suite checked into `fixtures/`.

| Layer | Test type | Snapshot file |
|-------|-----------|---------------|
| Lexer | Token stream snapshots | `fixtures/tokens/*.json` |
| Parser | AST snapshots | `fixtures/ast/*.json` |
| HIR | Resolved HIR + summary table | `fixtures/hir/*.json` |
| Type/grade check | Pass (Typed HIR) and fail (diagnostics JSON) | `fixtures/typeck/*.json` |
| CGIR | Pre-fusion graph | `fixtures/cgir/pre/*.json` |
| Fusion | Post-fusion graph | `fixtures/cgir/post/*.json` |
| Codegen | Generated Rust source | `fixtures/rust/*.rs` |
| Diagnostics | Full JSON diagnostic output | `fixtures/diagnostics/*.json` |
| Benchmarks | Baseline timings and cycle counts | `fixtures/bench/*.json` |

#### 11.12.2 Snapshot update protocol

Snapshots are never automatically overwritten by the test runner. A deliberate `optic snapshot-update --confirm` command is required. Every snapshot update must be reviewed in the diff. This prevents silent behavioral regressions.

#### 11.12.3 Required benchmark set

| Benchmark | What it measures |
|-----------|-----------------|
| SoA health decay loop (N=10k) | Zero-intermediate loop, no temporary allocation |
| Product update: health + position (N=10k) | Fused product, two-field access per iteration |
| Batch transform pipeline: `A >>> B >>> C` (N=10k) | Compose fusion depth-3 |
| Host boundary: counter increment | Minimal `HostContextLite` round-trip |

Each benchmark compares the generated Rust output to a handwritten baseline (`fixtures/bench/baselines/*.rs`). Acceptable drift is ±5% on cycle counts across three consecutive runs on the CI machine. Regressions beyond this threshold block the release.

#### 11.12.4 Release gates

A release candidate for the prelude must satisfy all of the following:

- All acceptance examples compile and produce correct output.
- All expected-failure examples fail with the exact intended diagnostic codes and fields.
- Generated Rust for all golden examples is stable (snapshot matches or change is reviewed).
- No internal compiler error on the regression suite.
- Benchmark drift within the accepted tolerance band.
- `optic doctor` succeeds on the full example repository.
- All fixture files are committed and up to date.

#### 11.12.5 Exit criteria for the prelude

##### 11.12.5.1 Evidence, not optimism

The project should treat each release gate as an empirical claim backed by artifacts:

- correctness is backed by green example suites and stable diagnostics,
- optimization is backed by before/after CGIR snapshots and benchmark witnesses,
- code-shape claims are backed by readable Rust golden files,
- architectural forward-compatibility is backed by reserved fields and traits that compile unused in v0.

This matters because the language is making unusually ambitious promises. The only healthy way to keep those promises credible is to convert them into repository evidence as early as possible.

The project leaves the prelude only after the small compiler is boring. Boring means: deterministic behavior, predictable diagnostics, no recurring architectural refactors in the front end or CGIR. Expansion begins only after boring is achieved.

---

