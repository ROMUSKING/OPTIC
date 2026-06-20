# The Optic Language Implementation Book

## From Narrow v0 to Kernel-Class Systems

**Status:** Publishable design and implementation guide  
**Audience:** Language implementers, compiler engineers, runtime authors, systems programmers, researchers, and coding agents  

---

## Abstract

This book is a complete implementation guide and design rationale for a new statically typed systems programming language whose central abstraction is the graded coalgebraic optic. The language claims that high-performance systems code — game engines, kernels, browsers, databases, realtime media pipelines, and compilers — already exhibits a recurring structure: explicit observations over flat data layouts, structured write-backs, and composable resource contracts. By making that structure first-class rather than reconstructed by convention, the language preserves the information the compiler's optimizer, alias analyser, and native backend need to generate tight, predictable, cache-aware machine code from declarative source programs.

The book begins with the narrowest compiler fragment that can test the architectural thesis honestly: in-memory structure-of-arrays data, lens-like optics, cache and ownership grades, a graph-shaped intermediate representation, three algebraically proven fusion rewrites, and a Rust transpiler whose output serves as a semantic microscope. It then grows systematically outward into prisms, traversals, coinduction, staging, a full eight-dimensional grade semiring grounded in recent type theory research, an LLVM native backend with TBAA metadata derived from region summaries, multicore partitioning, and memory-layout arithmetic. Domain playbooks demonstrate the language applied to kernels, browsers, databases, games, and self-hosted compiler tooling. A decision-rationale section records every major design choice with an explicit rejected alternative and a four-part justification: semantic obligation, compiler artifact, backend legality condition, and machine consequence. The book closes with a cross-language maturity analysis drawing on primary documentation from sixteen languages — from C++ and Rust to Go, Swift, Kotlin, Julia, Elixir, D, Nim, Pony, Idris, and ATS — to identify the late-stage failure modes that the current architecture must be designed to avoid.

---

## Preface — A Story About Abstraction and the Machine

There is a recurring failure mode in systems language design. A language begins with a beautiful abstraction story and assumes the compiler will discover the machine-level structure later. Or it begins with a machine-level story and assumes programmers will reconstruct the larger semantic picture from low-level parts. In both cases, the language eventually bends: either the abstraction becomes too thin to explain performance, or the implementation strategy becomes too ad hoc to support large programs coherently.

This book starts from the claim that those two stories should not be separated. If a language means to serve kernels, browsers, databases, game engines, media pipelines, and compilers, then the abstraction presented to the programmer has to preserve the very facts the machine cares about most: what is read, what is written, what may alias, what can be fused, what can be staged, what can be replayed, and what resource budget each transformation consumes.

That is the specific bet of the Optic language. It does not treat optics as a library convenience layered on top of an ordinary language. It treats graded coalgebraic optics as the central executable form of structured data transformation. The language says, in effect: if systems programs are already collections of repeated observations and disciplined write-backs over explicit data layouts, then the language should expose exactly that structure and keep it visible until the compiler has fully exploited it.

The intellectual lineage of that bet is broad but coherent. From categorical optics it takes the idea that focus and reinsertion are not accidents of API design but lawful structure. From graded type theory it takes the claim that resource usage belongs in types when resource composition matters semantically. From data-oriented design it takes the insistence that memory layout is not a backend detail but a first-class architectural choice. From modern compiler engineering it takes the lesson that a representation can only be optimized well if the optimizer still sees the right structure when it matters.

The implementation discipline that follows from this is intentionally strict. The narrow v0 is not a disposable prototype. It is the point where the language proves that its core abstraction survives contact with a parser, a type checker, an optimizer, a backend, and a benchmark suite. The book therefore treats the early compiler as the hard center of the language, not as scaffolding to be forgotten later. The larger ambitions — self-hosting, kernel-class systems, richer grades, network and storage costates, multicore scheduling, and a native backend — are all downstream of a small, mechanically checkable, aggressively testable core.

A reader does not need to be equally expert in category theory, LLVM, cache arithmetic, ECS architecture, and runtime systems to use this book. The point of the prose that follows is to make the causal chain explicit enough that a strong technical reader can move from one layer to another without having to guess where the semantic argument stops and the engineering argument begins.

### Relationship to existing work

This book synthesises three bodies of prior work that are rarely brought together.

**Categorical optics.** The lens concept as a coalgebra for the costate comonad was developed in the context of bidirectional model transformations and functionalised in Haskell through the van Laarhoven representation. The specific coalgebraic presentation used in this book descends from Matthew Riley's "Categories of Optics" (2018) and the generalisation to mixed V-enriched profunctor optics by Clarke, Elkins, Gibbons, Loregian, Milewski, Pillmore, and Román, published in Compositionality (2024). The language owes the lens laws, the product and composition operators, and the traversal characterisation to that line of work.

**Graded type theory.** The treatment of resource usage as semiring-annotated types builds on Robert Atkey's Quantitative Type Theory (LICS 2018), which showed how a partially ordered semiring of usage annotations can express linearity, sharing, and erasure in one framework. The practical implementation experience — using Z3 over linear integer arithmetic for grade inference, combining linear and indexed types in a working language, and adding fractional uniqueness grades for Rust-style ownership — comes primarily from the Granule project led by Dominic Orchard, Harley Eades, and Vilem-Benjamin Liepelt (ICFP 2019 and subsequent work through 2024, including Danielle Marshall and Dominic Orchard's OOPSLA 2024 paper on functional ownership through fractional uniqueness).

**Data-oriented design.** The insistence that memory layout is a first-class semantic concern rather than a backend accident reflects the data-oriented design methodology articulated by the game-engine community — Mike Acton's influential CppCon 2014 talk being the canonical practical reference — and operationalised in entity-component systems such as Bevy (Rust), Flecs (C), and Unity DOTS. The language's structure-of-arrays default, the `SoA<T>` surface syntax, and the cache grade arithmetic are direct expressions of the DoD approach applied at the type-system level.

The novel contribution of this language and book is the synthesis: demonstrating that these three bodies of work are not merely compatible but *isomorphic at the key point*. The coalgebraic optic is the categorical form of the load/modify/store loop. The grade semiring is the type-level form of the resource budgets DoD practitioners track by convention. The compiler artifacts that connect the two — `OpticSummary`, `RegionSet`, `CGIR`, `ConcreteGrade`, and `Cursor<S>` — are the bridge objects that make the synthesis executable rather than merely theoretical.

A second, more historical lineage matters as well. Several ideas that can look novel in Optic are better understood as recoveries or disciplined reframings of older traditions that never became the default systems vocabulary. Array languages such as APL made whole-array transformation primary rather than a library trick. Dataflow languages such as Lucid and SISAL kept program structure graph-shaped long enough to expose parallelism. Clean's uniqueness typing showed that destructive update and foreign-world interaction can live inside a strong static discipline. Synchronous languages such as Esterel and Lustre treated time, clocks, and reactivity as semantic structure rather than framework protocol. Capability-secure systems from Dennis and Van Horn through E treated authority as an explicit possession rather than ambient privilege. Datalog and executable attribute-grammar traditions treated querying and computing over semantic structure as a language activity in its own right. Optic does not copy any of these traditions whole. It recovers the parts that strengthen one explicit costate/optic/grade architecture and constrains the parts that history showed could become cognitively heavy, operationally vague, or ecosystem-fragmenting.

## Introduction — Three Programs That Expose the Problem

Before introducing optics, grades, or CGIR, it helps to look at three ordinary high-performance programs. They come from different domains, but they all force the same design questions.

### The first program: a game loop

A game loop updates tens or hundreds of thousands of entities every frame. Most frames touch a few hot fields — position, velocity, health, animation state, collision flags — and leave the rest alone. The dominant questions are brutally physical: are the fields laid out so the CPU can stream them efficiently, can multiple systems be fused without corrupting alias assumptions, and can the schedule be parallelized without introducing false sharing or opaque synchronization costs?

Traditional object-centric structure fights this program. A class hierarchy can describe the world, but it does not tell the compiler that a pass touches only four SoA fields in one contiguous sweep. The problem is not that the language is too expressive. The problem is that the language does not preserve the shape the hardware needs.

### The second program: a database query

A database query plan is a pipeline over structured state: scan, filter, project, join, aggregate, materialize, maybe spill, maybe stage. The dominant questions are different in detail but the same in spirit: which passes are streaming, which are blocking, which require random I/O, which can be fused, which must be staged, which consume bandwidth rather than CPU, and how much of the pipeline can be specialized once the plan shape is known?

Conventional languages can express these pipelines, but they often split the explanation across several layers: user code, query planner, executor, runtime scheduler, buffer manager, and profiler. The semantic structure becomes indirect just when optimization needs it to remain explicit.

### The third program: a network processor

A network processor — whether a TLS terminator, HTTP service, RPC fabric, or kernel packet path — lives on the boundary between semantic work and host interaction. It parses, validates, routes, transforms, and writes back under real latency and bandwidth budgets. It has to distinguish blocking from non-blocking work, compute over explicit buffer regions, and remain replayable or at least explainable when something goes wrong.

This is the domain where abstraction often fails twice. The high-level surface becomes a tangle of frameworks, callbacks, futures, or middleware layers, while the low-level performance story dissolves into a separate set of manual tuning conventions. The result is software whose architecture and performance model are explained in different languages.

### What today's language families usually give you

Every established language family solves part of this problem well. None of them solves the whole problem with one explicit semantic model.

| Language family | What it tends to offer first | What it often leaves implicit in these three programs |
|---|---|---|
| **C++** | manual layout control, templates, RAII, predictable lowering when written carefully | alias discipline, resource contracts, and large-pipeline explanation remain partly convention-driven and tool-dependent |
| **Java** | robust service architecture, mature concurrency libraries, JIT-based specialization, long-lived application engineering | memory layout, ownership, and cache behavior are largely outside the language's explicit model |
| **Python** | concise transformation code, rapid experimentation, rich library ecosystems, easy orchestration | hot-path structure is usually offloaded into extension libraries, vector packages, or external engines |
| **TypeScript** | explicit dataflow, strong API ergonomics, evented application structure, approachable structural typing | runtime representation and final performance behavior are mediated by a JavaScript engine rather than stated in the program's own semantics |
| **Rust** | ownership, explicit data structures, no-GC predictability, strong interoperability with LLVM and systems code | the resource story is still split between ownership rules, library conventions, and backend recovery of higher-level intent |
| **Typed FP / effect languages** | compositional semantics, optics, effects, coeffects, law-driven program structure | layout, cache, alias metadata, and machine scheduling often remain backend or runtime concerns rather than primary source-level structure |

The point of this table is not that existing languages are deficient. It is that each family foregrounds one layer of the problem and backgrounds another. The Optic language is trying to foreground a different unit of organization: explicit transformations over explicit context, each carrying enough static structure to be reasoned about semantically and lowered mechanically.

### The language's three-part response

The language's answer to the three archetypal programs can be stated plainly before any theory begins.

1. **Make the runtime explicit.** The program does not float in ambient services. It executes over a typed `Runtime` that makes application state, host interaction, and later control structure visible.
2. **Make transformations explicit.** The central unit is a graded optic: a lawful observation/update shape over a costate, rather than an arbitrary function that the compiler must reinterpret later.
3. **Make resource contracts explicit.** Grades are compile-time contracts over cache, ownership, latency, bandwidth, liveness, and related dimensions, so composition can be checked and optimization can be justified rather than guessed.

These three commitments are intentionally stronger than the usual promise that a sufficiently smart compiler will optimize high-level code away. The claim here is that the language should preserve the information the compiler needs instead of discarding it and hoping later passes can recover enough of it.

A corollary matters for the rest of the book: compile time and runtime should not require two unrelated languages. If the compiler already sees the program as an optic graph over explicit costates, then any subgraph whose inputs are fully known during the build can be executed during compilation and the remainder residualized into the final program. The language therefore aims for staged execution by ordinary evaluation and graph reduction, not by escaping into a separate macro DSL.

### The three readers this book keeps in view

This book is written for three readers at once.

- The **theorist** wants to know whether the abstraction is lawful, compositional, and precise enough to support reasoning.
- The **engineer** wants to know whether the compiler artifacts are concrete, deterministic, and buildable in a real repository.
- The **systems programmer** wants to know whether the final loop shape, alias metadata, memory layout, and runtime model will actually respect the machine.

Whenever a chapter introduces a new concept, the surrounding prose tries to answer the same three questions in order: what the concept means semantically, what compiler artifact it becomes, and what low-level behavior it is supposed to buy.

## How to read this book

This book is written for a reader who is technically strong but not necessarily equally fluent in every layer of the stack. Some readers will be more comfortable with type theory than cache hierarchies. Some will know LLVM and alias metadata intimately but need the optic and coalgebra background made concrete. Others will understand systems architecture well but need the compiler pipeline and grade algebra spelled out carefully. The structure of the book assumes that mixed audience.

The book therefore progresses in five regimes.

1. **Foundations.** Why the language exists, which hardware pressures it responds to, why `Runtime` is explicit, why optics and grades are the chosen compression of the problem.
2. **Narrow v0 implementation.** Concrete grammar, parser strategy, HIR, summaries, region sets, grade and alias algorithms, CGIR, fusion, Rust lowering, diagnostics, and release gates.
3. **Full-language growth.** Prisms, traversals, coinduction, staging, full grade algebra, research integration, the native backend, multicore, NUMA, and memory arithmetic.
4. **Domain playbooks.** What the language looks like when used for kernels, browsers, databases, games, realtime media, compilers, and developer tooling.
5. **Validation, rationale, and long-term maturity.** Explicit rejected alternatives, quantitative bridge rules, backend validation discipline, compatibility policy, a standing feature-admission checklist, and the late-stage ecosystem pressures the language must address before it can become a durable toolchain.

The central chain remains unchanged and should be visible in every chapter:

```text
high-level optic law
  -> explicit summary and grade rule
  -> optimizer legality condition
  -> concrete code shape
  -> measurable hardware behavior
```

Whenever the text uses a table or code example, the prose before and after it is meant to answer two questions directly:

- **Why is this structure the right semantic one?**
- **What exact low-level consequence does it buy?**

## Reader orientation by language background

The language sits at an unusual intersection. It borrows ideas from systems programming, typed functional programming, effect systems, data-oriented design, and compiler engineering. A reader's existing language background therefore changes which parts of the book feel immediately natural and which parts need more explanation.

| Background | Familiar instincts the book keeps | Habits the book asks you to suspend | Best entry chapters |
|---|---|---|---|
| **C++** | layout awareness, RAII, manual performance reasoning, template-era zero-cost instincts | ambient globals, class-centric AoS defaults, performance reasoning by convention rather than checked summaries | 2, 3, 5, 15, 16, 21 |
| **Java** | pipeline thinking, long-lived services, JIT-aware architecture, large-system modularity | reliance on object graphs, GC-opaque layout, framework-managed ambient context | 3, 5, 13, 15, 18, 21 |
| **Python** | high-level data transformation, rapid experimentation, metaprogramming intuition | dynamic late binding as the only source of flexibility, reliance on library escape hatches for performance | 2, 5, 13, 19, 20, 21 |
| **TypeScript** | structural typing, union-based API design, event-driven services, ergonomics of explicit data flow | type erasure before optimization, dependence on a JS engine for final performance behavior | 3, 12, 13, 15, 21 |
| **Rust** | ownership, explicit data structures, zero-cost abstraction discipline, LLVM familiarity | ownership as the whole static story, library-level optics as sufficient, MIR/LLVM as the first interesting IR layer | 4, 5, 9, 15, 21 |
| **Haskell / Koka / typed FP** | optics, algebraic composition, effects, coeffects, graded reasoning | treating machine layout as a backend detail rather than a first-class design pressure | 4, 12, 13, 14, 21 |

A useful reading rule is this: whenever a design choice seems surprising, ask which community would usually solve that problem differently. The comparison is often the shortest route to understanding why the book chooses a different abstraction. For that reason, major design-decision chapters include explicit comparisons to C++, Java, Python, TypeScript, Rust, and typed-functional/effect-oriented languages where the contrast clarifies the trade.

## Contents

### Front matter
- Abstract
- Preface — A Story About Abstraction and the Machine
  - Relationship to Existing Work
- Introduction — Three Programs That Expose the Problem
- How to Read This Book
- Reader Orientation by Language Background

### Part I — Foundations
Part I establishes the permanent semantic center of the language before any compiler mechanism is allowed to hide it. It begins with the implementation doctrine and the machine pressures that motivate the design, then makes `Runtime`, `Project`, optics, and grades explicit in the exact order the later compiler chapters depend on them.

- 1. Mission, Reading Strategy, and Implementation Doctrine
- 2. The Machine Problem: Locality, Latency, Bandwidth, and Explicit Structure
- 3. Runtime as Explicit Context: Costates, Host Boundaries, and Control
- 4. The Project as a Semantic Whole: Graph Store, Projections, and Build/Run Coherence
- 5. Optics as the Transformation Model
- 6. Grades, Ownership, and the Narrow v0 Contract

### Part II — The Narrow v0 Compiler
Part II turns the foundational claims into a running compiler. The sequence is intentional: grammar and parser first, then HIR and summaries, then type/grade/alias checking, then CGIR and fusion, and only then backend lowering, diagnostics, and release discipline.

- 7. Surface Language, Grammar, and Parser Strategy
- 8. HIR, Cursors, Names, and Summaries
- 9. Type Checking, Grade Inference, and Alias Safety
- 10. CGIR, Provenance, and Fusion
- 11. Rust Code Generation, Runtime Support, Diagnostics, and Release Discipline

### Part III — Extending the Core Without Breaking It
Part III grows the language only after the narrow compiler has made the core architecture believable. The chapters move from new optic kinds, to time and staging, to richer grades, to the native backend and multicore/layout consequences, so each extension reuses the proof objects the earlier chapters already established.

- 12. Why the Deferred Features Look the Way They Do
- 13. Prisms, Traversals, and the Branch/SIMD Bridge
- 14. Coinduction, Staging, Replay, and Observability
- 15. Full Grade Algebra and Research Integration
  - 15.7 Experimental mathematics tracks should enter through one reserved lane
- 16. LLVM, TBAA, Intrinsics, and Native Backend Strategy
- 17. Multicore, NUMA, Memory Layout Arithmetic, and Reclamation
- 18. Foreign Boundaries, Unsafe, and Full Generality

### Part IV — Domain Playbooks and Graph-Native Tooling
Part IV tests the language where its claims matter most: kernels, browsers, databases, games, services, compilers, and graph-resident tooling. The point is not to introduce new semantics by domain, but to show how one semantics behaves when pushed into very different operational envelopes.

- 19. Kernels and Kernel-Class Systems
- 20. Browsers and Interactive Rendering Systems
- 21. Databases, Games, and Realtime Media
- 22. Compilers, Graph-Resident Tooling, Self-Hosting, and Governance

### Part V — Validation, Rationale, and Long-Term Maturity
Part V changes mode from tutorial to validation, rationale, and closure. It records why the accepted designs were frozen, how quantitative rules connect back to the machine, how backend correctness is checked, and how the language is kept from drifting into an accumulation of loosely related mechanisms.

- 23. Language Design Rationale and Rejected Alternatives
  - 23.18 Historic paradigms recovered, reframed, and constrained
- 24. Quantitative Theory-to-Machine Bridge
- 25. Domain Blueprints and Operational Envelopes
- 26. Backend Validation, Portability, and Performance Discipline
- 27. Late-Stage Gaps, Cross-Language Lessons, and the Remaining Work
  - 27.17 Closing the Design: Core, Boundary, or Never
- 28. Feature Admission Checklist and Coding-Agent Failure Modes

### Appendices

The appendices are operational reference material rather than leftovers. They hold the stable schemas, grammars, milestone gates, reference tables, and policy ledgers that let the main body stay readable while still remaining executable as an implementation guide.
- Appendix A — Diagnostic Catalog
- Appendix B — Command Surface and Repository Layout
- Appendix C — Milestone Ladder
- Appendix D — Normative v0 EBNF
- Appendix E — Decision Matrix and Arithmetic Reference
- Appendix F — Boundary Contracts, Unsafe/FFI Reference, and Diagnostic Families
- Appendix G — Selected External Reference Points and the Lesson Each One Contributes
- Appendix H — Compiler Graph Store, Projection Filesystem, and Tool Protocol Reference
- Appendix I — Soundness Budgets, Artifact Publicity, and Self-Hosting Layer Split
- Appendix J — Native Project Queries, Graph Transactions, and the Semantic Query Engine
- Appendix K — Repository Agent Operating System and Cross-Tool Compatibility

# Part I — Foundations

Part I is arranged as a six-step argument, and the order matters. Chapter 1 states the implementation doctrine and success conditions so the reader knows what kind of book this is and what counts as evidence. Chapter 2 starts from the machine pressures—locality, latency, bandwidth, and explicit control—because the semantics only make sense if those pressures are visible first. Chapter 3 then makes the running program explicit through `Runtime`, and Chapter 4 widens that same move into `Project`, `BuildRuntime`, and graph-resident projections so the build and tooling story does not become a second model later. Only after those roots are in place does Chapter 5 introduce optics as the transformation form, and Chapter 6 turns resource and ownership constraints into grades and the narrow v0 contract. By the end of the part, the reader should understand not only what the semantic center is, but why each chapter had to appear before the next one.



Two practical consequences should be kept in view from the start. First, the graph-first architecture is not a declaration that ordinary files, editors, or UNIX-style workflows are obsolete; text remains the primary human editing and review surface, with the graph providing the compiler's stronger semantic center underneath it. Second, the emphasis on in-memory SoA in the early chapters names the narrowest proving ground, not the only admissible runtime shape. Later chapters must still justify AoSoA islands, arena-lowered trees, and bounded pointer-heavy regions under the same summary and legality model.## 1. Mission, Reading Strategy, and Implementation Doctrine

> **By the end of this chapter, a reader should understand:** the three-layer implementation program and why Layer 1 (the narrow v0) is treated as the permanent semantic foundation rather than a disposable first draft, what the implementation doctrine requires and why each rule is load-bearing, and what the book treats as non-goals of the prelude and why those deferrals are disciplined choices rather than omissions.

### 1.1 The book's stance

The implementation program has three layers.

- **Layer 1: narrow v0.** Parse a small language, infer and check a small grade algebra, produce a clear coalgebra graph IR, run a minimal set of sound fusion rewrites, and transpile to readable Rust.
- **Layer 2: full language.** Add richer optics, more grade dimensions, observability, replay, staging, and a real native backend without weakening the guarantees established in Layer 1.
- **Layer 3: systems ambition.** Self-host the compiler, grow a `no_std` runtime, and use the same costate/optic discipline to build kernel-class systems.

The narrow v0 is not a demo, a toy, or a disposable prototype. It is the permanent semantic foundation. If the language cannot make its basic promises inside that small fragment, then adding more expressive surface features only hides the failure.

### 1.2 How the book builds progressively

A reader who comes from type theory may be tempted to begin with optics, prisms, or the comonadic story. A reader who comes from systems programming may want to begin with cache lines, SIMD, io_uring, or TBAA metadata. An implementer may want to jump directly to the parser, HIR, and codegen sections. All three perspectives are valid. The problem is that none of them, by itself, tells the whole story.

The book therefore moves in a strict progression.

First it explains the pressure exerted by hardware and systems workloads. That pressure explains why the language rejects ambient effects, opaque runtime schedulers, and AoS-by-default layout assumptions. Next it explains the semantic core that compresses those pressures into one uniform model. Only after that does it spell out how to build the compiler and why the compiler artifacts have the shapes they do. The domain chapters come last because they are not the source of the design; they are the tests that the design must ultimately pass.

### 1.3 Implementation doctrine

The doctrine is intentionally strict.

| Rule | Why it exists | What it prevents |
|---|---|---|
| Build in vertical slices | each milestone must end in a runnable system | speculative architecture without working evidence |
| Keep HIR and CGIR explicit | semantic structure must survive until legality is proven | rediscovering optic intent from flattened control flow |
| Reject unsupported features loudly | capability gates are part of correctness | silent fallbacks that destroy trust |
| Erase grades only after checking | grades are contracts, not runtime values | ghost metadata leaking into codegen |
| Prefer readable first codegen | semantic debugging matters more than heroic lowering in v0 | backend cleverness masking front-end mistakes |
| Preserve provenance through fusion | source optics remain the unit of explanation | profiling and debugging collapse after optimization |
| Design diagnostics for agents as well as humans | the compiler is a collaboration tool | noisy, unstable, unranked error output |

Each doctrine item has a type-theoretic side and a machine side. For example, keeping CGIR above SSA is not merely a compiler engineering preference. It is the implementation form of the claim that composition, product, traversal, staging, and coinduction are primary structures of the language rather than patterns to be reconstructed later.

### 1.4 Success criteria

The language has earned the right to expand when the following are all true at the same time.

- The v0 compiler accepts a small but non-trivial example suite end to end.
- The generated Rust is readable enough that an implementer can audit whether the abstraction disappeared.
- Grade errors, alias errors, and type errors are deterministic, structured, and repairable.
- The optimizer performs a small rewrite set with clear proof obligations.
- Benchmarks show that the generated loops are close to the hand-written baselines they are supposed to replace.

If any of those are missing, the book treats that as a language-design failure, not a tooling inconvenience.

### 1.5 Non-goals of the prelude

The prelude is not trying to prove everything.

| Deferred topic | Why it is deferred |
|---|---|
| Full LLVM backend | a readable Rust backend is the semantic microscope for v0 |
| Symbolic grades | concrete bounded grades keep inference tractable and diagnostics crisp |
| General resumptive control effects | continuation machinery should not be introduced before the costate core is trusted |
| GPU and distributed costates | they add locality and scheduling models beyond the first implementation envelope |
| Kernel boot and drivers | kernel-class work is a downstream systems program, not the first proof target |
| Self-hosting | self-hosting should be a result of stability, not a substitute for it |

The important point is not that these are unimportant. It is that they only become believable after the smaller claims have been made true in code.

### 1.6 Transition

The rest of Part I answers the question that every ambitious systems language must answer early: why does the semantic model look the way it does instead of like effect rows, ambient runtime services, traits over iterators, or a conventional borrow-checked SSA compiler? The answer begins with the machine.

## 2. The Machine Problem: Locality, Latency, Bandwidth, and Explicit Structure

> **By the end of this chapter, a reader should understand:** the four recurring machine pressures that high-performance systems code must navigate (locality, latency, bandwidth, explicit control), why those pressures point toward explicit context rather than ambient effects, what the theory-to-machine bridge table means and why each row connects a semantic abstraction to a specific compiler artifact and machine-level consequence, and why in-memory SoA is the right first proof target.

### 2.1 Why systems software keeps rediscovering the same patterns

Systems programmers keep arriving at the same low-level truths because the hardware keeps charging the same debts.

A CPU does not execute types. It executes loads, stores, branches, address calculations, fences, and calls. Memory does not arrive as an abstract container of values; it arrives in cache-line-sized chunks. A scheduler does not care that a handler was elegant; it cares whether the handler blocked, missed its wakeup, or bounced cache lines across cores. A network server does not care that a parser was generic; it cares whether the hot loop copied data, mispredicted branches, or forced the runtime to allocate.

This is why high-performance code repeatedly converges on a small repertoire of patterns:

- structure-of-arrays instead of object graphs in the hot path,
- explicit state machines instead of ambient control,
- precomputed plans instead of repeated dynamic interpretation,
- fused loops instead of abstraction towers,
- domain-specific schedulers instead of opaque concurrency defaults.

The language in this book is an attempt to treat those patterns as semantic facts instead of folklore.

### 2.2 The four recurring machine pressures

The design is driven by four pressures.

#### 2.2.1 Locality

If the machine fetches 64 bytes and the program uses 4, the program is paying for 60 bytes it did not need. AoS layouts, pointer chasing, and late-bound indirection all amplify that waste. Any language that wants systems credibility must give the compiler a clean way to preserve and exploit locality information.

#### 2.2.2 Latency

Sequential composition is not free merely because the source language made it look declarative. If stage A costs 1 microsecond and stage B costs 2 microseconds, the whole pipeline costs roughly 3 microseconds before contention and scheduling overhead are even considered. That is why the language's grade algebra must have a true sequential composition operator rather than a generic lattice join.

#### 2.2.3 Bandwidth

A system can be CPU-light and still fail because it saturates memory bandwidth, PCIe, NIC throughput, or storage queues. A language that only models CPU-side control and ignores data movement cannot state the contracts that systems programmers actually need.

#### 2.2.4 Explicit control and ownership

A kernel, browser, game engine, or database survives on clear resource boundaries. Hidden shared state, implicit retries, invisible handlers, and untyped resource duplication are not ergonomic luxuries. They are system-level liabilities.

### 2.3 Why these pressures point toward explicit context

The conventional systems-language answer is a mixture of techniques.

- ownership or borrow checking for alias control,
- effect conventions in APIs for I/O and time,
- hand-rolled schedulers for live systems,
- profiling and careful review for locality,
- special runtime systems for replay, tracing, and benchmarking.

Those techniques work, but they are not one coherent model. The language here tries to compress them into a smaller number of first-class ideas.

The first compression is to make program context explicit. Rather than pretending that the environment is ambient and then rebuilding it in APIs, the language models the program's world as a typed root value, `Runtime`. Rather than hiding state updates inside arbitrary effects, it models structured observation and reinsertion with optics. Rather than leaving resource usage as a comment or benchmark surprise, it models it as a grade carried by the optic.

### 2.4 The theory-to-machine bridge in one table

The table below is the operational spine of the book.

| High-level idea | Static artifact | Backend or optimizer use | Machine-level consequence |
|---|---|---|---|
| explicit runtime context | `Runtime = AppWorld × HostContext × ControlRuntime` (full language: `Project` with `BuildRuntime` and `Runtime` as projections — see Ch. 4) | region-rooted summaries and capability boundaries | clearer replay, ownership, and host-interaction rules; hot loops over `AppWorld` stay free of host indirections |
| optic focus and reinsertion | `get`/`put` plus `Cursor<S>` | load/store synthesis and legality checking | direct index loops rather than iterator-shaped guesswork |
| resource contracts | `ConcreteGrade` and later full grades | bound checks, scheduling constraints, backend hints | fewer runtime checks and better hardware alignment |
| structural alias model | `RegionSet` and `OpticSummary` | product legality and TBAA generation | reordering and vectorization without unsound alias assumptions |
| graph-shaped compilation | `CGIR` | fusion, stage separation, provenance retention | one-pass loops and source-attributable optimization |

This table is worth lingering over because it explains why the language is neither "just optics" nor "just a systems DSL". It is a claim that the same set of structures can serve both semantic explanation and machine guidance.

### 2.5 Why the first target is in-memory SoA

The prelude chooses a tiny operational theater on purpose: in-memory SoA plus a narrow host boundary. The reason is not that the language only cares about games or ECS workloads. It is that SoA is the simplest domain in which locality, ownership, fusion, and clear code shape all show up at once.

If the language cannot generate convincing loops over a flat SoA arena, it has no business claiming that it will later generate convincing event loops over io_uring rings, page table walkers, or rasterization passes.

#### 2.5.1 SoA is the first proof target, not the universal decree

The important qualifier is that SoA is the first **proof target**, not the only shape the language will ever admit. It is chosen because it makes locality, alias safety, fusion, and direct loop lowering easiest to see and easiest to falsify. If the core abstraction fails here, it is not ready for richer runtime worlds.

#### 2.5.2 The architecture must later justify hybrid and graph-shaped layouts too

The same summary and region machinery will eventually need to justify more than flat arrays. Hybrid AoSoA islands, arena-lowered trees, DOM- and AST-like structures, and carefully bounded pointer-heavy regions all belong in the longer-range design. The real claim is therefore narrower and stronger: SoA is where the language proves itself first, and later chapters must show that the same legality model scales when the data is no longer perfectly flat.

### 2.6 Transition

Once the machine problem is understood, the next question is how to compress it into one uniform program model without erasing the distinctions that matter. That is what the runtime and costate story is for.

## 3. Runtime as Explicit Context: Costates, Host Boundaries, and Control

> **By the end of this chapter, a reader should understand:** why the language makes the program's execution context an explicit typed value rather than leaving it ambient, what the three components of `Runtime` are and why each is distinct, how the costate comonad connects data-as-storage to data-as-observation-context, and what the compiler's internal effect/coeffect judgment looks like in the prelude.

### 3.1 The root idea

The language treats the program not as a collection of floating functions but as a graph of transformations over an explicit root context.

```text
Runtime = AppWorld × HostContext × ControlRuntime
```

In the prelude, this root is narrowed to:

```text
Runtime_v0 = AppWorld × HostContextLite
```

That reduced form is enough to prove the architecture without pretending to solve networking, disk, clocks, randomness, or control effects all at once. `AppWorld` contains the user's SoA data arenas. `HostContextLite` contains a deterministic clock counter and nothing else.

This decomposition answers several questions at once.

- It gives effectful code a concrete place to live: writes go to named regions of `AppWorld` or `HostContext`, not into ambient state.
- It distinguishes semantic state (the entities, DOM nodes, pages, IR nodes, or rows the program reasons about) from host-facing interaction (clocks, sockets, file handles, GPU queues).
- It gives diagnostics and tooling a stable root from which region paths like `app.healths[*]` or `host.clock` can be constructed.
- It turns environment dependence into something the type checker and optimizer can both see directly.

**Full-language note.** As the language matures beyond v0, the program root expands to include a `Project` value that encapsulates both the durable build graph (source, compiled artifacts, target profiles, runtime blueprints) and the live runtime. The `Project` architecture — including graph-resident compilation, projections over the same costate, and the Smalltalk/Unison/MPS/Nix comparison — extends exactly this runtime logic upward. Chapter 4 makes that enlargement explicit so later discussions of staging, tooling, interfaces, and self-hosting can keep reusing the same vocabulary instead of introducing a second model.

### 3.2 Why `Runtime` is better than ambient effects for this language

A conventional effect system is good at telling us *what kind* of operation may occur: state, exception, I/O, time, randomness, and so on. For this language, that is not enough. The compiler also needs to know *where* the data lives, *which* region is accessed, *how* it is laid out, and *what* ownership and locality assumptions accompany the access.

`Runtime` solves that by turning ambient context into structured data. A parser does not merely have a generic "may read input" effect. It reads `Runtime.host.net.rx[*]` or `Runtime.host.file.buf[*]`. A physics system does not merely have "state"; it reads `Runtime.app.positions[*]` and writes `Runtime.app.velocities[*]`.

This is what lets the language model coeffects and effects in one place. The context requirement is a region set. The write effect is a region set. The resource contract is a grade. The whole triple can then be checked, optimized, and diagnosed together.

#### 3.2.1 Cross-language comparison: how other language families usually carry context

Readers from different language traditions often ask whether `Runtime` is just a renamed object graph, dependency bundle, module namespace, or service container. It is close to each of those in one respect and sharply different in another. The difference is that `Runtime` is not merely the thing you pass around. It is the static root from which the compiler derives region paths, grade summaries, determinism classes, replay boundaries, and backend legality facts.

| Language or family | Usual way context is carried | What that style gets right | Why Optic still chooses an explicit `Runtime` root |
|---|---|---|---|
| **C++** | explicit parameters, objects, RAII handles, sometimes globals or singletons | excellent local control and low overhead when the architecture is disciplined | the whole reachable context is rarely summarized as one typed root, so alias and replay reasoning remain partly conventional |
| **Java** | object graphs, dependency injection containers, thread locals, framework services | scalable application architecture and clear long-lived service boundaries | framework-managed context is often ambient from the compiler's point of view, so the optimizer cannot derive region-level contracts from it |
| **Python** | module globals, closure captures, context managers, runtime registries | flexibility and low ceremony | dynamic context is easy to evolve but difficult to summarize statically enough for layout and replay guarantees |
| **TypeScript** | object graphs, stores, closures, dependency injection, the JS event loop | explicit API shape and ergonomic composition | types erase before execution, so the compiler cannot keep context structure alive as optimizer-facing evidence |
| **Rust** | explicit parameters, typed state structs, traits, ownership-aware APIs | the closest mainstream relative in spirit | ownership helps with aliasing, but the language does not treat the whole reachable environment as a first-class effect/coeffect root by default |
| **Effect languages** | effect rows, capabilities, evidence passing, handlers | strong reasoning about what ambient operations are permitted | effect names alone usually do not identify concrete field paths or storage regions strongly enough for layout- and alias-driven code generation |

The purpose of `Runtime` is therefore not stylistic tidiness. It is to make the complete program-visible world available in a form the compiler can summarize, grade, and lower without reconstructing it later.

### 3.3 Costates: the semantic reading

The costate comonad provides the clean semantic compression.

```text
CoState(S, A) = (A, S -> A)
```

Read operationally, that means: a focused value of type `A` observed out of a larger state `S`, together with a way to put a new `A` back into `S`.

This is why the language treats data as costate rather than as a passive heap. A data arena is not just storage. It is the context over which structured observations and updates are defined. That is what makes lenses the natural optic in the prelude and why other optic varieties can later be added without changing the core picture.

### 3.4 Host context and why it stays explicit

The host side of `Runtime` is not an implementation detail. It is where clocks, randomness, network queues, file pages, GPU submissions, windows, input streams, and tracing sinks live.

The book insists on keeping host context explicit because the operational differences matter. Blocking and non-blocking I/O are different. Local and NUMA-remote memory are different. Read-only shared data and affine handles are different. If the language tried to collapse all of that into one generic effect marker, it would lose the information that backend and scheduling decisions require.

**Full-language note.** The host context expands significantly in the full language. Foreign libraries, device registers, DMA rings, plugin ABIs, graphics APIs, audio backends, scripting runtimes, and managed heaps all become host-facing regions with typed contracts. That expansion is covered in the Interlude chapter (Foreign boundaries, unsafe, and full generality) and in Chapter 4 (§4.6). In v0, `HostContextLite` holds only a deterministic clock field.

### 3.5 Why the prelude separates `AppWorld` from `HostContextLite`

The two-component `Runtime_v0 = AppWorld × HostContextLite` split is not just a size constraint. It embodies a performance and correctness principle that holds in the full language too.

Code that only touches `AppWorld` — the flat SoA arenas — is eligible for all of the optimizations the book is building toward: fusion, vectorization, TBAA-backed alias freedom, deterministic replay, and parallel partitioning. The compiler knows, from the static structure of `AppWorld`, that no host side effect can interfere with a traversal.

Code that touches `HostContextLite` — currently just a clock field — must be handled differently. A read from the clock is not a pure load; it has a determinism class and cannot be freely reordered with SoA writes. By putting the clock in `HostContextLite` and not in `AppWorld`, the compiler can prove that the game-loop SoA traversal does not secretly read the clock — without inspecting the traversal body.

This is the minimal-case demonstration of why explicit runtime context is more powerful than ambient effects. An effect system can tell us "this function reads the clock." The explicit `Runtime_v0` structure tells the optimizer "this traversal over `app.healths[*]` does not touch anything in `HostContextLite`" — which is the stronger fact it actually needs.

The full-language `Project` architecture (Chapter 4) builds on this same principle: explicit roots, explicit projections, explicit boundaries. Every layer of the system benefits from knowing *exactly* what it touches.

### 3.6 Control runtime and the boundary with general resumptions

The full long-range runtime model includes `ControlRuntime` because live systems, staging, coinduction, and any future handler-like features eventually need a home for control state. The prelude does not yet model general resumptions, captured continuations, or handler stacks as first-class surface constructs. That is intentional.

The correct long-term picture is:

```text
ControlRuntime = KontStack × HandlerStack × FrameStack
```

but the first compiler does not need to expose or optimize against that structure directly. It only needs to reserve the architectural space so later features do not require a semantic rupture.

### 3.7 Effect and coeffect judgment

With `Runtime` in place, the compiler's internal judgment can be stated plainly.

```text
Runtime ⊢ action : requires C ; writes W ; grade G ; determinism D
```

- `requires C` is the coeffect side: which regions must be readable.
- `writes W` is the effect side: which regions may change.
- `grade G` is the resource contract.
- `determinism D` is the replay or scheduling class.

This is a stronger statement than the usual "this function has state and I/O" description. It is specific enough for alias analysis, backend lowering, scheduling, and diagnostics.

### 3.8 Checked focusing, context elision, and zero-cost surface ergonomics

An explicit `Runtime` root is essential to the compiler's reasoning, but the surface language should not force programmers to spell the full root path at every use site. The right answer is not ambient dependency injection, service locators, or hidden thread-local context. The right answer is **checked focusing**: a surface form that lets the programmer name the relevant region once while the compiler proves that the elided path still lowers back to the same explicit root.

#### 3.8.1 Surface direction

The full language should admit focused blocks and focused parameter requirements over explicit roots, for example:

```optic
with runtime.app.world.physics {
    query(Integrate >>> BroadPhase >>> NarrowPhase).drive()
}

fn update_player(using app.world.players, host.clock) { ... }
```

These forms do not create a second context model. They are checked abbreviations over already explicit roots.

#### 3.8.2 Why this is not ambient dependency injection

Ambient dependency injection and service locators are rejected for the same reason ambient effects are rejected: they hide the true root from summaries, alias rules, replay, and staging. Checked focusing is different. The focused path must still be derivable from the enclosing `Runtime` or `Project` root, and the compiler must be able to recover that path mechanically.

#### 3.8.3 Lowering rule: elision becomes explicit root paths again

A focusing construct is legal only when it lowers back to ordinary explicit path reads and writes before summary construction finishes. That means the semantic artifacts remain unchanged: `RegionSet`, `OpticSummary`, determinism classes, and boundary checks still speak in terms of explicit root-relative paths.

#### 3.8.4 Diagnostic consequences

Because focusing is checked rather than ambient, the compiler can explain failures precisely. Typical failures are: the focused path is ambiguous, the elided form crosses a forbidden build/runtime boundary, or no zero-cost focus exists for the requested path. The same ergonomic rule should later apply to grades: grade elision and inference are acceptable only when the compiler can reconstruct the exact contract and show the inferred result explicitly.

### 3.9 Transition

Making runtime explicit tells us what the program transforms. The next step is to explain the shape of those transformations. That is where optics enter as the uniform language of structured focus, update, and composition.

---

## 4. The Project as a Semantic Whole: Graph Store, Projections, and Build/Run Coherence

> **By the end of this chapter, a reader should understand:** why `Project` sits above both `Runtime` and `BuildRuntime` rather than beside them, how the project graph functions as a durable costate rather than as a bag of compiler caches, why Smalltalk, Unison, projectional systems, and Nix each illuminate one part of the design without supplying the whole answer, why text remains a first-class writable projection instead of a legacy compatibility layer, and how this single-root architecture prepares staging, graph persistence, tool protocols, native package declarations, and self-hosting without introducing a second semantic center.

Chapter 3 made the running program explicit. This chapter lifts that same idea to the scale of the whole software system. `Project` is the durable semantic root that keeps source, build state, artifacts, projections, and runtime blueprints inside one typed structure, so later chapters can talk about staging, graph persistence, tool protocols, package declarations, and self-hosting without introducing a second model.

### 4.1 From Runtime to Project

Chapter 3 established that `Runtime = AppWorld × HostContext × ControlRuntime` is the explicit operational context for a running program. That is correct and sufficient for the prelude. But a real language development environment needs a stable structure above any particular running instance — one that encodes the source, the build graph, the target configurations, the artifact cache, and the runtime blueprints from which individual `Runtime` instances are spawned. That structure is `Project`.

```text
Project = ProjectGraph × RuntimeBlueprintSet

ProjectGraph =
    SourceGraph
  × PackageGraph
  × ModuleGraph
  × CompilerGraph
  × ArtifactIndex
  × ProjectionTable

BuildRuntime = Project.build
             = CompilerGraph × TargetProfile × BuildHostContext × ArtifactCache

Runtime = instantiate(Project.runtime[target], AppWorld, HostContext, ControlRuntime)
        = AppWorld × HostContext × ControlRuntime
```

`Project` is the durable, typed root of the whole software system. `BuildRuntime` is the build-time projection of that root: the context in which the compiler, type checker, optimizer, and staging system execute. `Runtime` is the live execution instance for one target.

This three-way split solves a naming and a semantic problem together. Earlier versions of the book needed both `Runtime` and `BuildRuntime` without an enclosing root. Once `Project` exists, they are not two unrelated worlds. They are two regimes — build and run — over one project-shaped whole.

The key invariant is that none of the three should be confused with the others:

| Context | Typical readable regions | Typical writes | Why the distinction matters |
|---|---|---|---|
| `Project` | source text, native package/workspace declarations, module interfaces, compiler summaries, runtime blueprints, projection metadata | graph revisions, projection materialization, artifact linkage | one durable source of truth above build and run |
| `BuildRuntime` | compiler graph, target profile, cached artifacts, recorded build inputs, whitelisted build host data | generated artifacts, specialization cache entries, derived tables | reproducibility, cache keys, and toolchain determinism |
| `Runtime` | live sockets, device state, timers, page caches, input streams, GUI surfaces | live buffers, queues, world state, scheduler-visible structures | latency, liveness, ownership, and real host interaction |

### 4.2 `Project` as a semantic image, not a heap image

Readers who know Smalltalk or other image-based environments will immediately see a family resemblance: a world where the compiler, indexer, refactoring tools, debugger, build planner, and package logic all operate over a living semantic structure rather than constantly rediscovering meaning from disconnected text files. That resemblance is real and worth preserving.

But `Project` is emphatically **not** a mutable heap image where every object that happened to exist during development becomes language truth. The authoritative thing is a typed, revisioned semantic graph. It has stable regions for source, summaries, interfaces, artifacts, runtime blueprints, and projections. It can be snapshotted, journaled, hashed, diffed, validated, and replayed. That distinction is what makes the model suitable for reproducible builds, self-hosting, multi-target compilation, and kernel-class systems.

#### 4.2.1 Four prior systems each illuminate one piece

The design draws a specific lesson from each of four prior environments, and refuses a specific part of each.

| Prior system | Lesson the language keeps | Lesson it declines |
|---|---|---|
| **Smalltalk images** | tools should operate over live authoritative semantic state, not stale text re-parses | the mutable heap image as the canonical artifact — Optic needs typed, revisioned, reproducible state |
| **Unison** | semantic identity should be stronger than filename identity where meaning is stable | a purely immutable content-addressed regime for all state — the compiler has extremely hot mutable work queues and provisional analysis caches |
| **Projectional editors (MPS)** | one semantic structure can support many coherent views — text, HIR, CGIR, diagnostics, build plans, interfaces | requiring a projection-only editor as the only participation model — text must remain a first-class, lossless, grep/diff-friendly projection |
| **Graph-first build systems (Nix)** | build closure discipline and reproducible artifacts are architectural requirements, not late-stage additions | a second authored build language that drifts from the semantics of the main language — build declarations belong in native source |

The synthesis is: `Project` should have the **semantic liveness** of a Smalltalk image, the **stable meaning** of a Unison-style codebase, the **many-views** discipline of a projectional system, and the **input-closure discipline** of a graph-first build tool — while remaining a systems language whose graph must survive target selection, FFI boundaries, and low-level layout reasoning.

### 4.3 The compiler graph as a costate

The compiler graph is best understood as the hot build-time sub-costate of a larger durable `Project` graph.

```text
CompilerGraph = ProjectGraph ⊳ compiler-facing projection

Typical regions of ProjectGraph:
    TextArena          -- authored UTF-8 source chunks and stable rope pieces
  × SyntaxArena        -- parsed syntax/HIR nodes
  × HirArena           -- resolved and typed HIR
  × SummaryTable       -- hot fixed-size OpticSummary and grade facts
  × CgirArena          -- CGIR nodes and edges
  × DiagnosticArena    -- current structured diagnostic stream
  × ModuleInterfaceArena  -- public semantic surface of each compiled module
  × ArtifactIndex      -- references to generated artifacts (content-addressed sidecars)
  × RuntimeBlueprintIndex -- target/runtime family blueprints
  × ProjectionTable    -- mounted views and per-tool cursors
  × WatchTable         -- subscriptions and invalidation streams
```

This is the same language idea used everywhere else in the book — the project graph is a **costate**; compiler passes are **optics** over its compiler-facing projection; source files, HIR dumps, diagnostics, module interfaces, and generated artifacts are **projections** of that costate; staging evaluates ordinary optic graphs over `BuildRuntime`, which is itself a projection of `Project`.

#### 4.3.1 Why a persistent graph store belongs inside the model

Most mature compilers eventually accumulate the same pieces whether they plan to or not: a text store, symbol tables, typed IR, module interfaces, cache keys, generated artifacts, diagnostics, and invalidation metadata. If these are scattered across ordinary files, cache directories, ad hoc databases, and process-local maps, the toolchain spends its time translating between representations that all describe the same program.

The language already has a better idea available: make the project graph itself explicit and keep it authoritative. Then every other surface becomes a view. The storage ethos is borrowed from high-performance systems software: one authoritative file, static or mostly-static allocation discipline, fixed hot records, append-oriented mutation, and a serially ordered state-machine core.

The practical persistent layout:

| Region | Typical contents | Access pattern | Layout preference |
|---|---|---|---|
| superblock | schema version, workspace id, root pointers, generation counters | read on open, rare write | one page, fixed offsets |
| transaction journal | batched edits, invalidation events, commit markers | sequential append | append-only |
| text and string arenas | source chunks, interned identifiers, paths | read-heavy, append on edit | chunked immutable blobs + interner |
| syntax/HIR/CGIR arenas | packed node records and edge lists | random read, append on rebuild | fixed headers + variable tails, cache-line aligned |
| summaries/diagnostics | hot fixed-size facts keyed by node id and revision | extremely hot lookups | dense tables |
| projection/watch tables | mounted views, subscriptions, per-tool cursors | mixed | compact mutable tables |
| artifact index | hashes, sidecar references, materialization metadata | read-heavy | compact key/value table |

Memory-mapped persistence is the natural first storage strategy because it provides stable object identity across phases, zero-copy reads for hot metadata, append-friendly updates, cheap snapshots, and a layout the compiler controls as aggressively as it controls user-facing SoA layouts.

### 4.4 Projections reuse the optic model directly

Once `Project` is authoritative, a source file stops being the primitive unit and becomes one important projection among several:

- the authored source text is a writable text projection,
- parsed syntax is a structured projection,
- HIR, summaries, CGIR, diagnostics, module interfaces, package plans, runtime blueprints, and build artifacts are further projections,
- some projections are editable, some are derived and read-only, some are materialized only on demand.

This is exactly optic-shaped. A projection is a focused view with explicit reinsertion rules:

```text
SourceTextProjection      : GradedOptic<ProjectGraph, Utf8Text,     Affine>
HirProjection             : GradedOptic<ProjectGraph, HirView,      Shared>
CgirProjection            : GradedOptic<ProjectGraph, CgirView,     Shared>
DiagnosticProjection      : GradedOptic<ProjectGraph, DiagStream,   Shared>
RuntimeBlueprintProjection: GradedOptic<ProjectGraph, BlueprintView,Shared>
```

The filesystem and direct tool-protocol stories should therefore be understood as two host interfaces over the same projection layer, not as competing sources of truth.

#### 4.4.1 Text remains the primary human working surface

The graph is authoritative for the compiler, but authored text remains the primary human surface for editing, review, grep, diff, and patch exchange. The point of graph authority is not to demote text into a legacy export. It is to keep semantic identity, summaries, artifacts, and build-time structure coherent underneath the text that humans already know how to work with.

#### 4.4.2 Lossless round-tripping and tool compatibility are language obligations

That means projection support is not optional polish. The language should promise that ordinary source modules round-trip losslessly between text and graph form, that first-party tooling can materialize a file tree on demand, and that editor integration ships with an LSP-compatible adapter from day one. The direct graph protocol may remain semantically primary, but file-centric workflows remain a permanent supported mode rather than a temporary migration concession.

### 4.5 Module interfaces and separate compilation are projections too

A module interface artifact sits between "one module compiled" and "whole program linked." It is not an alien build-system file. It is another semantic projection — the public semantic surface that downstream compilation needs without forcing a reparse or full re-analysis.

A workable interface projection should contain:

- exported type and optic signatures,
- public `OpticSummary` data and grade/boundary facts downstream code may rely on,
- edition, target-profile family, capability requirements, and staged-build exports,
- provenance roots and dependency-interface hashes,
- and enough compatibility metadata that diagnostics and incremental builds remain deterministic.

The artifact-publicity policy flows from this:

- **Public stable artifacts:** module interfaces, generated lock snapshots, registry publication records — versioned and migratable across editions
- **Toolchain-stable artifacts:** diagnostics schemas, graph-protocol versions, selected build-plan materializations — versioned with toolchain family
- **Internal artifacts:** HIR/CGIR caches, solver traces, invalidation journals — no compatibility promise

### 4.6 Build-time execution is compile time over `BuildRuntime`

With `Project` in place, the clean reading is that build-time execution is ordinary optic evaluation over the build projection of the project root, not over a secret compiler universe.

```text
BuildHostContext = BuildFiles × BuildEnv × ToolchainInfo × TargetCaps
BuildRuntime     = CompilerGraph × TargetProfile × BuildHostContext × ArtifactCache
```

`PackageGraph` in the long-range design is derived by evaluating ordinary source-language declarations — native `package`, `workspace`, target, dependency, and build-plan values — inside `BuildRuntime`. That keeps build structure under the same staging, hashing, and diagnostic model as every other compile-time graph.

This is the crucial difference between compile-time execution in Optic and macro cultures in other languages. C++ templates, Rust proc macros, Java annotation processors, Python import-time side effects, and TypeScript build transforms all do useful work, but they typically operate through separate mechanisms with weaker integration into the optimizer-facing IR. Optic wants compile time to be another region-aware phase of the same language, not a second syntax-oriented meta-language.

Native package declarations use the same surface syntax and execution model as any other staged language value:

```rust
pub let package = Package {
    name: "browser.layout",
    edition: Edition::current(),
    targets: [Target::linux_x86_64_musl(), Target::wasm32_wasi()],
    deps: [
        Dependency::registry("css.selectors").compatible("2.1"),
        Dependency::registry("gfx.raster").exact("5.0.3"),
    ],
    runtime_family: RuntimeFamily::native(),
    capabilities: [BuildCap::read_path("assets/fonts")],
    experimental: [],
};

pub fn build_plan(rt: &[SharedGrade] BuildRuntime) -> BuildPlan {
    rt.query(
        ResolveDependencies
        >>> BuildModuleInterfaces
        >>> StageSelectorAutomata
        >>> PlanArtifacts
    ).get()
}
```

A package is a value. A build plan is a staged graph. The compiler materializes generated lock snapshots, module-interface artifacts, and staged-artifact manifests from those declarations instead of asking the user to restate the same information in another language.

#### 4.6.1 The whole ecosystem can be read as a larger costate graph

Once package and workspace declarations are native graph values, external packages stop looking like ambient downloads that live outside the language's semantics. The most useful long-range view is that the local `ProjectGraph` is a checked slice of a larger ecosystem-shaped graph rather than a sealed island.

```text
EcosystemGraph =
    RegistryGraph
  × PackageUniverse
  × InterfaceUniverse
  × ArtifactUniverse
  × CapabilityAndTrustGraph
  × ToolchainUniverse

ProjectGraph = materialize_slice(
    EcosystemGraph,
    workspace_root,
    generated_lock_snapshot,
    target_profile,
    runtime_family
)
```

The point is not that every compiler instance should mmap the entire ecosystem. The point is that registries, published interfaces, generated bindings, remote artifact caches, replay capsules, benchmark baselines, and plugin metadata can all be given the same semantic shape as local compiler regions: typed graph nodes with stable identities, provenance, compatibility facts, and explicit trust boundaries. A dependency then enters the local project not as an opaque archive that only the package manager understands, but as imported interface summaries, runtime-family declarations, boundary contracts, artifact identities, and provenance roots.

#### 4.6.2 Consequences of the larger-graph formulation

Thinking of the wider language ecosystem this way changes more than package management.

- package resolution becomes a graph-query and graph-transaction problem rather than a manifest side channel;
- external packages become semantically queryable through published summaries instead of only linkable by file or archive identity;
- registry publication, lock snapshots, generated bindings, replay artifacts, and benchmark baselines become ordinary graph projections or sidecar materializations;
- compatibility checking becomes a graph property over editions, interfaces, target profiles, runtime families, and boundary contracts;
- supply-chain trust stops being purely procedural and becomes representable as explicit graph edges and capability facts.

The price of this formulation is that the ecosystem needs strong publicity classes and trust boundaries. Not every registry fact or external artifact belongs in the hot authoritative graph of a local workspace. What the local `Project` stores directly, what it references by content hash, and what it only materializes on demand must remain disciplined. Later chapters return to that policy from three directions: build/package identity, graph-store layout, and the closing design rule that divides future growth into core, boundary, or never in core.


#### 4.6.2.1 Experimental mathematics should enter as graph-visible research lanes

The same larger-graph formulation gives the language a disciplined place to investigate mathematically richer ideas without promoting them directly into the core calculus. The design rule should stay deliberately small: reserve one contextual keyword, `experimental`, for native `package`, `workspace`, and `build_plan` declarations; reserve one namespace root, `std.experimental`, for experimental libraries and graph query roots; and reserve one graph-native `ExperimentalArena` for typed witnesses, indexes, and benchmarkable experimental artifacts.

```rust
pub let package = Package {
    name: "browser.layout",
    edition: Edition::current(),
    targets: [Target::linux_x86_64_musl()],
    experimental: [
        Experimental::proof_equivalence(),
        Experimental::resource_logic(),
        Experimental::memory_model(),
        Experimental::sheaf_consistency(),
    ],
};
```

```text
ExperimentalArena =
    EquivalenceWitnessArena
  × SeparationArena
  × MemoryModelArena
  × GeometryArena
  × UltrametricIndexArena
  × SheafArena
  × ToposArena
  × StabilityArena
```

This keeps experimental mathematics inside the same graph as the rest of the language while preserving the closure rule developed later in the maturity chapters: a bold idea may be implemented, queried, benchmarked, and stress-tested without silently becoming part of the permanent core language.

#### 4.6.2.2 Topos, sheaf, ultrametric, and stability ideas fit the graph better than the surface language

Some research directions are promising precisely because the current graph model already gives them a natural home. Sheaf-like local/global reasoning fits distributed services, plugin ecosystems, and partial project projections. Topos- and guarded-recursion-inspired work fits proof-facing models of coinduction, staging, and clocked reasoning. Ultrametric and p-adic ideas fit hierarchical indexing, AST/CGIR clustering, and agent-memory retrieval better than they fit the core numeric tower. Geometric algebra is usually a domain- and backend-facing refinement rather than a graph-structural one, while symbolic-dynamical and ergodic ideas fit runtime-family analysis, queue stability, and agent-swarm discipline.

The important design consequence is negative as much as positive: none of these ideas should first arrive as a free-floating new top-level sublanguage. They should first arrive as namespaced graph experiments with explicit witnesses, metrics, diagnostics, and promotion criteria. Only then can the language discover whether any part of them deserves promotion into the ordinary semantic core.

#### 4.6.2.3 The direct experimental answers should lead v1, and simpler sidecars should stay available

Not every unresolved systems question needs the richest available mathematics first. The project should prefer the **smallest theory that closes the operational gap** while v1 is still stabilizing. For the memory-model and unsafe-boundary questions, the most direct experimental answers are resource/separation reasoning and weak-memory semantics: they speak the same language as regions, ownership, summaries, and reorder legality. In practical terms that means `std.experimental.sep` and `std.experimental.memory` should lead the v1 research ladder, while more global theories such as sheaves or toposes remain available as second-wave refinements when they start proving something the direct lanes cannot already express.

At the same time, some answers are simpler *because they remain external companions instead of new language features*. TLA+ and Alloy are good examples for protocol, callback, and distributed-runtime design exploration; typestate and explicit protocol automata are good examples of a narrower first answer before richer session or sheaf machinery; abstract interpretation is a good example of a conservative optimizer- and checker-facing method that can validate approximations without requiring proof-heavy machinery in the everyday compiler. The design consequence is the same as everywhere else in the book: keep one semantic center, but allow well-scoped sidecars while the language decides which deeper theories are worth promoting. In practice that means the v1 research order should be explicit: direct internal lanes (`sep`, `memory`) first, external sidecars second, richer second-wave theories only after the first two prove insufficient.

The practical implication is that no one experimental lane should be expected to answer the entire stabilization backlog. The graph-native direct lanes answer the core memory and provenance questions, the sidecars answer many protocol and approximation questions cheaply, and the richer tracks stay available for the places where the smaller layers genuinely stop being enough.

#### 4.6.3 Extending the model downward

If the larger `EcosystemGraph` extends the model upward into packages, registries, published interfaces, generated bindings, and artifact caches, the symmetric move is to extend the model downward until the machine-facing facts that genuinely matter become graph-visible as well.

That downward extension should **not** introduce a second root below `Project`. It should introduce a family of derived, queryable projections whose job is to preserve stable machine contracts rather than every transient backend detail.

```text
Downward projections(ProjectGraph) =
    LayoutGraph
  × AddressSpaceGraph
  × ScheduleGraph
  × TargetCapabilityGraph
  × DebugPerfGraph
```

- `LayoutGraph` records field order, SoA/AoS/AoSoA choices, stride, page grouping, and the cache/TLB envelopes those choices imply.
- `AddressSpaceGraph` refines regions by `Ram`, `Atomic`, `Volatile`, `Mmio`, `Dma`, `GpuVisible`, `ForeignHeap`, or managed-handle address spaces.
- `ScheduleGraph` records partitioning, queue and ring topology, backpressure edges, and thread-affinity or callback constraints.
- `TargetCapabilityGraph` records ISA features, vector width, page size, cache-line assumptions, host I/O facilities, and runtime-family facts that affect lowering.
- `DebugPerfGraph` records the downward half of provenance: semantic `PerfKey`s, lowered code ranges, crash capsules, and translation-validation anchors.

The design rule is the same as the one used elsewhere in the book: extend downward only to the point where the facts remain stable, compositional, and useful to legality or tooling. Below that point, machine detail belongs to backend implementation, not to language truth.

#### 4.6.4 Why downward projections are worth keeping in the same graph

This downward extension buys three kinds of coherence.

First, legality becomes sharper. Once regions are refined by address space and access mode, the backend no longer has to treat volatile/MMIO, atomics, DMA-visible buffers, and ordinary RAM as if they were all one kind of memory. Reordering, fusion, vectorization, and barrier insertion can all be justified from the same summaries and contracts rather than from ad hoc backend folklore.

Second, performance reasoning becomes queryable. The same graph that already stores package and interface identities can now explain page pressure, TLB exposure, false-sharing risk, queue depth, branch bias, lane width, or target-specific staging decisions as ordinary graph facts.

Third, debugging and replay stay unified. Provenance now runs both upward toward source optics and downward toward lowered code ranges, target-profile choices, queue/ring mappings, and semantic performance identities. That is what lets the language remain explainable after optimization instead of merely fast.

The combined lesson is simple: upward extension gives the language closure over software-state. Downward extension gives it legibility at the machine boundary. Keeping both as projections over one authoritative `Project` graph is what lets the language stay general without splitting into one language above and another below.


#### 4.6.5 Project initialization should be graph construction, not boilerplate cloning

A language that treats `Project` as the authoritative semantic root should not let the user's first interaction with the toolchain be a file copier that happens to produce a directory tree. The equivalent of `cargo init` or `go mod init` should instead be understood as the **first guided graph transaction** over an empty or near-empty project root.

That transaction should be human-oriented in surface form and graph-native in semantics. A first-party `optic init --wizard` should ask a small, ordered set of questions whose answers already matter elsewhere in the language:

- what kind of project is being created (library, service, game subsystem, browser subsystem, kernel component, compiler/tool, mixed workspace, and so on),
- which `RuntimeFamily` and target profiles are intended,
- whether the initial focus is mostly build-time tooling, mostly runtime code, or both,
- what the first `AppWorld` hot data shapes look like,
- which host boundaries or foreign surfaces are expected,
- which build capabilities are needed,
- and what observability, replay, benchmark, or agent-assistance posture the project should begin with.

The answers should not vanish into one-time templating metadata. They should materialize as ordinary graph-native declarations and projections:

- a `project_intent` or equivalent typed build-root value,
- native `package` and `workspace` declarations,
- an initial `RuntimeBlueprint`,
- one or more `AppWorld` skeletons,
- boundary-contract stubs where interop or hardware access is expected,
- and initial benchmark, replay, and diagnostic-policy artifacts.

That makes initialization an extension of the same architecture rather than an exception to it. The wizard is not a second configuration language. It is a structured front end for constructing the first meaningful `Project` slice.

#### 4.6.6 Procedural templates should be generated from domain blueprints, not copied as inert text

The domain chapters later in the book already define recurring blueprint families: kernels, browsers, databases, games, compilers, services, and supporting toolchains. The init system should reuse those blueprints procedurally.

In practice that means a template is not just a static file bundle. It is a parameterized graph generator that combines:

- a domain blueprint,
- the chosen runtime family,
- target-profile assumptions,
- package/workspace topology,
- build capabilities,
- expected boundary contracts,
- and the chosen agentic/tooling posture.

A service template and a browser-subsystem template might both emit a `package` value, a `RuntimeBlueprint`, and initial diagnostics/benchmark scaffolding, but they should differ in the host regions, stage roots, benchmark keys, and replay assumptions they materialize. This is what makes the templates procedural rather than ornamental.

The practical payoff is large.

- The generated skeleton already speaks the language's own semantic vocabulary.
- Agents inherit typed project intent instead of reverse-engineering it from boilerplate.
- The resulting project is immediately queryable through the same `Project` roots used elsewhere in the language.
- Re-running the wizard later can be treated as another graph transaction with previewable diffs instead of a destructive rewrite of hand-maintained files.

#### 4.6.7 Human mode and agent mode should share one underlying transaction model

The human-facing init wizard should remain the default because the first project description is usually underspecified and iterative. But the same mechanism should also support a headless, agent-friendly mode.

A good first-party shape is:

- `optic init --wizard` for interactive question/answer setup,
- `optic init --template service --target linux_x86_64_musl` for fast-path guided setup,
- `optic init --intent intent.json` (or equivalent structured submission) for automated or repeated generation,
- and `optic init --preview` for showing the graph and file projections that would be created before committing them.

The key rule is that these should all lower to the same project-transaction machinery. The human interface differs; the semantic operation does not. That is what keeps initialization compatible with later direct protocol use, coding-agent workflows, and graph-native reproducibility.



#### 4.6.8 Some agent memory belongs in the graph, and some should not

Once the project is treated as a durable semantic graph, the same question arises for collaboration memory: should architectural decisions, accepted repairs, prior failures, and agent-maintained repository knowledge live in that graph, or stay outside it as transient tooling state?

The clean answer is selective.

- **Graph-native memory** should hold durable, shared, provenance-linked knowledge that materially affects future reasoning: accepted architecture decisions, validated repair records, benchmark and replay explanations, boundary exceptions, task state that spans revisions, and other facts that later humans or agents should be able to query directly.
- **Sidecar or tool-local memory** should hold scratch notes, speculative plans, long transcripts, ranking heuristics, and other temporary context that is useful during a task but should not become semantic truth.
- **External or user-private memory** should remain outside the repository entirely when it does not belong to the codebase as a shared artifact.

A good conceptual extension is therefore:

```text
ProjectGraph = ... × AgentMemoryArena

AgentMemoryArena =
    DecisionRecord
  × TaskRecord
  × DiagnosticRepairRecord
  × BenchmarkKnowledge
  × FailedPatchRecord
```

This does **not** turn the graph into a notebook or a heap image. It applies the same discipline already used elsewhere in the chapter: durable, typed, queryable facts belong in the authoritative graph; bulky or low-trust material remains in sidecars or private tool state. The gain is that future agents and human reviewers can query the same semantic center for both code facts and the small body of maintenance knowledge that is worth preserving.

#### 4.6.9 Failed patches should be advisory graph records, not permanent bans

One especially valuable kind of maintenance knowledge is negative knowledge: a patch or rewrite that was attempted, tied to a clear goal, and then rejected because it broke a proof, regressed a benchmark, violated policy, or failed review. The important design choice is to store that information as **advisory, evidence-backed graph memory**, not as an untyped prohibition.

A useful conceptual record shape is:

```text
FailedPatchRecord {
  goal,
  target_nodes,
  target_regions,
  patch_fingerprint,
  attempted_at_revision,
  target_profile,
  runtime_family,
  outcome,
  reason_class,
  evidence_refs,
  superseded_by,
  revalidate_after,
  status,
}
```

The record is useful precisely because it is narrow. It says: this style of change was tried against this revision and target, for this reason, and failed with this evidence. Future agents can then de-rank similar repairs, and human reviewers can see whether a proposal is genuinely new or just a repetition of a known mistake. The crucial guardrail is that failed-patch records are **warnings with provenance**, not eternal laws. They need expiry, supersession, and explicit evidence so that the graph stores caution rather than cargo-cult taboo.

### 4.7 Many IRs, one authoritative graph

A reader may reasonably worry that the book is accumulating too many intermediate representations: text, syntax, HIR, typed HIR, summaries, CGIR, fused CGIR, interfaces, artifacts, debug views, benchmark views, and later query plans. That concern is healthy. A language can absolutely die of IR sprawl.

The answer in this architecture is not to collapse everything into one giant universal IR. That would force incompatible invariants into one structure and make every phase harder to explain. The answer is also not to let each phase become a disconnected silo with its own ids, caches, and tool story. That would destroy provenance, incrementalism, and queryability.

The right middle position is:

> **many IRs, one graph.**

Every important compiler form is a projection over one authoritative `ProjectGraph`, not a competing semantic world.

| Projection | Primary invariant | Main consumers |
|---|---|---|
| source text projection | exact authored bytes, comments, and spans | editor, formatter, migration tools |
| syntax projection | parsed structure and recovery points | parser diagnostics, early tooling |
| HIR projection | resolved names, cursor insertion, desugared queries | type checker, summary builder |
| typed-HIR / summary projection | region facts, grades, determinism, legality preconditions | alias checker, stageability, diagnostics |
| CGIR projection | optic composition and product structure remain explicit | optimizer, provenance, backend planning |
| backend projections | target-facing lowered structure | Rust path, LLVM path, object/debug emission |
| interface / artifact projections | externally consumed semantic and build products | package manager, incremental build, remote cache, plugins |

This organization is what keeps the architecture practical.

- A **single mega-IR** would blur the distinction between source fidelity, legality facts, optimizer structure, and target lowering.
- **Many disconnected IR silos** would force the compiler and tools to translate among half-truths that all describe the same program differently.

The projection model keeps one identity space and one provenance chain while still allowing each phase to add the information its own invariant needs.

#### 4.7.1 Authoritative, derived, and ephemeral projections

Not every projection deserves the same status.

| Class | Examples | Compatibility expectation |
|---|---|---|
| authoritative graph-native projections | HIR, typed summaries, CGIR, interface summaries, diagnostics, dependency graph | versioned as part of the compiler/toolchain contract |
| derived/materialized projections | text views, generated Rust, generated LLVM, lock snapshots, emitted build plans, exported benchmark capsules | reproducible from authoritative graph state |
| ephemeral/internal projections | worklists, temporary rewrite plans, solver traces, backend scratch state | no external stability promise |

This distinction matters throughout the rest of the book. Staging, replay, self-hosting, distributed build, and IDE/protocol support all become simpler once the language says clearly which forms are authoritative and which are only views.

### 4.8 Native Project queries as ordinary optics

Once `Project` becomes the semantic whole, the next question is whether users and tools should need a second query language to inspect it. The book's answer is no.

> **Queries over `Project` should be ordinary optic programs over explicit graph roots.**

That means the user-facing surface stays inside the language rather than escaping into a separate SQL-, Lisp-, or macro-like metatool. The language simply exposes stable graph roots such as:

- `ProjectSource`
- `ProjectSyntax`
- `ProjectHir`
- `ProjectSummaries`
- `ProjectCgir`
- `ProjectInterfaces`
- `ProjectDiagnostics`
- `ProjectArtifacts`
- `ProjectDependencies`

A query then looks like an ordinary staged optic pipeline over one of those roots.

```rust
pub fn hot_writers(p: &[SharedGrade] Project) -> List<NodeId> {
    p.query(
        ProjectCgir
        >>> Nodes
        >>> Filter(|n| overlaps(n.summary.put_writes, app.healths[*]))
        >>> Filter(|n| n.summary.get_grade.cache > 3)
        >>> Map(|n| n.id)
    ).get()
}
```

This is intentionally boring. It reuses the same vocabulary as the rest of the language:

- explicit root costates,
- optics and traversals,
- typed closures,
- stageability over `BuildRuntime`,
- ordinary returned values.

Internally, the compiler is free to lower these graph queries into a richer query IR with indexes, joins, path search, and repair synthesis. But that richer query engine remains an implementation detail. The user model is still “ordinary optic code over `Project`”.

Chapter 27 later turns that architectural preference into an explicit closure rule, and Chapter 28 turns it into a standing review checklist: a separate user-facing project-query language lives permanently in the “never in core” column, while native project queries remain core precisely because they reuse ordinary optics.

#### 4.8.1 Read-only by default, transactional when mutating

This query subset should be **mostly read-only**.

Ordinary graph queries inspect summaries, provenance, dependencies, diagnostics, and artifacts. They do not silently mutate the compiler graph. That keeps compile-time execution deterministic and keeps self-hosted tooling auditable.

When mutation is needed, the language should not pretend it is just another read query. Mutation happens through **compiler-phase transactions** over `Project.build`, which the next section formalizes.

#### 4.8.2 Why this matters outside tooling

Making project queries native is not a tooling luxury. It interacts with the whole language.

- **staging:** compile-time code can ask semantic questions about the graph without escaping into a second DSL;
- **self-hosting:** compiler passes and refactoring tools can inspect the compiler's own summaries using ordinary language constructs;
- **diagnostics:** ranked fixes and “why did this fail?” explanations become queries instead of ad hoc tool code;
- **packages and build plans:** package roots, interface dependencies, target-profile selection, and artifact planning remain inside the same typed model;
- **debugging and profiling:** fused ancestry, stage provenance, and semantic `PerfKey`s become queryable facts rather than reverse-engineered metadata.

### 4.9 Compiler phases as graph transactions

If project inspection is ordinary optic code, can compiler phases themselves also be expressed in the same model?

Yes — with one refinement.

> **A compilation phase is not merely a read query; it is a staged graph transaction over `Project.build`.**

That transaction has a regular shape:

```text
select affected graph regions
  -> analyze and compute derived facts
  -> synthesize replacement or materialized projection
  -> validate phase invariants
  -> commit a new project revision
```

This gives a clean interpretation of the compiler pipeline.

| Phase | Reads | Writes |
|---|---|---|
| parser | text projection | syntax projection |
| HIR lowering | syntax projection | HIR projection |
| summary builder | typed HIR | summary table |
| CGIR construction | HIR + summaries | CGIR projection |
| fusion | CGIR | fused CGIR + provenance updates |
| interface/artifact emission | summaries + CGIR + target profile | module interfaces, lock snapshots, generated artifacts |

The language-wide payoff is significant.

- **incremental compilation** becomes graph invalidation plus selective transaction replay rather than cache-directory archaeology;
- **distributed build** becomes scheduling over subgraph closures rather than file batches;
- **self-hosting** becomes more straightforward because parser, checker, optimizer, and materializer are all written as ordinary graph transformations over compiler-owned costates;
- **translation validation** becomes clearer because the Rust path, LLVM path, and any future backend all start from the same authoritative graph revision rather than from subtly different front-end pipelines.

#### 4.9.1 Why this is stronger than “mutating queries”

The phrase “mutating query” is suggestive, but it is too weak to be the canonical description.

A compiler phase must do more than mutate.

- it must preserve or deliberately evolve stable ids,
- it must carry provenance forward,
- it must validate summary and graph invariants,
- it must produce deterministic artifacts,
- and it must commit atomically as a new project revision.

That is why the right phrase for the book is **graph transaction** rather than merely “query with writes”.

### 4.10 Files-first bootstrap mode is still useful

None of this requires the first compiler release to start in graph-first daemon mode. A file-first bootstrap remains the simplest narrow-v0 operating mode: read ordinary files from disk, build the graph in memory, write conventional outputs, and keep the graph-store implementation behind an internal feature gate.

#### 4.10.1 File-first remains a permanent supported operating mode

The point of graph authority is not to make file-first workflows disposable. Even after a graph-resident daemon becomes the fastest or richest mode, the language should continue to support ordinary materialized source trees, deterministic text export, and CLI-first repository workflows. Humans should never be forced into a bespoke semantic editor just because the compiler's internal model is richer than text.

#### 4.10.2 Direct protocols and LSP adapters should ship together

The direct graph protocol remains the stronger semantic interface for tools, transactions, and agent workflows. But that does not excuse the language from shipping first-party adapters for existing editors and file-centric tooling. Projection filesystems, LSP adapters, and explicit materialization commands should therefore be treated as part of the permanent tool contract rather than as migration scaffolding.

What matters from day 0 is the architecture, not the rollout order. If the compiler already treats its internal state as one project-shaped graph costate, moving from process-local memory to a persistent memory-mapped graph later becomes a storage change, not a semantic rewrite.

### 4.11 Transition

Chapter 3 established why the runtime must be explicit. This chapter extended that idea to the full project root: one semantic structure, two regimes (build and run), many projections. The rest of Part I continues with the executable core — optics as the transformation model and grades as the resource contract. When the book returns to the full-language growth path in Part III, the `Project` architecture becomes the context for native package declarations (Chapter 14), compiler tooling (Chapter 22), and ecosystem maturity and feature-governance policy (Chapters 27–28).

---

## 5. Optics as the Transformation Model

> **By the end of this chapter, a reader should understand:** why the language uses optics as its primary abstraction for data transformation rather than methods, callbacks, iterators, or effect handlers; what the `get`/`put` pair represents operationally and why both halves matter for alias analysis and codegen; how the two composition operators (`>>>` and `***`) preserve structure in ways that a conventional function-call IR would not; and why `Cursor<S>` is the key bridge between the semantic model and the generated machine code.

### 5.1 Why optics are the right abstraction here

An optic is useful in this language only if it is more than a mathematical gesture. It has to survive the trip into compiler machinery and machine code.

#### 5.1.1 The compiler may speak category theory; the user does not have to

The internal justification for the design is unapologetically mathematical: costates, coalgebraic optics, and semiring-structured grades are the most compact way to state the laws the compiler relies on. But the default user-facing vocabulary should stay operational. Most programmers should be able to think in terms of focuses, regions, updates, hot paths, ownership shares, budgets, and replay boundaries without needing to adopt category theory as everyday surface language. The theory remains the compiler's organizing vocabulary; diagnostics and tutorials should default to the machine-facing one.

That is why the book treats optics operationally. A lens-like optic in the prelude is the promise that there exists a stable and inspectable pair of operations:

```text
Lens<S, A> ≈ { get: Cursor<S> -> A, put: (Cursor<S>, A) -> () }
```

That pair is not merely convenient for implementation. It is the executable form of the coalgebraic view. `get` tells the compiler where the focused value comes from. `put` tells the compiler exactly how updates re-enter the surrounding structure. Both pieces matter for alias safety, fusion, diagnostics, and later native-code lowering.

### 5.2 The prelude optic: lens-like only

The first compiler only supports lens-like optics. That choice is not a loss of theoretical ambition. It is a narrowing designed to maximize the ratio of semantic content to implementation risk.

A lens already gives the compiler everything it needs to prove the architecture.

- A focused read path
- A structured write-back path
- Composition laws
- Interaction with ownership and locality
- Enough surface syntax to make realistic examples

Prisms, traversals, folds, setters, and isos all build on the same basic bridge once the lens path is trusted.

### 5.3 Composition as structure, not syntax sugar

The language uses two primitive optic compositions in the core surface.

```text
A >>> B   -- sequential composition
A *** B   -- product over the same costate
```

Sequential composition is where intermediate structure flows. Product composition is where shared-costate parallel legality matters. These are not merely ergonomic operators. They are what let the compiler build a graph-shaped IR that still remembers what the programmer intended.

A conventional compiler that sees only functions and calls must reconstruct adjacency and sharing later. This language keeps them explicit from the start.

### 5.4 The optic varieties beyond v0

The full language adds other optic varieties because they correspond to recurring machine shapes.

| Optic variety | Semantic meaning | Dominant low-level shape |
|---|---|---|
| Lens | exactly one focus | direct load/modify/store |
| Prism | zero or one focus | conditional branch |
| Traversal | many homogeneous foci | bulk loop or SIMD pass |
| Fold | many read-only foci | reduction or scan |
| Setter | many write-only foci | masked bulk update |
| Iso | lossless conversion | representation rewrite or zero-cost coercion |

This table foreshadows the later chapters. The point is not that each optic kind is tied to one instruction. The point is that each kind has a dominant machine interpretation, and the language design follows that fact rather than hiding it.

### 5.5 Why optics instead of ambient handler evidence as the default

A language with evidence-passing handlers or effect rows can express broad classes of effectful computation. That is valuable. But for the class of systems work this language targets, evidence values are often the wrong default implementation form.

The compiler does not want to dynamically look up the evidence for reading `positions[*]`, writing `healths[*]`, or polling a clock field. It wants those as direct structured accesses over `Runtime`. Optics provide the right bridge because they expose both context requirement and update path in a way the compiler can summarize and erase.

This does not mean general handlers are impossible later. It means the default representation of structured stateful work should be optic-shaped, not handler-shaped.

#### 5.5.1 Cross-language comparison: optics versus the usual abstraction shapes

At first encounter, many readers map optics onto a more familiar abstraction they already know. That instinct is useful, but each analogy is incomplete.

| Language or family | Nearest familiar abstraction | Where the analogy helps | Where Optic deliberately goes further |
|---|---|---|---|
| **C++** | methods on structs, iterator/range pipelines, visitors | close to data layout and easy to imagine as inlineable code | optics keep `get` and `put` explicit and summary-friendly, so fusion and alias reasoning operate on structure rather than on conventions spread across methods and templates |
| **Java** | object methods, streams, visitors, framework pipelines | familiar composition vocabulary | typical Java abstractions do not preserve field-level layout and writeback structure as optimizer-facing artifacts |
| **Python** | object updates, comprehensions, decorators, Pandas/NumPy-style transforms | good intuition for "describe the transformation, not the loop" | the transformation remains statically typed and region-aware all the way into code generation instead of collapsing into dynamic dispatch |
| **TypeScript** | array combinators, unions, object-path updates, Rx-style streams | explicit dataflow APIs feel similar on the surface | Optic insists that the same composition survive into backend legality checks, not disappear after type erasure |
| **Rust** | iterator chains, view types, borrowed projections, lens crates | probably the closest mainstream comparison | optics are not just library patterns here; they are the language's primary transformation objects and the things CGIR is built out of |
| **Haskell / optics libraries** | lenses, prisms, traversals, folds | the optic vocabulary is directly shared | the goal is different: in Optic, the abstraction is judged partly by whether it produces strong machine-level consequences such as cache-lawful loops, TBAA, and SIMD eligibility |

The key point is simply that optics are the smallest abstraction that keeps observation, reinsertion, composition, and summary formation in one place.

### 5.6 The cursor as the operational heart of the prelude

Every successful prelude optic body is normalized through one executable form:

```text
Cursor<S> = { arena: &mut S, id: usize }
```

`Cursor<S>` is what makes the semantic story line up with the machine. The arena field is the base pointer. The index is the induction variable. Field access becomes address computation. Region extraction becomes structural and predictable.

That is why the cursor is not a trivial helper. It is the proof-carrying bridge between optic semantics and code generation.

### 5.7 Transition

If optics explain the program's shape, grades explain its contracts. The next chapter turns locality, exclusivity, latency, and replay constraints into static objects the compiler can manipulate.

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

# Part II — The Narrow v0 Compiler

Part II turns the semantic argument into a compiler that can be falsified. Each chapter introduces one new compiler artifact—grammar, HIR, summaries, checker facts, CGIR, fusion, and code generation—and insists that the artifact remain inspectable, deterministic, and evidence-bearing. The aim is not feature breadth; it is to make the core abstraction executable and auditable.

The important reading rule for this part is that none of these artifacts is merely local to the narrow compiler. Each one is also a **zero-cost structural hook** for later features. The hand-written parser is the future edition and migration surface. `OpticSummary` is the carrier that later holds asymmetric I/O grades, richer determinism classes, replay flags, and foreign-boundary facts. The abstract `GradeConstraintSolver` seam is the future insertion point for symbolic/Z3-backed reasoning. CGIR is the place where traversals, prisms, staging, coinduction, and proof-carrying rewrites can later enter without a semantic rupture. The Rust backend is the reference path against which LLVM, replay, debug provenance, and self-hosting will later be validated.

Read in that light, Part II is not a detour away from the grander systems ambitions. It is the place where the architecture proves that it already contains the attachment points those ambitions will need.
## 7. Surface Language, Grammar, and Parser Strategy

### 7.1 Why grammar matters more than it first appears

A language this structured cannot afford a vague parser. If two implementations disagree on token boundaries, precedence, or recovery points, every later artifact becomes unstable: AST fixtures, HIR shapes, diagnostics, CGIR snapshots, and codegen.

That is why the grammar chapter is not a formality. It is the first point where the book's insistence on determinism becomes operational.

### 7.2 The surface forms the prelude supports

The prelude surface is intentionally small.

- `data` declarations
- `optic` declarations with `get` and optional `put`
- `let` bindings for optic expressions
- `fn` declarations for small wrappers and tests
- optic composition with `>>>` and `***`
- query chains using `.query(...).get()`, `.set(...)`, and `.map(...)`

This subset is large enough to write meaningful examples and small enough to parse without ambiguous hidden desugarings.

### 7.3 Lexer rules that must be fixed early

The lexer must commit to a few facts up front.

| Token decision | Why it matters |
|---|---|
| `>>>` is one token | composition precedence depends on it |
| `***` is one token | product parsing depends on it |
| block comments are nestable | generated and hand-written examples both need safe commenting |
| keywords outrank identifiers | deterministic parsing and diagnostics |
| a lone `*` is invalid in surface syntax | avoids confusing arithmetic/operator overlap in v0 |

The longest-match rule is non-negotiable. It is the difference between a stable composition grammar and a parser that is forced to patch over tokenization mistakes later.

### 7.4 Precedence and why it mirrors the machine story

The prelude uses one strong precedence choice:

```text
>>> binds tighter than ***
```

That is not arbitrary. Sequential composition is the closer analogue to direct dataflow or function application. Product composition is the looser analogue to parallel juxtaposition over a shared costate. The tighter binding of `>>>` makes mixed expressions read in the same direction that the compiler will later build the graph.

The parser should still emit a targeted diagnostic if a mixed unparenthesized expression is likely to be confusing. The point is not merely to parse; it is to keep the source language auditable.

### 7.5 Parser architecture

The narrow grammar is best served by a hand-written recursive-descent parser with a small Pratt parser for optic expressions.

That choice is justified by three constraints.

- The grammar is not large enough to justify a heavy generated parser.
- Error recovery and spans matter more than grammar compression.
- Optic composition precedence is simpler to express as binding power than as left-factored productions.

#### 7.5.1 Binding-power sketch

```text
parse_optic_expr(min_bp = 0):
  lhs = parse_optic_atom()
  while next token is >>> or ***:
    read op
    if lbp(op) < min_bp: stop
    rhs = parse_optic_expr(rbp(op))
    lhs = make_node(op, lhs, rhs)
  return lhs
```

The full normative EBNF is placed in Appendix D so the main body can explain why the grammar is shaped this way without drowning the reader in productions.

### 7.6 Recovery is part of correctness

Agent-facing compilation changes the bar for parsing. A parser that stops at the first error is not merely inconvenient; it makes automated repair loops far less efficient.

The prelude parser therefore recovers to synchronization points.

| Context | Recovery tokens |
|---|---|
| top level | `data`, `optic`, `let`, `fn`, EOF |
| optic body | `get`, `put`, `}`, EOF |
| type position | `,`, `>`, `)`, `]`, `=`, `{`, EOF |
| query chain | `.get`, `.set`, `.map`, `;`, `}`, EOF |

The recovery design fits the rest of the compiler doctrine: produce as much deterministic evidence as possible from one run.

### 7.7 A concrete example and why it parses cleanly

```rust
data Entities {
    healths: SoA<f32>,
    positions: SoA<Vec2>,
}

optic HealthView: GradedOptic<Entities, f32, CacheGrade<1> + AffineGrade> {
    get  s      => s.healths[s.id]
    put  (s, v) => { s.healths[s.id] = v }
}

let player_view = HealthView *** PositionView;
```

This example is deliberately small. It exercises the core top-level forms, both field and index syntax, optic composition, and grade annotation without yet involving the later compiler phases. A parser that cannot make this example boring will make every later chapter fragile.

### 7.8 Transition

The parser's job is to preserve structure and spans. The next phase must resolve names, normalize field access through cursors, and compute the summaries that every later legality rule depends on.

### 7.9 Detailed implementation reference: concrete grammar, lexer behavior, and parser recovery

The main chapter introduced the narrative and the design constraints. The following material is the normative algorithmic supplement: exact token behavior, operator precedence, recovery discipline, and grammar fragments that keep independent implementations compatible.

A concrete, implementable grammar is essential for the prelude. Without it, parser behavior varies across implementations and makes golden AST tests fragile.

#### 7.9.1 Character classes

```
letter     ::= [a-zA-Z]
digit      ::= [0-9]
ident_char ::= letter | digit | '_'
```

#### 7.9.2 Token types

##### 7.9.2.1 Longest-match lexer algorithm

The lexer must implement longest-match tokenization for multi-character operators before falling back to single-character punctuation. This is not optional because the grammar relies on `>>>` and `***` being indivisible tokens.

```text
scan_token(i):
  if src[i..].starts_with('>>>'): emit(SEQ, span(i,i+3)); return i+3
  if src[i..].starts_with('***'): emit(PAR, span(i,i+3)); return i+3
  if src[i..].starts_with('=>'):  emit(FAT_ARROW, span(i,i+2)); return i+2
  if src[i..].starts_with('<='):  emit(LE, span(i,i+2)); return i+2
  if src[i..].starts_with('>='):  emit(GE, span(i,i+2)); return i+2
  if src[i..].starts_with('--'):  skip_line_comment()
  if src[i..].starts_with('{-'):  skip_block_comment_nested()
  if is_ident_start(src[i]):      scan_identifier_or_keyword()
  if is_digit(src[i]):            scan_number_literal()
  else                            scan_single_char_punct_or_error()
```

Nested block comments must be handled with a depth counter:

```text
skip_block_comment_nested():
  depth = 1
  while depth > 0:
    if next == '{-': depth += 1
    elif next == '-}': depth -= 1
    elif EOF: emit(PAR-030); break
    advance()
```

The implementation should resist the temptation to use ad hoc regex splitting. Nested comments, longest-match operators, and faithful spans are simpler to reason about in a single deterministic scanner.

| Token | Pattern | Notes |
|-------|---------|-------|
| `IDENT` | `letter ident_char*` | Keywords take priority |
| `INT_LIT` | `digit+` | Unsigned, fits u64 |
| `KW_DATA` | `data` | Keyword |
| `KW_OPTIC` | `optic` | Keyword |
| `KW_GET` | `get` | Keyword |
| `KW_PUT` | `put` | Keyword |
| `KW_LET` | `let` | Keyword |
| `KW_FN` | `fn` | Keyword |
| `KW_QUERY` | `query` (method form) | |
| `KW_MAP` | `map` | |
| `KW_GET_M` | `get` (method form) | |
| `KW_SET` | `set` | |
| `SEQ` | `>>>` | Sequence composition operator |
| `PAR` | `***` | Parallel product operator |
| `FAT_ARROW` | `=>` | Body separator |
| `COLON` | `:` | |
| `COMMA` | `,` | |
| `SEMI` | `;` | |
| `LBRACE` | `{` | |
| `RBRACE` | `}` | |
| `LPAREN` | `(` | |
| `RPAREN` | `)` | |
| `LBRACKET` | `[` | |
| `RBRACKET` | `]` | |
| `DOT` | `.` | |
| `PLUS` | `+` | Grade union in type position |
| `LT` | `<` | Grade parameter open |
| `GT` | `>` | Grade parameter close |
| `STAR` | `*` | (only `***` in surface; single `*` is not a valid operator) |
| `EQUALS` | `=` | |
| `COMMENT` | `--` to end-of-line | Discarded |
| `BLOCK_COMMENT` | `{- … -}` | Nestable; discarded |

#### 7.9.3 Operator precedence (binding tightest to loosest)

| Level | Operator | Associativity | Description |
|-------|----------|--------------|-------------|
| 5 | `>>>` | left | Sequential composition |
| 4 | `***` | left | Parallel product |
| 3 | type `+` | left | Grade union in type annotations |

Without parentheses, `A *** B >>> C` parses as `A *** (B >>> C)` because `>>>` binds tighter. The parser must enforce this and emit `PAR-010` when ambiguous forms appear without the required parentheses.

**Rationale for this precedence:** `>>>` is analogous to function application / sequential pipe (tight), while `***` is analogous to parallel juxtaposition (loose). Arithmetic analogy: `*` is `>>>`, `+` is `***`.

#### 7.9.4 v0 surface grammar (EBNF, canonical form)

Full EBNF is reproduced in Appendix E. The key productions:

```ebnf
program         ::= item* EOF
item            ::= data_decl | optic_decl | let_binding | fn_decl

data_decl       ::= 'data' IDENT '{' (field_decl (',' field_decl)* ','?)? '}'
field_decl      ::= IDENT ':' type_expr

optic_decl      ::= 'optic' IDENT ':' optic_type '{' optic_body '}'
optic_type      ::= 'GradedOptic' '<' type_expr ',' type_expr ',' grade_expr '>'
grade_expr      ::= grade_dim ('+' grade_dim)*
grade_dim       ::= 'CacheGrade' '<' INT_LIT '>'
                  | 'OwnershipGrade' '<' rational_lit '>'
                  | 'LinearGrade' | 'AffineGrade' | 'SharedGrade'
                  | '_'                                    (* infer *)
optic_body      ::= get_clause put_clause?
get_clause      ::= 'get' IDENT '=>' expr
put_clause      ::= 'put' '(' IDENT ',' IDENT ')' '=>' (expr | block_expr)
block_expr      ::= '{' stmt* '}'
stmt            ::= expr ';'

optic_expr      ::= optic_atom (optic_op optic_atom)*
optic_op        ::= '>>>' | '***'
optic_atom      ::= IDENT | '(' optic_expr ')'

let_binding     ::= 'let' IDENT '=' optic_expr ';'

query_chain     ::= expr '.query(' optic_expr ')' ('.' query_method)*
query_method    ::= 'get' '(' ')'
                  | 'set' '(' expr ')'
                  | 'map' '(' closure ')'

expr            ::= query_chain
                  | field_access
                  | index_expr
                  | tuple_expr
                  | IDENT
                  | INT_LIT
                  | '(' expr ')'
field_access    ::= expr '.' IDENT
index_expr      ::= expr '[' expr ']'
tuple_expr      ::= '(' expr (',' expr)+ ')'

type_expr       ::= IDENT ('<' type_expr (',' type_expr)* '>')?
                  | '(' type_expr (',' type_expr)* ')'
                  | 'SoA' '<' type_expr '>'
                  | 'BitSet'
```

#### 7.9.5 Parser strategy

##### 7.9.5.1 Pratt parser skeleton for optic expressions

The optic algebra is the one place where binding power genuinely matters. A Pratt parser makes the rule visible in code.

```text
parse_optic_expr(min_bp = 0):
  lhs = parse_optic_atom()
  while let op = peek_infix_optic_op():
    (lbp, rbp) = binding_power(op)
    if lbp < min_bp: break
    advance()
    rhs = parse_optic_expr(rbp)
    lhs = make_infix_optic_node(op, lhs, rhs)
  return lhs

binding_power('>>>') = (50, 51)
binding_power('***') = (40, 41)
```

This directly encodes the rule that `>>>` binds tighter than `***`.

##### 7.9.5.2 Error recovery sets by context

Recovery works best when synchronization tokens are specific to the parser context rather than globally fixed.

| Context | Sync tokens |
|---------|-------------|
| top-level item | `data`, `optic`, `let`, `fn`, EOF |
| inside optic body | `get`, `put`, `}`, EOF |
| inside type annotation | `,`, `>`, `)`, `]`, `=`, `{`, EOF |
| inside query chain | `.get`, `.set`, `.map`, `;`, `}`, EOF |

The parser should attach a recovery note to the diagnostic whenever tokens are skipped. That gives both humans and agents a concrete explanation for why later syntax may have been interpreted more weakly after the first error.

The v0 parser should be a hand-written recursive-descent parser with Pratt-style operator parsing for optic expressions. Rationale:

- Hand-written parsers produce better error messages than generated ones
- Pratt parsing handles the `>>>` / `***` precedence table cleanly
- The grammar is LL(2) everywhere except the `get`/`put` clause distinction (one token lookahead to the `=>` suffices)

**Error recovery strategy:** At any parse error, the parser should:
1. Emit a `PAR-0xx` diagnostic with the offending span
2. Consume tokens until it finds a synchronization token: `optic`, `data`, `let`, `fn`, `}`, or EOF
3. Resume parsing from the next top-level item

This guarantees that a file with N syntax errors produces N diagnostics rather than stopping at the first.

#### 7.9.6 Span representation

Every AST node must carry a `Span { file: FileId, start: u32, end: u32 }`. Byte offsets into the source file are preferred over line/column (line/column is computed on demand for display). The file must never be implicit; multi-file projects must always carry `FileId`.

---

## 8. HIR, Cursors, Names, and Summaries

### 8.1 Why HIR exists as a distinct phase

The HIR is where the language stops being surface syntax and starts being an analyzable program. This phase has one job: preserve semantic intent while removing superficial ambiguity.

If the compiler skipped directly from AST to type checking or codegen, every later phase would be forced to repeatedly rediscover the same facts: which identifier refers to a named optic, which field belongs to which costate, what the current cursor is, and whether a query chain represents a get, set, or map action.

HIR exists to make those facts explicit once.

That explicitness is also what keeps the later language from requiring a front-end rewrite. `Cursor<S>`, `PathLift`, and `OpticSummary` are not narrow-compiler conveniences. They are the hooks by which later chapters attach richer optic kinds, asymmetric grades, replay metadata, stageability facts, module-interface summaries, and foreign-boundary contracts without changing the basic shape of the front end.

### 8.2 Name resolution order

The narrow compiler resolves identifiers in a fixed order.

1. local variables in scope,
2. named optics,
3. named `data` declarations,
4. built-ins and primitive types.

This is simple, but the simplicity is load-bearing. Deterministic resolution is a prerequisite for deterministic diagnostics, stable summaries, and reproducible code generation.

### 8.3 The cursor model and why it matters

#### 8.3.1 Concrete cursor lowering table

| Surface form | HIR form | Why the rewrite matters |
|---|---|---|
| `s.field[s.id]` | `cursor.arena.field[cursor.id]` | normalizes the base pointer and induction variable |
| `s.id` | `cursor.id` | makes loop-carried identity explicit |
| `s` in read position | `cursor.arena` | allows whole-costate reads without losing the cursor anchor |
| `s.field[s.id] = v` | `cursor.arena.field[cursor.id] = v` | turns update paths into explicit store sites |

All prelude optic bodies are normalized through a cursor.

```text
Cursor<S> = { arena: &mut S, id: usize }
```

A surface access like:

```rust
s.healths[s.id]
```

becomes the HIR shape:

```text
cursor.arena.healths[cursor.id]
```

That normalization does several things at once.

- It makes the induction variable explicit.
- It gives later passes a single access shape to reason about.
- It turns region extraction into a structural walk rather than a syntax-sensitive guess.
- It helps the code generator produce direct index loops without inventing new variables ad hoc.

### 8.4 Query chains as explicit HIR nodes

#### 8.4.1 Core HIR node shapes

```text
HirOptic =
  | Named(name, span)
  | Compose(lhs, rhs, span)
  | Product(lhs, rhs, span)
  | Paren(inner, span)

HirQuery =
  | QueryGet  { costate, optic, cursor, span }
  | QuerySet  { costate, optic, cursor, value, span }
  | QueryMap  { costate, optic, cursor, fn, span }
```

#### 8.4.2 Query lowering sketch

```text
lower_query_chain(base, optic, methods):
  costate = lower_expr(base)
  optic'   = lower_optic(optic)
  cursor   = fresh('cur')
  current  = QuerySeed(costate, optic', cursor)
  for method in methods:
    current = lower_method(current, method)
  return current
```

The book keeps this algorithm in prose because the important point is not the loop itself; it is that the surface chain is lowered once into a shape that every later pass can trust.

The surface query syntax is intentionally pleasant. The HIR form is intentionally blunt.

```rust
entities.query(HealthView).map(|h| h - 10.0)
```

becomes:

```text
QueryMap {
  costate: Var("entities"),
  optic:   OpticRef("HealthView"),
  cursor:  FreshCursor("cur_0"),
  fn:      Closure { param: "h", body: Sub(Var("h"), FloatLit(10.0)) }
}
```

This is a perfect example of the book's general method: surface ergonomics for the human, explicit structure for the compiler.

### 8.5 Why summaries are the compiler's real semantic currency

#### 8.5.1 A concrete summary record

```text
OpticSummary {
  name,
  costate,
  focus,
  lift,
  get_reads,
  put_reads,
  put_writes,
  get_grade,
  put_grade,
  get_determinism,
  put_determinism,
  serializable,
  provenance,
}
```

#### 8.5.2 Why each field exists

| Field | Immediate consumer | Why it cannot be reconstructed cheaply later |
|---|---|---|
| `lift` | composition of nested optics | nested regions become ambiguous after lowering |
| `get_reads` | coeffect judgment and fusion legality | read structure gets flattened by codegen |
| `put_reads` | alias checker | read-for-update hazards are easy to miss later |
| `put_writes` | alias checker and backend stores | store sets must remain explicit |
| `get_grade` / `put_grade` | checker and future asymmetric optics | later host optics need direction-sensitive budgets |
| determinism bits | replay and coinduction gates | nondeterminism is easier to prevent than recover from |

Every named optic in the typed HIR carries an `OpticSummary`. Without that object, the compiler has no compact representation of the information later phases actually need.

```text
OpticSummary {
  costate,
  focus,
  lift,
  get_reads,
  put_reads,
  put_writes,
  get_grade,
  put_grade,
  get_determinism,
  put_determinism,
  serializable,
  provenance,
}
```

Each field exists because a later rule depends on it.

- `lift` is required for nested composition.
- `get_reads`, `put_reads`, and `put_writes` are required for alias analysis.
- `get_grade` and `put_grade` are required both for checking and future asymmetric I/O support.
- determinism and serializability are reserved because replay and coinduction will need them later.

This is the point where a future reader should see the blueprint logic explicitly. The summary is intentionally slightly richer than the prelude strictly needs because the later language will reuse exactly this record rather than inventing a second one. Asymmetric `get_grade`/`put_grade`, richer determinism classes, serializability, replay, and boundary summaries are all already visible here as dormant structure rather than future semantic ruptures.

### 8.6 Path lifting and why nested optics are otherwise underspecified

A child optic only knows how to speak about its focus-relative regions. A parent composition needs those regions restated in the source costate. That is the purpose of `PathLift`.

For a field lens focusing `positions` out of `Entities`, the lift maps focus-relative paths back to source paths. Without that operation, summary composition would be approximate in exactly the wrong places: nested writes and read-for-update hazards.

### 8.7 Summary composition sketches

For sequential composition, the key intuition is that a write-back through a nested optic may still need to read the outer focus again.

```text
summary(A >>> B):
  get_reads  = A.get_reads ∪ lift(A, B.get_reads)
  put_reads  = A.put_reads ∪ A.get_reads ∪ lift(A, B.put_reads)
  put_writes = A.put_writes ∪ lift(A, B.put_writes)
```

For product composition, the intuition is simpler: both children speak about the same costate, so the summary is mostly a union plus a later alias-safety check.

```text
summary(A *** B):
  get_reads  = A.get_reads ∪ B.get_reads
  put_reads  = A.put_reads ∪ B.put_reads
  put_writes = A.put_writes ∪ B.put_writes
```

The prose matters here because a raw formula can hide the operational reason. `put_reads` includes `A.get_reads` in the sequential case because rebuilding the enclosing structure is itself a read-for-update hazard.

### 8.8 Transition

Once HIR has explicit names, explicit cursors, and explicit summaries, the type checker can stop guessing and start enforcing. The next chapter explains how grades and alias safety are made algorithmic rather than aspirational.

### 8.9 Detailed implementation reference: HIR lowering, cursor insertion, and summary construction

This supplement makes the cursor model fully operational. It spells out the resolver tables, query-chain lowering, path lifting, and the specific summary-builder rules that later phases rely on.

The HIR phase performs name resolution, cursor insertion, query chain desugaring, and optic summary computation. It must not touch types or grades (those are `optic-typeck`'s job).

#### 8.9.1 Name resolution

##### 8.9.1.1 Resolver data structures

A minimal but robust resolver for v0 uses three explicit maps plus a scope stack:

```text
GlobalOptics   : Symbol -> OpticId
GlobalData     : Symbol -> DataId
BuiltinTable   : Symbol -> BuiltinId
LocalScopes    : Vec<HashMap<Symbol, LocalId>>
```

Resolution should never be "best effort". Every successful resolution produces a specific symbol class (`Local`, `NamedOptic`, `Data`, `Builtin`) and that class is recorded in the HIR node. This prevents later passes from reparsing or guessing symbol meaning.

##### 8.9.1.2 Resolver algorithm

```text
resolve_ident(name):
  for scope in LocalScopes from innermost to outermost:
    if name in scope: return Local(scope[name])
  if name in GlobalOptics: return NamedOptic(GlobalOptics[name])
  if name in GlobalData:   return Data(GlobalData[name])
  if name in BuiltinTable: return Builtin(BuiltinTable[name])
  emit RES-101(name)
  return ErrorSymbol
```

Determinism requires duplicate declarations to be rejected at insertion time. Shadowing inside closures is fine; ambiguous globals are not.

Resolution order for `IDENT` tokens:
1. Local variables in the current closure scope
2. Named optics in the current file
3. Named costates (`data` declarations) in the current file
4. Imported symbols (future; not v0)
5. Built-in types (`SoA`, `BitSet`, `Vec2`, `f32`, etc.)

Unresolved identifiers emit `RES-101`. Unresolved field names on a known costate emit `RES-111`.

Resolution must be deterministic: if two definitions share a name in the same scope, the second is an error, not a shadowing.

#### 8.9.2 Query chain lowering

Surface syntax:

```rust
entities.query(HealthView).map(|h| h - 10.0)
```

HIR form:

```text
QueryMap {
  costate: Var("entities"),
  optic:   OpticRef("HealthView"),
  cursor:  FreshCursor("cur_0"),
  fn:      Closure { param: "h", body: Sub(Var("h"), FloatLit(10.0)) },
  span:    ...,
}
```

The HIR makes the cursor explicit. Every query form introduces a fresh cursor name to make codegen straightforward.

##### 8.9.2.1 Query-chain lowering algorithm

```text
lower_query_chain(base_expr, optic_expr, methods):
  base   = lower_expr(base_expr)
  optic  = lower_optic_expr(optic_expr)
  cursor = fresh_symbol('cur')

  current = QuerySeed { costate: base, optic: optic, cursor: cursor }
  for m in methods:
    match m:
      get()   -> current = QueryGet { ...from current... }
      set(v)  -> current = QuerySet { ...from current..., value: lower_expr(v) }
      map(cl) -> current = QueryMap { ...from current..., fn: lower_closure(cl) }
  return current
```

The lowering is intentionally left-associated over the method chain so that source order is preserved exactly in spans and diagnostics. Any syntactic sugar that changes source ordering should be delayed until after type checking, not hidden inside the HIR builder.

#### 8.9.3 Cursor insertion rules

| Surface form | HIR cursor form |
|-------------|----------------|
| `s.healths[s.id]` in get body | `cursor.arena.healths[cursor.id]` |
| `s.healths[s.id] = v` in put body | `cursor.arena.healths[cursor.id] = v` |
| `s.id` | `cursor.id` |
| `s` (whole costate in get) | `cursor.arena` (read-only reference) |

The compiler should detect and reject any attempt to store or alias the cursor itself — it is a temporary view, not a first-class value in v0.

#### 8.9.4 HIR optic node shapes

```text
HirOptic =
  | Named(name: Symbol, span: Span)
  | Compose(lhs: HirOptic, rhs: HirOptic, span: Span)    -- >>>
  | Product(lhs: HirOptic, rhs: HirOptic, span: Span)    -- ***
  | Paren(inner: HirOptic, span: Span)                   -- for provenance
```

The HIR preserves parentheses as `Paren` nodes for error attribution. They are stripped during CGIR construction after operator precedence is confirmed.

#### 8.9.5 Summary computation

##### 8.9.5.1 Path-lift-aware composition rules

Simple unions are not precise enough once nested field composition matters. The implementation should compute summary composition with explicit path lifting.

For a summary:

```text
Ω = ⟨π, Rg, Rp, W, Gg, Gp, Dg, Dp, Ξ⟩
```

where `π` is the `PathLift`, the composition rules are:

```text
summary(A >>> B) =
  lift       = πA ∘ πB
  get_reads  = A.get_reads ∪ πA(B.get_reads)
  put_reads  = A.put_reads ∪ A.get_reads ∪ πA(B.put_reads)
  put_writes = A.put_writes ∪ πA(B.put_writes)
  get_grade  = combine_seq(A.get_grade, B.get_grade)
  put_grade  = combine_seq(A.get_grade, combine_seq(B.put_grade, A.put_grade))
  get_det    = join_det(A.get_det, B.get_det)
  put_det    = join_det(A.get_det, join_det(B.put_det, A.put_det))
  serializable = A.serializable && B.serializable
```

The extra `A.get_reads` term in `put_reads` is the important one: to put through a composed optic, the outer optic often needs to re-read the enclosing focus in order to rebuild it.

For product:

```text
summary(A *** B) =
  lift       = pair_lift(πA, πB)
  get_reads  = A.get_reads ∪ B.get_reads
  put_reads  = A.put_reads ∪ B.put_reads
  put_writes = A.put_writes ∪ B.put_writes   -- legality checked separately
  get_grade  = combine_par(A.get_grade, B.get_grade)
  put_grade  = combine_par(A.put_grade, B.put_grade)
  get_det    = join_det(A.get_det, B.get_det)
  put_det    = join_det(A.put_det, B.put_det)
  serializable = A.serializable && B.serializable
```

##### 8.9.5.2 Summary builder algorithm

```text
build_summary(named_optic):
  get_reads  = collect_regions(named_optic.get_body, mode='read')
  put_reads  = collect_regions(named_optic.put_body, mode='read')
  put_writes = collect_regions(named_optic.put_body, mode='write')
  get_grade  = infer_get_grade(get_reads)
  put_grade  = infer_put_grade(put_reads, put_writes)
  lift       = infer_lift(named_optic.focus_path)
  return OpticSummary(...)
```

This builder is the compiler's first real abstraction barrier. Every later legality decision assumes the summary is correct.

---

## 9. Type Checking, Grade Inference, and Alias Safety

### 9.1 Why this phase is more than conventional type checking

The prelude type checker is doing several jobs that many compilers scatter across different stages.

- ordinary type compatibility,
- ownership compatibility,
- grade inference and bound checking,
- product alias legality,
- summary validation,
- future-facing determinism bookkeeping.

The reason to keep them close is that the same artifact, `OpticSummary`, feeds all of them.

This is also where the narrow compiler most clearly acts as a blueprint for later growth. The concrete checker is intentionally small, but it is not a dead end. The same summary-driven discipline that powers the coarse v0 checker is the one later used by fractional ownership, asymmetric grades, and symbolic solving. The narrow checker therefore proves not only that the core laws can be enforced, but that they can be enforced through artifacts the later compiler can refine rather than replace.

### 9.2 The prelude type universe

The v0 type system is intentionally conservative.

- primitives,
- tuples,
- `SoA<T>`,
- `BitSet`,
- monomorphic user `data` types,
- `GradedOptic<S, A, G>`.

This small universe is not just easier to implement. It is easier to make *explainable*. When a product optic fails or a composition mismatches, the compiler can describe the failure in terms of concrete costate and focus types without having to explain higher-rank polymorphism or inference across a large subtype lattice.

### 9.3 Grade combination rules in v0

#### 9.3.1 Concrete carrier and operators

```rust
struct ConcreteGrade {
    cache: u8,                 // 255 = unbounded
    ownership: OwnershipDim,
}

struct OwnershipDim {
    share: Rational,           // 0 < share <= 1
    read_only: bool,           // true for shared borrows
    must_use: bool,            // true for linear resources
}

// Surface aliases used throughout the prelude examples:
// SharedGrade  = inferred read-only share ρ with 0 < ρ < 1
// AffineGrade  = { share: 1, read_only: false, must_use: false }
// LinearGrade  = { share: 1, read_only: false, must_use: true }
```

```text
sat_add(x, y) =
  if x == 255 or y == 255 or x + y > 254 then 255
  else x + y
```

The prelude grade algebra is small enough to state directly.

| Dimension | Sequential `>>>` | Product `***` |
|---|---|---|
| Cache | saturating add | conservative max |
| Ownership | take the stronger requirement (`share = max`, `must_use = or`, `read_only = and`) | prove either structural disjointness or a partition-safe fractional split; otherwise reject |

That table is easy to memorize, but the prose behind it matters.

Sequential cache cost uses saturating add because separate stages still touch separate field families even after fusion. Product uses conservative max because the stages share a pass and the hardware can often overlap some of the locality cost, but the compiler stays conservative.

Ownership is now slightly richer than the v0-era prose suggested. For ordinary composed optics, sequential composition simply preserves the stronger ownership requirement of the two children. Parallel product is the interesting case. It is legal for two reasons only:

1. the regions are structurally disjoint in the existing conservative region language; or
2. the regions participate in an already-established partition witness and the claimed ownership fractions sum to at most one.

This is the precise place where early fractional ownership earns its keep. The checker still accepts the easy field-wise cases through structural disjointness, but it no longer needs a later carrier swap when partition-shaped multicore work begins to matter.

### 9.4 Why grade inference is region-driven

A prelude compiler should not infer grades from source syntax quirks. It should infer them from normalized region summaries.

```text
reads  = distinct field roots touched by get
writes = distinct field roots touched by put
cache  = reads + writes, saturated
```

This is intentionally coarse. That coarseness is a feature in v0 because it makes the algorithm stable across harmless rewrites.

### 9.5 Alias safety under product composition

#### 9.5.1 Alias-checking sketch

```text
overlaps(r1, r2):
  is_subregion(r1, r2) or is_subregion(r2, r1)

same_partition_family(r1, r2):
  r1.partition_witness is not None
  and r1.partition_witness == r2.partition_witness

alias_check(left, right):
  left_effective  = left.get_reads ∪ left.put_reads ∪ left.put_writes
  right_effective = right.get_reads ∪ right.put_reads ∪ right.put_writes

  for each overlapping pair (l, r) with l in left.put_writes and r in right_effective:
    if left.ownership.read_only and right.ownership.read_only:
      require left.ownership.share + right.ownership.share <= 1
      continue
    if same_partition_family(l, r):
      require left.ownership.share + right.ownership.share <= 1
      continue
    return conflict(left_region=l, right_region=r)

  repeat symmetrically for right.put_writes against left_effective

  return safe
```

The algorithm remains conservative on purpose. Structural region disjointness still proves the easy cases; fractional arithmetic takes over only when the program has already established a partition witness. This is the precise compromise the book now adopts: front-load the harder carrier without pretending the prelude can already infer every dynamic partition automatically.

Parallel product is one of the most attractive surface forms in the language because it reads like a data-oriented system query. It is also one of the places where unsoundness could sneak in if the compiler were optimistic.

The rule is now slightly richer than the original v0 prose implied.

```text
alias_safe(left, right) iff
  for every overlapping or potentially-overlapping region pair:
    either the regions are structurally disjoint,
    or both sides are read-only and their shares sum to at most one,
    or both sides participate in the same established partition family and their shares sum to at most one;
  otherwise reject.
```

The important point is still that `put_reads` counts. If a write on one side overlaps with the other side's read-for-update path, the product is unsafe even if the second side never stores directly to that field. Fractional ownership does not weaken that rule; it only gives the checker a principled way to accept partitioned parallel products without inventing a second ownership system later.

### 9.6 Why the checker stays conservative

The prelude does not try to prove index-level disjointness, dependence on runtime values, or deep symbolic region separation. It normalizes indexed SoA access to `field[*]` and accepts the resulting false rejections.

That choice is not cowardice. It is the correct first trade.

- A false rejection is annoying but explainable.
- A false acceptance is unsound and poisons later optimization.

A compiler meant to support agent workflows should prefer conservatism with excellent evidence over adventurous acceptance with hidden traps.

#### 9.6.1 Boundary contracts extend `OpticSummary` without creating a second effect system

The narrow checker already depends on `OpticSummary` to carry the facts that later phases need: regions, grades, determinism, and provenance. The most conservative way to admit FFI, `unsafe`, MMIO, callbacks, and legacy runtimes is therefore to extend that same summary rather than invent a parallel subsystem.

A minimal extension looks like this.

```rust
struct BoundaryContract {
    kind: BoundaryKind,              // Local | Extern | Intrinsic | Asm | Volatile
    abi: Option<AbiKind>,
    unwind: UnwindPolicy,            // NoUnwind | MayUnwind | ForeignException
    may_callback: bool,
    reentrant: Reentrancy,           // No | Yes
    thread_affinity: ThreadAffinity, // Any | Main | Render | Audio | Cpu(n)
    address_space: AddressSpace,     // Ram | Mmio | Dma | Gpu | ForeignHeap | ManagedHeap
    volatility: Volatility,          // Ordinary | Volatile
    atomicity: Atomicity,            // None | Atomic(ordering)
    privilege: PrivilegeLevel,       // User | Kernel | Interrupt
    pinning: PinRequirement,
    allocator: AllocatorContract,
    layout: LayoutContract,
    stageability: Stageability,      // Static | Residual | Dynamic
    safety_clauses: Vec<SafetyClause>,
}

struct OpticSummary {
    // existing fields omitted
    boundary: Option<BoundaryContract>,
}
```

The key design choice is what does **not** change. The language still has the same root runtime, the same region model, the same `get_reads`/`put_reads`/`put_writes` split, the same grade product, and the same CGIR composition operators. A foreign or unsafe leaf is still an optic leaf. It simply carries extra obligations.

That keeps the architecture small enough to reason about. The type checker still asks: what regions are read, what regions are written, what grade is consumed, and what determinism class results? The boundary contract only answers the extra questions that ordinary in-memory optics never had to answer: what ABI is crossed, whether unwinding may escape, whether the call may reenter the language, which address space the memory lives in, whether the operation is volatile or atomic, and what safety preconditions must already hold.

A raw foreign declaration may therefore exist in the surface language, but it becomes useful to the compiler only once it is wrapped in an optic-shaped summary. That is why the book prefers safe or semi-safe wrappers over raw foreign items in ordinary code. The raw item establishes the ABI. The optic wrapper re-enters the graph.

```rust
extern "C" fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8;

unsafe optic MmioReg32: GradedOptic<Mmio<U32Reg>, u32,
    BlockingGrade<Never> + LinearGrade>
{
    get  r => volatile_load_u32(r.base + r.offset)
    put  (r, v) => volatile_store_u32(r.base + r.offset, v)
    safety {
        requires privilege(Kernel)
        requires mapped_mmio(r.base, 4)
        requires aligned(r.base + r.offset, 4)
        ensures no_unwind
    }
}
```

The first declaration is a raw ABI fact. The second is the graph-facing, summary-bearing object the rest of the language can reason about.

### 9.7 Diagnostics must expose the proof, not just the verdict

A grade or alias error becomes useful only when the compiler tells the user what it saw.

For example:

```text
error[ALI-201]: product alias conflict
  left writes:  app.healths[*]
  right reads:  app.healths[*]
  note: reads in the right optic occur during put-read reconstruction
```

That is better than a generic "cannot borrow mutably" style message because it is phrased in the language's own conceptual units.

### 9.8 Transition

Type checking proves local legality. The next chapter explains why the compiler still needs a graph-shaped IR above SSA, how that IR is constructed, and how it becomes the home of fusion and provenance.

### 9.9 Detailed implementation reference: concrete grade arithmetic and checker structure

The main chapter explains the purpose of the checker; the material below gives the concrete carrier types, inference path, and the solver-separation pattern that keeps v0 forward-compatible with symbolic grades.

#### 9.9.1 Grade representation

In v0, a `ConcreteGrade` is a pair:

```rust
struct ConcreteGrade {
    cache:     CacheDim,      // u8; 0 = no cache cost; 255 = unbounded
    ownership: OwnershipDim,  // Shared | Affine | Linear
}
```

Grade annotations in source can use `_` for inference. The compiler fills in the tightest provable grade.

#### 9.9.2 Grade combination rules

Sequential composition `(A >>> B)`:

```text
combine_seq(a, b).cache     = sat_add(a.cache, b.cache)
combine_seq(a, b).ownership = max_exclusivity(a.ownership, b.ownership)

sat_add(x, y) = if x == 255 || y == 255 || x + y > 254 { 255 } else { x + y }
max_exclusivity(Shared, Shared)   = Shared
max_exclusivity(Shared, Affine)   = Affine
max_exclusivity(Shared, Linear)   = Linear
max_exclusivity(Affine, Affine)   = Affine
max_exclusivity(Affine, Linear)   = Linear
max_exclusivity(Linear, Linear)   = Linear
-- symmetric
```

Parallel product `(A *** B)` (after alias safety confirmed):

```text
combine_par(a, b).cache     = max(a.cache, b.cache)   -- conservative
combine_par(a, b).ownership = max_exclusivity(a.ownership, b.ownership)
```

#### 9.9.3 Grade inference algorithm (v0: concrete only)

##### 9.9.3.1 Region-driven touch counting

The concrete inference path should operate over the normalized region summary, not over surface syntax. That keeps the algorithm stable under harmless source rewrites.

```text
count_distinct_field_reads(expr):
  regions = collect_regions(expr, mode='read')
  return cardinality(normalize_to_field_roots(regions))

count_distinct_field_writes(expr):
  regions = collect_regions(expr, mode='write')
  return cardinality(normalize_to_field_roots(regions))
```

`normalize_to_field_roots` deliberately collapses `app.positions[*].x` and `app.positions[*].y` to the same field family if they live in the same SoA vector, because the v0 cache story is about field-touch count, not scalar-lane count.

##### 9.9.3.2 Why `u8` with 255 = unbounded is the right carrier in v0

`u8` keeps grades cheap to serialize, compare, dump, and embed in diagnostics. More importantly, it makes saturation behavior explicit and testable. Every time the compiler reaches `255`, it is forced to admit that the structural approximation has escaped the bounded regime.

That is a useful signal to both humans and agents. An unbounded grade is not a mysterious solver failure; it is a visible boundary of the prelude's approximation.

Grade inference fills in `_` annotations in optic declarations. The algorithm is a single bottom-up pass over the optic body.

```text
infer_grade(optic_body) ->
  reads  = count_distinct_field_reads(get_body)   -- number of distinct SoA fields read
  writes = count_distinct_field_writes(put_body)  -- number of distinct SoA fields written
  cache  = reads + writes                          -- conservative: one cache line per field
  ownership = if put_body is empty { Shared }
              else if put_body does not read same field it writes { Affine }
              else { Affine }   -- Linear requires explicit declaration for now
  return ConcreteGrade { cache, ownership }
```

Inferred grades are upper bounds; they may be tighter than the true hardware cost. This is intentional: inference is conservative, declaration is the programmer's claim.

If a declared grade is tighter than the inferred grade (e.g., declared `CacheGrade<1>` but body reads two fields), the compiler emits `GRA-110`:

```text
error[GRA-110]: declared grade is tighter than inferred grade
  declared: CacheGrade<1>
  inferred: CacheGrade<2>  (body reads: app.healths, app.positions)
  note: either tighten the body or relax the declared grade
```

#### 9.9.4 Grade bound checking

After composition, the composed grade is checked against any declared bound on the outer let-binding or function signature.

```text
check_grade_bound(composed: ConcreteGrade, declared_bound: ConcreteGrade) ->
  if composed.cache > declared_bound.cache:
    emit GRA-104
  if exceeds_ownership(composed.ownership, declared_bound.ownership):
    emit GRA-121
```

#### 9.9.5 Preparation for two-tier inference (full language)

##### 9.9.5.1 Division of responsibilities between inference and checking

Even before symbolic grades exist, the compiler should keep three logically separate phases:

1. **Summary extraction** — derive structural facts from optic bodies.
2. **Grade expression construction** — build a grade object from those facts.
3. **Bound checking** — compare the resulting grade against declarations or enclosing requirements.

This separation matters because only phase 2 needs a future symbolic solver. Phases 1 and 3 should remain structurally the same when the full-language solver arrives.

The v0 implementation should structure grade checking so that the concrete inference path (§8.3) and the constraint-emission path (§8.4) are separate functions. When symbolic grades are added in the full language, the constraint-emission path will delegate to a Z3 query manager rather than performing arithmetic directly. The architecture must support this substitution without a rewrite.

The Z3 interface (not implemented in v0, but the interface must be sketched):

```rust
trait GradeConstraintSolver {
    fn check_bound(composed: GradeExpr, bound: GradeExpr) -> Result<(), GradeViolation>;
    fn infer_tightest(body: &OpticBody) -> GradeExpr;
}

// v0: concrete arithmetic
struct ConcreteGradeSolver;
impl GradeConstraintSolver for ConcreteGradeSolver { ... }

// full language: Z3 QF_LIA
struct Z3GradeSolver { solver: z3::Solver }
impl GradeConstraintSolver for Z3GradeSolver { ... }
```

This abstraction costs nothing in v0 and eliminates a later architectural rupture.

The important implementation consequence is that the symbolic solver should first arrive as a comparison and advisory lane rather than as a destabilizing replacement for the narrow checker. That preserves the prelude’s auditability while still letting the project measure where the coarse checker is too conservative for realistic workloads.

---

## 10. CGIR, Provenance, and Fusion

### 10.1 Why CGIR exists above SSA

SSA is excellent once the compiler has already committed to a lower-level control-flow structure. It is a poor place to *discover* optic composition, product adjacency, or provenance. CGIR exists because the language wants its own structure to remain explicit until that structure has paid out its optimization and diagnostic value.

Seen historically, this is a deliberate defense against a recurring compiler failure mode: lower away the language’s structure too early, then spend the rest of the optimizer trying to reconstruct what the source already knew. CGIR is a recovered paradigm in that sense. It keeps the graph-shaped structure that older optimizer pipelines often flattened into generic control flow before fusion legality, provenance, and domain intent had been fully harvested.

In other words: CGIR is not a fancy pretty-printer for the AST. It is the implementation form of the language.

### 10.2 Core node families

#### 10.2.1 A concrete CGIR catalog

```text
CgirNode =
  | OpticLeaf  { id, name, costate, focus, summary, get_fn, put_fn, provenance }
  | Compose    { id, lhs, rhs, grade, provenance }
  | Product    { id, lhs, rhs, grade, alias_safe, provenance }
  | QueryGet   { id, optic, costate, cursor, provenance }
  | QuerySet   { id, optic, costate, cursor, value, provenance }
  | QueryMap   { id, optic, costate, cursor, map_fn, provenance }
  | FusedLoop  { id, original_ids, costate, body, provenance }
  | Tap(...) | Coinductive(...) | Stage(...) | Record(...)   // reserved in v0
```

#### 10.2.2 Invariants the verifier must enforce

| Invariant | Consequence if violated |
|---|---|
| node ids are unique | snapshots and provenance become unstable |
| `Compose.lhs.focus == Compose.rhs.costate` | fusion may synthesize invalid loops |
| `Product` children share costate type | generated loop would read different arenas as one |
| `alias_safe` is true before codegen | product lowering becomes unsound |
| fused loops name at least two originals | provenance is lying about optimization |

The prelude CGIR needs only a small node catalog, but each node must carry real semantic weight.

| Node family | Purpose |
|---|---|
| `OpticLeaf` | named, summarized optic definition |
| `Compose` | sequential optic composition |
| `Product` | same-costate product composition |
| `QueryGet`, `QuerySet`, `QueryMap` | action roots over a costate |
| `FusedLoop` | post-optimization materialized loop body |

Reserved future nodes such as `Tap`, `Coinductive`, `Stage`, and `Record` should exist in the type universe even if the prelude rejects them in source. Reserving structure early avoids a later IR rupture.

### 10.3 Construction rules and why they are bottom-up

CGIR is built bottom-up from typed HIR because the legality checks are compositional.

- A named optic becomes an `OpticLeaf` populated from its summary.
- `A >>> B` becomes `Compose(A, B)` after checking focus-costate compatibility.
- `A *** B` becomes `Product(A, B)` after checking shared costate type and alias safety.
- Query actions become graph roots because they define how the optic graph is actually run.

This bottom-up shape mirrors the book's broader design rule: do not flatten structure earlier than necessary.

### 10.4 Provenance is a semantic requirement, not a debugging afterthought

One of the critique's strongest warnings was that aggressive fusion can destroy debuggability if provenance is treated casually. This book accepts that warning as a design requirement.

Every node in CGIR must have non-dummy provenance. Every fused node must carry the union of original node ids and spans. Every codegen path must preserve enough of that provenance that a profiler or diagnostic can still name the source optics involved.

The reason this is so important is simple: if the language's unit of reasoning is the optic, then the implementation must preserve optics as the unit of explanation even after it stops preserving them as separate runtime loops.

### 10.5 The three prelude fusion passes

#### 10.5.1 Fixed-point driver

```text
optimize(graph):
  changed = true
  iters = 0
  while changed and iters < 8:
    changed = false
    graph, c1 = map_fusion(graph)
    graph, c2 = compose_fusion(graph)
    graph, c3 = product_flatten(graph)
    changed = c1 or c2 or c3
    verify(graph)
    iters += 1
  return graph
```

#### 10.5.2 Why the pass order is deliberate

Map fusion runs first because it removes trivial intermediate structure without changing graph edges. Compose fusion then sees cleaner escape patterns. Product flattening runs last because it is primarily a canonicalization pass for the backend.

The prelude optimizer stays intentionally small.

That smallness should not be misread as an attempt to replace a mature SSA optimizer. CGIR’s job is narrower and more specific: preserve optic structure long enough to justify structural fusion, provenance retention, and summary-driven legality. Once that work is complete, the native backend is expected to take over ordinary scalar and target-level optimization rather than forcing CGIR to imitate decades of SSA engineering.

#### 10.5.3 Map fusion

Chained pure maps over the same query root collapse into one map. The gain is reduced intermediate value traffic and clearer generated code.

#### 10.5.4 Compose fusion

When an intermediate focus does not escape, a sequential composition can collapse into one loop body. This is the first place where the language demonstrates a real zero-cost abstraction claim.

#### 10.5.5 Product flattening

Nested products normalize into a flatter internal representation so codegen does not spend its budget on nested tuple noise.

### 10.6 Why fixed-point optimization is enough in v0

The prelude optimizer should be a tiny fixed-point engine over a very small pass set. That is the right level of ambition.

A big rewrite system would create two problems at once.

- It would make the legality story harder to audit.
- It would make performance regressions harder to localize.

By contrast, a fixed pass order over a small node set gives the project something it badly needs early: predictable diffs.

### 10.7 A representative fused loop story

Suppose the source says:

```rust
entities.query(HealthView *** PositionView).map(|(h, p)| damage(h, p));
```

CGIR first makes the product explicit. Fusion then proves that both fields can be read in one pass and the update can be written back without alias conflict. The backend no longer sees "a library abstraction over two optics". It sees a single loop with two field loads, one transformation, and two stores.

That is the whole point of keeping CGIR above SSA long enough for the optic structure to matter.

### 10.8 Transition

Once CGIR is fused and canonicalized, the last prelude question is whether the backend can make the abstraction disappear in readable code. The answer must be yes before the language moves on.

### 10.9 Detailed implementation reference: CGIR catalog, construction, and verifier rules

The next sections are the precise node-level reference for CGIR. They specify graph container shape, node variants, builder behavior, and the invariants that `optic dump-cgir --check` must enforce.

The Coalgebra Graph IR is the compiler's optimization boundary. Every optimization pass operates on the CGIR and must preserve the provenance links that connect each node to source spans.

#### 10.9.1 Design principles

- **Explicit provenance on every node.** A node without a source span is a compiler bug.
- **No implicit sharing.** Each CGIR node has a stable `NodeId: u32` assigned at construction. Nodes are not deduplicated silently.
- **Immutable construction.** The initial CGIR is constructed from typed HIR and is never mutated in place. Each optimization pass produces a new CGIR from the old one.
- **Inspectable at every stage.** `optic dump-cgir --node X` must work on both pre- and post-fusion CGIR.

#### 10.9.2 Node catalog

##### 10.9.2.1 Supporting types and graph container

The node catalog is easier to implement if the graph container itself is explicit.

```rust
pub type NodeId = u32;

pub struct CgirGraph {
    pub nodes: IndexVec<NodeId, CgirNode>,
    pub roots: Vec<NodeId>,
    pub provenance_index: BTreeMap<NodeId, FusionProvenance>,
}

pub struct FusionProvenance {
    pub original_ids: Vec<NodeId>,
    pub spans: Vec<SourceSpan>,
    pub reason: FusionReason,
}

pub enum FusionReason {
    MapFusion,
    ComposeFusion,
    ProductFlattening,
}

pub enum Determinism {
    Pure,
    Seeded,
    Recorded,
    Opaque,
}
```

Even in v0, storing a structured determinism enum is worth it. The prelude may only exercise `Pure` and `Opaque`, but reserving the full shape now prevents a later refactor when replay and recorded inputs appear.

##### 10.9.2.2 Canonical node-shape rules

A CGIR builder should enforce the following canonical forms before optimization begins:

- `Paren` nodes from HIR do not survive into CGIR; their spans are merged into child provenance.
- Query nodes are graph roots; they are never nested under other query nodes.
- `Compose` and `Product` nodes refer only to non-query child nodes.
- `OpticLeaf.get_fn` and `put_fn` contain only normalized cursor forms, not arbitrary source syntax.
- Reserved future nodes may exist in the enum but are never constructed in v0.

Canonical forms shrink the optimizer surface area and make diff-based debugging much easier.

```text
CgirNode =
  -- Leaf nodes
  | OpticLeaf {
      id:         NodeId,
      name:       Symbol,
      costate:    TypeRef,
      focus:      TypeRef,
      grade:      ConcreteGrade,
      get_fn:     CgirExpr,
      put_fn:     CgirExpr,
      summary:    OpticSummary,
      provenance: SourceSpan,
    }

  -- Composition nodes
  | Compose {
      id:         NodeId,
      lhs:        NodeId,
      rhs:        NodeId,
      grade:      ConcreteGrade,       -- semiring product of lhs and rhs grades
      provenance: SourceSpan,
    }
  | Product {
      id:         NodeId,
      lhs:        NodeId,
      rhs:        NodeId,
      grade:      ConcreteGrade,       -- semiring sum of lhs and rhs grades
      alias_safe: bool,                -- set by alias checker, never assumed
      provenance: SourceSpan,
    }

  -- Query nodes (attached to a costate variable)
  | QueryGet {
      id:         NodeId,
      optic:      NodeId,
      costate:    CgirExpr,            -- the arena variable
      cursor:     Symbol,              -- fresh cursor name
      provenance: SourceSpan,
    }
  | QuerySet {
      id:         NodeId,
      optic:      NodeId,
      costate:    CgirExpr,
      cursor:     Symbol,
      value:      CgirExpr,
      provenance: SourceSpan,
    }
  | QueryMap {
      id:         NodeId,
      optic:      NodeId,
      costate:    CgirExpr,
      cursor:     Symbol,
      map_fn:     CgirExpr,            -- (focus) -> focus
      provenance: SourceSpan,
    }

  -- Fusion artifacts (only appear post-optimization)
  | FusedLoop {
      id:           NodeId,
      original_ids: Vec<NodeId>,       -- all nodes that were fused into this
      costate:      CgirExpr,
      body:         CgirExpr,          -- the merged loop body
      provenance:   FusionProvenance,  -- span union of all originals
    }

  -- Reserved future nodes (present in node type but rejected in v0)
  | Tap(...)         -- observability tap (reserved)
  | Coinductive(...) -- reactive loop (reserved)
  | Stage(...)       -- partial evaluation boundary (reserved)
  | Record(...)      -- DST recording (reserved)
```

Any attempt to construct a reserved node in v0 emits `OPT-3xx` and halts CGIR construction.

#### 10.9.3 CGIR expressions (`CgirExpr`)

```text
CgirExpr =
  | Var(Symbol)
  | FieldAccess(CgirExpr, Symbol)         -- expr.field
  | IndexAccess(CgirExpr, CgirExpr)       -- expr[idx]
  | CursorField(Symbol, CursorField)      -- cursor.arena | cursor.id
  | Assign(CgirExpr, CgirExpr)           -- lhs = rhs (in put bodies)
  | Call(Symbol, Vec<CgirExpr>)          -- helper call
  | Tuple(Vec<CgirExpr>)                 -- (a, b, ...)
  | TupleGet(CgirExpr, usize)            -- expr.N
  | Lit(Literal)                          -- integer/float literal
  | BinOp(BinOp, CgirExpr, CgirExpr)
  | Closure(Vec<Symbol>, Box<CgirExpr>)  -- |params| body
  | Block(Vec<CgirStmt>, Box<CgirExpr>)  -- { stmts; expr }
```

#### 10.9.4 CGIR construction rules

##### 10.9.4.1 Builder driver and invalid-node propagation

```text
build_cgir(typed_hir_program):
  graph = empty_graph()
  for item in typed_hir_program.items:
    if item is named optic:
      build_named_optic(graph, item)
    elif item is let-bound optic expression:
      build_root_expr(graph, item.expr)
    elif item is query root:
      root_id = build_query_root(graph, item)
      graph.roots.push(root_id)
  return graph

build_root_expr(graph, expr):
  match expr:
    Named(name)      -> make_leaf(graph, summary_table[name])
    Compose(l, r)    -> build_compose(graph, build_root_expr(graph,l), build_root_expr(graph,r))
    Product(l, r)    -> build_product(graph, build_root_expr(graph,l), build_root_expr(graph,r))
    ErrorExpr        -> make_invalid(graph)
```

Every builder entry point returns either a valid `NodeId` or an `Invalid` placeholder node that carries the emitted diagnostic id. Invalid nodes allow the compiler to continue collecting errors in a single run while still preventing optimization and codegen from operating on broken graphs.

CGIR is constructed from typed HIR in a single bottom-up traversal. Construction rules:

```text
construct(HirOptic::Named(name)) ->
  OpticLeaf { ... fields from OpticSummary table ... }

construct(HirOptic::Compose(lhs, rhs)) ->
  let l = construct(lhs)
  let r = construct(rhs)
  -- type check: l.focus == r.costate, else TYP-201
  let grade = combine_seq(l.grade, r.grade)
  -- grade bound check: if grade > declared bound, emit GRA-104
  Compose { lhs: l.id, rhs: r.id, grade, ... }

construct(HirOptic::Product(lhs, rhs)) ->
  let l = construct(lhs)
  let r = construct(rhs)
  -- costate must match: l.costate == r.costate, else TYP-201
  let grade = combine_par(l.grade, r.grade)
  let safe = alias_check(l.summary, r.summary)
  if !safe: emit ALI-201
  Product { lhs: l.id, rhs: r.id, grade, alias_safe: safe, ... }
```

If any construction step fails, the CGIR node is marked `Invalid` and construction continues to collect further errors. An `Invalid` node must never be passed to the optimizer or codegen.

#### 10.9.5 CGIR invariants

##### 10.9.5.1 Verifier algorithm

The invariant checker should be a first-class command, not an internal assertion bundle. A simple verifier pass is enough for v0:

```text
verify(graph):
  check_unique_node_ids(graph)
  check_root_ids_exist(graph)
  dfs_check_acyclic(graph)
  for node in graph.nodes:
    match node:
      Compose(lhs, rhs):
        require focus(lhs) == costate(rhs) else CGI-410
      Product(lhs, rhs, alias_safe):
        require costate(lhs) == costate(rhs) else CGI-410
        require alias_safe == true else CGI-410
      FusedLoop(original_ids):
        require len(original_ids) >= 2 else CGI-410
      _:
        pass
    require provenance(node) != Span::DUMMY else CGI-410
```

The verifier is the compiler's internal consistency oracle. Any pass that produces a graph that fails `verify` has not merely discovered a user error; it has violated a compiler contract.

These invariants must be checked by `optic-cgir`'s invariant checker (`optic dump-cgir --check`):

1. Every node has a unique `NodeId`.
2. Every `Compose` node's `lhs.focus == rhs.costate`.
3. Every `Product` node's `lhs.costate == rhs.costate`.
4. No `Invalid` nodes are present after type checking completes.
5. `alias_safe` on every `Product` node is `true` (alias checker ran and passed).
6. No cycles in the node reference graph.
7. Every `FusedLoop` node lists at least two `original_ids`.
8. `provenance` is never `Span::DUMMY` on a non-fused node.

---

### 10.10 Detailed implementation reference: fusion laws, fixed-point driver, and provenance obligations

Fusion is where the book’s promises become executable. The material below makes every rewrite concrete: pattern, precondition, rewrite shape, provenance retention, and the conditions under which a blocked fusion becomes a deliberate diagnostic instead of a silent miss.

The v0 optimizer implements exactly three fusion passes, applied in order. Each pass has named preconditions; if any precondition fails, the pass is skipped for that node and emits a `FUS-5xx` diagnostic if the failure was unexpected.

#### 10.10.1 Pass order

```text
Pass 1: Map fusion          (eliminates intermediate focus values between chained maps)
Pass 2: Compose fusion      (merges sequential optic loops into a single loop body)
Pass 3: Product flattening  (normalizes nested products into flat tuples)
```

Passes are applied to a post-type-check CGIR and produce a new CGIR. The original is preserved for provenance and `dump-cgir --before-fusion` inspection.

#### 10.10.2 Pass 1: Map fusion

##### 10.10.2.1 Map-fusion algorithm

```text
map_fusion(graph):
  changed = false
  for root in graph.roots:
    walk postorder(root):
      if node matches QueryMap(QueryMap(seed, f), g)
         and same_costate_and_optic(seed)
         and pure(f) and pure(g)
         and not captures_escape(f, g):
           replace node with QueryMap(seed, compose_closures(f, g))
           record_fusion(MapFusion, originals=[inner, outer])
           changed = true
  return (graph, changed)
```

Purity here means: no arena mutation, no host calls, no opaque builtins. The prelude should be explicit about this rather than silently assuming lambdas are harmless.

**Pattern:**

```text
QueryMap(QueryMap(costate, optic, cursor_a, f), optic, cursor_b, g)
```

**Precondition:** Both maps apply to the same optic on the same costate, and neither map body captures variables that escape into the outer context.

**Rewrite:**

```text
QueryMap(costate, optic, cursor_c, |x| g(f(x)))
```

**Provenance:** The `FusedLoop` node carries both original `QueryMap` spans. The generated Rust loop gets a `// fused: [Map1, Map2]` comment.

**Rewrite rule statement (for the test suite):**

```text
-- Map fusion law:
query(o).map(f).map(g)  ≡  query(o).map(x => g(f(x)))
-- provided f and g are pure (no arena side effects)
```

#### 10.10.3 Pass 2: Compose fusion

##### 10.10.3.1 Compose-fusion algorithm

```text
compose_fusion(graph):
  changed = false
  for root in graph.roots:
    walk postorder(root):
      if node matches QueryMap(costate, Compose(a,b), map_fn)
         and deterministic(a) and deterministic(b)
         and not intermediate_escapes(node)
         and compatible_for_single_loop(a,b):
           fused = make_fused_loop(costate, [a,b,node], synthesize_fused_body(a,b,map_fn))
           replace node with fused
           changed = true
  return (graph, changed)
```

`synthesize_fused_body` may still introduce a register-resident temporary for the outer focus. Fusion only promises elimination of *heap* or *loop-level* intermediates, not a mystical absence of temporaries altogether.

**Pattern:**

```text
QueryMap(costate, Compose(A, B), cursor, f)
```

**Precondition:**
1. Neither A nor B is `Invalid`.
2. `A.focus == B.costate` (type compatibility; already checked).
3. The intermediate focus value (A's output / B's input) does not escape into any outer binding.
4. Both A and B have `determinism = Deterministic`.

**Rewrite:**

```text
FusedLoop {
  original_ids: [A.id, B.id, QueryMap.id],
  body: |cursor| {
    let intermediate = A.get(cursor);
    let result = B.get(intermediate);   -- B operates on intermediate, not cursor directly
    let updated = f(result);
    B.put(cursor, updated);
    A.put(cursor, intermediate);        -- put flows right-to-left
  }
}
```

This is the core DoD optimization: two optics that would require two loop passes are merged into one.

**Fusion blocked:** If the intermediate value does escape (captured in a `let` that is used outside the query chain), emit `FUS-501`:

```text
note[FUS-501]: compose fusion blocked — intermediate value escapes
  introduce a stage boundary or capture the value explicitly
```

#### 10.10.4 Pass 3: Product flattening

##### 10.10.4.1 Product-flattening algorithm

```text
product_flatten(graph):
  changed = false
  for node in graph.nodes:
    if node matches Product(Product(a,b), c):
      replace node with ProductFlat([a,b,c])
      changed = true
    elif node matches Product(a, Product(b,c)):
      replace node with ProductFlat([a,b,c])
      changed = true
  return (graph, changed)
```

The point of flattening is not aesthetic. A flat product lowers to fewer nested tuples, fewer tuple projections, and more direct register allocation in the generated loop body.

**Pattern:**

```text
Product(Product(A, B), C)   or   Product(A, Product(B, C))
```

**Precondition:** All three optics share the same costate type.

**Rewrite:** Normalize to a left-leaning flat `Product` chain:

```text
Product_flat(A, B, C)   -- single flat product node
```

This reduces the number of nested tuple operations in the generated Rust.

#### 10.10.5 Provenance preservation rules

##### 10.10.5.1 Optimizer fixed-point driver

The optimizer should be implemented as a small deterministic fixed-point engine with explicit pass order and iteration caps.

```text
optimize(graph):
  g = graph
  changed = true
  iters = 0
  while changed and iters < 8:
    changed = false
    for pass in [map_fusion, compose_fusion, product_flatten]:
      (g2, changed_pass) = pass(g)
      g = g2
      changed = changed or changed_pass
    verify(g)
    iters += 1
  return g
```

The cap is not about hiding bugs; it is about making optimization behavior reproducible. If a rewrite system does not converge quickly on these tiny graph shapes, the rewrite laws themselves need attention.

##### 10.10.5.2 Why the pass order is semantically sensible

- Map fusion comes first because it reduces noise without changing the graph's structural edges.
- Compose fusion comes second because fewer intermediate map nodes means fewer apparent escape paths.
- Product flattening comes last because it is a shape-normalization pass whose result is easiest to reason about after other local simplifications have already happened.

This ordering is not sacred forever, but it is a clean v0 default with predictable diffs.

Every fusion pass must:

1. Carry all original `NodeId`s in the `FusedLoop.original_ids` list.
2. Carry the span union of all fused nodes as the new `provenance`.
3. Store the pre-fusion CGIR snapshot in the diagnostics context (accessible via `dump-cgir --before-fusion`).
4. Name the generated loop variable after the outermost optic in the fusion.
5. Emit `// optic(fused): [A, B, C]` as a comment in the Rust output.

Provenance must survive the full pipeline: fusion → codegen → benchmark. A profiler report must be traceable back to a named optic, even if that optic was fused.

---

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

# Part III — Extending the Core Without Breaking It

Part III grows the language only after the prelude has made the core boring in the right way. Every new feature in this part must reuse the same bridge objects—explicit roots, summaries, grades, provenance, and graph structure—rather than introducing a second semantic center. The order is deliberate and should be read as an argument. First come the new optic kinds whose machine shapes are the easiest to see: prisms become branches and traversals become bulk SIMD candidates. Then comes coinduction and staging, because staging is the universal glue that lets the same graph support local specialization, package resolution, artifact planning, generated bindings, and later self-hosting without splitting into a second meta-language. Only after those bridges are visible does the book enlarge the grade algebra, then hand the resulting proof objects to the native backend and multicore chapters, and finally widen the hostile-boundary story to cover full systems generality. Part V will return to several of the same themes, but in a different register: there the goal is to record the decisions and rejected alternatives, not to reteach the mechanics established here.

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

## 13. Prisms, Traversals, and the Branch/SIMD Bridge

Before the chapter names any optic-law detail, it should be clear why these two optic kinds enter the language before the rest. A **prism** is worth adding because it gives the compiler an explicit branch form rather than forcing it to rediscover partiality from boolean filters or hidden exception edges. A **traversal** is worth adding because it gives the compiler an explicit bulk-update form rather than hoping that a later backend recovers lane independence from an already-flattened loop nest. The type-theoretic refinement follows that machine picture; it does not replace it.

### 13.1 Prisms: typed partiality as branch structure

A prism focuses zero or one value.

```text
Prism<S, A> ≈ {
  preview: S -> Option<A>,
  review:  A -> S
}
```

The key reason prisms matter to this language is not just that they model optionality elegantly. It is that the machine-level realization of a prism is a branch. Once that is acknowledged explicitly, the design can connect type theory to branch prediction rather than treating control flow as an afterthought.

#### 13.1.1 Why `Option<A>` matters operationally

`Option<A>` is the coproduct `A + 1`. On the machine, eliminating a coproduct becomes a conditional. That is why a prism over an alive-bit, parse success, tag check, or fast-path validation naturally lowers to a branch.

#### 13.1.2 Branch bias as a grade

The full language therefore introduces a lightweight branch-bias grade.

| Bias | Backend consequence |
|---|---|
| `Likely` | emit likely metadata or equivalent branch hint |
| `Unlikely` | emit unlikely metadata |
| `Unknown` | no hint |

This is a good example of the language's style. A small piece of type-level information becomes a zero-cost backend hint with measurable pipeline consequences.

#### 13.1.3 Comparison to mainstream error and branching models

The comparison matters only at the point where it changes the design: other languages split ordinary partiality among nullable values, optionals, exceptions, and narrowing conventions, while Optic keeps recoverable partiality explicit enough to lower as visible branch structure.

| Language | Common partiality mechanism | What it gets right | Why Optic still prefers prisms in the core |
|---|---|---|---|
| **C++** | `bool` + out-params, `std::optional`, `std::variant`, often exceptions in legacy code | flexible and efficient when carefully designed | exceptions hide control edges; optionals are values but not automatically tied to reinsertion/update structure |
| **Java** | `null`, `Optional`, checked or unchecked exceptions | familiar enterprise control flow and rich exception taxonomies | `null` is too implicit, exceptions are too non-local, and `Optional` does not by itself become optimizer-facing branch structure |
| **Python** | `None`, EAFP exceptions, dynamic truthiness checks | concise code and easy local recovery | control shape is highly dynamic and therefore mostly invisible to a static optimizer |
| **TypeScript** | unions with `undefined`/`null`, user-defined narrowing, exceptions from JavaScript | ergonomic API typing | types erase before execution, so the narrowing information does not become a backend contract |
| **Rust** | `Option` and `Result` | the closest mainstream analogue to what Optic wants | prisms make the same value-level partiality composable as an optic form and connect it directly to branch-aware summaries and lowering rules |

The common thread is simple: Optic keeps ordinary failure value-shaped and optimizer-visible, and leaves non-local control for later optional machinery rather than making it the default.

### 13.2 Traversals: many homogeneous foci as bulk dataflow

A traversal enters the language because it is the first abstraction whose machine consequence is already obvious: if the same arithmetic transform applies independently to a regular field-wise layout, the backend should be able to emit packed loads, packed arithmetic, and packed stores rather than rediscovering that opportunity later by heuristic pattern matching. The semantics naturally line up with that data-parallel picture.

```text
Traversal<S, A> ≈ {
  traverse: S -> Vec<A>,
  update:   (S, Vec<A>) -> S
}
```

The machine does not literally need to allocate a `Vec<A>` in the hot path. What it needs is the semantic guarantee that a uniform transformation is being applied over many homogeneous elements.

That is exactly the shape that auto-vectorization and explicit SIMD prefer.

### 13.3 Why traversals map to SIMD so naturally

#### 13.3.1 `SimdEligible` checklist

A traversal is SIMD-eligible when all of the following are provable.

1. element `i` does not read or write element `j`,
2. the layout is regular and stride-uniform,
3. the mapped element type has a supported vector representation,
4. the element body has no prism-like divergence that would destroy lane coherence,
5. alias proofs are strong enough that vector stores are legal.

When these conditions hold, the backend can lower from a semantic traversal to a vector-width-sensitive loop rather than hoping a backend auto-vectorizer rediscovers the same facts later.

A traversal becomes SIMD-eligible when five conditions are met.

1. No cross-element dependencies
2. Uniform stride and regular layout
3. A homogeneous element type with supported lanes
4. No branchy element body that would explode divergence
5. Alias safety strong enough to reorder or batch stores

This is why the traversal chapter belongs next to the grade and memory-layout story. SIMD is not a "later optimization" layered onto an arbitrary abstraction. It is the machine image of a very particular semantic shape.

#### 13.3.2 Comparison to mainstream iteration and bulk-data traditions

The useful contrast is not that other languages lack bulk operators; it is that they usually express them either as library pipelines or engine-specific fast paths. Optic turns the vectorizable case into an explicit semantic kind.

| Language | Familiar bulk-transformation idiom | Why it is useful | What Optic adds |
|---|---|---|---|
| **C++** | STL algorithms, ranges, expression templates, manual SIMD libraries | excellent control when container and layout choices are disciplined | traversal shape, lane legality, and layout assumptions become part of the static semantic object rather than a happy accident of templates plus optimizer heuristics |
| **Java** | Streams, fork/join pipelines, vector API experiments | clear pipeline expression and runtime parallel execution | object layout and stream boxing often blur the direct path from semantic pipeline to cache-lawful native loop |
| **Python** | list comprehensions, generator chains, NumPy/Pandas vector kernels | unmatched expressive ease | serious performance usually moves into special array libraries; Optic tries to make the vector-friendly shape a language-level concept rather than a library escape |
| **TypeScript** | array methods, typed arrays, dataflow libraries | readable pipeline style | optimization is delegated to the JavaScript engine, and typed-array hot paths live beside a broader object world rather than organizing it |
| **Rust** | iterators, `rayon`, `packed_simd`/portable SIMD, explicit slices | very strong zero-cost bulk-programming story | the traversal kind records the vectorization-relevant semantics earlier and more explicitly than a general iterator chain |

Historically, the closest family resemblance is to APL and later array-language traditions. Those systems treated whole-array transformation as the semantic default rather than as a library optimization, and they made it natural to think in terms of bulk shape-preserving operations instead of scalar loops. Optic deliberately recovers that strength in a narrower, more systems-oriented form: the traversal is the array-lawful semantic shape, but it remains tied to explicit `SoA` layout, region summaries, and backend legality checks rather than becoming a universal glyph-heavy surface for every kind of program.

The point is therefore narrow and concrete: the traversal kind records lane legality, layout regularity, and update shape early enough that vectorization is justified by the semantics rather than recovered late by heuristics.

### 13.4 Example: health decay as traversal

#### 13.4.1 Scalar source, vector consequence

```rust
optic AllHealths: GradedTraversal<Entities, f32,
    TraversalGrade<N> + CacheGrade<1>> {
    traverse s => s.healths.iter()
    update   (s, f) => s.healths.iter_mut().for_each(f)
}
```

A backend targeting AVX2 can turn the hot path into an 8-lane `f32` loop; AVX-512 can widen that further. The important claim is not the exact speedup number. It is that the traversal semantics, SoA layout, and alias proofs together make the vector lowering *earned* rather than speculative.

```rust
optic AllHealths: GradedTraversal<Entities, f32,
    TraversalGrade<N> + CacheGrade<1>> {
    traverse s => s.healths.iter()
    update   (s, f) => s.healths.iter_mut().for_each(f)
}
```

If the traversal body is pure arithmetic, the backend can lower this to packed vector loads, arithmetic, and stores. The gain is not accidental. It follows from the traversal's multi-focus semantics and the SoA memory contract.

### 13.5 Lens and prism subtyping

A lens is a total prism. That coercion should be free. The language benefits from making it explicit because mixed pipelines become much more natural.

A total field lens can flow into a prism-accepting pipeline without any runtime wrapper; the compiler simply treats the `Some` arm as unconditional.

### 13.6 Transition

Prisms and traversals extend the static shape vocabulary. The next chapter extends the temporal and operational vocabulary: infinite loops, staged specialization, replay, and observability.

### 13.7 Detailed implementation reference: prisms as typed branch structure

The main chapter established the intuition. The following sections walk the entire chain from the coproduct reading of `Option<A>` to conditional lowering, branch-bias grades, and the reason prism subtyping gives a free coercion from total focus to partial focus.

#### 13.7.1 The type theory

A prism `P` focuses on a value of type `A` that may or may not be present inside a costate `S`. Categorically, it is a morphism in the category of optics with a cocartesian monoidal action:

```text
Prism<S, A> ≈ {
  preview: S -> Option<A>,   -- the partial observation
  review:  A -> S,           -- the injection back
}
```

The lens laws extend to two prism laws:

```text
-- Put-get (partial): if preview(s) == Some(a) then preview(review(a)) == Some(a)
-- Get-put (partial): if preview(s) == Some(a) then review(a) == s
```

These laws enforce that `review` and `preview` are mutually consistent and that the prism does not lose information when the focus is present. The compiler checks these laws statically for any prism body whose `preview` and `review` are pure expressions. For opaque prisms (bodies calling external functions), law checking is asserted at declaration time via `[checked]` or left to property-based tests.

#### 13.7.2 Why `Option<A>` is not just a convenience type

`Option<A>` is the canonical representation of the coproduct `A + Unit` — the sum type where the unit branch means "not present". This matters because the prism's machine-level lowering is exactly a conditional: either the focus is present (take the `Some` branch, produce `A`) or it is not (take the `None` branch, skip the update). The type forces the compiler to generate code that correctly handles both branches, with no risk of silent no-ops masquerading as failures.

#### 13.7.3 The machine-level picture

A prism query lowers to a conditional around the lens-like get/put body:

```text
-- Source:
entities.query(AliveFilter *** PositionView).map(|(h, p)| ...)

-- Lowered CGIR (after product with prism):
for id_0 in 0..entities.len() {
    if entities.flags.is_alive(id_0) {          // prism branch
        let _h = entities.healths[id_0];
        let _p = entities.positions[id_0];
        let (_h_new, _p_new) = update(_h, _p);
        entities.healths[id_0]   = _h_new;
        entities.positions[id_0] = _p_new;
    }
    // else: None branch is a no-op; no stores, no indirection
}
```

The grade arithmetic changes: a `Prism` under `***` with a `Lens` does not add cache pressure for the prism itself when the `preview` check is a single bit read (the common case for entity alive-flags). The cache grade for the conditional test is `CacheGrade<0>` if the flag is in the same cache line as the id counter, or `CacheGrade<1>` for a separate bitset. This is why prisms participate in the grade algebra directly: the conditional test cost is not free.

#### 13.7.4 Branch prediction implications

Because the prism branch is known to be data-dependent (not control-flow-dependent), the optimizer can emit `unlikely` hints for the `None` arm in dense living-entity worlds, or `likely` hints for sparsely-populated dead-entity passes. The grade carries a `BranchBias` annotation in the full language:

```rust
optic AliveFilter: GradedPrism<Entities, f32,
    CacheGrade<1> + BranchBias<Likely>>
```

`BranchBias<Likely>` tells the backend to emit `[[likely]]` on the `Some` branch in LLVM IR. `BranchBias<Unlikely>` emits `[[unlikely]]`. This is a grade dimension that is zero-cost after erasure but has a measurable effect on branch predictor warmup in the generated native code.

#### 13.7.5 Prism subtyping: Lens <: Prism

A `Lens` is a `Prism` that never returns `None`. In the type system, `Lens<S,A>` is a subtype of `Prism<S,A>` because every lens is a total prism. This means a function that accepts a `Prism` will also accept a `Lens`. The coercion is free: the compiler converts the lens's `get` into `preview = |s| Some(s.get())` during subtyping.

The key consequence is that mixed compositions like `Lens >>> Prism` type-check naturally. The composed optic is a `Prism`: it focuses on the lens focus if it exists, then optionally focuses further if the prism focus exists. Grade arithmetic carries through because the sequential composition rule still applies.

---

### 13.8 Detailed implementation reference: traversals, `TraversalGrade`, and SIMD eligibility

Traversals are where the language’s semantic uniformity starts paying direct machine dividends. The detailed sections below make the traversal-to-SIMD bridge explicit and spell out the conditions under which the backend may legally replace scalar loops with vector loads and stores.

#### 13.8.1 The type theory

A traversal focuses on zero or more values of type `A` inside a costate `S`. It is the optic corresponding to the `Traverse` type class in Haskell, but in the coalgebraic model it is a coalgebra for the monad of finite multisets:

```text
Traversal<S, A> ≈ {
  traverse: S -> Vec<A>,         -- all focused values (observations)
  update:   (S, Vec<A>) -> S,    -- update all focuses simultaneously
}
```

The key traversal law is **shape preservation**: the length of the `Vec<A>` returned by `traverse` must equal the length expected by `update`. Violating this is a type error (checked statically for pure traversals, asserted dynamically for opaque ones).

#### 13.8.2 Why traversals need their own grade dimension

A lens reads one element. A traversal reads all elements. The cache cost is not `CacheGrade<1>`; it is proportional to the collection size. For v0 this is expressed as `CacheGrade<N>` where `N` is statically known, or `CacheGrade<∞>` when the collection size is runtime-determined. In the full language, `TraversalGrade<n: Nat>` replaces the approximation with a proper size parameter:

```rust
optic AllHealths: GradedTraversal<Entities, f32, TraversalGrade<{entities.len()}>>;
```

When `n` is a compile-time constant (fixed-size arrays, stack-allocated rings), `TraversalGrade<n>` degrades to `CacheGrade<n / CACHE_LINE_SIZE>` exactly. When `n` is a runtime value, the grade is symbolic and handled by the Z3 solver tier.

#### 13.8.3 The SIMD opportunity

The traversal semantics — "apply the same function to all focused values" — is the precise statement of SIMD vectorization. A traversal body that satisfies:

1. No inter-element dependencies (element `i` does not read or write element `j`)
2. Uniform element stride (SoA layout, not AoS)
3. Arithmetic-only map function (no branches inside the element body)

...can be automatically lowered to SIMD instructions. The CGIR carries a `SimdEligible` flag on `Traversal` nodes that meet these conditions. The LLVM backend uses this flag to emit `llvm.x86.avx2.` or portable `LLVMBuildVectorStore` intrinsics rather than scalar loops.

```text
SimdEligible(traversal):
  traversal.map_fn has no inter-element reads/writes (alias check)
  traversal.costate has uniform element stride (SoA layout, always true in v0)
  traversal.map_fn has no prism branches (all paths execute for all elements)
  traversal.element_type is a SIMD-compatible scalar (f32, f64, i32, i64, u8...)
```

When `SimdEligible` is true, the optimizer emits:

```rust
// optic(simd): AllHealths
let chunks = entities.healths.chunks_exact_mut(8); // AVX2: 8 f32 per register
for chunk in chunks {
    let v = f32x8::from_slice_unaligned(chunk);
    let v_new = v - f32x8::splat(damage);
    v_new.write_to_slice_unaligned(chunk);
}
// handle remainder
for h in entities.healths[aligned_len..].iter_mut() { *h -= damage; }
```

This is a concrete, measurable throughput improvement (typically 6–8× over scalar for f32 arithmetic) that falls out of the traversal semantics without any user annotation.

#### 13.8.4 Traversal fusion with SIMD

When two SIMD-eligible traversals are fused (`AllHealths *** AllDamages`), the optimizer must decide whether to use two SIMD passes or one wider scalar pass. The grade algebra guides this decision:

```text
if combine_par(A.get_grade, B.get_grade).cache <= SIMD_REGISTER_WIDTH / ELEMENT_STRIDE:
    emit single fused SIMD loop (pack both fields into wider registers)
else:
    emit two separate SIMD loops (cache-friendly, prevents register pressure)
```

This is not guesswork. The grade arithmetic directly encodes the register width budget. If the total grade fits in one SIMD register, one pass. Otherwise two. The compiler makes this decision statically, not speculatively.

---

## 14. Coinduction, Staging, Replay, and Observability

### 14.1 Why live systems need coinduction

An HTTP server, scheduler, event pump, or frame loop is not an inductive structure that finishes by construction. It is an ongoing process observed step by step. Coinduction gives the language a way to say that honestly.

In the semantic core, a coinductive optic is the greatest fixed point of a step relation over a costate and its focused observation. In operational terms, it is the promise that the program can keep producing one more observable step without materializing an entire infinite structure.

### 14.2 Why coinduction maps cleanly to event rings

#### 14.2.1 A representative lowering sketch

```text
loop {
  submit_and_wait(ring, 1)
  cqe = next_completion(ring)
  packet = PacketPipeline.get(runtime, cqe)
  response = process(packet)
  PacketPipeline.put(runtime, response)
  advance_completion(ring)
}
```

The point of the sketch is not to claim that every coinductive pipeline becomes exactly this loop. It is to show why the semantic story lines up so naturally with ring-buffer-driven systems programming.

The later machine target for coinductive loops is a structured event loop over something like an io_uring completion ring.

The ring is the costate. Each completion is the next focus. The loop unfolds one observation at a time. The optic updates ring-visible state and host-facing buffers in place. This is one of the book's cleanest theory-to-machine correspondences.

```text
coinductive optic
  -> repeated observation of next completion
  -> no heap allocation in the hot path
  -> stable ring pointers and explicit liveness rules
```

#### 14.2.2 Comparison to existing async and event-loop cultures

Most mainstream languages already have ways to express long-lived I/O or reactive control. The design question here is narrower: which representation keeps enough structure alive long enough to guide optimization, replay, and tooling?

| Language | Common long-lived control model | Strength | Why Optic still wants coinductive nodes |
|---|---|---|---|
| **C++** | callbacks, coroutines, executors, custom event loops | high control and low overhead when hand-tuned | the source abstraction often becomes a state machine or callback graph before domain structure is available to the optimizer as a semantic object |
| **Java** | thread pools, NIO, Netty-style reactors, `CompletableFuture` | mature server architecture and tooling | runtime frameworks, not the type system, usually own liveness and backpressure structure |
| **Python** | generators, `asyncio`, callback/event loops | approachable cooperative concurrency | dynamic scheduling and interpreter overhead make machine consequences indirect and framework-dependent |
| **TypeScript** | promises, async functions, Node/browser event loops | natural fit for event-driven applications | the event loop is ambient and types erase, so the compiler cannot use it as an optimizer-facing graph node |
| **Rust** | `Future`, pinning, executors, async runtimes | explicit and efficient compared with other mainstream async systems | the final state-machine form is excellent for execution but does not preserve domain pipeline structure as a first-class graph the way CGIR coinduction can |

Historically, the closest semantic relatives are the synchronous and reactive languages Esterel and Lustre. Those languages made streams, clocks, and reactivity into first-class semantic structure rather than leaving them to framework protocol or callback folklore. Optic adopts that lesson in `Coinductive` nodes, `LivenessGrade`, and injected clock/input costates, but it deliberately does not make synchrony the universal execution model. Synchrony becomes a runtime-family choice and a typed host/grade contract rather than the semantics of every program.

Optic therefore treats coinduction as the semantic representation and executors or rings as lowerings, not as the starting point.

### 14.3 Liveness as a first-class grade

Once the language admits ongoing loops, it needs a liveness grade.

| Grade | Meaning |
|---|---|
| `Always` | intended to keep producing steps indefinitely |
| `Bounded(n)` | must terminate within `n` steps |
| `OnSignal` | terminates on an external control event |

This is the right place to encode facts that are too important to leave in runtime comments.

### 14.4 Compile-time execution, staging, and why no macro sublanguage is necessary

Staging is not an isolated convenience feature in this design. It is the **universal glue** that keeps the project coherent across scales. The same staging machinery that specializes a hot local loop also builds parser tables, resolves package graphs, materializes module interfaces, emits generated bindings, plans artifacts, and later lets the compiler run large parts of itself over `BuildRuntime`. If that work had to escape into separate macro systems, build scripts, or code generators, the language would immediately split into several partially visible semantic worlds.

The graph-based compiler architecture therefore makes a stronger promise plausible than most mainstream specialization systems attempt: a large fraction of the language can execute at compile time without retreating into token-tree macros, quasiquotation, AST splicing, or a second compile-time DSL.

The reason is structural. By the time the compiler reaches typed HIR and CGIR, it already has:

- an explicit graph of optic composition,
- region summaries over explicit roots,
- determinism and ownership facts,
- a distinction between build-time and runtime costates,
- and a place to cache specialized subgraphs.

That means compile-time execution can be treated as ordinary evaluation of ordinary optic graphs over `BuildRuntime`, followed by residualization of whatever still depends on live `Runtime` data.

#### 14.4.1 How much of the language can run at compile time?

The useful answer is neither "only tiny constexpr fragments" nor "everything, because the compiler is a graph". The real answer is:

> any maximal optic subgraph may execute at compile time if all of its free inputs are build-known, its operations are stage-admissible, it is deterministic and bounded enough to run in the compiler, and its effects are confined to compiler-owned artifacts rather than live runtime capabilities.

That is a broad set. It includes far more than scalar constant folding.

| Broadly compile-time eligible | Conditionally stageable | Runtime by default |
|---|---|---|
| parser tables, DFAs, perfect hashes, serialization tables | target-specific instruction selection, target-profile-dependent layout choices | live sockets, live disks, timers, randomness, user input |
| query plans, join orders, route tries, protocol state machines | file embedding, asset baking, shader reflection, device-tree decoding | coinductive event loops and scheduler ticks |
| compiler passes over AST/HIR/CGIR, peephole tables, register-class maps | data-dependent unrolling, precomputed lookup tables from large static inputs | DMA, GPU submission, MMIO, and other live linear host resources |
| ECS archetype masks, DOM/CSS selector automata, layout rule tables | staged code generation for hot paths when runtime shape is partly known | large traversals whose size, contents, or control flow are only known at runtime |

The practical slogan is simple: **most structure is stageable; live interaction is not**.

#### 14.4.2 Compile time is ordinary evaluation over the build projection of `Project`

This is where the costate story pays off. A compile-time route compiler, query planner, parser generator, IR rewrite pass, or layout planner should be ordinary Optic code over explicit build-time data:

```text
Project = ProjectGraph × RuntimeBlueprintSet
BuildRuntime = Project.build
             = CompilerGraph × TargetProfile × BuildHostContext × ArtifactCache
Runtime = Project.run[target, instance]
        = AppWorld × HostContext × ControlRuntime
```

A staged computation is therefore not "code that rewrites syntax". It is ordinary graph evaluation whose inputs happen to be statically available through the build projection of the project root. That is why the same surface forms can serve in both phases:

- `data` still declares costates,
- `optic` still declares transformations,
- `let` still binds graph structure,
- `query` still applies optics to explicit roots.

What changes is not the language. What changes is the phase and the costate.

That is why staging should be read as the operational engine of the whole project rather than as a local code-generation trick. Local specialization, package resolution, build planning, dependency closure, generated-binding pipelines, artifact planning, and self-hosted compiler passes are all the same kind of move: evaluate the static slice of the graph now, validate the residual obligations, and cache the result under an explicit project/root/target key.

#### 14.4.3 `stage {}` remains, but as a boundary hint, proof target, and cache-key anchor

The book still keeps `stage {}`. But its role is deliberately narrower than a macro system.

It exists for three reasons:
1. to let the programmer assert that a region of the graph ought to become static,
2. to give the compiler a named specialization and cache-key boundary,
3. to give diagnostics a precise place to explain why static execution failed or was declined.

Without `stage {}`, the compiler may still stage any maximal subgraph it can prove admissible. With `stage {}`, the compiler must either succeed, residualize with an explicit explanation, or emit a staging diagnostic. In other words, `stage {}` is a contract and tooling hook, not a token-rewriting escape hatch.

#### 14.4.4 Comparison to mainstream compile-time mechanisms

The comparison is only useful where it changes policy: Optic wants compile time to stay inside the same typed graph model instead of splitting across several partially visible mechanisms.

| Language | Usual compile-time mechanism | What it does well | Why Optic still chooses graph-stage evaluation |
|---|---|---|---|
| **C++** | templates, `constexpr`, generated code | extremely aggressive ahead-of-time specialization | template expansion is not the same as typed graph residualization over explicit runtime/build-time regions |
| **Java** | annotation processors, bytecode generation, JIT speculation | strong tooling and late adaptive optimization | compile-time and runtime specialization are usually framework or VM concerns, not part of one static effect/coeffect story |
| **Python** | import-time execution, decorators, metaclasses, codegen scripts | very flexible and expressive | build-time behavior is hard to bound, reproduce, or connect to optimizer legality facts |
| **TypeScript** | erased types, transformers, bundlers, code generators | strong design-time tooling | the type system disappears before code generation, so compile-time structure does not survive as optimizer-facing evidence |
| **Rust** | `const`, const generics, `build.rs`, declarative/proc macros | strong compile-time computation toolbox | compile time is split across several mechanisms, and macros still rewrite syntax rather than evaluate typed IR graphs |
| **Optic** | inferred stageability + optional `stage {}` over CGIR | one language, one graph, explicit cache keys and legality facts | requires stronger determinism, phase, and profitability analysis in the compiler |

#### 14.4.5 The real difficulty: policy, not representation

The graph structure makes compile-time execution easy to *represent*. It does not make it free to *govern*. The difficult questions are:

- what inputs are legal to depend on,
- how termination is proven,
- how staged results are cached,
- how code-size growth is bounded,
- and when the compiler should residualize instead of specializing.

That is why compile-time execution affects so many parts of the design: runtime modeling, host boundaries, grade algebra, diagnostics, backend caches, incremental compilation, and domain libraries all need a coherent stage story.

#### 14.4.6 Stageability is a phase judgment, not just a grade

`CompileTimeGrade` is useful, but it measures *cost once a computation has already been admitted into the compile-time regime*. It does not by itself prove that a node is legal to run at compile time.

Legality comes from a separate judgment:

```text
Γbuild ; Γrun ⊢ e : τ ▷ Phase

Phase ::= Static | Residual | Dynamic
```

Operationally:
- `Static` means the node can be executed now over `BuildRuntime`;
- `Dynamic` means it depends on live runtime context;
- `Residual` means a mixed graph where the static prefix can be executed now and the dynamic remainder must be left in the residual program.

#### 14.4.7 A practical stageability algorithm

1. Build typed HIR and CGIR as usual.
2. Mark leaves as `Static` if they are literals, pure builtins, or reads from `BuildRuntime` regions.
3. Mark leaves as `Dynamic` if they read `Runtime` or live host regions, or invoke opaque host operations.
4. Propagate phase bottom-up:
   - `Compose` is static if both children are static and the operator is stage-admissible;
   - `Product` is static if both children are static and alias-safe in build space;
   - `Traversal` is static only when its extent is compile-time known or explicitly bounded;
   - `Coinductive` is dynamic by default, except for bounded unfolding used solely to construct static tables.
5. Extract maximal static subgraphs.
6. Evaluate them in the compiler's graph interpreter over `BuildRuntime`.
7. Replace the result with a literal, an embedded artifact handle, or a specialized monomorphic CGIR node.
8. Record a specialization key from source hash, static inputs, and target profile.

This is compile-time evaluation without macro expansion.

#### 14.4.8 Compile-time leverage by domain

| Domain | High-value compile-time targets | Why they belong in the build phase |
|---|---|---|
| kernels | syscall tables, interrupt descriptors, page layout constants, protocol tables, decoded device trees | removes branchy setup work from privileged runtime paths |
| browsers | parser tables, CSS selector automata, route manifests, layout rule tables, shader/material pipelines | shifts structural work out of frame-time and interaction-time loops |
| databases | query plans, codecs, schema-derived layouts, index operator tables | reduces per-query planning overhead and improves artifact cacheability |
| games | archetype masks, system schedules, animation state graphs, asset conversion products | keeps the frame loop close to bulk-data traversal |
| compilers | grammar tables, rewrite sets, register classes, instruction selectors, backend legality tables | compilation itself becomes ordinary graph execution over compiler-owned IR costates |

#### 14.4.9 Comparison to familiar compile-time tools

Readers from other language backgrounds often ask whether this is merely another name for compile-time evaluation they already know. The family resemblance is real, but the boundary is different:

- compared with **C++ templates**, Optic works over typed graph nodes and explicit regions instead of syntax instantiation alone;
- compared with **Rust macros**, Optic specializes semantics and graph structure rather than rewriting token streams;
- compared with **Java annotation processors**, Optic keeps the transformation in the same optimizer-facing language rather than in a side-channel;
- compared with **Python metaprogramming**, Optic insists on reproducibility, boundedness, and explicit build inputs;
- compared with **TypeScript build transforms**, Optic retains typed structure after specialization because the graph is the language's optimizer form.

#### 14.4.10 What this means for self-hosting

Once the compiler itself is written in the language, a large part of the compiler becomes ordinary compile-time Optic code:

- parser-table generation,
- name-resolution tables,
- peephole rewrite sets,
- data-layout decisions,
- instruction-selection tables,
- optimization over AST/HIR/CGIR costates,
- and specialization of backend patterns.

That is one of the strongest long-range reasons to avoid a separate macro language. The compiler can explain its own compile-time work in the same semantic vocabulary it gives to user programs.

### 14.5 Replay and observability as graph nodes, not ad hoc logging

#### 14.5.1 Reserved observer node shapes

```text
Tap    { inner, flags, formatter }
Record { inner, mode }
Stage  { inner }
```

Modeling these explicitly keeps the semantic pipeline pure while still giving the compiler a place to insert profiling, tracing, or replay recording. It also makes compile-time erasure straightforward when those facilities are disabled.

One of the strongest long-range claims of the language is that debugging, replay, profiling, and tracing become natural once the whole program is an explicit optic graph over explicit costates.

That claim can only be made honestly if observability is modeled as its own graph structure rather than as arbitrary side effects hidden inside ordinary optics. The right design is therefore to reserve dedicated observer or record nodes in the IR.

That design also depends on a sharper distinction than the language has had to state explicitly before: semantic nondeterminism, memory-order visibility, and microarchitectural variability are not one problem. Clocks, RNG, input order, and hidden callbacks affect replay and staging. Atomics, volatility, MMIO, DMA, and interrupt-visible writes affect legality and barriers. Branch prediction, out-of-order execution, and cache behavior affect profitability rather than source semantics. Chapter 16 returns to that distinction directly under the heading **Determinism, Ordering, and Speculation**.

This preserves the law that the semantic pipeline can still be optimized as a dataflow graph, while observer nodes can be erased, sampled, or recorded in later phases.

### 14.6 Why general resumptions are still separate

The language can reserve architectural space for control-runtime modeling without pretending that general continuation capture is already a solved part of the optic core. This is an important boundary.

Resumptions capture delimited control context, not just focused data context. That makes them shape-similar to zippers or branch runtimes, but not reducible to ordinary lens or traversal machinery. The language should therefore treat rich handler-style resumptions as an optional later layer, not as the default semantics of ordinary data transformations.

### 14.7 Transition

Once time, replay, and staging are in view, the next question is whether the grade system scales beyond the prelude. The answer is yes, but only if its dimensions remain honest about what they model.

### 14.8 Detailed implementation reference: coinduction as event-loop structure

The narrative chapter gave the big picture. The following material records the stronger mathematical and operational story: greatest fixed points, unfolding, `drive`, completion rings, and liveness as a grade that constrains composition.

#### 14.8.1 Why induction is wrong for systems code

Inductive types are defined by construction and must be finite. A web server does not terminate; a game loop does not terminate; an OS scheduler does not terminate. Forcing infinite behavior into inductive types produces ugly workarounds (explicit `loop {}`, `Stream` traits with manual wakeup, `tokio::spawn` with opaque futures). The optic model instead models infinite behavior as **coinduction**: a value defined by observation, not by construction. The `corecursive` combinator introduces a coinductive optic that produces observations indefinitely without committing to a finite structure.

#### 14.8.2 The mathematical structure

Let `nu X. F(X)` be the greatest fixed point of a functor `F`. A coinductive optic for a stream of packets is:

```text
StreamOptic<S, A> = nu X. GradedCoState<G>(S, A) × X
```

This reads: "I am a potentially infinite sequence of (costate, observation) pairs." Each step is a graded observation; the tail is another stream of the same type. Unfolding produces one observation; the rest is deferred.

The `unfold`/`drive` pair in the surface syntax corresponds exactly to corecursion introduction and elimination:

```text
sessions.query(PacketPipeline).coinductive().drive()
-- unfolds to: GradedCoState<G>(S, Packet) × StreamOptic<S, Packet>
-- drive() eliminates the coinductive value by running it until it terminates externally
```

#### 14.8.3 The machine lowering: io_uring completion ring

An `io_uring` completion ring is a stateful circular buffer of completed I/O events. The kernel appends completions; the process reads them. This is exactly a coinductive costate: the ring is the `S`, each completion event is the `A`, and the observation is "read the next completion without copying the buffer."

The compiler lowers `.coinductive().drive()` to:

```text
-- io_uring setup (once, outside the loop):
let ring = io_uring::IoUring::new(RING_SIZE);
let costate_optic = TcpSession { ring, ... };

-- the coinductive hot loop:
loop {
    ring.submit_and_wait(1);                      // block until next completion
    let cqe = ring.completion().next();           // one observation
    let packet = PacketPipeline.get(&costate_optic, cqe);  // optic get
    let response = process(packet);               // map_fn
    PacketPipeline.put(&mut costate_optic, response);      // optic put
    ring.advance_cq(1);                           // advance the completion ring pointer
}
```

The zero-copy guarantee is maintained: the optic's `get` body is constrained to reference the ring buffer directly (not copy into a new allocation). The grade enforces this: a `CopyGrade` dimension (full language) flags any optic body that allocates in its `get`.

#### 14.8.4 The coinductive grade: `LivenessGrade`

A coinductive loop that runs forever is `LivenessGrade<Always>`. One that terminates on an external signal is `LivenessGrade<OnSignal>`. One that terminates after at most `N` iterations is `LivenessGrade<Bounded(N)>`. The grade algebra uses `max` for sequential composition (the loop is as live as its most aggressive component) and `min` for conditional loops (the whole system terminates when any component terminates):

```text
combine_seq(LivenessGrade<Always>, LivenessGrade<Bounded(N)>) = LivenessGrade<Always>
-- always-running stage followed by bounded stage is still always-running
```

This catches a common systems bug: a bounded component incorrectly composed with an unbounded one to produce a bounded result. The grade checker rejects the composition and explains why.

---

#### 14.8.5 Experimental note: guarded recursion should remain a proof-facing lane first

The current coinduction and staging design is already compatible with guarded-recursion models such as the topos-of-trees line of work. That is valuable because it gives the language a serious research path for proving productivity, clock discipline, and mixed static/dynamic recursion without replacing the ordinary surface calculus. The practical design rule should stay conservative: reserve structures such as `GuardedClock`, `Later`, and `GuardedRecursionWitness` under `std.experimental.topos`, use them first in proof artifacts and validation tools, and only promote any of that machinery into the core surface if it earns a clear summary form, legality rule, and backend payoff.

### 14.9 Detailed implementation reference: staging, compile-time execution, and monomorphic hot paths

The narrative chapter made the policy claim. This supplement records the operational one: compile time is not a side-channel macro evaluator, but ordinary graph evaluation over a compiler-owned costate.

#### 14.9.1 Two-level type theory, but with explicit costates

Staged compilation is the practice of evaluating part of a program at compile time and deferring the rest to runtime. The theoretical foundation is still **two-level type theory** (2LTT): the language distinguishes `static` (known now) from `dynamic` (known later), and only certain flows from static into dynamic are permitted.

The important adaptation in Optic is that the static side is not abstract. It is represented as explicit costate:

```text
Project = ProjectGraph × RuntimeBlueprintSet
BuildRuntime = Project.build = CompilerGraph × TargetProfile × BuildHostContext × ArtifactCache
ProjectGraph = SourceGraph × PackageGraph × ModuleGraph × CompilerGraph × ArtifactIndex × ProjectionTable
```

That means a compile-time computation is just an optic or optic graph whose required regions all live under `BuildRuntime`.

#### 14.9.2 BuildRuntime and BuildHostContext

A practical build-time root should look like:

```text
BuildHostContext = {
  files:        BuildFiles,
  env:          BuildEnv,
  toolchain:    ToolchainInfo,
  target_caps:  TargetProfile,
  artifacts:    ArtifactCache,
}
```

The compiler may permit reads from these regions only when it can account for them in reproducibility and caching. That implies:

- file reads must be content-hashed,
- environment reads must be whitelisted and hashed,
- target-profile reads must become part of the specialization key,
- writes must target compiler-owned artifacts rather than arbitrary host state.

Any attempt to bypass these channels should produce a staging diagnostic rather than silently collapsing into build-script behavior.

#### 14.9.3 What can and cannot execute at compile time

| Category | Typical examples | Why |
|---|---|---|
| always good static candidates | parser tables, DFA construction, route tries, query plans, ECS archetype masks, register classes | structure is known early and runtime savings are persistent |
| static if inputs are declared | file embedding, shader reflection, asset baking, device-tree decoding, code-size-sensitive unrolling | admissible when the build inputs are explicit and cacheable |
| usually dynamic | live sockets, live clocks, user input, runtime-sized traversals over live state, coinductive event loops | these depend on runtime-only host channels or unbounded live behavior |
| static only by bounded evaluation | partial unrolling, bounded search, small compile-time data transforms | needs an explicit bound or proven termination |

#### 14.9.4 Stageability judgment

Compile-time eligibility is not the same thing as compile-time cost. The implementation should carry a separate phase judgment:

```text
Γbuild ; Γrun ⊢ e : τ ▷ Phase

Phase ::= Static | Residual | Dynamic
```

Basic rules:

- literals, type metadata, and pure builtins are `Static`;
- reads from `BuildRuntime` are `Static` when the region is reproducible and hashable;
- reads from live `Runtime` or opaque host services are `Dynamic`;
- `Compose(a,b)` is `Static` only when both children are static and the operator is stage-admissible;
- `Product(a,b)` is `Static` only when both children are static and their build-time writes are non-conflicting;
- `Traversal` is `Static` only when the extent is compile-time known or a bounded compile-time iterator;
- `Coinductive` is `Dynamic` by default, except for bounded unfolding used to synthesize finite static data.

#### 14.9.5 Stage-admissible operators and forbidden ones

Stage-admissible operations in the full language should include:

- pure arithmetic and structural transforms,
- optic composition over static roots,
- bounded traversals over static collections,
- graph rewrites over compiler IR costates,
- artifact emission into compiler-owned caches,
- target-profile-conditioned specialization.

Operations that should be forbidden or explicitly wrapped at compile time include:

- unbounded or undecidable recursion,
- live time, random, or network access,
- mutation of non-artifact host state,
- use of runtime-only linear capabilities,
- specialization whose cache key would be incomplete or unrepresentable.

#### 14.9.6 Residualization algorithm

A practical residualization pass can be defined over CGIR:

```text
1. build typed HIR and CGIR normally
2. compute Phase for every node bottom-up
3. carve out maximal Static subgraphs
4. evaluate those subgraphs in the compiler interpreter over BuildRuntime
5. replace each static subgraph with:
   - a literal/constant blob,
   - an embedded artifact handle,
   - or a specialized monomorphic CGIR node
6. leave Dynamic nodes untouched
7. leave Residual nodes as mixed graphs with explicit static inputs materialized
```

This algorithm is what makes compile-time execution feel like ordinary evaluation rather than macro expansion. The compiler is not rewriting syntax. It is reducing graph fragments whose values are already known.

#### 14.9.7 Specialization cache keys and artifact emission

Because compile-time execution is graph evaluation, its cache key should be graph-shaped as well:

```text
SpecializationKey = hash(
  source_subgraph,
  static_inputs,
  target_profile,
  relevant_build_env,
  language_version,
)
```

A staged result may materialize as:

1. a constant embedded in readonly data,
2. a generated but still typed artifact under `ArtifactCache`,
3. a monomorphic CGIR node ready for backend lowering,
4. or a cached specialization reused across incremental builds.

This makes staging part of incremental compilation, not a bolt-on feature.

#### 14.9.8 Why `CompileTimeGrade` is a budget, not the proof of stageability

`CompileTimeGrade` should track build-time work once a node is already static. It is useful for:

- capping expensive compile-time rewrites,
- surfacing build-time hotspots,
- choosing whether specialization is profitable,
- and explaining why a candidate static graph was intentionally residualized.

But it is not the same thing as the phase judgment. A computation can have a tiny compile-time cost and still be illegal to execute at compile time if it reads live host state. Conversely, a legal static computation may be residualized because its compile-time cost or code-size impact is too high.

A reasonable full-language carrier is:

```text
CompileTimeGrade<work, code_size_delta>
```

with conservative additive composition, while phase legality remains separate.

#### 14.9.9 Why staging matters for systems code

The common high-performance pattern is still "setup once, run many":

- database plans are optimized once and executed many times,
- ECS archetype and schedule structure is fixed long before the frame loop,
- browser selector automatons and route tables are compiled before interaction,
- compiler optimization tables and instruction selectors are built before code emission,
- kernel tables and protocol decode maps are fixed before the hot path begins.

The language should make these cases cheap without forcing the programmer into macros, code generators, or out-of-band build scripts.

#### 14.9.10 The machine consequence: monomorphic native code and precomputed artifacts

A static optic subgraph may lower to one of three concrete backend consequences:

- **embedded data**: lookup tables, DFAs, route tries, precomputed masks, serialized blobs;
- **specialized code**: monomorphic loops or branches with structure selection erased;
- **artifact-cache handles**: prebuilt intermediate forms reused across builds or runs.

The backend therefore sees a smaller and more monomorphic graph. That reduces dynamic dispatch, removes structural branching from hot paths, and gives LLVM or the Rust backend clearer code to optimize.

#### 14.9.11 Comparison to familiar compile-time tools

| Tooling tradition | Typical unit of compile-time work | Limitation that Optic addresses |
|---|---|---|
| C++ templates / `constexpr` | syntax instantiation plus constant evaluation | no single optimizer-facing graph artifact that unifies staged dataflow with runtime dataflow |
| Rust macros / `build.rs` / const eval | token rewriting, build scripting, and const evaluation split across mechanisms | compile time is fragmented across several subsystems with different visibility to the optimizer |
| Java annotation processing | class and bytecode generation outside the main optimizer story | generated structure is usually not expressed in one typed graph that can be graded and residualized |
| Python metaprogramming | import-time execution and library-specific generators | powerful but difficult to bound, cache, and reproduce rigorously |
| TypeScript transforms | build-step rewriting before runtime | erased types mean the compiler does not keep the transformed structure as typed optimizer evidence |

Optic tries to keep compile-time work in one semantic pipeline so that diagnostics, caching, optimization, and self-hosting all speak the same language.

#### 14.9.12 AI-friendly staging diagnostics

Compile-time execution needs its own structured diagnostics, especially for agent workflows. A good initial reserved family is `STG-*`:

| Code | Meaning | Preferred first action |
|---|---|---|
| `STG-101` | candidate static graph reads a runtime-only region | move the read behind a residual boundary or route it through `BuildRuntime` |
| `STG-111` | compile-time host access is not reproducible or not whitelisted | declare the input explicitly in `BuildHostContext` |
| `STG-121` | compile-time recursion or traversal lacks a proven bound | add a bound or residualize the computation |
| `STG-131` | staged result would exceed code-size or artifact-size budget | keep structure staged but residualize data |
| `STG-141` | compile-time work exceeds declared `CompileTimeGrade` budget | split specialization, cache earlier, or relax the budget |

These diagnostics matter because the likely failure mode of a macro-free stage system is not "cannot parse" but "cannot prove this should be static".

#### 14.9.13 Native build, package, and module language

Once compile-time evaluation is ordinary graph evaluation over `BuildRuntime`, the strongest default is that project configuration, package declarations, build plans, feature selection, target matrices, module exports, generated bindings, and artifact policies should all be expressed in the language itself. The language should not require a second handwritten TOML, YAML, JSON, XML, or CMake-like surface merely to describe the graph the compiler already knows how to evaluate.

The important claim is not simply "configuration as code." Many ecosystems already have that in one form or another. The stronger claim is that build structure should be **ordinary typed optic-friendly code over explicit build costate**. That means the same concepts already used elsewhere in the book — `BuildRuntime`, stageability, `CompileTimeGrade`, `TargetProfile`, `BoundaryContract`, `OpticSummary`, provenance, and artifact hashes — should remain the semantic center.

In practical terms, the authoring surface can stay extremely small. A package or workspace can just be a well-known exported value in an ordinary module:

```rust
pub let package = Package {
    name: "engine.renderer",
    edition: Edition::current(),
    targets: [
        Target::linux_x86_64_musl(),
        Target::windows_x86_64_msvc(),
    ],
    deps: [
        Dependency::registry("math.vec").exact("1.2.0"),
        Dependency::path("../gpu_core"),
    ],
    ffi: [SystemLib::new("vulkan")],
    experimental: [],
    capabilities: [
        BuildCap::read_path("assets/shaders"),
        BuildCap::env("SDKROOT"),
    ],
};

pub let workspace = Workspace {
    members: ["compiler", "runtime", "renderer"],
    default_target: Target::linux_x86_64_musl(),
};
```

Nothing about this requires macro syntax or a second meta-language. These are ordinary values in ordinary modules. The compiler's job is to look for the conventional exported build roots, evaluate them in `BuildRuntime`, and derive `PackageGraph`, target selection, dependency closure, and artifact policy from there.

#### 14.9.14 Build planning as staged optic code

The same logic extends from static declarations to dynamic build planning. A build plan is just another staged computation over `BuildRuntime`:

```rust
pub fn build_plan(rt: &[SharedGrade] BuildRuntime) -> BuildPlan {
    rt.query(
        LoadWorkspace
        >>> ResolveDependencies
        >>> CheckTargetProfile
        >>> ExpandGeneratedBindings
        >>> PlanArtifacts
    ).get()
}
```

This is a much better fit for the language than a separate build-script model for three reasons.

First, the build graph becomes visible to the same optimizer and provenance machinery that already understands ordinary optic graphs. Second, build failures can use the same structured diagnostics model rather than degenerating into shell-script stderr. Third, compile-time work becomes budgetable: `CompileTimeGrade` can describe how expensive dependency resolution, code generation, shader baking, or interface summarization is expected to be.

The language should still distinguish between **declarative build roots** and **imperative build planning**, but that distinction can be made with types and phases rather than with separate syntactic worlds. A `package` value is just data. A `build_plan` optic graph is just staged code. Both live under `BuildRuntime`.

#### 14.9.15 Graph-first build state, projections, and materialization

Native build declarations become much more powerful when the compiler graph is treated as the source of truth rather than as a cache derived from ordinary files after the fact.

The cleanest long-range picture is:

- native `package`, `workspace`, and `build_plan` declarations are authored as ordinary language values,
- the compiler evaluates them into regions of the `CompilerGraph`,
- generated lock snapshots, module interfaces, build plans, and artifact manifests are compiler-owned projections of that graph,
- and ordinary files are one compatibility surface for humans and legacy tools rather than the only semantic home of the build.

This is not an argument against text. It is an argument for **authoritative structure first, materialization second**. Text remains crucial because humans read, diff, review, and version it well. But if text is the only truth, then every other tool has to rediscover the graph the compiler already knows.

Once the graph becomes authoritative, several hard problems simplify at once.

1. **Incremental compilation** becomes graph invalidation instead of cache-directory archaeology.
2. **Native build declarations** become ordinary build-time regions rather than one more manifest parser.
3. **Module interfaces** become stable serialized summaries of graph regions.
4. **Generated artifacts** become materializations of graph state rather than magical side effects.
5. **Tool interoperability** can use either direct protocol access or file-like projections without splitting semantics.

The practical rule should be:

> authored text is a first-class writable projection of the compiler graph; generated files are materialized projections; internal summaries remain graph-native.

That rule preserves both human workflows and structural truth.

#### 14.9.16 Native project queries stay inside the language

The previous chapter argued that the `Project` graph should be natively queryable through ordinary optic code rather than through a separate user-facing query language. Staging is where that decision becomes operationally important.

A staged planner, binding generator, migration tool, or optimization audit should not have to leave the language to ask questions such as:

- which nodes write this region,
- which interface artifacts depend on this summary,
- why was this composition not fused,
- which generated artifacts belong to this target profile,
- or which boundary contracts make a candidate static subgraph residual instead.

Those are ordinary semantic questions about `Project.build`, so they should be expressible as ordinary graph-rooted queries over `BuildRuntime`. Internally the compiler may lower them into a richer query engine with indexes and path search, but the source-facing model stays uniform.

Historically, the closest precedents are Datalog and executable attribute grammars. Datalog showed that large semantic structures can be queried declaratively and optimized through indexing and fixed-point evaluation. Attribute grammars showed that meaning can be computed over program structure without collapsing into hand-written compiler plumbing. Optic reuses both lessons but constrains them: user-facing Project queries remain ordinary optic programs over `Project`, while richer QIR, query optimization, and repair synthesis stay internal; compiler phases operate over the full project graph, not only over syntax-tree-local attributes.

This matters well beyond tooling. It means staged package resolution, generated bindings, replay capsule construction, self-hosted compiler audits, and benchmark attribution all reuse the same phase model instead of creating a second configuration or metaprogramming language.

#### 14.9.17 Compiler phases reuse the same substrate, but as transactions

The same graph-query substrate also explains compilation itself. A read-only graph query and a compiler pass begin the same way — by selecting and analyzing graph regions — but they diverge at commit time.

A compile-time phase is therefore best understood as a **graph transaction over `BuildRuntime`**:

```text
query/selection
  -> analysis
  -> rewrite or materialization
  -> invariant check
  -> revision commit
```

That interpretation ties staging directly to several later chapters.

- In **Chapter 16**, the backend becomes one more graph-materialization transaction from CGIR to native artifacts.
- In **Chapter 22**, the self-hosted compiler becomes a library of staged graph transactions over compiler-owned costates.
- In **Appendix J**, the internal query engine, repair synthesis, and distributed scheduler all reuse the same substrate.

The practical design rule is therefore:

> ordinary project queries are read-mostly optic programs; compiler phases are staged graph transactions built from the same optic substrate.

#### 14.9.18 No user-authored config files does not mean no serialized artifacts

Eliminating handwritten external configuration does **not** eliminate the need for stable serialized artifacts. It only changes which side of the boundary they live on.

A mature toolchain still needs machine-readable artifacts for:

- dependency-resolution snapshots,
- module/interface summaries,
- staged-artifact caches,
- registry publication,
- binary-distribution metadata,
- reproducible-build attestations,
- and IDE / CI / remote-cache integration.

The key design rule is therefore:

> **Source of truth is native language code. Serialized artifacts are compiler-generated views of that source of truth.**

That distinction keeps the ecosystem coherent. Users do not hand-maintain separate manifest DSLs and lockfile semantics. The compiler evaluates native declarations, then emits canonical artifacts such as:

- a generated dependency lock snapshot,
- a generated module-interface artifact built from exported summaries,
- a generated staged-artifact manifest,
- and generated registry metadata.

Those files may still be text or binary on disk, but they are outputs of the compiler/toolchain, not author-edited configuration languages that drift away from the semantics.

#### 14.9.19 Why this is better than the usual split

Many mainstream ecosystems split the project description across a language and a separate build/config language. C and C++ often split code from CMake or Meson. Rust splits crate source from `Cargo.toml` and sometimes `build.rs`. JavaScript and TypeScript split code from `package.json` plus bundler configuration. Python splits code from `pyproject.toml` and environment metadata. Those systems can work well, but they create a permanent translation boundary: the compiler only partially owns the structure that decides how code is discovered, specialized, linked, or cached.

Optic can avoid that split because compile-time execution is already part of the core semantics. The build graph does not need to be re-expressed in a weaker language. It can be written directly in the same typed system that already knows about target profiles, staged artifacts, host capabilities, and foreign boundaries.

That gives several concrete advantages:

| Concern | Usual split-language cost | Native Optic build language answer |
|---|---|---|
| package discovery | parser for a separate manifest format | evaluate exported `package` / `workspace` values |
| build logic | shell scripts, build DSLs, or host-language plugins | staged optic graphs over `BuildRuntime` |
| target selection | duplicated between build file and source code | one `TargetProfile` value reused everywhere |
| generated code and bindings | side channels and ad hoc file emission | compiler-owned artifact emission with provenance |
| diagnostics | build-tool errors and compiler errors speak different schemas | one diagnostic model, one provenance story |
| reproducibility | separate lockfile semantics and config hashing rules | generated lock snapshot from explicit build inputs |

#### 14.9.20 What this changes elsewhere in the design

Treating build and packaging as native staged code affects several parts of the architecture.

1. **Module system.** A module is no longer just a namespace of runtime declarations. It can also export build-root values and staged artifact policies. Module interfaces should therefore serialize both semantic exports and build-surface summaries.
2. **Package identity.** Package identity should be computed from native declarations plus generated lock snapshots, not from a handwritten manifest alone.
3. **Dependency resolution.** Fetching, version solving, and native-library discovery become explicit build-time boundary operations. They should read through `BuildHostContext` and carry determinism and capability facts.
4. **Incremental compilation.** Staged package and build values become part of specialization keys, so changing a dependency declaration invalidates exactly the affected build subgraph rather than the whole world.
5. **FFI and generated bindings.** Header import, IDL generation, shader reflection, and binding emission can all be ordinary staged pipelines whose results are compiler-owned artifacts.
6. **Self-hosting.** The self-hosted compiler should use the same native package/workspace/build declarations as ordinary user code. That avoids a "compiler has its own build language" fork from day 0.

#### 14.9.21 Hermetic-by-default build semantics

The strongest default is that staged build evaluation is hermetic unless capabilities are granted explicitly. That means:

- reading a source file is legal because it is already part of `SourceGraph`,
- reading a whitelisted environment variable is legal because it is declared in `BuildHostContext`,
- reading arbitrary host files is illegal unless granted,
- network access for dependency fetching is illegal unless routed through an explicit fetch optic,
- wall-clock reads, random, and ambient process state are illegal in reproducible mode,
- and artifact emission may only target compiler-owned output regions.

This is how the language avoids turning compile-time execution into a fancier `build.rs`. The power comes from reusing the same runtime/boundary model, not from granting the compiler a privileged imperative side channel.

#### 14.9.22 Native example: package declaration, generated bindings, and target-specific build planning

```rust
pub let package = Package {
    name: "net.httpd",
    edition: Edition::current(),
    targets: [Target::linux_x86_64_musl()],
    deps: [Dependency::registry("crypto.tls").exact("1.4.0")],
    ffi: [HeaderImport::new("openssl/ssl.h")],
    capabilities: [BuildCap::read_path("include/"), BuildCap::env("OPENSSL_DIR")],
    experimental: [],
};

pub fn build_plan(rt: &[SharedGrade] BuildRuntime) -> BuildPlan {
    rt.query(
        ResolveDependencies
        >>> GenerateCBindings
        >>> SelectTargetLibraries
        >>> PlanArtifacts
    ).get()
}

pub fn main(rt: &[AffineGrade] Runtime) {
    rt.query(HttpServerPipeline).coinductive().drive();
}
```

This example uses the same language for package description, binding generation, artifact planning, and runtime entry. The important separation is not syntactic. It is phase-based: `package` and `build_plan` live over `BuildRuntime`; `main` lives over `Runtime`.

#### 14.9.23 Solver systems should be optics-native first

The same compile-time model should also govern solver construction. A solver stack is usually a mixture of:

- problem description,
- structural analysis,
- generated kernels or sparsity plans,
- and a runtime stepping or iteration loop.

That is exactly the kind of workflow the current staging model already handles well. A solver should therefore default to **ordinary optics and staged graph evaluation over `BuildRuntime`**, not to a new core DSL. When domain-specific equation notation genuinely improves clarity, it should enter as an imported or embedded frontend that lowers into the same summaries, CGIR nodes, staged artifacts, and diagnostics.

This keeps solver generation consistent with the rest of the language. A staged Jacobian builder, a sparsity-plan compiler, and an imported finite-difference stencil frontend all become ordinary graph work rather than a second compiler hiding inside the first.

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

## 16. LLVM, TBAA, Intrinsics, and Native Backend Strategy

### 16.1 Why the native backend is a second regime, not a quick swap

Moving from a Rust transpiler to a native LLVM backend is not just a matter of replacing one pretty-printer with another. It is the point where the compiler stops relying on Rust as its semantic reference and begins carrying its own backend proof obligations.

That is why the book insists on a staged transition.

1. Rust transpiler as the semantic oracle
2. LLVM emitter validated against the Rust path
3. LLVM as the primary path with Rust retained as an audit target

#### 16.1.1 Comparison to native, managed, interpreted, and transpiled backend cultures

The backend strategy will also be easier to understand if it is contrasted with the dominant compilation cultures of other languages.

| Language | Normal backend culture | What that culture is good at | Why the book chooses a Rust-first then LLVM-native ladder instead |
|---|---|---|---|
| **C++** | direct native lowering with template-driven ahead-of-time specialization | mature native performance and explicit ABI control | Optic wants similar ultimate control, but it first needs a readable semantic oracle because the core risk is not template expressivity; it is preserving optic structure correctly |
| **Java** | bytecode plus JIT and runtime profiling | adaptive optimization and strong portability | many optimizations are intentionally deferred to runtime, whereas Optic wants more legality decisions to be made from static summaries and grades |
| **Python** | interpreter first, optional tracing JITs, native extension escape hatches | superb flexibility and interactive workflows | performance-critical structure is too often moved into foreign libraries; Optic wants the language itself to own the hot-path story |
| **TypeScript** | transpile to JavaScript and rely on the JS engine | excellent ecosystem reach and tooling | the final optimizer sees JavaScript semantics, not the richer static story that TypeScript expressed during checking |
| **Rust** | MIR to LLVM with strong ahead-of-time optimization | closest mainstream analogue to the final target discipline | Optic still inserts a readable Rust output stage because the semantic risk profile of a new language is higher than that of a new crate or MIR pass |

This staged backend plan is not timidity. It is a deliberate method for separating semantic bring-up risk from native-code optimization risk.

#### 16.1.2 Compile-time execution changes what reaches the backend

Once staged subgraphs are admitted into the compile-time regime, the backend no longer sees one uniform stream of runtime work. It sees a mixture of residual runtime code and already-computed structure. In practice, a successful specialization pass can remove or shrink:

- runtime graph selection,
- late binding over route/query/archetype/layout structure,
- repeated plan construction,
- and branches whose result is already known from `BuildRuntime`.

The backend therefore needs three stable target shapes for staged results:

1. **embedded readonly data** such as tables, automatons, masks, and constant plans;
2. **specialized monomorphic functions** where structural choices are erased;
3. **artifact-cache references** for large derived assets that should not be rederived or embedded repeatedly.

This affects validation too. A native backend must be checked not only for runtime correctness, but also for whether staged results were residualized, embedded, or cached in the intended way.

### 16.2 TBAA and why RegionSet is not just for diagnostics

#### 16.2.1 Per-field alias metadata sketch

```text
generate_tbaa(all_fields):
  root = tbaa_root('optic')
  for data_type in all_fields.group_by_costate():
    parent = tbaa_scalar(data_type.name, root)
    for field in data_type.fields:
      emit_field_tbaa(parent, field.path, field.offset, field.size)
```

The same proof object that lets `***` compile safely is therefore also the object that tells LLVM which loads and stores may be treated as distinct.

One of the most important links between the middle end and LLVM is type-based alias analysis metadata. The compiler's region summaries are exactly the information needed to construct useful per-field alias trees.

That means the same structural proofs used for `***` legality also become optimizer fuel in the native backend. This is a good example of why the language carries explicit region structure rather than flattening into opaque references early.

### 16.3 Node-to-IR lowering discipline

#### 16.3.1 Representative IR shape

```llvm
define float @HealthView_get(%Entities* %arena, i64 %id) {
entry:
  %healths_ptr = getelementptr inbounds %Entities, %Entities* %arena, i32 0, i32 0
  %healths_data = load float*, float** %healths_ptr
  %elem_ptr = getelementptr inbounds float, float* %healths_data, i64 %id
  %val = load float, float* %elem_ptr, !tbaa !health_tbaa
  ret float %val
}
```

Again, the purpose of showing IR is to make the bridge explicit: the `Cursor<S>` and `RegionSet` story really does become address calculation plus alias metadata.

The backend strategy should remain node-oriented.

- `OpticLeaf` lowers to a small load/modify/store fragment,
- `Compose` lowers after fusion or becomes nested fragments,
- `Product` lowers to shared-loop fragments when alias-safe,
- traversals lower to loop nests or SIMD kernels,
- coinductive nodes lower to structured event loops,
- staged nodes lower to already-specialized concrete IR.

The same principle applies as earlier: preserve structure until a specific backend form has earned the right to erase it.

#### 16.3.2 Debug metadata, crash packets, and profiler continuity are backend obligations too

A native backend that preserves correctness but loses semantic identity has still broken one of the book's central promises. After fusion and staging, the emitted machine program must remain explainable in the language's own terms.

The backend should therefore maintain an explicit lowering chain:

```text
backend code range
  -> fused-loop provenance
  -> CGIR nodes
  -> HIR items
  -> source spans
```

On DWARF-like targets that means using inline scopes, discriminators, and side tables aggressively enough that a fused loop can still be attributed back to several optic names rather than flattened into one anonymous line record. On other targets the same logical mapping still has to exist even if the container format differs.

The same requirement applies to crash reporting. A mature backend should be able to package a crash capsule containing at least:

- toolchain id and edition,
- target profile,
- module-interface hashes,
- fused-node provenance,
- nearby boundary contracts,
- and any staged artifact ids that materially shaped the emitted code.

This is what makes later crash reproduction, profiler attribution, and translation validation feasible across optimized backends.

### 16.4 Intrinsics belong behind grade and capability checks

AVX, AES-NI, io_uring, DMA helpers, and similar facilities are powerful precisely because they are not generic. The backend should therefore activate them only when both the optic semantics and the target profile say they are legal.

This is another reason the grade system matters. It gives the compiler a typed, auditable way to turn high-level declarations into target-specific code without pretending that portability and specialization are the same thing.

#### 16.4.1 Foreign and unsafe lowering should reuse `OpticLeaf`, not invent a second IR

A new systems language is always tempted to split in two the moment it learns about FFI or raw hardware.

- one IR for "real" language constructs,
- a second escape IR for extern calls, volatile memory, inline assembly, or callback bridges.

That split is attractive because it seems practical. In fact it is one of the fastest ways to make the language impossible to reason about globally. The optimizer stops seeing the whole graph, provenance becomes fragmented, and diagnostics lose the thread that connects source intentions to backend constraints.

The current model already has a better place to put this information: `OpticLeaf` plus `OpticSummary` plus the boundary contract introduced earlier. In other words, the backend should treat foreign and unsafe work as **annotated leaves in the same graph**, not as a separate optimizer-unfriendly world.

The lowering distinction is then a property of the leaf implementation kind rather than a property of the surrounding calculus.

| Leaf kind | Typical source form | Backend consequence |
|---|---|---|
| `Local` | ordinary optic body | normal load/store or call lowering |
| `Extern` | ABI-stable foreign call | calling convention, symbol linkage, unwind attributes, alias barriers as required |
| `Intrinsic` | target intrinsic wrapper | target-specific LLVM intrinsic or builtin call |
| `Volatile` | MMIO or special memory access | volatile load/store, no illicit reordering |
| `Asm` | inline assembly wrapper | explicit clobbers, side-effect flags, fence semantics |

This is the minimal-extension rule in backend form. The graph does not change shape just because one leaf is foreign. The leaf acquires a different lowering recipe and a stronger safety contract.

That reuse also explains how legacy interop stays compatible with the optimizer. TBAA, region sets, and ownership proofs remain valid up to the boundary and resume after it. The backend only has to insert the right barriers or attributes at the leaf itself.

### 16.5 Portability without lowest-common-denominator semantics

A systems language does not gain credibility by pretending every architecture and OS behaves the same. It gains credibility by making the target profile explicit and by lowering accordingly.

The design therefore treats target capabilities as structured inputs to backend choice, not as a reason to weaken the semantic model.


### 16.6 Determinism, Ordering, and Speculation

The language should be machine-aware without pretending that microarchitectural behavior becomes source semantics. That requires a clean separation among three different phenomena that are often collapsed under the word “nondeterminism”.

#### 16.6.1 Three different sources of variation

**Semantic nondeterminism** comes from clocks, RNG, packet arrival order, hidden global state, callbacks, and unsummarized foreign boundaries. This is the form that replay, staging, and deterministic testing care about most directly, and it is exactly why `OpticSummary` already carries determinism classes such as `Pure`, `Seeded`, `Recorded`, and `Opaque`.

**Memory-order and visibility variation** comes from atomics, volatile/MMIO access, DMA-visible buffers, interrupt-context writes, and callback-visible shared state. These do not usually change the meaning of an expression-level calculation, but they absolutely change what reorderings are legal and which barriers must be emitted.

**Microarchitectural variability** comes from branch prediction, out-of-order execution, cache hierarchy behavior, SIMD width, prefetch, and target-specific execution resources. This is the level at which the language should drive profitability and target-aware lowering rather than claim exact source-level determinism.

The compiler only stays honest if it tracks these three classes separately.

#### 16.6.2 Out-of-order execution is a legality question, not a semantic model

The language should not attempt to simulate reorder buffers or predictor tables in its core semantics. It only needs to make the **legality of reordering** explicit enough that the backend can exploit out-of-order hardware without changing meaning.

The current summary model already points to the right design:

- `get_reads`, `put_reads`, and `put_writes` expose true read-for-update hazards;
- `BoundaryContract` distinguishes ordinary, atomic, volatile, MMIO, DMA, and managed regions;
- address-space-aware regions tell the backend which operations may be speculated, hoisted, duplicated, or vectorized and which may not.

That means ordinary RAM traversals can still be aggressively reordered when alias proofs allow it, while MMIO, volatile, and callback-visible paths remain order-sensitive without requiring a second semantics.

#### 16.6.3 Branch prediction belongs to prisms and traversals

Prisms are already the branch form of the language, and traversals are already the vectorization form. A branch-sensitive backend therefore has a natural place to attach microarchitectural guidance without contaminating source semantics:

- prisms may carry `BranchBias<Likely|Unlikely|Neutral>`;
- traversal+prism combinations may choose among ordinary branching, mask lowering, compact-then-traverse, or split-phase filtering;
- branch metadata remains a profitability hint, not an observable semantic change.

This is the same pattern used throughout the book: static structure stays explicit long enough that the backend can make a target-aware decision with less guesswork.

#### 16.6.4 Order-sensitive versus speculation-safe regions

The backend benefits from one additional derived distinction.

- **speculation-safe** regions are ordinary RAM reads and writes whose reorderability is already captured by regions, grades, and alias proofs;
- **order-sensitive** regions are volatile/MMIO, atomic, DMA, callback-visible, or otherwise host-observable regions that constrain speculation and motion.

This distinction does not need a new surface-language feature. It can be derived from `OpticSummary`, `BoundaryContract`, `AddressSpace`, and memory-order annotations. But once derived, it gives the backend and tooling a direct answer to questions such as:

- may this load be hoisted?
- may this branch be if-converted?
- may this loop be vectorized?
- does this path require a fence or barrier?

That keeps the optimizer from treating every effectful edge as equally hostile while still respecting the ones that truly are.

#### 16.6.5 Replay, validation, and performance identity complete the story

A nondeterminism-aware language also needs operational evidence. The same distinction among semantic nondeterminism, memory visibility, and microarchitectural variability should flow into:

- replay classification (`Pure`, `Seeded`, `Recorded`, `Opaque`);
- stageability analysis over `BuildRuntime`;
- translation validation between Rust and LLVM backends;
- semantic `PerfKey`s that track optimized code by provenance rather than by raw symbol names.

That is what lets the language say, with a straight face, that it is machine-aware without making the source language itself depend on the exact behavior of one branch predictor or one cache hierarchy.

### 16.7 Transition

Once the native backend exists, the next pressure appears immediately: multicore execution, NUMA, memory layout selection, and safe reclamation. Those are not separate side quests. They are the next obvious machine consequences of the language's explicit resource model.

### 16.8 Detailed implementation reference: LLVM lowering, TBAA construction, and target intrinsics

The narrative chapter explains why the LLVM backend is a second regime rather than a direct replacement for the Rust path. The material below preserves the concrete IR shapes, per-field TBAA story, and intrinsic-lowering rules that make the native backend auditable.

### 16.9 Why the Rust Transpiler Comes First

The v0 Rust backend is not a stepping stone to be discarded. It is the formal specification of the language's code shape in a human-readable and independently verifiable form. The LLVM backend must produce code that is *equivalent* to the Rust transpiler's output, not merely *similar* to it. The Rust-to-LLVM path through `rustc` serves as the reference implementation for the first year of the LLVM backend's existence.

The transition from Rust transpiler to LLVM backend happens in three phases:

**Phase 1 (current): Rust transpiler**. The compiler emits Rust source. The Rust source is compiled by `rustc`. The developer inspects the generated Rust to audit code shape. This is the phase where bugs in the language semantics are cheapest to find.

**Phase 2: LLVM IR emitter, validated against Rust output**. The compiler emits LLVM IR in addition to Rust. A differential test suite checks that the LLVM IR, when compiled, produces identical behavior on the benchmark suite. The Rust path remains the golden reference. Phase 2 begins after M6.

**Phase 3: LLVM IR as primary output, Rust as optional audit trail**. The Rust transpiler becomes an `--emit-rust` flag for human-readable debugging. The LLVM backend handles all production builds. Phase 3 begins after Phase 2 has passed the benchmark suite at parity.

### 16.10 The LLVM IR Shape

For every CGIR node kind, the LLVM IR shape is specified here. These shapes are the normative target for the Phase 2 emitter.

#### 16.9.1 Leaf optic get/put

```llvm
; optic: HealthView (get)
define float @HealthView_get(%Entities* %arena, i64 %id) #0 {
entry:
  ; arena->healths is a Vec<f32>; ptr is stored at offset 0
  %healths_ptr = getelementptr inbounds %Entities, %Entities* %arena, i32 0, i32 0
  %healths_data = load float*, float** %healths_ptr, align 8
  %elem_ptr = getelementptr inbounds float, float* %healths_data, i64 %id
  %val = load float, float* %elem_ptr, align 4, !tbaa !health_tbaa
  ret float %val
}

; optic: HealthView (put)
define void @HealthView_put(%Entities* %arena, i64 %id, float %new_val) #0 {
entry:
  %healths_ptr = getelementptr inbounds %Entities, %Entities* %arena, i32 0, i32 0
  %healths_data = load float*, float** %healths_ptr, align 8
  %elem_ptr = getelementptr inbounds float, float* %healths_data, i64 %id
  store float %new_val, float* %elem_ptr, align 4, !tbaa !health_tbaa
  ret void
}
```

The `!tbaa` metadata is critical. It tells LLVM that `entities.healths` and `entities.positions` are non-overlapping memory regions, enabling load/store reordering and vectorization across field boundaries. The TBAA (Type-Based Alias Analysis) metadata is generated from the region summary in the `OpticSummary`: each SoA field gets its own TBAA tag derived from its field path, ensuring that the compiler's alias proofs are communicated to LLVM in the form it understands.

#### 16.9.2 Fused product loop

```llvm
; optic(fused): [HealthView, PositionView]
define void @fused_health_position_map(%Entities* %arena, i64 %len) #1 {
entry:
  %healths_ptr  = getelementptr inbounds %Entities, %Entities* %arena, i32 0, i32 0
  %positions_ptr= getelementptr inbounds %Entities, %Entities* %arena, i32 0, i32 1
  %h_data = load float*, float** %healths_ptr, align 8
  %p_data = load %Vec2*, %Vec2** %positions_ptr, align 8
  br label %loop

loop:
  %id = phi i64 [ 0, %entry ], [ %id_next, %loop ]
  ; load both fields (one pass, both fields in the loop body)
  %h_ptr = getelementptr inbounds float, float* %h_data, i64 %id
  %h = load float, float* %h_ptr, align 4, !tbaa !health_tbaa
  %p_ptr = getelementptr inbounds %Vec2, %Vec2* %p_data, i64 %id
  %p = load %Vec2, %Vec2* %p_ptr, align 8, !tbaa !position_tbaa
  ; map function inline
  %h_new = call float @update_health(float %h, %Vec2 %p)
  %p_new = call %Vec2 @update_position(float %h, %Vec2 %p)
  ; store both fields
  store float %h_new, float* %h_ptr, align 4, !tbaa !health_tbaa
  store %Vec2 %p_new, %Vec2* %p_ptr, align 8, !tbaa !position_tbaa
  ; advance
  %id_next = add nuw i64 %id, 1
  %done = icmp eq i64 %id_next, %len
  br i1 %done, label %exit, label %loop, !llvm.loop !loop_hint

exit:
  ret void
}
```

The `!llvm.loop` metadata carries vectorization hints derived from the grade:
- If `SimdEligible` is set: `!{ !"llvm.loop.vectorize.enable", i1 true }`
- If `CacheGrade<n>` implies interleaving: `!{ !"llvm.loop.interleave.count", i32 N }`

#### 16.9.3 SIMD traversal path (AVX2 example)

```llvm
; optic(simd): AllHealths
define void @AllHealths_simd_map(%Entities* %arena, i64 %len, float %damage) #2 {
entry:
  %h_ptr = ... (same as above)
  %vec8_damage = insertelement <8 x float> undef, float %damage, i32 0
  %damage_splat = shufflevector <8 x float> %vec8_damage, <8 x float> undef,
                                <8 x i32> zeroinitializer
  %aligned_len = and i64 %len, -8                  ; round down to multiple of 8
  br label %simd_loop

simd_loop:
  %i = phi i64 [ 0, %entry ], [ %i_next, %simd_loop ]
  %base = getelementptr inbounds float, float* %h_ptr, i64 %i
  %v = load <8 x float>, <8 x float>* %base, align 4
  %v_new = fsub <8 x float> %v, %damage_splat
  store <8 x float> %v_new, <8 x float>* %base, align 4
  %i_next = add nuw i64 %i, 8
  %done = icmp eq i64 %i_next, %aligned_len
  br i1 %done, label %scalar_tail, label %simd_loop

scalar_tail:
  ; handle remainder elements (0–7) with scalar loop
  ...
}
```

### 16.11 TBAA Metadata Generation from OpticSummary

The `RegionSet` in every `OpticSummary` is the direct source for TBAA metadata. Each field path in the region set becomes a distinct TBAA node in the LLVM module. The generation algorithm:

```text
generate_tbaa(summary_table):
  tbaa_root = TBAA::root("optic_tbaa_root")
  for (data_type, fields) in summary_table.all_fields():
    type_node = TBAA::scalar_node(data_type.name, tbaa_root)
    for field in fields:
      TBAA::field_node(
        parent:  type_node,
        name:    field.path,    -- e.g., "Entities.healths"
        offset:  field.offset,
        size:    field.size,
      )
```

The critical invariant: two TBAA nodes that were proven alias-safe by the `alias_check()` function in §4.6 must have unrelated TBAA trees. This is enforced by generating TBAA nodes per-field rather than per-type.

### 16.12 Intrinsics and Target-Specific Lowering

The LLVM backend must support target-specific lowering for performance-critical patterns:

| Pattern | LLVM intrinsic | Trigger condition |
|---------|---------------|-------------------|
| SIMD traversal map | `llvm.x86.avx2.*/llvm.aarch64.neon.*` | `SimdEligible` flag |
| AES-NI decryption | `llvm.x86.aesni.*` | `CryptoGrade` on TLS optic |
| Prefetch | `llvm.prefetch` | `CacheGrade<N>` where N > prefetch threshold |
| io_uring submission | `@io_uring_submit` | `LivenessGrade<Always>` + network costate |
| DMA mapping | `@dma_map_single` | `DmaGrade` (kernel target only) |

Target capability is queried at compile time via a `TargetProfile`:

```rust
TargetProfile {
    arch:    X86_64 | AArch64 | RiscV64 | Wasm32,
    simd:    SimdCaps { avx2: bool, avx512: bool, neon: bool, ... },
    crypto:  CryptoCaps { aes_ni: bool, sha_ni: bool, ... },
    io:      IoCaps { io_uring: bool, kqueue: bool, iocp: bool },
}
```

An optic body that requests `CryptoGrade` on a target without `aes_ni` emits a diagnostic recommending either software AES or a different target tier. This is the same discipline as grade checking applied to target capabilities.

---

## 17. Multicore, NUMA, Memory Layout Arithmetic, and Reclamation

### 17.1 Why multicore is fundamentally a grade and layout problem

Shared-memory concurrency is not only a scheduler problem. It is a combined question about exclusivity, alias safety, cache-line ownership, and placement. That is why the language should extend its grade model rather than bolt a separate concurrency discipline onto the side.

### 17.2 False sharing as a static concern

#### 17.2.1 A practical guard formula

A simple parallel legality check can use:

```text
stride(field) >= CACHE_LINE_SIZE / threads_per_sharing_domain
```

When this inequality fails for a write-heavy parallel pass, the compiler should warn or reject depending on the requested strictness level. The point is not perfect hardware prediction; it is early visibility into a class of performance pathologies that is otherwise discovered too late.

When two cores update adjacent values that land in the same cache line, the program pays coherence costs even if the source language says the updates are independent. This is precisely the kind of machine fact the language wants to surface early.

A product or traversal that is safe in a single-threaded sense may still be a bad parallel candidate if its chunking and field stride invite false sharing. That is why the multicore story depends on both ownership grades and memory-layout arithmetic.

### 17.3 Atomic grades and memory order

Once the language admits lock-free updates, it needs a place for atomic capability and memory ordering. These do not replace ownership; they refine the meaning of shared mutation.

The right design is to make atomic use explicit in the optic's grade and in the field type. That keeps the control visible to both the checker and the backend.

### 17.4 Reclamation as optic-structured discipline

Hazard pointers and epoch-based reclamation can be understood as structured access disciplines over a shared data structure plus a reclamation context. That makes them good candidates for later optic-structured library support, but only once the ownership and host-context model is strong enough to carry their lifecycle rules honestly.

### 17.5 Memory layout arithmetic

#### 17.5.1 Concrete cache-line arithmetic

Assuming 64-byte cache lines:

| Element type | Bytes | Elements per line |
|---|---:|---:|
| `f32` | 4 | 16 |
| `Vec2` | 8 | 8 |
| `Vec3` | 12 | 5 with padding pressure |
| `u8` | 1 | 64 |
| `u64` | 8 | 8 |

#### 17.5.2 SoA vs AoS break-even intuition

A useful first-order heuristic is:

```text
utilization = fields_accessed / total_fields
```

When utilization is low, SoA wins because it stops the machine from dragging cold fields through the cache hierarchy. When utilization is close to one, AoS or AoSoA islands become more attractive.

A language that talks about cache grades should also explain when its layout choices pay off.

The essential break-even story is simple.

- If the hot path touches only a small fraction of fields, SoA wins decisively.
- If the hot path touches almost every field together, AoS or hybrid AoSoA islands can be better.
- If the platform is NUMA, locality is no longer just about cache lines but about socket placement.

This is why the book later treats memory layout not as a backend curiosity but as a language-level design topic.

#### 17.5.3 Comparison to layout defaults across common languages

The book spends unusual effort on SoA, AoS, AoSoA, cache lines, and NUMA because mainstream languages make very different default bets here.

| Language | Default layout instinct | What it optimizes for | Why Optic chooses a different default |
|---|---|---|---|
| **C++** | class/struct-centric AoS unless the programmer manually builds SoA containers | convenient expression of object-oriented or value-centric design | the target workloads of Optic are dominated by bulk field access, so the language refuses to make the cache-hostile default the silent one |
| **Java** | object graphs on a managed heap | programmer productivity and VM freedom to move objects | object indirection and layout opacity are acceptable for many domains but fight the language's attempt to make cache contracts explicit |
| **Python** | boxed objects and dictionaries, with array performance delegated to NumPy or other libraries | expressiveness and dynamism | the performance-critical representation lives outside the core language; Optic brings that representation into the language proper |
| **TypeScript** | JavaScript objects plus engine-specific hidden classes; typed arrays as a separate specialist tool | flexible application objects and browser/server portability | high-performance layout is split off into special APIs rather than serving as the default structural law |
| **Rust** | similar raw freedom to C++ with stronger type discipline | explicit control for expert authors | Optic agrees with Rust about explicitness but goes further by making field-wise data layout part of the surface language and grade story |

That is why SoA is treated as a memory law rather than as an optional library trick the optimizer is supposed to recover later.

### 17.6 Transition

With the core and the full-language machinery now established, the book can finally turn to the domains that motivated the language's ambition in the first place.

---

### 17.7 Detailed implementation reference: parallel products, atomics, and reclamation grades

Concurrency cannot be bolted on after the fact. The sections below show how fractional ownership, false-sharing guards, atomic grades, and reclamation protocols extend the same structural story rather than replacing it with an unrelated concurrency subsystem.

### 17.8 Why Shared Memory Concurrency is a Grade Problem

The grade model is now fractional from the outset, but multicore is where that decision begins to pay off visibly. `OwnershipGrade<r>` encodes a claim on an ownership budget, and the multicore extension makes that claim operational for partitioned parallel work. This produces a model where:

- **Exclusive mutable access** (`OwnershipGrade<1>`, single thread): one writer, no readers. Equivalent to a mutex-protected critical section.
- **Shared read access** (`OwnershipGrade<r>` for `r < 1`, multiple threads): any number of readers, no writer. Equivalent to a read-write lock in read mode.
- **Concurrent write access** (explicitly requires `AtomicGrade`): only permitted if the underlying field has an atomic type (e.g., `Atomic<u64>`, `Atomic<f32>`).

The grade system enforces these constraints at compile time, eliminating the need for runtime lock acquisition in the common case.

### 17.9 Parallel Product Under `***`

The `***` operator in the full language supports a `parallel` qualifier:

```rust
entities
    .query(HealthView *** PositionView)
    .parallel(grade: CacheGrade<8> + OwnershipGrade<1>)
    .map(|(h, p)| update(h, p));
```

The `parallel` qualifier tells the compiler to split the entity range into chunks and schedule them across available cores. The grade check must now verify:

1. **Partition independence**: each element's update depends only on that element, not its neighbors. This is checked via the alias checker: if `put_writes` does not include any cross-index dependency, partition independence holds.
2. **Cache grade under parallelism**: `CacheGrade<8>` in a parallel context means "no more than 8 cache lines are touched per thread per chunk." This prevents false sharing: if two threads write to adjacent cache lines, the hardware coherence protocol forces cache-to-cache transfers.
3. **False-sharing guard**: the compiler checks that `stride(field) >= CACHE_LINE_SIZE / num_threads` when the parallel grade is active. If false sharing would occur, it emits `PAR-601` recommending either padding the SoA fields or adjusting the chunk size.

The lowering uses a rayon-style work-stealing executor in the full language runtime, but the scheduling is driven by the grade, not by runtime heuristics:

```rust
// optic(parallel): [HealthView, PositionView], CacheGrade<8>
let chunk_size = CACHE_LINE_SIZE * 2 / core::mem::size_of::<(f32, Vec2)>();
entities.healths.par_chunks_mut(chunk_size)
    .zip(entities.positions.par_chunks_mut(chunk_size))
    .for_each(|(h_chunk, p_chunk)| {
        for (h, p) in h_chunk.iter_mut().zip(p_chunk.iter_mut()) {
            let (h_new, p_new) = update(*h, *p);
            *h = h_new;
            *p = p_new;
        }
    });
```

### 17.10 The Lock-Free Case: Atomic Grades

When a field is `Atomic<T>` in the costate, the ownership grade may be `AtomicGrade<CAS>` (compare-and-swap) or `AtomicGrade<FAA>` (fetch-and-add). These grades:

1. Allow multiple concurrent writers without exclusivity
2. Constrain the put body to use only atomic operations (checked by the type system)
3. Carry a `MemoryOrderGrade` dimension (SeqCst, AcqRel, Relaxed) with its own composition rules

```rust
optic AtomicHealthDecay: GradedOptic<Entities, Atomic<f32>,
    AtomicGrade<FAA> + MemoryOrderGrade<Relaxed>>
{
    get  s => s.atomic_healths[s.id].load(Ordering::Relaxed)
    put  (s, delta) => { s.atomic_healths[s.id].fetch_sub(delta, Ordering::Relaxed); }
}
```

The grade checker verifies that `MemoryOrderGrade<Relaxed>` is only permitted when the field has no sequencing dependencies (no other field's read depends on this field's atomically-visible write). Stronger orderings (`AcqRel`, `SeqCst`) are always safe but carry a `LatencyGrade` penalty that the scheduler optic enforces.

### 17.11 Hazard Pointers, Epochs, and the `ReclamationGrade`

For lock-free data structures that require safe memory reclamation (linked lists, hash maps, skip lists), the full language introduces `ReclamationGrade`:

```rust
optic HazardRead: GradedOptic<LockFreeList<T>, T,
    ReclamationGrade<HazardPointer> + SharedGrade>
{
    get  c => {
        c.hazard.protect(c.list.head());  // publish hazard pointer
        c.list.head().value               // safe to read
    }
    put  (c, _) => {}  // read-only; reclamation guard drops hazard on scope exit
}
```

`ReclamationGrade<HazardPointer>` tells the compiler that the `get` body publishes a hazard pointer that must be retracted before the cursor goes out of scope. The compiler generates the retraction automatically:

```rust
// Generated:
let cursor_0 = Cursor::new(&list, 0);
let _val = HazardRead.get(cursor_0);
// ... use _val ...
cursor_0.hazard.retract(); // generated automatically at cursor drop
```

This is the same pattern as Rust's lifetimes applied to cross-thread memory visibility rather than single-thread pointer validity.

---

### 17.12 Detailed implementation reference: cache arithmetic, AoSoA, and NUMA reasoning

The following material makes the memory-layout claims quantitative. It gives the arithmetic behind cache-line utilization, AoS versus SoA break-even points, hybrid layouts, and the way NUMA pressure becomes a first-class grade dimension.

### 17.13 The Cache Line as a First-Class Resource

A modern CPU cache line is 64 bytes. This is not an implementation detail; it is a resource that the programmer is responsible for allocating wisely. The grade model exists to make that responsibility explicit and checkable.

When you read one field from a 64-byte cache line, you pay the full cache miss cost but use only a fraction of the data. The fraction is `sizeof(field) / 64`. For a `f32` health value, you use `4 / 64 = 6.25%` of the cache line you paid for.

AoS (Array of Structures) places all fields of each entity together:

```text
AoS layout:
entity_0: [health: f32, position: Vec2(f32,f32), velocity: Vec2, flags: u8, ...]
entity_1: [health: f32, position: Vec2(f32,f32), ...]
...
```

If you want only `health`, you still load 64 bytes per entity because all fields are in the same cache line. For N entities, you load `N * 64` bytes but use only `N * 4` bytes: 6.25% utilization.

SoA (Structure of Arrays) separates fields into homogeneous arrays:

```text
SoA layout:
healths:   [h0, h1, h2, ..., h15, | h16, h17, ...]  -- 16 f32s per cache line
positions: [p0, p1, p2, ...,  p7, |  p8, p9, ...]   -- 8 Vec2s per cache line
...
```

If you want only `health`, you load 16 health values per cache line: 100% utilization. For N entities, you load `N / 16 * 64 = N * 4` bytes and use all of them.

The cache grade is directly proportional to this utilization: a `CacheGrade<1>` optic on a SoA field means "one cache line contains the relevant data for many elements, and we use all of it." A `CacheGrade<1>` optic on an AoS field means "one cache line contains one element's data, and we use a small fraction of it."

The compiler knows which layout is in use (the `data` declaration specifies `SoA<T>` explicitly) and computes grades accordingly. This is why the language requires explicit SoA declarations: the grade arithmetic depends on layout.

### 17.14 When to Break SoA: AoS Islands

SoA is not universally optimal. The break-even point occurs when:

```text
AoS is better when:
  (number of fields accessed per element) / (total fields per element)
  ≥ CACHE_LINE_SIZE / max(sizeof(field))

-- Example: 6 fields of f32 accessed out of 8 total fields
-- Break-even: 6/8 = 0.75 ≥ 64 / 4 = 0.0625... no, AoS is worse here
-- But with 8/8 fields accessed and element size = 64 bytes exactly:
-- AoS == SoA (same cache footprint)
```

The practical rule: use SoA when accessing fewer than ~50% of the fields. Use AoS (or AoSoA: Arrays of Structures of Arrays) when accessing most fields simultaneously. The language supports hybrid layouts:

```rust
data PhysicsEntities {
    // Hot path: position + velocity updated every frame together
    hot: SoA<PhysicsHot { position: Vec3, velocity: Vec3 }>,  // AoS island within SoA
    // Cold path: mass, material properties accessed rarely
    mass:     SoA<f32>,
    material: SoA<u32>,
}
```

The `PhysicsHot` sub-struct keeps position and velocity together (since they are always accessed together), while separating mass and material (which are rarely needed). The compiler computes the grade of the `hot` field as `CacheGrade<1>` per element (since 32 bytes fits in half a cache line), not `CacheGrade<2>`.

### 17.15 Cache Grade Arithmetic: Concrete Numbers

The grade algebra is an approximation, but it should track reality closely enough to prevent common mistakes. Here are the concrete numbers behind the grade calculation:

```text
CACHE_LINE_BYTES = 64
ELEMENT_STRIDE(f32)  = 4   => elements per line = 16
ELEMENT_STRIDE(Vec2) = 8   => elements per line = 8
ELEMENT_STRIDE(Vec3) = 12  => elements per line = 5 (with 4 bytes padding)
ELEMENT_STRIDE(u8)   = 1   => elements per line = 64
ELEMENT_STRIDE(u64)  = 8   => elements per line = 8
```

For a traversal over N elements of type `f32`:
- Cache lines accessed = `ceil(N / 16)`
- Grade for fixed N: `CacheGrade<ceil(N / 16)>`
- Grade for symbolic N: `TraversalGrade<N>` (deferred to Z3)

For a product `SoA<f32> *** SoA<Vec2>`:
- Both arrays are traversed in lockstep
- Cache lines per element for f32: 1/16 (amortized)
- Cache lines per element for Vec2: 1/8 (amortized)
- Sum per iteration: 1/16 + 1/8 = 3/16
- For 16 elements: 3 cache lines total
- Grade: `CacheGrade<3>` per 16-element block

This is why `combine_par` uses `max` not `add` for the cache dimension: both arrays are accessed in the same loop, but the CPU's hardware prefetcher can hide the latency of the second stream if the first is in cache. The `max` is the pessimistic bound; real hardware often achieves better due to prefetch overlap.

### 17.16 When NUMA Changes Everything

On multi-socket NUMA machines, the "cache line" model breaks down: memory on a remote NUMA node has 2–4× the latency of local memory. The full language adds a `NUMAGrade` dimension:

```rust
optic RemoteHealthRead: GradedOptic<RemoteEntities, f32,
    CacheGrade<1> + NUMAGrade<Remote(socket=1)>>
```

The `NUMAGrade` dimension triggers a different code path in the parallel scheduler: instead of distributing work by entity range, it distributes work by NUMA node locality. Entities on socket 0 are processed by threads pinned to socket 0; entities on socket 1 by socket 1 threads. The grade system makes this locality visible rather than hoping the OS gets it right.

---

## 18. Foreign Boundaries, Unsafe, and Full Generality

### 18.1 Why this interlude exists

The earlier chapters deliberately began with the smallest fragment of the language that could be made mechanically honest. That was the right starting point, but it can create a reasonable fear: perhaps the architecture only works while the language remains sheltered from the things that make real kernels, engines, browsers, databases, and large tools difficult. Real systems cross foreign ABIs, manipulate hardware directly, interact with managed runtimes, use callbacks and plugins, pin memory, coordinate with linkers and loaders, and occasionally require inline assembly or privileged instructions.

A language that cannot absorb those pressures without abandoning its own model is not a fully general systems language. It is an elegant research subset with a growing pile of escape hatches.

The design goal here is the opposite. The language should become fully general **without** introducing a second semantics for the hard cases. The earlier machinery — explicit runtime, costates, graded optics, region summaries, CGIR, staging, determinism classes, and agent-facing diagnostics — should remain the primary architecture. The extension should be as small as possible and should reuse those concepts aggressively.

### 18.2 The minimal-extension rule

The best way to keep the language coherent is to adopt one strong rule:

> `unsafe` does not mean “leave the language.”  
> It means “introduce a trusted boundary node with explicit obligations.”

That single sentence reshapes a large design space.

- FFI is not a magical portal. It is a boundary leaf.
- MMIO is not ordinary memory, but it is still a region with an address space and volatility contract.
- A callback-capable API is not “just a function pointer”; it is a boundary that may reenter the graph.
- An allocator is not invisible background machinery; it is a host capability with ownership and layout rules.
- A plugin ABI is not outside the runtime model; it is a stable module boundary inside `HostContext`.

This rule preserves a major advantage over C++, Rust, and many existing runtime ecosystems. Those languages absolutely can interoperate with foreign code and hardware, but the semantic burden is often split between `unsafe` blocks, calling-convention attributes, build-system conventions, linker scripts, hand-maintained documentation, and profiler lore. The Optic approach tries to keep all of that visible in a single graph-facing summary.

Chapter 27 later names this more sharply as the **boundary** lane of the language, and Chapter 28 turns it into a recurring proposal test: support the use case fully, but keep it attached to the same summaries, grades, and graph revisions instead of treating it as a pretext for a second semantic center.

### 18.3 Reusing `Runtime`: the host is where foreign boundaries already belong

The current model already says that the program runs over an explicit root:

```text
Runtime = AppWorld × HostContext × ControlRuntime
```

That is already enough to host general interop. The language does not need a second top-level universe called `ForeignWorld` or `UnsafeWorld`. It needs a broader `HostContext` whose subregions describe the realities that real systems touch.

```text
HostContext =
    Clock
  × Net
  × Disk
  × Devices
  × ForeignAbi
  × PluginRegistry
  × ManagedRuntimes
  × Allocators
  × Tracing
```

Several consequences fall out immediately.

First, FFI and hardware access become **location-aware**. A function that talks to a C library, a Vulkan driver, a POSIX socket, a Windows handle table, a Python runtime, or an x86 APIC register is not just "doing I/O". It is reading or writing a specific host-facing region.

Second, determinism and replay stay meaningful. A boundary that reads wall-clock time, polls the OS scheduler, consumes device interrupts, or invokes a foreign callback changes the determinism class of the surrounding pipeline. Treating the boundary as part of `HostContext` makes that visible.

Third, capability control becomes tractable. A module that should never touch MMIO or emit privileged instructions can be denied the relevant host subregions entirely, or given only wrapper optics whose boundary contracts rule out those operations.

### 18.4 Reusing optics: foreign work is still an optic-shaped transformation

The easiest way to widen the language would be to allow arbitrary foreign calls anywhere an expression is permitted and then let programmers wrap them however they like. That is also the easiest way to destroy the book's central invariant: the compiler stops seeing structured observations and updates over explicit context.

A better approach is to keep the foreign-facing API optic-shaped.

There are still two layers, but they are deliberately asymmetric.

1. **Raw foreign items** establish ABI facts. They tell the compiler about symbol names, calling conventions, and raw argument/result types.
2. **Boundary optics** turn those raw items back into graph-shaped transformations over explicit costates.

That means ordinary code ideally touches boundary optics, not raw foreign declarations, for the same reason ordinary code ideally touches safe wrappers rather than scattering `unsafe` blocks everywhere.

```rust
extern "C" fn c_poll(fd: i32, buf: *mut u8, len: usize) -> isize;

unsafe optic SocketRead: GradedOptic<SocketState, ByteBuf,
    BandwidthGrade<10Gbps> + BlockingGrade<MayBlock> + AffineGrade>
{
    get  s => poll_socket_via_c(s.fd, s.rx_buf)
    put  (s, b) => { s.rx_buf = b }
    safety {
        requires valid_fd(s.fd)
        requires writable_region(s.rx_buf)
        ensures no_alias_with_tx(s)
        summary abi(C)
    }
}
```

The raw call exists, but the optimizer, diagnostics engine, and scheduler reason primarily about `SocketRead`, not about the C symbol directly.

#### 18.4.1 Legacy adoption should look incremental, not utopian

The architecture should be judged by how well it wraps real legacy systems, not by how elegant it looks in a greenfield example. In practice this means opaque C and C++ handles, hand-tuned storage engines, graphics drivers, and platform SDKs should enter the language as host-facing regions plus explicit boundary optics. The ordinary path for adoption should therefore be: import a raw foreign surface, attach or infer a `BoundaryContract`, wrap it in one or more safe optics, and then let the rest of the compiler reason about summaries again.

A useful mental model is that legacy code does not disappear when Optic arrives. It becomes one more already-existing subsystem inside `HostContext` or a domain-specific costate, and the job of the language is to make the crossing explicit rather than heroic.

### 18.5 Reusing summaries: `BoundaryContract` is the minimal new static object

The current book already depends heavily on `OpticSummary` as the compiler's semantic currency. That is exactly where generality should be attached.

A foreign or unsafe optic differs from an ordinary lens-like optic in only one fundamental respect: the compiler needs more facts before it can trust the leaf. Those extra facts are not a second effect system. They are a **boundary contract**.

A useful contract schema is:

```rust
struct BoundaryContract {
    kind: BoundaryKind,              // Local | Extern | Intrinsic | Asm | Volatile
    abi: Option<AbiKind>,
    callconv: Option<CallConv>,
    unwind: UnwindPolicy,            // NoUnwind | MayUnwind | ForeignException
    may_callback: bool,
    reentrant: Reentrancy,
    thread_affinity: ThreadAffinity, // Any | Main | Render | Audio | Cpu(n)
    context: ExecContext,            // User | Kernel | Interrupt | SignalSafe
    address_space: AddressSpace,     // Ram | Mmio | Dma | Gpu | ForeignHeap | ManagedHeap
    volatility: Volatility,          // Ordinary | Volatile
    atomicity: Atomicity,            // None | Atomic(ordering)
    privilege: PrivilegeLevel,
    pinning: PinRequirement,
    allocator: AllocatorContract,
    layout: LayoutContract,
    stageability: Stageability,      // Static | Residual | Dynamic
    safety_clauses: Vec<SafetyClause>,
}
```

The important design point is that the existing summary still does most of the work. The compiler still cares about `get_reads`, `put_reads`, `put_writes`, grades, determinism, and provenance. The boundary contract simply tells later phases **how** to interpret those reads and writes safely.

This preserves a large amount of reuse.

- the alias checker still operates on regions,
- the grade checker still operates on the grade product,
- staging still asks whether a node is static, residual, or dynamic,
- replay still asks whether the node is pure, seeded, recorded, or opaque,
- CGIR still composes the same way,
- diagnostics still explain one summary at a time.

#### 18.5.1 Opaque foreign structs still reenter the graph as summarized costates

A common stress case is a legacy storage engine or graphics runtime whose public API is a forest of opaque C handles. The language should not pretend these handles are transparent. It should instead model them as explicit foreign regions with local summaries.

```rust
extern "C" {
    type kv_txn;
    fn kv_begin(db: *mut kv_db) -> *mut kv_txn;
    fn kv_get(tx: *mut kv_txn, key: *const u8, len: usize) -> kv_slice;
    fn kv_commit(tx: *mut kv_txn) -> i32;
}

data LegacyTxn {
    handle: ForeignHandle<kv_txn>,
    row_cache: SoA<Row>,
    status: TxnStatus,
}

unsafe optic LegacyTxnRead: AsymmetricGradedOptic<LegacyTxn, RowRef,
    IOGrade<2ms> + TransactionGrade<ReadOnly>,
    CacheGrade<1> + TransactionGrade<ReadOnly>>
{
    get  t => legacy_kv_get(t.handle, t.row_cache, t.status)
    put  (t, row) => { t.row_cache[t.id] = row.materialize(); }
    safety {
        requires valid_handle(t.handle)
        requires pinned(t.row_cache)
        summary abi(C)
        summary may_callback(false)
    }
}
```

The important point is not the exact storage engine API. It is that even opaque foreign structures still reenter the language through the ordinary summary path. The alias checker still sees regions, the grade checker still sees quantitative budgets, and the backend still sees one graph rather than a special legacy side-channel.

### 18.6 Reusing grades — and refusing to overload them

Earlier chapters argued that grades are the right place for algebraic resource facts. That remains true here, but it becomes even more important not to overload the semiring.

A good rule of thumb is:

- if the property naturally combines like a quantity or budget, it probably belongs in the grade product;
- if the property states how a boundary must be crossed, it probably belongs in the boundary contract.

So the model should continue to treat these as grades:

- cache footprint,
- latency envelope,
- bandwidth consumption,
- blocking budget,
- liveness,
- ownership strength,
- NUMA penalty,
- maybe atomic-cost classes.

And it should continue to treat these as qualifiers or contract fields:

- ABI and calling convention,
- may-unwind,
- may-callback,
- reentrancy,
- thread affinity,
- privilege level,
- address space,
- volatility,
- layout guarantees,
- allocator ownership and pinning.

This matters for clarity and for solver cost. A semiring that tries to represent both cache addition and C ABI calling conventions will quickly become nonsensical. The current architecture stays much cleaner if the algebra continues to talk about budgets while the boundary contract talks about legality.

### 18.7 The memory model under the same architecture

A fully general systems language needs an explicit memory model. That requirement does not invalidate the optic story; it tells the optic story what kinds of reads and writes exist.

The minimal extension is again to reuse the current region machinery and refine the meaning of a region by address space and access mode.

```text
Region = Path × AddressSpace × AccessMode
```

The important cases are:

- **ordinary RAM** — reorderable subject to alias and atomic rules,
- **atomic memory** — reorderable only according to memory order,
- **volatile/MMIO** — observable by devices or hardware and therefore not freely reorderable,
- **DMA-visible memory** — shared with a device and therefore subject to coherence and pinning rules,
- **managed heaps** — subject to rooting, pinning, and foreign ownership protocols.

This reuse is enough to explain why ordinary product fusion is still sound for two disjoint RAM regions, but not obviously sound for an MMIO write followed by a normal cached read, or for a DMA queue that must observe descriptor writes before a doorbell register is hit. Those cases are still region-based. They simply carry stronger barriers.

The optimizer then gets a precise rule: ordinary regions may use ordinary TBAA and reordering proofs; volatile or device-visible regions insert the required barriers and may block vectorization or fusion.

This is also where the language must stay precise about nondeterminism. Volatile and device-visible regions are not “nondeterministic” in the same sense as clocks or packet arrival order; they are **order-sensitive**. The backend may speculate ordinary pure RAM reads aggressively, but it must treat MMIO, atomics, DMA doorbells, and callback-visible writes through the stricter legality matrix developed in Chapter 16.

#### 18.7.1 The shortest path to a usable memory model is access-based, resource-aware, and weak-memory conscious

The direct research path for v1 should therefore stay deliberately practical. Start with an **access-based abstract machine** over `Region = Path × AddressSpace × AccessMode`; refine alias and provenance reasoning with ownership and resource invariants; and refine reorder legality with a weak-memory model that can talk directly about atomics, fences, volatility, and device-visible order. This path is simpler to implement than importing the richest available proof theories first, and it aligns naturally with the existing `RegionSet`, `BoundaryContract`, and grade architecture.

#### 18.7.2 The direct experimental answers should therefore live in `std.experimental.sep` and `std.experimental.memory`

The book's experimental lane should reflect that sequencing explicitly. `std.experimental.sep` is the right place to prototype resource/separation-style reasoning over regions, ownership fractions, boundary obligations, and local unsafe proofs. `std.experimental.memory` is the right place to prototype weak-memory and ordering witnesses, litmus tests, fence rules, and backend reorder legality. These two lanes answer the open memory-model questions more directly than the richer proof-facing tracks, while still remaining fully compatible with the one-summary, one-graph architecture.

#### 18.7.3 Simpler sidecars should remain available while the language decides what belongs in core

Some of the most useful experiments should remain *outside* the core language until they prove that graph-native promotion is worthwhile. TLA+ and Alloy are especially good sidecars for protocol, callback, interrupt, and distributed-runtime design because they explore state-space and counterexamples without committing the language to new core syntax. Typestate and explicit protocol automata are often a simpler first answer than full session-theoretic or sheaf-theoretic machinery. Abstract interpretation is often a simpler first answer for conservative optimization and legality approximations than proof-heavy machinery. The language should therefore treat these as companion methods for v1 stabilization rather than as evidence that every unresolved question needs a new feature.

#### 18.7.4 A practical v1 stack should map each open question to one direct lane and one simpler fallback

The memory-model backlog is easier to stabilize when each open question has an explicit first answer, a simpler fallback, and only then a richer second-wave theory.

| Open question | Direct internal lane | Simpler fallback or sidecar | Richer second-wave candidate |
|---|---|---|---|
| pointer provenance and unsafe aliasing | `std.experimental.sep` with borrow/resource witnesses | conservative abstract interpretation over region/access summaries | `std.experimental.proof` |
| atomics, fences, volatility, DMA ordering | `std.experimental.memory` with litmus and reorder witnesses | translation validation plus backend litmus suites | `std.experimental.proof` |
| callback, interrupt, and protocol-state discipline | explicit typestate / protocol automata over existing summaries | TLA+ state-machine models | `std.experimental.topos` or later session/sheaf work |
| local-to-global consistency across projections, plugins, or distributed views | existing boundary/interface contracts plus Alloy/TLA+ modeling | Alloy constraint models over interface/runtime metadata | `std.experimental.sheaf` |
| optimizer legality and conservative approximation | summary-driven abstract interpretation | ordinary benchmark + diagnostic discipline | `std.experimental.proof` |

This table is the practical meaning of the book's “smallest theory first” rule. It keeps the v1 roadmap aligned with the core model of coalgebraic graded optics over explicit regions and boundaries, while still leaving room for richer proof and categorical tracks to earn promotion later.

#### 18.7.5 The practical implementation sequence should stay narrower than the full research surface

The easiest way to lose coherence here would be to treat every mathematically respectable answer as part of one required stack. The implementation story should stay narrower:

1. formalize an access-based region and boundary model;
2. prototype ownership/provenance reasoning in `std.experimental.sep`;
3. prototype weak-memory and ordering witnesses in `std.experimental.memory`;
4. validate the resulting rules with litmus suites, backend translation checks, and explicit sidecars where they are cheaper;
5. only then decide whether the richer `proof`, `sheaf`, `topos`, or `dynamics` tracks answer something the first stack still cannot.

That sequence is the best fit for the language because it reuses the same `RegionSet`, `BoundaryContract`, summary, and backend-legality objects the compiler already has. It also keeps the memory-model work compatible with the closure rule introduced later in the book: a feature only graduates when the simpler answer is no longer enough.

### 18.8 Address spaces and hardware costates

One of the critique's strongest points is that "data as costate" cannot by itself erase the operational difference between cacheable memory, blocking I/O, and device interaction. The minimal answer is not to abandon the costate model. It is to make address spaces explicit inside the model. fileciteturn2file2

That yields types such as:

```text
Ram<T>
Mmio<T>
Dma<T>
GpuVisible<T>
ForeignHeap<T>
ManagedHandle<T>
```

Now a page table, a ring descriptor array, a GPU-visible staging buffer, and a Java object handle are all still regions that optics can focus on, but they are no longer accidentally treated as ordinary cacheable arrays.

This is enough to express extremely low-level code without creating a new mini-language.

```rust
unsafe optic LocalApicEoi: GradedOptic<Mmio<LocalApic>, u32,
    BlockingGrade<Never> + LinearGrade>
{
    get  r => volatile_load_u32(r.base + EOI_OFF)
    put  (r, v) => volatile_store_u32(r.base + EOI_OFF, v)
    safety {
        requires privilege(Kernel)
        requires mapped_mmio(r.base, 4)
        requires aligned(r.base + EOI_OFF, 4)
        ensures no_unwind
    }
}
```

The optic is still just an optic. What changes is the address space and the safety clauses.

### 18.9 ABI, layout, calling conventions, unwinding, and callbacks

A language becomes fully general only when it can cross the host's actual binary boundaries without pretending those boundaries are trivial.

That means admitting several facts explicitly.

- C ABI should be the default interoperable baseline.
- C++ interop should normally begin through C shims, not through heroic direct ABI matching.
- Interrupts, syscalls, vectorcall-style ABIs, and platform-specific callbacks must be expressible when the target requires them.
- Struct layout and alignment must be controllable with explicit contracts.
- Unwinding across boundaries must never be implicit.

The current model can absorb all of these as contract fields on boundary leaves and module exports. That has two advantages.

First, the decision remains local. The ABI rule lives on the declaration or export that crosses the boundary; it does not infect the optic calculus itself.

Second, diagnostics gain direct evidence. If a callback can only run on the render thread, or an exported function may not unwind, or a kernel entry point requires an interrupt-safe calling convention, the diagnostic can quote the exact boundary field that made the code illegal.

Callbacks deserve special emphasis because they are where many systems stop looking like neat request/response code. GUI frameworks, window systems, graphics APIs, audio backends, browser embeddings, and game-engine plugins all reenter application code from the host side. In the current model that is not a reason to give up. It is a reason to mark the boundary with:

- `may_callback`,
- `reentrant`,
- `thread_affinity`,
- and an execution context such as `Main`, `Render`, `Audio`, `Kernel`, or `Interrupt`.

Those four fields already explain most of what makes callback-heavy systems difficult.

### 18.10 Allocation, ownership bridges, and managed runtimes

If the language is meant to build real kernels and engines, it needs a first-class story for allocation and foreign ownership.

That includes:

- arena, slab, pool, and page allocators,
- foreign alloc/free pairs,
- pinned memory,
- physically contiguous DMA buffers,
- huge pages,
- executable memory,
- reference-counted foreign objects,
- managed GC roots and handles.

The current ownership model is an excellent starting point here because it already distinguishes shared, affine, and linear use. The minimal extension is to add allocator and ownership-bridge contracts to boundary summaries rather than inventing a separate resource language.

This is where comparisons with mainstream languages are useful again.

| Ecosystem | Typical boundary style | What Optic wants to reuse or improve |
|---|---|---|
| **C++** | custom allocators, RAII wrappers, inline UB hazards, plugin ABI headers | keep explicit ownership and layout control, but summarize foreign boundaries more uniformly |
| **Rust** | `extern`, `unsafe`, raw pointers, `repr(C)`, pinning, allocator APIs | keep the discipline of localizing danger, but feed the same facts into optic summaries and CGIR |
| **Java** | JNI / Panama handles, GC roots, native pinning | model managed handles as host regions instead of letting them live in a separate conceptual universe |
| **Python** | C API / CFFI / ctypes, refcounts, extension modules | make reference ownership and callback rules explicit rather than purely API-conventional |
| **TypeScript / JS** | N-API, WebAssembly, browser-host callbacks | preserve thread/context and callback contracts instead of treating host edges as engine magic |

A managed runtime therefore does not force a new semantics. It forces `ManagedHandle<T>`, root-registration rules, finalization policies, and callback contracts — all of which fit naturally into the boundary model already described.

### 18.11 Inline assembly, intrinsics, and privileged instructions

Kernels, low-level runtimes, and performance-critical engines sometimes need direct access to instructions that no portable surface syntax should pretend to abstract away.

Examples include:

- interrupt enable/disable,
- cache maintenance and TLB instructions,
- privileged CSR/MSR access,
- explicit memory fences,
- vector or crypto intrinsics not otherwise surfaced,
- tiny critical inline assembly sequences with register constraints.

The minimal-extension rule still applies. These should not create a separate compiler sublanguage. They should appear as boundary leaves with explicit clobbers, side-effect flags, privilege requirements, and volatility. In the current model, a small `Asm` boundary kind is enough. The CGIR still sees a leaf. The backend still sees a contract.

### 18.12 Separate compilation, dynamic loading, and plugins

A fully general language cannot assume whole-program compilation forever. Real engines load plugins. Real kernels load modules. Real browsers and databases expose extension APIs. Real organizations ship stable binary boundaries long before they recompile the world as one unit.

This, too, fits the current architecture if modules and plugins are treated as named foreign regions plus ABI-stable exports. The key additions are:

- module/export declarations with boundary contracts,
- stable symbol names and layout rules,
- explicit monomorphization boundaries,
- dynamic loader integration,
- hot-reload or plugin registries as host costates.

A plugin is then simply a host-provided module whose entry points are boundary optics. The same diagnostics that catch an unwind violation at a C ABI call can catch an invalid engine-plugin callback or a mismatched kernel-module entry point.

### 18.13 Determinism, replay, and foreign boundaries

The earlier replay model already classifies computations as `Pure`, `Seeded`, `Recorded`, or `Opaque`. Foreign boundaries should not bypass that classification. They should refine it.

A foreign call that depends only on its inputs may still be `Pure`. One that consumes a seedable RNG is `Seeded`. One that reads packets or system input may be `Recorded`. One that consults the ambient scheduler, wall clock, or hidden global state is `Opaque` until wrapped or instrumented.

This is especially important for kernels and engines. Once devices, callbacks, and plugin boundaries exist, reproducibility and debugging become harder, not easier. Treating foreign edges as summary-bearing nodes is exactly what keeps replay and observability alive in the harder regime.

### 18.14 Security, capability gating, and auditability

The moment a language gains raw pointers, privileged instructions, MMIO, plugins, or managed-runtime bridges, it also needs a story about who is allowed to cross which boundary.

The current model already points toward one: capabilities over host subregions and module-level permissions. That lets the language say not only "this operation is unsafe" but "this module is not even allowed to name the relevant host capability unless explicitly granted it."

This helps in several ways.

- kernels can isolate driver-facing capabilities from ordinary subsystems,
- browsers can separate renderer, network, and GPU privileges,
- games can isolate editor plugins from shipping-runtime capabilities,
- build tools can distinguish compile-time filesystem access from runtime network access.

Auditability then becomes a natural extension of the existing diagnostics discipline. The compiler can enumerate all boundary leaves and their contracts as part of the build output. Unsafe does not disappear. It becomes locally declared, summarized, and searchable.

### 18.15 What full generality still requires beyond FFI itself

FFI and `unsafe` are essential, but they are not the only missing pieces between a research-strong language and a fully general systems language. Under the current model, the remaining requirements fall into a few coherent groups.

1. **Memory-model completion** — atomics, fences, volatility, provenance, DMA coherence.
2. **Module and ABI discipline** — stable exports, separate compilation, plugins, loaders, hot reload.
3. **Allocator and ownership bridges** — foreign allocators, pinning, root registration, reference-counted foreign objects.
4. **Context-sensitive execution** — callbacks, reentrancy, thread affinity, interrupt and signal contexts.
5. **Target and privilege control** — no-std startup, linker sections, privileged instructions, address spaces.
6. **Tooling and diagnostics** — audit surfaces, boundary summaries, machine-readable unsafe failures, replay classification.

The important point is that none of these requires a new semantic center. They all grow out of the same root objects the book has already established.

- `Runtime` and `HostContext` describe where the boundary lives.
- optics describe the transformation shape.
- grades describe algebraic resource contracts.
- boundary contracts describe non-algebraic legality facts.
- `OpticSummary` packages those facts for the checker, optimizer, backend, and diagnostics.
- CGIR keeps the graph visible until the backend can lower it correctly.

### 18.16 Why this is enough for a fully general language

A language becomes incoherent when it must explain simple in-memory code one way, compile-time specialization another way, networking a third way, raw hardware a fourth way, and foreign legacy code a fifth way. The book's argument is that Optic can avoid that fate.

The language does not claim that every domain behaves the same. It claims something more specific and more useful:

> every domain can be brought under one compiler architecture if the boundary between domains is modeled explicitly enough.

That is why the extension proposed here stays so conservative. It does not add a special macro language for compile time, a separate FFI IR for foreign code, a callback calculus just for GUI or engine APIs, or a different semantics for kernel-only code. It reuses the current model and admits only the extra contracts that the harder domains objectively require.

That is also why the next chapters can stay domain-specific without becoming semantically fragmented. Kernels, browsers, databases, games, compilers, and plugins are all different. But they can still share one core picture: explicit costates, explicit optics, explicit summaries, explicit grades, and explicit boundary contracts wherever the host or hardware enters the story.

### 18.17 Transition

With the generality question addressed, the domain playbooks can now be read more concretely. The point of the next part is not to sell the language abstractly, but to show what the same model looks like when it is pushed into kernels, browsers, databases, games, compilers, and the operational environments that surround them.

---

# Part IV — Domain Playbooks and Graph-Native Tooling

Part IV asks whether the architecture still reads naturally once it leaves the whiteboard. The domains here are not marketing examples; they are stress tests. Each chapter starts from the same question: if the language is taken seriously in this domain, what are the root costates, hot loops, grade pressures, and boundary contracts that make or break credibility?

## 19. Kernels and Kernel-Class Systems

### 19.1 Why kernels are a meaningful stress test

A kernel is where the language's promises meet their harshest version.

- resources are linear or affine in the strongest possible sense,
- blocking mistakes can deadlock the system,
- latency and liveness rules matter,
- host boundaries are the whole game,
- provenance and observability are operational necessities rather than nice extras.

That makes kernels the hardest long-range test of the language's design.

### 19.2 Core kernel costates

#### 19.2.1 Representative kernel root decomposition

```text
KernelRuntime =
  FrameAllocator × AddressSpaces × RunQueues × DeviceRegistry × TraceBuffers
```

A kernel-class implementation naturally decomposes into costates such as:

- physical frame allocator,
- page tables and address spaces,
- run queues and scheduler state,
- IRQ or event routing structures,
- socket and channel tables,
- device and DMA descriptors,
- tracing buffers.

The point is not that the kernel becomes one giant lens. It becomes an explicit graph of transformations over explicit subsystems, each with strong ownership and timing rules.

### 19.3 Example: frame allocation as a linear optic

```rust
optic FrameAlloc: GradedOptic<FrameAllocator, PhysFrame,
    CacheGrade<2> + LinearGrade> {
    get  fa => choose_free_frame(fa)
    put  (fa, frame) => mark_allocated(fa, frame)
}
```

The key design claim is that a correctness property usually enforced by discipline or post hoc testing becomes part of the type and grade story: a frame is not just an integer index, it is a linear resource passing through a structured update path.

A frame allocator is a clean example because it makes the ownership story concrete. A frame must not be allocated twice, freed twice, or leaked silently.

```rust
optic FrameAlloc: GradedOptic<FrameAllocator, PhysFrame,
    CacheGrade<2> + LinearGrade> {
    get  fa => choose_free_frame(fa)
    put  (fa, frame) => mark_allocated(fa, frame)
}
```

The exact code shape may vary, but the point is stable: the type and grade structure are carrying a real systems invariant.

### 19.4 Page walks, interrupts, and scheduling

```rust
optic PageTableWalk: GradedOptic<AddressSpace, PhysFrame,
    CacheGrade<4> + LinearGrade> {
    get  as => walk_tables(as, va)
    put  (as, frame) => install_mapping(as, va, frame)
}
```

A four-level x86-64 walk is a nice pedagogical example because the grade is not an abstract number. It mirrors the hardware's own table depth almost exactly.

Page-table walks map neatly onto nested or composed optics because the hardware itself is a structured traversal through a fixed-depth hierarchy. Interrupt handling maps onto optics with hard non-blocking and latency grades. Scheduling maps onto optics over run queues and task state with liveness and ownership constraints.

This is where the book's insistence on explicit host context pays off. Hardware-visible and scheduler-visible state is not smuggled in through ambient calls. It is where the program actually says it is.

#### 19.4.1 Kernel reality: MMIO, DMA, interrupts, and assembly stay in the same graph

A real kernel does not live purely inside RAM-shaped data structures. It talks to devices through MMIO windows, submits DMA descriptors, acknowledges interrupts, manipulates page tables, fences memory, and occasionally uses privileged instructions or tiny sequences of assembly. The language only remains credible for kernel work if those operations fit the same summary model rather than exploding into ad hoc escape hatches.

The good news is that the current model is already close. An MMIO page is simply not `Ram<T>`; it is `Mmio<T>`. A descriptor ring shared with a NIC is not an ordinary queue; it is `Dma<DescriptorRing>`. An interrupt handler is not just another callback; it carries `ExecContext::Interrupt`, `BlockingGrade<Never>`, and a privilege requirement. Inline assembly is not a separate little language from the compiler's point of view; it is a boundary leaf with clobbers, volatility, and ordering constraints.

That reuse is what lets kernel code still benefit from the rest of the system.

- alias summaries continue to explain which page-table or queue regions are touched;
- grades continue to explain latency, blocking, ownership, and liveness constraints;
- determinism summaries continue to say which paths are replayable in simulation and which are not;
- diagnostics continue to point to one boundary declaration rather than to an opaque `unsafe` region spread across half a driver.

The hard rule is that the kernel subset should not get a shadow semantics. It gets stricter boundary contracts, stricter capabilities, and stricter execution-context rules, but it still lives in the same graph.

### 19.5 The kernel ladder

No serious project should jump from a Rust-hosted v0 compiler directly to a Linux-equivalent kernel claim. The ladder should be gradual.

1. user-space kernel simulator,
2. `no_std` runtime and allocator,
3. memory management and timers,
4. cooperative then preemptive scheduling,
5. block and network I/O,
6. SMP and capability/security layers.

The language earns each step only after the smaller invariants stay stable.

### 19.6 Transition

Kernels are the most austere systems domain. Browsers are nearly the opposite: huge structured dataflow systems with rendering, DOM mutation, incremental layout, and rich host interaction. The next chapter shows why the same core abstractions still apply.

### 19.7 Detailed implementation reference: kernel subsystems as costates and optics

Kernels are the harshest environment for the language’s promises because they expose real resource edges: frames, address spaces, interrupts, queues, devices, and scheduling deadlines. The detailed sections below make those mappings concrete.

A kernel's job is to mediate between hardware resources and processes. Every kernel subsystem manages a typed costate (a region of structured data) and exposes typed operations (optics) over it.

#### 19.7.1 Physical Memory Manager

```text
Costate:      FrameAllocator { free_list: BitSet, frames: SoA<FrameState> }
Focus type:   PhysFrame (a single 4KB page frame)
Optic:        FrameAlloc: GradedOptic<FrameAllocator, PhysFrame, LinearGrade + CacheGrade<2>>
```

The `LinearGrade` ensures each frame is allocated exactly once and freed exactly once. The type system prevents double-free and use-after-free at compile time: a `PhysFrame` value with `LinearGrade` must be passed to `FrameFree` before the scope exits, or the compiler emits `ALI-221`.

```rust
optic FrameAlloc: GradedOptic<FrameAllocator, PhysFrame, LinearGrade + CacheGrade<2>> {
    get  fa => {
        let idx = fa.free_list.find_first_set();  // bitset scan: CacheGrade<1>
        fa.free_list.clear(idx);                   // bitset write: CacheGrade<1>
        PhysFrame { idx }
    }
    put  (fa, frame) => { fa.free_list.set(frame.idx); }  -- this is actually FrameFree
}
```

The grade `CacheGrade<2>` reflects: one cache line for the bitset (free list), one for the frame state array. On a 64-bit bitset with 64 entries per word, `find_first_set` touches exactly one 64-byte cache line.

#### 19.7.2 Virtual Memory and Page Table Walks

A page table walk in x86-64 is a four-level traversal: PML4 → PDPT → PD → PT → physical frame. This is naturally a `Traversal` or nested `Compose`:

```rust
optic PageTableWalk: GradedOptic<AddressSpace, PhysFrame,
    CacheGrade<4> + LinearGrade + IOGrade<0ns>>
{
    -- get: walk the table, load the terminal PTE
    get  as =>
        as.pml4[va.pml4_idx()]
          .pdpt[va.pdpt_idx()]
          .pd[va.pd_idx()]
          .pt[va.pt_idx()]
          .frame()

    -- put: update the terminal PTE
    put  (as, frame) => {
        as.pml4[va.pml4_idx()]
          .pdpt[va.pdpt_idx()]
          .pd[va.pd_idx()]
          .pt[va.pt_idx()]
          .set_frame(frame);
        flush_tlb(va);
    }
}
```

`CacheGrade<4>` is exact: four cache lines, one per table level. `IOGrade<0ns>` confirms this is purely in-memory (no I/O). The `LinearGrade` ensures the frame is not aliased into multiple page table entries.

#### 19.7.3 Interrupt Handler Discipline

An interrupt handler in a kernel context has two requirements: it must be fast (bounded latency), and it must not block (no sleep, no mutex acquisition that could deadlock with the interrupted code).

The grade system enforces these via `LatencyGrade` and `BlockingGrade`:

```rust
optic IRQHandler: GradedOptic<IRQState, IRQEvent,
    LatencyGrade<5us> + BlockingGrade<NonBlocking> + LinearGrade>
{
    get  irq => irq.pending_events.pop_front()
    put  (irq, event) => irq.handled.push_back(event)
}
```

`BlockingGrade<NonBlocking>` is checked by the compiler: any optic body that could block (mutex lock, channel receive, I/O wait) inside a `NonBlocking`-graded optic is a `KRN-1xx` error. The compiler's optic body analyzer detects:
- Any `put_reads` or `put_writes` to a `Mutex<T>` costate
- Any call to a function with `BlockingGrade<MayBlock>`
- Any coinductive optic call without `LivenessGrade<Bounded>`

#### 19.7.4 Scheduler as Optic

The process scheduler manages a `RunQueueSet` costate. Scheduling decisions are optics over that costate:

```rust
optic CFS_Dequeue: GradedOptic<RunQueueSet, Task,
    CacheGrade<3> + LinearGrade + LatencyGrade<1us>>
{
    get  rqs => {
        let cpu = rqs.current_cpu();
        rqs.per_cpu[cpu].run_queue.min_vruntime_task()  // O(log n) red-black tree
    }
    put  (rqs, task) => {
        let cpu = rqs.current_cpu();
        rqs.per_cpu[cpu].run_queue.remove(task);
        rqs.per_cpu[cpu].current = Some(task);
    }
}
```

The composition `CFS_Dequeue >>> ContextSwitch` is the full scheduler tick: dequeue the next task, then switch to it. The grade algebra computes the total latency:

```text
combine_seq(LatencyGrade<1us>, LatencyGrade<2us>) = LatencyGrade<3us>
```

If the platform has a hard real-time budget of 5µs per tick, the compiler verifies `3 ≤ 5` and accepts the composition. An over-budget composition is a compile-time error, not a runtime overrun.

---

## 20. Browsers and Interactive Rendering Systems

### 20.1 Why browsers fit the model

A browser is a long pipeline of transformations over structured data.

- parse HTML into DOM-like structures,
- cascade style,
- measure and place layout boxes,
- build paint and compositing data,
- rasterize and present,
- feed events back into document state.

This is almost the ideal demonstration that the language is not just for kernels or ECS loops.

### 20.2 DOM as a structured costate

```rust
data Document {
    nodes:          SoA<DomNode>,
    css_properties: SoA<ComputedStyle>,
    layout_boxes:   SoA<LayoutBox>,
    paint_layers:   SoA<PaintLayer>,
}
```

A browser does not need to use this exact layout everywhere, but the hot paths benefit enormously when DOM-adjacent data stops being a pointer-rich object graph and becomes an explicit structured arena.

A browser implementation can choose to model DOM nodes, computed styles, layout boxes, paint layers, and event registrations as typed arrays keyed by node id rather than as pointer-heavy object graphs in the hot path.

That makes style resolution and layout natural traversal and composition problems instead of opaque tree walkers.

### 20.3 Two-pass layout as explicit sequential composition

```rust
let layout_pipeline = MeasurePass >>> PlacementPass;
```

This tiny line is representative of the language's whole strategy. A common browser-engine fact becomes an explicit composition node that can carry cache, latency, staging, and provenance information instead of being hidden in a call stack.

Layout is a strong example of why sequential composition should stay explicit.

- first pass: intrinsic measurement,
- second pass: placement given containing blocks and constraints.

Treating that as `MeasurePass >>> PlacementPass` keeps the budget, fusion, and staging story honest. The compiler can see that this is two related passes over the same world rather than two arbitrary functions that happen to be called in sequence.

### 20.4 Rendering as staged and coinductive structure

```rust
stage {
    let visible_meshes = FrustumCull >>> SortByDepth >>> BuildDrawCall;
}

frame_buffer
    .query(visible_meshes)
    .coinductive()
    .drive();
```

The important part of the example is not the exact names of the passes. It is the separation between compile-or-frame-time specialization and live repeated execution.

Rendering is exactly the kind of problem that benefits from staged precomputation and then repeated event-driven execution.

- stage scene- or frame-level plans,
- coinductively drive input, animation, layout invalidation, and raster updates,
- keep provenance and diagnostics over the pipeline graph.

This is also one of the clearest examples of why observability as graph nodes is valuable. Browsers are notoriously difficult to debug once optimization, caching, and incremental invalidation are in play. A language-aware graph view is an unusually strong fit here.

### 20.5 Transition

Databases and games push on different aspects of the same architecture: one emphasizes plans, indexes, and transactional boundaries; the other emphasizes bulk data, SIMD, and frame budgets.

### 20.6 Detailed implementation reference: DOM, style, layout, and rendering as structured pipelines

Browser engines are full of repeated passes over large, heterogeneous state. The supplement below shows how the language forces those passes into explicit, checkable structures rather than allowing them to disappear into ad hoc graph walks and callback layers.

A web browser is essentially a complex data transformation pipeline from HTML/CSS source to rasterized pixels. Each stage in the pipeline is a natural optic over a typed costate.

#### 20.6.1 The DOM as a Costate

The Document Object Model is the browser's central costate. Each node is a focus; CSS properties, layout boxes, and event handlers are separate SoA fields hung off the same node IDs:

```rust
data Document {
    nodes:          SoA<DomNode>,       -- tag, id, class list
    css_properties: SoA<ComputedStyle>, -- resolved CSS values
    layout_boxes:   SoA<LayoutBox>,     -- position, size, overflow
    paint_layers:   SoA<PaintLayer>,    -- compositing layer assignment
    event_handlers: SoA<HandlerList>,   -- registered event listeners
}
```

This is exactly SoA layout for a DOM: instead of a pointer-chained tree of heterogeneous nodes (the traditional implementation), all nodes are stored in flat arrays indexed by node ID. CSS selectors become queries over `css_properties`; layout is a traversal over `layout_boxes`; painting is a traversal over `paint_layers`.

#### 20.6.2 Style Resolution as a Traversal

CSS cascading resolves computed styles from inherited rules, author stylesheets, and user-agent defaults:

```rust
optic StyleResolution: GradedTraversal<Document, ComputedStyle,
    CacheGrade<4> + SharedGrade + LatencyGrade<100us>>
{
    traverse doc => doc.css_properties.iter_mut()
        .zip(doc.nodes.iter())
        .filter(|(_, node)| node.is_element())
    update (doc, resolved_styles) =>
        doc.css_properties.iter_mut().zip(resolved_styles).for_each(|(cs, rs)| *cs = rs)
}
```

The traversal over all DOM elements respects selector specificity and inheritance order. Because `SharedGrade` is used (no in-place mutation during traversal), the traversal can be parallelized across subtrees of the DOM tree that have no CSS-inheritance dependency between them.

#### 20.6.3 Layout as Sequential Composition

Layout in a browser is inherently two-pass: first measure the intrinsic sizes of all nodes (bottom-up), then place them based on the containing block geometry (top-down). This is `MeasurePass >>> PlacementPass`:

```rust
optic MeasurePass: GradedOptic<Document, SizedNode, CacheGrade<3> + AffineGrade> {
    get  doc => measure_intrinsic(doc.nodes, doc.css_properties)
    put  (doc, sizes) => { doc.layout_boxes = sizes; }
}

optic PlacementPass: GradedOptic<Document, PlacedNode, CacheGrade<2> + AffineGrade> {
    get  doc => place_in_containing_block(doc.layout_boxes, doc.viewport)
    put  (doc, placed) => { doc.layout_boxes = placed; }
}

let layout_pipeline = MeasurePass >>> PlacementPass;
```

The grade of the composed pipeline is `CacheGrade<5>` (3 + 2) and `LatencyGrade<frame_budget - style_budget>`. If the composed grade exceeds the frame budget, the compiler requires splitting the pipeline and running the two passes in separate frame phases.

#### 20.6.4 The Rendering Pipeline as a Staged Optic

The rendering pipeline from layout to rasterized pixels is a natural staged optic. The layout boxes are known at "layout time" (before paint); the rasterization happens "paint time":

```rust
stage {
    -- Layout-time: compute which elements need their own compositing layer
    let layer_assignment = PaintLayerOptic >>> CompositingDecision;
}

-- Paint-time: rasterize using the staged (pre-computed) layer decisions
frame_buffer.query(RasterizeLayer(layer_assignment)).drive();
```

The `stage { }` block ensures that layer decisions (which are expensive to recompute) are cached and reused across frames unless the DOM changes. The grade algebra tracks which fields are accessed in the staged portion vs. the live paint loop, preventing the staged optic from accidentally reading live paint data.

---

## 21. Databases, Games, and Realtime Media

### 21.1 Databases: plans and storage as optics over explicit state

```rust
let query = AccountScan >>> BalanceFilter >>> ProjectIdBalance;
```

As a staged or compiled plan, this pipeline is not merely readable. It gives the optimizer, I/O model, and diagnostic system a single shared structure. That is exactly what database engines spend enormous effort reconstructing from more weakly typed plan representations.

A database query plan is almost tailor-made for a staged optic graph.

- scan,
- filter,
- project,
- join,
- aggregate,
- write back or stream result.

The language's stage machinery can specialize a plan once while leaving execution as a clear bulk-data pipeline. The storage engine side benefits from typed page, B-tree, and MVCC costates whose ownership and I/O budgets are explicit rather than implicit.

### 21.2 Why databases need asymmetric grades

```rust
optic DiskRead: AsymmetricGradedOptic<PageCache, Row,
    IOGrade<4ms>, CacheGrade<1>> {
    get  c => fetch_page(c, c.id)
    put  (c, r) => { c.pages[c.id] = r }
}
```

This is the canonical example of why the prelude's equal `get_grade` and `put_grade` fields are only a temporary simplification. Database and storage work make direction-sensitive costs unavoidable.

Database work makes asymmetric grades feel inevitable. A page read path and an in-memory projection or hash update path simply do not cost the same thing. The earlier decision to reserve `get_grade` and `put_grade` fields in summaries becomes a direct operational asset here.

### 21.3 Games: ECS, frame budgets, and SIMD bulk updates

```rust
world
    .query(AliveFilter *** Integrate)
    .parallel(grade: CacheGrade<8> + OwnershipGrade<1>)
    .map(update_physics)
    .drive();
```

Games make the language's central synthesis unusually tangible: structure-of-arrays layout, product composition, SIMD-friendly traversals, and per-frame resource budgeting all pull in the same direction.

Games were one of the earliest motivating examples because they sit at the intersection of locality, bulk iteration, and hard per-frame timing budgets.

Positions, velocities, health, transforms, and component flags all fit the `AppWorld` story naturally. Archetype specialization fits staging. Physics and animation fit traversal plus product composition. Audio fits SIMD-friendly signal pipelines.

This is one domain where the language can directly compete on code shape because the target loops are already familiar and concrete.

#### 21.3.1 Games in the real world: legacy engines, plugins, graphics APIs, and scripting VMs

A real game rarely owns its whole software stack. It sits inside or alongside a renderer, audio backend, input layer, asset pipeline, scripting runtime, editor process, hot-reload system, telemetry layer, platform SDK, and often several large legacy libraries. If the language only handles the pure ECS core elegantly, it will still fail the actual domain.

The current model scales better than it first appears because each of those edges is naturally a host boundary rather than a new semantic regime.

- graphics devices and command queues are host regions with thread-affinity and callback contracts;
- audio backends are boundary optics with strict no-allocation and bounded-latency requirements;
- scripting VMs become managed-runtime regions with handle, rooting, and callback policies;
- editor and plugin ecosystems become dynamic module tables with stable ABI contracts;
- hot reload becomes a staging and dynamic-loading story, not a magical mode switch.

This matters directly to engine architecture. A rendering submission path that may only run on the render thread, an audio callback that may never block, and a scripting bridge that may callback into gameplay code all become first-class boundary descriptions rather than folklore around framework APIs.

That, in turn, helps the optimizer and the programmer for the same reason as elsewhere in the book: the architecture and the machine story remain expressed in the same language.

### 21.4 Realtime media and audio

Audio processing is another excellent traversal domain because it turns bulk homogeneous sample updates into a strong SIMD story. The same core machinery that powers ECS traversals can power packed signal-processing passes. The domain changes; the bridge does not.

### 21.5 Transition

The final application chapter turns inward. A compiler is itself a systems program over structured IR, and it is the natural domain in which the language's self-hosting ambition becomes concrete.

### 21.6 Detailed implementation reference: storage engines, plans, indexes, and transaction optics

Database systems are a good test because they combine storage latency, structural planning, and high-contention state management. The following material grounds those concerns in concrete optic decompositions.

A database is a system for storing, querying, and updating large persistent datasets. Every component — storage engine, query optimizer, transaction manager, index structure — maps naturally to the optic model.

#### 21.6.1 Query Execution as Optic Composition

A SQL query `SELECT h.id, h.balance FROM accounts h WHERE h.balance > 100` decomposes as:

```rust
optic AccountScan: GradedTraversal<PageCache, AccountRow,
    IOGrade<4ms> + SharedGrade>
{
    traverse cache => cache.pages.iter().flat_map(|p| p.rows::<AccountRow>())
}

optic BalanceFilter: GradedPrism<AccountRow, AccountRow, CacheGrade<1>> {
    preview row => if row.balance > 100.0 { Some(row) } else { None }
    review  row => row
}

optic ProjectIdBalance: GradedOptic<AccountRow, (u64, f64), CacheGrade<1>> {
    get  row => (row.id, row.balance)
    put  _ => panic!("projection is read-only")  -- compile-time SharedGrade enforces this
}

let query = AccountScan >>> BalanceFilter >>> ProjectIdBalance;
result_set.query(query).collect()
```

This is a real query executor. The compiler fuses `BalanceFilter >>> ProjectIdBalance` since the prism focus type matches the projection input type. The `AccountScan` traversal is not fused with the filter because it crosses a page boundary (IO grade prevents trivial fusion with in-memory operations).

#### 21.6.2 B-Tree as a Recursive Optic

A B-tree index is a recursive data structure where each node contains a sorted array of keys and child pointers. The optic over a B-tree is a recursive traversal:

```rust
optic BTreeSearch<K: Ord, V>: GradedOptic<BTreeIndex<K, V>, V,
    IOGrade<{tree_height * PAGE_LOAD_TIME}> + SharedGrade>
{
    get  tree =>
        match tree.root.search(tree.query_key) {
            Found(leaf) => leaf.value,
            NotFound    => None,
        }
    put  (tree, v) => tree.root.insert(tree.query_key, v)
}
```

The grade `IOGrade<{tree_height * PAGE_LOAD_TIME}>` is a symbolic grade that the Z3 solver evaluates when `tree_height` is known statically (e.g., a fixed-depth index on a known-size dataset). For variable-height trees, the grade is left as a symbolic expression that the scheduler uses to estimate I/O budget.

#### 21.6.3 MVCC Transaction Isolation as Ownership Grades

Multi-Version Concurrency Control (MVCC) allows concurrent readers and a single writer by keeping multiple versions of each row. In the optic model, a row version is a focus type parameterized by a `TransactionGrade<T>`:

```rust
optic ReadSnapshot<T: TransactionId>:
    GradedOptic<MVCCStorage, RowVersion,
        SharedGrade + TransactionGrade<T> + SnapshotIsolation>
{
    get  storage => storage.versions.visible_at(T::txn_id()).latest()
    put  _ => {}  -- snapshot reads are always read-only
}

optic WriteRow<T: TransactionId>:
    GradedOptic<MVCCStorage, RowVersion,
        LinearGrade + TransactionGrade<T> + ReadCommitted>
{
    get  storage => storage.versions.latest_committed()
    put  (storage, new_version) => storage.versions.append(T::txn_id(), new_version)
}
```

`TransactionGrade<T>` carries the transaction ID as a type parameter. Two optics with different `TransactionGrade<T1>` and `TransactionGrade<T2>` are alias-safe by construction: they cannot conflict because they operate under different transaction contexts. This eliminates false conflicts in the MVCC conflict detection algorithm.

#### 21.6.4 Query Optimizer as Staged Optic Over IR

The query optimizer is itself a program that transforms query plans. A query plan is a costate; each optimizer rule is an optic over that costate:

```rust
data QueryPlan {
    nodes: SoA<PlanNode>,     -- scan, filter, join, project, sort, ...
    edges: SoA<DataflowEdge>, -- data flow between nodes
    stats: SoA<TableStats>,   -- cardinality estimates
}

optic PushdownFilter: GradedOptic<QueryPlan, QueryPlan,
    CacheGrade<4> + CompileTimeGrade>
{
    -- Move filter nodes above their input scans when predicate references only scan fields
    get  plan => analyze_pushdown_opportunities(plan)
    put  (plan, rewritten) => apply_rewrites(plan, rewritten)
}
```

Because the optimizer runs at query compilation time (not query execution time), it uses `CompileTimeGrade`. The staged optic model naturally separates plan optimization (compile time) from plan execution (runtime), with explicit grade boundaries ensuring no mixing.

---

### 21.7 Detailed implementation reference: ECS, archetypes, frame pipelines, and realtime media

Games and realtime media are where the language’s data-oriented roots are easiest to recognize. This supplement restores the detailed examples connecting archetypes, staged specialization, coinductive frame loops, and SIMD-friendly bulk processing.

A game engine's performance demands are the original motivation for the ECS / DoD approach, and the language's optic model is its natural theoretical home.

#### 21.7.1 The ECS as Graded Coalgebraic Optics

An Entity-Component-System stores components in SoA arrays indexed by entity ID. Each "system" in ECS is a loop over a subset of components. In the optic model:

- **Entity ID**: the cursor index
- **Component array**: a SoA field in the `World` costate
- **System**: an optic over the `World` costate focusing on one or more components
- **System schedule**: a CGIR graph of `Product` and `Compose` nodes over system optics

The full game world costate:

```rust
data World {
    -- Transform components
    positions:   SoA<Vec3>,
    rotations:   SoA<Quat>,
    scales:      SoA<Vec3>,
    -- Physics components
    velocities:  SoA<Vec3>,
    forces:      SoA<Vec3>,
    masses:      SoA<f32>,
    -- Rendering components
    meshes:      SoA<MeshHandle>,
    materials:   SoA<MaterialHandle>,
    -- Entity metadata
    archetypes:  SoA<ArchetypeId>,
    alive:       BitSet,
}
```

The physics integration system is:

```rust
optic Integrate: GradedOptic<World, (Vec3, Vec3),  -- (position, velocity)
    CacheGrade<4> + AffineGrade + LatencyGrade<1ms>>
{
    get  w => (w.positions[w.id], w.velocities[w.id], w.forces[w.id], w.masses[w.id])
    put  (w, (new_pos, new_vel)) => {
        w.positions[w.id]  = new_pos;
        w.velocities[w.id] = new_vel;
        w.forces[w.id]     = Vec3::ZERO;  -- clear accumulated forces
    }
}

-- Full physics tick:
world
    .query(AliveFilter *** Integrate)
    .parallel(grade: CacheGrade<8> + OwnershipGrade<1>)
    .map(|(_, (pos, vel, force, mass))| {
        let dt = 1.0 / 60.0;
        let accel = force / mass;
        let new_vel = vel + accel * dt;
        let new_pos = pos + new_vel * dt;
        (new_pos, new_vel)
    })
    .drive();
```

`CacheGrade<4>` reflects: positions (1), velocities (1), forces (1), masses (1) — four distinct SoA fields, four cache line families. The `.parallel()` splits the entity range across cores. The grade arithmetic prevents the mistake of over-scheduling (combining too many fields in one parallel pass creates false-sharing pressure).

#### 21.7.2 Archetype-Based Query Acceleration

Modern game engines use archetypes (fixed component sets per entity group) to avoid iterating over all entities. An archetype in the optic model is a staged product:

```rust
stage {
    -- Define archetype: entities with exactly {position, velocity, health}
    let PhysicsAndHealth = PositionView *** VelocityView *** HealthView;
    let archetype_0 = world.archetype_query(PhysicsAndHealth);
}

-- Hot loop: only iterates entities matching the archetype
archetype_0.query(PhysicsAndHealth).parallel().map(|...| ...).drive();
```

The `stage { }` block computes the archetype mask at world-creation time. The hot loop never tests which components an entity has; it iterates directly over the SoA arrays for the matching archetype. This is the same speedup that Bevy, Flecs, and Unity DOTS achieve, but derived automatically from the type system.

#### 21.7.3 Rendering Pipeline as Staged + Coinductive Composition

The rendering pipeline has two regimes: setup (staged, once per frame or scene load) and draw (coinductive, once per entity per frame):

```rust
stage {
    -- Scene baking: pre-compute visibility, sort draw calls, build command buffers
    let visible_meshes = FrustumCull *** SortByDepth >>> BuildDrawCall;
}

-- Frame loop:
frame_buffer
    .query(visible_meshes)
    .coinductive()
    .parallel(grade: GpuCacheGrade<16>)
    .drive();
```

`GpuCacheGrade<16>` is a new dimension (full language) that tracks GPU L1 cache pressure instead of CPU cache pressure. It triggers GPU-specific lowering: instead of LLVM, the compositor emits GPU command buffer entries using the Vulkan/Metal/DirectX API via a target-specific code path.

#### 21.7.4 Sound Engine as Signal Processing Optics

An audio engine processes samples in buffers. Each effect (reverb, EQ, compression) is an optic over a `SoundBuffer` costate:

```rust
data SoundBuffer {
    samples: SoA<f32>,   -- interleaved stereo or multi-channel
    sample_rate: u32,
    channels:   u8,
}

optic Reverb: GradedOptic<SoundBuffer, f32,
    CacheGrade<2> + LatencyGrade<5ms> + SimdEligible>
{
    -- convolution reverb: FIR filter over sample history
    get  buf => convolve(buf.samples[buf.id], IRF_BUFFER)
    put  (buf, processed) => { buf.samples[buf.id] = processed; }
}

let audio_pipeline = (LowPassFilter *** HighPassFilter) >>> Reverb >>> Compressor >>> Limiter;
```

Because `SimdEligible` is set on all these optics (they are arithmetic-only, no cross-sample dependencies), the compiler automatically emits AVX-512 code for the 16-sample-wide SIMD path. An audio buffer of 1024 samples processes in 64 AVX-512 iterations instead of 1024 scalar iterations: an 8× throughput improvement.

---

## 22. Compilers, Graph-Resident Tooling, Self-Hosting, and Governance

### 22.1 Why compilers are a natural target

```rust
optic ConstFold: GradedOptic<Module, Instr, CacheGrade<2> + CompileTimeGrade> {
    get  m => fold_if_constant(m.instructions[m.id])
    put  (m, i) => { m.instructions[m.id] = i }
}
```

Compiler passes are especially revealing because they show that the optic model is not only for physical resources. It is also a disciplined way to talk about IR-to-IR transformation while keeping provenance and legality visible.

A compiler already manipulates explicit graphs and trees, performs staged analysis and transformation, and cares deeply about diagnostics, provenance, and optimization legality. That makes it one of the most natural downstream domains for the language.

Dead-code elimination, constant folding, register allocation, CFG simplification, scheduling, and lowering all fit the optic story surprisingly well once the core calculus is strong enough.

### 22.2 Self-hosting as a late-stage discipline

#### 22.2.1 Differential trust loop

```text
1. compile compiler sources with the Rust seed
2. compile the same sources with the Optic compiler
3. compare diagnostics JSON on green and red suites
4. compare generated Rust or native output on canonical examples
5. compare benchmark drift against seed tolerances
```

Self-hosting becomes meaningful only when this loop is boring and repeatable.

The book treats self-hosting as a long-range systems milestone, not a branding event. The right sequence remains:

1. build and freeze the Rust prelude compiler,
2. move front-end pieces into the language,
3. move type checking, summaries, and CGIR machinery into the language,
4. validate against the Rust seed with differential tests,
5. only then move the rest of the compiler under self-hosting discipline.

This is one of the places where the book's governance stance matters. A self-hosted compiler that is not yet reproducible, diagnosable, and benchmarkable is not progress. It is a new source of ambiguity.

#### 22.2.2 What should become reusable libraries, and what must stay compiler-specific

A self-hosted compiler should not be understood as "ordinary user code plus the standard library". That model either bloats the standard library with compiler internals or forces every compiler component to reinvent the same substrate privately. The healthier split has three rings.

| Ring | Typical contents | Stability expectation | Why it belongs there |
|---|---|---|---|
| core standard library | collections, text/bytes primitives, paths, numeric utilities, `Option`/`Result`, general optic combinators, target-neutral concurrency helpers | language-level and broadly stable | ordinary programs should use these without inheriting compiler-version coupling |
| first-party toolchain support libraries | spans and file maps, interning, stable hashing, index arenas, mmap/journal graph-store primitives, module-interface codecs, diagnostics schema/renderers, target-profile descriptions, object/debug-info emitters, package-resolution engine, graph-protocol libraries | toolchain-stable, may evolve with compiler editions | reusable by the compiler, language server, package manager, alternate front ends, and analysis tools without freezing language law |
| compiler-private implementation | grammar tables, HIR/CGIR schemas, resolver, type rules, grade solver, summary builder, alias checker, fusion legality, backend legality, edition migrators, invalidation policy | compiler-private | these components embody the language's semantic authority and must remain free to evolve with the compiler |

The practical conclusion is subtle but important. A **large share of the infrastructure by code volume** may eventually live in reusable first-party libraries, while the **semantic authority of the compiler** should remain concentrated in compiler-specific crates. That is the right balance. It keeps the self-hosted compiler from degenerating into a pile of bespoke utilities, but it also avoids freezing optimizer laws or IR shapes as if they were standard-library promises.

A good promotion rule is conservative: move a component from compiler-private to first-party library only after at least two independent consumers need it and its contract can be described without reference to one particular compiler pass. Generic graph stores, span/file-map utilities, stable hashing, artifact codecs, and diagnostic schemas often meet that bar. HIR node sets, grade-law tables, or rewrite legality checks usually do not.

This split should become explicit in the long-range governance material and is summarized again in Appendix I together with the soundness-budget ledger and artifact-publicity classes.

#### 22.2.3 Toolchains are foreign-boundary systems too

Compilers often look self-contained on paper. In practice they are some of the most boundary-heavy programs in a systems stack. They parse files, cache artifacts, call assemblers and linkers, embed debuggers and profilers, interface with platform SDKs, consume foreign optimization libraries, and increasingly cooperate with editors, language servers, package managers, and AI agents.

That makes the compiler domain a good confirmation of the earlier interlude. The same boundary model that explains MMIO and graphics callbacks also explains toolchain reality.

- an external assembler or linker is an ABI boundary with determinism and staging consequences;
- LLVM or platform SDK bindings are foreign libraries whose summaries should remain visible to diagnostics and replay tooling;
- editor and LSP integration is a callback-heavy host interface rather than a separate semantic universe;
- AI-assisted repair loops rely on the same structured diagnostics whether the failing boundary is a parser rule, an optimizer invariant, or an FFI contract.

A compiler written in the language should therefore not need a privileged "metacompiler" escape hatch. It should exercise the same boundary discipline the language expects of kernels, services, and engines.

### 22.3 The project graph as the compiler and tooling system's primary costate

A mature compiler eventually starts to look like a database whether it admits this or not. It stores source text, syntax trees, typed IR, summaries, module interfaces, package resolution, diagnostics, generated artifacts, benchmark baselines, runtime blueprints, and invalidation metadata. If those are all kept in separate places, the toolchain spends an increasing fraction of its complexity budget just translating among its own partial copies of the world.

The better long-range architecture is to make the **project graph** primary and to treat the compiler graph as one hot projection of it.

```text
ProjectRuntime = ProjectGraph × CompilerHost × SessionState
ProjectGraph   = TextArena × SyntaxArena × HirArena × SummaryTable × CgirArena × ArtifactIndex × RuntimeBlueprintIndex × ProjectionTable × WatchTable
CompilerGraph  = ProjectGraph ⊳ compiler view
```

This reuses the language's existing conceptual center instead of inventing a second one. The project graph is just another explicit costate. Parser, resolver, type checker, optimizer, serializer, materializer, package planner, and runtime-blueprint generator are all optics over that costate. Source files, HIR views, CGIR views, diagnostics, lock snapshots, generated outputs, and target runtimes are projections.

#### 22.3.1 Why a single memory-mapped graph file is attractive

The compiler's hot working set is metadata-heavy and pointer-rich. A memory-mapped project graph file with compiler-controlled layout gives three benefits that align well with the rest of the language design.

First, it gives the compiler stable identities and revisions across process restarts without rebuilding the entire world from text on every invocation. Second, it keeps hot summaries and edge tables in one place with explicit layout, which is much closer to the language's data-oriented ethos than a maze of process-local hash maps and cache directories. Third, it makes snapshots, replay, and direct tool attachment much simpler because every participant can agree on one project-graph revision.

The storage model should stay conservative and deterministic.

- one serial commit path,
- many readers,
- append-oriented mutation,
- fixed-format hot records,
- explicit schema versioning,
- periodic compaction,
- and crash recovery driven by journal + checkpoint rather than best-effort cache regeneration.

That discipline is especially valuable for a self-hosted compiler, where the compiler's own internal state becomes part of the language's operational contract.

#### 22.3.2 The graph should remain hotter than the artifacts

A project graph is not a generic blob store. Its job is to keep the metadata and dependency structure hot. Large immutable outputs are better referenced than embedded.

| Keep graph-native | Usually sidecar / content-addressed |
|---|---|
| source text chunks, interned strings, spans, syntax/HIR/CGIR nodes, summaries, diagnostics, module-interface summaries, watch state, runtime blueprints, target/profile metadata | object files, archives, debug symbol bundles, large baked assets, generated tables too large for hot metadata |

This keeps the mapped file small enough to stay operationally pleasant while preserving one authoritative graph.

#### 22.3.3 Many IRs stay manageable because they are projections, not rival truths

At this point in the book, the reader has seen a growing list of representations: source text, syntax, HIR, typed HIR, summary tables, CGIR, fused CGIR, interface artifacts, generated Rust, LLVM, benchmark identities, debug metadata, and project-graph projections. That can look like architectural drift unless the book says plainly what the relationship is.

The relationship is the same one introduced in Chapter 4:

> the compiler has many representations, but one authoritative graph.

The compiler therefore should not attempt to squash all phases into one mega-IR. Each phase has a different invariant and a different audience.

- source text preserves authored structure and comments,
- HIR preserves name and cursor meaning,
- summaries preserve legality facts,
- CGIR preserves optimization structure,
- backend forms preserve target-facing execution shape,
- interface and artifact forms preserve distribution and reuse boundaries.

What keeps this practical is shared identity and provenance. Node ids, region ids, type ids, artifact keys, and revision ids belong to the `Project` graph, not to one local pass. That is why self-hosting, distributed build, debugger attribution, and coding-agent workflows can all speak about “the same node” even when that node is being viewed through different projections.

#### 22.3.4 Compiler passes should be described as graph transactions

Earlier chapters already describe parser, lowering, summary construction, fusion, and artifact emission as optics over compiler-owned costates. The full project model makes the next step natural: each pass is a **transaction over `Project.build`**.

That means a pass has five recognizable stages.

1. select the affected graph region,
2. query the graph and compute derived facts,
3. synthesize replacements or derived projections,
4. validate the relevant invariants,
5. commit a new project revision.

This description is not just philosophical. It clarifies how several other parts of the language fit together.

- **incremental compilation** becomes dependency- and region-aware transaction invalidation;
- **distributed build** becomes scheduling over subgraph closures;
- **debugging and profiling** can attach to revisions and provenance sets rather than to transient pass-local data structures;
- **self-hosting** becomes more tractable because compiler passes are ordinary staged graph programs rather than privileged compiler-only magic.

#### 22.3.5 Why the direct protocol and the native query subset belong together

The direct graph protocol should therefore not be understood as merely a replacement for file watching. It is the transport for three closely related things:

- read-mostly project queries expressed in the ordinary optic subset,
- mutating compiler/tool transactions over `Project.build`,
- and projection/materialization requests for text, interfaces, artifacts, and crash or benchmark capsules.

This is one reason the book continues to insist that the projection filesystem is useful but secondary. Files are excellent human surfaces. Transactions, semantic queries, and provenance-aware revisions are better expressed over the graph directly.

### 22.4 Projection filesystems: useful, but secondary

Once the compiler graph is authoritative, a file tree becomes one compatibility surface rather than the semantic center.

A projection filesystem can mount selected views of the graph for tools that expect files:

```text
/optic/src/...          authored source text projection
/optic/hir/...          pretty or structured HIR projection
/optic/cgir/...         graph projection
/optic/diag/...         current diagnostics projection
/optic/pkg/...          package/workspace/build projection
/optic/gen/...          generated artifacts and interfaces
/optic/tool/<name>/...  tool-specific projections
```

This is a strong idea because it preserves compatibility with editors, grep-like tools, indexers, diff tools, and existing file-oriented workflows while keeping the graph itself primary.

But the filesystem surface should remain **secondary** for two reasons.

First, file protocols are weak. A write to a text file does not say whether the caller wants an atomic text patch, a semantic rename, a projection re-materialization, or a transaction over several related regions. Second, user-space filesystem layers inevitably add mediation overhead and liveness hazards that do not belong on the compiler's critical path.

So the right stance is:

- projection filesystem for compatibility and human convenience,
- direct graph protocol for primary tool integration,
- and ordinary exported text materialization for environments where mounting is unavailable or undesirable.

#### 22.4.1 Writable versus derived projections

Not every projection should be writable.

- authored text projections are writable;
- selected build-root values may be writable through structured edits;
- HIR, CGIR, diagnostics, and most summary projections are derived and read-only;
- generated outputs are re-materialized, not hand-edited.

This matters because the optic analogy should remain honest. A writable projection must have a clear reinsertion path. A read-only diagnostic stream does not.

### 22.5 Direct tool protocol over the graph

The primary integration surface for editors, language servers, build tools, test runners, indexers, and coding agents should be a concrete protocol over the compiler graph.

A good default is a local binary protocol over a Unix domain socket or platform equivalent, with batched requests and explicit graph revisions. The important design choice is not the exact wire format. It is the transactional model.

#### 22.5.1 Protocol principles

1. **Serial mutation, cheap reads.** One ordered commit path keeps the graph deterministic. Readers attach to revisions or snapshots.
2. **Batch by default.** A tool should be able to submit a text patch, ask for updated diagnostics, request affected summaries, and subscribe to a watch in one batch.
3. **Stable identities.** Nodes, projections, and revisions need durable ids so tools can correlate changes across requests.
4. **Capability-aware access.** A formatter, an indexer, and an AI agent do not need identical write powers.
5. **Projection-aware replies.** Tools can ask for text, structured data, materialized artifacts, or graph-native handles.

#### 22.5.2 Representative request families

| Request family | Purpose |
|---|---|
| session / capability negotiation | open workspace, negotiate protocol version, declare requested powers |
| text and patch operations | read or patch authored text projections |
| semantic queries | fetch HIR/CGIR/summaries/diagnostics/provenance |
| build and staging queries | evaluate package roots, build plans, stageable subgraphs, artifact plans |
| watch and invalidation | subscribe to revision deltas, changed projections, diagnostic updates |
| materialization | export source view, interface artifact, generated output, or reproducible workspace snapshot |
| explanation | ask why a node is dynamic, why a fusion failed, why a grade bound was violated |

#### 22.5.3 Why the protocol should be primary even if a projection filesystem exists

A direct protocol can express operations that files cannot express cleanly: graph revision selection, semantic explanation, batched transactions, stable node ids, structured diagnostics deltas, materialization policies, and provenance queries. It is also the natural place for AI-agent-friendly operations such as "apply this patch, tell me what invalidated, and return ranked next repairs".

That is why the projection filesystem should be treated as an adapter, not as the foundation.

### 22.6 The init tool should be graph-first and agent-aware

The first contact most users have with a language toolchain is not the optimizer or the debugger. It is the command that creates the project skeleton. That command therefore deserves architectural seriousness.

For Optic, the equivalent of `cargo init` or `go mod init` should not be a thin file copier wrapped around a handful of inert text templates. It should be a **guided project transaction** that creates the initial `Project`, `BuildRuntime`, `RuntimeBlueprint`, and `AppWorld` regions in a form the rest of the toolchain can immediately understand.

A first-party `optic init` should therefore do more than choose a package name. In human-oriented wizard mode it should help the author describe:

- the intended domain or blueprint family,
- the initial package/workspace topology,
- the expected `RuntimeFamily` and target profiles,
- the initial `AppWorld` hot data layout,
- the major host or foreign boundaries,
- the desired diagnostics, replay, and benchmark posture,
- and whether the project begins as a library, service, tool, mixed workspace, or subsystem.

The output of that interaction should be native Optic declarations and graph regions, not a second configuration vocabulary. At minimum the tool should be able to generate:

- `package` and `workspace` declarations,
- an initial `RuntimeBlueprint`,
- one or more `data AppWorld` skeletons,
- boundary-contract stubs for expected interop,
- benchmark and replay scaffolding with semantic `PerfKey`s,
- and procedural domain templates parameterized by the answers the user just gave.

This is where the earlier architecture pays off. The same domain blueprints that later guide optimizations and agent workflows can guide initialization. A service template, a browser-subsystem template, and a compiler-tooling template do not need separate template languages. They need one generator that emits ordinary language values and projections.

Just as importantly, the init path should be available in both **human mode** and **agent mode**. Interactive wizarding is the right default because most first projects are underspecified. But a headless mode should accept structured intent data and use the same graph transaction internally, so automation does not become a second, weaker entry point into the ecosystem.

The design consequence is broader than ergonomics. If the init tool emits native declarations, benchmark scaffolds, boundary stubs, and runtime-family metadata from the beginning, then large-codebase agents, package tools, and later self-hosted compiler components start from typed project intent instead of trying to infer it from boilerplate. That is exactly the kind of early structure that makes the rest of the language easier to automate without adding another special-purpose DSL.

### 22.7 Coding-agent-friendly diagnostics as a design inheritance

Compilers are also where the agent-friendly diagnostic architecture returns as a systems advantage. If the language is to host its own compiler and broader ecosystem tooling, its diagnostic story must already be good enough to support iterative automated development.

That is why the earlier discipline around stable codes, structured evidence, and ranked repairs should not be thought of as only "tooling". It is part of the language's self-hosting readiness.

### 22.7.1 A repository agent operating system should be treated as a first-party graph client

Once the project graph, direct protocol, init flow, and diagnostic schema exist, a repository-local agent operating system stops looking like an external convenience layer and starts looking like another serious graph client. The same graph that supports editors, debuggers, package tools, and self-hosted compiler passes can support a coordinating agent, a small family of specialists, explicit work packets, shared and role-local memory, and generated context indexes. The important design rule is that this agent layer should **reuse** the graph-native tooling story rather than inventing a second semantics for automation.

That is why the repository package accompanying this book ships a canonical `AGENTS.md`, tool-specific wrappers, specialized agent files, explicit memory ledgers, and a maintenance script that regenerates context indexes from checked-in state. The files are not an afterthought. They are a worked example of how graph-native tooling, agent-oriented diagnostics, and direct protocol thinking can be turned into a repeatable collaboration system for both humans and coding agents. Appendix K records that operating system in compact reference form.


### 22.7.2 Durable agent memory belongs in the graph; scratch memory does not

The repository agent operating system is strongest when it shares the same semantic center as the compiler and tooling. That does **not** mean every trace of agent activity should become graph truth. It means the durable, shared, query-worthy subset of maintenance knowledge should be represented alongside the rest of the project graph.

Good graph-native candidates include:

- accepted architectural decisions,
- validated diagnostic-repair records,
- benchmark and replay explanations,
- task state that spans revisions,
- boundary exceptions and local audit notes,
- and other facts that later humans or agents genuinely need to query.

Poor graph-native candidates include raw scratchpads, long transcripts, speculative notes, and tool-specific hidden state. Those belong in sidecars or private tool memory because the project graph should remain a typed semantic database, not a dump of every conversation the repository ever triggered.

### 22.7.3 Failed patches are first-class negative knowledge

One category deserves explicit treatment: prior failed patches. Large codebases waste enormous effort retrying similar bad ideas because the only record of failure lives in old chat threads, abandoned branches, or one person's memory.

The right response is to treat failed patches as **typed negative knowledge**. A failed patch record should say what was attempted, which nodes or regions it touched, which revision and target profile it was evaluated against, which evidence proved the failure, and whether the result is still believed to matter. It should never become a blanket “never do this again” rule.

That gives the toolchain and future agents a much better default. Query→fix synthesis can de-rank repair candidates that already failed for the same reason, while human review can inspect the evidence rather than trusting folklore. Appendix K records the repository-local file-level version of this idea; the long-range `ProjectGraph` version is to keep the durable summary in the graph and the large patch diff or transcript in sidecar storage.

### 22.8 Governance and invariants

The book closes the main text with a governance point because ambitious systems languages fail as often by losing their discipline as by losing their semantics.

Three governance rules matter most.

- No feature enters the language without a lowering story.
- No optimization enters the compiler without a provenance and benchmark story.
- No milestone is declared complete without repository evidence.

A mature language also needs explicit policy for the parts that are easy to postpone and hard to retrofit: editions, source compatibility, package compatibility, module artifacts, binary interfaces, migration tooling, and debugger/profiler stability through optimization. Those pressures are common late-stage failure points in other ecosystems, and they are gathered into a concrete maturity chapter in Chapter 27 and a standing proposal checklist in Chapter 28.

### 22.9 Final transition

The appendices that follow are not leftovers. They are the working reference material that turns the book's conceptual narrative back into day-to-day implementation practice.

---

### 22.10 Detailed implementation reference: compiler passes as optics over explicit IR costates

Compilers are not merely a future bootstrap target; they are also one of the clearest demonstrations that the optic model handles richly structured transformation systems. The detailed examples below restore that perspective.

A compiler is a program that transforms programs. Every compiler pass is naturally an optic over a typed IR costate.

#### 22.9.1 The Compiler IR as Costate

```rust
data Module {
    functions:    SoA<FunctionDef>,
    instructions: SoA<Instr>,         -- flat instruction arena
    types:        SoA<TypeInfo>,
    dominators:   SoA<DomTreeNode>,   -- pre-computed for analysis passes
    use_def:      SoA<UseDefChain>,   -- pre-computed use-def chains
    metadata:     SoA<PassMetadata>,  -- per-pass scratch storage
}
```

#### 22.9.2 Dead Code Elimination as a Traversal + Prism

```rust
optic DeadInstr: GradedPrism<Module, Instr,
    CacheGrade<2> + CompileTimeGrade>
{
    preview m => {
        let instr = m.instructions[m.id];
        if m.use_def[m.id].uses.is_empty() && instr.has_no_side_effects() {
            Some(instr)
        } else {
            None
        }
    }
    review  instr => instr
}

-- DCE pass:
module
    .query(AllInstrs *** DeadInstr)
    .map(|(all, dead)| all.remove(dead))
    .drive();
```

#### 22.9.3 Constant Folding as Lens + Map

```rust
optic ConstFold: GradedOptic<Module, Instr, CacheGrade<2> + CompileTimeGrade> {
    get  m => {
        let instr = m.instructions[m.id];
        match instr {
            Instr::BinOp(op, Const(a), Const(b)) => Instr::Const(eval_op(op, a, b)),
            other => other,
        }
    }
    put  (m, folded) => { m.instructions[m.id] = folded; }
}
```

#### 22.9.4 Register Allocation as Optic Over Live Ranges

Register allocation is the assignment of virtual registers (unbounded) to physical registers (bounded, 16 on x86-64). The optic model:

```rust
data LiveRangeArena {
    ranges:    SoA<LiveRange>,    -- (virtual_reg, start_point, end_point)
    conflicts: SoA<ConflictSet>,  -- which virtual regs interfere
    colors:    SoA<Option<Reg>>,  -- assigned physical register (None = spill)
}

optic ColorRegister: GradedPrism<LiveRangeArena, Reg,
    CacheGrade<3> + CompileTimeGrade>
{
    preview lra => {
        let range = lra.ranges[lra.id];
        let available = ALL_REGS - lra.conflicts[lra.id].used_regs();
        available.first()
    }
    review  reg => reg
}
```

When `ColorRegister.preview` returns `None`, the register is spilled to the stack. The `None` path is itself an optic:

```rust
optic SpillToStack: GradedOptic<LiveRangeArena, StackSlot, CacheGrade<1>> {
    get  lra => lra.stack_frame.allocate_slot(sizeof(lra.ranges[lra.id].type_))
    put  (lra, slot) => { lra.colors[lra.id] = None; lra.stack_slots[lra.id] = slot; }
}

let alloc_pass = ColorRegister >>> SpillToStack;
-- If ColorRegister succeeds (Some(reg)), SpillToStack is skipped (prism law)
-- If ColorRegister fails (None), SpillToStack handles the spill
```

This is register allocation expressed as composable optics. The grade `CompileTimeGrade` ensures the entire allocation pass runs at compile time, not at runtime of the compiled program.

---

### 22.11 Detailed implementation reference: self-hosting ladder and differential trust loop

The self-hosting plan belongs next to the tooling chapter because it is a discipline of validation, not just an implementation milestone. The following material records the explicit trust-chain and bootstrap-ladder guidance needed to keep that discipline honest.

#### 22.10.1 Bootstrapping ladder

| Stage | Compiler impl | Output |
|-------|--------------|--------|
| S0 | Rust prelude compiler | Rust source |
| S1 | Optic front-end libraries compiled by Rust | Rust source |
| S2 | Optic parser, HIR, diagnostics written in Optic | Rust source |
| S3 | Optic type checker, CGIR, optimizer written in Optic | Rust source |
| S4 | Mixed: most passes in Optic, small Rust shell | Rust + native |
| S5 | Fully self-hosted with native backend underway | Native + Rust fallback |

#### 22.10.2 Compiler passes as optics

Every compiler pass is itself an optic over a compiler costate:

| Pass | Costate | Focus |
|------|---------|-------|
| Parser | `SourceFile` | `Ast` |
| HIR lowering | `HirArena` | `HirItem` |
| Type check | `TypeckCtx` | `TypedHirItem` |
| CGIR construction | `CgirGraph` | `CgirNode` |
| Fusion | `CgirGraph` | `CgirNode` (rewrite) |
| Codegen | `RustAstArena` | `RustItem` |

This is only valuable once the prelude IR and summary machinery are robust. Do not force early self-description.

#### 22.10.3 Trust chain

- Reproducible builds of the Rust prelude compiler
- Golden snapshot suites shared across Rust and Optic implementations
- Differential testing between Rust-hosted and Optic-hosted compilers
- Stable diagnostic codes across both implementations
- A frozen bootstrap seed for release lines

#### 22.10.4 Self-hosting exit criteria

##### 22.10.4.1 Differential validation loop

Self-hosting should be treated as a translation-validation problem, not as a declaration of maturity. The minimum practical loop is:

```text
1. compile compiler sources with seed Rust implementation
2. compile same sources with Optic implementation
3. compare diagnostics JSON for known-good and known-bad suites
4. compare generated Rust for canonical examples
5. compare benchmark deltas against seed tolerances
6. only then consider the Optic compiler a valid successor for that revision
```

This keeps self-hosting grounded in observable equivalence rather than in the prestige of saying the compiler is written in its own language.

The compiler may be called self-hosting only when:

- It can compile itself from a clean checkout
- The generated compiler passes the same regression and benchmark suite as the seed
- Diagnostics remain stable enough for coding agents to work against either implementation
- The trusted Rust shell is small, audited, and shrinking

---

# Part V — Validation, Rationale, and Long-Term Maturity

Part V closes the loop by changing the mode of explanation. Parts I–IV established the semantic core, the narrow compiler, the full-language growth path, and the domain applications. This part assumes that machinery and asks a different question: which of those choices are worth freezing, how are they validated quantitatively, what ecosystem policies prevent the language from succeeding locally while failing operationally later, and what standing checklist future feature proposals must pass before they are allowed to reshape the language. The goal is to preserve the book's substance without re-walking the full semantic ladder a second time.

## 23. Language Design Rationale and Rejected Alternatives

This chapter is a decision ledger, not a second tutorial for Part III. The earlier chapters already explained how prisms become branches, how traversals become vectorization candidates, how coinduction becomes event-loop structure, how staging becomes ordinary graph evaluation over `BuildRuntime`, and how the LLVM backend consumes summaries and regions. Here the same topics are revisited only to record the irreversible design choices, the alternatives that were rejected, and the specific cost or legality argument that made the choice stick.

The practical reading rule is simple: when a section in this chapter names a feature already developed in Parts III or IV, read it as a compact justification and tradeoff record, not as a reintroduction of the feature from first principles.

### 23.1 The master rule: no abstraction without a lowering story

The language accepts a high-level abstraction only when four questions all have good answers.

- **Can the semantics be stated without hidden ambient behavior?** Hidden behavior makes grades, replay, and optimization unverifiable.
- **Can the compiler carry the needed information explicitly?** If the optimizer must rediscover the abstraction later, the abstraction is too weak.
- **Is there a legality test before low-level exploitation?** Otherwise the backend becomes heuristic rather than proof-directed.
- **Is there a measurable machine consequence?** If not, the abstraction may still be valid, but it is not yet justified in a systems language core.

This rule is what keeps the language from turning into a catalog of elegant but operationally vague concepts.

#### 23.1.1 Why mixed-domain collisions are the real extensibility test

A new abstraction should not be judged only in the domain where it was invented. The harder and more revealing test is whether it survives **domain collisions** without forcing the compiler to invent ad hoc side rules. A frame-budgeted renderer that consumes network state, a transactional storage wrapper over a legacy C engine, or an experimental geometric kernel inside an otherwise ordinary traversal are better tests of architectural integrity than any isolated success case.

This is why the book keeps insisting on one summary model. Grades continue to express quantitative budgets, boundary contracts continue to express qualitative legality, and CGIR continues to be the one structural graph even when several domains meet. If a proposed feature only works in isolation and falls apart under cross-domain composition, it is still a research result, not yet a language feature.

### 23.2 Why `Project` sits above `BuildRuntime` and `Runtime`

#### 23.2.1 The accepted design

The language uses one durable root above both build-time and live execution:

```text
Project = ProjectGraph × RuntimeBlueprintSet

BuildRuntime = Project.build
Runtime      = Project.run[target, instance]
Runtime      = AppWorld × HostContext × ControlRuntime
```

- `ProjectGraph` is the durable source of truth: source text, package/workspace declarations, module boundaries, compiler summaries, projections, generated artifacts, and runtime blueprints.
- `BuildRuntime` is the compiler-facing build view over that project root.
- `Runtime` is the live instantiated program world for one target and one running instance.

#### 23.2.2 Semantic obligation

A systems language that supports native build execution, graph-resident tooling, replay, projections, and live runtimes eventually needs to answer a simple question: what is the one thing all of those views belong to? If build time and run time are modeled as completely separate roots, the architecture has to keep rediscovering how source declarations, artifacts, runtime blueprints, and live program instances relate to one another. `Project` makes that relationship explicit.

#### 23.2.3 Compiler artifact

`RegionSet`, `PathLift`, `OpticSummary`, `TargetProfile`, projection tables, and artifact indices all assume that reads and writes can be expressed as paths from a root costate. Once the build graph becomes first-class, that root is no longer just `Runtime`; it is `Project`, with `BuildRuntime` and `Runtime` as disciplined projections.

#### 23.2.4 Backend and tooling legality condition

The backend may exploit read/write, determinism, and layout facts only when it knows which regime a path belongs to. A compile-time route compiler may read `Project.build.package_graph`; a hot runtime traversal over `app.positions[*]` may not silently read from build-only state; a filesystem or socket handle in `Runtime.host` must not be mistaken for durable project metadata. The enclosing `Project` root keeps these boundaries explicit without splitting the language into separate semantic worlds.

#### 23.2.5 Machine consequence

- source, build graph, artifacts, and runtime blueprints have one durable identity space;
- direct tool protocols and projection filesystems can speak in terms of project revisions instead of ad hoc cache files;
- staging can cache against project-root hashes rather than reconstructing hidden build context;
- multi-target outputs and self-hosted toolchains remain organized under one graph rather than one directory tree plus several caches.

#### 23.2.6 Rejected alternative: two peer roots (`Runtime` and `BuildRuntime` only)

The obvious alternative is to keep `Runtime` and `BuildRuntime` as separate semantic roots and relate them only operationally. That looks simpler at first, but it creates a recurring architecture problem: source declarations, build plans, artifacts, runtime blueprints, and live instances stop sharing one typed identity space. The language then has to reintroduce that relationship through conventions, sidecar metadata, or tool-specific glue.

`Project` avoids that without requiring a large new theory. It is the minimal enclosing extension that keeps the compiler graph, native build system, projection filesystem, direct tool protocol, and live runtime under one optic-friendly model.

#### 23.2.7 Why graph authority does not demote text or ordinary tooling

Graph authority exists for the compiler, not to abolish the file-based human workflow. The accepted design keeps text as the primary human editing and review surface, and treats LSP adapters, materialized source trees, and projection filesystems as permanent compatibility obligations rather than temporary migration tools. The graph becomes the stronger semantic center underneath that workflow, not a declaration that text is obsolete.

#### 23.2.8 Why checked focusing and elision are allowed but ambient dependency injection is rejected

The language does allow a more concise surface over explicit roots, but only through checked focusing and elision. That is acceptable because the compiler can lower the focused form back into explicit root-relative paths before summaries and legality checks are finalized. Ambient dependency injection is rejected because it would hide the true root from the same static artifacts the rest of the architecture depends on.

### 23.3 Why graded optics instead of effect rows or evidence-passing alone

#### 23.3.1 Accepted design

Ordinary systems transformations are modeled as optics over explicit runtime state, with grades attached to those optics.

#### 23.3.2 Semantic obligation

The language needs to capture three things at once:

1. what part of the context a transformation needs,
2. what part it rewrites,
3. what resource contract that transformation carries.

Effect rows are good at the first item in a coarse sense, but usually poor at the second and third unless heavily extended. Evidence passing gives an implementation technique for handlers, but not a structural program model for memory layout, alias sets, or traversal fusion.

#### 23.3.3 Compiler artifact

The accepted artifacts are:

- `OpticSummary` for read/write/grade/determinism facts,
- CGIR nodes that keep composition explicit,
- `Runtime` paths rather than abstract effect names.

#### 23.3.4 Backend legality condition

A backend can exploit an effect description only if it knows where in memory or which host channel is involved. The path `app.positions[*]` can become TBAA metadata and direct GEP indexing. An effect row `State + IO + Time` cannot.

#### 23.3.5 Machine consequence

The design removes a large class of runtime evidence dictionaries and replaces them with direct field access and static summaries. In the hot path, that means fewer indirect calls, fewer opaque handler lookups, and fewer optimization barriers.

#### 23.3.6 Rejected alternative: handler-evidence as the default runtime representation

General effect handlers are retained as an optional future layer for genuine control effects, but they are not the foundational representation for ordinary state/context/resource effects. The reason is practical: a web server packet parse, an ECS physics update, and a page-table walk are all more profitably understood as structured observations/updates over concrete regions than as requests serviced by ambient handlers.

### 23.4 Why the grade algebra is a semiring, not a lattice, trait bag, or ad hoc rule table

#### 23.4.1 Accepted design

The language uses a product of dimension-specific semirings.

#### 23.4.2 Semantic obligation

The language must distinguish at least two modes of composition:

- sequential composition, where some costs accumulate;
- parallel/product composition, where some costs merge differently.

A lattice alone cannot say "these two latency budgets add when sequenced" while also saying "these two branch alternatives merge by worst case." A trait bag has no algebra at all. An ad hoc rule table becomes impossible to reason about compositionally. The clean mental model is to treat the semiring as the language's **budget sheet** and `BoundaryContract` as its **zoning law**. Cache, latency, bandwidth, compile-time work, ownership, liveness, session, and security behave like quantities that compose. ABI, unwind, callback, thread-affinity, address-space, volatility, allocator, and privilege facts behave like qualitative boundary permissions. They remain static, but they do not belong inside the semiring.

#### 23.4.3 Compiler artifact

`ConcreteGrade`, later `GradeExpr`, and `GradeConstraintSolver` embody the algebra explicitly. Every pass sees the same carrier and the same operations.

#### 23.4.4 Backend legality condition

A backend can use a grade only when the corresponding dimension's composition law is stable and checkable. For instance:

- `LatencyGrade` must compose additively in sequence because the scheduler and deadline checks rely on it.
- `OwnershipGrade` must become at least as restrictive under composition because alias-safety proofs rely on monotonicity.
- `CacheGrade` may use approximate upper bounds, but the approximation must be monotone and stable under fusion.

#### 23.4.5 Machine consequence

The semiring structure makes optimization *proof-directed* rather than folklore-directed. A fused loop is legal because its grade is computed, not because the compiler "thinks" fusion is good.

#### 23.4.6 Rejected alternative: one universal "cost" scalar

A single scalar cost would force incomparable resources into one number, making it impossible to tell whether a composition is bad because of cache pressure, blocking I/O, ownership, or latency. The product-of-dimensions design keeps reasons separate while remaining compositional.

#### 23.4.7 Why grade elision and inference belong in the language rather than only in the checker

The semantic core should keep grades explicit, but the full language should still allow programmers to omit dimensions the compiler can reconstruct exactly or to specify only the dimensions that matter operationally at a given boundary. This is not a retreat into implicitness. It is the grade-side analogue of checked focusing over `Runtime` and `Project`: the omitted structure is acceptable only because the compiler can recover it, serialize it, and explain it deterministically. Hidden resource defaults are rejected; checked elision and explicit inference are accepted.

### 23.5 Why typed errors and prisms replace exceptions in the core

Part III already established the operational picture: a prism is value-level partiality that survives into CGIR as explicit branch structure. The design decision recorded here is narrower. The language keeps ordinary failure in the graph as `Option`, `Result`, and prism composition, and refuses to make hidden exception edges the default control mechanism.

That choice buys three concrete things at once.

- The semantic shape stays local and inspectable: success, failure, and reinsertion are all explicit.
- The compiler retains branch structure long enough to drive fusion legality, branch hints, mask lowering, and replay classification.
- The backend never has to guess whether a non-local exit exists outside the visible graph.

The rejected alternative is not “all control effects forever.” It is specifically **ambient exception edges for ordinary failure**. Once failure stops being explicit, CGIR loses one of the main properties it was created to preserve: source-level control shape that still means something after optimization.

### 23.6 Why general handler-based algebraic effects are optional, not foundational

The language does not reject richer control effects forever; it rejects them as the *foundational* representation of ordinary systems work. The core problems this book is solving first—layout, locality, alias safety, host boundaries, staging, replay, and graph-level fusion—do not require continuation capture to be present everywhere.

That is why the accepted design keeps the primary artifacts small: `Runtime`, `RegionSet`, `OpticSummary`, CGIR, and later reserved `ControlRuntime` / control-oriented IR nodes. If resumptions, prompts, or handler stacks arrive later, they arrive as a deliberate extension over that base, not as the execution model that every hot loop is forced through from the beginning.

The rejected alternative is a control-first language where everything is lowered through handlers or continuation transforms. That path would front-load runtime and backend complexity into the very place where the current design wants the simplest loops and the clearest alias proofs.

### 23.7 Why prisms are the right branch form

As argued in Chapter 13, prisms are the smallest optic form that simultaneously preserves typed partiality, explicit reinsertion, and backend-visible branch structure. The decision here is therefore not about syntax preference. It is about choosing the branch representation that keeps the most useful static information alive.

The accepted path is:

- `preview` expresses the test,
- `review` expresses the reinsertion/update path,
- branch bias remains available as zero-cost metadata,
- and the backend can still choose between branch hints, predication, or masked lowering when the target and control shape justify it.

The rejected alternative—external boolean filters as the dominant branch abstraction—loses the connection between success and reinsertion. It can express “keep or drop,” but it cannot by itself express the lawful update relationship that makes prisms composable with the rest of the optic hierarchy.

### 23.8 Why traversals are the vectorization form

Chapter 13 already made the main bridge explicit: traversals are the semantic form of same-shape bulk transformation, and that is exactly the shape a vector backend wants to see. Part V only needs to record why this design won over the obvious alternative.

The accepted design keeps traversals explicit because they carry the legality facts that matter most for SIMD:

- lane independence,
- regular stride,
- homogeneous element shape,
- and update shape preservation.

That is stronger than saying “the optimizer might spot an iterator pattern.” It gives the backend a typed reason to choose scalar lowering, predicated lowering, packed vector code, or a split compaction phase.

The rejected alternative is backend-only vectorization heuristics over already-lowered loops. That path can still optimize, but it loses the earlier semantic evidence the language has already paid to compute.

### 23.9 Why coinduction is the event-loop form

The detailed mechanics live in Chapter 14. The design decision recorded here is that long-lived, observation-driven pipelines should stay explicit as coinductive graph structure rather than collapse immediately into callback registries or future state machines.

That buys the language three things.

- Liveness and backpressure become properties of explicit nodes rather than folklore around a runtime.
- Ring-based or queue-based lowering can remain provenance-preserving.
- Replay and observability have a natural place to attach because the event structure is still visible when the optimizer and backend see it.

The rejected alternative is to let `async/await`, callbacks, or framework schedulers become the only visible representation of live computation. Those models can be good surface syntax later, but they are not the right foundational graph form if the language wants coinduction, replay, scheduling, and host interaction to remain one connected story.

### 23.10 Why compile-time execution is ordinary graph evaluation, not a macro sublanguage

The mechanics of `BuildRuntime`, stageability, and `CompileTimeGrade` were already established in Chapters 4 and 14. What Part V needs to record is the architectural payoff of that choice.

The accepted design says that compile-time work is ordinary optic evaluation over the build projection of `Project`, optionally bracketed by `stage {}` as a cache-key and proof boundary. The compiler may lower user-facing graph queries into an internal QIR, but it does not expose a second user-facing macro or query language for this purpose. Staging is therefore not a narrow optimization trick. It is the universal glue that lets one semantic graph serve local specialization, dependency resolution, build planning, generated bindings, artifact materialization, and later self-hosted compiler execution.

That buys a single semantic story for:

- package and workspace declarations,
- build plans,
- generated bindings,
- staged specialization,
- and later self-hosting compiler passes.

The rejected alternative is a pile of separate compile-time mechanisms: token macros, build scripts, special processors, and plugin-specific rewrite APIs. Those systems can all be powerful, but they also create a second compatibility surface. Optic's design intentionally front-loads the harder work so that compile-time execution stays under the same summaries, grades, provenance, and artifact discipline as the rest of the language.

### 23.11 Why CGIR sits above SSA

#### 23.11.1 CGIR as a recovered structural defense

Part II already made the mechanics visible. The design rationale adds the historical interpretation: CGIR exists because earlier compiler traditions repeatedly taught the same painful lesson — lowering away structure too early forces the optimizer to reconstruct domain intent from generic control flow after the most valuable legality and provenance facts have already been blurred. CGIR is therefore not only a convenient IR. It is a deliberate defense against that recurrent failure mode.

This choice is already operationally justified in Part II and exploited throughout Part III. The remaining rationale is simply to state the irreversible trade: the optimizer is allowed to lower into SSA only after it has finished using the graph structure that the source language made explicit.

The accepted design keeps CGIR as the primary optimization form because it preserves exactly the things the backend still needs a little longer:

- optic composition and products,
- explicit traversal and prism structure,
- staging and coinductive boundaries,
- and provenance that survives fusion.

The rejected alternative—early SSA plus later recovery—front-loads information loss. That cost is not theoretical; it shows up immediately in weaker fusion legality, harder source attribution, and more fragile diagnostics.

### 23.12 Why Rust transpilation precedes LLVM emission

The earlier chapters already made the practical argument: a readable Rust path is the semantic microscope for the first compiler. Part V only needs to make the decision boundary explicit.

The accepted path is:

1. use Rust output to validate that summaries, cursor lowering, and fusion produce the intended loop shape,
2. build the LLVM backend against that reference,
3. and make LLVM authoritative only after translation validation and performance validation both pass.

The rejected alternative is LLVM-first bring-up. It is attractive for prestige and misleading for engineering. A native loop can be fast and still be semantically wrong; readable Rust keeps early debugging cheap and keeps the implementation honest.

### 23.13 Why SoA is the default memory law

Part I and Chapter 16 already explained the arithmetic. The decision ledger entry is therefore short: the language defaults to the layout assumption that best matches its target workloads and its optimizer's strongest proofs.

SoA won because it keeps hot fields contiguous, keeps cache and SIMD reasoning structural, and lets `RegionSet` plus TBAA reflect the programmer's real dataflow. AoS remains available where utilization is high and hybrid AoSoA islands remain important, but they are explicit deviations from the law rather than the law itself.

The rejected alternative—AoS by default—would make the language's most important performance story a library convention instead of a language-level decision.

### 23.14 Why ownership is graded instead of expressed as a separate borrow syntax

The detailed ownership story was established in Part I and extended in Chapter 16. The accepted design keeps ownership inside the same grade and summary machinery because that is the only way the same proof can feed both safety and optimization.

This matters most in two places:

- ordinary exclusivity and alias legality,
- and later fractional or partitioned proofs for multicore products.

A separate borrow language would duplicate mechanisms and force users and tools to reconcile two static analyses that both describe the same underlying facts. Keeping ownership in the grade story means the same summary can justify type checking, parallel legality, and backend alias assumptions.

### 23.15 Why diagnostics are deterministic and agent-oriented

#### 23.15.1 Diagnostics are the public telemetry of the semantic core

The narrow compiler chapters already showed the record format. The rationale to record here is stronger: deterministic diagnostics are one of the language’s main proofs that front-loaded semantic structure is doing real work. If summaries, regions, grades, and provenance are explicit enough to drive optimization, they should also be explicit enough to produce stable evidence, ranked repairs, and source-faithful explanations without fallback to compiler folklore.

Part II already specified the record format and validation discipline. The design rationale here is simply that the language is treating diagnostics as part of the semantic interface, not as after-the-fact prose.

That buys the project something larger than “better error messages.” It gives the language:

- stable machine-consumable evidence for coding agents,
- a durable explanation layer for graph-native tooling,
- and a migration surface that can survive optimizer, backend, and edition changes.

The rejected alternative is prose-only diagnostics that are pleasant in isolation but too unstable to serve as a protocol between the compiler, IDEs, package tools, and automated repair loops. A second rejection sits just behind it: diagnostics that expose only the formal law without translating it into the machine-facing consequence. The accepted design insists on both layers at once.

### 23.16 Rejected alternatives matrix

- **Runtime model:** explicit `Runtime` root, not ambient environment, because region-level effect/coeffect accountability would otherwise be lost.
- **Resource model:** semiring grades, not trait bags or comments, because resource composition must remain formal and machine-usable.
- **Error model:** prisms and `Result`, not hidden exceptions, because control edges must stay visible to CGIR.
- **Iteration model:** traversals, not generic iterator abstractions alone, because structure must arrive early enough for grade-guided fusion.
- **Infinite behavior:** coinduction, not `async/await` as the foundation, because event structure should not collapse into runtime state machines too early.
- **Specialization:** staging, not macros or templates, because syntax expansion is not the same as typed specialization.
- **IR structure:** CGIR above SSA, not early SSA lowering, because source structure would be lost too early.
- **First backend:** Rust, not LLVM-first, because semantic auditability matters more during bring-up.
- **Layout default:** SoA, not AoS, because the target workloads are bulk-field oriented and vectorization-sensitive.
- **Ownership:** graded ownership, not a separate borrow calculus, because ownership should participate in the same resource algebra as the other grades.
- **Context ergonomics:** checked focusing and root-relative elision, not ambient dependency injection, because the compiler must still recover explicit paths before summaries are built.
- **Grade ergonomics:** checked grade elision and inference, not hidden resource defaults, because omitted contracts are acceptable only when the compiler can reconstruct and explain them exactly.
- **Diagnostics:** stable JSON with ranked fixes, not prose-only messaging, because the workflow is intentionally human-and-agent facing.

### 23.17 Cross-tradition shorthand

The earlier foundation and feature chapters already did the reader-by-reader comparison in the places where it changed a concrete design choice. The decision ledger only needs the short synthesis.

C++ and Rust validate the emphasis on explicit layout, ownership, and native-code credibility. Java validates the need for large-system architectural seriousness. Python and TypeScript validate the value of a readable transformation surface but also warn against leaving performance or structure outside the compiler's semantic core. Typed-functional and effect-oriented languages validate optics and graded reasoning while leaving more of the machine story implicit than Optic is willing to.

That is enough for the decision record because the accepted choices have already been justified individually above. What matters here is the combined outcome:

- Optic keeps the **layout and zero-cost discipline** that C++ and Rust make non-negotiable,
- keeps the **service- and module-level seriousness** associated with Java-style large systems,
- keeps the **readable transformation surface** that Python and TypeScript users expect,
- and keeps the **compositional semantic vocabulary** of typed-functional languages,

while insisting that those strengths survive into summaries, grades, graph legality, and backend consequences rather than being split across separate layers of the toolchain.

That contemporary synthesis also makes the historical question sharper: several older traditions already solved part of Optic's problem, but only in ways that later proved difficult to keep as one coherent systems vocabulary.

#### 23.17.1 Experimental mathematics never creates a second rulebook

The same closure rule applies to the experimental lane. Geometric algebra, nonstandard analysis, sheaf-style local/global reasoning, or any later mathematical extension may enrich the compiler's witnesses, analyses, and domain libraries, but none of them is permitted to introduce an alternative legality pipeline. The accepted design is that every experimental feature lowers back into the same `OpticSummary`, `RegionSet`, grade product, boundary-contract schema, and CGIR node families as the ordinary language.

That subservience requirement is what keeps the architecture from fracturing. A multivector kernel, for example, may carry richer domain structure than a plain `Vec4<f32>` traversal, but the backend still receives ordinary read/write regions, cache and lane facts, and explicit target-profile assumptions. The experimental mathematics enriches the path to those facts; it does not replace them.

### 23.18 Historic paradigms recovered, reframed, and constrained

This section is not a tour of curiosities. Its purpose is to recover a small set of older, often sidelined paradigms whose strongest ideas still solve problems Optic must solve, while also showing how the language constrains the growth dynamic that made those traditions hard to keep as a general systems default. The measure throughout is whether each recovered insight can stay attached to the book's one semantic center—explicit costates, optics, summaries, grades, and graph transactions—instead of turning into a second language personality, a parallel toolchain, or a separate ecosystem.

| Historical paradigm or design | Original strong insight | Where it reappears in Optic | What Optic refuses to inherit unchanged |
|---|---|---|---|
| dataflow and single-assignment languages | keep program structure graph-shaped long enough to expose parallelism and staging | `CGIR`, staged graph evaluation, compiler phases as graph transactions | whole-language implicit parallelism and a one-size-fits-all dataflow surface |
| array languages | make whole-array transformation and shape-preserving bulk operations primary | traversals, `SimdEligible`, `SoA` defaults, vector-friendly summaries | array notation as the universal surface for all programming |
| uniqueness typing | use type information to justify destructive update and disciplined world interaction | ownership grades, fractional permissions, explicit runtime regions | a single world token or a separate ownership language detached from other grades |
| synchronous/reactive languages | treat time, clocks, and reactivity as semantic structure | coinduction, liveness grades, injected clocks and event sources | global synchrony as the semantics of every program |
| capability systems | make authority explicit rather than ambient | `BoundaryContract`, capability-gated host subregions, runtime-family permissions | a capability calculus as the only organizing principle of the language |
| logic/query languages and attribute grammars | query and compute over semantic structure rather than over raw text or opaque objects | native Project queries, internal QIR, query→fix synthesis, compiler phases as graph transactions | a second user-facing query language or syntax-tree-only semantic decoration |

Read the four columns as a compact record of what each tradition got right, where Optic deliberately reuses it, and what that tradition eventually stopped being able to say no to.

The common pattern is clear. Each historical tradition contributes one part of the answer to a problem that Optic also faces. What the current design tries to avoid is the historical failure mode in which that one part grows until it becomes a second language personality or a whole separate ecosystem.

#### 23.18.1 Dataflow and single-assignment languages

Lucid, SISAL, and related dataflow traditions are the closest historical relatives of Optic's graph-centered compiler architecture. Their enduring insight is that a program is often easier to optimize when its semantic structure remains graph-shaped rather than being flattened too early into sequential control. That lesson is visible in Optic's insistence on `CGIR`, graph-preserving staging, and compiler phases expressed as graph transactions over `Project.build`.

What Optic refuses to inherit unchanged is the tendency to make graph reduction or implicit parallelism the whole surface-language personality. A systems language still needs explicit host boundaries, explicit address spaces, explicit ownership, and explicit low-level lowering obligations. The historical dataflow insight is therefore recovered mainly in the compiler form and in the staging/runtime split, not as a replacement for the entire user language.

#### 23.18.2 Array languages

APL and the array-language family contributed a different but equally durable insight: whole-array or whole-collection transformation is often the real semantic unit, while scalar loops are an implementation detail. Optic recovers that insight through traversals, `TraversalGrade`, `SimdEligible`, and the language-wide assumption that `SoA` layout is the ordinary bulk-data form.

The constraint is deliberate. Optic does not try to make array notation the universal syntax of the language, nor does it assume that every domain is naturally a dense homogeneous tensor world. Instead it says: when a program really *is* shape-preserving bulk transformation over regular layout, that fact should be explicit enough to justify vectorization and cache reasoning before the backend has to guess.

#### 23.18.3 Uniqueness typing and single-threaded state

Clean's uniqueness typing is one of the clearest historical demonstrations that strong static reasoning can permit destructive update and efficient foreign-world interaction without giving up a high-level programming model. Optic recovers that gain in its ownership grades and, later, fractional ownership model.

The difference is scope. Uniqueness is not a second subsystem beside the rest of the resource story. It is one dimension in a larger grade algebra, and it ranges over explicit runtime regions rather than over one global world token. That widening is what lets the same ownership fact participate in alias checking, multicore partition proofs, host-boundary legality, and backend metadata generation.

#### 23.18.4 Synchronous and reactive languages

Esterel and Lustre made a powerful claim that mainstream languages still often blur: time and reactivity are semantic structure, not merely library convention. Optic recovers that claim in `Coinductive` nodes, `LivenessGrade`, injected clocks, replay classification, and runtime-family distinctions.

The key constraint is that synchrony is not made universal. Some domains want globally synchronous reasoning. Others want event loops, bounded queues, or mixed hosted/freestanding execution families. Optic therefore treats synchrony as one disciplined runtime form inside a larger architecture rather than as the semantics of every computation.

#### 23.18.5 Capability systems and authority as data

From Dennis and Van Horn through E and later capability-secure languages, one lesson keeps recurring: authority is easier to reason about when it is carried explicitly rather than ambiently. Optic recovers that lesson in `BoundaryContract`, capability-gated host subregions, runtime-family declarations, and the insistence that unsafe or foreign access always reenters the graph as an explicit leaf with a local contract.

What Optic avoids is turning the whole language into a single capability calculus. Capability facts complement, rather than replace, types, grades, region summaries, and target profiles. This keeps authority visible without forcing all program meaning into one security-centric abstraction.

#### 23.18.6 Logic/query languages and attribute grammars

Datalog and executable attribute grammars are especially relevant to Optic's newer project-graph direction. Datalog showed that semantic structures can be queried declaratively, incrementally, and efficiently through indexing and fixed-point evaluation. Attribute grammars showed that meaning and translation can be computed over program structure rather than reconstructed procedurally from scratch.

Optic reuses both ideas, but again with an important constraint. User-facing Project queries remain ordinary optic-based code over `Project`, not a second full user language. The richer QIR and query optimizer remain internal. Likewise, compiler phases are not syntax-tree-only attribute computations; they are graph transactions over the full `Project` root, with syntax, HIR, summaries, CGIR, artifacts, and diagnostics all treated as projections of one semantic whole.

#### 23.18.7 Why these historical recoveries matter to the language as a whole

These historical references matter because they clarify a recurring design habit in the book: when the language faces a hard problem, it tries first to recover an older strong idea and then constrain it so it fits one explicit compiler architecture. That is true for array semantics, dataflow graphs, uniqueness, synchrony, capability discipline, and declarative project queries alike. The result is not a collage of historical features. It is a language that tries to keep one semantic center while admitting that several of its strongest ideas were discovered long before the current generation of mainstream languages.

The practical consequence is immediate. When a new feature is proposed, the right first question is not just whether it can be encoded inside the current calculus. The right first question is whether some earlier language, paradigm, or toolchain already tried a close relative of that idea, and if so, what it eventually demanded of the implementation, the optimizer, the runtime, and the surrounding ecosystem. That historical check should happen before surface syntax is frozen, because the real danger is usually not the first local use case; it is the growth pattern that follows once the feature becomes convenient.

The deeper insight is even plainer: these historical paradigms did not fail to become default vocabulary because their core insights were wrong. They became difficult to keep as the default because they grew without enough constraint. Array notation could become glyphic and totalizing, dataflow could become too implicit about scheduling and evaluation, uniqueness and capability systems could become second personalities of a language, and query or attribute-grammar layers could drift into separate sublanguages. That is why the four-column table above should be read not only as a lineage map, but as a record of what each tradition got right and what it eventually stopped being able to say no to.

## 24. Quantitative Theory-to-Machine Bridge

The earlier chapters explain the qualitative bridge from semantics to machine behavior. This chapter makes the bridge quantitative: branch-predictor arithmetic for prisms, vector-lane arithmetic for traversals, cache and page calculations for layouts, and queue-depth reasoning for coinductive event loops.

This chapter collects the quantitative rules of thumb and exact formulas that connect the language's abstractions to hardware behavior. These numbers are not a substitute for benchmarking, but they are the right first-order model for compiler design.

### 24.1 Prisms, branches, and branch-predictor arithmetic

#### 24.1.1 The semantic shape

A prism test is a coproduct elimination:

```text
preview : S -> Option<A>
```

Operationally that becomes a conditional.

#### 24.1.2 The machine model

For a branch with taken probability `p`, a first-order expected penalty model is:

```text
E[cost] = p * cost_taken + (1 - p) * cost_not_taken + miss_rate(p, history) * mispredict_penalty
```

The exact `miss_rate` depends on the predictor, but three compiler-level facts matter immediately:

1. dense success cases (`p` close to 1) want fall-through on the success arm;
2. sparse success cases (`p` close to 0) want fall-through on the failure arm;
3. if the branch body is simple enough, if-conversion or masking may beat an unpredictable branch.

#### 24.1.3 Compiler artifact

`BranchBias<Likely|Unlikely|Neutral>` lives as a zero-cost dimension or annotation on prisms and branch-producing traversals.

#### 24.1.4 Backend legality condition

Hints may only be emitted when the prism semantics remain unchanged under the control transformation. Branch bias cannot justify reordering that would change observable writes.

#### 24.1.5 Backend mapping

| IR fact | LLVM lowering |
|---------|---------------|
| `BranchBias<Likely>` | branch weight metadata favoring the success edge |
| `BranchBias<Unlikely>` | branch weight metadata favoring the failure edge |
| mask-lowerable prism + traversal | predicated vector body or masked stores |

#### 24.1.6 Practical rule

- keep predictable branches as branches;
- convert unpredictable simple prisms to masks when doing so reduces mispredict cost and does not increase memory traffic too much;
- for sparse traversals, consider a two-phase design: compact indices first, then run a dense traversal over the compacted set.

### 24.2 Traversals, vector lanes, and remainder handling

#### 24.2.1 The law

A traversal promises same-shape element-wise visitation.

#### 24.2.2 SIMD legality checklist

A traversal is vectorizable when all of the following hold:

1. no inter-lane data dependency,
2. regular stride or acceptable gather/scatter cost,
3. element operation has no unstructured control dependence,
4. updates do not alias across lanes,
5. remainder handling is well-defined.

#### 24.2.3 Lane arithmetic

For vector width `W` bytes and element size `E` bytes:

```text
lanes = floor(W / E)
```

Examples:

- **`f32` (4 bytes):** 8 lanes on AVX2, 16 lanes on AVX-512.
- **`f64` (8 bytes):** 4 lanes on AVX2, 8 lanes on AVX-512.
- **`u8` (1 byte):** 32 lanes on AVX2, 64 lanes on AVX-512.
- **`Vec2<f32>` (8 bytes):** 4 logical pairs on AVX2, 8 logical pairs on AVX-512.

#### 24.2.4 Remainder policy

The backend must choose one of three remainder strategies:

- scalar tail loop,
- masked vector tail,
- peeled loop with alignment and multiple vector bodies.

The default policy should be:

- scalar tail for short or infrequent tails,
- masked tail when the target has cheap masks and the traversal body is arithmetic-heavy,
- peeled bodies when alignment matters strongly or when the body contains wide loads/stores.

### 24.3 Cache lines, pages, TLBs, and prefetch

#### 24.3.1 Cache arithmetic

Given 64-byte cache lines:

```text
elements_per_line = floor(64 / stride)
lines_for_N       = ceil(N / elements_per_line)
```

Examples:

- **`f32` (stride 4):** 16 elements per 64-byte cache line.
- **`Vec2<f32>` (stride 8):** 8 elements per cache line.
- **`Vec3<f32>` padded to 16:** 4 elements per cache line.
- **`u64` (stride 8):** 8 elements per cache line.

#### 24.3.2 Page arithmetic

Assuming 4 KiB pages:

```text
pages_for_field = ceil(bytes(field) / 4096)
```

If a traversal touches more pages than the TLB can cover cheaply, the language should treat that as a layout or tiling question, not just a cache-grade question.

#### 24.3.3 Prefetch reasoning

Software prefetch is worth considering only when:

- stride is regular,
- latency to data exceeds useful work in one or two iterations,
- and hardware prefetch is not already saturating.

The language should use grade and target-profile facts to decide when to emit `llvm.prefetch`, but it must keep this as an optimization hint, not a semantic requirement.

### 24.4 Cursor and `PathLift` become pointer arithmetic

#### 24.4.1 From theory to address calculation

`Cursor<S>` plus a normalized field path imply a direct address calculation:

```text
addr(field, id) = base(field) + id * stride(field)
```

`PathLift` exists precisely to preserve that relationship when optics are composed. A nested optic that focuses on `transform.position.x` must still eventually lower to a legal address expression or a small chain of register-resident projections plus one final store.

#### 24.4.2 Why this matters

If composed optics lost their path meaning, the backend would have to recover alias sets by heuristic field-sensitive analysis. `PathLift` prevents that by carrying path meaning explicitly.

### 24.5 Region sets, TBAA, and store reordering

#### 24.5.1 Static rule

If two `RegionSet`s are proven disjoint in the conservative region language, the backend may materialize that proof into distinct TBAA nodes.

#### 24.5.2 Machine consequence

That unlocks:

- better load/store scheduling,
- more reliable auto-vectorization,
- fewer false alias dependencies,
- improved LICM and GVN opportunities.

#### 24.5.3 Caution

TBAA is only as good as the region summary. Under-approximation is catastrophic. The language therefore chooses conservative region normalization in the prelude.

### 24.6 Staging cost model: specialization versus code size

Staging is not free. It trades runtime selection cost for compile-time work and code-size growth.

A simple first-order specialization profitability model is:

```text
profit(stage) = executions * (dynamic_overhead_removed + optimization_gain)
              - compile_time_cost
              - code_size_penalty
```

Where the language has enough information, `CompileTimeGrade` should track at least the compile-time work. The runtime payoff still needs empirical benchmarking.

#### 24.6.1 Practical staging rules

- stage structure, not data;
- stage archetype layouts, query plans, routing graphs, and fixed protocol stacks;
- do not stage rarely executed code unless it removes a very large dynamic cost;
- cache specialization products by structural hash.

### 24.7 Coinduction, rings, queue depth, and backpressure

#### 24.7.1 Ring arithmetic

For a ring of capacity `Q` and average service time `S`, throughput pressure roughly appears when arrival rate `λ` approaches `Q / S` over the window the ring can absorb. The language does not pretend to prove queueing theory results, but it should at least make queue capacity and liveness visible enough that the backend/runtime can choose sane defaults.

#### 24.7.2 Backpressure rule

A coinductive pipeline that reads from a host ring and writes to another host queue must carry a backpressure policy. If the sink queue is bounded, the pipeline's liveness and latency grades must reflect whether it:

- blocks,
- drops,
- buffers,
- or applies feedback upstream.

Without that, the semantics are incomplete.

### 24.8 Multicore partitioning and false sharing formulas

For `T` threads and cache line size `CL`:

```text
false_sharing_risk if stride(write_target) * chunk_granularity < CL and neighboring threads write adjacent regions
```

A rough safe chunk rule is:

```text
chunk_bytes_per_thread >= 2 * CL
```

for write-heavy kernels, so that each thread is likely to own multiple lines without bouncing a single line back and forth.

#### 24.8.1 Work partitioning principles

- partition by contiguous ranges for SoA traversals,
- partition by NUMA node before by core when remote-memory penalties are large,
- prefer static chunking for uniform kernels,
- use work stealing only when load imbalance dominates locality loss.

### 24.9 Deterministic replay cost model

Replay can be achieved by some combination of:

- full snapshots,
- deltas / write logs,
- recorded external inputs,
- deterministic clock and RNG sources.

The cost trade-off is straightforward:

- **Full snapshots:** high storage cost, very fast replay, best for sparse checkpoints and debugging jumps.
- **Deltas only:** low-to-medium storage cost, slower replay, best for long-running recordings.
- **Inputs only:** low storage cost, replay speed depends on re-execution cost, best when the system is highly deterministic and cheap to recompute.
- **Hybrid:** medium storage cost and medium-to-high replay speed, usually the most practical overall choice.

The language should keep replay as an explicit structural feature rather than an afterthought because its `Runtime` model already exposes the right boundaries.

### 24.10 Quantitative cheat sheet for implementers

- **Vector lanes:** `floor(vector_width / element_size)`.
- **Cache lines for `N` elements:** `ceil(N * stride / 64)`.
- **Pages for a field:** `ceil(total_bytes / 4096)`.
- **Contiguous SoA address:** `base + id * stride`.
- **Branch success hint:** choose `likely` only when success frequency is stable and high.
- **False-sharing warning:** neighboring writers touch the same 64-byte line.
- **Profitable staging:** runtime savings exceed compile-time work and code-size cost over expected executions.
- **Mask versus branch:** prefer masks when the branch is hard to predict and the body is simple enough.

## 25. Domain Blueprints and Operational Envelopes

The domain chapters in Part IV explain how the language maps into major software families. The operational-envelope chapter below complements them by recording, in one place, the hot loops, non-negotiable design rules, and benchmark envelopes that determine whether a domain implementation is actually credible.

This chapter extends the domain application chapter by turning each domain into an implementation playbook. Each playbook answers the same questions:

1. what the root costate is,
2. what the hot loops are,
3. what grades matter most,
4. what layout decisions are non-negotiable,
5. what diagnostics and benchmarks define progress.

### 25.1 Cross-language map of the domain playbooks

Each domain discussed below already has dominant implementation traditions. The point of these playbooks is not to pretend those traditions are wrong. It is to explain what the Optic design is trying to preserve from them and what it is trying to change.

| Domain | Dominant implementation traditions today | What those traditions get right | What the Optic design tries to change |
|---|---|---|---|
| **Kernels** | mostly C, with increasing Rust adoption and selective C++ in subsystems | exact hardware control, ABI stability, ruthless attention to hot loops | replace convention-heavy safety and locality reasoning with typed grades, explicit costates, and replay-aware structure |
| **Browsers** | C++ and Rust engine code, JavaScript and TypeScript at the application layer | mature pipelines, strong JIT integration, careful rendering architecture | make more of the engine's pipeline structure explicit earlier so layout, staging, and coinduction are not merely engineering patterns |
| **Databases** | C, C++, Rust, and some Java systems | strong storage-engine design and practical concurrency control | unify storage, plans, transaction state, and optimizer rewrites under one graph and one grade story |
| **Games** | C++ engines, C# gameplay layers, Python or TypeScript tooling and content pipelines | high-performance ECS cores and productive tooling splits | narrow the conceptual gap between the hottest loops and the highest-level data transformations |
| **Compilers and tools** | C++, Rust, OCaml, Java, and Python-heavy tool ecosystems | rich IR cultures and flexible analysis tooling | make passes, diagnostics, and rewrite legality uniform over explicit costates instead of switching abstraction styles between phases |

The sections that follow should be read as domain translations of the same core architecture, not as separate mini-languages.

### 25.2 Kernels

#### 25.2.1 Root runtime decomposition

```text
KernelRuntime =
  MemoryWorld
  × DeviceWorld
  × SchedulerWorld
  × HostContext(kernel mode)
```

#### 25.2.2 Non-negotiable design rules

- frame allocators and DMA buffers carry linear or uniquely-owned grades;
- interrupt optics are explicitly non-blocking;
- MMIO regions are never treated as ordinary cacheable memory;
- page-table walks expose level count and TLB effects explicitly.

#### 25.2.3 Critical hot loops

- free-list or buddy allocation,
- page-table walk/update,
- run queue pick-next-task,
- RX/TX ring processing,
- block completion handling.

#### 25.2.4 Why the optic model fits

Kernel subsystems are structured update problems over explicit tables and queues. The language's root advantage in this domain is not abstract elegance. It is that the same static machinery used for user-space SoA loops also expresses:

- uniqueness of resources,
- queue/event processing,
- table walks,
- and replay/trace boundaries.

#### 25.2.5 What must be rejected early

- hidden allocation in interrupt paths,
- uncontrolled blocking in scheduler or IRQ optics,
- copying linear host resources into branch runtimes,
- alias-unsafe parallel updates to shared tables.

#### 25.2.6 Benchmarks that matter

- frame allocate/free throughput,
- page-table map/unmap latency,
- IRQ tail latency,
- scheduler tick budget,
- RX/TX packets per second with replayable input traces.

### 25.3 Browsers

#### 25.3.1 Root runtime decomposition

```text
BrowserRuntime =
  DocumentWorld
  × StyleWorld
  × LayoutWorld
  × PaintWorld
  × InputWorld
  × NetworkWorld
```

#### 25.3.2 Hot loops and pipelines

- selector matching,
- style cascade update,
- measure/place layout passes,
- paint command assembly,
- event dispatch,
- resource loading and decode.

#### 25.3.3 Why the optic model fits

A browser is a graph of data refinements from parsed input to pixels. The strong fit is in making each phase explicit and separately gradable:

- traversals for selector and layout passes,
- prisms for DOM shape-dependent behavior,
- staged pipelines for compiled selectors and layout plans,
- coinduction for network and event ingress.

#### 25.3.4 Machine-sympathetic rules

- DOM storage should prefer ID-indexed arenas rather than pointer forests in hot subsystems;
- style and layout should be SoA or segmented SoA for hot fields;
- paint batching should be staged and cache persistent command shapes where possible;
- event propagation should preserve provenance for tooling, not hide everything in callback tables.

#### 25.3.5 Benchmarks that matter

- style recalc time on large DOMs,
- layout time on incremental updates,
- paint command build time,
- frame latency under input and network activity,
- memory footprint per DOM node and per paintable object.

### 25.4 Databases

#### 25.4.1 Root runtime decomposition

```text
DbRuntime =
  BufferPool
  × LogWorld
  × TxnWorld
  × IndexWorld
  × QueryWorld
  × NetworkWorld
```

#### 25.4.2 Hot loops and structural pipelines

- tuple scan/filter/project,
- index probe and page walk,
- log append and flush,
- MVCC visibility checks,
- query-plan execution over staged operators.

#### 25.4.3 Why the optic model fits

Database internals already decompose into costates naturally:

- buffer pool pages,
- indexes,
- transaction tables,
- row-version chains,
- query plans.

The language's static value is strongest where databases struggle today:

- making page and buffer locality explicit,
- distinguishing compile-time query specialization from runtime tuple flow,
- and making protocol or transaction invariants visible to the type system.

#### 25.4.4 Machine-sympathetic rules

- hot row fields accessed by scans should be columnar/SoA where engine design allows;
- B-tree or Bw-tree traversal optics must expose page count and I/O grade;
- commit and replication paths should carry explicit latency and bandwidth grades;
- replay and deterministic test harnesses should treat clocks and RNG as injected host costates.

#### 25.4.5 Benchmarks that matter

- scan throughput by layout,
- index probe latency by tree height,
- transaction commit latency under contention,
- log bandwidth,
- query compilation versus execution trade-offs for staged plans.

### 25.5 Games

#### 25.5.1 Root runtime decomposition

```text
GameRuntime =
  World
  × RenderWorld
  × AudioWorld
  × InputWorld
  × NetWorld(optional)
```

#### 25.5.2 Hot loops

- ECS system traversals,
- transform propagation,
- physics integration,
- visibility and culling,
- animation sampling,
- audio DSP,
- packet replication / prediction if networked.

#### 25.5.3 Why the optic model fits

Games are the clearest case where DoD and optics line up directly:

- components are fields in SoA arenas,
- systems are optics or optic products,
- schedules are CGIR graphs,
- staged archetypes map to compiled query shapes,
- SIMD traversals fall out of traversal laws.

#### 25.5.4 Machine-sympathetic rules

- keep hot components SoA and separate from cold/editor-only metadata;
- use staged archetype selection to avoid per-entity component tests in hot loops;
- treat prediction/rollback as replay over explicit world snapshots rather than bespoke ad hoc machinery;
- isolate render and simulation costates unless a shared hot path justifies fusion.

#### 25.5.5 Benchmarks that matter

- entities updated per frame,
- frame-time variance under mixed simulation/render load,
- rollback/replay speed,
- SIMD efficiency for physics and audio kernels,
- memory footprint by archetype.

### 25.6 Compilers and developer tools

#### 25.6.1 Root runtime decomposition

```text
CompilerRuntime =
  SourceWorld
  × AstWorld
  × HirWorld
  × TypeWorld
  × IrWorld
  × ArtifactWorld
```

#### 25.6.2 Hot loops and passes

- parse/token traversals,
- symbol-table lookups,
- type-check walks,
- dataflow analyses,
- rewrite passes,
- register allocation,
- code emission and object linking orchestration.

#### 25.6.3 Why the optic model fits

A compiler is literally a pipeline of structured observations and rewrites over typed artifacts. The language gains a self-hosting advantage if its own compiler passes can be described by the same summary and diagnostic model it uses for user code.

#### 25.6.4 Machine-sympathetic rules

- keep dense IR nodes in flat arenas rather than pointer-heavy recursive structures once lowering begins;
- stage expensive but structurally stable analyses where possible;
- use traversals and prisms to express rewrite opportunities explicitly, preserving provenance for diagnostics;
- benchmark pass-by-pass time and memory, not just whole-compiler wall clock.

#### 25.6.5 Benchmarks that matter

- parse throughput,
- memory per AST/HIR/IR node,
- type-check throughput,
- optimizer pass timing,
- object code size and compile-time by backend target.

### 25.7 Network services and control planes

The original book focuses on kernels, browsers, databases, games, and compilers. A sixth domain deserves explicit treatment because it sits at the intersection of them all: network services and control-plane software.

#### 25.7.1 Root runtime decomposition

```text
ServiceRuntime =
  SessionWorld
  × RequestWorld
  × CacheWorld
  × StorageWorld
  × HostContext
```

#### 25.7.2 Hot loops

- packet/session state progression,
- parse/auth/route pipelines,
- request batching,
- serialization and zero-copy writeback,
- timer-wheel or deadline queue maintenance.

#### 25.7.3 Why the optic model fits

Network services are often written as deeply nested callback, async, or framework stacks that obscure the real dataflow. The optic graph exposes the real pipeline explicitly, which is valuable for:

- static latency and bandwidth contracts,
- zero-copy paths,
- explicit session protocols,
- deterministic replay of traffic.

#### 25.7.4 Machine-sympathetic rules

- keep request/session hot fields in explicit arenas;
- stage routing and protocol dispatch tables;
- make backpressure policy a first-class part of coinductive pipelines;
- never hide clocks, RNG, or I/O behind ambient calls when deterministic testing matters.

#### 25.7.5 Benchmarks that matter

- p50/p99 latency by route,
- throughput under replayed traffic,
- copy count in hot paths,
- queue depth behavior under burst traffic,
- connection state memory footprint.

### 25.8 Scientific computing, PDEs, and solver systems

#### 25.8.1 Root runtime decomposition

```text
SolverRuntime =
  FieldWorld
  × OperatorWorld
  × ConstraintWorld
  × HostContext
```

#### 25.8.2 Hot loops and staged artifacts

The dominant solver costs are usually split across two regimes.

- **build-time / setup work**: mesh normalization, sparsity discovery, Jacobian generation, stencil assembly, blocking and tiling plans, imported equation lowering;
- **runtime work**: stepping, residual evaluation, linear solves, convergence checks, prolongation/restriction, and output or checkpoint handling.

That split aligns unusually well with the language's existing build/run distinction.

#### 25.8.3 Why the optic model fits

Solvers are best understood here as optics-native pipelines. Problem state is a costate. Residuals, Jacobians, preconditioners, and steppers are ordinary optics. Generated kernels and sparsity plans are staged artifacts. The important implication is that the language does **not** need a separate solver semantics in order to support serious solver work.

The right default is:

- optics-native solver libraries under `std.solver.*`,
- staged operator and kernel generation over `BuildRuntime`,
- optional imported or embedded equation frontends that still lower into the same summaries and graph.

#### 25.8.4 Machine-sympathetic rules

- keep field and operator state in explicit arenas rather than opaque object graphs;
- treat generated kernels, sparsity plans, and symbolic eliminations as staged artifacts with clear cache keys;
- prefer optics-native solver composition over a second solver DSL in the core language;
- permit imported equation or stencil frontends only when they lower into the same provenance, summary, and artifact model.

#### 25.8.5 Benchmarks that matter

- time-to-first-solve versus steady-state solve throughput,
- generated-kernel code size,
- residual/Jacobian assembly cost,
- convergence steps for canonical problems,
- and target-profile-sensitive stencil or sparse-kernel throughput.

### 25.9 Cross-domain reusable patterns

Across all domains, the same structural patterns recur. The book treats them as reusable architecture motifs rather than separate inventions each time.

- **Table walk** — usually a lens/traversal composition; common in kernels, databases, and compilers.
- **Ring processing** — usually a coinductive optic; common in kernels, services, and browsers.
- **Filter then transform** — usually a prism plus traversal; common in databases, games, browsers, and compilers.
- **Stage then execute** — usually `stage` plus traversal; common in databases, games, browsers, and services.
- **Partitioned parallel loop** — usually a traversal plus parallel product; common in games, analytics, and browsers.
- **Snapshot/replay** — usually record/replay over the runtime root; common in kernels, services, games, and compilers.

These motifs are evidence that the language is not overfit to one niche. The same reasoning structure repeats because the same machine pressures repeat.

### 25.10 Mixed-domain collision stress tests

The cleanest way to test extensibility is to force unlike domains to meet while preserving the same summary and grade rules. The language should therefore treat a small set of mixed-domain collision suites as standing stress tests rather than as colorful demos.

#### 25.10.1 A bounded render loop meets an unbounded network stack

A renderer wants a tight frame budget; a network ingress path may have bursty or effectively unbounded arrival behavior. The architecture should not solve this by hidden heuristics or by smuggling a second scheduler into the language. The accepted pattern is to keep the render loop inside bounded latency and liveness grades, keep the network path inside coinductive and backpressure-aware host boundaries, and connect the two through an explicit queue or mailbox costate whose policy is visible in summaries. The algebraic resolution is therefore: bounded rendering stays bounded, network ingress stays host-facing, and the queue between them absorbs the mismatch under an explicit contract.

#### 25.10.2 A transactional Optic wrapper over a legacy C storage engine

A realistic systems language must also survive a mixed boundary between modern summaries and opaque historical code. A useful stress case is an MVCC or transaction-graded optic over a legacy C storage engine. The success criterion is not that the foreign handle becomes magically transparent; it is that the wrapper optic still publishes ordinary regions, quantitative grades, and local boundary obligations so the rest of the compiler can keep reasoning in one vocabulary.

#### 25.10.3 Experimental mathematics inside ordinary hot paths

A third stress test is to take an experimental domain kernel — for example a geometric-algebra multivector transform or an experimental nonstandard-analysis-inspired solver step — and embed it in an otherwise ordinary traversal or staged build plan. The implementation passes only if the experimental layer still lowers through ordinary summaries, target profiles, cache grades, and backend legality checks. This is the practical meaning of saying that the single-graph foundation remains absolute: experimental mathematics may enrich the path to machine facts, but it may not invent a second path around them.

## 26. Backend Validation, Portability, and Performance Discipline

Once the language grows beyond the Rust path into a native backend, correctness requires more than green tests. It requires translation validation, soundness envelopes for alias metadata and vectorization, disciplined target profiling, and benchmark procedures that can survive long-lived compiler evolution.

### 26.1 Translation validation: Rust path versus LLVM path

The LLVM backend should be treated as correct only when it repeatedly matches the semantics of the Rust path on the regression corpus.

#### 26.1.1 Required validation layers

1. diagnostics agreement on accepted/rejected examples,
2. HIR and summary agreement,
3. CGIR agreement before backend lowering,
4. output behavior agreement on e2e tests,
5. benchmark comparison against handwritten and Rust-path baselines.

#### 26.1.2 Practical rule

When the LLVM backend disagrees with the Rust path, the default assumption should be that the LLVM path is wrong until proven otherwise.

### 26.2 Soundness envelopes for alias metadata and vectorization

The backend must document which proof obligations justify each optimization class.

- **TBAA-based alias disambiguation:** requires conservative `RegionSet` disjointness.
- **SIMD traversal lowering:** requires traversal legality plus lane independence.
- **Loop fusion:** requires non-escaping intermediates plus grade legality.
- **Parallel lowering:** requires partition independence plus false-sharing checks.
- **Event-loop lowering:** requires a coinductive node plus explicit host queue semantics.

If any required artifact is missing, the backend must use the safe fallback path.

### 26.3 Target profiles and portability

The full language should compile against explicit target profiles, not implicit assumptions about the local machine.

#### 26.3.1 Minimum target-profile contents

- architecture (`x86_64`, `aarch64`, `riscv64`, later `wasm32` if desired),
- cache line size and page size assumptions,
- SIMD capability set,
- atomics and memory-order support,
- host I/O facilities (`io_uring`, `kqueue`, `epoll`, IOCP, none),
- `no_std` capability,
- endianness and alignment rules.

The grade system should *not* pretend these are portable by magic. It should make the relevant target assumptions explicit.

### 26.4 Hosted versus `no_std` kernel-class targets

The backend should support at least three target tiers:

- **Hosted-debug:** normal user-space build with maximum diagnostics; easiest validation path.
- **Hosted-release:** optimized native build; primary benchmark and service path.
- **Kernel / `no_std`:** no allocator or hosted runtime assumed; explicit host adapters required.

A language feature that only works in hosted builds must say so. Silent dependence on hosted services is unacceptable for the kernel-class roadmap.

### 26.5 Benchmark suites by domain

The performance discipline should track both universal microbenchmarks and domain-specific suites.

#### 26.5.1 Universal suites

- single-field traversal,
- product traversal,
- filtered traversal,
- staged specialization overhead/payoff,
- ring/event-loop throughput,
- alias-safe parallel traversal.

#### 26.5.2 Domain suites

- kernel: allocator, page walk, scheduler, RX/TX ring;
- browser: style/layout/paint;
- database: scan/probe/commit;
- game: ECS update/render/audio;
- compiler: parse/type/opt/codegen;
- service: request pipeline and replay throughput.

### 26.6 Agent-oriented backend diagnostics

The backend should extend the diagnostic discipline instead of abandoning it.

#### 26.6.1 Suggested backend families

- **`LLV-1xx`** — IR emission shape or legality failure.
- **`LLV-2xx`** — metadata or target-profile mismatch.
- **`VEC-3xx`** — vectorization blocked or unsafe.
- **`PAR-6xx`** — parallel lowering, false-sharing, or chunking problem.
- **`PER-7xx`** — benchmark regression or specialization-profitability concern.

Each backend diagnostic should include:

- the blocked optimization,
- the missing proof artifact,
- the safe fallback that was chosen,
- the smallest next inspection command.

### 26.7 Performance regression workflow

A full-language compiler that makes strong performance claims must treat performance regressions as first-class failures.

#### 26.7.1 Workflow

1. record baseline per benchmark and target profile,
2. diff HIR/summary/CGIR when a benchmark regresses,
3. identify whether the regression came from legality loss, missed rewrite, target-profile change, or backend drift,
4. add a regression test that guards the discovered cause.

The purpose of the workflow is not merely to recover performance once. It is to turn each loss into repository knowledge.

### 26.8 Mixed-domain collision suites and experimental-lane lowering checks

Backend validation should include a small set of stress suites where unlike domains meet and where experimental-domain kernels must still lower through ordinary summaries. Examples include:

- a frame-budgeted render/update loop fed by a coinductive network ingress queue,
- a transaction-graded database wrapper over a legacy C storage engine,
- an experimental geometric or solver kernel embedded inside an otherwise ordinary traversal.

The backend passes these suites only when it can show the same properties it shows elsewhere in the book: ordinary provenance retention, ordinary `RegionSet`-based legality, ordinary grade accounting, and no hidden optimizer side rules for the experimental path.

### 26.9 Release gates for full-language milestones

A full-language milestone should ship only when all of the following hold:

- semantic and diagnostic agreement with the Rust reference path where applicable,
- stable benchmark deltas on the gated corpus,
- no unexplained legality downgrades on hot-path examples,
- target-profile assumptions documented and tested,
- provenance preserved through optimization and backend lowering,
- replay/tooling hooks still work after the backend changes.

These gates are still only the backend-facing slice of maturity. Once a language becomes usable by other people, the harder questions shift toward compatibility policy, build-graph identity, ecosystem tooling, binary distribution, and runtime coherence. The next chapter names those pressures directly and ties them back to the current architecture.

The last chapter of this part steps back from backend validation and asks the broader maturity question: what still has to be fixed, specified, or governed before the language can survive real ecosystems rather than only compelling examples?

## 27. Late-Stage Gaps, Cross-Language Lessons, and the Remaining Work

### 27.1 Why languages often hit their hardest problems after the core seems to work

Early language work is usually dominated by syntax, typing, lowering, and a first convincing example program. Those are real problems, but they are not usually the problems that decide whether a language becomes operationally complete. The later-stage pressures are different.

A language becomes hard in a new way when other people depend on it. At that point the questions are no longer only:

- can the compiler lower this feature correctly,
- can the optimizer justify this rewrite,
- can the runtime make this program fast.

They become:

- what exactly is stable,
- what is allowed to change,
- how separate compilation is represented,
- what a package or binary artifact means,
- how foreign boundaries are audited,
- how a debugger or profiler continues to identify the source program after heavy optimization,
- and how the ecosystem survives a change in runtime or concurrency semantics.

This is where many languages discover that their core design was only half a language. The missing half is not "ergonomics" in the narrow sense. It is the operational contract between the language, the toolchain, other compilers, package managers, foreign code, and long-lived codebases.

The Optic design is already better positioned than most experimental languages because it begins from explicit runtime structure, explicit summaries, explicit grades, and explicit graph-level optimization. But those strengths only help if the language also grows explicit policy for the layers that usually become folklore in other ecosystems.

This chapter therefore does two things.

First, it identifies the main gaps that still need dedicated design work before the language can credibly claim to be fully general.

Second, it uses other ecosystems as cautionary evidence, not as targets for imitation. The point is not to repeat the mistakes of C++, Rust, Java, Python, or TypeScript in a new syntax. The point is to learn which problems refuse to stay postponed.

Selected external reference points are collected in Appendix G so that the comparisons in this chapter stay tied to primary documentation rather than community mythology.

### 27.2 A map of the remaining gaps

From a distance, the remaining work looks like a long backlog. Up close, it is more structured than that. Almost every unresolved item falls into one of four recurring pressures.

**First, the language needs a compatibility story.** A prototype can survive on good will and a fast-moving compiler. A real ecosystem cannot. Module interfaces, editions, target profiles, lock snapshots, and artifact schema versions all belong here. This is the pressure C++, Go, Swift, and TypeScript make impossible to ignore.

**Second, the language needs a complete host-boundary and runtime story.** Kernels, browsers, engines, services, and plugins all eventually force the same questions: what is the memory model, what can unwind, what may callback, which runtime family is assumed, and which unsafe facts are merely local obligations rather than silent global assumptions. This is where Python's runtime shifts, Rust's FFI discipline, and Java's foreign-memory work all become directly relevant.

**Third, the language needs a durable tooling and observability contract.** Provenance through fusion, stable debug identities, profiler keys, crash capsules, and machine-readable diagnostics are not decorative extras. They are what make aggressive optimization socially affordable.

**Fourth, the language needs an ecosystem center of gravity.** A strong standard library split, a canonical package workflow, generated lock artifacts, first-party binding generation, and clear publicity classes for artifacts all belong here. Without that center, the ecosystem learns its own rules by rumor.

The encouraging point is that none of these pressures demand a second semantic core. They all reuse machinery the book already introduced earlier: explicit `Runtime` and `Project` roots, `OpticSummary`, `RegionSet`, `BoundaryContract`, target profiles, provenance-carrying CGIR, staged artifacts, and structured diagnostics. The remaining work is therefore not a search for a new theory. It is the work of extending the existing theory into the places where languages usually become operationally fragile.

#### 27.2.1 Three buckets clarify sequencing

These pressures should not all be attacked at once. The book's own architecture suggests a conservative order.

**Maturity blockers** come first: module-interface artifacts, artifact versioning, edition and migration policy, public-versus-internal artifact classes, and the standard-library/package-workflow center of gravity. Without these, the ecosystem cannot stabilize even if the compiler already works well.

**Systems-completeness blockers** come next: the formal memory model, the unsafe/FFI soundness ledger, profiler and debug provenance, crash capsules, runtime-family declarations, and boundary contracts rich enough for real host interaction. Without these, the language cannot honestly claim kernel-, browser-, database-, or engine-class generality.

**Research-expansion items** can remain later: general resumptions, distributed and GPU costates, stronger symbolic solving, deeper bootstrap compression, and heavier formal verification. These matter for power and elegance, but they do not have to precede ecosystem stability.

That ordering is intentionally conservative. A language can live for a while without captured continuations or distributed costates. It does not live well for long without interface artifacts, migration policy, and an explicit unsafe audit story.

### 27.3 Compatibility has layers, not one promise

One of the most common late-stage mistakes in language development is to say that a language is either "stable" or "unstable" without naming what is actually being stabilized. In practice compatibility has several layers, and they must be separated.

#### 27.3.1 Source compatibility

Source compatibility answers: will a program that compiled yesterday still parse, type-check, and mean the same thing today?

This is where operator precedence, keyword growth, standard-library changes, edition boundaries, and diagnostic-guided migrations live. A language that waits too long to define editions usually ends up with either permanent syntax regrets or ecosystem-wide breakage.

For Optic, source compatibility should be handled with:

- explicit editions,
- edition-aware parsing and reserved-keyword policy,
- compiler-provided migration commands,
- and stability tiers for library APIs and language features.

The current parser architecture, structured diagnostics, and fix ranking already make this plausible. The missing piece is policy, not mechanism.

#### 27.3.2 Package compatibility

Package compatibility answers: can two independently versioned libraries be built, resolved, and installed together in a reproducible way?

This is where generated lock snapshots, semver policy, target profiles, capability requirements, and build reproducibility matter. A language can delay this during the prototype period, but a self-hosting compiler, a browser engine, or a kernel module ecosystem cannot.

#### 27.3.3 ABI compatibility

ABI compatibility answers: can separately compiled binary artifacts call each other without rebuilding from source?

C++ is the canonical warning here. GCC still documents a dual ABI for libstdc++ because old and new implementations of key standard-library types had to coexist for compatibility reasons, and Clang's C++ modules documentation treats consistency requirements and ABI impacts as first-class concerns rather than side notes. This is what happens when source structure, compiled artifacts, and binary interfaces are allowed to drift apart. See Appendix G.2 for the reference points behind this comparison.

Optic should therefore avoid the vague phrase "stable ABI" unless it is immediately qualified. The language needs at least four distinct positions:

1. **stable C-facing boundary ABI** for FFI and plugins,
2. **edition-scoped source stability** for ordinary code,
3. **compiler-internal IR and metadata** that are explicitly unstable unless versioned,
4. **optional native plugin ABI** only after the runtime, memory model, and unwinding policies are frozen enough to deserve it.

The current `BoundaryContract` model already gives the right shape for this. The remaining work is to name which parts are promised, which parts are versioned, and which parts are intentionally unstable.

#### 27.3.4 Runtime compatibility

Runtime compatibility answers: can code built under one scheduler, memory model, or host-boundary policy safely interoperate with code built under another?

This matters more than many language teams expect. If the language eventually supports kernel-class targets, no-std targets, coinductive event loops, plugin loading, managed-runtime bridges, and possibly distributed or GPU targets, then the runtime contract must be versioned just as carefully as the source contract.

A plugin compiled for one memory model or callback policy must not silently load into a host that assumes another.

#### 27.3.5 Semantic compatibility

Semantic compatibility is the hardest layer and the one most likely to be forgotten. It asks whether the language still means the same thing after optimization, staging, fusion, or backend changes.

The book already answers this better than most language plans because provenance, summaries, and translation validation are treated as first-class. The missing piece is to define a stability surface for those artifacts: which provenance identifiers, debug-info mappings, and diagnostic codes are stable enough for tooling and automation to build on.

### 27.4 Five mainstream ecosystems expose five different maturity cliffs

Earlier chapters already used C++, Rust, Java, Python, and TypeScript to justify specific core choices in layout, ownership, boundary contracts, packaging, and diagnostics. At the maturity stage, the useful question is narrower: which late-stage pressure does each ecosystem make impossible to ignore?

C++ shows that modules do not dissolve artifact identity, consistency requirements, or ABI management. That is why Optic needs explicit module interfaces, explicit artifact hashing, and a compatibility policy that distinguishes source stability from binary stability.

Rust shows that layout, ABI choice, and unwind policy are semantic boundary facts rather than implementation footnotes. That is why `BoundaryContract`, layout policy, and unwind matrices must become public-facing parts of the language rather than remaining compiler folklore.

Java shows that foreign access eventually needs a safer, more structured replacement for folklore wrappers. JNI's long shadow is the clearest warning here. That is why foreign memory, binding generation, and callback contracts should stay on the main road of the language, not in an expert annex.

Python shows that packaging and runtime shifts become ecosystem-wide operational events. Reproducible environments, lock artifacts, and concurrency-model changes all spill into build tooling, extension compatibility, and deployment practice. That is why generated lock snapshots, runtime-family declarations, and staged-artifact identity need to arrive early.

TypeScript shows two things at once: a language can openly admit a soundness trade, and a compiler eventually has to own the project graph if it wants to scale. That is why Optic should keep its soundness budget explicit and should treat project-graph identity as part of the language architecture rather than as an implementation convenience.

The point of this comparison set is not to restate earlier cross-language material in full. It is to make one maturity claim precise: once a language has real users, the pressure points stop being optional. Optic's job is not to avoid them. It is to meet them with the same explicitness it already applies to costates, optics, grades, and provenance.

### 27.5 Modules, separate compilation, and artifact identity are still underspecified

The current book is strong on AST, HIR, CGIR, summaries, and generated-code artifacts. It is comparatively thin on the artifact that sits between "one file compiled" and "whole program linked": the module interface artifact.

That gap needs to be closed before the language grows a serious library ecosystem.

#### 27.5.1 What a module artifact should contain

A module artifact should not be only a list of exported names. For this language, it should also carry enough static structure that downstream compilation can continue to work at the language's natural abstraction level.

A plausible artifact should contain:

- exported type signatures,
- exported optic signatures,
- serialized `OpticSummary` data,
- dependency fingerprints,
- edition and target-profile requirements,
- boundary-contract metadata for exported foreign items,
- and enough provenance to make diagnostics deterministic across separate compilation.

This is not "too much metadata". It is exactly the information the compiler already knows and already uses. Refusing to serialize it would only force later recompilation or re-inference.

#### 27.5.2 What the build graph should key on

The current book already uses content-hash ideas for node identity and cache reuse. The same principle should be extended to modules.

A build artifact key should at least include:

- source content hash,
- edition,
- target profile,
- enabled capability set,
- transitive imported interface hashes,
- build-host policy for staged artifacts,
- and optimizer profile when it affects generated interface facts.

That last point matters because staged computation and foreign-boundary summaries can affect what downstream crates are allowed to assume.

#### 27.5.3 Why this belongs under the current model

This does not require inventing a second compiler architecture. It is a direct reuse of the existing summary discipline:

- HIR already identifies the public semantic surface,
- `OpticSummary` already describes the legal read/write and grade behavior of exported optics,
- CGIR hashes already provide stable shape information,
- diagnostics already want stable ids and spans.

The missing work is therefore serialization policy, artifact versioning, and invalidation rules.

#### 27.5.4 Public, toolchain-stable, and internal artifacts must be distinguished

The module-interface story also needs a publicity policy. A mature toolchain emits many serialized artifacts, but not all of them should become ecosystem promises.

The clean split is:

- **public stable artifacts**: module interfaces, generated lock snapshots, registry publication records, and any replay or benchmark capsule formats users exchange directly;
- **toolchain-stable artifacts**: diagnostics schemas, graph-protocol revisions, selected build-plan materializations, and debug/performance sidecars consumed by first-party tools;
- **internal artifacts**: HIR caches, CGIR caches, solver traces, invalidation journals, and provisional optimizer notes.

That distinction feeds directly into invalidation rules. Public artifacts participate in compatibility promises. Toolchain-stable artifacts participate in version negotiation and migration tooling. Internal artifacts may be dropped and regenerated whenever the compiler improves.

### 27.6 Packaging, reproducible environments, and binary distribution need their own chapter because they will not stay small

Packaging is where many technically strong languages discover that their users do not actually consume "the language". They consume a stack of tools: dependency resolution, build planning, target selection, generated bindings, binary artifacts, native dependencies, deployment conventions, and the metadata that makes all of those reproducible.

The important refinement, given the language's compile-time execution model, is that Optic does **not** need a second handwritten manifest or build DSL as the source of truth. The package and workspace description can live in the language itself as native declarations evaluated over `BuildRuntime`. What the ecosystem still needs are **compiler-generated artifacts** that serialize the results of that native description.

That distinction is worth making explicit:

- **authoring surface:** native `package`, `workspace`, and staged `build_plan` declarations in Optic source;
- **generated artifacts:** dependency lock snapshots, module-interface summaries, staged-artifact manifests, registry metadata, and binary-distribution metadata emitted by the toolchain.

This is a better fit for the rest of the architecture because it keeps packaging under the same typed model as the compiler proper. Dependency solving, target-profile selection, native-library discovery, binding generation, and artifact planning become ordinary staged optic graphs. They can be cached, diagnosed, and audited using the same `BuildRuntime`, `TargetProfile`, `BoundaryContract`, provenance, and `CompileTimeGrade` machinery that already exists elsewhere in the book.

#### 27.6.1 What is the canonical unit of publication?

For Optic there are still at least four plausible publication units:

- source packages,
- precompiled module/interface artifacts,
- native libraries or executables,
- staged data artifacts such as generated tables or pre-specialized graph fragments.

The language should support all four, but it should define one canonical **package identity** that describes them together. That identity should be derived from native declarations plus generated artifact hashes, not from a separately maintained manifest language.

#### 27.6.2 What belongs in native package/workspace declarations?

At minimum, the native build surface should be able to express:

- edition,
- package version,
- target-profile constraints,
- capability requirements,
- required runtime family,
- dependency requirements,
- boundary-contract exports,
- and whether staged artifacts are included, derived, or forbidden.

These facts belong in the source language because they influence staging, target checking, FFI legality, cache keys, and code generation. Keeping them outside the language would force the compiler to recover information that could simply be stated once.

#### 27.6.3 What is the lock story if there is no user-authored manifest file?

A language that wants reproducible builds still needs a canonical resolved snapshot. The difference is that Optic's lock state should be **generated**, not hand-authored.

A good Optic lock artifact would record:

- exact package versions,
- exact target profile and capability profile,
- exact native or staged artifact hashes,
- exact foreign-binding generator versions where generated bindings affect ABI,
- and enough build-input hashes to reproduce `BuildRuntime` decisions.

That artifact may still be stored on disk and checked into repositories when teams want that workflow. But it is no longer the semantic source of the build. It is the compiler's canonical serialization of a native staged decision.

#### 27.6.4 Why this still belongs under the current model

The package system should not invent its own notion of capability or target. It should reuse:

- target profiles from the backend chapters,
- boundary contracts from the unsafe/FFI interlude,
- staged artifact ids from the compile-time execution chapter,
- module-interface summaries derived from `OpticSummary`,
- and diagnostic identifiers from the agent-facing tooling chapters.

That reuse is the difference between a coherent language toolchain and an ecosystem that quietly forks its own metadata models.

#### 27.6.5 Native packaging example

```rust
pub let package = Package {
    name: "browser.layout",
    edition: Edition::current(),
    targets: [Target::linux_x86_64_musl(), Target::wasm32_wasi()],
    deps: [
        Dependency::registry("css.selectors").compatible("2.1"),
        Dependency::registry("gfx.raster").exact("5.0.3"),
    ],
    runtime_family: RuntimeFamily::native(),
    capabilities: [BuildCap::read_path("assets/fonts"), BuildCap::env("SDKROOT")],
    experimental: [],
};

pub fn build_plan(rt: &[SharedGrade] BuildRuntime) -> BuildPlan {
    rt.query(
        ResolveDependencies
        >>> BuildModuleInterfaces
        >>> StageSelectorAutomata
        >>> PlanArtifacts
    ).get()
}
```

This example is deliberately ordinary. A package is a value. A build plan is a staged graph. The compiler materializes the generated lock snapshot, module-interface artifacts, and staged-artifact manifests from those declarations instead of asking the user to restate the same information in another language.


#### 27.6.6 The init path should be part of the package and tooling contract

If native `package`, `workspace`, and `build_plan` declarations are the canonical authored surface, then the first-party package tool should make those declarations the default output of project creation rather than leaving new users to write them from scratch or to copy stale boilerplate.

That is why the language should treat `optic init` as more than a convenience command. It is the first ecosystem-scale decision point where the toolchain can help the user state the facts that later chapters keep insisting must be explicit anyway:

- runtime family,
- target profile,
- initial `AppWorld` shape,
- expected host capabilities and foreign boundaries,
- workspace topology,
- and the desired replay/benchmark/tooling posture.

A graph-first, wizard-driven init flow also prevents an especially common maturity failure: the ecosystem improvises several incompatible “normal” project layouts before the language has said what the canonical one is. In other words, a good init tool is part of closure, not just onboarding.

### 27.7 Runtime-model changes are ecosystem changes

The concurrency and runtime chapter already explains how grades, `HostContext`, and `ControlRuntime` provide the language with a coherent way to talk about event loops, parallel products, and liveness. What is still missing is an explicit policy for runtime-model change.

The reason this matters is visible in Python's current free-threading work. Making the GIL optional is not just "a performance feature". It changes ABI expectations, extension requirements, and the practical meaning of thread safety for user code. Python's own documentation and PEP 703 treat those consequences as real migration issues, not incidental fallout. Appendix G.5 provides the concrete Python references.

For Optic, the corresponding rule should be:

> Any change to the memory model, callback model, scheduler semantics, or host-boundary semantics is a compatibility event.

That implies several concrete policies.

- Plugin and dynamic-library metadata must declare the runtime model they were built against.
- The standard library must not ship two subtly incompatible "official" event models without making the distinction explicit in types and native package declarations.
- The memory model for atomics, volatility, address spaces, and callbacks must be frozen before a plugin ABI is promised.
- Tooling should surface runtime-model mismatches as first-class diagnostics rather than opaque linker or loader failures.

The good news is that the current architecture is already explicit enough to carry these facts. The missing work is to elevate them from internal assumptions to public contract.

### 27.8 Soundness budgets and escape hatches must stay explicit

The language cannot remain pleasant if every hard boundary forces the user into theorem-prover mode. It also cannot remain analyzable if convenience features silently bypass the model.

TypeScript is a useful contrast because it explicitly documents its choice to permit some unsound behavior for usability and JavaScript compatibility. That honesty is valuable even if the language's tradeoffs are different from Optic's. Appendix G.6 points to the relevant handbook section.

The corresponding rule for Optic should be:

> Ordinary optic composition remains sound by default. Any escape from that regime must be named, localized, and auditable.

Under the current model that means:

- no silent weakening of alias safety,
- no invisible fallback from checked grades to unchecked ones,
- no hidden optimizer assumptions across foreign boundaries,
- no unchecked control effects smuggled through ordinary lens/traversal syntax,
- no unsound subtyping rule that can be triggered accidentally.

The language already has the right raw materials for explicit escape hatches:

- `unsafe optic`,
- `BoundaryContract`,
- gradual grades,
- explicit stageability judgments,
- explicit address-space types,
- and explicit capability gating.

The missing work is editorial and procedural. The book should eventually include a short "soundness budget" chapter that states which escape hatches exist, why they exist, what proofs they bypass, and what audit obligations they create.

#### 27.8.1 A published soundness-budget ledger should become part of the toolchain contract

The missing soundness-budget chapter should not be only reflective prose. It should be a compact ledger with one row per sanctioned escape hatch.

| Escape hatch | What proof it bypasses | Why it exists | Required local contract | Diagnostic family | Audit owner |
|---|---|---|---|---|---|
| `unsafe optic` | local alias/completeness proof | raw hardware or foreign interaction | `BoundaryContract` + safety clauses | `UNS-*` | subsystem owner |
| `extern` item | language-internal ABI/layout proof | legacy interop | ABI/layout/unwind contract | `FFI-*` | binding owner |
| volatile/MMIO | ordinary reordering assumptions | devices and control registers | address-space + volatility contract | `MMIO-*` | device owner |
| inline assembly | optimizer visibility of effects | privileged or target-specific instructions | clobber/fence/privilege contract | `ASM-*` | platform owner |
| staged host access | hermetic compile-time guarantee | controlled build integration | capability declaration + determinism class | `STG-*` | build owner |
| gradual grade `?` | static precision of one grade dimension | adoption path | observation wrapper + repair flow | `GRD-*` | local module owner |

That ledger should live close to the compiler and tooling, not only in a design appendix. The compiler can then emit diagnostics that quote the exact ledger entry and tell the user which proof obligation is now manual rather than automatic. Appendix I records the shape such a ledger should take.

### 27.9 Tooling, migration, and deprecation need a public policy, not only a good compiler

A language can survive a rough optimizer more easily than it can survive a chaotic migration story.

Once other teams and codebases depend on the language, the painful questions change again:

- how are keywords added,
- how are operators retired or reinterpreted,
- how are diagnostics kept stable enough for tooling,
- what kind of autofixes are promised,
- whether formatter output is stable across editions,
- and whether a machine-readable representation of the source or summaries is treated as public or internal.

The current book already does unusually well on diagnostics. That is a strong base. The missing layer is policy.

A practical mature-language policy for Optic should include:

1. an edition mechanism,
2. a documented deprecation window,
3. a compiler `fix` mode keyed by stable diagnostics,
4. a formatter with explicitly versioned stability expectations,
5. a linter/profile system that reuses diagnostic ids instead of inventing a second namespace,
6. and a statement about which machine-readable artifacts are considered stable enough for tools.

The user's earlier suggestion that the book may eventually be easier to maintain in JSON points to the same underlying pressure. Tooling ecosystems grow around structured data. If Optic wants to be friendly to editors, refactoring tools, package tooling, and coding agents, it should eventually define stable schemas for at least:

- diagnostics,
- module interface summaries,
- benchmark/regression metadata,
- and perhaps selected HIR/CGIR export views.

#### 27.9.1 A concrete public policy is better than "we'll be careful"

A credible first compatibility and migration policy could be stated very plainly.

- new hard keywords first land as warnings or contextual keywords, then become hard only at an edition boundary;
- operator precedence may not change within an edition line;
- formatter output is stable within an edition and target-independent except for explicit line-ending policy;
- `optic fix`, `optic migrate`, and edition-lint modes are first-party tools rather than community afterthoughts;
- every machine-readable artifact is labeled `public-stable`, `toolchain-stable`, or `internal`;
- diagnostic codes remain stable across editions unless the underlying rule disappears entirely, in which case the old code remains reserved with migration notes.

These rules are not glamorous, but they prevent the ecosystem from learning compatibility policy by rumor.

### 27.10 Debuggability, crash reporting, and profiler identity still need dedicated treatment

The book already insists that provenance survive fusion, which is the right foundation. What it does not yet describe in enough detail is the full debug/profiling/crash pipeline for a mature toolchain.

The missing questions include:

- how fused loops map back to source lines and optic names in DWARF or equivalent debug info,
- how staged artifacts preserve source identity,
- how panic/unwind/fault boundaries report boundary contracts,
- how the compiler can package a minimal reproducer when a crash crosses multiple fused nodes,
- and how benchmark or profiler samples remain comparable across editions and backends.

These are late-stage issues because they matter most once optimization becomes aggressive and third-party tooling begins to depend on the optimizer's output. They are also exactly the place where the current graph-first architecture is strongest. Source mapping should be easier here than in conventional compilers because the optimizer never had to rediscover the user's structure after lowering.

That strength is worth turning into a dedicated chapter later. The book should eventually contain a chapter on:

- debug info strategy,
- profiler attribution,
- crash capsules,
- stage artifact provenance,
- and translation validation for optimized code.

#### 27.10.1 Minimum mature debugging and profiling contract

Before the language can call itself operationally mature, the toolchain should guarantee at least the following.

- fused loops retain a logical callsite chain or provenance table in debug info;
- staged artifacts record their origin module, stage key, and source provenance;
- crash reports can emit a minimal reproducible capsule keyed by interface hashes and fused-node provenance;
- benchmark baselines and profiler samples are keyed by semantic `PerfKey`, target profile, and backend family rather than by raw symbol names;
- and translation-validation failures between backends preserve the same source-facing ids.

This is what makes optimization socially affordable. Engineers only accept aggressive fusion and staging when those transformations remain explainable after a production crash or regression.

### 27.11 The standard library, runtime surface, and package tool need an explicit center of gravity

Languages often fragment not because the core language is weak, but because the first wave of users grows several incompatible unofficial answers to the same operational question.

Examples include:

- multiple competing package workflows,
- multiple incompatible async runtimes,
- multiple partially overlapping FFI wrapper conventions,
- several no-std or freestanding subsets,
- incompatible plugin-loading models,
- and ad hoc generated-binding tools.

The current book already proposes a tiny `optic-runtime` for the prelude and explicit host-costate modeling for the rest. That is a good start. But the language will still need to say which tools and runtime surfaces are "canonical enough" that ecosystem code can depend on them.

The goal is not to monopolize experimentation. The goal is to prevent structural fragmentation in the areas where interoperability matters most.

For Optic, the most important candidates for a canonical first-party story are:

- native package declaration format and generated lock snapshot format,
- module interface artifact format,
- standard diagnostics schema,
- boundary-contract declaration and binding-generation pipeline,
- replay and benchmark artifact format,
- and the hosted versus `no_std` runtime split.

#### 27.11.1 The center of gravity should be organized in rings

The language does not need one enormous blessed platform. It does need a clear center of gravity.

A useful division is:

- **core standard library** for universally available language-facing types and algorithms;
- **runtime families** for hosted, `no_std`/freestanding, and later specialized embeddings, with explicit compatibility matrices;
- **first-party toolchain ecosystem** for package workflow, interface artifacts, binding generation, replay and benchmark formats, graph protocol, and plugin policy.

The key governance rule is that code should not need to guess which ring a dependency belongs to. Hosted versus freestanding, canonical package workflow, plugin loading, and async/coinductive runtime discipline must all have clear first-party answers even if alternative experiments remain possible.

### 27.12 What the current model still needs before it is operationally complete

By this point the remaining work is no longer mysterious. It falls into a small number of concrete chapters, schemas, and policies that the language still needs to write down. The list below is best read as a sequence of maturity passes, not as a pile of disconnected future work.

#### 27.12.1 Sequencing relative to the milestone ladder

The first wave belongs before M7: compatibility policy, module-interface artifacts, and native package declarations with generated lock snapshots. Those three items determine how the language will grow a library ecosystem, and they are harder to retrofit than they look.

The second wave belongs before M9: conformance and multi-implementation discipline. Self-hosting only becomes meaningful once the seed compiler and the self-hosted compiler can be compared against the same artifact schemas, diagnostics, and benchmark keys.

The third wave belongs before M10: the formal memory model, the debug/profiler/crash-provenance story, and the runtime-family/plugin policy. Kernel-class claims and aggressive backend optimization are simply too strong to make without those contracts in place.

Beyond that lies full maturity work: binding generation, standard-library governance, and the long tail of conformance infrastructure. Those are not optional, but they benefit from a real ecosystem already existing.

#### 27.12.2 A. Compatibility and edition policy

This work should define the edition mechanism, the deprecation window, the migration-tool contract, and the stability tiers for language and library features. The compiler already has the technical ingredients — edition-aware parsing, structured diagnostics, and fix-oriented tooling. What is missing is a public policy that tells users when and how change is allowed.

#### 27.12.3 B. Module interface and separate-compilation artifacts

This work should define the serialized interface artifact, the dependency and invalidation rules that govern it, the artifact-hashing policy, and the conditions under which a target or runtime-profile change forces rebuild. The current model already has the right raw material in `OpticSummary`, provenance, CGIR identity, and target profiles; it now needs a stable wire format and versioning discipline.

#### 27.12.4 C. Native package declarations, generated lock snapshots, and binary artifact story

This work should define the canonical native package/workspace surface, the generated lock artifact, staged and binary artifact metadata, supply-chain provenance hooks, and system-package integration policy. Because the build language is native Optic code, the real design job is not inventing another manifest DSL. It is defining the generated artifacts that serialize the results of that code in a reproducible way.

#### 27.12.5 D. Formal memory model chapter

This chapter needs to freeze pointer provenance, volatile and atomic semantics, fence and barrier rules, callback and interrupt visibility, and the stability policy for those rules across editions and runtime families. Until this exists, claims about kernels, plugins, DMA, or aggressive low-level optimization remain necessarily incomplete.

#### 27.12.6 E. Debug/profiling/crash provenance chapter

This chapter should define the debug-info mapping policy, profiler attribution format, crash capsules, and minimized reproducible failure bundles. The book already has the right conceptual foundation in provenance-preserving CGIR. What is missing is a stable external contract for the debugging and profiling ecosystem that grows around it.

#### 27.12.7 F. Foreign binding-generation chapter

This work should cover header and schema import, generated binding provenance, boundary-contract inference and override rules, regeneration workflows, and how generated bindings interact with lock snapshots and module interfaces. Hand-authored `extern` blocks are a good start, but not a complete long-term interop story.

#### 27.12.8 G. Conformance and multi-implementation discipline

This work should define conformance suites, normative artifact schemas, translation-validation expectations, and an implementation-divergence policy. The language's self-hosting ambition, multi-backend ambition, and tooling ambition all depend on this becoming explicit earlier than feels comfortable.

#### 27.12.9 H. Runtime-family and plugin policy

This chapter should define runtime-family identifiers, plugin ABI/runtime declarations, managed-runtime bridge guarantees, and callback contracts by runtime family. Once hosted, freestanding, kernel, event-loop, and managed embeddings all exist, the language needs a public way to say which world a package or artifact assumes.

#### 27.12.10 I. Standard library scope and governance

This work should fix the criteria for first-party libraries, the split between core and batteries, the stability promises by crate or package tier, and the deprecation/removal process. Without that center of gravity, the ecosystem will eventually rediscover several incompatible “normal” ways to solve the same operational problem.

#### 27.12.11 J. A practical research ladder for v1 stabilization

Not every unresolved semantic question needs the richest available mathematics first. For v1 the language should prefer the **smallest theory that closes the operational gap** while still fitting the existing costate/optic/summary architecture.

The direct experimental candidates are:

- **resource and separation reasoning** for ownership, provenance, unsafe boundaries, and local alias proofs;
- **weak-memory and ordering semantics** for atomics, fences, volatility, DMA visibility, and backend reorder legality.

These are more direct answers to the current memory-model backlog than starting with sheaf, topos, or HoTT machinery. They fit the present architecture with fewer new concepts because they already speak in terms of resources, accesses, obligations, and reorderings.

Alongside those direct lanes, the language should keep several **simpler sidecars** available while the core stabilizes:

- **typestate and explicit protocol automata** as a narrower first answer before richer session or sheaf machinery;
- **TLA+ and Alloy** as external specification/model-checking companions for callback, protocol, scheduler, and distributed-runtime design;
- **abstract interpretation** as a conservative optimizer and checker companion before proof-heavier machinery is justified.

The richer experimental lanes — `std.experimental.proof`, `std.experimental.sheaf`, `std.experimental.topos`, and `std.experimental.dynamics` — remain valuable, but they should compete against these smaller answers rather than being assumed to be the first step.

A good working rule for the first stabilized line of the language is therefore:

- keep **one direct internal stack** for the memory-model backlog — access-based regions, fractional/resource ownership, weak-memory legality, and proof/translation witnesses;
- keep **one sidecar stack** for protocol and approximation work — typestate/protocol automata, TLA+, Alloy, and abstract interpretation;
- treat richer proof, sheaf, topos, and dynamics lanes as **second-wave refinements** that must show what they can answer that the two simpler stacks cannot.

This is the smallest coherent research stack that still aligns with the rest of the architecture. It avoids forcing the v1 compiler, package tool, and runtime-family policy to absorb every available theory before the narrower answers have been exhausted.

It also makes one limit explicit: **no single experimental lane answers all of the remaining obligations**. The language still needs a stack, not a silver bullet. The direct lanes answer the memory-model core, the sidecars answer many protocol and approximation questions cheaply, and the richer second-wave tracks remain available when the first two layers stop being enough.

### 27.13 More mainstream languages still expose different failure surfaces

Section 27.4 already covered the five ecosystems most repeatedly used earlier in the book. This section adds a different set of lessons: runtime-family clarity, binary-evolution policy, compile-time latency discipline, and mixed-language adoption pressure. The question stays the same — which pressure became non-negotiable once a language stopped being a research artifact and became an ecosystem? — but the examples are chosen to add new constraints rather than to repeat the earlier comparison set.

#### 27.13.1 A compact map before the detailed discussion

The additional languages in this section matter because they each make a different maturity pressure impossible to ignore.

Go foregrounds the value of one compiler-owned workflow and a narrow, explicit compatibility promise. Swift makes binary evolution and module stability impossible to treat as afterthoughts. Kotlin shows that gradual interop has to feel routine if a new language wants adoption inside existing codebases. C# and .NET force runtime-family diversity into the open. Node.js and Elixir demonstrate that a runtime model can become part of a language's public identity rather than just a library choice. Julia turns compile-time latency into a product issue. Scala reminds us that macros, plugins, and migration boundaries can quietly become a second language if they are not constrained early.

The detailed subsections that follow draw out those pressures one by one. The goal is not breadth for its own sake. It is to widen the chapter's evidence base without repeating the same earlier comparison logic in a new table.

#### 27.13.2 Go: compatibility promises and one-tool workflows reduce ambient uncertainty

Go is an especially useful contrast because many of its strengths are social-architectural rather than type-theoretic. The language and toolchain deliberately narrowed the number of moving pieces that ordinary developers had to reason about: a source-compatibility promise under Go 1, one primary `go` command, and a module system owned by the toolchain rather than delegated entirely to third-party convention.

The key lesson is not that Optic should become minimal in the Go sense. The key lesson is that a language earns trust when it makes the boundary between "the language" and "the build/package tool" unusually crisp.

For Optic, the analogous guardrails are:

- the compiler should own module interface artifacts, not merely consume ad hoc metadata files,
- package identity and checksums should be compiler-visible facts, not only package-manager facts,
- and the compatibility promise should be written down in layers.

Go is also a warning against a common mistake. The Go 1 promise is source-level; it is not a blanket promise of compiled binary compatibility. That separation is healthy. Optic should make the same distinction explicit instead of allowing source compatibility, serialized module-interface compatibility, plugin ABI compatibility, and runtime-family compatibility to blur into one marketing phrase.

Cross-reference this directly with §§27.3, 25.5, and 25.6. The book already argues that compatibility has layers and that package metadata needs its own chapter. Go strengthens that argument by showing that explicitness itself is a large part of the ergonomic win.

#### 27.13.3 Swift: ABI, module stability, and library evolution became separate jobs

Swift is the clearest industrial example of a language discovering, in public, that source language design is only one part of ecosystem stability. ABI stability, module stability, and library evolution had to be named separately because they solve different problems:

- ABI stability answers whether binaries compiled with different compiler/runtime versions can interoperate.
- Module stability answers whether one compiler can understand another compiler's module artifact.
- Library evolution answers how a library changes without forcing all clients to rebuild.

The important point for Optic is not the specific Swift mechanism. It is the deeper systems lesson:

> once binaries, plugins, drivers, or separately shipped libraries matter, resilience is a costed feature, not a free slogan.

Swift is explicit that library-evolution support changes performance characteristics and even some source-language behavior around exhaustiveness. That is the kind of honesty Optic will need once it moves from "compile everything together" to plugin ecosystems, driver surfaces, or binary-distributed runtime libraries.

The current model already has the right seeds for this:

- `TargetProfile`
- `BoundaryContract`
- staged and target-specific artifacts
- explicit provenance and summaries

What is missing is a first-class *resilience policy*. The day-0 rule should be: if a package wants a stable binary surface, it opts into a stronger and more expensive contract. Otherwise the compiler is free to keep the stronger whole-program assumptions that make the generated code better.

#### 27.13.4 Kotlin: adoption accelerates when interop is gradual, not all-or-nothing

Kotlin's most important non-theoretical lesson is that a language can gain real adoption by making coexistence with legacy systems feel routine. Kotlin's Java interop is not treated as a side door. Mixed Kotlin/Java builds, API-shaping rules for Java-facing Kotlin code, and migration tutorials are all first-class parts of the official story.

That matters directly to Optic's full-generality goal. If the language is supposed to grow into kernels, browsers, compilers, games, databases, and services, it will spend a long time embedded in mixed environments:

- kernels with C and assembly,
- engines with C++ and scripting,
- enterprise systems with Java and .NET islands,
- existing Rust/C libraries,
- vendor SDKs,
- and generated or imported bindings.

The guardrail is straightforward:

> mixed-language builds and generated foreign bindings must be part of the happy path, not a heroic path.

This reinforces the earlier boundary-contract work rather than replacing it. `BoundaryContract` stays the semantic core. The extra requirement is operational: the package tool and module system must make "Optic plus legacy code" feel like one supported mode of development rather than an unofficial workaround.

#### 27.13.5 C# and .NET: one language may need several runtime families at once

The .NET ecosystem is useful because it demonstrates that a successful language may need to inhabit multiple deployment families simultaneously:

- JIT-compiled, managed, runtime-hosted execution,
- ahead-of-time native deployment,
- restricted environments where JIT is disallowed,
- and generated interop code that moves work from runtime to build time.

For Optic, this is a direct confirmation of a theme already emerging elsewhere in the book: runtime family must be a named part of the architecture. A language that wants to span hosted services, freestanding utilities, kernels, tooling, compile-time graph evaluation, and embedding into managed runtimes cannot pretend these are all one operational environment.

The current model can absorb this with minimal extension if the team keeps reusing existing concepts:

- extend `TargetProfile` with `RuntimeFamily`,
- keep `BuildRuntime` distinct from ordinary `Runtime`,
- and let `BoundaryContract` describe whether a foreign boundary is hosted, freestanding, managed, callback-heavy, unwind-capable, or AOT-only.

The deeper lesson from .NET is that interop generation is not a secondary tool once performance and deployment diversity matter. It becomes part of the compiler story.

#### 27.13.6 Node.js / JavaScript: the event loop becomes part of the language's practical identity

Node.js is valuable here not because Optic should imitate JavaScript, but because Node makes one fact unusually obvious: the runtime's concurrency shape becomes one of the language's public design decisions. The event loop is not merely a library convention. It governs what kinds of workloads are natural, which kinds of bugs dominate production, and how users reason about blocking.

That observation strengthens the earlier claim that Optic should never speak about "async" as if it were one thing. The language's own concepts already point in a better direction:

- `Coinductive` nodes,
- `LivenessGrade`,
- `BlockingGrade`,
- `HostContext`,
- and explicit driver/runtime families.

The right extension is therefore not a new async syntax regime. It is a clearer operational taxonomy: event-loop runtime, work-stealing shared-memory runtime, isolated-process runtime, freestanding/kernel runtime, and build-time runtime. Different domains will use different families, but they should all remain visible in the same artifact model.

#### 27.13.7 Elixir: isolated lightweight processes show that runtime semantics can be a product feature

Elixir and the BEAM ecosystem show the opposite pole from Node.js. Instead of a single-threaded event loop with kernel offload, the runtime is built around isolated lightweight processes, message passing, supervision, and fault containment.

The relevant lesson for Optic is not that it should turn into an actor language. It is that runtime semantics can be so central that they become part of the language's market identity. Once that happens, leaving runtime family implicit becomes dangerous. Users start writing systems that depend on hidden operational assumptions.

Optic should therefore treat runtime family the same way it treats ownership and host boundaries: explicit, typed, and inspectable. A coinductive driver that targets an event loop is not the same thing as a process-supervision runtime, even if both are expressible in the same surface language.

#### 27.13.8 Julia: compile-time latency is a product issue, not a side note

Julia is an especially important comparison for Optic because it validates the book's recent shift toward explicit compile-time execution. Julia's own documentation now treats package images, ahead-of-time compilation, and time-to-first-execution as first-class engineering topics. That is exactly the mindset Optic should adopt early.

If Optic intends to make staging and compile-time graph evaluation central, then three things need to be true from day 0:

1. compile-time work is represented explicitly (`BuildRuntime`, stageability judgment, compile-time artifacts),
2. compile-time cost is budgeted explicitly (`CompileTimeGrade`, stage boundaries, cache keys),
3. and compiled/staged results are cacheable as ordinary artifacts, not hidden compiler ephemera.

This is one of the strongest arguments for keeping the macro-free design. A second meta-language would make compile-time work harder to budget, cache, explain, and attribute. Ordinary optic graphs over `BuildRuntime` keep it under the same model as the rest of the language.

#### 27.13.9 Scala: migration cliffs often come from macros, plugins, and ecosystem coupling

Scala is already discussed earlier in this part of the book, but it belongs in this broader comparison set as well because it is one of the clearest examples of a language whose core ideas were strong enough for major adoption in some domains, while ecosystem evolution still had to contend with macro libraries, compiler plugins, and migration prerequisites.

The architectural lesson is straightforward:

> metaprogramming power should not be allowed to become a second, unstable compatibility surface by accident.

For Optic, that means the default metaprogramming path should remain staged graph transformation over explicit IR-like structures. If the language later admits plugins or richer compile-time APIs, they should ride on versioned, constrained schemas rather than compiler-internal structures.

### 27.14 Languages with strong ideas that stayed niche still matter

It is easy to learn the wrong lesson from niche languages. The wrong lesson is that their ideas were bad. The more useful lesson is usually that the ideas were real, but the surrounding adoption costs stayed too high for broad uptake.

That is exactly why they are valuable to study.

They often reveal failure modes earlier and more starkly than mainstream languages do.

#### 27.14.1 D: a systems language should not need a second personality to interoperate cleanly

D is a particularly important cautionary example for Optic because it lives close to the same aspiration space: systems programming, native code, performance, safer subsets, and C interop.

Its `@safe`/`@trusted`/`@system` split is useful. Its BetterC subset is also revealing for a different reason. The official documentation is explicit that linking D into C is not straightforward, and that BetterC exists as a restricted subset precisely to avoid depending on the D runtime. The documentation also warns that BetterC use can lead to cryptic compiler or linker errors when libraries assume more runtime support than the subset provides.

The design lesson for Optic is strong:

> do not let hosted Optic, freestanding Optic, kernel Optic, and interop Optic become four different personalities of the language.

The current architecture can avoid that if it keeps using:

- `TargetProfile`,
- `RuntimeFamily`,
- `BoundaryContract`,
- `unsafe optic`,
- and capability-gated host costates.

That is the minimal-extension way to absorb the hard realities D exposes without inventing a separate BetterOptic dialect.

#### 27.14.2 Nim: pleasant syntax does not rescue weak artifact and package discipline

Nim is a useful caution because it shows that a friendly surface language and straightforward C interop do not automatically produce a broadly trusted operational ecosystem. Nimble is practical and lightweight, but its own guide still reflects a world where external VCS tools are assumed and where package resolution behavior follows repository tags and, in their absence, even the latest commit. That is workable for an enthusiast ecosystem. It is a weak default for a language that wants kernel-class tooling, long-lived services, or reproducible industrial builds.

The relevant Optic lesson is not about syntax at all. It is about artifact determinism:

- package resolution must be reproducible,
- lock/checksum behavior must be part of the standard workflow,
- and generated bindings or foreign summaries must participate in that same artifact identity.

This fits naturally with the current model if module interface artifacts, staged artifacts, and boundary-generated code all share the same hash/manifest discipline.

#### 27.14.3 Pony: very strong concurrency semantics can narrow a language's perceived domain

Pony's actor model and reference capabilities are intellectually impressive. The language promises data-race freedom, no locks, and capability-checked concurrency without runtime overhead. But the tutorial also makes clear just how much of the language's everyday mental model is organized around actors, asynchronous behaviours, and a capability lattice with several modes.

The lesson is not that Pony's model is wrong. The lesson is that a language can ask too much cognitive buy-in up front if its most unusual subsystem becomes the default way to think about all programs.

Optic should avoid that trap. It already has a better route available:

- keep `Runtime` and optics as the general base model,
- treat actor/process/event-loop/shared-memory execution as runtime-family choices,
- and keep advanced concurrency structure visible where it matters without requiring every introductory program to internalize the whole concurrency calculus.

#### 27.14.4 Idris: proof power is real, but proof burden changes the social surface of the language

Idris is a classic example of a language whose ideas have influenced the broader field without itself becoming a mainstream implementation language. The official tutorial and docs foreground dependent types, totality, and the fact that only total functions are safe to evaluate during type checking. The official site also still describes Idris 2 as a work in progress.

Those are not flaws. They are signals of what kind of ecosystem burden a dependently typed language carries:

- proof obligations are real work,
- compile-time execution must be tightly controlled,
- and tool maturity matters enormously because the type checker is no longer just checking simple shape compatibility.

The Optic lesson is not to abandon expressive static reasoning. It is to stratify it.

Routine programming should stay inside a fast, predictable, executable core. Stronger proof-oriented modes should be opt-in and local: gradual grades, checked unsafe wrappers, staged graph proofs, or dedicated verification-oriented crates and chapters.

#### 27.14.5 ATS: theoretical power alone does not produce a large ecosystem

ATS is another language that deserves respect rather than dismissal. Its own description emphasizes dependent types, linear types, formal specification, and C-level efficiency. That is an extraordinary technical package.

It is also a warning. A language can be extremely powerful and still remain niche if:

- ordinary programming feels like theorem-guided programming too early,
- the tooling surface is too specialized,
- or the migration and interop story asks too much up-front expertise.

For Optic, this reinforces the narrow-v0-first doctrine. The language should not make users pay proof-theory prices in the common case if its public ambition is broad systems adoption.

#### 27.14.6 These niche cases should be read as design warnings, not as dismissals

D, Nim, Pony, Idris, and ATS did not "lose" because their ideas were empty. Many of their ideas continue to shape other languages.

The more accurate interpretation is this:

- D warns about split language personalities.
- Nim warns about package and artifact looseness.
- Pony warns about making one advanced runtime model the default way to think.
- Idris warns about proof burden and tool maturity.
- ATS warns that raw expressiveness is not a substitute for adoption architecture.

Optic can benefit from all of them if it converts those warnings into day-0 constraints rather than into retrospective regrets.

### 27.15 The recurring reasons strong languages stall or fracture

Looking across both mainstream and niche languages, a surprisingly small set of recurring patterns appears.

#### 27.15.1 Pattern 1: too many new mental models arrive at once

When a language asks users to adopt a new memory model, new concurrency model, new package model, new macro model, and new proof model at the same time, even strong ideas become socially expensive. Pony, Idris, and ATS reveal this starkly. Scala also exposed a milder form of the same problem around macros and contextual abstractions.

**Optic guardrail:** keep the prelude psychologically small even if the long-term architecture is large. Advanced grades, coinduction, richer optics, and proof-oriented staging should layer on top of one executable core rather than replacing it.

#### 27.15.2 Pattern 2: interop exists, but only through a second personality of the language

D's BetterC story is the clearest warning. As soon as a language needs a special freestanding/interoperable subset, users start learning two models and toolchains start encoding special cases.

**Optic guardrail:** model all hostile boundaries through `BoundaryContract`, `unsafe optic`, `TargetProfile`, and `RuntimeFamily` rather than through a second sublanguage.

#### 27.15.3 Pattern 3: package and artifact determinism arrive late

Python had to grow standard environment and lockfile work. Nimble exposes the cost of a lighter package discipline. Go demonstrates the opposite: the module file is part of the ordinary compiler/tool story.

**Optic guardrail:** ship one canonical native workspace/package declaration surface, one compiler-generated lock/checksum story, and one compiler-visible module-interface artifact before the ecosystem improvises its own.

#### 27.15.4 Pattern 4: the runtime family remains implicit for too long

Node.js, Elixir, Python free-threading, and .NET Native AOT all show that runtime-model changes are not merely implementation details. They alter libraries, deployment, interop, debugging, and performance expectations.

**Optic guardrail:** runtime family must appear in native package/workspace declarations, generated module-interface artifacts, diagnostics, and target profiles.

#### 27.15.5 Pattern 5: compile-time work is treated as free until users revolt

Julia's package images and AOT systems make the issue explicit. Scala's macro and plugin migration pain is a related story from the metaprogramming side.

**Optic guardrail:** treat compile time and cold-start latency as budgeted resources. `BuildRuntime`, stageability, compile-time caches, and `CompileTimeGrade` already point in that direction and should remain central.

#### 27.15.6 Pattern 6: compatibility is promised too vaguely

Go shows the value of a narrow, explicit source compatibility promise. Swift shows how ABI, module, and library evolution become separate operational concerns. C++ shows how modules do not erase ABI or build consistency.

**Optic guardrail:** define compatibility as a matrix, not a slogan:

- syntax/edition compatibility,
- source compatibility,
- module-interface compatibility,
- package/build reproducibility,
- binary/plugin ABI compatibility,
- runtime-family compatibility.

#### 27.15.7 Pattern 7: macros, plugins, and code generation quietly become a second language

Scala is the strongest warning here, but many ecosystems run into the same issue. Once code generation depends on unstable internal compiler APIs, the language acquires a shadow language that is hard to migrate and hard to specify.

**Optic guardrail:** keep compile-time programming inside ordinary staged optic graphs as long as possible. If plugins exist later, make them schema-driven and versioned.

#### 27.15.8 Pattern 8: the standard library and package ecosystem develop without a center of gravity

Many ecosystems eventually accumulate multiple partially incompatible "normal" ways to solve the same runtime, package, async, FFI, or plugin problem.

**Optic guardrail:** name the first-party center early: native workspace/package declarations, generated lock artifacts, diagnostics schema, boundary contracts, replay artifacts, and runtime-family declarations.

### 27.16 Architecting against those failures from day 0

The most useful response is not a warning list. It is a concrete set of architectural rules that translate the warnings into design constraints.

The next section closes that argument by turning those rules into a hard classification scheme for future growth.

#### 27.16.1 Keep one semantic center

The language should continue to treat `Runtime`, `HostContext`, `ControlRuntime`, `BuildRuntime`, optics, summaries, and boundary contracts as one connected model.

That single-model discipline is the main protection against D-style split subsets, macro shadow-languages, and ad hoc runtime families.

#### 27.16.2 Make compatibility a typed matrix, not a promise paragraph

Add explicit fields to native package/workspace declarations and generated module-interface artifacts for at least:

- `edition`,
- `target_profile`,
- `runtime_family`,
- `interface_schema_version`,
- `boundary_contract_schema_version`,
- and package checksum / artifact identity.

This is the minimal extension that absorbs Go's compatibility clarity, Swift's resilience distinction, and C++'s module/ABI separation.

#### 27.16.3 Treat module interfaces as serialized summaries, not opaque compiler trivia

The current architecture already revolves around `OpticSummary`, `RegionSet`, provenance, target profiles, and boundary contracts. That is enough to define a stable module-interface artifact.

Such an artifact should serialize:

- exported type and optic signatures,
- grades and constraints,
- region and boundary summaries,
- runtime-family declarations,
- target-profile assumptions,
- and artifact hashes of staged/generated subgraphs.

That is the direct way to prevent separate-compilation drift from becoming folklore.

#### 27.16.4 Keep interop on the main road

Interoperability should be built around one official pipeline:

1. import or declare foreign surfaces,
2. infer or author `BoundaryContract`s,
3. generate wrappers/bindings,
4. wrap unsafe leaves with safe optics,
5. publish the resulting summaries in the same module-interface artifact.

This is how Optic can borrow the good lesson from Kotlin and Java/Panama without turning into a language where unsafe interop lives outside the model.

#### 27.16.5 Do not create a BetterOptic subset

Hosted service builds, `no_std` utilities, kernel code, plugins, and build-time graph execution should all stay in the same language.

The legal differences should come from:

- `TargetProfile`,
- `RuntimeFamily`,
- capability-gated host costates,
- and `BoundaryContract` restrictions.

This is the clean way to incorporate D's warning while reusing existing concepts.

#### 27.16.6 Make package resolution and artifact identity reproducible from the beginning

The package tool should not be a later add-on. It should understand:

- native package/workspace declarations,
- generated lock snapshots and checksums,
- module-interface artifacts,
- generated bindings,
- staged artifacts,
- and target/runtime-family splits.

That is how Optic avoids recreating Python's and Nim's later packaging pressure in a new form.

#### 27.16.7 Treat compile-time work as a resource with budgets and caches

The existing compile-time execution model should be deepened, not replaced.

Specifically:

- `BuildRuntime` remains explicit,
- stageability remains a judgment over ordinary graphs,
- compile-time caches remain artifactized,
- and `CompileTimeGrade` remains the place where cold-start and build-time cost become visible.

This is the architectural response to Julia's lesson and to macro/plugin ecosystems that become opaque performance sinks.

#### 27.16.8 Keep advanced static reasoning opt-in and layered

The language should welcome richer grade dimensions, verification-oriented stages, and stronger proof machinery later, but ordinary production programming should still be possible within the executable core.

That means:

- fast prelude diagnostics,
- gradual grades where appropriate,
- local unsafe/boundary obligations,
- and specialized proof-heavy workflows that do not infect the everyday build.

This is how Optic can benefit from Idris-, ATS-, and Pony-like lessons without inheriting their adoption friction.

The practical corollary is that the v1 research program should prefer **smaller direct theories first**: resource/separation reasoning and weak-memory operational models before richer categorical machinery for the same questions, typestate before fully general protocol theories where that suffices, and sidecar specification/model-checking tools before new core syntax when the latter would only duplicate the former.

#### 27.16.9 Constrain metaprogramming and compiler extension points early

The safest default remains:

- stage ordinary optic graphs,
- serialize structured artifacts,
- avoid freeform AST-rewriting macros,
- and keep any future plugin system schema-driven and editioned.

This is the cleanest response to Scala-style migration cliffs and to the general tendency of macros to become a second unstable language.

#### 27.16.10 Make runtime families explicit everywhere they matter

A package, module artifact, benchmark, replay artifact, and diagnostic should all be able to say which runtime family they assume.

That includes at least:

- build-time runtime,
- hosted shared-memory runtime,
- event-loop/coinductive runtime,
- isolated-process/supervision runtime,
- freestanding runtime,
- kernel runtime,
- managed-host embedding runtime.

This is how Node.js, Elixir, .NET, and Python's runtime shifts are converted into a reusable Optic rule.

#### 27.16.11 Design migration tools before the first ecosystem fracture

The compiler already has strong diagnostics. That foundation should grow into:

- edition-aware rewrites,
- schema migrations for module artifacts,
- package-declaration schema migrations,
- generated-binding regeneration workflows,
- and source-format stability rules.

Go, Swift, and Scala all suggest the same thing from different angles: migration cannot remain improvised once the language is real.

#### 27.16.12 Publish the conformance and artifact story earlier than feels necessary

Because Optic wants to be self-hosting, performance-sensitive, and eventually multi-backend, it should publish conformance suites, artifact schemas, and translation-validation expectations earlier than most ecosystems do.

This is not bureaucracy. It is the only reliable way to stop later disagreements from becoming silent language forks.

#### 27.16.13 Stress-test domain collisions before v1 claims harden

A language that aims at kernels, browsers, databases, games, services, and compilers should not rely only on isolated domain benchmarks when deciding whether a feature is ready. Mixed-domain collision suites — bounded render loops fed by network ingress, transactional wrappers over legacy engines, experimental kernels embedded in ordinary traversals — are better proofs that the single-graph architecture is really holding.

#### 27.16.14 Experimental mathematics must remain subordinate to the ordinary summary path

Experimental tracks are valuable only if they enrich the existing architecture rather than fork it. The acceptance test for any mathematical extension is therefore whether it still lowers into ordinary summaries, grades, boundary contracts, and target-profile assumptions. If it needs a parallel legality pipeline, it belongs in research infrastructure, not in the language roadmap.

### 27.17 Closing the Design: Core, Boundary, or Never

By this point the book has enough architecture that future growth should stop being discussed as if every new use case deserves a new core mechanism. The language will stay coherent only if it becomes harder, not easier, to grow the surface irresponsibly. The right closure rule is therefore simple: every future demand must be classified as **core**, **boundary**, or **never in core**.

That closure rule now has one deliberate safety valve: a **quarantined experimental lane**. Experimental mathematics, proof machinery, and domain-specific research ideas may be implemented and benchmarked under the reserved `experimental` contextual keyword, the `std.experimental` namespace root, and the graph-native `ExperimentalArena`. They do **not** become part of the ordinary core surface merely because they are implemented. The point of the lane is to keep research inside the same semantic graph while keeping promotion into core rare and reviewable.

#### 27.17.1 What belongs in core

Core features are the ones that must participate directly in the language's ordinary semantic pipeline:

- explicit `Runtime`, `BuildRuntime`, and `Project` roots,
- optics and ordinary staged evaluation,
- `OpticSummary`, `RegionSet`, grades, and determinism classes,
- CGIR and provenance,
- native package/workspace declarations,
- ordinary optic-based queries over `Project`,
- and optics-native domain libraries such as `std.solver.*` when they lower through the ordinary summary and staging pipeline.

A feature belongs in core only when an existing optic, staged build graph, boundary contract, or generated artifact cannot already express the use case without losing semantic truth. The goal is not minimalism for its own sake. The goal is to keep the semantic center small enough that every core feature still enjoys the full summary, legality, provenance, and backend story.

#### 27.17.2 What belongs at the boundary

Boundary features are fully supported, but they are supported as explicit edges of the model rather than as new centers of it. They include foreign ABIs, raw hardware access, managed-runtime bridges, plugin ABIs, registry and package fetches, generated bindings, projection filesystems, direct graph protocols, and imported external package interfaces.

Their common rule is that they must pass through `BoundaryContract`, generated interface artifacts, runtime-family declarations, capability declarations, imported-front-end lowering, or compiler-owned materializations. The language covers the use case, but it does not multiply the number of semantic worlds in which that use case can be expressed.

#### 27.17.3 Permanent “never in core” table

| Tempting feature or promise | Why it is attractive | Why it stays out of the language core | Supported instead via |
|---|---|---|---|
| a second authored build/config language | familiarity from TOML/YAML/CMake-style ecosystems | creates a second semantic center and duplicates build facts the compiler already knows | native `package` / `workspace` declarations plus generated lock, interface, and build artifacts |
| a primary syntax-rewriting macro system | short-term expressive power and ecosystem experimentation | rewrites syntax rather than semantic graph structure and tends to become a shadow language | staged optic graphs, generated artifacts, and later schema-driven plugin points |
| ambient exceptions as the default control model | terse error propagation and legacy familiarity | hides control edges from CGIR, replay, provenance, and staged legality checks | prisms, `Result`, `Option`, and a possible later explicit control layer |
| a BetterOptic-style hosted/freestanding/kernel dialect split | makes hard domains feel easier by narrowing semantics | fractures the language into personalities and duplicates runtime/toolchain policy | `RuntimeFamily`, `TargetProfile`, capability-gated host regions, and `BoundaryContract` |
| a vague blanket “stable ABI” promise | sounds ecosystem-friendly | source, module, runtime, and plugin compatibility are different contracts with different costs | explicit C-facing ABI guarantees first, opt-in plugin/runtime-family contracts later |
| multiple blessed package workflows | social pressure from different communities | fragments ecosystem identity, caching, and artifact schemas | one first-party native package/workspace/build path with generated artifacts |
| unstructured `unsafe` | convenience at hard boundaries | destroys summary completeness and auditability | `unsafe optic`, `BoundaryContract`, and the published soundness-budget ledger |
| a separate user-facing query language for `Project` | query ergonomics and tooling power | recreates a second meta-language beside ordinary code | ordinary optic-based project queries with internal QIR lowering |
| a core-language solver DSL | dense equation syntax and domain familiarity | creates a second semantic center for numerical work and duplicates staging, summaries, and artifact rules | optics-native solver libraries plus imported or embedded modeling frontends that lower into the same graph |
| plugins over unstable internal compiler IRs | fast experimentation | creates a shadow compiler API that is harder to migrate than the core language | versioned graph protocol, schema-driven tool/plugin surfaces, generated interface artifacts |

#### 27.17.4 Support use cases, not surface imitation

This is the practical feature-admission test.

1. Does the proposal unlock a genuinely new use case?
2. If so, can that use case already be covered by ordinary optics, staged build-time execution, boundary contracts, or generated artifacts?
3. If yes, do not add a new core feature.
4. If no, either admit it into core with a full summary/lowering story or keep it explicitly at the boundary.

The language should therefore measure itself against the use cases of C++, Rust, Go, Java, Python, TypeScript, and the more niche languages discussed later in this chapter without inheriting their surplus surface area. Cover the use case; do not inherit the extra semantic center that originally carried it.
A solver stack is a good concrete example. The language should absolutely cover imported equation systems, generated stencils, Jacobian builders, sparse kernels, and target-aware stepping loops. It does not follow that the core language needs a second solver calculus. In Optic, the default answer should remain optics-native solver libraries plus staged artifact generation, with any denser notation treated as a frontend rather than as a second semantics.

#### 27.17.5 Why this matters for the whole ecosystem

This closure rule does more than control feature creep. It also explains how the whole language ecosystem can live inside one larger costate graph. Registries, external packages, interface artifacts, generated bindings, replay capsules, benchmark baselines, plugin descriptors, and tool protocols all become ordinary regions, projections, or boundary edges of the larger semantic graph precisely because the language refuses to treat each new pressure as grounds for a new sublanguage. Saying “never in core” is the positive act that makes one ecosystem graph rich enough to host all of them.

### 27.18 Why these gaps still fit the current architecture

The strongest conclusion of this chapter is not that the language needs a second theoretical center. It is that the current center is strong enough to carry the missing work if the team keeps reusing it.

- Compatibility policy reuses editions, target profiles, and boundary contracts.
- Module artifacts reuse summaries, provenance, and graph hashes.
- Packaging reuses target profiles, staged artifact ids, and diagnostics.
- Runtime coherence reuses `Runtime`, `HostContext`, `ControlRuntime`, and grades.
- Soundness boundaries reuse `unsafe optic` and `BoundaryContract`.
- Debuggability reuses provenance-preserving CGIR and stable diagnostic ids.

That reuse matters. Many languages become internally inconsistent when they solve late-stage problems with ad hoc subsystems that do not share the same vocabulary as the core language. The Optic design has a chance to avoid that, but only if it keeps refusing the temptation to solve operational maturity with hidden mechanisms.

### 27.19 Implementation priorities and remaining empirical questions

This section gathers the considerations that belong in the book but do not fit neatly under any single gap category. They arise directly from the question: given the current specification, what are the next concrete decisions and risks?

#### 27.19.1 The compiler critical path

The language has a good specification and a publishable book. The most important next step is M0–M6: building the narrow v0 compiler that makes the book's claims real rather than plausible. Without running code, every subsequent claim is design fiction.

The five milestones have a specific risk ordering.

**M0–M1 (parser and HIR)** are mostly mechanical. The risk is grade inference diverging from what the specification says. The concrete two-dimensional arithmetic is simple, but the interaction between inferred grades and declared bounds at composition sites needs test coverage from day one — not as a later clean-up pass.

**M2 (typed HIR)** contains the hardest v0 problem: the alias checker. Write/write and write/read conflicts in product compositions are the first place where the theory must become executable code and produce deterministic diagnostics. The `invalid_alias.opt` acceptance example must be committed and passing before M2 is declared. The alias checker must be tested against pathological cases — deeply nested products, self-referential costates, and compositions where the conflict is in the `put_reads` field rather than `put_writes` — because `put_reads` conflicts are the most common false negative.

**M3–M4 (CGIR and fusion)** introduce the risk of optimization incorrectness. The three fusion passes must have algebraic soundness arguments written down before they are merged, not after. Provenance — the `original_ids` mechanism that keeps fused loops traceable back to source optics — is easy to underspecify and very hard to retrofit. It must be part of the fusion pass specification from the beginning, not added when debugging becomes difficult.

**M5–M6 (Rust backend and prelude release)** introduce a different risk: the generated code looks correct but is not what the benchmark baselines should be. Benchmark baselines must be committed before M6 is declared, not measured afterward. The benchmark harness should record the semantic `PerfKey` (optic name and composition shape), the target profile, and the tolerance band. Without committed baselines, "benchmarks green" has no meaning.

#### 27.19.2 Three front-loaded decisions that unblock M7

These questions are no longer left as open design suspense. They are fixed early because each one changes artifacts that the implementation should not have to retrofit later.

**Decision 1: fractional ownership is the underlying carrier from the beginning.**

The architecture now adopts fractional ownership early, with `SharedGrade`, `AffineGrade`, and `LinearGrade` retained as named surface aliases. This is the right compromise. It front-loads the hard carrier decision, preserves a friendly surface for most programs, and avoids building an alias checker that would later need a semantic transplant.

The main payoff is compositional parallel-product proofs for partition-shaped programs. Two secondary gains follow: field-level lending and stronger proof transport through composition. The book is now explicit that the multicore payoff lands later, but the *carrier decision* belongs early.

**Decision 2: asymmetric I/O optics use an explicit surface type.**

The surface form is fixed as `AsymmetricGradedOptic<S, A, G_get, G_put>`, with ordinary `GradedOptic<S, A, G>` remaining as sugar for the symmetric case. The composition laws are fixed in Chapter 15 and no longer left for a later design note.

**Decision 3: the first session-type scope is intentionally narrow.**

The first session-type release is binary, linear, non-delegating, and checked at optic boundaries by structural duality. This is enough for the network and protocol use cases that motivate the feature, while keeping the checker and diagnostics tractable.

The broader lesson is that the language should front-load decisions whenever they affect long-lived carriers, summary formats, or artifact schemas. That is the point where design indecision becomes retrofit cost.

#### 27.19.3 Formal properties that remain unverified

The book makes claims about fusion soundness, grade algebra well-formedness, and alias safety completeness that are asserted but not machine-checked. This is not unusual for a first publishable edition of a language specification, but the claims should be explicitly tagged as conjectures with stated proof obligations rather than allowed to harden into assumed facts.

**Fusion soundness.** The three v0 fusion passes (map fusion, compose fusion, product flattening) are presented with correctness arguments but no formal proof. For the prelude, thorough golden-test coverage is sufficient to proceed. For the full language — where traversal fusion, prism fusion, and staged fusion add significantly more complex rewrite rules — an unsound rewrite is a serious risk. The target for formal verification is Lean 4 or Agda, operating over a small-step operational semantics for the grade calculus. This should be pursued as a background obligation alongside M7, not deferred to post-M10.

**Grade algebra well-formedness.** The product of eight semirings must satisfy the semiring axioms for each dimension and the distributive law for the product. The less obvious dimensions — session types under sequential composition, security lattice under parallel product — need case-by-case checks. An error in the distributive law for any dimension would make grade normal forms non-unique, which would invalidate the claim that fusion legality is proof-directed rather than heuristic.

**Alias safety completeness.** The conservative region language is intentionally an over-approximation. The open question is whether it produces too many false-positive alias conflicts in realistic programs. This requires empirical measurement against real systems code — game engine ECS loops, database query plans, kernel page table walkers — not just the nine acceptance test cases. A false positive rate above roughly 5% on realistic programs would suggest the region language needs refinement before the full language ships.

#### 27.19.4 Three tooling investments that compound early

These are not blocking for any milestone, but they pay dividends across every subsequent session of work. The cost of establishing them during M5–M6 is low; the cost of retrofitting them during M9 is high.

**A structured benchmark harness with committed baselines.** The benchmark format should record: the semantic `PerfKey` (optic name and composition shape, not a raw symbol name), the target profile (architecture, ISA extensions, cache configuration), the backend family (Rust transpiler vs LLVM), and a tolerance band for pass/fail. Without committed baselines, benchmark regressions are invisible until they accumulate into production failures. This harness should be part of M5 release gates, not an M6 add-on.

**Agent-facing diagnostic validation before M6.** The structured diagnostic schema — stable codes, evidence objects, ranked fixes, next-command suggestions — is already specified. Before M6 is declared, the schema should be validated against a real automated repair loop: an agent given a grade violation, an alias conflict, and a type mismatch, allowed to resolve each in one pass using only the structured JSON output with no additional context. If any of the three requires more than one pass, the evidence field for that diagnostic code is missing something specific. That specific thing should be added before M6, not filed as a future enhancement.

**Translation-validation harness between Rust and LLVM backends.** Before the LLVM backend is authoritative, the project needs a way to verify that LLVM-generated code is semantically equivalent to Rust-transpiled code for the same source program. The harness should: compare diagnostic JSON for the known-good acceptance suite between both backends; compare generated loop shapes for canonical examples; compare benchmark deltas against seed tolerances. Without this harness, LLVM codegen cannot be trusted to maintain the semantic invariants the book claims it preserves, and the claim that the Rust backend is a "semantic microscope" becomes unfalsifiable once the LLVM backend is the default.

### 27.20 Summary: the road to a working language

By the end of this chapter, the remaining work should feel less like an unbounded backlog and more like a sequence of obligations that follow directly from the architecture the book has already chosen.

The first obligation is still the narrow compiler: M0–M6 must turn the core thesis into running code, with the alias checker at M2 as the hardest immediate risk. The second is to implement against the now-front-loaded M7 decisions — fractional ownership as the underlying carrier, explicit asymmetric-grade syntax, and a narrow first session-type scope — so the full-language work does not begin from an unstable summary format. The third is to publish the ecosystem-facing contracts early: module interfaces, edition policy, and native package plus lock artifacts before the first library culture improvises its own rules.

After that, the priorities become the things that keep a promising language from becoming a durable one: benchmark and translation-validation discipline, a published soundness budget, an explicit memory model, runtime-family and plugin policy, and the debugging/profiling contracts that make optimized code explainable. Formal verification, richer session and security grades, and later research expansions still matter, but they no longer look like the main gate between the current project and a working language.

That is the final shape of the argument. The language does not need a new semantic center to mature. It needs the courage to keep extending the current one — explicit runtime roots, explicit optics, explicit summaries, explicit grades, explicit boundary contracts, explicit graph artifacts, and explicit policy — into the places where other languages most often become vague.

### 27.21 Transition to the standing checklist chapter

The maturity chapter ends by naming the pressures and the remaining obligations. The next chapter turns those pressures into a standing review discipline: a feature-admission checklist that future proposals must answer in one place, plus a summary of what current coding agents still struggle with in large codebases and why those struggles matter to Optic's architecture.

That ordering is deliberate. The language first identifies the late-stage risks, then it fixes the review questions that stop those risks from reappearing as ad hoc exceptions.

## 28. Feature Admission Checklist and Coding-Agent Failure Modes

This final chapter turns the maturity analysis into a standing review instrument. The earlier chapters identified the recurring late-stage pressures — compatibility, boundary discipline, runtime-family coherence, artifact identity, provenance, and toolchain policy. Those pressures become useful only if future feature proposals are forced to answer them in the same order every time.

The chapter therefore has two tasks. First, it defines a repeatable checklist for deciding whether a proposal belongs in the language core, in the boundary lane, in a quarantined experimental lane, or nowhere in the language at all. Second, it grounds that checklist in a practical observation from the current generation of coding agents: large codebases fail not only because models lack raw coding ability, but because repository knowledge is hard to retrieve, stale guidance crowds out relevant context, multi-repository or multi-artifact work exceeds the default tool envelope, and verification remains expensive and fragile. Those failures are not separate from language design. They are exactly the kind of operational pressure that either gets absorbed into the language's explicit model or later returns as feature creep.

### 28.1 Why a standing checklist belongs in the book

A strong language design can still decay if every later proposal is argued from scratch. The natural failure pattern is familiar: one proposal is allowed because it seems convenient, the next because it looks compatible with the first, and eventually a second semantic center appears by accumulation rather than by declaration.

The book's architecture already has the right counterweight. It insists on one semantic center, one authored language, one summary model, one graph-shaped compiler story, and one explicit boundary lane for realities that the core language should not pretend away. What it still needs is a permanent review surface that makes those commitments operational.

The checklist below should therefore be treated as part of the language's governance model. A proposal that cannot answer these questions has not matured enough to be accepted, no matter how attractive the feature looks in isolation.

### 28.2 The feature-admission checklist at a glance

| Review question | Why it matters | Typical outcomes |
|---|---|---|
| What exact use case is unlocked? | prevents surface imitation of another language without a new operational gain | keep, defer, or reject |
| Which existing lane should carry it: core, boundary, generated artifact, internal toolchain, or quarantined experimental? | prevents second-language creep while still giving research a disciplined home | core / boundary / generated / internal / experimental |
| What earlier artifact proves it is legal? | stops the compiler from rediscovering meaning heuristically | summary, grade, boundary contract, target profile, interface artifact |
| What is the smallest theory or sidecar that already answers the question? | prevents over-theorized core growth and keeps v1 implementation tractable | resource logic / weak memory / typestate / abstract interpretation / TLA+ / Alloy / none |
| What mixed-domain collision should this feature survive? | prevents isolated examples from disguising architectural fractures | render + network, storage + legacy FFI, experimental kernel + ordinary traversal, or an explicitly justified alternative |
| Does it lower through the ordinary summary and boundary path? | prevents experimental mathematics or domain frontends from creating a second rulebook | yes with `OpticSummary`/`BoundaryContract`/CGIR, or reject / quarantine |
| What stable identity does its output have? | makes incremental builds and reproducibility possible | node hash, artifact key, interface hash, revision id |
| Can the ergonomic goal be met by checked focusing/elision or grade inference instead of a new ambient mechanism? | prevents prop-drilling fixes or resource-default conveniences from turning into hidden semantics | focused path, inferred grade, or explicit rejection |
| What compatibility surface does it affect? | keeps source, interface, ABI, and runtime promises separate | edition, interface schema, runtime family, plugin ABI |
| What proof or guarantee does it bypass, if any? | turns unsoundness into a budget instead of folklore | soundness-ledger entry or rejection |
| What provenance survives after optimization and lowering? | keeps debugging, profiling, and agents grounded | source span, node id, fused provenance set, PerfKey |
| What is the smallest rollback path if the feature proves too costly? | prevents irreversible accretion | deprecate, gate, boundary-only, internal-only |

This table is intentionally compact. The longer sections that follow explain how each question should be applied.

Two of the checklist questions deserve special emphasis. First, a proposal should be able to name at least one **mixed-domain collision** it survives without inventing special escape rules, because a systems language proves its extensibility when unlike subsystems meet. Second, experimental mathematics and imported domain frontends are welcome only when they remain subordinate to the ordinary summary, grade, provenance, and boundary pipeline.

### 28.3 Semantic and compiler questions every proposal must answer

#### 28.3.1 What genuinely new use case is unlocked?

A proposal should begin with a use case, not with a borrowed surface feature. The right opening question is not “which mainstream language already has this syntax?” but “what important program shape or system boundary remains impossible or unreasonably awkward under the current model?”

This is the chapter's most important guardrail. If the proposal only makes an existing use case feel more familiar to users of another language, then the default answer should be “not in core.” The current architecture already covers a wide range of use cases through ordinary optics, staged build-time execution, boundary contracts, and generated artifacts. A proposal that does not unlock new operational territory must justify why a second mechanism is worth the long-term compatibility cost.

#### 28.3.2 Which lane carries it: core, boundary, generated artifact, internal toolchain, or quarantined experimental work?

Every proposal should be classified immediately into one of five lanes.

- **Core** means ordinary authored source, ordinary optic semantics, ordinary summaries, and ordinary compile-time and runtime behavior.
- **Boundary** means the use case is fully supported, but it crosses a hostile ABI, hardware, runtime-family, or capability edge and therefore rides through `BoundaryContract`, `unsafe optic`, target/runtime declarations, or generated bindings.
- **Generated artifact** means the feature should exist as compiler-emitted material — lock snapshots, interface files, generated bindings, crash capsules, replay traces, benchmark metadata — rather than as a second authored surface.
- **Internal toolchain** means the capability is real but should remain compiler- or protocol-facing rather than becoming source syntax.
- **Experimental** means the idea is important enough to implement and test, but not yet stable enough for the ordinary language contract. Experimental work enters only through the contextual keyword `experimental`, the `std.experimental` namespace root, and graph-native experimental arenas or artifacts.

The point of this early classification is not bureaucracy. It is to prevent every ecosystem pressure from being interpreted as evidence that the core surface language must grow.
A practical example is solver design. Equation-heavy notation, imported model formats, generated stencils, and symbolic preprocessing may all be valuable. The checklist should still ask whether those needs can be carried by optics-native solver libraries, staged artifacts, and imported frontends before admitting a new solver-specific core syntax.

#### 28.3.2.1 If the idea is still research, quarantine it early

A mathematically interesting idea does not have to jump directly into the core language in order to be taken seriously. The review question should therefore be explicit: can the proposal live under the experimental lane first? If the answer is yes, the default should be to keep it there until it has: (1) a stable summary form, (2) a clear legality rule, (3) a measurable backend or tooling consequence, and (4) a credible rollback path.

This is especially important for proof-oriented, geometric, ultrametric, sheaf-like, topos-like, or dynamical-system-inspired features. Those ideas may well matter to the language, but the burden is to show which layer they refine — proof artifacts, domain numerics, graph retrieval, coinductive stability, or distributed consistency — before they are allowed to reshape ordinary user code.

#### 28.3.2.3 Prefer the smallest theory that closes the operational gap

The review process should ask one question earlier and more bluntly than most language teams do: could a **simpler theory or an external sidecar** answer the same problem with less permanent surface-area cost?

For Optic this matters directly. The open memory-model questions are often better served first by resource/separation reasoning and weak-memory operational models than by richer proof or categorical machinery. Protocol and callback-state questions are often better served first by typestate or explicit protocol automata than by full session or sheaf machinery. Some scheduler, distributed-runtime, and plugin questions are often better explored first in TLA+ or Alloy than as new language features. Conservative optimizer questions are often better answered first through abstract interpretation than through proof-heavy machinery.

The practical rule is simple: if a narrower theory or sidecar already answers the operational question, the burden shifts to the richer proposal to show what new capability it adds beyond elegance.

#### 28.3.2.4 Prefer a decision ladder over a feature pile

For proposals touching memory, boundaries, protocols, or optimizer legality, the review should ask for an explicit ladder:

1. what is the **direct internal lane** that fits the existing model best;
2. what is the **simpler sidecar** that could answer the question without enlarging the language;
3. what is the **richer second-wave theory** that should only be tried if the first two are insufficient.

In current Optic terms, that usually means:
- `std.experimental.sep` before richer provenance or ownership theories,
- `std.experimental.memory` before broader categorical or proof-heavy concurrency foundations,
- typestate, TLA+, Alloy, or abstract interpretation before new protocol or optimizer features,
- and only then `proof`, `sheaf`, `topos`, or `dynamics` as possible deeper refinements.

A proposal that skips this ladder should be presumed too expensive for v1 unless it can show that the smaller answers are already inadequate.

In practical review terms, the committee or maintainer should be able to answer three yes/no questions before approving the work:

- have the direct internal lanes already been tried or clearly ruled out,
- have the smaller sidecars already been tried or clearly ruled out,
- and does the richer proposal preserve the one-summary, one-graph architecture instead of introducing a shadow semantics?

If any of those answers is still "no", the proposal belongs in research notes rather than in the design freeze.

#### 28.3.3 What earlier artifact proves it is legal?

A feature belongs in the language only if the compiler can point to a specific earlier artifact that justifies using it later. In Optic, those artifacts are already known: `OpticSummary`, `RegionSet`, grade expressions, boundary contracts, interface artifacts, target profiles, runtime families, and graph revisions.

If a proposal cannot say which artifact carries its legality proof, then the backend or tooling will eventually have to recover the feature heuristically from lowered code or opaque metadata. That is exactly the failure mode the book has been trying to avoid from the beginning.

#### 28.3.4 What stable identity does the feature's output have?

A large language ecosystem lives or dies by stable identities. A feature that produces code, metadata, interfaces, staged artifacts, replay traces, bindings, or plans must say what identifies those things across revisions.

That identity might be a node hash, a revision id, an interface checksum, a staged-artifact key, or a `PerfKey`. The important point is that the answer cannot be “whatever file happened to be written by the current compiler build.” If it is, incremental compilation, reproducibility, and multi-tool reasoning will all become fragile later.

### 28.4 Compatibility, boundary, and rollback questions

#### 28.4.1 Which compatibility layer does it affect?

Every feature proposal must state explicitly whether it affects:

- source/edition compatibility,
- module-interface compatibility,
- package/build reproducibility,
- binary or plugin ABI,
- runtime-family compatibility,
- or only internal toolchain state.

This is how the language avoids the most common maturity mistake: saying “stable” without naming what is stabilized.

#### 28.4.2 What proof or guarantee does it bypass?

Some features really are escape hatches. That does not disqualify them, but it does change the review rule. If a proposal weakens a proof, it must enter the soundness-budget ledger with:

- the proof it bypasses,
- the local contract that replaces it,
- the diagnostic family attached to it,
- and the audit owner responsible for it.

If the proposal cannot be localized this way, it should not be admitted.

#### 28.4.3 What provenance survives after optimization?

A feature that is semantically acceptable but destroys provenance is still too expensive for a language that wants graph-native debugging, profiling, and coding-agent workflows. Every proposal should therefore answer:

- what source identities survive into CGIR,
- what fused or staged ancestry survives lowering,
- what debug/perf identity survives backend emission,
- and how a crash or benchmark result would still be attributed to the feature.

#### 28.4.4 What is the rollback path?

Every feature should have a stated retreat path. If it turns out to be too costly, too unstable, or too socially fragmenting, can it be:

- demoted from core to boundary,
- demoted from authored source to generated artifact,
- kept as internal tooling only,
- or edition-gated and eventually deprecated?

If the answer is no, the admission bar should be correspondingly higher.

### 28.5 Permanent “never in core” checklist

The closure rule from Chapter 27 becomes more useful when phrased as a standing negative table rather than a one-time design note.

| Candidate | Why it keeps reappearing | Why it still stays out of core | Supported instead via |
|---|---|---|---|
| a second authored build/config language | familiarity from TOML/YAML/CMake ecosystems | duplicates facts already present in `Project` and creates a second semantic center | native `package` / `workspace` declarations plus generated lock, interface, and build artifacts |
| a primary syntax-rewriting macro system | expressive short-term escape hatch | grows into a shadow language and weakens graph-level staging and provenance | staged optic graphs, generated artifacts, and schema-driven plugin points |
| ambient exceptions as the ordinary control model | terse error propagation and legacy familiarity | hides control edges from CGIR, replay, and provenance | prisms, `Result`, `Option`, and any later control layer kept explicit |
| a BetterOptic hosted/freestanding/kernel dialect split | makes hard domains feel easier by narrowing semantics | fractures the language into personalities and duplicates policy | `RuntimeFamily`, `TargetProfile`, capability-gated host regions, and `BoundaryContract` |
| a blanket “stable ABI” slogan | sounds ecosystem-friendly | source, interface, runtime, and plugin compatibility are distinct contracts | explicit C-facing ABI first, opt-in plugin/runtime contracts later |
| multiple blessed package workflows | community pressure from different domains | fragments caching, artifact schemas, and social defaults | one first-party native package/workspace/build path |
| unstructured `unsafe` | convenience at hard boundaries | destroys summary completeness and auditability | `unsafe optic`, `BoundaryContract`, and the published soundness-budget ledger |
| a separate user-facing query language for `Project` | query ergonomics and tooling power | recreates a second meta-language beside ordinary code | ordinary optic-based project queries with internal QIR lowering |
| plugins over unstable internal compiler IRs | rapid experimentation | creates a shadow compiler API that is harder to migrate than the language core | versioned graph protocol, schema-driven tool/plugin surfaces, generated interface artifacts |

The table should be treated as a standing rebuttal to feature creep. A proposal that matches one of these rows starts from “no” and must prove that it does not in fact recreate the excluded pattern under a new name.
The same test should be applied to mathematically rich proposals. Hyperreal or nonstandard-analysis ideas, for example, may be extremely valuable as analysis artifacts, solver-generation helpers, or approximation witnesses. They should still enter through the quarantined experimental lane first rather than by immediately changing the core numeric tower.

### 28.6 What coding agents still struggle with in large codebases

The current generation of coding agents is already useful, but the public evidence points to a set of recurring weak points that matter directly to language and compiler architecture.

#### 28.6.1 Context is scarce, and monolithic guidance rots

Anthropic's engineering guidance on context engineering is explicit that context is finite, that models show “context rot” as token counts rise, and that long-horizon tasks need compaction, structured notes, and often multi-agent decomposition rather than one ever-growing conversation. OpenAI's own engineering notes on Codex report a parallel lesson: a single giant `AGENTS.md` failed because it crowded out task-relevant information, decayed quickly, and was hard to verify mechanically. Those two sources point to the same operational conclusion: repository knowledge must be structured, queryable, and freshness-aware instead of poured into one static manual.

#### 28.6.2 Repository boundaries and tool envelopes are still narrow

GitHub's own documentation for Copilot coding agent states that, by default, the agent only accesses context in the current repository, cannot modify multiple repositories in one run, and opens one pull request per assigned task. It also documents compatibility limits around rulesets, hosted-runner assumptions, and content exclusions. These are sensible safety constraints, but they expose a real implementation problem for large systems: many important changes cross repository, artifact, or policy boundaries that the default agent envelope does not naturally see.

#### 28.6.3 Experience reuse is fragile and poor retrieval can make agents worse

SWE-ContextBench was proposed precisely because repository-level software engineering is not a sequence of independent tasks. Its abstract reports that the ability to accumulate, retrieve, and apply prior experience across related tasks had been under-measured, and that correctly selected summarized experience improves accuracy while reducing runtime and token cost. Just as important, incorrectly selected or unfiltered experience gives limited or negative benefit. In other words, memory helps only when its retrieval and summarization are right.

#### 28.6.4 Real software-engineering tasks remain difficult, and environment variability matters

OpenAI's SWE-Lancer benchmark reports that frontier models still fail on the majority of real-world freelance software engineering tasks. The same benchmark was later updated to remove the requirement for internet connectivity during execution specifically because it was a major source of evaluation variability. This is a useful caution: repository reasoning is hard enough even before hidden environment dependence is added on top.

### 28.7 What the architecture should do in response

The point of the previous section is not to chase the weaknesses of today's agents reactively. It is to read those weaknesses as design pressure on the language itself.

A language that wants to cooperate well with coding agents at large-codebase scale should therefore front-load the following architectural responses.

1. **Repository knowledge must be the system of record.** The earlier `ProjectGraph` decision is not just elegant; it is the right answer to stale manuals and context crowding. Agents should query graph-native summaries, interfaces, diagnostics, and provenance rather than consume one monolithic instruction file.
2. **The build graph and artifact graph must be semantically visible.** Multi-repository or multi-artifact work should be represented in the graph rather than disappearing into external scripts. This is what lets the toolchain grow beyond single-repository task envelopes without inventing a second language.
3. **Summaries and provenance must be stable enough to serve as retrieval anchors.** A coding agent needs something better than fuzzy search. `OpticSummary`, `BoundaryContract`, `RegionSet`, stable node ids, and `PerfKey`-style semantic identities are the natural retrieval units.
4. **Diagnostics must remain structured and replayable.** Large-codebase work fails expensively when the agent cannot tell whether it broke source compatibility, interface compatibility, runtime-family assumptions, or artifact identity. The diagnostic schema already points in the right direction; it should remain a first-class protocol.
5. **Experience reuse must be selective, not ambient.** The lesson from SWE-ContextBench is that summaries and prior traces help only when they are retrieved and compressed well. That is an argument for queryable project graphs and explicit artifact classes, not for giant persistent transcripts.
6. **Environment dependence must be explicit.** The SWE-Lancer update is a reminder that hidden external dependencies make evaluation and repair brittle. Build-time capabilities, runtime-family declarations, and target profiles should therefore stay explicit and hashable.

These responses are not “AI features” layered onto the language. They are reasons to keep `Project`, native package declarations, graph-native tooling, and optic-based project queries central.

### 28.7.1 From agent failure modes to a repository agent operating system

The architectural response should therefore not stop at better prompts or larger context windows. A language project that expects serious agent participation should ship a **repository agent operating system**: one coordinating role, a small number of sharply bounded specialists, explicit work packets, explicit shared memory, explicit per-agent memory, and a generated context index derived from checked-in state rather than from one long transcript.

This is the smallest structure that directly answers the documented weak points. Context crowding is handled by specialist isolation and compact memory. Cross-file drift is handled by an orchestrator and coherence review. Verification remains explicit because the task packet names the expected validation steps. Experience reuse becomes selective because stable lessons are promoted into shared memory while temporary findings remain attached to the task that discovered them.

### 28.7.2 Canonical hub, thin wrappers, explicit memory

The repository agent system should also follow the same closure rule the language applies elsewhere: one semantic center, many adapters. For agent tooling that means one canonical instruction hub (`AGENTS.md`) plus thin wrappers for tool-specific entry points such as `CLAUDE.md`, `GEMINI.md`, GitHub Copilot instruction files, and Kilo agent files. The wrappers should narrow or point back to the canonical contract, not fork it.

The same rule applies to memory. Auto-memory features in individual tools can help, but the canonical repository memory should remain checked in, reviewable, and tool-agnostic: one shared memory file for stable repo-wide truths and one bounded memory file per agent role. That keeps the system portable across tools and legible to human maintainers.

### 28.7.3 The extrapolation to human-driven software development is direct

This structure is useful not because software development is becoming less human, but because large development efforts already have the same shape when they work well: one coordinator, several specialists, explicit work packets, explicit review, and short written memory that survives personnel changes. The repository agent operating system is therefore best understood as a codification of good software-development practice that happens to be executable by coding agents as well as humans.

Appendix K records one concrete file-level realization of this idea for the split book package: role files, memory ledgers, task packet templates, compatibility wrappers, and a small maintenance script that keeps the context index in sync with the checked-in system definition.


### 28.7.4 Repository memory should be typed, selective, and queryable

The research on coding-agent failure modes points toward a precise architectural response: the repository should preserve the small subset of maintenance knowledge that is durable enough to matter again, and it should preserve it as structured data rather than as one long transcript.

For Optic, the right default is to keep that durable subset inside the same semantic world as the compiler graph whenever possible. That means accepted decisions, validated repair records, benchmark explanations, runtime-family and boundary notes, and long-lived task state should all be eligible for graph-native representation. They become another query surface over `Project`, not a second, informal knowledge base.

The corresponding negative rule is just as important. Scratch notes, speculative search trails, giant chat logs, and opaque embedding caches should not become authoritative graph truth. They may help a local tool, but they do not deserve the same trust class as source, summaries, diagnostics, or interfaces.

### 28.7.5 Failed patches are advisory negative knowledge, not bans

Large codebases accumulate negative knowledge whether they admit it or not. A patch was tried. It broke alias safety, regressed a benchmark, failed translation validation, or was rejected because it weakened a boundary contract. If that information is lost, future agents and humans will eventually rediscover the same failure the expensive way.

The language and toolchain should therefore support a typed failed-patch record with at least: goal, target nodes or regions, patch fingerprint, attempted revision, target profile, outcome class, reason class, evidence references, supersession link, and expiry or revalidation policy.

The design principle is caution without taboo. A failed-patch record is a warning backed by provenance and evidence. It is not a permanent prohibition. Future agents should use it to de-rank similar proposals and to inspect prior evidence before retrying them, not to freeze the design around stale mistakes.

### 28.8 A concrete review procedure for future proposals

A proposal review should be short, repeatable, and ruthless enough that the language does not drift.

#### Step 1 — classify the proposal

State in one sentence:
- the use case,
- the lane (`core`, `boundary`, `generated artifact`, `internal toolchain`),
- and the compatibility layers affected.

If that sentence cannot be written clearly, the proposal is not ready.

#### Step 2 — answer the checklist

Every proposal document should answer at least these questions directly:

1. What exact use case is unlocked?
2. Which lane carries it?
3. What earlier artifact proves it is legal?
4. What stable identity does its output have?
5. Which compatibility surface does it affect?
6. What proof, if any, does it bypass?
7. What provenance survives optimization and lowering?
8. What is the rollback path if the proposal proves too costly?

#### Step 3 — classify the outcome

A proposal should end in exactly one of these outcomes.

- **Accept into core** — if the use case is new, the summary/lowering story is explicit, and no second semantic center is introduced.
- **Support at the boundary** — if the use case is real but belongs behind `BoundaryContract`, `unsafe optic`, generated bindings, runtime-family declarations, or explicit capabilities.
- **Emit as generated artifact** — if the need is real but should live as compiler-emitted data rather than authored syntax.
- **Keep internal** — if the mechanism is useful for the compiler or tooling but should not become part of the language surface.
- **Accept into the experimental lane** — if the idea is promising, measurable, and worth implementing, but still too unstable for the ordinary language contract.
- **Reject / never in core** — if the proposal recreates one of the permanently excluded patterns.

#### Step 4 — add a regression anchor

Every accepted proposal should add one new lasting anchor to the repository:
- a fixture,
- a benchmark,
- an artifact-schema example,
- a diagnostic case,
- or a translation-validation check.

That is how the checklist remains part of the implementation rather than a policy memo.

### 28.9 Final closing argument

The purpose of this chapter is not to freeze the language out of fear. It is to freeze the *review discipline* so the language can continue to grow without becoming structurally incoherent.

The recurring lesson from both mature ecosystems and current coding-agent behavior is the same: large systems fail when they rely on hidden context, vague compatibility promises, unversioned boundaries, and too many unofficial ways to say the same thing. Optic already has a stronger starting point than most languages because it treats runtime roots, summaries, grades, boundary contracts, graph revisions, and generated artifacts as explicit semantic objects. The checklist simply makes that advantage harder to lose.

### 28.10 Transition to the appendices

The appendices that follow are now easier to read in operational terms. They are not just loose reference material. They are the concrete schemas, ladders, contracts, and query surfaces that support the review discipline established in this chapter.
# Appendices

The appendices are working reference rather than leftovers. They capture the normative and operational material the main text relies on: diagnostics, command surfaces, milestone gates, executable grammar, arithmetic defaults, boundary contracts, ecosystem references, graph-store rules, soundness budgets, semantic-query support, and the repository agent operating system that maintains this split package coherently across tools.

## Appendix A — Diagnostic Catalog

This appendix keeps the diagnostic catalog explicit and stable. The catalog is intentionally load-bearing: both humans and coding agents need a durable reference for the compiler's failure modes, and later migration or automation work depends on those codes remaining a reliable protocol rather than a prose convenience.

The codes are theorem-stable, but the preferred rendering should default to machine-facing language. When the internal rule is about semiring composition, coalgebraic structure, or phase legality, the default message should still tell the reader what collided, overflowed, widened, or crossed a forbidden boundary in operational terms.

| Code | Phase | Meaning | Preferred first action |
|---|---|---|---|
| `PAR-010` | parse | Mixed `>>>` and `***` without parentheses | Add parentheses according to the fixed precedence rule |
| `PAR-011` | parse | `>>>` used where `***` was likely intended, or vice versa | Re-check pipeline grouping intent |
| `PAR-020` | parse | Unexpected token in optic declaration | Compare against the nearest valid optic form |
| `PAR-021` | parse | Missing `=>` in a `get` or `put` clause | Insert `=>` after the parameter list |
| `PAR-030` | parse | Unterminated block expression or block comment | Add the missing terminator |
| `RES-101` | resolve | Unknown symbol | Check declarations, shadowing, and spelling |
| `RES-111` | resolve | Unknown field on costate | Inspect the `data` declaration and field spelling |
| `RES-121` | resolve | Optic used before declaration in an order-sensitive context | Reorder or introduce a declaration boundary |
| `TYP-201` | type | Focus type mismatch in composition | Dump summaries for both optics |
| `TYP-211` | type | Costate type mismatch in product | Ensure both sides of `***` share the same costate |
| `TYP-221` | type | Unsupported optic kind in the prelude | Rewrite to lens form or gate for a later milestone |
| `TYP-231` | type | Map closure parameter type mismatch | Check the focus type of the optic |
| `GRA-101` | grade | Grade dimension missing and inference unavailable | Add an explicit grade annotation |
| `GRA-104` | grade | Sequential composition exceeds declared cache bound | Split the pipeline or relax the bound |
| `GRA-110` | grade | Declared grade is tighter than inferred grade | Tighten the body or relax the declaration |
| `GRA-121` | grade | Ownership grade incompatible with composition or context | Re-check exclusivity assumptions |
| `GRA-131` | grade | Unbounded grade used in a bounded context | Bound the body or remove the bound |
| `ALI-201` | alias | Write/write conflict in product | Separate the product or make one side read-only |
| `ALI-211` | alias | Write/read conflict in product | Make the writer exclusive or the reader strictly read-only |
| `ALI-221` | alias | Linear optic reused after consumption | Ensure the linear resource is used exactly once |
| `OPT-301` | feature | Prism syntax used in v0 | Rewrite to a supported lens-like form |
| `OPT-311` | feature | Traversal syntax used in v0 | Rewrite to the supported prelude subset |
| `OPT-321` | feature | `.coinductive()` or `.drive()` used in v0 | Remove or gate for the full language |
| `OPT-331` | feature | `.replay()` used in v0 (deferred beyond narrow M8) | Use `.record("event")` or defer replay CLI; see **OBS-701** |
| `OPT-341` | feature | `stage` keyword used in v0 | Remove or gate for the staging milestone |
| `CGI-410` | CGIR | CGIR invariant failure during construction | Dump HIR and the failing node; likely compiler bug |
| `CGI-420` | CGIR | Cycle detected in the CGIR graph | Treat as compiler bug |
| `FUS-501` | fusion | Fusion blocked because an intermediate escapes | Introduce a stage boundary or keep the unfused form |
| `FUS-511` | fusion | Compose fusion blocked by nondeterministic or unsupported shape | Separate the stage or mark as approximate later |
| `FUS-521` | fusion | Fused form regressed benchmark performance | File a performance bug and disable the rewrite for that case |
| `COD-601` | codegen | Rust generation failed for an unsupported shape | Dump fused CGIR; likely compiler bug |
| `COD-611` | codegen | Generated Rust failed `cargo check` | Dump fused CGIR; compiler bug until disproven |
| `STG-101` | staging | Candidate static graph reads a runtime-only region | Move the read behind a residual boundary or route it through `BuildRuntime` |
| `STG-111` | staging | Compile-time host access is not reproducible or not whitelisted | Declare the input explicitly in `BuildHostContext` |
| `STG-121` | staging | Compile-time recursion or traversal lacks a proven bound | Add a bound or residualize the computation |
| `STG-131` | staging | Staged result exceeds code-size or artifact-size budget | Keep structure staged but residualize bulky data |
| `STG-141` | staging | Compile-time work exceeds declared budget | Split specialization, cache earlier, or relax the budget |
| `CTX-101` | context | Focused path is ambiguous in the current root | Disambiguate the focused path or make the root path explicit |
| `CTX-111` | context | Focused form crosses a forbidden build/runtime boundary | Re-scope the focus or split the computation across the correct root |
| `CTX-121` | context | No zero-cost focus path exists for the requested view | Fall back to the explicit root path or refactor the surrounding optic |
| `CTX-131` | context | Focused form would widen alias, determinism, or boundary assumptions illegally | Keep the explicit path or tighten the local region/boundary declaration |
| `GRD-151` | grade | Elided or partially specified grade cannot be reconstructed exactly | Make the missing dimensions explicit or switch to gradual `?` |
| `GRD-161` | grade | Observed development-time grade diverges from the declared or inferred contract | Accept the suggested grade, tighten the implementation, or declare a wider bound |
| `GRD-171` | grade | Elided grade would widen an exported or persisted contract illegally | Spell out the hidden dimensions explicitly at the boundary |
| `OBS-701` | observability | Unsupported observability query method (`.profile`/`.replay` deferred in narrow v0) | Use `.tap("label")` or `.record("event")`; defer profile/replay CLI until M8+ |
| `OBS-702` | observability | Observability hook appears after `.get`/`.set`/`.map` (prefix-only in v0) | Move `.tap`/`.record` before query methods |
| `BTS-801` | bootstrap | Self-hosted output diverged from the seed compiler | Run differential harness and compare diagnostics |
| `KRN-901` | kernel | Kernel target used an unsupported host dependency | Replace with a target-approved runtime optic |
| `INI-101` | init | Project intent is incomplete or contradictory | Supply or revise runtime family, target, or domain answers in the wizard or intent file |
| `INI-111` | init | Selected template conflicts with declared target/runtime/capability set | Choose a compatible blueprint or adjust the declared constraints |
| `INI-121` | init | Procedural scaffold generation could not synthesize required build roots or boundary stubs | Accept the suggested missing fields or switch to manual graph-root authoring |

The diagnostic record should be emitted in both human-readable and JSON forms. A good machine-facing record includes:

- stable code,
- phase,
- severity,
- primary and related spans,
- violated rule,
- evidence object,
- ranked local fix options,
- preferred fix,
- next commands.

That structure is what lets coding agents repair locally instead of resorting to whole-file rewrites.

---

## Appendix B — Command Surface and Repository Layout

This appendix gathers the operational surfaces an implementer, tool author, or coding agent will touch most often: the command set, the repository shape, and the minimum bootstrap layout for the first compiler milestones. The aim is practical clarity rather than narrative development.

### B.1 Command surface

| Command | Purpose |
|---|---|
| `optic init --wizard` | Guided project creation: collect project intent, generate native package/workspace/build roots, runtime blueprints, AppWorld scaffolds, and initial benchmark/diagnostic artifacts |
| `optic init --template NAME` | Fast-path project creation from a parameterized domain blueprint |
| `optic init --intent FILE` | Headless project creation from structured intent data |
| `optic init --preview` | Show the graph and file projections that would be created without committing them |
| `optic lsp serve` | Serve the first-party LSP adapter over the graph protocol |
| `optic materialize [PATH]` | Materialize graph-backed source or generated projections into an ordinary file tree |
| `optic sync-projections` | Force projection refresh between graph and writable text surfaces |
| `optic check file.opt` | Parse, resolve, type-check, and emit diagnostics |
| `optic check file.opt --json` | Machine-readable diagnostics |
| `optic explain CODE` | Show the rule, examples, and likely repairs for a diagnostic |
| `optic explain-focus file.opt --node NAME` | Show the explicit root-path form of a focused/elided expression |
| `optic explain-grade file.opt --node NAME` | Show the normalized grade after inference or partial elision for one optic |
| `optic explain-grade file.opt --node NAME` | Show the normalized grade after inference or partial elision |
| `optic dump-ast file.opt` | Print AST |
| `optic dump-hir file.opt` | Print resolved HIR and summaries |
| `optic dump-summary file.opt --node NAME` | Show one optic summary |
| `optic dump-cgir file.opt` | Print post-fusion CGIR |
| `optic dump-cgir file.opt --before-fusion` | Print pre-fusion CGIR |
| `optic dump-cgir file.opt --node N` | Print one node and its provenance |
| `optic dump-cgir file.opt --check` | Run invariant checks on CGIR |
| `optic transpile file.opt` | Emit Rust |
| `optic doctor file.opt` | Run consistency checks and suggest next actions |
| `optic bench file.opt` | Run benchmark harness and compare to baselines |
| `optic snapshot-update --confirm` | Update golden fixtures after review |
| `optic bootstrap` | Later: compare seed and self-hosted output |
| `optic experiment list` | Show available experimental lanes and package-level experiment declarations |
| `optic experiment doctor` | Check experimental-artifact schemas, witness freshness, and promotion blockers |

### B.2 Repository layout

```text
optic/
  Cargo.toml
  crates/
    optic-cli/
    optic-diagnostics/
    optic-syntax/
    optic-ast/
    optic-hir/
    optic-typeck/
    optic-cgir/
    optic-opt/
    optic-codegen-rust/
    optic-runtime/
    optic-tests/
  docs/
    implementation-book.md
    v0-executable-spec.md
    observability-v0.md
    effect-coeffect-v0.md
  examples/
    health_get.opt
    health_set.opt
    health_decay.opt
    health_position.opt
    nested_position.opt
    invalid_grade.opt
    invalid_alias.opt
    unsupported_prism.opt
    grade_mismatch.opt
    host_boundary.opt
  fixtures/
    tokens/
    ast/
    hir/
    cgir/
      pre/
      post/
    rust/
    diagnostics/
    bench/
      baselines/
```

### B.3 Minimal file-level bootstrap plan for M0–M2

The narrow compiler should treat M0–M2 as a concrete repository bring-up, not just as milestone labels. This subsection is intentionally practical: it names the first files that should exist and the first responsibilities they should carry.

#### B.3.1 M0 — parser and lexer

```text
crates/optic-syntax/
  tokens.rs
  lexer.rs
  parser.rs
  span.rs
```

Primary tasks:
- longest-match tokenization for `>>>` / `***`,
- nested block comments,
- deterministic recovery,
- token and AST fixtures.

#### B.3.2 M1 — HIR and cursor normalization

```text
crates/optic-hir/
  hir.rs
  resolver.rs
  lower.rs
  cursor.rs
```

Primary tasks:
- name resolution,
- query-chain lowering,
- cursor insertion,
- `PathLift` construction.

#### B.3.3 M2 — summaries, grades, and alias checking

```text
crates/optic-summary/
  infer.rs
  regions.rs
  grade.rs
  determinism.rs

crates/optic-typeck/
  checker.rs
  alias.rs
```

Primary tasks:
- sound `OpticSummary` inference,
- concrete cache grade inference,
- fractional ownership carrier plus named aliases,
- alias conflict diagnostics,
- `invalid_alias.opt` and grade-bound tests.

The important point is that M2 is the semantic hinge. If summary inference or alias checking is wrong, the rest of the compiler only becomes a faster way to be wrong.

### B.4 Agent workflow discipline

A coding agent working in this repository should normally proceed in this order.

1. parse errors,
2. resolve errors,
3. type errors,
4. grade and alias errors,
5. HIR and CGIR inspection,
6. codegen inspection,
7. benchmark review.

This order matters because later phases often depend on earlier-phase structure being clean.

---

## Appendix C — Milestone Ladder

This appendix turns the milestone story into a compact control surface. The ladder is capability-based, not calendar-based, and each gate is written as a claim that must be made true in repository evidence rather than by schedule pressure alone.

| Milestone | Gate | Primary risk |
|---|---|---|
| M0 — lexer and parser | deterministic parsing, all syntax errors collected in one pass, token and AST snapshots committed | operator tokenization (`>>>`, `***` as indivisible tokens); recovery producing spurious later errors |
| M1 — HIR | names resolve, query chains lower to explicit cursors, summaries exist for all named optics | `PathLift` producing incomplete region sets for nested optics; `put_reads` field populated incorrectly |
| M2 — typed HIR | types, concrete grades, and alias rules enforced with stable diagnostics | alias checker false negatives on `put_reads` conflicts; grade composition arithmetic saturating incorrectly at composition boundaries |
| M3 — CGIR | graph construction deterministic, invariant checker passes, dump output stable | provenance fields (`original_ids`) not threaded through node construction from day one |
| M4 — minimal fusion | map fusion, compose fusion, and product flattening sound and provenance-preserving | rewrite introducing a fusion that is semantically wrong but fast; soundness argument must be written before merge |
| M5 — Rust backend | generated Rust compiles, acceptance examples pass, runtime crate complete, **benchmark baselines committed** | benchmark baselines not committed at M5 makes "benchmarks green" at M6 meaningless |
| M6 — prelude release | benchmarks green within tolerance, diagnostics stable, fixtures frozen, **agent-repair loop validated** | agent requires more than one pass to resolve a grade or alias diagnostic — missing evidence field |
| M7 — full language begins | prisms, traversals, richer grades, coinduction on top of stable M6; **ownership/session/asymmetry decisions already fixed** | implementation must use the fractional carrier, explicit asymmetric syntax, and the narrowed session scope from day one of M7 |
| M8 — observability and replay | tap, record, profile, time and RNG injection, replay hooks | time injection not threaded from `HostContextLite` from M0 — requires retroactive refactor |
| M9 — self-hosting ladder | mixed seed/self-hosted compiler with differential validation; **translation-validation harness operational** | self-hosted output diverges in grade or alias diagnostics from seed without a harness to detect it |
| M10 — kernel-class ladder | `no_std` runtime and staged kernel-domain progression; **memory model chapter written** | kernel targets claim correctness without a stated memory model for atomics, volatility, and callbacks |

### C.1 Critical path notes

The notes below are the places where sequencing matters most. They are included here so that the ladder remains a management tool rather than a decorative roadmap.

**The alias checker is the hardest part of M2.** The acceptance test `invalid_alias.opt` must be committed and passing before M2 is declared. The checker must be exercised against: deeply nested products, self-referential costates, compositions where the conflict is in `put_reads` rather than `put_writes`, and `Linear` optic double-use. These are the cases most likely to produce false negatives.

**Provenance must be designed into M3, not retrofitted at M8.** Once the CGIR graph is constructed without `original_ids` on every node, adding them requires touching every pass that creates or rewrites nodes. That is an M4–M5 disruption if left to M8.

**Benchmark baselines must be committed at M5, not M6.** The M6 gate "benchmarks green within tolerance" is only meaningful if the baselines existed before M6 work began. A baseline committed on the same day as the gate check is a benchmark regression waiting to happen at M7.

**The front-loaded ownership/asymmetry/session decisions (§27.19) must already be reflected in the implementation by M7.** Any M7 code written against a different carrier or syntax regime will require avoidable rework. They should therefore be treated as M6 exit criteria in practice.

**Before M9:** Edition policy, module interface format, and conformance suite must exist. Self-hosting is only meaningful if there is a canonical artifact to validate against.

**Before M10:** Formal memory model, debug/profiling provenance strategy, and runtime-family policy. A kernel-class target without a stated memory model is an unsafe library, not a language.

The milestone ladder is one of the book's governance tools. It prevents the project from confusing architectural ambition with implementation readiness.

---

## Appendix D — Normative v0 EBNF

This appendix is normative rather than illustrative. The parser for the prelude must implement the following grammar exactly, and any divergence between implementation and grammar should be treated as a compiler defect rather than a user-level ambiguity.

```ebnf
program         ::= item* EOF
item            ::= data_decl
                  | optic_decl
                  | let_binding
                  | fn_decl

data_decl       ::= 'data' IDENT '{' field_list? '}'
field_list      ::= field_decl (',' field_decl)* ','?
field_decl      ::= IDENT ':' type_expr

optic_decl      ::= 'optic' IDENT ':' optic_type_ann '{' optic_body '}'
optic_type_ann  ::= 'GradedOptic' '<' type_expr ',' type_expr ',' grade_ann '>'
grade_ann       ::= grade_dim ('+' grade_dim)*
                  | '_'
grade_dim       ::= 'CacheGrade' '<' INT_LIT '>'
                  | 'OwnershipGrade' '<' rational_lit '>'
                  | 'LinearGrade'
                  | 'AffineGrade'
                  | 'SharedGrade'
                  | 'CacheGrade' '<' '_' '>'
optic_body      ::= get_clause put_clause?
get_clause      ::= 'get' IDENT '=>' expr
put_clause      ::= 'put' '(' IDENT ',' IDENT ')' '=>' (expr | block_expr)
block_expr      ::= '{' stmt* expr? '}'
stmt            ::= (IDENT '=')? expr ';'

optic_expr      ::= optic_par
optic_par       ::= optic_seq ('***' optic_seq)*
optic_seq       ::= optic_atom ('>>>' optic_atom)*
optic_atom      ::= IDENT
                  | '(' optic_expr ')'

let_binding     ::= 'let' IDENT ('=' optic_expr
                  | ':' optic_type_ann '=' optic_expr) ';'

fn_decl         ::= 'fn' IDENT '(' param_list? ')' ('->' type_expr)? '{'
                    stmt* expr? '}'
param_list      ::= param (',' param)*
param           ::= IDENT ':' type_expr

query_chain     ::= expr '.query(' optic_expr ')' query_method+
query_method    ::= '.get()'
                  | '.set(' expr ')'
                  | '.map(' closure ')'
closure         ::= '|' IDENT '|' expr
                  | '|' '(' IDENT (',' IDENT)* ')' '|' expr

expr            ::= query_chain
                  | assign_expr
assign_expr     ::= field_expr (('=' assign_expr) | )
field_expr      ::= atom_expr ('.' IDENT | '[' expr ']')*
atom_expr       ::= IDENT
                  | INT_LIT
                  | FLOAT_LIT
                  | '(' expr (',' expr)* ')'
                  | '(' expr ')'
                  | block_expr
                  | binary_expr

binary_expr     ::= atom_expr bin_op atom_expr
bin_op          ::= '+' | '-' | '*' | '/' | '<' | '>' | '<=' | '>='

type_expr       ::= 'SoA' '<' type_expr '>'
                  | 'BitSet'
                  | '(' type_expr (',' type_expr)+ ')'
                  | IDENT ('<' type_args '>')?
type_args       ::= type_expr (',' type_expr)*
rational_lit    ::= INT_LIT '/' INT_LIT | INT_LIT

IDENT           ::= [a-zA-Z][a-zA-Z0-9_]*
INT_LIT         ::= [0-9]+
FLOAT_LIT       ::= [0-9]+ '.' [0-9]+
RATIONAL_LIT    ::= [0-9]+ '/' [0-9]+
COMMENT         ::= '--' [^\n]* '\n'
BLOCK_COMMENT   ::= '{-' (BLOCK_COMMENT | [^-] | '-' [^}])* '-}'
```

### D.1 Disambiguation notes

- `>>>` is a single token, not three `>` tokens.
- `***` is a single token, not three `*` tokens.
- a lone `*` is invalid in the surface language.
- `{-` starts a nestable block comment even inside expressions.
- whitespace is ignored except inside future literal forms.

---


### D.2 Reserved experimental keyword and namespace roots

The contextual keyword `experimental` is reserved for post-v0 package/workspace/build declarations even though the narrow compiler does not implement those declarations yet. Feature names such as `sep`, `memory`, `proof`, `geo`, `ultra`, `sheaf`, `topos`, `dynamics`, and `nonstandard` are **not** reserved as global hard keywords; they are reserved as namespace segments under `std.experimental.*`. In the current roadmap, `sep` and `memory` are the primary direct internal lanes for the open memory-model questions; the others remain second-wave or domain-oriented lanes. TLA+, Alloy, typestate, and abstract interpretation remain external sidecars rather than reserved namespace segments. This keeps the experimentation lane explicit without polluting ordinary source syntax or prematurely promoting every research answer into the language surface.
## Appendix E — Decision Matrix and Arithmetic Reference

This appendix condenses the book's most reused quantitative and decision-level facts into one working reference. It is intentionally compact: the main text gives the arguments, while this appendix keeps the constants, default assumptions, loop-shape rules, and maintenance tests close at hand for implementation work.

### E.1 Decision matrix

- **Runtime root:** explicit `Runtime`; requires region-rooted summaries; pays off as explicit host-versus-semantic separation; primarily touches `RES-*`, `OBS-*`, and `KRN-*` diagnostics.
- **State/context model:** optic over costate; requires `OpticSummary`; pays off as direct loops and address arithmetic; primarily touches `TYP-*` and `GRA-*`.
- **Error model:** prisms and typed results; requires branch nodes and prism summaries; pays off as branch hints and mask lowering; primarily touches `OPT-*` and future prism diagnostics.
- **Iteration model:** traversal; requires traversal-legality flags; pays off as SIMD and dense loop formation; primarily touches `VEC-*` and `FUS-*`.
- **Infinite behavior:** coinduction; requires coinductive CGIR nodes; pays off as event loops and queue-aware lowering; primarily touches `OPT-*` and future liveness diagnostics.
- **Specialization:** staging; requires `Stage` nodes and a specialization cache; pays off as monomorphic hot code; primarily touches `FUS-*` and `PER-*`.
- **Ownership:** graded dimension; requires an ownership-aware alias checker; pays off as lock-free or lock-minimized proven-exclusive paths; primarily touches `ALI-*` and `GRA-*`.
- **Alias metadata:** region-based TBAA; requires region trees; pays off as better reordering and vectorization; primarily touches `CGI-*` and `LLV-*`.
- **Backend bring-up:** Rust first, then LLVM; requires a translation-validation harness; pays off as faster semantic debugging; primarily touches `COD-*` and `LLV-*`.
- **Diagnostics:** stable machine-readable schema; requires structured evidence and ranked fixes; pays off as agent efficiency and reproducibility; touches every diagnostic family.

### E.2 Hardware constants and default assumptions

- **Cache line size:** 64 bytes.
- **Page size:** 4 KiB.
- **AVX2 vector width:** 256 bits.
- **AVX-512 vector width:** 512 bits.
- **Common scalar sizes:** `u8=1`, `u32=4`, `u64=8`, `f32=4`, `f64=8`.
- **`Vec2<f32>` stride:** 8 bytes.
- **Padded `Vec3<f32>` stride:** usually 16 bytes in SIMD-friendly layouts.

These defaults are not global truths. They are the assumed baseline for the first target profiles and for the book's arithmetic examples.

### E.3 Grade defaults by domain

- **Kernels:** ownership, latency, blocking, DMA/MMIO, and liveness.
- **Browsers:** cache, staging, traversal, and latency.
- **Databases:** I/O, cache, transaction/session, and staging.
- **Games:** cache, traversal/SIMD, parallel ownership, and latency.
- **Compilers:** compile-time, cache, staging, and provenance.
- **Services:** latency, bandwidth, coinductive liveness, and replay determinism.

### E.4 Loop-shape cookbook

- **Single lens map:** one scalar SoA loop.
- **Product of lenses:** one multi-load/store loop.
- **Traversal with pure arithmetic:** vector loop plus scalar tail.
- **Prism plus traversal:** branchy loop, or a mask plus compact pass when profitable.
- **Composed lens chain:** one fused loop with register-resident intermediate.
- **Coinductive pipeline:** ring or event loop with explicit yield points.
- **Staged operator graph:** one monomorphic specialized loop or function.

### E.5 Maintenance rule

When a future revision proposes a new abstraction, it should be added to this appendix only after the following are written down explicitly:

1. its semantic obligation,
2. its compiler artifact,
3. its legality condition,
4. its machine consequence,
5. the rejected alternative.

If any of those are missing, the feature is not ready for the core language.

### E.6 Closing note

The design is ambitious, but the book's real argument is modest: keep the semantic core small enough that it can be proven in code, then let every later feature justify itself by the same standard.

That is the discipline that makes the full language credible. It is also the discipline that makes the path to kernels, browsers, databases, games, compilers, and a self-hosted ecosystem believable rather than theatrical.

---

## Appendix F — Boundary Contracts, Unsafe/FFI Reference, and Diagnostic Families

This appendix gives the compact reference form of the book's foreign-boundary story. The main chapters explain why `unsafe`, FFI, address spaces, callbacks, and privileged operations stay inside the same semantic model; this appendix keeps the field layout, surface forms, and diagnostic families easy to recover during implementation.

It is the compact reference for the **boundary** lane of the closure rule in §27.17: these mechanisms are fully supported, but they do not become a second semantic center of the language.

### F.1 The minimal-extension rule

The language should become fully general by extending the existing model, not by creating a second one.

- **Keep** `Runtime`, `HostContext`, `OpticSummary`, `RegionSet`, grades, CGIR, and staged execution.
- **Add** boundary contracts to the existing summaries.
- **Prefer** safe wrappers and boundary optics over raw foreign items in ordinary code.
- **Treat** `unsafe` as a trusted boundary declaration, not as "turn off the language".

### F.2 Preferred representation

```rust
struct BoundaryContract {
    kind: BoundaryKind,
    abi: Option<AbiKind>,
    callconv: Option<CallConv>,
    unwind: UnwindPolicy,
    may_callback: bool,
    reentrant: Reentrancy,
    thread_affinity: ThreadAffinity,
    context: ExecContext,
    address_space: AddressSpace,
    volatility: Volatility,
    atomicity: Atomicity,
    privilege: PrivilegeLevel,
    pinning: PinRequirement,
    allocator: AllocatorContract,
    layout: LayoutContract,
    stageability: Stageability,
    safety_clauses: Vec<SafetyClause>,
}

struct OpticSummary {
    // existing summary fields
    boundary: Option<BoundaryContract>,
}
```

### F.3 What should remain a grade

Use grades when the property composes like a budget or quantity.

- cache footprint,
- latency,
- bandwidth,
- blocking budget,
- liveness,
- ownership strength,
- NUMA penalty,
- optional atomic-cost categories.

### F.4 What should remain a qualifier or contract

Use qualifiers/contracts when the property describes how a boundary must be crossed rather than how a quantity composes.

- ABI and symbol naming,
- calling convention,
- unwind policy,
- may-callback / reentrancy,
- thread affinity,
- privilege level,
- address space,
- volatility,
- layout guarantees,
- allocator ownership and pinning.

### F.5 Recommended surface forms

| Surface form | Role | Expected use |
|---|---|---|
| `extern fn` | raw ABI declaration | lowest-level import/export fact |
| `unsafe optic` | graph-facing boundary wrapper | preferred unit for unsafe or foreign interaction |
| `safety { ... }` clauses | explicit preconditions and guarantees | localize trusted assumptions |
| typed address-space wrappers (`Mmio<T>`, `Dma<T>`, `ManagedHandle<T>`) | distinguish memory domains | prevent ordinary optimizations from crossing special boundaries unsafely |

### F.6 Recommended CGIR reuse

Do not introduce a separate foreign IR unless the existing leaf model fails completely. Prefer:

```text
OpticLeaf + BoundaryContract + LeafImplementationKind
```

Possible implementation kinds:

- `Local`
- `Extern`
- `Intrinsic`
- `Volatile`
- `Asm`
- `ManagedBridge`

### F.7 Required diagnostic families

| Prefix | Meaning |
|---|---|
| `FFI-*` | ABI, symbol, or layout mismatch |
| `UNS-*` | unsafe precondition unsatisfied |
| `MMIO-*` | volatile/address-space misuse |
| `DMA-*` | pinning or coherency violation |
| `ATM-*` | atomic-ordering or fence misuse |
| `UNW-*` | unwind/exception boundary violation |
| `CBK-*` | callback, reentrancy, or thread-affinity violation |
| `ASM-*` | inline assembly constraint or clobber violation |
| `MOD-*` | module, plugin, or dynamic-loading contract violation |

Each diagnostic should expose:

- the violated boundary field,
- the exact source span and declaration site,
- the relevant region/grade context,
- the minimal safe repair,
- and the next command or artifact to inspect.

### F.8 Full-generality checklist

A language revision is not yet fully general until it has a coherent answer for all of the following under the same model:

1. raw pointers and address spaces,
2. atomics, fences, volatility, and provenance,
3. FFI ABI and layout control,
4. unwinding and foreign exceptions,
5. callbacks, reentrancy, and thread affinity,
6. allocators, pinning, and foreign ownership,
7. DMA/MMIO and privileged instructions,
8. separate compilation, plugins, and stable ABIs,
9. managed-runtime interop,
10. determinism/replay classification at boundaries,
11. capability gating and auditability,
12. structured diagnostics and tooling support,
13. edition, migration, and deprecation policy,
14. native package declarations, generated lock snapshots, and reproducible-environment policy,
15. module-interface artifacts and cache invalidation rules,
16. debugger/profiler/crash provenance through fusion and staging,
17. conformance suites and translation validation across implementations,
18. supply-chain and generated-binding provenance.

The book's central claim is that all eighteen fit the same architecture once boundary contracts are made explicit and the surrounding policy is made first-class.

## Appendix G — Selected External Reference Points and the Lesson Each One Contributes

This appendix collects the external reference points used in the maturity chapter. Its role is narrow but important: the main text draws lessons from other languages, and this appendix keeps those lessons tied to primary material rather than to community folklore or selective memory.

### G.1 Why this appendix exists

Many later-stage language problems are easy to misremember because they are retold as folklore. The short list below is not meant to be exhaustive; it is meant to keep the maturity chapter anchored to primary material whenever the book draws a cautionary lesson from another ecosystem:

- "C++ modules solved headers"
- "Rust solved FFI safety"
- "Java solved native interop"
- "Python packaging is just a tooling problem"
- "TypeScript is unsound because JavaScript forced it"

Each of those slogans is partly true and partly misleading. The references below are the minimum corrective.

### G.2 C++: modules, consistency, and ABI really are separate problems

| Source | What it contributes to this book |
|---|---|
| [Clang Standard C++ Modules documentation](https://clang.llvm.org/docs/StandardCPlusPlusModules.html) | shows that modules require explicit consistency rules, artifact handling, and ABI discussion rather than magically replacing build-graph complexity |
| [GCC libstdc++ dual ABI documentation](https://gcc.gnu.org/onlinedocs/libstdc++/manual/using_dual_abi.html) | shows that library and ABI evolution can force long-lived dual compatibility surfaces |

Relevant lesson for Optic: module artifacts and binary interfaces must be designed independently, even if both ultimately derive from the same summaries and graph structure.

### G.3 Rust: boundary facts must be explicit

| Source | What it contributes to this book |
|---|---|
| [Rust Reference — Type layout](https://doc.rust-lang.org/stable/reference/type-layout.html) | makes clear that most layout facts are not generally stable except under explicit representation guarantees |
| [Rustonomicon — FFI and unwinding](https://doc.rust-lang.org/nomicon/ffi.html) | makes explicit that ABI and unwind policy must be named correctly at foreign boundaries |

Relevant lesson for Optic: `BoundaryContract` should remain a first-class summary object, not a comment convention.

### G.4 Java: foreign memory and native calls eventually need a better story than JNI

| Source | What it contributes to this book |
|---|---|
| [JEP 454 — Foreign Function & Memory API](https://openjdk.org/jeps/454) | shows an industrial language and runtime moving to a safer, more structured foreign interop model specifically because JNI was too brittle and dangerous |
| [Project Panama overview](https://openjdk.org/projects/panama/) | shows that native calls, foreign memory, layouts, and generated bindings eventually become a language/runtime project, not a library afterthought |

Relevant lesson for Optic: full generality requires binding generation, layout declarations, and foreign memory to sit inside the language's ordinary static model.

### G.5 Python: packaging and runtime-model shifts become ecosystem events

| Source | What it contributes to this book |
|---|---|
| [Python Packaging User Guide — Overview](https://packaging.python.org/en/latest/overview/) | frames packaging as a deployment- and environment-dependent design problem, not a single universal workflow |
| [Python Packaging User Guide — Virtual Environments](https://packaging.python.org/specifications/virtual-environments/) | shows how environment isolation had to become a standard concept rather than an ad hoc tool convention |
| [Python Packaging User Guide — `pylock.toml`](https://packaging.python.org/en/latest/specifications/pylock-toml/) | shows how reproducible installation eventually needs a standardized lockfile |
| [PEP 703 — Making the GIL Optional](https://peps.python.org/pep-0703/) | shows that a concurrency-model shift can ripple into ABI and extension-compatibility policy |
| [Python free-threading HOWTO](https://docs.python.org/3/howto/free-threading-python.html) | shows the operational consequences of that shift for real users and extensions |

Relevant lesson for Optic: package metadata, lockfiles, runtime-model declarations, and compatibility policy should be designed before the ecosystem hardens around accidental assumptions.

### G.6 TypeScript: explicit build graphs and explicit soundness tradeoffs

| Source | What it contributes to this book |
|---|---|
| [TypeScript Handbook — Type Compatibility](https://www.typescriptlang.org/docs/handbook/type-compatibility.html) | explicitly documents the language's soundness tradeoffs rather than pretending they do not exist |
| [TypeScript Handbook — Project References](https://www.typescriptlang.org/docs/handbook/project-references.html) | shows that compiler-understood project boundaries become necessary once codebases and editor workloads grow |

Relevant lesson for Optic: if the language ever relaxes a proof, it should do so explicitly; and if the build graph matters for performance, the compiler must own that fact rather than leaving it entirely to external tools.

### G.7 How to read these references without overfitting to them

These references should not be read as templates to copy line for line.

- Optic is not C++, so it should not inherit C++'s template and ABI complexity by default.
- Optic is not Rust, so it can choose a different unsafe surface while still taking boundary explicitness seriously.
- Optic is not Java, so it does not need to carry JVM constraints into its core model.
- Optic is not Python, so it can choose more static structure around packaging and concurrency.
- Optic is not TypeScript, so it should not normalize convenience-driven unsoundness in ordinary code.

The purpose of the appendix is narrower: it anchors the book's maturity warnings in concrete, documented pressure points from other ecosystems.

### G.8 The practical summary

If the book's main architecture is sound, then the original external references imply the following concrete priority order for the next maturity wave.

1. define editions and compatibility surfaces,
2. define module interface artifacts,
3. define native package declarations and generated lock snapshot semantics,
4. freeze the memory and runtime model before promising plugin ABI,
5. standardize foreign binding generation and boundary diagnostics,
6. standardize debug/profiler/crash provenance,
7. publish conformance suites and artifact schemas.

That order is not accidental. It follows the order in which other ecosystems discovered that a technically strong core was not yet a complete language.

### G.9 Go: source compatibility and module discipline are part of the language experience

| Source | What it contributes to this book |
|---|---|
| [Go 1 and the Future of Go Programs](https://go.dev/doc/go1compat) | shows the value and limits of an explicit source-compatibility promise |
| [Go Modules Reference](https://go.dev/ref/mod/) | shows that module identity, dependency management, and deterministic builds belong to the ordinary toolchain model |
| [Go Modules wiki FAQ](https://tip.golang.org/wiki/Modules) | highlights the role of `go.mod`/`go.sum` in reproducible builds and auditable dependency state |

Relevant lesson for Optic: package identity, compatibility scope, and compiler-visible artifacts should be designed together rather than split between loosely connected tools.

### G.10 Swift: binary evolution has real semantic and performance consequences

| Source | What it contributes to this book |
|---|---|
| [ABI Stability and More](https://www.swift.org/blog/abi-stability-and-more/) | separates ABI stability, module stability, and library evolution into distinct operational concerns |
| [Library Evolution in Swift](https://www.swift.org/blog/library-evolution/) | shows that binary evolution support changes performance characteristics and some source-language expectations |
| [Swift Package Manager documentation](https://docs.swift.org/package-manager/PackageDescription/PackageDescription.html) | shows a language-owned manifest/tooling surface with explicit tools-versioning and dependency semantics |

Relevant lesson for Optic: binary resilience, module interfaces, and native package declarations should remain explicit contracts rather than informal promises.

### G.11 Kotlin: gradual migration is a first-class product feature

| Source | What it contributes to this book |
|---|---|
| [Calling Java from Kotlin](https://kotlinlang.org/docs/java-interop.html) | shows deep design attention to seamless use of a dominant legacy ecosystem |
| [Calling Kotlin from Java](https://kotlinlang.org/docs/java-to-kotlin-interop.html) | shows that public API shaping for interop needs official conventions |
| [Adding Kotlin to a Java project](https://kotlinlang.org/docs/mixing-java-kotlin-intellij.html) | shows mixed-language builds as a normal workflow rather than a special migration mode |

Relevant lesson for Optic: interop and mixed-language builds should be part of the standard path from the start.

### G.12 C# and .NET: deployment family and interop generation belong in the core tool story

| Source | What it contributes to this book |
|---|---|
| [Native AOT deployment overview](https://learn.microsoft.com/en-us/dotnet/core/deploying/native-aot/) | shows one language spanning JIT-hosted and ahead-of-time deployment families with different operational assumptions |
| [P/Invoke source generation](https://learn.microsoft.com/en-us/dotnet/standard/native-interop/pinvoke-source-generation) | shows foreign interop generation moving compile-time work out of runtime stubs |

Relevant lesson for Optic: runtime family and binding generation should be compiler-visible design elements, not afterthought tooling.

### G.13 Node.js and Elixir: runtime semantics become ecosystem identity

| Source | What it contributes to this book |
|---|---|
| [Node.js event loop guide](https://nodejs.org/fa/docs/guides/event-loop-timers-and-nexttick) | shows the event loop as the central operational model for a large ecosystem |
| [Elixir processes documentation](https://hexdocs.pm/elixir/1.18.1/processes.html) | shows isolated lightweight processes and message passing as another explicit runtime identity |

Relevant lesson for Optic: event-loop, shared-memory, and process-supervision styles should be treated as explicit runtime families rather than one vague "async" story.

### G.14 Julia: compilation latency and precompiled artifacts are product-level concerns

| Source | What it contributes to this book |
|---|---|
| [Julia performance tips](https://docs.julialang.org/en/v1/manual/performance-tips/) | explicitly treats time-to-first-execution and specialization effects as real user-facing performance concerns |
| [Julia package images](https://docs.julialang.org/en/v1.12-dev/devdocs/pkgimg/) | shows artifactized native-code caches for packages |
| [Julia ahead-of-time compilation docs](https://docs.julialang.org/en/v1.12-dev/devdocs/aot/) | shows the internal structure of compile-time specialization and saved code artifacts |

Relevant lesson for Optic: compile-time execution, staging, and artifact caching must be first-class architectural concerns.

### G.15 D and Nim: interop subsets and loose package discipline create long shadows

| Source | What it contributes to this book |
|---|---|
| [D BetterC](https://dlang.org/spec/betterc.html) | shows the pressure created when a systems language needs a freestanding/interop subset |
| [D memory-safe subset](https://dlang.org/spec/memory-safe-d.html) | shows a useful but explicit safe/unsafe/trusted split |
| [Nimble User Guide](https://nim-lang.github.io/nimble/index.html) | shows package management assumptions that lean on external VCS tooling and lightweight conventions |
| [Using Nimble packages](https://nim-lang.github.io/nimble/use-packages.html) | shows resolution behavior that is practical but less locked-down than industrial reproducibility demands |
| [Nim manual](https://nim-lang.org/docs/manual.html) | shows low-level interop facilities and unsafe casts as part of the language surface |

Relevant lesson for Optic: avoid second-language personalities for freestanding/interop use, and make reproducible artifact identity stricter than a repository-convention workflow.

### G.16 Pony, Idris, and ATS: powerful ideas can still remain niche if the adoption cost stays high

| Source | What it contributes to this book |
|---|---|
| [Pony overview](https://www.ponylang.io/) | shows a language organized around actor-model safety and capability-secure concurrency |
| [Pony actors tutorial](https://tutorial.ponylang.io/types/actors.html) | shows asynchronous behaviours and actor semantics as a pervasive programming model |
| [Pony reference capabilities](https://tutorial.ponylang.io/reference-capabilities/reference-capabilities.html) | shows the cognitive weight of a rich capability lattice |
| [Idris 2 tutorial on dependent types](https://idris-community.github.io/idris2-tutorial/Tutorial/Dependent.html) | shows the expressive power and proof-oriented style of dependent typing |
| [Idris 2 documentation](https://www.idris-lang.org/pages/documentation.html) | makes clear that the implementation and ecosystem are still evolving |
| [ATS language overview](https://www.cs.bu.edu/~hwxi/atslangweb) | shows a language that combines dependent and linear types with C-level performance goals |

Relevant lesson for Optic: expressiveness should be layered so that powerful proof and concurrency machinery does not become mandatory cognitive overhead for ordinary code.

### G.17 What these broader references add to the book's design rules

The original appendix already established several core maturity lessons from C++, Rust, Java, Python, and TypeScript. The additional references broaden that picture in four useful ways.

1. **They add positive examples, not only warnings.** Go and Kotlin show the adoption value of strong compatibility and interop discipline. Swift shows the value of naming binary-evolution costs precisely.
2. **They force runtime family to become explicit.** Node.js, Elixir, .NET, and Julia all show that deployment and runtime model are major architectural facts, not implementation footnotes.
3. **They sharpen the package/build lesson.** Go is a positive control, Nim a caution, Julia a latency/caching pressure point.
4. **They keep Optic honest about adoption cost.** Pony, Idris, and ATS show that strong ideas do not automatically turn into a large ecosystem unless the language also minimizes migration, tooling, and mental-model burden.

The practical consequence is simple: Optic should continue to expand by reusing the current model, but it should do so under a stricter set of operational guardrails than the core chapters originally needed to state explicitly.


### G.18 Historic paradigms and language designs that still inform the architecture

| Source | What it contributes to this book |
|---|---|
| [Clean language features](https://clean.cs.ru.nl/Language_features) | shows uniqueness typing used to justify destructive update and efficient foreign-world interaction inside a strong static discipline |
| [Clean documentation / language report index](https://clean.cs.ru.nl/Documentation) | anchors uniqueness typing as part of a full language definition rather than as a one-off paper idea |
| [The Esterel synchronous programming language: design, semantics, implementation](https://www.sciencedirect.com/science/article/pii/016764239290005V) | shows a language making reactivity and synchrony first-class semantic structure rather than library convention |
| [The synchronous dataflow programming language LUSTRE](https://www.researchgate.net/publication/2984467_The_synchronous_data_flow_programming_language_LUSTRE) | shows synchronous dataflow compiled into efficient sequential code and tied directly to verification methodology |
| [A report on the SISAL language project](https://www.sciencedirect.com/science/article/pii/074373159090035N) | shows a dataflow/single-assignment language pursuing efficient array handling and cost-effective speedup on shared-memory multiprocessors |
| [NSF Turing Awardees — Kenneth E. Iverson](https://www.nsf.gov/cise/turing-awardees) | anchors APL historically as a major programming-language contribution centered on array notation and interactive systems |
| [APL (A Programming Language) — Computing History](https://www.computinghistory.org.uk/sec/2098/APL-%28A-Programming-Language%29/) | highlights whole-array operations and the use of Iverson notation to describe machine structure concisely |
| [ERights.org / E language overview](https://www.erights.org/) | keeps the capability-secure lineage tied to an actual language/runtime project rather than to summary folklore |
| [E language design goals](https://erights.org/e/e-goals.html) | shows capability security and explicit authority shaping the language's goals from the beginning |
| [What You Always Wanted to Know About Datalog (And Never Dared to Ask)](https://www.sigmod.org/publications/dblp/db/journals/tkde/CeriGT89.html) | anchors Datalog as a declarative query tradition optimized around semantic structure rather than raw execution steps |
| [Datalog User Manual](https://datalog.sourceforge.net/datalog.html) | shows a small, implementation-oriented Datalog system built around tabled evaluation and guaranteed query termination |
| [Constructing Programs as Executable Attribute Grammars](https://academic.oup.com/comjnl/article/35/4/376/348233) | shows semantic computation and translation expressed over structured program descriptions rather than manual compiler plumbing |
| [Modular Attribute Grammars](https://academic.oup.com/comjnl/article/33/2/164/385978) | shows how attribute-grammar style reasoning becomes much more practical when modularized rather than fused to one monolithic syntax description |

Relevant lesson for Optic: several of its strongest ideas are not unprecedented but recovered. The design challenge is therefore not to invent them from nothing, but to reframe them so they strengthen one explicit costate/optic/grade architecture instead of becoming disconnected sublanguages or isolated specialist traditions.



### G.19 Coding agents: large-codebase limits are largely context, retrieval, and environment limits

| Source | What it contributes to this book |
|---|---|
| [Effective context engineering for AI agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) | shows that context is finite, that long contexts degrade, and that long-horizon tasks need compaction, structured note-taking, and sometimes multi-agent decomposition |
| [Harness engineering: leveraging Codex in an agent-first world](https://openai.com/index/harness-engineering/) | argues that repository knowledge should be the system of record and shows why giant monolithic instruction files become stale, noisy, and hard to verify |
| [GitHub Copilot coding agent limitations](https://docs.github.com/en/copilot/concepts/coding-agent/about-copilot-coding-agent) | documents concrete operational limits: same-repository context by default, one pull request per task, and compatibility constraints around runners and rulesets |
| [Introducing SWE-Lancer](https://openai.com/index/swe-lancer/) | shows that frontier models still fail on the majority of real software-engineering tasks and that environment assumptions such as internet access materially affect evaluation stability |
| [SWE-ContextBench: A Benchmark for Context Learning in Coding](https://huggingface.co/papers/2602.08316) | shows that experience reuse helps only when retrieval and summarization are selective; poorly chosen context can reduce accuracy and efficiency |

Relevant lesson for Optic: large-codebase agents struggle less with raw syntax generation than with structured retrieval, stale or oversized context, cross-repository boundaries, and environment-dependent verification. The right response is a graph-native project model with stable summaries, explicit artifact identity, and queryable provenance rather than ever-larger prompt manuals.


### G.20 Agent tooling conventions and repository-local agent systems

| Source | What it contributes to this book |
|---|---|
| [How Claude remembers your project](https://code.claude.com/docs/en/memory) | shows `CLAUDE.md` as a durable instruction mechanism, auto memory as a separate layer, and makes clear that concise, scoped instructions work better than oversized manuals |
| [Create custom subagents](https://code.claude.com/docs/en/sub-agents) | shows that specialized subagents with separate context windows, independent permissions, and bounded roles are a first-class pattern for large tasks |
| [Introducing Codex](https://openai.com/index/introducing-codex/) | shows that AGENTS.md-guided, well-scoped tasks can be run in parallel and that multiple agents working on narrow tasks is an intended workflow |
| [AGENTS.md standard](https://github.com/openai/agents.md) | anchors the open, tool-agnostic instruction-file format that this package uses as its canonical hub |
| [Adding repository custom instructions for GitHub Copilot](https://docs.github.com/en/copilot/how-tos/configure-custom-instructions/add-repository-instructions?tool=vscode) | shows the file-level conventions Copilot supports: repository instructions, path-specific instructions, and agent instruction files such as `AGENTS.md`, `CLAUDE.md`, and `GEMINI.md` |
| [Provide context with GEMINI.md files](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/gemini-md.md) | shows that Gemini CLI uses hierarchical `GEMINI.md` context files, reinforcing the need for a concise wrapper rather than a forked instruction system |
| [Kilo custom instructions](https://kilo.ai/docs/customize/custom-instructions) | shows that Kilo supports `AGENTS.md`, `CLAUDE.md`, `CONTEXT.md`, and per-directory instruction files as repository context inputs |
| [Kilo custom modes / agents](https://kilo.ai/docs/customize/custom-modes) | shows that Kilo CLI and extension can define specialized agents as markdown files with frontmatter, which matches the repository-local agent-system approach used here |
| [LangChain multi-agent patterns](https://docs.langchain.com/oss/javascript/langchain/multi-agent) | contributes the practical taxonomy of subagents, handoffs, skills, and router patterns, especially the emphasis on context management and parallelization |

Relevant lesson for Optic: the current tooling landscape is converging on a small set of durable patterns—canonical instruction hubs, path-specific narrowing, specialized subagents, explicit memory, and well-scoped parallel tasks. A repository-local agent operating system should therefore be designed as a first-party graph client and a worked example of those patterns, not as a one-off prompt file.


### G.21 Underutilized mathematics worth keeping as explicit experimental tracks

| Source | What it contributes to this book |
|---|---|
| [The HoTT Book](https://homotopytypetheory.org/book/) | anchors univalence and machine-checkable equivalence as a serious foundation for proof-oriented equivalence work rather than as a vague slogan |
| [Introducing CliffordLayers](https://www.microsoft.com/en-us/research/articles/introducing-cliffordlayers-neural-network-layers-inspired-by-clifford-geometric-algebras/) | shows geometric/Clifford algebra already being turned into practical neural layers for structured physical domains |
| [Clifford Group Equivariant Neural Networks](https://openreview.net/forum?id=n84bzMrGUD) | shows multivector-structured equivariant models with concrete empirical payoff in geometric domains |
| [p-adic Cellular Neural Networks](https://link.springer.com/article/10.1007/s44198-022-00071-8) | shows ultrametric and rooted-tree organization used directly in neural architectures rather than remaining purely abstract number theory |
| [Learning with the p-adics](https://arxiv.org/abs/2512.22692) | shows p-adic representation learning becoming concrete enough to justify exploratory language/tooling interest, while still remaining an early-stage line of work |
| [Sheaf Semantics for Concurrent Interacting Objects](https://www.cambridge.org/core/journals/mathematical-structures-in-computer-science/article/sheaf-semantics-for-concurrent-interacting-objects/604DA5071EA19CDD65293205C066929B) | ties sheaf theory directly to concurrency, deadlock, non-interference, and structured system composition |
| [Sheaves, Objects, and Distributed Systems](https://www.sciencedirect.com/science/article/pii/S1571066108005264) | makes the local-to-global interpretation of sheaves explicit for distributed-system semantics |
| [A Sheaf-Theoretic Characterization of Tasks in Distributed Systems](https://arxiv.org/abs/2503.02556) | shows that sheaf-theoretic task reasoning is still a live research direction for distributed protocols and solvability |
| [Realizability Toposes and Language Semantics](https://era.ed.ac.uk/handle/1842/402) | anchors topos theory as a genuine programming-language semantics tool rather than a purely philosophical curiosity |
| [First Steps in Synthetic Guarded Domain Theory: Step-Indexing in the Topos of Trees](https://pure.itu.dk/en/publications/first-steps-in-synthetic-guarded-domain-theory-step-indexing-in-t-2/) | connects the topos of trees directly to guarded recursion and step-indexed models of programming languages |
| [The Guarded Lambda-Calculus: Programming and Reasoning with Guarded Recursion for Coinductive Types](https://pure.au.dk/portal/en/publications/the-guarded-lambda-calculus-programming-and-reasoning-with-guarde/) | shows guarded recursion as a concrete language-design line rather than a purely semantic curiosity |
| [Dynamics of Multi-Agent Actor-Critic Learning in Stochastic Games: from Multistability and Chaos to Stable Cooperation](https://arxiv.org/abs/2601.07142) | shows dynamical-systems language—multistability, chaos, basin structure—already becoming operational in multi-agent AI analysis |
| [A Survey of the Proof-Theoretic Foundations of Logic Programming](https://arxiv.org/abs/2109.01483) | reinforces that proof theory itself remains an underused but practical design source for executable semantic systems |

Relevant lesson for Optic: some mathematically rich directions are worth reserving structurally before they are justified as core language features. The right default is to give them a single experimental lane—graph-visible, benchmarkable, and removable—rather than either ignoring them or prematurely promoting them into the everyday surface language.

### G.22 How these mathematics map onto the Optic design

The practical mapping used in the book is intentionally narrow.

- **HoTT / univalence** strengthens proof and equivalence artifacts.
- **Geometric / Clifford algebra** strengthens first-party domain numerics, layout, and backend hooks for geometry-heavy systems.
- **Ultrametric and p-adic ideas** strengthen hierarchical indexing, graph retrieval, and agent-memory organization rather than the core numeric tower.
- **Sheaf theory** strengthens local-to-global consistency across partial graph views, distributed runtimes, and package or plugin boundaries.
- **Topos and guarded-domain ideas** strengthen proof-facing accounts of coinduction, staging, and guarded recursion.
- **Dynamical-systems and ergodic ideas** strengthen runtime-family, queue, scheduler, and agent-swarm stability analysis.

The important closure rule is that none of these should first appear as a second permanent sublanguage. They should first appear as `std.experimental.*` tracks with explicit witnesses, diagnostics, benchmarks, and promotion criteria.


### G.23 Solver systems, equation frontends, and nonstandard numerical methods

| Source | What it contributes to this book |
|---|---|
| [Devito documentation](https://devitocodes.github.io/) | shows that dense symbolic finite-difference notation can be treated as a high-level frontend that lowers into generated optimized kernels rather than requiring a second semantics in the host language |
| [Devito API and overview docs](https://devitocodes.github.io/devito/) | reinforces the idea that equation/stencil frontends and code generation can stay compiler-shaped and artifact-aware |
| [ModelingToolkit.jl home and `System` documentation](https://docs.sciml.ai/ModelingToolkit/dev/) | shows a layered design where a symbolic `System` representation sits between high-level modeling notation and generated numerical code |
| [ModelingToolkit internals](https://docs.sciml.ai/ModelingToolkit/dev/internals/) | shows structural transformation and graph representations as first-class internal compiler objects for solver/model generation |
| [Diffrax documentation](https://docs.kidger.site/diffrax/) | shows that many solver families can also live behind a unified library API rather than requiring a separate user-facing DSL |
| [Nonstandard finite difference schemes for a fractional-order Brusselator system](https://link.springer.com/article/10.1186/1687-1847-2013-102) | gives a concrete example of nonstandard-analysis-inspired numerical methods being useful in discretization rather than merely in abstract logic |
| [Exact and nonstandard finite difference schemes for the generalized KdV–Burgers equation](https://link.springer.com/article/10.1186/s13662-020-02584-2) | reinforces that nonstandard finite-difference ideas can live at the solver/discretization layer without demanding a whole new programming language |
| [Simple Nonstandard Analysis and Applications](https://link.springer.com/content/pdf/10.1007/978-94-017-7327-0_1) | anchors hyperreal and infinitesimal reasoning as a serious analysis tool while reminding the reader that it is still a mathematical framework, not a drop-in replacement for ordinary floating-point engineering |

Relevant lesson for Optic: solver-heavy domains do not automatically justify a core solver DSL. A language can support equation notation, imported modeling frontends, generated stencils, symbolic preprocessing, and even nonstandard-analysis-inspired numerical ideas while still insisting that the lowered semantics remain ordinary optics, staged graph evaluation, generated artifacts, and ordinary diagnostics.


### G.24 Direct memory-model companions that fit the current Optic architecture

| Source | What it contributes to this book |
|---|---|
| [Iris Project](https://iris-project.org/) | anchors higher-order concurrent separation logic as a practical framework for reasoning about resources, invariants, and concurrent safety without inventing a new surface programming language |
| [Iris tutorial materials](https://iris-project.org/tutorial-material.html) | shows that Iris is teachable enough to function as a serious experimental companion rather than a purely specialist curiosity |
| [Iron: Managing Obligations in Higher-Order Concurrent Separation Logic](https://iris-project.org/iron/) | makes obligations such as reclamation and resource disposal explicit, which aligns well with Optic’s boundary and ownership contracts |
| [Actris](https://iris-project.org/actris/) | shows that session-style reasoning can be embedded in separation logic rather than requiring a separate foundational commitment first |
| [Aneris](https://iris-project.org/aneris/) | shows that distributed and network reasoning can be layered on the same resource-logic foundation |
| [Cerberus](https://www.cl.cam.ac.uk/~pes20/cerberus/) | makes pointer provenance and low-level memory semantics the primary problem rather than an afterthought |
| [Exploring C Semantics and Pointer Provenance](https://www.cl.cam.ac.uk/research/security/ctsrd/pdfs/201901-popl-cerberus.pdf) | shows why provenance must be modeled explicitly if compiler optimizations and systems code are to agree |
| [Stacked Borrows](https://plv.mpi-sws.org/rustbelt/stacked-borrows/) | gives an operational aliasing model that directly targets optimizer-facing reasoning about unsafe code |
| [Tree Borrows](https://research.ralfj.de/papers/2025-pldi-tree-borrows.pdf) | provides a newer aliasing/provenance refinement path for unsafe Rust-style reasoning that is close to Optic’s access- and region-oriented concerns |
| [A promising semantics for relaxed-memory concurrency](https://pure.kaist.ac.kr/en/publications/a-promising-semantics-for-relaxed-memory-concurrency/) | gives a direct route to weak-memory and reorder legality that is much closer to Optic’s open atomics/fence questions than the richer categorical tracks |
| [Rust Reference: Memory model](https://doc.rust-lang.org/reference/memory-model.html) | usefully demonstrates that a language can publish an incomplete but explicit memory-model skeleton early |
| [Rust atomics](https://doc.rust-lang.org/core/sync/atomic/index.html) | makes clear that an access-based reinterpretation of the C++20 atomic rules is a practical near-term design choice |

Relevant lesson for Optic: the most direct answers to provenance, alias safety, weak memory, and boundary legality come from **resource/separation reasoning plus weak-memory operational semantics**. Those approaches align with `RegionSet`, fractional ownership, `BoundaryContract`, and backend legality with fewer new semantic centers than the richer proof or categorical tracks.

### G.25 Simpler stabilizing sidecars and narrower alternatives

| Source | What it contributes to this book |
|---|---|
| [TLA+ home](https://lamport.org/tla/tla.html) | frames TLA+ as a high-level language for modeling software and distributed systems and a practical companion for eliminating design errors before code |
| [Specifying Systems](https://lamport.org/tla/book.html) | shows that protocol, liveness, fairness, and composition questions can often be settled in an external specification language before the host language grows new features |
| [Alloy tutorial](https://alloytools.org/tutorials/online/maintext-default.html) | shows a lightweight relational modeling language with fully automatic bounded analysis |
| [What are Alloy and the Alloy Analyzer?](https://alloytools.org/faq/what_are_alloy_and_the_alloy_analyzer.html) | reinforces Alloy as a compact structural modeling and counterexample-finding sidecar rather than a general-purpose implementation language |
| [Abstract Interpretation in a Nutshell](https://www.di.ens.fr/~cousot/AI/IntroAbsInt.html) | anchors abstract interpretation as the standard way to build conservative static analyses from abstractions of program semantics |
| [Typestate Programming — The Embedded Rust Book](https://doc.rust-lang.org/beta/embedded-book/static-guarantees/typestate-programming.html) | shows typestate as a practical, lightweight way to encode protocol/state transitions in ordinary typed code |
| [The Plaid Programming Language](https://www.cs.cmu.edu/~aldrich/plaid/) | gives a language-level typestate example and shows how protocol/state transitions can be first-class without invoking a broader session or sheaf foundation |
| [Foundations of Typestate-Oriented Programming](https://www.cs.cmu.edu/~aldrich/papers/toplas14-typestate.pdf) | shows typestate as a real semantic design space rather than a builder-pattern trick |

Relevant lesson for Optic: some unresolved questions are better answered first by **narrower theories or external sidecars**. TLA+ and Alloy are strong companions for protocol, callback, and distributed-runtime design; abstract interpretation is a strong companion for conservative optimizer and checker approximations; typestate is a strong narrower answer for many protocol-state questions before richer session or sheaf machinery is justified. These alternatives let v1 stabilize without forcing every open question into the language core.

### G.26 Practical synthesis for the v1 memory-model and proof backlog

The references above point to a useful ordering rather than a grab bag of theories.

- **Direct internal answers:** resource/separation reasoning and weak-memory operational semantics map most directly onto Optic’s `RegionSet`, ownership fractions, `BoundaryContract`, and optimizer legality rules.
- **Simpler sidecars:** TLA+, Alloy, typestate/protocol automata, and abstract interpretation can answer many protocol, callback, scheduler, and conservative-analysis questions without enlarging the language core.
- **Second-wave refinements:** proof/equivalence tracks, sheaf-style local/global consistency, guarded/topos semantics, and dynamics/stability analysis should be promoted only when the first two layers stop being enough.

The architectural lesson is that a language with one semantic center should prefer **the smallest theory that closes the operational gap** and only then escalate to richer foundations.

### G.27 How these alternatives line up for v1

The references in G.24–G.26 support a clearer ordering than “collect every available theory and start implementing”. A practical v1 ordering is:

1. **direct internal lanes** — separation/resource reasoning plus weak-memory semantics for the memory-model backlog itself;
2. **simpler external sidecars** — typestate, TLA+, Alloy, and abstract interpretation for protocol, scheduler, callback, and approximation work that does not yet need graph-native promotion;
3. **richer second-wave theories** — proof/equivalence, sheaf, topos, and dynamics tracks when the first two layers stop being enough.

The value of this ordering is not merely lower implementation cost. It also keeps the language aligned with its own closure rule: richer theories must show what they add beyond the smaller answers, rather than entering the design because they are mathematically impressive on their own.



## G.22 Historic defenses that Part II recovers

Several Part II artifacts are easier to understand when read as defenses against older compiler and language failure modes rather than as isolated implementation machinery. `OpticSummary` and its future-facing fields defend against front ends that repeatedly rediscover legality facts after lowering. CGIR defends against lowering structural programs into generic control flow too early and then trying to recover fusion or provenance heuristically. Deterministic diagnostics defend against the long history of compilers whose internal rigor is invisible to users and therefore hard for humans and tools to trust.


### G.28 Mixed-domain collisions and the subservience rule for experimental mathematics

The later maturity and experimental-lane chapters now use two additional organizing rules.

First, unlike domains should be made to collide deliberately during validation. It is not enough to show that a renderer, database wrapper, network stack, or solver kernel works in isolation; the stronger test is whether their summaries, grades, provenance, and boundary contracts still compose when they share one graph.

Second, experimental mathematics does not earn a place in the language by introducing a parallel semantics. It earns a place only when it can be lowered back into the same core objects the rest of the language already trusts: `OpticSummary`, `RegionSet`, `BoundaryContract`, `GradeExpr`, target profiles, and CGIR nodes. This is the book's practical closure rule for research features. It ensures that Geometric Algebra, nonstandard numerics, sheaf-style consistency layers, or any future mathematical experiment enrich the compiler without fracturing it into multiple rulebooks.
## Appendix H — Compiler Graph Store, Projection Filesystem, and Tool Protocol Reference

This appendix keeps the graph-resident compiler and tooling architecture in reference form. It does not introduce a second system. It restates, in compact operational terms, how the book's existing ideas—explicit costates, projections, `BuildRuntime`, `Project`, and structured host boundaries—fit together in storage, filesystem views, and direct tool protocols.

It should also be read as the local-storage view of the larger ecosystem-graph formulation from §4.6.1: the mapped project graph is the checked workspace slice, not the entire universe of packages and artifacts.

### H.1 Core idea in one sentence

The graph-store architecture only remains coherent if this sentence stays true all the way through implementation, tooling, and self-hosting.

```text
CompilerGraph is the authoritative build-time costate; files, diagnostics, interfaces, and generated outputs are projections; tools talk to the graph directly through a revisioned protocol.
```

### H.2 Comparative shorthand: semantic image, semantic identity, projections, and derivations

Chapter 4 does the full narrative work. This appendix keeps the short mnemonic close to the operational details.

- **Smalltalk** contributes the lesson that tools should live over the authoritative semantic state, not over stale reconstructions.
- **Unison** contributes the lesson that semantic identity should be stronger than filename identity where meaning is stable.
- **Projectional systems such as MPS** contribute the lesson that one semantic structure can support many coherent views.
- **Graph-first build systems such as Nix** contribute the lesson that build closure and derived artifacts should be explicit and reproducible.

The Optic design keeps all four lessons but refuses to let any single one dominate. The project graph is a semantic image, not a mutable heap image; it uses content addressing where meaning is stable and revisioned ids where operation is hot; it treats text as a first-class projection rather than an obsolete compatibility layer; and it keeps native build declarations inside the language rather than inventing a second authored DSL.

### H.3 Minimal persistent layout

| Region | Responsibility | Notes |
|---|---|---|
| superblock | schema version, workspace identity, root offsets, current revision | fixed offset, one page |
| journal | append-only transaction log and commit markers | source of crash recovery between checkpoints |
| text arena | authored UTF-8 chunks and stable rope pieces | writable through patch transactions |
| interner arena | symbols, paths, canonical strings | read-heavy, deduplicated |
| syntax/HIR/CGIR arenas | packed nodes and edge payloads | generation-counted ids recommended |
| summary/diagnostic tables | hot fixed-size facts | dense, cache-friendly |
| projection table | maps projection ids to graph regions and policies | some writable, most read-only |
| watch table | subscriptions and invalidation cursors | per tool session |
| artifact index | references to sidecar CAS blobs and materializations | large blobs stay out of the hot graph |
| experimental arena | direct internal lanes first (`sep`, `memory`), then proof witnesses, geometric kernels, ultrametric indexes, sheaf/topos records, and stability analyses | graph-native, opt-in, and explicitly provisional |
| agent memory arena | durable decision records, task state, advisory failed-patch records | small, typed, queryable maintenance knowledge stays near the graph; large diffs and transcripts remain sidecar |

### H.4 Projection classes

| Projection class | Typical payload | Writable? | Reinsertion rule |
|---|---|---|---|
| authored text | UTF-8 source module | yes | patch text region and invalidate dependent summaries |
| structured syntax | AST / HIR pretty or serialized view | usually no | derived |
| semantic graph | CGIR / summary / provenance view | no | derived |
| diagnostics | current structured diagnostic stream | no | derived |
| build roots | package / workspace / target / build-plan values | selectively | structured edit over build-root region |
| generated artifact | interface summary, emitted source, object reference | no | materialize or regenerate |

### H.5 Filesystem projection guidelines

Projection filesystems are valuable adapters, but they are not the semantic center. Use them for:

- editor and shell compatibility,
- human inspection,
- gradual adoption,
- and simple read-mostly tooling.

Do not require them for:

- primary incremental compilation,
- semantic refactoring,
- graph queries,
- batched edits,
- or AI-agent repair loops.

### H.5.1 LSP and ordinary editor adapters are first-party compatibility surfaces

The direct graph protocol is semantically primary, but ordinary editor support is not optional. The toolchain should ship a first-party LSP adapter that translates between graph-native revisions and the file-oriented expectations of existing editors, code review tools, and patch workflows. Projection filesystems remain useful compatibility adapters, but the language should not require a bespoke graph-native editor before ordinary development becomes practical.

### H.6 Direct protocol request families

| Family | Representative operations |
|---|---|
| session | open workspace, negotiate schema/protocol version, declare capabilities |
| snapshot | pin graph revision, open read snapshot, release snapshot |
| patch | apply text patch, replace build-root value, submit batched edits |
| init/template | start init wizard, submit structured intent answers, preview scaffold diff, commit generated project roots |
| query | fetch diagnostics, summaries, HIR, CGIR, provenance, target profile |
| query-experimental | fetch direct internal witnesses first (separation/resource and weak-memory), then proof witnesses, geometry kernels, ultrametric indexes, sheaf/topos records, and stability analyses |
| query-agent-memory | fetch decision records, task records, and advisory failed-patch records |
| watch | subscribe to invalidation stream, changed projections, or diagnostics deltas |
| stage/build | evaluate package roots, build plans, stageable subgraphs, artifact plans |
| materialize | export projection to ordinary files, emit generated artifact, write reproducible snapshot |
| explain | explain failed fusion, failed staging, failed grade bound, failed boundary contract, or why a similar patch failed earlier |

The ordering in `query-experimental` is normative rather than cosmetic. Clients should treat the direct internal lanes as the first place to look for answers to memory, provenance, and reorder-legality questions, then try the simpler sidecar outputs attached to those nodes, and only then fall through to richer proof or categorical tracks.

### H.7 New diagnostic families worth reserving

| Prefix | Meaning |
|---|---|
| `GRF` | graph-file format, schema, or recovery issues |
| `PRJ` | projection read/write policy violations |
| `IPC` | direct protocol version, capability, or revision mismatch |
| `VFS` | projection filesystem adapter failure or degraded mode |
| `INI` | init wizard, intent validation, or template-generation failure |
| `XPR` | experimental-lane schema, feature-line, or promotion-policy violations |

### H.8 Staged-build consequence

Once the compiler graph is authoritative, native package declarations and build plans do not need a second handwritten config language. The authored build roots live in graph-backed source modules, and lock snapshots, module interfaces, build plans, and generated manifests become graph projections or sidecar materializations.

This is the cleanest way to keep build, compilation, tooling, and self-hosting under one model. A graph-first `optic init` naturally belongs in the same family: it is simply the earliest project transaction over those build-root regions.


### H.9 Repository agent systems are ordinary graph clients

A repository-local agent operating system should be treated as one more first-party client of the graph protocol rather than as a parallel toolchain. That means task packets, context indexes, shared-memory updates, and failed-patch records should reuse the same revision, capability, and materialization discipline as other graph consumers. Appendix K records the file-level form used in the split book package.
## Appendix I — Soundness Budgets, Artifact Publicity, and Self-Hosting Layer Split

### I.1 Why this appendix exists

The main text argues that a mature systems language needs explicit contracts not only for semantics, but also for escape hatches, artifact classes, and compiler/toolchain structure. This appendix gathers those policies into a compact working reference so they can later be turned into checklists, schemas, and audit tooling without changing their meaning. Chapter 28 is the first such standing checklist chapter; this appendix remains the compact ledger it depends on.

It is the audit-facing companion to the closure rule in §27.17: the point of these ledgers and publicity classes is to keep boundary support explicit and to stop it from silently leaking into the core language.

### I.2 Soundness-budget ledger template

A soundness budget is the language's published ledger of where proofs stop being automatic. The point is not to hide unsoundness behind a respectable name; it is to keep every escape hatch named, localized, and reviewable.

| Escape hatch or boundary | What proof is suspended | Why it is allowed | Required local contract | Expected diagnostics | Audit obligation |
|---|---|---|---|---|---|
| `unsafe optic` | local alias/completeness proof | raw hardware, specialized foreign semantics | `BoundaryContract` + explicit safety clauses | `UNS-*` | local review plus tests |
| `extern` item | ABI/layout/unwind proof | legacy interop | ABI, layout, unwind, callback, and capability facts | `FFI-*` | binding owner |
| raw pointer / address-space cast | pointer provenance and region interpretation | low-level memory manipulation | address-space and ownership contract | `PTR-*` or `FFI-*` | subsystem owner |
| volatile/MMIO access | ordinary reordering and caching assumptions | devices and control registers | volatility, privilege, and mapped-range contract | `MMIO-*` | device owner |
| inline assembly / target intrinsic | optimizer visibility of effects | privileged or target-specific instructions | clobber, fence, privilege, and target-profile contract | `ASM-*` | platform owner |
| staged host access | hermetic compile-time guarantee | controlled build integration | declared capability + determinism class | `STG-*` | build owner |
| gradual grade `?` | static precision of one grade dimension | adoption path | runtime observation wrapper + repair workflow | `GRD-*` | local module owner |
| dynamic plugin load | closed-world interface assumptions | extensibility | signed module/interface contract + loader policy | `PLG-*` | runtime/tooling owner |
| callback entry from foreign runtime | one-way call-stack model | GUI/audio/browser/tool integrations | callback, reentrancy, thread-affinity, and unwind contract | `CBK-*` | boundary owner |

The purpose of the ledger is not to make unsafe code disappear. It is to keep every unsound or partially-sound move named, localized, and auditable.

### I.3 Artifact publicity classes

Experimental mathematics artifacts should default to the narrowest publicity class that still makes evaluation possible. A useful default is:

- graph-native witness records and benchmark summaries are **toolchain-stable** while the experiment line is active,
- large proof traces, generated kernels, and search artefacts remain **internal** or sidecar,
- nothing under `std.experimental.*` becomes **public-stable** until it is promoted through the feature-admission checklist and receives an explicit schema/version promise.


The toolchain should classify emitted artifacts before the ecosystem does it accidentally.

| Class | Examples | Stability promise | Typical consumers |
|---|---|---|---|
| public stable | module interfaces, generated lock snapshots, registry publication manifests, replay/benchmark capsules exchanged between teams | versioned and migratable across editions | packages, registries, CI, remote caches |
| toolchain-stable | diagnostics JSON schema, graph-protocol revisions, selected build-plan and debug-info sidecars | versioned with toolchain family and edition, with migration support | editors, language servers, IDEs, analysis tools, coding agents |
| internal | HIR caches, CGIR caches, solver traces, invalidation journals, provisional optimization notes | no compatibility promise | compiler itself |

A good rule is simple: if third-party tools are expected to consume an artifact directly, it must not remain unnamed private compiler trivia.


### I.3.1 Agent memory and failed-patch records are graph-native but advisory

Not every graph-native artifact is part of the language's semantic contract in the same way. Durable agent memory is a good example. Decision records, task records, validated repair history, and advisory failed-patch records are worth storing in or beside the project graph because later humans and agents benefit from querying them. But they should normally be treated as **toolchain-stable, repo-local advisory artifacts**, not as package-public interface promises.

A useful publicity split is:

- accepted architectural decisions and benchmark explanations may be toolchain-stable and queryable;
- task records and failed-patch records are repo-local and advisory;
- large diffs, transcripts, and speculative notes remain sidecar or private memory, not public graph truth.

This keeps the graph useful without promoting every maintenance artifact into a cross-package compatibility promise.

### I.4 Self-hosting layer split

A self-hosted compiler should be structured in three rings rather than pretending that everything is either standard library or compiler monolith.

| Layer | Typical contents | Should ordinary user code depend on it? | Why |
|---|---|---|---|
| core standard library | collections, text/bytes primitives, paths, numeric utilities, general optic combinators, target-neutral primitives | yes | broadly useful, semantically stable, not specific to compiler revisions |
| first-party toolchain support libraries | spans and file maps, interning, stable hashing, index arenas, graph-store primitives, module-interface codecs, diagnostics schema/renderers, target-profile descriptions, object/debug-info emitters, package resolution, graph protocol | sometimes, by tool authors and advanced users | reusable across compiler, package tool, language server, alternative front ends, and analysis tooling |
| compiler-private crates | grammar tables, HIR/CGIR schemas, type rules, grade solver, summary builder, alias checker, optimizer legality, backend legality, edition migrators, invalidation policy | no | these embody the language's semantic authority and must be free to evolve |

A useful rule of thumb is:

> a large share of the **infrastructure by code volume** may live in reusable first-party libraries, while the **semantic authority of the compiler** remains compiler-specific.

### I.5 Promotion rule

A component should move outward only when all of the following are true.

1. At least two independent consumers need it.
2. Its contract can be described without referring to one specific compiler pass.
3. Stabilizing it costs less than the ecosystem confusion caused by keeping it private.
4. Its evolution story is compatible with editions and artifact versioning.

This rule prevents the standard library from absorbing compiler internals while still letting the toolchain grow a reusable substrate.

### I.6 Formal verification obligations

Several of the book's core claims are asserted but not machine-checked. This section records them explicitly as conjectures with stated proof obligations, so they cannot harden into assumed facts by omission.

| Claim | Current status | Proof target | Priority |
|---|---|---|---|
| Map fusion is semantically sound | Correctness argument in Ch. 10; golden tests | Small-step operational semantics for grade calculus; Lean 4 or Agda | Before M7 full-language traversal fusion |
| Compose fusion is semantically sound | Same | Same | Same |
| Product flattening is semantically sound | Same | Same | Same |
| Grade semiring distributive law holds for all 8 dimensions | Asserted in Ch. 6 and Ch. 15 | Algebraic proof per dimension, especially session and security | Before M7 grade algebra ships |
| Region language over-approximation is conservative | Stated as design intent | Proof that no true alias is reported as non-aliasing | Before M7 traversal alias checking |
| Session type composition is decidable in the narrow scope | Assumed from binary session theory | Mechanical check of the narrow grammar under structural duality | Before M7 session grade ships |
| Fractional ownership semiring is well-formed | Cited from Marshall–Orchard 2024 | Verify the specific composition and partition laws used in Optic match the adopted early carrier model | Before M7 implementation of explicit fractional syntax ships |

#### I.6.1 Why formal verification belongs in the soundness budget

An unsound fusion rewrite is more dangerous than a missing feature because it produces silently wrong programs that pass the type checker. The language's central claim — that fusion is proof-directed rather than heuristic — becomes false if a rewrite is unsound.

The practical approach is not to block all development on formal proofs. It is to treat the proof obligations as explicit tracked debts in the same ledger as the soundness-budget escape hatches. A fusion rule that enters the codebase before its soundness argument is written down is analogous to an `unsafe optic` without a `BoundaryContract`: it is permitted, but it must be flagged and owned.

**Recommended practice:** Each new fusion rule added at M7 and beyond should be accompanied by either (a) a formal proof in the target calculus, or (b) an explicit entry in this table labeled `UNVERIFIED` with the conjectured soundness argument and a tracking issue. The tracking issue remains open until the proof is completed.

#### I.6.2 Target proof framework

The most practical target is Lean 4, for three reasons: it has a strong category-theory library (Mathlib), it is increasingly used for compiler verification, and its proof terms can be extracted and type-checked independently. An alternative is Agda, which has stronger universe polymorphism for the categorical arguments. Either is acceptable; the constraint is that proofs must be machine-checkable by a toolchain that does not require the Optic compiler itself.

The scope for an initial verification effort is deliberately narrow: a small-step operational semantics for the grade calculus (not the full language), covering only the three v0 fusion rules. That is enough to validate the book's core claim — that the v0 optimizer is proof-directed — without requiring a full formal semantics of the surface language.

## Appendix J — Native Project Queries, Graph Transactions, and the Semantic Query Engine

This appendix records the graph-query and graph-transaction story in reference form. It does not introduce a second language. Instead, it shows how the authoritative `Project` graph becomes queryable, rewritable, and distributable while still reusing the ordinary optic model established in the main text.

Under the closure rule in §27.17 and the standing proposal review in Chapter 28, native project queries remain a **core** facility precisely because they reuse ordinary optics; the richer internal QIR remains an implementation detail and therefore never becomes a second user-facing language.

### J.1 One semantic graph, many projections

The query system makes sense only if it inherits the same architectural rule the rest of the compiler already follows: one authoritative graph, many projections, no second semantic center. Chapter 4 established that graph-as-costate principle at the scale of the whole project; this appendix turns that architectural claim into a query and transaction reference.

The project graph is the semantic database of the language. Source text, syntax, HIR, summaries, CGIR, interfaces, diagnostics, artifacts, benchmark metadata, and runtime blueprints are not rival truths. They are projections over one durable graph revision.

The critical rule is therefore:

> if a fact matters to legality, optimization, diagnostics, staging, build planning, or reproducibility, it should be queryable from the Project graph as structured data.

That includes at least:

- node kind and stable ids,
- `OpticSummary` fields,
- `RegionSet` memberships,
- grades and bounds,
- determinism and stageability,
- provenance and fused ancestry,
- boundary contracts,
- dependency edges,
- interface and artifact identity,
- diagnostic records and ranked repairs.

### J.2 Native Project queries as an ordinary optic subset

The source-facing query model should remain inside the language. That includes the durable, shared subset of maintenance knowledge: decisions, validated repair records, and advisory failed-patch records should be queryable through ordinary project roots rather than hidden behind tool-specific history panes.

Project queries are therefore expressed as ordinary optic programs rooted at stable graph projections such as:

- `ProjectSource`
- `ProjectSyntax`
- `ProjectHir`
- `ProjectSummaries`
- `ProjectCgir`
- `ProjectInterfaces`
- `ProjectDiagnostics`
- `ProjectArtifacts`
- `ProjectDependencies`
- `ProjectExperimental`
- `ProjectAgentMemory`
- `ProjectFailedPatches`

A representative query:

A repository-local agent operating system can use exactly the same subset. In practice that means task decomposition, coherence checks, fix synthesis, and release assembly can all be expressed as ordinary project queries plus graph transactions rather than as a second automation DSL. The split book package that accompanies this manuscript uses that idea directly; Appendix K records the resulting file-level operating model.

```rust
pub fn fusion_candidates(p: &[SharedGrade] Project) -> List<NodeId> {
    p.query(
        ProjectCgir
        >>> Nodes
        >>> Filter(|n| n.kind == NodeKind::Compose)
        >>> Filter(|n| !n.flags.fusion_blocked)
        >>> Filter(|n| n.summary.get_grade.cache < 8)
        >>> Map(|n| n.id)
    ).get()
}
```

This subset should remain:

- typed,
- deterministic,
- mostly read-only,
- bounded enough for compile-time use,
- and ordinary in its surface rules.

It should not become a second full user-facing query language with an unrelated parser, type checker, and migration story.

### J.3 Read-only queries versus graph transactions

The previous section describes read-mostly project inspection. Compiler phases and tool actions need one extra capability: they must commit graph revisions.

The right model is:

> read-mostly project queries and mutating compiler passes reuse the same optic substrate, but mutating phases are graph transactions rather than plain queries.

A graph transaction has the common shape:

```text
select graph regions
  -> analyze and derive facts
  -> synthesize replacements or materialized projections
  -> validate invariants
  -> commit a new revision
```

Examples:

| Transaction | Reads | Writes |
|---|---|---|
| parser | text projection | syntax projection |
| HIR lowering | syntax projection | HIR projection |
| summary builder | typed HIR | summary table |
| CGIR construction | HIR + summaries | CGIR projection |
| fusion | CGIR | fused CGIR + provenance links |
| interface emission | summaries + CGIR + target profile | interface artifacts |
| build planning | package/workspace roots + target profile | build-plan artifacts |

This is the compact operational summary of the compiler pipeline as described throughout the book. The same model also covers project initialization: `optic init` should be understood as an early graph transaction that writes build roots, runtime blueprints, `AppWorld` scaffolds, and procedural template projections rather than as a separate boilerplate generator.

### J.4 Internal QIR remains an implementation detail

The internal query engine may still lower project queries and graph transactions into a richer internal Query IR.

A practical internal shape is:

```text
QIR ::= Scan(source)
      | Filter(q, predicate)
      | Project(q, fields)
      | Join(q1, q2, condition)
      | Paths(root, condition)
      | Explain(node)
      | Why(node, property)
      | Repair(goal, node)
```

QIR exists so the compiler and tools can optimize queries, use indexes, prune paths, batch requests, and synthesize fixes efficiently. Experimental roots such as `ProjectExperimental` lower through the same mechanism; they do not get a second query language or a special non-graph protocol.

The book's design rule is that QIR should remain **internal**. The user-facing model is still the ordinary optic subset over `Project` roots. That prevents the language from drifting into “normal code plus an unrelated query language.”

### J.5 Query execution and indexes

The semantic query engine should execute against graph-native indexes rather than against repeated full scans wherever possible.

Useful indexes include:

- `RegionIndex` — region → readers and writers,
- `GradeIndex` — grade buckets, bounds, and violations,
- `KindIndex` — node kind → node ids,
- `DependencyIndex` — node → dependents and closure roots,
- `ProvenanceIndex` — source span → nodes,
- `ArtifactIndex` — artifact key → graph region and projection metadata.

A minimal optimizer should perform:

1. predicate pushdown,
2. projection pruning,
3. index selection,
4. path pruning,
5. limit-aware early termination,
6. cached query reuse by `(query, revision)` hash.

This makes coding-agent workflows and IDE integration fast enough to rely on the graph directly rather than on repeated text scraping.

### J.6 Query-to-fix synthesis

Because diagnostics already carry structured evidence and ranked repairs, the next step is to let the toolchain synthesize fixes from graph facts rather than only from hand-written diagnostic templates.

A repair request is best understood as a constrained graph transformation:

```text
repair { goal grade.cache <= 4; node pipeline }
```

becomes:

- identify the grade-producing subgraph,
- enumerate minimal legal transformations,
- score them by locality, safety, and churn,
- emit a graph patch plus projection edits,
- revalidate against the original constraint.

Typical repair families:

- split a composition,
- insert a stage boundary,
- reorder a product,
- relax a declared bound,
- convert a writer to read-only,
- wrap an unsafe boundary behind a safer optic.

The important language-wide point is that fix synthesis remains graph-native and provenance-aware. It does not guess from text alone.

### J.7 External frontends target the graph, not a parallel IR

The same project graph can host external-language ingestion, experimental mathematics arenas, agent memory, and advisory failed-patch knowledge without creating a second query language.

The rule is:

> external languages may project into the Project graph, but they do not redefine its guarantees.

That yields three extension roles.

1. **Frontends** — C/C++, Rust, or other languages lower into HIR/CGIR plus conservative summaries.
2. **Backends/projections** — CGIR lowers to Rust, LLVM, or other artifact families.
3. **Tooling clients** — editors, profilers, debuggers, and agents query the graph directly.

A frontend contract should guarantee:

- conservative `RegionSet` completeness,
- explicit determinism classification,
- stable provenance mapping,
- a declared capability tier (`opaque`, `regions-only`, `summary-complete`, and so on).

This is what allows mixed-language projects to participate in one semantic toolchain rather than merely coexisting beside it.

### J.8 Distributed build over graph transactions

Once the project graph is canonical, distributed compilation also becomes graph-shaped rather than file-shaped.

A build task is the transitive closure of a subgraph rooted at one exported node or artifact request:

```text
BuildTask = { root, closure, summaries, target_profile, artifact_key }
```

The scheduler operates over the dependency DAG of these tasks. Cache keys are computed from:

- subgraph hash,
- interface hashes,
- target profile,
- relevant staged inputs,
- runtime-family and capability assumptions.

This gives language-agnostic distributed builds because all frontends converge to the same graph substrate.

### J.9 Persistent graph store and direct protocol

The project graph should remain persistent and cheaply queryable. The graph-store design in Appendix H already motivates a single mmap-friendly authoritative file with append-oriented mutation, journaled commits, and fixed hot records. That same structure is what makes native project queries and graph transactions practical.

The direct protocol should therefore be the primary tool surface, with the projection filesystem remaining a compatibility adapter.

Representative protocol families:

- session and capability negotiation,
- read/query over a chosen graph revision,
- patch text or patch graph transaction,
- explain / why / repair,
- subscribe to revision deltas,
- materialize projections or reproducible capsules.

The projection filesystem remains valuable for grep, diff, and file-oriented editors, but the semantic center stays with the graph and the direct protocol.

### J.10 One-sentence summary

The long-range toolchain direction can be stated compactly:

> the Project graph is not only the compiler's internal state; it is the language's semantic database, query surface, transactional build graph, and tooling protocol root.

## Appendix K — Repository Agent Operating System and Cross-Tool Compatibility

This appendix documents the repository-local agent operating system shipped with the split book package. It is not a second semantics for the language. It is a practical, cross-tool collaboration layer built from the same architectural convictions as the rest of the manuscript: one canonical center of truth, explicit summaries and identities, explicit task boundaries, explicit memory, explicit generated artifacts, and explicit validation.

### K.1 Why the package ships an agent operating system at all

The main text argues that large codebases become fragile when too much knowledge is left ambient. That argument applies just as much to the maintenance of the book package itself. The package therefore ships a small operating system for coding agents and human collaborators so that:

- instructions do not fragment across tools,
- work decomposition is explicit,
- context and memory are bounded and reviewable,
- and the single-file release artifact is always regenerated from authoritative split sources.

### K.2 Canonical file roles

| File or directory | Role | Authority level |
|---|---|---|
| `AGENTS.md` | canonical cross-tool instruction hub | canonical |
| `CLAUDE.md` | Claude wrapper and loader | wrapper |
| `GEMINI.md` | Gemini wrapper | wrapper |
| `CONTEXT.md` | Kilo-compatible wrapper | wrapper |
| `.github/copilot-instructions.md` | repository-wide Copilot guidance | wrapper |
| `.github/instructions/*.instructions.md` | path-specific Copilot narrowing | wrapper |
| `.claude/agents/*.md` | Claude project subagents | tool-specific agent files |
| `.kilo/agents/*.md` | Kilo custom agents | tool-specific agent files |
| `agent-system/registry.json` | canonical role registry | canonical |
| `agent-system/memory/SHARED_MEMORY.md` | stable shared repository memory | canonical |
| `agent-system/memory/agents/*.md` | role-specific memory | canonical but local in scope |
| `agent-system/tasks/` | explicit task graph | canonical workflow state |
| `agent-system/generated/*.md` | generated context and compatibility indexes | generated |
| `tools/agent_sync.py` | validates and regenerates the system indexes | generated-artifact maintenance tool |

### K.3 Agent topology

| Agent | Main responsibility | Normal write surface | Typical delegation targets |
|---|---|---|---|
| `orchestrator` | decompose, sequence, merge, and close tasks | task packets, shared memory, broad coordinating edits | every specialist |
| `research-librarian` | gather external evidence and historical comparisons | task packets, Appendix G-related material | none by default |
| `book-architect` | chapter placement, ordering, and cross-link structure | frontmatter, chapter structure, manifest | editor, auditor |
| `chapter-editor` | local prose, tables, examples, and bounded chapter edits | split source chapters and appendices | auditor |
| `coherence-auditor` | numbering, duplication, contradiction, and drift review | task notes, local memory, targeted correction suggestions | none by default |
| `tooling-compatibility` | instruction files and tool-specific wrappers | AGENTS / CLAUDE / GEMINI / Copilot / Kilo files | release-assembler |
| `release-assembler` | generated indexes, validation, and final assembly | generated context indexes, `assembled.md` | none by default |

### K.4 Task packets are the unit of delegated work

The operating system uses one explicit work unit: the task packet. A task packet names the objective, owner, dependencies, read/write set, delegated subtasks, validation steps, and expected outputs.

Packets move through four states:

1. `inbox` — captured but not decomposed,
2. `active` — claimed and subdivided,
3. `review` — waiting on coherence or human approval,
4. `done` — merged and archived.

This makes multi-agent work visible and reviewable instead of burying it in chat history.

### K.5 Shared memory versus per-agent memory

The memory system mirrors the findings summarized in Chapter 28 and Appendix G: context helps only when it stays selective.

- **Shared memory** is for stable repo-wide truths that should affect many future tasks.
- **Per-agent memory** is for role-specific tactics, recurring pitfalls, and unresolved concerns.
- **Failed-patch records** are advisory negative knowledge: they capture a previously attempted change, the evidence that it failed, and whether the warning is still current.

Promotion rule: move a fact from per-agent memory to shared memory only when it is stable, cross-role, and likely to matter again.

Compaction rule: memory files are not transcripts. They should contain durable facts and concise tactics, not full historical conversation.

#### K.5.1 Durable graph-worthy memory versus sidecar scratch

The operating system should mirror the long-range `ProjectGraph` policy described in the main text.

- Keep **durable, semantic, shared** maintenance knowledge close to the authoritative repository memory.
- Keep **large diffs, transcripts, speculative notes, and ranking caches** outside the authoritative memory surface.

In the split package this means checked-in markdown ledgers stand in for the future graph-native memory arena. Later, the same categories can be moved into an actual `AgentMemoryArena` without changing the policy.

#### K.5.2 Failed-patch record schema

A failed patch should be remembered as a caution record, not a ban. A useful minimal schema is:

```text
FailedPatchRecord {
  id,
  goal,
  target_nodes_or_files,
  patch_fingerprint,
  attempted_at_revision,
  outcome,
  reason_class,
  evidence_refs,
  superseded_by,
  revalidate_after,
  status,
}
```

Good outcome classes include: `TypeRejected`, `AliasRejected`, `TestFailed`, `PerfRegression`, `HumanRejected`, and `Superseded`. Good reason classes include: `Unsound`, `Incomplete`, `Policy`, `Regression`, and `Duplicate`.

The important rule is that failed-patch records are **advisory**. Future agents and reviewers should use them to de-rank similar proposals and inspect prior evidence, not to turn one bad attempt into an eternal prohibition.

### K.6 Mutual delegation and subdivision rules

The system supports mutual delegation, but in a disciplined way. A specialist may request another specialist, yet broad cross-agent scheduling should normally be routed through the orchestrator or a human coordinator. This keeps the system compatible with tools that differ in their support for nested subagents or multi-agent teams.

The guiding rule is simple: let specialists stay narrow, and let the orchestrator own graph-wide coordination.

### K.7 Extrapolation to human-driven development

The same structure improves ordinary human software development. Most successful engineering teams already converge on:

- a coordinator or lead,
- specialists,
- bounded work packets,
- explicit review,
- and short written memory that survives handoff.

The repository agent operating system is therefore not a robot-only layer. It is a codification of good large-codebase practice that both humans and agents can follow.

### K.8 Generated indexes and self-maintenance

The package includes a maintenance script, `tools/agent_sync.py`, that validates the file set and regenerates:

- `agent-system/generated/context-index.md`
- `agent-system/generated/tool-matrix.md`

This keeps the system self-maintaining in a narrow, reviewable sense: the canonical role registry is checked in, the generated indexes are reproducible, and drift between the registry, memory files, and tool wrappers becomes visible immediately.

### K.9 Cross-tool compatibility matrix

| Tool | Canonical files used in this package |
|---|---|
| Claude Code | `CLAUDE.md`, `.claude/agents/*.md`, `AGENTS.md` |
| OpenAI Codex | `AGENTS.md` |
| Gemini CLI | `GEMINI.md`, `AGENTS.md` |
| GitHub Copilot / VS Code Copilot | `.github/copilot-instructions.md`, `.github/instructions/*.instructions.md`, `AGENTS.md` |
| Kilo CLI / extension | `AGENTS.md`, `CONTEXT.md`, `.kilo/agents/*.md` |

The compatibility rule is that these files should form a **wrapper stack**, not several competing centers of truth.

### K.10 Operating rules worth keeping short and permanent

- Edit split sources, not `assembled.md`.
- Reassemble on output.
- Run `python tools/agent_sync.py` after changing the agent system.
- Use task packets for multi-file or research-heavy work.
- Keep memory compact and explicit.
- Keep `AGENTS.md` canonical and wrappers thin.
- Let the orchestrator or a human coordinator own wide merges and role arbitration.
