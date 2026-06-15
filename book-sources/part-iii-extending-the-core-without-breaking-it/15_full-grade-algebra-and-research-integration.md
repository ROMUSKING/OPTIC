## 15. Full Grade Algebra and Research Integration

### 15.1 From two dimensions to a product of resource logics

The easiest way to keep the full algebra understandable is to separate two jobs that the language performs statically. **Grades are the quantitative budgets**: they answer questions like how much cache pressure, latency, bandwidth, compile-time work, or ownership share a composition consumes. **Boundary contracts are the qualitative zoning laws**: they answer questions like whether a call may unwind, whether a region is volatile or MMIO, whether a callback may reenter, or whether a runtime family boundary may be crossed at all. Both are static obligations. Only the first class belongs in the semiring. This distinction keeps the grade algebra mathematically crisp while still letting the language talk honestly about hard systems boundaries.

#### 15.1.1 Full-dimension summary table

| Dimension | Sequential rule | Product rule | Why the dimension exists |
|---|---|---|---|
| cache | add with saturation | conservative max | locality and pass structure |
| latency | add | max | end-to-end timing budgets |
| bandwidth | max | add | competing throughput demands |
| I/O | add | max | blocking and queueing costs |
| compile-time work | add | add | specialization budget and artifact/build cost |
| ownership | stricter wins | stricter wins if alias-safe | exclusivity and alias control |
| liveness | stricter progression | conservative combine | live-loop behavior |
| session | protocol sequence | protocol pairing | network legality |
| security | lattice join | lattice join | information-flow control |

The budgeting-versus-zoning split matters immediately here. `CacheGrade`, `LatencyGrade`, `BandwidthGrade`, `IOGrade`, `CompileTimeGrade`, `OwnershipGrade`, `LivenessGrade`, `SessionGrade`, and `SecurityGrade` belong in the algebra because they still compose in law-like ways. ABI, unwind, callback, thread-affinity, address-space, volatility, allocator, and privilege facts do not; they continue to live in `BoundaryContract`, where the checker can treat them as legality conditions rather than as quantities to add or maximize.

The full language extends the prelude pair into a multi-dimensional grade product.

| Dimension | Purpose | Dominant machine consequence |
|---|---|---|
| cache | locality envelope | fusion, prefetch, layout decisions |
| latency | time budget | scheduling and admission rules |
| bandwidth | throughput envelope | throttling and batching decisions |
| I/O | blocking or queueing cost | host boundary legality |
| compile-time work | build-time specialization budget | caching, residualization, and code-size control |
| ownership | exclusivity and alias control | parallel safety and update discipline |
| liveness | temporal progression | event loop and cancellation behavior |
| session | protocol legality | network state-machine correctness |
| security | information-flow policy | compile-time non-interference checks |

The important design point is that these live in one product space so the compiler can carry one grade object through composition rather than a pile of disconnected annotations.

#### 15.1.2 Compile-time work is a grade; stageability is a phase judgment

The staging chapter introduced an important separation that deserves to be repeated here. `CompileTimeGrade` measures the *cost* of specialization once a subgraph is already static enough to run in the compiler. It is not the same thing as the proof that the subgraph is legal to execute at compile time. That proof comes from the phase analysis over `BuildRuntime` versus `Runtime`.

This separation avoids a damaging confusion. A computation can be cheap enough to execute during compilation and still be illegal because it reads a live runtime socket or an unrecorded environment input. It can also be perfectly legal to execute at compile time and still be residualized because the build-time cost or code-size growth is not worth it.

The language therefore needs both:

- a **phase judgment** that says whether a node is `Static`, `Residual`, or `Dynamic`;
- and a **compile-time work grade** that says how expensive it is to specialize once it is static.

Without the first, compile time becomes an unsound build-script escape. Without the second, compile-time execution becomes impossible to budget, explain, or tune. This is why staging belongs in the center of the architecture: it is the one mechanism that connects local specialization, package/build evaluation, artifact generation, and later self-hosting without requiring the language to fracture into separate compile-time and runtime personalities.

### 15.2 Why asymmetric grades are necessary

The prelude already reserves separate `get_grade` and `put_grade` fields even though it sets them equal. That reservation exists because real host optics are often asymmetric.

A disk or network optic may have an expensive read path and a cheap write-back path, or vice versa. A compiler pass may analyze expensively but rewrite cheaply. If the language forced one symmetric grade everywhere, it would either understate or overstate crucial operational facts.

