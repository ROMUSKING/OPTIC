## 13. Prisms, Traversals, and the Branch/SIMD Bridge

Before the chapter names any optic-law detail, it should be clear why these two optic kinds enter the language before the rest. A **prism** is worth adding because it gives the compiler an explicit branch form rather than forcing it to rediscover partiality from boolean filters or hidden exception edges. A **traversal** is worth adding because it gives the compiler an explicit bulk-update form rather than hoping that a later backend recovers lane independence from an already-flattened loop nest. The type-theoretic refinement follows that machine picture; it does not replace it.

### 13.1 Prisms: typed partiality as branch structure

A prism focuses zero or one value.

```text
Prism<S, A> ≈ {
  preview: S -> Option<A>,
  review:  A -> S
}
```

The key reason prisms matter to this language is not just that they model optionality elegantly. It is that the machine-level realization of a prism is a branch. Once that is acknowledged explicitly, the design can connect type theory to branch prediction rather than treating control flow as an afterthought.

#### 13.1.1 Why `Option<A>` matters operationally

`Option<A>` is the coproduct `A + 1`. On the machine, eliminating a coproduct becomes a conditional. That is why a prism over an alive-bit, parse success, tag check, or fast-path validation naturally lowers to a branch.

#### 13.1.2 Branch bias as a grade

The full language therefore introduces a lightweight branch-bias grade.

| Bias | Backend consequence |
|---|---|
| `Likely` | emit likely metadata or equivalent branch hint |
| `Unlikely` | emit unlikely metadata |
| `Unknown` | no hint |

This is a good example of the language's style. A small piece of type-level information becomes a zero-cost backend hint with measurable pipeline consequences.

#### 13.1.3 Comparison to mainstream error and branching models

The comparison matters only at the point where it changes the design: other languages split ordinary partiality among nullable values, optionals, exceptions, and narrowing conventions, while Optic keeps recoverable partiality explicit enough to lower as visible branch structure.

| Language | Common partiality mechanism | What it gets right | Why Optic still prefers prisms in the core |
|---|---|---|---|
| **C++** | `bool` + out-params, `std::optional`, `std::variant`, often exceptions in legacy code | flexible and efficient when carefully designed | exceptions hide control edges; optionals are values but not automatically tied to reinsertion/update structure |
| **Java** | `null`, `Optional`, checked or unchecked exceptions | familiar enterprise control flow and rich exception taxonomies | `null` is too implicit, exceptions are too non-local, and `Optional` does not by itself become optimizer-facing branch structure |
| **Python** | `None`, EAFP exceptions, dynamic truthiness checks | concise code and easy local recovery | control shape is highly dynamic and therefore mostly invisible to a static optimizer |
| **TypeScript** | unions with `undefined`/`null`, user-defined narrowing, exceptions from JavaScript | ergonomic API typing | types erase before execution, so the narrowing information does not become a backend contract |
| **Rust** | `Option` and `Result` | the closest mainstream analogue to what Optic wants | prisms make the same value-level partiality composable as an optic form and connect it directly to branch-aware summaries and lowering rules |

The common thread is simple: Optic keeps ordinary failure value-shaped and optimizer-visible, and leaves non-local control for later optional machinery rather than making it the default.

### 13.2 Traversals: many homogeneous foci as bulk dataflow

A traversal enters the language because it is the first abstraction whose machine consequence is already obvious: if the same arithmetic transform applies independently to a regular field-wise layout, the backend should be able to emit packed loads, packed arithmetic, and packed stores rather than rediscovering that opportunity later by heuristic pattern matching. The semantics naturally line up with that data-parallel picture.

```text
Traversal<S, A> ≈ {
  traverse: S -> Vec<A>,
  update:   (S, Vec<A>) -> S
}
```

The machine does not literally need to allocate a `Vec<A>` in the hot path. What it needs is the semantic guarantee that a uniform transformation is being applied over many homogeneous elements.

That is exactly the shape that auto-vectorization and explicit SIMD prefer.

### 13.3 Why traversals map to SIMD so naturally

#### 13.3.1 `SimdEligible` checklist

A traversal is SIMD-eligible when all of the following are provable.

