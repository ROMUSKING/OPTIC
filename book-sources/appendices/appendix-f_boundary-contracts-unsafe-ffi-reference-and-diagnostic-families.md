## Appendix F — Boundary Contracts, Unsafe/FFI Reference, and Diagnostic Families

This appendix gives the compact reference form of the book's foreign-boundary story. The main chapters explain why `unsafe`, FFI, address spaces, callbacks, and privileged operations stay inside the same semantic model; this appendix keeps the field layout, surface forms, and diagnostic families easy to recover during implementation.

It is the compact reference for the **boundary** lane of the closure rule in §27.17: these mechanisms are fully supported, but they do not become a second semantic center of the language.

### F.1 The minimal-extension rule

The language should become fully general by extending the existing model, not by creating a second one.

- **Keep** `Runtime`, `HostContext`, `OpticSummary`, `RegionSet`, grades, CGIR, and staged execution.
- **Add** boundary contracts to the existing summaries.
- **Prefer** safe wrappers and boundary optics over raw foreign items in ordinary code.
- **Treat** `unsafe` as a trusted boundary declaration, not as "turn off the language".

### F.2 Preferred representation

```rust
struct BoundaryContract {
    kind: BoundaryKind,
    abi: Option<AbiKind>,
    callconv: Option<CallConv>,
    unwind: UnwindPolicy,
    may_callback: bool,
    reentrant: Reentrancy,
    thread_affinity: ThreadAffinity,
    context: ExecContext,
    address_space: AddressSpace,
    volatility: Volatility,
    atomicity: Atomicity,
    privilege: PrivilegeLevel,
    pinning: PinRequirement,
    allocator: AllocatorContract,
    layout: LayoutContract,
    stageability: Stageability,
    safety_clauses: Vec<SafetyClause>,
}

struct OpticSummary {
    // existing summary fields
    boundary: Option<BoundaryContract>,
}
```

### F.3 What should remain a grade

Use grades when the property composes like a budget or quantity.

- cache footprint,
- latency,
- bandwidth,
- blocking budget,
- liveness,
- ownership strength,
- NUMA penalty,
- optional atomic-cost categories.

### F.4 What should remain a qualifier or contract

Use qualifiers/contracts when the property describes how a boundary must be crossed rather than how a quantity composes.

- ABI and symbol naming,
- calling convention,
- unwind policy,
- may-callback / reentrancy,
- thread affinity,
- privilege level,
- address space,
- volatility,
- layout guarantees,
- allocator ownership and pinning.

### F.5 Recommended surface forms

| Surface form | Role | Expected use |
|---|---|---|
| `extern fn` | raw ABI declaration | lowest-level import/export fact |
| `unsafe optic` | graph-facing boundary wrapper | preferred unit for unsafe or foreign interaction |
| `safety { ... }` clauses | explicit preconditions and guarantees | localize trusted assumptions |
| typed address-space wrappers (`Mmio<T>`, `Dma<T>`, `ManagedHandle<T>`) | distinguish memory domains | prevent ordinary optimizations from crossing special boundaries unsafely |

### F.6 Recommended CGIR reuse

Do not introduce a separate foreign IR unless the existing leaf model fails completely. Prefer:

```text
OpticLeaf + BoundaryContract + LeafImplementationKind
```

Possible implementation kinds:

- `Local`
- `Extern`
- `Intrinsic`
- `Volatile`
- `Asm`
- `ManagedBridge`

### F.7 Required diagnostic families

| Prefix | Meaning |
|---|---|
| `FFI-*` | ABI, symbol, or layout mismatch |
| `UNS-*` | unsafe precondition unsatisfied |
| `MMIO-*` | volatile/address-space misuse |
| `DMA-*` | pinning or coherency violation |
| `ATM-*` | atomic-ordering or fence misuse |
| `UNW-*` | unwind/exception boundary violation |
| `CBK-*` | callback, reentrancy, or thread-affinity violation |
| `ASM-*` | inline assembly constraint or clobber violation |
| `MOD-*` | module, plugin, or dynamic-loading contract violation |

Each diagnostic should expose:

- the violated boundary field,
- the exact source span and declaration site,
- the relevant region/grade context,
- the minimal safe repair,
- and the next command or artifact to inspect.

### F.8 Full-generality checklist

A language revision is not yet fully general until it has a coherent answer for all of the following under the same model:

1. raw pointers and address spaces,
2. atomics, fences, volatility, and provenance,
3. FFI ABI and layout control,
4. unwinding and foreign exceptions,
5. callbacks, reentrancy, and thread affinity,
6. allocators, pinning, and foreign ownership,
7. DMA/MMIO and privileged instructions,
8. separate compilation, plugins, and stable ABIs,
9. managed-runtime interop,
10. determinism/replay classification at boundaries,
11. capability gating and auditability,
12. structured diagnostics and tooling support,
13. edition, migration, and deprecation policy,
14. native package declarations, generated lock snapshots, and reproducible-environment policy,
15. module-interface artifacts and cache invalidation rules,
16. debugger/profiler/crash provenance through fusion and staging,
17. conformance suites and translation validation across implementations,
18. supply-chain and generated-binding provenance.

The book's central claim is that all eighteen fit the same architecture once boundary contracts are made explicit and the surrounding policy is made first-class.

