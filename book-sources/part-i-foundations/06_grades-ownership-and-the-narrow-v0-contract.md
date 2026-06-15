## 6. Grades, Ownership, and the Narrow v0 Contract

> **By the end of this chapter, a reader should understand:** why static resource contracts belong in the type system rather than in documentation or profiling; why the grade algebra must be a semiring (not a lattice, trait bag, or ad hoc rule table); why ownership is modeled fractionally from the beginning even though most early programs use named special cases; why `u8` with 255 meaning unbounded is the correct v0 cache carrier; and what the narrow v0 contract actually promises — and does not promise — about the language's full ambition.

### 6.1 Why grades exist at all

Many languages can describe what a computation returns. Fewer can describe what resources it consumes in a form that the compiler can actually check and erase. Grades exist to close that gap.

A grade is not a profiler trace. It is not a promise of exact cycle counts. It is a static contract over a structured upper-bound approximation of the resources that matter to the language.

In the prelude, those resources are deliberately few.

- cache-sensitive field touch count,
- exclusivity and ownership mode.

That is already enough to catch meaningful bad compositions and to shape code generation.

### 6.2 Why the algebra is a semiring

The language uses a semiring because it needs two different notions of combination.

- Sequential composition adds some costs and propagates others by stricter dominance.
- Product composition combines some costs by a worst-case or concurrency-aware rule.

A lattice alone is too weak because it cannot express the essential fact that a sequential pipeline really does accumulate latency or cache pressure structurally. An ad hoc rule table is too weak because the compiler needs algebraic closure and predictable normal forms.

### 6.3 The prelude grade carrier

The prelude keeps the carrier tiny.

```rust
struct ConcreteGrade {
    cache: CacheDim,          // u8, with 255 meaning unbounded
    ownership: OwnershipDim,  // fractional carrier with named aliases
}

struct OwnershipDim {
    share: Rational,          // 0 < share <= 1
    read_only: bool,          // true for shared borrows
    must_use: bool,           // true for linear resources
}
```

The point is not to claim that two dimensions are the whole truth. The point is to choose the smallest carrier that already changes legality and code shape.

#### 6.3.1 Cache dimension

The cache dimension is a structural proxy. It counts distinct field families touched in a pass, not exact microarchitectural misses. That approximation is intentionally humble, but it is useful because it tracks the decisions the compiler actually controls: field adjacency, number of passes, fusion, and loop width.

#### 6.3.2 Ownership dimension

The ownership dimension is a semantic and operational bridge. In everyday source it still appears mostly through `SharedGrade`, `AffineGrade`, and `LinearGrade`, but those names now denote specific points inside one fractional ownership model. That matters because the compiler can keep using the same carrier when it graduates from simple field-wise exclusivity to partitioned parallelism and split-lending.

### 6.4 Why `u8` with 255 = unbounded is the right v0 cache carrier

This is one of those design decisions that looks small but carries real weight.

A `u8` carrier makes grades cheap to store, compare, serialize, and emit in diagnostics. Saturation at 255 forces the compiler to admit when it has left the bounded regime rather than silently pretending the structural approximation remained precise. That is exactly the kind of visible conservatism a first compiler should embrace.

#### 6.4.1 Grade elision and inference should be explicit surface features

The same ergonomic principle that justifies checked runtime focusing also applies to grades. The semantic core should continue to treat grades as explicit contracts, but the full language should let programmers **elide** grades the compiler can reconstruct exactly and **partially specify** grades when only some dimensions need to be fixed by hand.

Concretely, this means grade elision and inference are not merely checker accidents; they are part of the user model. A programmer should be able to write:

```optic
optic HealthView: GradedOptic<Entities, f32, _> { ... }
optic PacketPath: AsymmetricGradedOptic<Conn, Packet, IOGrade<4ms>, _> { ... }
optic KernelRing: GradedOptic<RxRing, Packet, CacheGrade<2> + _ + _> { ... }
```