1. element `i` does not read or write element `j`,
2. the layout is regular and stride-uniform,
3. the mapped element type has a supported vector representation,
4. the element body has no prism-like divergence that would destroy lane coherence,
5. alias proofs are strong enough that vector stores are legal.

When these conditions hold, the backend can lower from a semantic traversal to a vector-width-sensitive loop rather than hoping a backend auto-vectorizer rediscovers the same facts later.

A traversal becomes SIMD-eligible when five conditions are met.

1. No cross-element dependencies
2. Uniform stride and regular layout
3. A homogeneous element type with supported lanes
4. No branchy element body that would explode divergence
5. Alias safety strong enough to reorder or batch stores

This is why the traversal chapter belongs next to the grade and memory-layout story. SIMD is not a "later optimization" layered onto an arbitrary abstraction. It is the machine image of a very particular semantic shape.

#### 13.3.2 Comparison to mainstream iteration and bulk-data traditions

The useful contrast is not that other languages lack bulk operators; it is that they usually express them either as library pipelines or engine-specific fast paths. Optic turns the vectorizable case into an explicit semantic kind.

| Language | Familiar bulk-transformation idiom | Why it is useful | What Optic adds |
|---|---|---|---|
| **C++** | STL algorithms, ranges, expression templates, manual SIMD libraries | excellent control when container and layout choices are disciplined | traversal shape, lane legality, and layout assumptions become part of the static semantic object rather than a happy accident of templates plus optimizer heuristics |
| **Java** | Streams, fork/join pipelines, vector API experiments | clear pipeline expression and runtime parallel execution | object layout and stream boxing often blur the direct path from semantic pipeline to cache-lawful native loop |
| **Python** | list comprehensions, generator chains, NumPy/Pandas vector kernels | unmatched expressive ease | serious performance usually moves into special array libraries; Optic tries to make the vector-friendly shape a language-level concept rather than a library escape |
| **TypeScript** | array methods, typed arrays, dataflow libraries | readable pipeline style | optimization is delegated to the JavaScript engine, and typed-array hot paths live beside a broader object world rather than organizing it |
| **Rust** | iterators, `rayon`, `packed_simd`/portable SIMD, explicit slices | very strong zero-cost bulk-programming story | the traversal kind records the vectorization-relevant semantics earlier and more explicitly than a general iterator chain |

Historically, the closest family resemblance is to APL and later array-language traditions. Those systems treated whole-array transformation as the semantic default rather than as a library optimization, and they made it natural to think in terms of bulk shape-preserving operations instead of scalar loops. Optic deliberately recovers that strength in a narrower, more systems-oriented form: the traversal is the array-lawful semantic shape, but it remains tied to explicit `SoA` layout, region summaries, and backend legality checks rather than becoming a universal glyph-heavy surface for every kind of program.

The point is therefore narrow and concrete: the traversal kind records lane legality, layout regularity, and update shape early enough that vectorization is justified by the semantics rather than recovered late by heuristics.

### 13.4 Example: health decay as traversal

#### 13.4.1 Scalar source, vector consequence

```rust
optic AllHealths: GradedTraversal<Entities, f32,
    TraversalGrade<N> + CacheGrade<1>> {
    traverse s => s.healths.iter()
    update   (s, f) => s.healths.iter_mut().for_each(f)
}
```

A backend targeting AVX2 can turn the hot path into an 8-lane `f32` loop; AVX-512 can widen that further. The important claim is not the exact speedup number. It is that the traversal semantics, SoA layout, and alias proofs together make the vector lowering *earned* rather than speculative.

```rust
optic AllHealths: GradedTraversal<Entities, f32,
    TraversalGrade<N> + CacheGrade<1>> {
    traverse s => s.healths.iter()
    update   (s, f) => s.healths.iter_mut().for_each(f)
}
```

If the traversal body is pure arithmetic, the backend can lower this to packed vector loads, arithmetic, and stores. The gain is not accidental. It follows from the traversal's multi-focus semantics and the SoA memory contract.

### 13.5 Lens and prism subtyping

A lens is a total prism. That coercion should be free. The language benefits from making it explicit because mixed pipelines become much more natural.

A total field lens can flow into a prism-accepting pipeline without any runtime wrapper; the compiler simply treats the `Some` arm as unconditional.

