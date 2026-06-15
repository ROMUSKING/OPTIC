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