and have the compiler reconstruct the missing dimensions, serialize the normalized grade into summaries and interfaces, and explain the inferred result in machine-facing terms. The intended surface is deliberately narrow:

- `_` means **infer exactly** from summaries and composition laws;
- partial elision means **infer the omitted dimensions only**;
- `?` remains the separate gradual-grade form for development-time observation rather than exact inference.

Where exact reconstruction is impossible, the language should force a choice among: explicit annotation, gradual grade `?`, or a bounded/approximate declaration. The programmer should also be able to ask the toolchain to materialize the normalized result explicitly, for example through an `optic explain-grade ...` style command or interface dump.

This keeps the surface usable without reintroducing hidden defaults. The rule is the same as for context elision: omission is allowed only when the compiler can recover the exact contract or can make the remaining uncertainty explicit. Grade elision is therefore a surface convenience over an unchanged semantic core, not a second defaulting mechanism hidden in the checker.

### 6.5 Ownership as the first unifying safety/performance grade

Ownership is where the language's systems ambitions become more than a type-system flourish.

- `Shared` optics can coexist over the same costate because they are read-only.
- `Affine` optics require exclusive access but do not enforce use.
- `Linear` optics enforce single-use handling for resources that must be consumed or discharged.

This is why the ownership grade is both a safety mechanism and an optimizer fuel source. If the compiler has a valid exclusivity proof, it can schedule and lower with fewer runtime checks and stronger alias assumptions.

#### 6.5.1 Early fractional ownership, with named cases

The design now commits to **fractional ownership as the underlying ownership carrier from the beginning**. The narrow prelude still teaches and mostly uses the readable names `SharedGrade`, `AffineGrade`, and `LinearGrade`, but those names are surface aliases over a single underlying representation rather than a temporary three-value ownership world that will later be replaced.

The important reason for doing this early is not aesthetic. It is the first genuinely compelling gain of fractional ownership for this language: **compositional parallel-product proofs**.

With the discrete `Shared/Affine/Linear` model, a product such as:

```text
PhysicsThread1 *** PhysicsThread2 *** ... *** PhysicsThread8
```

can only be justified by repeatedly re-inspecting structural region disjointness. That works for simple field-wise products, but it becomes unnecessarily conservative for partition-shaped programs where the language already knows that each optic owns a fraction of an enclosing traversal or arena. A fractional carrier lets the compiler preserve and compose those partition proofs through `***`, staging, and later fusion without turning every parallel proof into another bespoke region analysis pass.

Two secondary gains matter as well.

- **Field-level lending and split borrows become algebraic.** Temporary lending of a fraction into an inner scope no longer requires a second borrow mechanism beside the grade system.
- **Alias proofs survive composition better.** Once an ownership fraction has been established, composition need not rediscover the same exclusivity fact from field-path structure at every later node.

Historically the closest strong precedent is Clean's uniqueness typing. Clean demonstrated that destructive update and efficient interaction with the nonfunctional world can be admitted inside a high-level language when the type system can distinguish single-threaded use from ordinary shared use. Optic keeps that insight but broadens the setting: ownership is one dimension inside a larger resource algebra, and the relevant world is not a single global I/O token but explicit regions of `Runtime` and, later, `Project`.

This is also the right point to be precise about what fractional ownership does *not* do. It does not eliminate region reasoning. Structural region disjointness still proves the easy and common cases. Fractions become decisive once the program has already established a partition witness — for example, a staged split of an entity range or a work-stealing chunk boundary — and the compiler needs that proof to remain compositional instead of being re-derived from index arithmetic every time.

The book therefore adopts the following surface policy early:

| Surface form | Underlying meaning |
|---|---|
| `SharedGrade` | an inferred read-only share `ρ` with `0 < ρ < 1` |
| `AffineGrade` | full share `1`, writable, droppable |
| `LinearGrade` | full share `1`, writable, must-use |
| `OwnershipGrade<p/q>` | an explicit fractional share, primarily for partition optics and advanced lending |

