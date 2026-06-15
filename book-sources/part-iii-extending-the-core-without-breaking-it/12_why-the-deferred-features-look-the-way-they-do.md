## 12. Why the Deferred Features Look the Way They Do

> **By the end of this chapter, a reader should understand:** what the extension rule requires of any feature before it enters the full language; the full optic kind hierarchy and which machine-level shape each kind corresponds to; the full grade algebra table with all eight dimensions; the reading map for Part III, what each chapter proves, and which prelude facts it depends on; and the complete forward-compatibility hook table that v0 must reserve to avoid later architectural ruptures.

### 12.1 The extension rule

A deferred feature earns a place in the language only if it satisfies the same bridge discipline as the prelude.

```text
semantic form
  -> summary and grade representation
  -> legality rule
  -> optimizer or backend consequence
  -> domain-level payoff
```

This rule is what prevents the full language from becoming a bag of elegant but operationally vague ideas. The prelude proved the chain for lens-like optics and cache/ownership grades. Every subsequent feature must prove the same chain for its own contribution. Features that cannot complete all four steps are research candidates, not language features.

Before the chapter names new optic kinds or grade dimensions, one practical reading rule is worth stating explicitly: **the first question for every deferred feature is not what subtyping relationship it has, but what machine shape it buys and what legality proof it requires**. A prism matters because it becomes a branch that the compiler can bias, predicate, or mask. A traversal matters because it becomes the vectorization form of the language. Staging matters because it is the operational glue that turns the same graph into a package planner, a generated-binding pipeline, a compiler pass, or a hot-path specializer without inventing a second compile-time language. The type-theoretic story is still real; it simply follows the machine-facing justification instead of replacing it.

### 12.2 What the prelude proved, and what it left open

The narrow v0 is a complete proof of one specific architectural claim. Understanding what it does and does not prove is the right starting point for Part III.

#### 12.2.1 What the prelude proved

- **Lens-like optics survive compilation.** A named optic body produces a complete `OpticSummary`. That summary drives type checking, alias checking, fusion legality, and TBAA metadata. The abstraction is not erased before it is useful.
- **Grades can be checked early and erased completely.** A `ConcreteGrade` with two dimensions already changes legality decisions and code shape. The erased runtime has no grade overhead.
- **Explicit runtime context beats ambient effects for this domain.** The `get_reads`/`put_reads`/`put_writes` triple is more useful to the optimizer than an abstract effect name because it identifies concrete regions, not categories.
- **The generated hot-path code looks like hand-written index loops.** The abstraction earns its place not by surviving but by lowering to what the programmer would have written anyway.

#### 12.2.2 What the prelude left open

- **Other optic kinds.** Prisms, traversals, folds, setters, and isos each require different summary fields, different fusion laws, and different code shapes.
- **Richer grade dimensions.** Latency, bandwidth, I/O, liveness, session types, security lattices — each adds a new dimension to the semiring and new legality rules to the checker.
- **Coinductive and staged execution.** Live event loops and compile-time specialization require dedicated CGIR node kinds and backend lowering strategies.
- **A native backend.** The Rust transpiler is the semantic microscope. The LLVM backend is the performance backend, with TBAA metadata, intrinsics, and vector lowering.
- **Multicore and memory layout.** The parallel product operator and NUMA-aware grades require a richer ownership model and explicit partitioning strategy.

### 12.3 The full optic kind hierarchy

The prelude used lens-like optics only. The full language adds five more optic kinds. Each one corresponds to a recurring machine pattern.

| Optic kind | Semantic meaning | Lens laws extension | Primary machine shape | Grade implication |
|---|---|---|---|---|
| Lens | exactly one focus | get-put, put-get, put-put | direct load/modify/store | `CacheGrade<N>` per field; `AffineGrade` |
| Prism | zero or one focus | preview-review consistency | conditional branch | `BranchBias` dimension; mask-lowerable |
| Traversal | many same-shape foci | shape preservation | bulk loop or SIMD pass | `TraversalGrade<N>`; `SimdEligible` flag |
| Fold | many read-only foci | no update path | reduction or scan | `SharedGrade`; no write regions |
| Setter | many write-only foci | no read path | masked bulk update | no read regions |
| Iso | lossless conversion | to-from and from-to identity | zero-cost coercion | no grade cost at the boundary |

The subtyping hierarchy is: `Iso <: Lens <: Prism <: Traversal <: Fold` (by capability loss). A `Lens` is a total `Prism`. A `Prism` is a `Traversal` of at most one element. A `Traversal` is a `Fold` with an update path.

This hierarchy has a compiler consequence: a function accepting a `Prism` also accepts a `Lens`. The coercion is free — the compiler produces `preview = |s| Some(s.get())` automatically. This means composition across optic kinds works without explicit conversion syntax.

### 12.4 The full grade algebra

