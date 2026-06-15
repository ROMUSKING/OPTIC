## 26. Backend Validation, Portability, and Performance Discipline

Once the language grows beyond the Rust path into a native backend, correctness requires more than green tests. It requires translation validation, soundness envelopes for alias metadata and vectorization, disciplined target profiling, and benchmark procedures that can survive long-lived compiler evolution.

### 26.1 Translation validation: Rust path versus LLVM path

The LLVM backend should be treated as correct only when it repeatedly matches the semantics of the Rust path on the regression corpus.

#### 26.1.1 Required validation layers

1. diagnostics agreement on accepted/rejected examples,
2. HIR and summary agreement,
3. CGIR agreement before backend lowering,
4. output behavior agreement on e2e tests,
5. benchmark comparison against handwritten and Rust-path baselines.

#### 26.1.2 Practical rule

When the LLVM backend disagrees with the Rust path, the default assumption should be that the LLVM path is wrong until proven otherwise.

### 26.2 Soundness envelopes for alias metadata and vectorization

The backend must document which proof obligations justify each optimization class.

- **TBAA-based alias disambiguation:** requires conservative `RegionSet` disjointness.
- **SIMD traversal lowering:** requires traversal legality plus lane independence.
- **Loop fusion:** requires non-escaping intermediates plus grade legality.
- **Parallel lowering:** requires partition independence plus false-sharing checks.
- **Event-loop lowering:** requires a coinductive node plus explicit host queue semantics.

If any required artifact is missing, the backend must use the safe fallback path.

### 26.3 Target profiles and portability

The full language should compile against explicit target profiles, not implicit assumptions about the local machine.

#### 26.3.1 Minimum target-profile contents

- architecture (`x86_64`, `aarch64`, `riscv64`, later `wasm32` if desired),
- cache line size and page size assumptions,
- SIMD capability set,
- atomics and memory-order support,
- host I/O facilities (`io_uring`, `kqueue`, `epoll`, IOCP, none),
- `no_std` capability,
- endianness and alignment rules.

The grade system should *not* pretend these are portable by magic. It should make the relevant target assumptions explicit.

### 26.4 Hosted versus `no_std` kernel-class targets

The backend should support at least three target tiers:

- **Hosted-debug:** normal user-space build with maximum diagnostics; easiest validation path.
- **Hosted-release:** optimized native build; primary benchmark and service path.
- **Kernel / `no_std`:** no allocator or hosted runtime assumed; explicit host adapters required.

A language feature that only works in hosted builds must say so. Silent dependence on hosted services is unacceptable for the kernel-class roadmap.

### 26.5 Benchmark suites by domain

The performance discipline should track both universal microbenchmarks and domain-specific suites.

#### 26.5.1 Universal suites

- single-field traversal,
- product traversal,
- filtered traversal,
- staged specialization overhead/payoff,
- ring/event-loop throughput,
- alias-safe parallel traversal.

#### 26.5.2 Domain suites

- kernel: allocator, page walk, scheduler, RX/TX ring;
- browser: style/layout/paint;
- database: scan/probe/commit;
- game: ECS update/render/audio;
- compiler: parse/type/opt/codegen;
- service: request pipeline and replay throughput.

### 26.6 Agent-oriented backend diagnostics

The backend should extend the diagnostic discipline instead of abandoning it.

#### 26.6.1 Suggested backend families

- **`LLV-1xx`** — IR emission shape or legality failure.
- **`LLV-2xx`** — metadata or target-profile mismatch.
- **`VEC-3xx`** — vectorization blocked or unsafe.
- **`PAR-6xx`** — parallel lowering, false-sharing, or chunking problem.
- **`PER-7xx`** — benchmark regression or specialization-profitability concern.

Each backend diagnostic should include:

- the blocked optimization,
- the missing proof artifact,
- the safe fallback that was chosen,
- the smallest next inspection command.

### 26.7 Performance regression workflow

A full-language compiler that makes strong performance claims must treat performance regressions as first-class failures.

#### 26.7.1 Workflow

1. record baseline per benchmark and target profile,
2. diff HIR/summary/CGIR when a benchmark regresses,
3. identify whether the regression came from legality loss, missed rewrite, target-profile change, or backend drift,
4. add a regression test that guards the discovered cause.

The purpose of the workflow is not merely to recover performance once. It is to turn each loss into repository knowledge.

### 26.8 Mixed-domain collision suites and experimental-lane lowering checks

Backend validation should include a small set of stress suites where unlike domains meet and where experimental-domain kernels must still lower through ordinary summaries. Examples include:

- a frame-budgeted render/update loop fed by a coinductive network ingress queue,
- a transaction-graded database wrapper over a legacy C storage engine,
- an experimental geometric or solver kernel embedded inside an otherwise ordinary traversal.

The backend passes these suites only when it can show the same properties it shows elsewhere in the book: ordinary provenance retention, ordinary `RegionSet`-based legality, ordinary grade accounting, and no hidden optimizer side rules for the experimental path.

### 26.9 Release gates for full-language milestones

A full-language milestone should ship only when all of the following hold:

- semantic and diagnostic agreement with the Rust reference path where applicable,
- stable benchmark deltas on the gated corpus,
- no unexplained legality downgrades on hot-path examples,
- target-profile assumptions documented and tested,
- provenance preserved through optimization and backend lowering,
- replay/tooling hooks still work after the backend changes.

These gates are still only the backend-facing slice of maturity. Once a language becomes usable by other people, the harder questions shift toward compatibility policy, build-graph identity, ecosystem tooling, binary distribution, and runtime coherence. The next chapter names those pressures directly and ties them back to the current architecture.

The last chapter of this part steps back from backend validation and asks the broader maturity question: what still has to be fixed, specified, or governed before the language can survive real ecosystems rather than only compelling examples?

