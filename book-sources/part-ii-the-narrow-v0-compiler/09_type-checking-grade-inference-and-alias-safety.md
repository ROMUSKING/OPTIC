## 9. Type Checking, Grade Inference, and Alias Safety

### 9.1 Why this phase is more than conventional type checking

The prelude type checker is doing several jobs that many compilers scatter across different stages.

- ordinary type compatibility,
- ownership compatibility,
- grade inference and bound checking,
- product alias legality,
- summary validation,
- future-facing determinism bookkeeping.

The reason to keep them close is that the same artifact, `OpticSummary`, feeds all of them.

This is also where the narrow compiler most clearly acts as a blueprint for later growth. The concrete checker is intentionally small, but it is not a dead end. The same summary-driven discipline that powers the coarse v0 checker is the one later used by fractional ownership, asymmetric grades, and symbolic solving. The narrow checker therefore proves not only that the core laws can be enforced, but that they can be enforced through artifacts the later compiler can refine rather than replace.

### 9.2 The prelude type universe

The v0 type system is intentionally conservative.

- primitives,
- tuples,
- `SoA<T>`,
- `BitSet`,
- monomorphic user `data` types,
- `GradedOptic<S, A, G>`.

This small universe is not just easier to implement. It is easier to make *explainable*. When a product optic fails or a composition mismatches, the compiler can describe the failure in terms of concrete costate and focus types without having to explain higher-rank polymorphism or inference across a large subtype lattice.

### 9.3 Grade combination rules in v0

#### 9.3.1 Concrete carrier and operators

```rust
struct ConcreteGrade {
    cache: u8,                 // 255 = unbounded
    ownership: OwnershipDim,
}

struct OwnershipDim {
    share: Rational,           // 0 < share <= 1
    read_only: bool,           // true for shared borrows
    must_use: bool,            // true for linear resources
}

// Surface aliases used throughout the prelude examples:
// SharedGrade  = inferred read-only share ρ with 0 < ρ < 1
// AffineGrade  = { share: 1, read_only: false, must_use: false }
// LinearGrade  = { share: 1, read_only: false, must_use: true }
```

```text
sat_add(x, y) =
  if x == 255 or y == 255 or x + y > 254 then 255
  else x + y
```

The prelude grade algebra is small enough to state directly.

| Dimension | Sequential `>>>` | Product `***` |
|---|---|---|
| Cache | saturating add | conservative max |
| Ownership | take the stronger requirement (`share = max`, `must_use = or`, `read_only = and`) | prove either structural disjointness or a partition-safe fractional split; otherwise reject |

That table is easy to memorize, but the prose behind it matters.

Sequential cache cost uses saturating add because separate stages still touch separate field families even after fusion. Product uses conservative max because the stages share a pass and the hardware can often overlap some of the locality cost, but the compiler stays conservative.

Ownership is now slightly richer than the v0-era prose suggested. For ordinary composed optics, sequential composition simply preserves the stronger ownership requirement of the two children. Parallel product is the interesting case. It is legal for two reasons only:

1. the regions are structurally disjoint in the existing conservative region language; or
2. the regions participate in an already-established partition witness and the claimed ownership fractions sum to at most one.

This is the precise place where early fractional ownership earns its keep. The checker still accepts the easy field-wise cases through structural disjointness, but it no longer needs a later carrier swap when partition-shaped multicore work begins to matter.

### 9.4 Why grade inference is region-driven

A prelude compiler should not infer grades from source syntax quirks. It should infer them from normalized region summaries.

```text
reads  = distinct field roots touched by get
writes = distinct field roots touched by put
cache  = reads + writes, saturated
```

This is intentionally coarse. That coarseness is a feature in v0 because it makes the algorithm stable across harmless rewrites.

### 9.5 Alias safety under product composition

#### 9.5.1 Alias-checking sketch