### 15.3 Solver integration without architectural rupture

The language should treat solver systems the same way it treats build planning, routing, parser tables, or optimizer passes: as structured graph computations over explicit costates, not as excuses to introduce a second core language. The default answer is therefore **optics-native solvers first, dedicated solver DSLs only as imported or embedded frontends when the notation genuinely pays for itself**.

The architecture already has the right pieces. A solver state is just another costate. Residual evaluation, Jacobian assembly, preconditioning, time stepping, convergence checks, mesh or stencil plans, and generated kernels are all ordinary transformations that can be expressed with lenses, traversals, prisms, staging, and generated artifacts. The gain is that solver work then reuses the same summaries, provenance, diagnostics, and target-profile machinery as the rest of the language instead of carving out a separate semantic world.

#### 15.3.1 Solver systems are naturally costate-shaped

Typical solver roots already fit the existing model:

- ODE state vectors and controller state,
- PDE field grids, sparse matrices, and boundary-condition tables,
- nonlinear solve worlds carrying residuals, Jacobians, and preconditioners,
- compiled plan artifacts such as sparsity patterns, stencil kernels, and mesh/blocking layouts.

Once those are costates, the main solver operators become ordinary optics:

- `Residual`,
- `Jacobian`,
- `Precondition`,
- `Step`,
- `AcceptReject`,
- `Restrict` and `Prolong`,
- `AssembleStencil`,
- `AssembleSparsePattern`.

This is a better default than a separate solver sublanguage because the same `OpticSummary`, `RegionSet`, grade, and provenance machinery can still explain legality and backend consequences.

#### 15.3.2 Solver notation may be special; solver semantics should not be

There are domains where equation-heavy notation is genuinely useful: PDEs, DAEs, control systems, multiphysics, and imported scientific formats. The book should not deny that. But the right way to admit such notation is as an **embedded or imported modeling frontend** that lowers into the same graph, not as a second semantic center.

This is the lesson visible in current solver ecosystems: Devito is explicit about being a symbolic finite-difference DSL that lowers high-level equations into optimized stencil kernels, while ModelingToolkit represents numerical problems as `System` values and then compiles and transforms those systems before code generation. Diffrax, by contrast, shows that many differential-equation solver families can also live behind one unified library interface without inventing a separate standalone language. Those examples point to the same Optic rule: the syntax may vary, but the lowered artifact should still be a single graph of typed operators, generated kernels, and target-aware plans.

#### 15.3.3 First-party solver libraries should therefore be optics-native

A good long-range library split is:

- `std.solver.ode`
- `std.solver.pde`
- `std.solver.linear`
- `std.solver.nonlinear`
- `std.solver.control`

These should be ordinary first-party libraries, not new surface-language regimes. Their implementations may stage generated kernels, sparsity plans, or solver traces, but those artifacts should remain ordinary outputs of `BuildRuntime` evaluation and ordinary entries in the artifact index.

#### 15.3.4 The experimental lane is the right place for richer numerical foundations

More speculative mathematics should enter solver work through the already reserved experimental lane. In practice that means:

- geometric-algebra kernels under `std.experimental.geo`,
- nonstandard-analysis-inspired asymptotic and grid ideas under `std.experimental.nonstandard`,
- proof or equivalence witnesses for generated solver kernels under `std.experimental.proof`.

That keeps solver experimentation inside the same graph and benchmarking discipline while preserving the closure rule that ordinary user code should not need to learn a second theorem-heavy or notation-heavy language.

#### 15.3.5 The practical design rule

The governing rule should be stated plainly:

> solver notation may be special; solver semantics should not be.

If a solver or modeling frontend cannot lower into ordinary summaries, ordinary graph rewrites, ordinary staged artifacts, and ordinary diagnostics, then it should not be treated as part of the language proper.

### 15.4 Fractional ownership and multicore growth

The language now commits to **fractional ownership as the underlying ownership carrier early**, with `SharedGrade`, `AffineGrade`, and `LinearGrade` retained as named special cases in the surface language and teaching examples. The main gain is not cosmetic. It is the ability to carry compositional partition proofs through `***`, staging, and later multicore work without retrofitting the alias checker around a new carrier.

The practical staging of the feature is therefore:

- M0–M6: the parser, summaries, and alias checker already use the fractional carrier internally; most user code still writes the named cases;
- M7–M15: explicit `OwnershipGrade<p/q>` is available where partitioning or lending actually needs it;
- M16+: multicore partition optics, NUMA policies, and lock-free libraries use the same carrier more aggressively.