### 13.6 Transition

Prisms and traversals extend the static shape vocabulary. The next chapter extends the temporal and operational vocabulary: infinite loops, staged specialization, replay, and observability.

### 13.7 Detailed implementation reference: prisms as typed branch structure

The main chapter established the intuition. The following sections walk the entire chain from the coproduct reading of `Option<A>` to conditional lowering, branch-bias grades, and the reason prism subtyping gives a free coercion from total focus to partial focus.

#### 13.7.1 The type theory

A prism `P` focuses on a value of type `A` that may or may not be present inside a costate `S`. Categorically, it is a morphism in the category of optics with a cocartesian monoidal action:

```text
Prism<S, A> ≈ {
  preview: S -> Option<A>,   -- the partial observation
  review:  A -> S,           -- the injection back
}
```

The lens laws extend to two prism laws:

```text
-- Put-get (partial): if preview(s) == Some(a) then preview(review(a)) == Some(a)
-- Get-put (partial): if preview(s) == Some(a) then review(a) == s
```

These laws enforce that `review` and `preview` are mutually consistent and that the prism does not lose information when the focus is present. The compiler checks these laws statically for any prism body whose `preview` and `review` are pure expressions. For opaque prisms (bodies calling external functions), law checking is asserted at declaration time via `[checked]` or left to property-based tests.

#### 13.7.2 Why `Option<A>` is not just a convenience type

`Option<A>` is the canonical representation of the coproduct `A + Unit` — the sum type where the unit branch means "not present". This matters because the prism's machine-level lowering is exactly a conditional: either the focus is present (take the `Some` branch, produce `A`) or it is not (take the `None` branch, skip the update). The type forces the compiler to generate code that correctly handles both branches, with no risk of silent no-ops masquerading as failures.

#### 13.7.3 The machine-level picture

A prism query lowers to a conditional around the lens-like get/put body:

```text
-- Source:
entities.query(AliveFilter *** PositionView).map(|(h, p)| ...)

-- Lowered CGIR (after product with prism):
for id_0 in 0..entities.len() {
    if entities.flags.is_alive(id_0) {          // prism branch
        let _h = entities.healths[id_0];
        let _p = entities.positions[id_0];
        let (_h_new, _p_new) = update(_h, _p);
        entities.healths[id_0]   = _h_new;
        entities.positions[id_0] = _p_new;
    }
    // else: None branch is a no-op; no stores, no indirection
}
```

The grade arithmetic changes: a `Prism` under `***` with a `Lens` does not add cache pressure for the prism itself when the `preview` check is a single bit read (the common case for entity alive-flags). The cache grade for the conditional test is `CacheGrade<0>` if the flag is in the same cache line as the id counter, or `CacheGrade<1>` for a separate bitset. This is why prisms participate in the grade algebra directly: the conditional test cost is not free.

#### 13.7.4 Branch prediction implications

Because the prism branch is known to be data-dependent (not control-flow-dependent), the optimizer can emit `unlikely` hints for the `None` arm in dense living-entity worlds, or `likely` hints for sparsely-populated dead-entity passes. The grade carries a `BranchBias` annotation in the full language:

```rust
optic AliveFilter: GradedPrism<Entities, f32,
    CacheGrade<1> + BranchBias<Likely>>
```

`BranchBias<Likely>` tells the backend to emit `[[likely]]` on the `Some` branch in LLVM IR. `BranchBias<Unlikely>` emits `[[unlikely]]`. This is a grade dimension that is zero-cost after erasure but has a measurable effect on branch predictor warmup in the generated native code.

#### 13.7.5 Prism subtyping: Lens <: Prism

A `Lens` is a `Prism` that never returns `None`. In the type system, `Lens<S,A>` is a subtype of `Prism<S,A>` because every lens is a total prism. This means a function that accepts a `Prism` will also accept a `Lens`. The coercion is free: the compiler converts the lens's `get` into `preview = |s| Some(s.get())` during subtyping.

The key consequence is that mixed compositions like `Lens >>> Prism` type-check naturally. The composed optic is a `Prism`: it focuses on the lens focus if it exists, then optionally focuses further if the prism focus exists. Grade arithmetic carries through because the sequential composition rule still applies.

