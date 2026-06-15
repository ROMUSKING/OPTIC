## 24. Quantitative Theory-to-Machine Bridge

The earlier chapters explain the qualitative bridge from semantics to machine behavior. This chapter makes the bridge quantitative: branch-predictor arithmetic for prisms, vector-lane arithmetic for traversals, cache and page calculations for layouts, and queue-depth reasoning for coinductive event loops.

This chapter collects the quantitative rules of thumb and exact formulas that connect the language's abstractions to hardware behavior. These numbers are not a substitute for benchmarking, but they are the right first-order model for compiler design.

### 24.1 Prisms, branches, and branch-predictor arithmetic

#### 24.1.1 The semantic shape

A prism test is a coproduct elimination:

```text
preview : S -> Option<A>
```

Operationally that becomes a conditional.

#### 24.1.2 The machine model

For a branch with taken probability `p`, a first-order expected penalty model is:

```text
E[cost] = p * cost_taken + (1 - p) * cost_not_taken + miss_rate(p, history) * mispredict_penalty
```

The exact `miss_rate` depends on the predictor, but three compiler-level facts matter immediately:

1. dense success cases (`p` close to 1) want fall-through on the success arm;
2. sparse success cases (`p` close to 0) want fall-through on the failure arm;
3. if the branch body is simple enough, if-conversion or masking may beat an unpredictable branch.

#### 24.1.3 Compiler artifact

`BranchBias<Likely|Unlikely|Neutral>` lives as a zero-cost dimension or annotation on prisms and branch-producing traversals.

#### 24.1.4 Backend legality condition

Hints may only be emitted when the prism semantics remain unchanged under the control transformation. Branch bias cannot justify reordering that would change observable writes.

#### 24.1.5 Backend mapping

| IR fact | LLVM lowering |
|---------|---------------|
| `BranchBias<Likely>` | branch weight metadata favoring the success edge |
| `BranchBias<Unlikely>` | branch weight metadata favoring the failure edge |
| mask-lowerable prism + traversal | predicated vector body or masked stores |

#### 24.1.6 Practical rule

- keep predictable branches as branches;
- convert unpredictable simple prisms to masks when doing so reduces mispredict cost and does not increase memory traffic too much;
- for sparse traversals, consider a two-phase design: compact indices first, then run a dense traversal over the compacted set.

### 24.2 Traversals, vector lanes, and remainder handling

#### 24.2.1 The law

A traversal promises same-shape element-wise visitation.

#### 24.2.2 SIMD legality checklist

A traversal is vectorizable when all of the following hold:

1. no inter-lane data dependency,
2. regular stride or acceptable gather/scatter cost,
3. element operation has no unstructured control dependence,
4. updates do not alias across lanes,
5. remainder handling is well-defined.

#### 24.2.3 Lane arithmetic

For vector width `W` bytes and element size `E` bytes:

```text
lanes = floor(W / E)
```

Examples:

- **`f32` (4 bytes):** 8 lanes on AVX2, 16 lanes on AVX-512.
- **`f64` (8 bytes):** 4 lanes on AVX2, 8 lanes on AVX-512.
- **`u8` (1 byte):** 32 lanes on AVX2, 64 lanes on AVX-512.
- **`Vec2<f32>` (8 bytes):** 4 logical pairs on AVX2, 8 logical pairs on AVX-512.

#### 24.2.4 Remainder policy

The backend must choose one of three remainder strategies:

- scalar tail loop,
- masked vector tail,
- peeled loop with alignment and multiple vector bodies.

The default policy should be:

- scalar tail for short or infrequent tails,
- masked tail when the target has cheap masks and the traversal body is arithmetic-heavy,
- peeled bodies when alignment matters strongly or when the body contains wide loads/stores.

### 24.3 Cache lines, pages, TLBs, and prefetch

#### 24.3.1 Cache arithmetic

Given 64-byte cache lines:

```text
elements_per_line = floor(64 / stride)
lines_for_N       = ceil(N / elements_per_line)
```

Examples:

- **`f32` (stride 4):** 16 elements per 64-byte cache line.
- **`Vec2<f32>` (stride 8):** 8 elements per cache line.
- **`Vec3<f32>` padded to 16:** 4 elements per cache line.
- **`u64` (stride 8):** 8 elements per cache line.

#### 24.3.2 Page arithmetic

Assuming 4 KiB pages:

```text
pages_for_field = ceil(bytes(field) / 4096)
```

If a traversal touches more pages than the TLB can cover cheaply, the language should treat that as a layout or tiling question, not just a cache-grade question.

#### 24.3.3 Prefetch reasoning

Software prefetch is worth considering only when:

- stride is regular,
- latency to data exceeds useful work in one or two iterations,
- and hardware prefetch is not already saturating.

