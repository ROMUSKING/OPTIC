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

