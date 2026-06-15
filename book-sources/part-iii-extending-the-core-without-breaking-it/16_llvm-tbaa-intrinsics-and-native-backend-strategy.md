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