```text
overlaps(r1, r2):
  is_subregion(r1, r2) or is_subregion(r2, r1)

same_partition_family(r1, r2):
  r1.partition_witness is not None
  and r1.partition_witness == r2.partition_witness

alias_check(left, right):
  left_effective  = left.get_reads ∪ left.put_reads ∪ left.put_writes
  right_effective = right.get_reads ∪ right.put_reads ∪ right.put_writes

  for each overlapping pair (l, r) with l in left.put_writes and r in right_effective:
    if left.ownership.read_only and right.ownership.read_only:
      require left.ownership.share + right.ownership.share <= 1
      continue
    if same_partition_family(l, r):
      require left.ownership.share + right.ownership.share <= 1
      continue
    return conflict(left_region=l, right_region=r)

  repeat symmetrically for right.put_writes against left_effective

  return safe
```

The algorithm remains conservative on purpose. Structural region disjointness still proves the easy cases; fractional arithmetic takes over only when the program has already established a partition witness. This is the precise compromise the book now adopts: front-load the harder carrier without pretending the prelude can already infer every dynamic partition automatically.

Parallel product is one of the most attractive surface forms in the language because it reads like a data-oriented system query. It is also one of the places where unsoundness could sneak in if the compiler were optimistic.

The rule is now slightly richer than the original v0 prose implied.

```text
alias_safe(left, right) iff
  for every overlapping or potentially-overlapping region pair:
    either the regions are structurally disjoint,
    or both sides are read-only and their shares sum to at most one,
    or both sides participate in the same established partition family and their shares sum to at most one;
  otherwise reject.
```

The important point is still that `put_reads` counts. If a write on one side overlaps with the other side's read-for-update path, the product is unsafe even if the second side never stores directly to that field. Fractional ownership does not weaken that rule; it only gives the checker a principled way to accept partitioned parallel products without inventing a second ownership system later.

### 9.6 Why the checker stays conservative

The prelude does not try to prove index-level disjointness, dependence on runtime values, or deep symbolic region separation. It normalizes indexed SoA access to `field[*]` and accepts the resulting false rejections.

That choice is not cowardice. It is the correct first trade.

- A false rejection is annoying but explainable.
- A false acceptance is unsound and poisons later optimization.

A compiler meant to support agent workflows should prefer conservatism with excellent evidence over adventurous acceptance with hidden traps.

#### 9.6.1 Boundary contracts extend `OpticSummary` without creating a second effect system

The narrow checker already depends on `OpticSummary` to carry the facts that later phases need: regions, grades, determinism, and provenance. The most conservative way to admit FFI, `unsafe`, MMIO, callbacks, and legacy runtimes is therefore to extend that same summary rather than invent a parallel subsystem.

A minimal extension looks like this.

```rust
struct BoundaryContract {
    kind: BoundaryKind,              // Local | Extern | Intrinsic | Asm | Volatile
    abi: Option<AbiKind>,
    unwind: UnwindPolicy,            // NoUnwind | MayUnwind | ForeignException
    may_callback: bool,
    reentrant: Reentrancy,           // No | Yes
    thread_affinity: ThreadAffinity, // Any | Main | Render | Audio | Cpu(n)
    address_space: AddressSpace,     // Ram | Mmio | Dma | Gpu | ForeignHeap | ManagedHeap
    volatility: Volatility,          // Ordinary | Volatile
    atomicity: Atomicity,            // None | Atomic(ordering)
    privilege: PrivilegeLevel,       // User | Kernel | Interrupt
    pinning: PinRequirement,
    allocator: AllocatorContract,
    layout: LayoutContract,
    stageability: Stageability,      // Static | Residual | Dynamic
    safety_clauses: Vec<SafetyClause>,
}

struct OpticSummary {
    // existing fields omitted
    boundary: Option<BoundaryContract>,
}
```

The key design choice is what does **not** change. The language still has the same root runtime, the same region model, the same `get_reads`/`put_reads`/`put_writes` split, the same grade product, and the same CGIR composition operators. A foreign or unsafe leaf is still an optic leaf. It simply carries extra obligations.

