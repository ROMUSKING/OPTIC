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