Most prelude code should still use the named cases. The point of introducing the fractional carrier early is not to force every beginner example to talk about eighths and sixteenths. It is to ensure that the first real alias checker, summary format, and CGIR ownership nodes are built on the carrier the language ultimately needs rather than on a deliberately temporary substitute.

#### 6.5.2 Cross-language comparison: ownership and resource contracts across common languages

Ownership and resource accounting are the first place where readers from different language traditions feel the language's priorities most sharply.

| Language | Typical ownership/resource story | Strength | Limitation that motivates Optic's grade model |
|---|---|---|---|
| **C++** | RAII, move semantics, `const`, smart pointers, conventions for latency/cache | extremely powerful in expert hands | ownership, cache, latency, and protocol constraints live in different mechanisms, many of them conventional rather than one compositional algebra |
| **Java** | garbage collection, visibility modifiers, `synchronized`, `volatile`, executor policies | productivity and robust large-system engineering | memory ownership is intentionally abstracted away, and cache/latency claims remain comments, profiling results, or framework contracts |
| **Python** | reference counting plus GC, dynamic protocols, GIL or library-specific concurrency rules | flexibility and interactivity | almost no static resource contract survives to compilation, so performance boundaries are mostly library escape hatches |
| **TypeScript** | JavaScript GC, erased static types, API-level immutability conventions | good design-time ergonomics | ownership is not a runtime-enforced static concept, and performance behavior is delegated to the engine |
| **Rust** | borrow checker, lifetimes, `Send`/`Sync`, `unsafe` escape hatches | the strongest mainstream ownership discipline | ownership is rich, but other resource dimensions such as cache, latency, and I/O budgets are largely outside the type system |
| **Optic** | one grade product carrying ownership plus other resource dimensions | one compositional account of safety and cost | intentionally narrower at first, because the algebra must stay mechanically checkable before it grows |

The point is not to replace every other resource story. It is to unify the parts that most directly affect legality, optimization, and diagnostics inside one summary and one composition law.

#### 6.5.3 Not every boundary fact should be forced into the grade semiring

A language that already has grades is tempted to make everything a grade. That would be a mistake. Some properties are genuinely quantitative and compose algebraically. Others are not really "resources" in the semiring sense; they are boundary facts the compiler must respect.

The practical rule is simple.

- Use **grades** for facts that accumulate, bound, or distribute in a law-like way across composition.
- Use **qualifiers or contracts** for facts that are discrete, capability-like, or ABI-specific.

| Keep as grades | Keep as qualifiers or contracts |
|---|---|
| cache footprint, latency, bandwidth, blocking budget, liveness, ownership strength, NUMA penalty, atomic cost classes | ABI, calling convention, may-unwind, may-callback, reentrancy, thread affinity, privilege level, address space, volatility, layout rules, allocator contract, pinning |

The reason is not aesthetic. If every boundary property is pushed into the semiring, the algebra becomes bloated and loses the clarity that makes it useful. If none of those properties is modeled statically, the compiler loses the very facts it needs to generate correct low-level code.

This division of labor lets the current model scale with minimal disruption. The grade product remains the place where the language composes performance and ownership contracts. The summary and boundary metadata carry the non-algebraic facts that make FFI, raw pointers, callbacks, plugins, and direct hardware access safe enough to optimize around.

That distinction also helps comparisons with other languages. C++ and Rust already have many of these discrete facts in attributes, calling conventions, `unsafe` blocks, or API conventions. Java, Python, and TypeScript often hide them behind managed runtimes or extension layers. Optic's contribution is not to pretend these facts are all the same. It is to make them live in one summary model so the compiler, runtime, diagnostics, and tooling can talk about them consistently.

### 6.6 The narrow v0 contract

The narrow v0 exists to prove exactly one architectural thesis: the language can parse, type, summarize, graph, fuse, diagnose, and transpile a small optic calculus without losing either semantic clarity or machine sympathy.

The scope is therefore strict.