That keeps the architecture small enough to reason about. The type checker still asks: what regions are read, what regions are written, what grade is consumed, and what determinism class results? The boundary contract only answers the extra questions that ordinary in-memory optics never had to answer: what ABI is crossed, whether unwinding may escape, whether the call may reenter the language, which address space the memory lives in, whether the operation is volatile or atomic, and what safety preconditions must already hold.

A raw foreign declaration may therefore exist in the surface language, but it becomes useful to the compiler only once it is wrapped in an optic-shaped summary. That is why the book prefers safe or semi-safe wrappers over raw foreign items in ordinary code. The raw item establishes the ABI. The optic wrapper re-enters the graph.

```rust
extern "C" fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8;

unsafe optic MmioReg32: GradedOptic<Mmio<U32Reg>, u32,
    BlockingGrade<Never> + LinearGrade>
{
    get  r => volatile_load_u32(r.base + r.offset)
    put  (r, v) => volatile_store_u32(r.base + r.offset, v)
    safety {
        requires privilege(Kernel)
        requires mapped_mmio(r.base, 4)
        requires aligned(r.base + r.offset, 4)
        ensures no_unwind
    }
}
```

The first declaration is a raw ABI fact. The second is the graph-facing, summary-bearing object the rest of the language can reason about.

### 9.7 Diagnostics must expose the proof, not just the verdict

A grade or alias error becomes useful only when the compiler tells the user what it saw.

For example:

```text
error[ALI-201]: product alias conflict
  left writes:  app.healths[*]
  right reads:  app.healths[*]
  note: reads in the right optic occur during put-read reconstruction
```

That is better than a generic "cannot borrow mutably" style message because it is phrased in the language's own conceptual units.

### 9.8 Transition

Type checking proves local legality. The next chapter explains why the compiler still needs a graph-shaped IR above SSA, how that IR is constructed, and how it becomes the home of fusion and provenance.

### 9.9 Detailed implementation reference: concrete grade arithmetic and checker structure

The main chapter explains the purpose of the checker; the material below gives the concrete carrier types, inference path, and the solver-separation pattern that keeps v0 forward-compatible with symbolic grades.

#### 9.9.1 Grade representation

In v0, a `ConcreteGrade` is a pair:

```rust
struct ConcreteGrade {
    cache:     CacheDim,      // u8; 0 = no cache cost; 255 = unbounded
    ownership: OwnershipDim,  // Shared | Affine | Linear
}
```

Grade annotations in source can use `_` for inference. The compiler fills in the tightest provable grade.

#### 9.9.2 Grade combination rules

Sequential composition `(A >>> B)`:

```text
combine_seq(a, b).cache     = sat_add(a.cache, b.cache)
combine_seq(a, b).ownership = max_exclusivity(a.ownership, b.ownership)

sat_add(x, y) = if x == 255 || y == 255 || x + y > 254 { 255 } else { x + y }
max_exclusivity(Shared, Shared)   = Shared
max_exclusivity(Shared, Affine)   = Affine
max_exclusivity(Shared, Linear)   = Linear
max_exclusivity(Affine, Affine)   = Affine
max_exclusivity(Affine, Linear)   = Linear
max_exclusivity(Linear, Linear)   = Linear
-- symmetric
```

Parallel product `(A *** B)` (after alias safety confirmed):

```text
combine_par(a, b).cache     = max(a.cache, b.cache)   -- conservative
combine_par(a, b).ownership = max_exclusivity(a.ownership, b.ownership)
```

#### 9.9.3 Grade inference algorithm (v0: concrete only)

##### 9.9.3.1 Region-driven touch counting

The concrete inference path should operate over the normalized region summary, not over surface syntax. That keeps the algorithm stable under harmless source rewrites.

```text
count_distinct_field_reads(expr):
  regions = collect_regions(expr, mode='read')
  return cardinality(normalize_to_field_roots(regions))

count_distinct_field_writes(expr):
  regions = collect_regions(expr, mode='write')
  return cardinality(normalize_to_field_roots(regions))
```

