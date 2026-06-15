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