This front-loads the hard design decision while still keeping the common teaching surface readable.

### 15.5 Gradual and observed grades

Adoption matters. A language that insists on full precise grades before it can be used at all may create unnecessary friction. The later gradual-grade path is therefore reasonable so long as it is honest: unknown grades should compile to observed development wrappers, emit repair-oriented diagnostics, and disappear only after being concretized.

#### 15.5.1 Grade elision and inference belong in the full-language surface

The language should also make **checked grade elision and partial inference** an explicit surface feature. Programmers should be free to omit dimensions the compiler can reconstruct from summaries, to specify only the dimensions that matter operationally at that site, and to ask the compiler to materialize the fully normalized grade in interfaces, diagnostics, and generated documentation. This is the grade-side analogue of checked runtime focusing: a convenience on the surface, but never a hidden default in the semantic core.

A practical surface rule is:

- `_` requests exact reconstruction from summaries and composition laws,
- partial grade expressions such as `CacheGrade<2> + _ + _` freeze only the dimensions the programmer has named,
- `?` remains the gradual-grade form for observation and repair rather than exact inference,
- exported/public interfaces may refuse elision when it would hide a contract the downstream compiler must see explicitly.

That last point matters because grade elision belongs in the language, not just in the checker, only if the compiler can still decide when a human-authored contract must stay visible at a boundary.

### 15.6 Replay and determinism refinements

The research story around deterministic simulation testing is a good fit for the language as long as it remains precise about boundaries.

- time must be injected,
- randomness must be seeded or recorded,
- semantic state must be serializable,
- host-facing nondeterminism must be explicit.

That is why the full determinism and replay story depends on architectural hooks already present in the prelude rather than on later ad hoc instrumentation.

### 15.7 Experimental mathematics tracks should enter through one reserved lane

The language should not respond to every promising mathematical idea by minting a new top-level keyword or a second proof sublanguage. The safer rule is to reserve one contextual keyword, `experimental`, for native `package`, `workspace`, and `build_plan` declarations, one namespace root, `std.experimental`, for library-facing experiments, and one graph-native `ExperimentalArena` for typed witnesses, indexes, staged artifacts, and diagnostics that are explicitly provisional.

```rust
pub let package = Package {
    name: "compiler.core",
    edition: Edition::current(),
    targets: [Target::linux_x86_64_musl()],
    experimental: [
        Experimental::proof_equivalence(),
        Experimental::geo_algebra(),
    ],
};
```

The gain is architectural, not merely organizational. Experimental features can be implemented, tested, benchmarked, and queried through the same `Project` graph without silently becoming part of the ordinary language or forcing the team to invent a second research DSL. Promotion into core still goes through the same checklist as any other feature.

The important refinement is that the experimental lane should not be flat. Some questions in the language — especially the memory model, provenance, alias safety, and reorder legality — have **direct** experimental answers that fit the current architecture with relatively few new concepts. Other questions are better handled first as external specification or analysis sidecars. Richer categorical or foundational theories should therefore compete against these simpler answers rather than being assumed to be the first step.

Two additional rules keep this lane from becoming a second semantics. First, every experimental track remains *subordinate* to the same core artifacts as ordinary language features: it must lower into ordinary `OpticSummary`, `RegionSet`, `BoundaryContract`, `GradeExpr`, and CGIR nodes before it can participate in optimization, code generation, replay, or diagnostics. An experimental geometry kernel, nonstandard-analysis witness, or sheaf-style consistency check may enrich the graph, but it may not invent an alternative legality path that bypasses the main summary model.

Second, experimental mathematics should be judged against **mixed-domain collision tests**, not only against isolated demos. A feature is much more convincing when it survives contact between unlike domains — for example, a staged geometric transform inside a frame-budgeted renderer that also streams networked state, or a solver-generated stencil kernel embedded in a larger service pipeline — because those collisions reveal whether the experimental lane really composes with the language's existing grade and boundary rules.

#### 15.7.1 Resource and separation reasoning belongs under `std.experimental.sep`

The most direct experimental companion to the current ownership, region, and boundary model is a separation/resource-logic lane. Reserved structures here should include `ResourceWitness`, `OwnershipSplit`, `RegionInvariant`, `BorrowStackWitness`, and `BoundaryObligation`. This lane is the most natural place to refine fractional ownership, local unsafe proofs, callback/resource obligations, and provenance-aware alias reasoning because it speaks the same language as `RegionSet`, ownership grades, and `BoundaryContract`.

