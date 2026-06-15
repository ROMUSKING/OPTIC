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

