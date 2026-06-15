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