#### 15.7.2 Weak-memory and ordering semantics belong under `std.experimental.memory`

The second direct lane should be a dedicated memory-model and weak-memory track. Reserved structures should include `LitmusCase`, `PromiseWitness`, `OrderingConstraint`, `FenceWitness`, `VolatileRegionRecord`, and `ReorderLegalityWitness`. This lane exists because atomics, fences, volatility, DMA visibility, and backend reorder legality are more directly answered by weak-memory operational models than by the richer proof-facing tracks. It should refine `AccessMode`, `AtomicGrade`, and boundary legality rather than compete with them.

#### 15.7.3 The proof and equivalence track belongs under `std.experimental.proof`

The strongest proof-facing refinement remains a dedicated equivalence lane rather than a wholesale proof-oriented surface language. A reserved proof track should hold structures such as `RewriteWitness`, `EquivalenceWitness`, `TransportPlan`, and `TranslationValidationRecord`. The purpose is to make rewrite equivalence, staged residualization, and cross-backend validation more explicit without turning everyday Optic code into a proof assistant language.

#### 15.7.4 The geometric-algebra track belongs under `std.experimental.geo`

The language should reserve `Multivector`, `Bivector`, `Rotor`, `Motor`, and `GeoKernel` under `std.experimental.geo` as first-party experimental numerics for geometry-heavy domains. This track belongs mainly to domain libraries, layout choices, and backend intrinsics for graphics, robotics, realtime media, and simulation. It should not replace the ordinary numeric tower unless it proves a broader compiler and ecosystem payoff.

#### 15.7.5 The ultrametric and p-adic track belongs under `std.experimental.ultra`

Ultrametric and p-adic ideas are most promising for hierarchy-shaped retrieval and indexing rather than for the core arithmetic of the language. The reserved structures here should be things like `UltrametricIndex`, `HierarchyMetric`, and `PadicScalar`, with the main initial targets being `ProjectGraph` search, AST/CGIR clustering, agent-memory retrieval, and failed-patch similarity search.

#### 15.7.6 Sheaf-style local/global reasoning belongs under `std.experimental.sheaf`

Sheaf-inspired structures should first appear as graph-facing consistency machinery: `LocalObservation`, `Cover`, `Section`, `GlueWitness`, and `ConsistencySheaf`. The natural use cases are distributed services, plugin ecosystems, partial projections of `ProjectGraph`, and cross-package or cross-runtime consistency where local facts must glue into one global judgment.

#### 15.7.7 Topos and guarded-domain work belongs under `std.experimental.topos`

The topos/guarded lane should be reserved for proof and semantics work around internal logic, guarded recursion, clocked reasoning, and richer models of staged and coinductive computation. Useful reserved structures include `Site`, `ToposModel`, `GuardedClock`, `Later`, and `GuardedRecursionWitness`. The practical rule is that this lane may sharpen the proof story of coinduction and staging long before it changes the ordinary surface language.

#### 15.7.8 Dynamics and stability analysis belongs under `std.experimental.dynamics`

The runtime, scheduler, and agent-operating-system work needs a place for explicit stability analysis. The reserved structures here should be `AttractorWitness`, `StabilityRecord`, `ErgodicEnvelope`, and `BackpressureInvariant`. These are most naturally attached to coinductive runtimes, distributed builds, queueing systems, and large agent populations rather than to ordinary source-level control flow.

#### 15.7.9 Nonstandard analysis belongs under `std.experimental.nonstandard`

Hyperreal and nonstandard-analysis ideas are worth tracking, but only in a sharply constrained role. The immediate gain is **not** a replacement numeric tower for ordinary Optic programs. The more credible uses are:

- asymptotic witnesses more precise than plain Big-O for staged profitability and target routing,
- hyperfinite or nonstandard grid ideas for solver-generation libraries,
- sensitivity and approximation records attached to generated numerical artifacts.

The reserved structures should therefore be things like `AsymptoticWitness`, `HyperfiniteGrid`, `InfinitesimalSensitivityRecord`, and `NonstandardApproximationRecord` under `std.experimental.nonstandard`. The closure rule remains strict: this lane may refine analysis, solver generation, and proof artifacts, but it does **not** by itself justify changing the core numeric tower or claiming that ordinary training or optimization problems become exact by fiat.

#### 15.7.10 Simpler sidecars should stay available while v1 stabilizes

