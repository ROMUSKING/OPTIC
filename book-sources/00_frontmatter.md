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