| Area | Implement now | Defer |
|---|---|---|
| Costates | in-memory SoA and a tiny host boundary | network, disk, GPU, distributed arenas |
| Optics | lens-like optics only | prisms, traversals, folds, setters, isos |
| Grades | cache plus ownership (fractional internally, named cases in most prelude source) | latency, bandwidth, I/O, liveness, session, security |
| Queries | `.get()`, `.set()`, `.map()` | `.parallel()`, `.coinductive()`, `.drive()`, staging |
| Backend | Rust transpiler | LLVM primary backend |
| Polymorphism | monomorphic user programs | richer generic optic abstractions |

### 6.7 What the narrow prelude proves

The prelude proves four things.

1. The optic abstraction survives compilation.
2. Grades can be checked early and erased cleanly.
3. Explicit runtime context beats ambient effect descriptions for this domain.
4. The generated hot-path code can look like hand-written index loops rather than abstraction-heavy scaffolding.

It does **not** prove that every future grade dimension will perfectly predict hardware or that every future effect-like feature should be compiled as an optic. Those are later questions.

### 6.8 Transition

The rest of the book now moves from conceptual foundation into the narrow compiler itself. The question changes from "why this design?" to "what exactly must the compiler do first?"

---

### 6.9 Bridging note: what the narrow compiler proves, and what comes after

Part I has established five foundational ideas: the implementation doctrine and the three-layer program (Chapter 1); the four machine pressures and the theory-to-machine bridge (Chapter 2); the explicit `Runtime` costate and why it beats ambient effects (Chapter 3); the `Project` semantic root that governs build and run together (Chapter 4); optics as the transformation model with composition preserved through `>>>` and `***` (Chapter 5); and grades as semiring-structured resource contracts erased before codegen (Chapter 6).

Those ideas motivate a large design space. Chapter 4 already introduced `Project`, staging, module interfaces, and native package declarations. Part III will introduce prisms, traversals, coinduction, LLVM, multicore, and the full grade algebra. Part IV will apply all of it to kernels, browsers, databases, games, and compilers.

**Part II does none of that yet.**

Part II builds a narrow compiler — a compiler that handles only in-memory SoA, lens-like optics, cache and ownership grades, a graph IR, three fusion passes, and a Rust transpiler. That narrowness is deliberate. The five chapters of Part II collectively prove a specific architectural thesis:

> The language's core abstraction — graded coalgebraic optics over explicit costates — can be parsed, typed, summarized, graphed, fused, diagnosed, lowered, and benchmarked end-to-end, with each intermediate artifact inspectable, deterministic, and traceable back to the source optic that produced it.

If that thesis fails in the narrow setting, expanding the language will only make the failure harder to diagnose. If it succeeds, the expansions in Parts III and IV each add one more link to a chain that is already trusted.

The five Part II chapters have the following specific proof obligations:

| Chapter | What it proves |
|---|---|
| 6 — Grammar and parser | Parser output is deterministic; all syntax errors are collected in one pass; operator precedence is stable and follows the theory; token boundaries are unambiguous |
| 7 — HIR and summaries | Every named optic produces a complete `OpticSummary`; cursor normalization is total and stable; name resolution is deterministic; query chain lowering is unambiguous |
| 8 — Type and grade checking | Type errors, grade bound violations, and alias conflicts all produce stable, structured, machine-readable diagnostics; no check depends on heuristics |
| 9 — CGIR and fusion | The CGIR graph is well-formed, provenance-tagged, and invariant-checked; the three fusion passes are sound (map fusion, compose fusion, product flattening) and improve generated code shape |
| 10 — Codegen and release | Generated Rust is direct, readable, and benchmarkable; release gates are objective and repository-backed; the prelude is "boring" in the specific sense the book requires |

A reader who wants to implement the compiler should read Part II front to back. A reader who wants to understand how the design scales should read Chapters 6–7, then jump to Part III, then return for the remaining detail. Both paths are valid.

---