Not every unresolved question should first be answered inside the language or its graph-native experimental namespaces. Some of the most practical near-term experiments are *external companion methods* that keep the core language smaller while still answering important questions. TLA+ and Alloy are good examples for protocol, callback, and distributed-runtime design exploration. Typestate and explicit protocol automata are good examples of a narrower first answer before richer session or sheaf machinery. Abstract interpretation is a good example of a conservative optimizer and checker companion when proof-heavy approaches are not yet justified by implementation cost.

The practical rule is to prefer the smallest apparatus that closes the operational gap. If a sidecar specification or analysis tool already answers the question well enough for v1, the language should treat that as evidence against minting a new core feature.

#### 15.7.11 A practical ordering for experimentation

When several mathematically respectable answers compete, the language should not treat them as peers by default. A useful ordering for v1 is:

1. **direct internal lanes** that already speak in the language's native units (`RegionSet`, ownership fractions, `BoundaryContract`, reorder legality) — today that means `std.experimental.sep` and `std.experimental.memory`;
2. **external sidecars** that answer a design question without enlarging the language — today that means TLA+, Alloy, typestate/protocol automata, and abstract interpretation;
3. **richer second-wave theories** that may eventually sharpen the proof or consistency story once the first two layers stop being enough — today that means the `proof`, `sheaf`, `topos`, and `dynamics` tracks.

This ordering should be visible in implementation planning. Pointer provenance and local unsafe reasoning should start with region-aware resource witnesses before they are reformulated as broader categorical consistency problems. Atomics, fences, volatility, and reorder legality should start with a weak-memory lane and litmus discipline before they are pulled into richer proof artifacts. Callback, scheduler, and protocol-state questions should usually be prototyped with typestate or TLA+ before richer session or sheaf machinery is promoted. Translation validation and rewrite equivalence should begin with differential harnesses and graph-native witnesses before stronger foundational claims are made.

The point is not to dismiss the richer theories. It is to prevent the language from paying proof-theory or category-theory costs before the smaller answers have had a chance to prove that they are insufficient.

#### 15.7.12 No single experimental lane closes the whole v1 backlog

The direct lanes are strong because they answer the *current* operational questions with the fewest new concepts. They are not a claim that one experimental namespace will solve every remaining proof obligation by itself. A practical reading of the design is:

- `std.experimental.sep` answers ownership, provenance, and local unsafe-boundary questions most directly;
- `std.experimental.memory` answers atomics, fences, volatility, DMA ordering, and backend reorder legality most directly;
- `std.experimental.proof` turns whichever answers are chosen into explicit rewrite, equivalence, and translation-validation witnesses;
- sidecars such as typestate, TLA+, Alloy, and abstract interpretation continue to matter because some protocol and approximation questions are cheaper to explore there first.

This division of labour is intentional. It keeps the ordinary core language centered on costates, optics, summaries, and boundaries while still giving the project a disciplined way to investigate harder proofs and richer semantics.

| Remaining obligation | First answer to try | Why it is first | What still may be needed later |
|---|---|---|---|
| ownership/provenance/unsafe locality | `std.experimental.sep` | already speaks in resources, regions, and obligations | proof witnesses, richer categorical consistency machinery |
| atomics/fences/volatile/DMA order | `std.experimental.memory` | already speaks in accesses, events, and reorder legality | proof witnesses, richer runtime/stability theories |
| callback/protocol/runtime-state design | typestate or TLA+ sidecars | smaller and faster to iterate than new core syntax | session, sheaf, or topos refinements |
| conservative optimizer/checker approximations | abstract interpretation sidecar | integrates well with summaries and existing passes | proof artifacts if stronger guarantees become necessary |

### 15.8 Transition

A larger grade algebra is valuable only if the backend can consume the proof objects it produces. The next chapter explains the native backend path that makes those proofs mechanically useful.

### 15.9 Front-loaded decisions before the full grade algebra ships

The book previously treated several M7 questions as still open. That is no longer the right stance. These decisions affect summaries, alias checking, syntax, and artifact schemas strongly enough that they should be fixed **before** the full-language implementation work begins.

#### 15.9.1 Decision A: fractional ownership is adopted early, with named special cases

The language now commits to the fractional carrier from the start. This does **not** mean ordinary code must immediately speak in fractions. It means the compiler, summaries, and alias checker no longer pretend the discrete three-value model is the real thing.

The rationale is precise.