The language should use grade and target-profile facts to decide when to emit `llvm.prefetch`, but it must keep this as an optimization hint, not a semantic requirement.

### 24.4 Cursor and `PathLift` become pointer arithmetic

#### 24.4.1 From theory to address calculation

`Cursor<S>` plus a normalized field path imply a direct address calculation:

```text
addr(field, id) = base(field) + id * stride(field)
```

`PathLift` exists precisely to preserve that relationship when optics are composed. A nested optic that focuses on `transform.position.x` must still eventually lower to a legal address expression or a small chain of register-resident projections plus one final store.

#### 24.4.2 Why this matters

If composed optics lost their path meaning, the backend would have to recover alias sets by heuristic field-sensitive analysis. `PathLift` prevents that by carrying path meaning explicitly.

### 24.5 Region sets, TBAA, and store reordering

#### 24.5.1 Static rule

If two `RegionSet`s are proven disjoint in the conservative region language, the backend may materialize that proof into distinct TBAA nodes.

#### 24.5.2 Machine consequence

That unlocks:

- better load/store scheduling,
- more reliable auto-vectorization,
- fewer false alias dependencies,
- improved LICM and GVN opportunities.

#### 24.5.3 Caution

TBAA is only as good as the region summary. Under-approximation is catastrophic. The language therefore chooses conservative region normalization in the prelude.

### 24.6 Staging cost model: specialization versus code size

Staging is not free. It trades runtime selection cost for compile-time work and code-size growth.

A simple first-order specialization profitability model is:

```text
profit(stage) = executions * (dynamic_overhead_removed + optimization_gain)
              - compile_time_cost
              - code_size_penalty
```

Where the language has enough information, `CompileTimeGrade` should track at least the compile-time work. The runtime payoff still needs empirical benchmarking.

#### 24.6.1 Practical staging rules

- stage structure, not data;
- stage archetype layouts, query plans, routing graphs, and fixed protocol stacks;
- do not stage rarely executed code unless it removes a very large dynamic cost;
- cache specialization products by structural hash.

### 24.7 Coinduction, rings, queue depth, and backpressure

#### 24.7.1 Ring arithmetic

For a ring of capacity `Q` and average service time `S`, throughput pressure roughly appears when arrival rate `λ` approaches `Q / S` over the window the ring can absorb. The language does not pretend to prove queueing theory results, but it should at least make queue capacity and liveness visible enough that the backend/runtime can choose sane defaults.

#### 24.7.2 Backpressure rule

A coinductive pipeline that reads from a host ring and writes to another host queue must carry a backpressure policy. If the sink queue is bounded, the pipeline's liveness and latency grades must reflect whether it:

- blocks,
- drops,
- buffers,
- or applies feedback upstream.

Without that, the semantics are incomplete.

### 24.8 Multicore partitioning and false sharing formulas

For `T` threads and cache line size `CL`:

```text
false_sharing_risk if stride(write_target) * chunk_granularity < CL and neighboring threads write adjacent regions
```

A rough safe chunk rule is:

```text
chunk_bytes_per_thread >= 2 * CL
```

for write-heavy kernels, so that each thread is likely to own multiple lines without bouncing a single line back and forth.

#### 24.8.1 Work partitioning principles

- partition by contiguous ranges for SoA traversals,
- partition by NUMA node before by core when remote-memory penalties are large,
- prefer static chunking for uniform kernels,
- use work stealing only when load imbalance dominates locality loss.

### 24.9 Deterministic replay cost model

Replay can be achieved by some combination of:

- full snapshots,
- deltas / write logs,
- recorded external inputs,
- deterministic clock and RNG sources.

The cost trade-off is straightforward:

- **Full snapshots:** high storage cost, very fast replay, best for sparse checkpoints and debugging jumps.
- **Deltas only:** low-to-medium storage cost, slower replay, best for long-running recordings.
- **Inputs only:** low storage cost, replay speed depends on re-execution cost, best when the system is highly deterministic and cheap to recompute.
- **Hybrid:** medium storage cost and medium-to-high replay speed, usually the most practical overall choice.

The language should keep replay as an explicit structural feature rather than an afterthought because its `Runtime` model already exposes the right boundaries.

### 24.10 Quantitative cheat sheet for implementers

- **Vector lanes:** `floor(vector_width / element_size)`.
- **Cache lines for `N` elements:** `ceil(N * stride / 64)`.
- **Pages for a field:** `ceil(total_bytes / 4096)`.
- **Contiguous SoA address:** `base + id * stride`.
- **Branch success hint:** choose `likely` only when success frequency is stable and high.
- **False-sharing warning:** neighboring writers touch the same 64-byte line.
- **Profitable staging:** runtime savings exceed compile-time work and code-size cost over expected executions.
- **Mask versus branch:** prefer masks when the branch is hard to predict and the body is simple enough.

