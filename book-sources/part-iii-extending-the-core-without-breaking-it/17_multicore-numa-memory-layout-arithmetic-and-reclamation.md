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