- The one major gain is compositional parallel-product proofs for partition-shaped programs.
- Two secondary gains are field-level lending and stronger proof transport through composition.
- The multicore payoff arrives later, but the retrofit cost would be paid early if the carrier stayed discrete.

#### 15.9.2 Decision B: asymmetric I/O optics use an explicit surface form

The surface syntax is fixed as:

```rust
AsymmetricGradedOptic<S, A, G_get, G_put>
```

with:

```rust
GradedOptic<S, A, G>
```

as sugar for the symmetric case where `G_get == G_put == G`.

The canonical composition rules are fixed early as well:

```text
(A >>> B).get_grade = combine_seq(A.get_grade, B.get_grade)
(A >>> B).put_grade = combine_seq(A.get_grade, combine_seq(B.put_grade, A.put_grade))
(A *** B).get_grade = combine_par(A.get_grade, B.get_grade)
(A *** B).put_grade = combine_par(A.put_grade, B.put_grade)
```

This front-loads the direction-sensitive semantics before network and disk costates enter the language.

#### 15.9.3 Decision C: the first session-type scope is intentionally narrow

The first `SessionGrade<T>` scope is fixed as:

- binary sessions only,
- linear channels only,
- no delegation,
- no general recursive session grammar in the surface,
- compatibility checked at optic boundaries by structural duality.

That is enough to cover the motivating client/server and request/response cases while keeping the checker and diagnostics tractable.

These decisions are front-loaded not because every benefit is needed immediately, but because the compiler structures they affect — summaries, ownership carrier, diagnostics, and artifact schemas — are expensive to retrofit once the implementation and ecosystem begin to harden.

### 15.10 Detailed implementation reference: full grade algebra and machine consequences by dimension

The full language only deserves a larger grade space if each new dimension pays for itself in better legality checks, better scheduling decisions, or better backend lowering. The sections below keep that discipline explicit.

The full language's grade algebra extends the v0 two-dimension pair to a product of eight semi-independent dimensions. Each dimension answers a different question about resource consumption.

| Dimension | Carrier | Sequential (`>>>`) | Parallel (`***`) | Machine consequence |
|-----------|---------|-------------------|-----------------|---------------------|
| `CacheGrade<n>` | `u8 \| ∞` | `sat_add` | `max` | Cache line budget; drives fusion and prefetch decisions |
| `LatencyGrade<d>` | `Duration` | `add` | `max` | Real-time bound; enforced by scheduler optic |
| `BandwidthGrade<bps>` | `u64 \| ∞` | `max` | `add` | Network/bus saturation; throttling decisions |
| `IOGrade<d>` | `Duration \| 0` | `add` | `max` | Blocking I/O budget; zero = async-only |
| `OwnershipGrade<r>` | `Fraction(0,1]` | preserve the stronger share/obligation | disjointness or partition-sum proof | Alias safety, borrow checking, partitioned parallel update |
| `SecurityGrade<L>` | `Lattice` | `join` | `join` | Information-flow: non-interference theorem |
| `SessionGrade<T>` | `SessionType` | sequential compose | pair | Protocol correctness; rules out wrong-order messages |
| `LivenessGrade<L>` | `Always \| Bounded \| OnSignal` | `max` | `min` | Loop termination; scheduler wakeup model |

The reason these are dimensions in a product rather than separate type parameters is compositional closure: when you compose two optics, you always get a single grade in the same product space. There is no need to thread eight separate type parameters through every composition site. The compiler computes the composed grade as a product-element-wise operation, checks it against bounds, and then erases it entirely before codegen.

#### 15.10.1 Why the grade algebra is a semiring, not a meet-semilattice

A naive design might use a lattice (partial order with meet and join) for grades. A semiring is strictly more expressive because it has two distinct operations whose interaction is constrained by distributivity. The product operation (`*`, sequential composition) distributes over the sum (`+`, parallel product), just as in arithmetic. This distributivity law is what makes grade checking decidable: the compiler can reduce any grade expression to a normal form using distributivity, then compare with the declared bound.

A lattice cannot express "the latency of `A >>> B` is the sum of `A`'s and `B`'s latencies" — it can only express "the latency is at most the maximum of the two." The sequential add is essential for latency budgets in real-time systems. The semiring structure provides it.

---

### 15.11 Detailed implementation reference: research integrations reserved without v0 rupture

The following material keeps the refinements from the design-spec research notes in view: two-tier grade solving, fractional ownership, asymmetric grades, session and security dimensions, gradual grades, and DST-oriented determinism hooks.