The prelude uses two grade dimensions. The full language uses eight, each with its own semiring operations. The crucial boundary to keep in mind from the beginning is that **grades are quantitative budgets, while boundary contracts are qualitative legality constraints**. The easiest way to remember the split is: grades behave like financial budgets that can be added, compared, saturated, or distributed through composition; boundary contracts behave like zoning laws that say where the compiler may build, reorder, inline, callback, or stage at all. Both are static. Only one of them is a semiring.

| Dimension | Carrier | Sequential `>>>` | Parallel `***` | Machine consequence |
|---|---|---|---|---|
| `CacheGrade<n>` | `u8 \| ∞` | `sat_add` | `max` | Cache line budget; drives fusion and prefetch |
| `LatencyGrade<d>` | `Duration` | `add` | `max` | Real-time bound; enforced by scheduler |
| `BandwidthGrade<bps>` | `u64 \| ∞` | `max` | `add` | Network/bus saturation |
| `IOGrade<d>` | `Duration \| 0` | `add` | `max` | Blocking I/O budget; 0 means async-only |
| `OwnershipGrade<r>` | `Fraction(0,1]` | preserve the stronger share/obligation | disjointness or partition-sum proof | Alias safety, partitioned parallel update |
| `SecurityGrade<L>` | `Lattice` | `join` | `join` | Information-flow non-interference |
| `SessionGrade<T>` | `SessionType` | sequential compose | pair | Protocol correctness |
| `LivenessGrade<L>` | `Always \| Bounded \| OnSignal` | `max` | `min` | Loop termination; scheduler wakeup |

The grade semiring remains a product of these dimensions. Composition uses dimension-specific operations. Grade checking remains decidable because each dimension's semiring has a finite normal form for the expressions that arise in practice.

In v0, only `CacheGrade` and `OwnershipGrade` are active. The remaining six slots are reserved — the compiler carries the data structure but treats unknown dimensions as `UNBOUNDED`. When a new dimension is added in the full language, it replaces `UNBOUNDED` with a type-checked value. This is the structural hook that makes grade expansion a localized change.

### 12.5 Reading map for Part III

Part III chapters depend on each other and on the prelude in a specific order.

| Chapter | Depends on prelude facts | What it adds |
|---|---|---|
| 12 — Prisms, traversals, SIMD | lens summaries, product alias check, CGIR node kinds | `preview`/`review` pairs; traversal shape preservation; `SimdEligible` flag; branch-bias metadata |
| 13 — Coinduction, staging, observability | CGIR node structure; determinism enum; `BuildRuntime` from Ch. 4 | `Coinductive` nodes; `Stage` nodes with `CompileTimeGrade`; tap/record/profile CGIR nodes with grade-controlled erasure |
| 14 — Full grade algebra and research integration | grade carrier and semiring; `GradeConstraintSolver` trait | Z3 QF_LIA inference for symbolic grades; fractional ownership; session type grades; security grades |
| 15 — LLVM backend | CGIR; `RegionSet`; `OpticSummary` | TBAA metadata from region sets; SIMD intrinsics from traversal grades; io_uring lowering from coinductive nodes |
| 16 — Multicore and memory layout | ownership grade; parallel product alias check; cache grade arithmetic | false-sharing guards; work partitioning strategy; NUMA grades; AoSoA hybrid layouts |

Chapters 12 and 13 are the most fundamental dependencies. A reader who wants to understand the full language before reading all of Parts III–IV should focus on Chapters 12 and 13 first.

### 12.6 Why forward-compatibility must be structural, not rhetorical

The prelude must reserve architectural space for later features at zero cost. The table below matches each future feature to the specific structural hook that v0 must provide.

| Future feature | v0 structural hook | What it prevents if absent |
|---|---|---|
| Symbolic/Z3 grade solver | `GradeConstraintSolver` trait with concrete arithmetic impl | Every grade call site must be changed when symbolic grades arrive |
| Fractional ownership grades | `OwnershipDim` used abstractly in the alias checker | The alias checker must be rewritten around a new carrier |
| Asymmetric get/put grades | Separate `get_grade` and `put_grade` in `OpticSummary` | I/O optics require a structural refactor of every summary consumer |
| Replay and DST | `Determinism` enum + `serializable` bit in `OpticSummary` | A second effect system must be invented retrospectively |
| Observability taps | Reserved `Tap`/`Record`/`Profile` CGIR node variants | The IR must be reshaped after self-hosting is underway |
| Richer ownership | Abstract `OwnershipDim` in checker | Ownership reasoning becomes two unrelated systems |
| Session type grades | Reserved grade dimension slot | Backward-incompatible diagnostic and syntax churn |
| Security grades | Reserved grade dimension slot | Same as session types |
| Prisms, traversals | Summary fields for `preview`/`review` and shape | Summary format cannot absorb new optic kinds without breaking existing tooling |

The principle is simple: reserve *structure*, not *behavior*. A dormant field or enum variant costs nothing. An architectural rewrite after the ecosystem has hardened costs a great deal.

### 12.7 Transition

The first deferred family is the other optic kinds. Prisms and traversals matter because they connect directly to branches, bulk loops, and vectorization — the primary machine-level optimizations that field-only lens operations cannot express.