`normalize_to_field_roots` deliberately collapses `app.positions[*].x` and `app.positions[*].y` to the same field family if they live in the same SoA vector, because the v0 cache story is about field-touch count, not scalar-lane count.

##### 9.9.3.2 Why `u8` with 255 = unbounded is the right carrier in v0

`u8` keeps grades cheap to serialize, compare, dump, and embed in diagnostics. More importantly, it makes saturation behavior explicit and testable. Every time the compiler reaches `255`, it is forced to admit that the structural approximation has escaped the bounded regime.

That is a useful signal to both humans and agents. An unbounded grade is not a mysterious solver failure; it is a visible boundary of the prelude's approximation.

Grade inference fills in `_` annotations in optic declarations. The algorithm is a single bottom-up pass over the optic body.

```text
infer_grade(optic_body) ->
  reads  = count_distinct_field_reads(get_body)   -- number of distinct SoA fields read
  writes = count_distinct_field_writes(put_body)  -- number of distinct SoA fields written
  cache  = reads + writes                          -- conservative: one cache line per field
  ownership = if put_body is empty { Shared }
              else if put_body does not read same field it writes { Affine }
              else { Affine }   -- Linear requires explicit declaration for now
  return ConcreteGrade { cache, ownership }
```

Inferred grades are upper bounds; they may be tighter than the true hardware cost. This is intentional: inference is conservative, declaration is the programmer's claim.

If a declared grade is tighter than the inferred grade (e.g., declared `CacheGrade<1>` but body reads two fields), the compiler emits `GRA-110`:

```text
error[GRA-110]: declared grade is tighter than inferred grade
  declared: CacheGrade<1>
  inferred: CacheGrade<2>  (body reads: app.healths, app.positions)
  note: either tighten the body or relax the declared grade
```

#### 9.9.4 Grade bound checking

After composition, the composed grade is checked against any declared bound on the outer let-binding or function signature.

```text
check_grade_bound(composed: ConcreteGrade, declared_bound: ConcreteGrade) ->
  if composed.cache > declared_bound.cache:
    emit GRA-104
  if exceeds_ownership(composed.ownership, declared_bound.ownership):
    emit GRA-121
```

#### 9.9.5 Preparation for two-tier inference (full language)

##### 9.9.5.1 Division of responsibilities between inference and checking

Even before symbolic grades exist, the compiler should keep three logically separate phases:

1. **Summary extraction** — derive structural facts from optic bodies.
2. **Grade expression construction** — build a grade object from those facts.
3. **Bound checking** — compare the resulting grade against declarations or enclosing requirements.

This separation matters because only phase 2 needs a future symbolic solver. Phases 1 and 3 should remain structurally the same when the full-language solver arrives.

The v0 implementation should structure grade checking so that the concrete inference path (§8.3) and the constraint-emission path (§8.4) are separate functions. When symbolic grades are added in the full language, the constraint-emission path will delegate to a Z3 query manager rather than performing arithmetic directly. The architecture must support this substitution without a rewrite.

The Z3 interface (not implemented in v0, but the interface must be sketched):

```rust
trait GradeConstraintSolver {
    fn check_bound(composed: GradeExpr, bound: GradeExpr) -> Result<(), GradeViolation>;
    fn infer_tightest(body: &OpticBody) -> GradeExpr;
}

// v0: concrete arithmetic
struct ConcreteGradeSolver;
impl GradeConstraintSolver for ConcreteGradeSolver { ... }

// full language: Z3 QF_LIA
struct Z3GradeSolver { solver: z3::Solver }
impl GradeConstraintSolver for Z3GradeSolver { ... }
```

This abstraction costs nothing in v0 and eliminates a later architectural rupture.

The important implementation consequence is that the symbolic solver should first arrive as a comparison and advisory lane rather than as a destabilizing replacement for the narrow checker. That preserves the prelude’s auditability while still letting the project measure where the coarse checker is too conservative for realistic workloads.

---