This section details how the research refinements from the design specification (§16 of the v0.3.0 spec) are integrated into the implementation plan. Each subsection names the relevant spec section, summarizes the refinement, specifies which milestone it targets, and describes any architectural preparation needed in v0.

#### 15.11.1 Two-tier grade inference (Granule precedent)

**Spec reference:** §16.1, Lesson 1.  
**Target milestone:** M7 (full language growth, grade algebra extension).

The Granule language uses Z3 over the `QF_LIA` (quantifier-free linear integer arithmetic) theory for grade constraint solving. This is sound and practically fast for typical per-function constraints.

**v0 preparation:**
- Structure grade checking as a `GradeConstraintSolver` trait (§8.5) so the concrete arithmetic solver and the Z3 solver are interchangeable.
- When adding symbolic grade parameters (e.g., `CacheGrade<2*N>`), emit constraints as `z3::Int` arithmetic expressions, not as Rust `u8` arithmetic.
- Z3 call results are cached by content hash of the optic body; a cache hit on an unchanged optic avoids re-solving.

**Approximation flag:**  
Optics that contain recursion must declare an upper-bound approximation: `optic Foo [approx] { ... }`. The `approx` flag signals that the grade is asserted (not derived) and bypasses the exact inference check. Non-recursive optics require exact inference.

#### 15.11.2 Fractional uniqueness grades

**Spec reference:** §16.1, Lesson 3 (Marshall & Orchard OOPSLA 2024).  
**Target milestone:** adopted architecturally from the beginning; exploited more fully from M7 onward.

The book now resolves this question in favor of **early adoption**. `OwnershipGrade<r: Fraction>` with `r ∈ Q ∩ (0,1]` is the underlying carrier from the first implementation onward. The named surface cases remain because they are the right teaching and authoring vocabulary for most prelude programs.

| Surface form | Underlying meaning | Rust analogy |
|-------|---------|-------------|
| `SharedGrade` | inferred read-only share `ρ`, `0 < ρ < 1` | shared borrow |
| `AffineGrade` | full share `1`, writable, droppable | owned value that may be dropped |
| `LinearGrade` | full share `1`, writable, must-use | owned value that must be discharged |
| `OwnershipGrade<1/2>` | explicit fractional share, typically for partition optics or lending | split permission / partition budget |

The genuinely important gain is compositional parallel-product proofs. Fractional ownership lets the checker preserve partition facts across `***` without rewriting the ownership model later. Two secondary gains follow from the same choice: field-level lending becomes algebraic rather than ad hoc, and alias proofs survive composition without repeated structural reinspection.

This does **not** mean the prelude must become notation-heavy. The expected style remains: write `SharedGrade`, `AffineGrade`, and `LinearGrade` unless an explicit fraction communicates a real partition or lending fact.

#### 15.11.3 Asymmetric grades for I/O optics

**Spec reference:** §16.2 (Clarke et al. 2024 mixed profunctor optics).  
**Target milestone:** M7, extension 6 (typed host costates).

For I/O optics, the `get` direction (reading from disk or network) is expensive (`IOGrade<4ms>`) while the `put` direction (writing to a pre-allocated buffer) is cheap (`CacheGrade<1>`). Forcing both into the same grade is overly conservative.

**Refinement:** Introduce `AsymmetricGradedOptic<S, A, G_get, G_put>` as a surface type alias:

```rust
optic DiskRead: AsymmetricGradedOptic<PageCache, Row, IOGrade<4ms>, CacheGrade<1>> {
    get  c => fetch_page(c, c.id)   // IOGrade<4ms> path
    put  (c, r) => { c.pages[c.id] = r }  // CacheGrade<1> path
}
```

When `G_get == G_put`, this degenerates to `GradedOptic<S, A, G>` (symmetric, the v0 default). The CGIR node gains `get_grade` and `put_grade` fields (already in `OpticSummary` from v0); the v0 compiler simply sets them equal.

#### 15.11.4 Session type grades

**Spec reference:** §16.3 (Marshall & Orchard 2022).  
**Target milestone:** M7, extension 8 (session type grades on network optics).

Session types encode communication protocols as types: `Send<Request, Recv<Response, End>>`. A `SessionGrade<T>` dimension on network optics checks that the optic's `get`/`put` sequence respects the protocol.

**Architecture:** Session types are a recursive type family. The grade checker must support recursive grade dimension resolution. The Z3 solver (§14.1) handles session type compatibility via unfolding.

