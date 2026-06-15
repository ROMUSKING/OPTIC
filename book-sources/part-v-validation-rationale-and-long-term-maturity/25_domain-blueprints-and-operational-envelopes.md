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