---

### 13.8 Detailed implementation reference: traversals, `TraversalGrade`, and SIMD eligibility

Traversals are where the language’s semantic uniformity starts paying direct machine dividends. The detailed sections below make the traversal-to-SIMD bridge explicit and spell out the conditions under which the backend may legally replace scalar loops with vector loads and stores.

#### 13.8.1 The type theory

A traversal focuses on zero or more values of type `A` inside a costate `S`. It is the optic corresponding to the `Traverse` type class in Haskell, but in the coalgebraic model it is a coalgebra for the monad of finite multisets:

```text
Traversal<S, A> ≈ {
  traverse: S -> Vec<A>,         -- all focused values (observations)
  update:   (S, Vec<A>) -> S,    -- update all focuses simultaneously
}
```

The key traversal law is **shape preservation**: the length of the `Vec<A>` returned by `traverse` must equal the length expected by `update`. Violating this is a type error (checked statically for pure traversals, asserted dynamically for opaque ones).

#### 13.8.2 Why traversals need their own grade dimension

A lens reads one element. A traversal reads all elements. The cache cost is not `CacheGrade<1>`; it is proportional to the collection size. For v0 this is expressed as `CacheGrade<N>` where `N` is statically known, or `CacheGrade<∞>` when the collection size is runtime-determined. In the full language, `TraversalGrade<n: Nat>` replaces the approximation with a proper size parameter:

```rust
optic AllHealths: GradedTraversal<Entities, f32, TraversalGrade<{entities.len()}>>;
```

When `n` is a compile-time constant (fixed-size arrays, stack-allocated rings), `TraversalGrade<n>` degrades to `CacheGrade<n / CACHE_LINE_SIZE>` exactly. When `n` is a runtime value, the grade is symbolic and handled by the Z3 solver tier.

#### 13.8.3 The SIMD opportunity

The traversal semantics — "apply the same function to all focused values" — is the precise statement of SIMD vectorization. A traversal body that satisfies:

1. No inter-element dependencies (element `i` does not read or write element `j`)
2. Uniform element stride (SoA layout, not AoS)
3. Arithmetic-only map function (no branches inside the element body)

...can be automatically lowered to SIMD instructions. The CGIR carries a `SimdEligible` flag on `Traversal` nodes that meet these conditions. The LLVM backend uses this flag to emit `llvm.x86.avx2.` or portable `LLVMBuildVectorStore` intrinsics rather than scalar loops.

```text
SimdEligible(traversal):
  traversal.map_fn has no inter-element reads/writes (alias check)
  traversal.costate has uniform element stride (SoA layout, always true in v0)
  traversal.map_fn has no prism branches (all paths execute for all elements)
  traversal.element_type is a SIMD-compatible scalar (f32, f64, i32, i64, u8...)
```

When `SimdEligible` is true, the optimizer emits:

```rust
// optic(simd): AllHealths
let chunks = entities.healths.chunks_exact_mut(8); // AVX2: 8 f32 per register
for chunk in chunks {
    let v = f32x8::from_slice_unaligned(chunk);
    let v_new = v - f32x8::splat(damage);
    v_new.write_to_slice_unaligned(chunk);
}
// handle remainder
for h in entities.healths[aligned_len..].iter_mut() { *h -= damage; }
```

This is a concrete, measurable throughput improvement (typically 6–8× over scalar for f32 arithmetic) that falls out of the traversal semantics without any user annotation.

#### 13.8.4 Traversal fusion with SIMD

When two SIMD-eligible traversals are fused (`AllHealths *** AllDamages`), the optimizer must decide whether to use two SIMD passes or one wider scalar pass. The grade algebra guides this decision:

```text
if combine_par(A.get_grade, B.get_grade).cache <= SIMD_REGISTER_WIDTH / ELEMENT_STRIDE:
    emit single fused SIMD loop (pack both fields into wider registers)
else:
    emit two separate SIMD loops (cache-friendly, prevents register pressure)
```

This is not guesswork. The grade arithmetic directly encodes the register width budget. If the total grade fits in one SIMD register, one pass. Otherwise two. The compiler makes this decision statically, not speculatively.

---