**v0 preparation:** Reserve a `SessionGrade` variant in the grade dimension enum (returns `OPT-3xx` if used in v0). This ensures the diagnostic catalog is forward-compatible.

#### 15.11.5 Information-flow security grades

**Spec reference:** §16.4.  
**Target milestone:** M7, extension 9.

A `SecurityGrade<L: SecurityLattice>` dimension encodes information-flow labels. The semiring is `(Lattice, max, min, Public, TopSecret)`. A composition that routes Confidential data to a Public-grade sink is a compile-time type error.

**Architecture:** Requires the grade semiring to support lattice-ordered dimensions (not just linear-ordered). The grade combine rules in §8.2 must be extended to handle `max` and `min` differently for lattice dimensions vs. natural-number dimensions.

#### 15.11.6 Gradual grades for adoption

**Spec reference:** §16.6.  
**Target milestone:** M7, extension 10.

Gradual grade `?` allows programmers to omit grade annotations and have them observed at runtime during development. This lowers the adoption barrier for teams porting from Rust.

```rust
optic LegacyParser: GradedOptic<Connection, Request, CacheGrade<?>> { ... }
```

**Runtime behavior:** An optic with grade `?` records its actual cache cost on first call and emits a structured diagnostic suggesting the tightest concrete grade. After the suggestion is applied, `?` is replaced with a concrete value.

**Architecture:** Gradual grade `?` compiles to a wrapper that records measurements at runtime. This wrapper is stripped in `--release`. The `?` grade propagates conservatively through composition: `CacheGrade<?> *** CacheGrade<2>` = `CacheGrade<?>`.

#### 15.11.7 DST mitigations

##### 15.11.7.1 Zero-cost extension slots that v0 must reserve

The table below summarizes the key forward-compatibility hooks that should exist in v0 even before the corresponding full-language feature is enabled.

| Future feature | v0 hook to reserve now | Why reserving it is cheap | What architectural rupture it avoids later |
|----------------|------------------------|---------------------------|-------------------------------------------|
| symbolic/Z3 grades | `GradeConstraintSolver` trait | one indirection at check time, zero runtime cost | rewiring every grade call site |
| fractional uniqueness | early fractional carrier with named aliases | the harder ownership problem is solved once | avoiding a later carrier retrofit across checker, summaries, and diagnostics |
| asymmetric optics | separate `get_grade` / `put_grade` in `OpticSummary` | fields already stored; equal in v0 | refactoring summaries, diagnostics, and CGIR nodes later |
| replay / DST | `Determinism` enum + `serializable` bit | metadata only | inventing a second effect system later |
| session/security grades | reserved grade-dimension variants and diagnostics | inert enum slots | backwards-incompatible diagnostic and syntax churn |
| observability taps | reserved CGIR node variants | rejected in v0, no optimizer cost | reshaping the IR to insert debug/profile nodes later |

The discipline here is simple: reserve *structure*, not *behavior*. A dormant field or enum variant is cheap; an architectural rewrite after bootstrapping is not.

**Spec reference:** §16.5 (FoundationDB, TigerBeetle lessons).

The design spec identifies four DST failure modes. The implementation must address them:

**Problem 1 — Clock non-determinism.** All time access must go through a `TimeGrade` optic over the `HostContextLite.clock` field. The `--require-determinism` compiler flag (added in M8) warns on any optic body that calls OS time directly.

```rust
// Correct (injectable):
optic SystemTick: GradedOptic<HostContextLite, u64, SharedGrade> {
    get  ctx => ctx.clock
    put  (ctx, t) => { ctx.clock = t }
}
```

**Problem 2 — PRNG non-determinism.** Same pattern — a `RngGrade<Seeded>` optic over an injectable PRNG costate. Direct calls to `rand` in an optic body with `DeterministicGrade` are a compile error in M8.

**Problem 3 — State-space coverage.** The `.fuzz(FuzzGrade)` combinator (M8+) injects random costate mutations at optic boundaries, guided by CGIR-level coverage feedback (not branch coverage). The CGIR graph structure tells the fuzzer exactly which optic transitions have not been exercised.

**Problem 4 — Seed stability across code changes.** The `.to_integration_test("name")` method on a failing replay serialises the exact costate sequence as a named, version-stable fixture file. This is the same format as the golden fixtures in `fixtures/typeck/*.json`.

**v0 preparation:** The `HostContextLite` must expose its clock as a mutable field (not a system call) from day one. The runtime costate model must never call `std::time::Instant::now()` directly.

---

