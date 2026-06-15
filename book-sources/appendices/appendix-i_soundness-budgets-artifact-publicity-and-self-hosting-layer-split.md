## Appendix I — Soundness Budgets, Artifact Publicity, and Self-Hosting Layer Split

### I.1 Why this appendix exists

The main text argues that a mature systems language needs explicit contracts not only for semantics, but also for escape hatches, artifact classes, and compiler/toolchain structure. This appendix gathers those policies into a compact working reference so they can later be turned into checklists, schemas, and audit tooling without changing their meaning. Chapter 28 is the first such standing checklist chapter; this appendix remains the compact ledger it depends on.

It is the audit-facing companion to the closure rule in §27.17: the point of these ledgers and publicity classes is to keep boundary support explicit and to stop it from silently leaking into the core language.

### I.2 Soundness-budget ledger template

A soundness budget is the language's published ledger of where proofs stop being automatic. The point is not to hide unsoundness behind a respectable name; it is to keep every escape hatch named, localized, and reviewable.

| Escape hatch or boundary | What proof is suspended | Why it is allowed | Required local contract | Expected diagnostics | Audit obligation |
|---|---|---|---|---|---|
| `unsafe optic` | local alias/completeness proof | raw hardware, specialized foreign semantics | `BoundaryContract` + explicit safety clauses | `UNS-*` | local review plus tests |
| `extern` item | ABI/layout/unwind proof | legacy interop | ABI, layout, unwind, callback, and capability facts | `FFI-*` | binding owner |
| raw pointer / address-space cast | pointer provenance and region interpretation | low-level memory manipulation | address-space and ownership contract | `PTR-*` or `FFI-*` | subsystem owner |
| volatile/MMIO access | ordinary reordering and caching assumptions | devices and control registers | volatility, privilege, and mapped-range contract | `MMIO-*` | device owner |
| inline assembly / target intrinsic | optimizer visibility of effects | privileged or target-specific instructions | clobber, fence, privilege, and target-profile contract | `ASM-*` | platform owner |
| staged host access | hermetic compile-time guarantee | controlled build integration | declared capability + determinism class | `STG-*` | build owner |
| gradual grade `?` | static precision of one grade dimension | adoption path | runtime observation wrapper + repair workflow | `GRD-*` | local module owner |
| dynamic plugin load | closed-world interface assumptions | extensibility | signed module/interface contract + loader policy | `PLG-*` | runtime/tooling owner |
| callback entry from foreign runtime | one-way call-stack model | GUI/audio/browser/tool integrations | callback, reentrancy, thread-affinity, and unwind contract | `CBK-*` | boundary owner |

The purpose of the ledger is not to make unsafe code disappear. It is to keep every unsound or partially-sound move named, localized, and auditable.

### I.3 Artifact publicity classes

Experimental mathematics artifacts should default to the narrowest publicity class that still makes evaluation possible. A useful default is:

- graph-native witness records and benchmark summaries are **toolchain-stable** while the experiment line is active,
- large proof traces, generated kernels, and search artefacts remain **internal** or sidecar,
- nothing under `std.experimental.*` becomes **public-stable** until it is promoted through the feature-admission checklist and receives an explicit schema/version promise.


The toolchain should classify emitted artifacts before the ecosystem does it accidentally.

| Class | Examples | Stability promise | Typical consumers |
|---|---|---|---|
| public stable | module interfaces, generated lock snapshots, registry publication manifests, replay/benchmark capsules exchanged between teams | versioned and migratable across editions | packages, registries, CI, remote caches |
| toolchain-stable | diagnostics JSON schema, graph-protocol revisions, selected build-plan and debug-info sidecars | versioned with toolchain family and edition, with migration support | editors, language servers, IDEs, analysis tools, coding agents |
| internal | HIR caches, CGIR caches, solver traces, invalidation journals, provisional optimization notes | no compatibility promise | compiler itself |

A good rule is simple: if third-party tools are expected to consume an artifact directly, it must not remain unnamed private compiler trivia.


### I.3.1 Agent memory and failed-patch records are graph-native but advisory

Not every graph-native artifact is part of the language's semantic contract in the same way. Durable agent memory is a good example. Decision records, task records, validated repair history, and advisory failed-patch records are worth storing in or beside the project graph because later humans and agents benefit from querying them. But they should normally be treated as **toolchain-stable, repo-local advisory artifacts**, not as package-public interface promises.

A useful publicity split is:

- accepted architectural decisions and benchmark explanations may be toolchain-stable and queryable;
- task records and failed-patch records are repo-local and advisory;
- large diffs, transcripts, and speculative notes remain sidecar or private memory, not public graph truth.

This keeps the graph useful without promoting every maintenance artifact into a cross-package compatibility promise.

### I.4 Self-hosting layer split

A self-hosted compiler should be structured in three rings rather than pretending that everything is either standard library or compiler monolith.

| Layer | Typical contents | Should ordinary user code depend on it? | Why |
|---|---|---|---|
| core standard library | collections, text/bytes primitives, paths, numeric utilities, general optic combinators, target-neutral primitives | yes | broadly useful, semantically stable, not specific to compiler revisions |
| first-party toolchain support libraries | spans and file maps, interning, stable hashing, index arenas, graph-store primitives, module-interface codecs, diagnostics schema/renderers, target-profile descriptions, object/debug-info emitters, package resolution, graph protocol | sometimes, by tool authors and advanced users | reusable across compiler, package tool, language server, alternative front ends, and analysis tooling |
| compiler-private crates | grammar tables, HIR/CGIR schemas, type rules, grade solver, summary builder, alias checker, optimizer legality, backend legality, edition migrators, invalidation policy | no | these embody the language's semantic authority and must be free to evolve |

A useful rule of thumb is:

> a large share of the **infrastructure by code volume** may live in reusable first-party libraries, while the **semantic authority of the compiler** remains compiler-specific.

### I.5 Promotion rule

A component should move outward only when all of the following are true.

1. At least two independent consumers need it.
2. Its contract can be described without referring to one specific compiler pass.
3. Stabilizing it costs less than the ecosystem confusion caused by keeping it private.
4. Its evolution story is compatible with editions and artifact versioning.

This rule prevents the standard library from absorbing compiler internals while still letting the toolchain grow a reusable substrate.

### I.6 Formal verification obligations

Several of the book's core claims are asserted but not machine-checked. This section records them explicitly as conjectures with stated proof obligations, so they cannot harden into assumed facts by omission.

| Claim | Current status | Proof target | Priority |
|---|---|---|---|
| Map fusion is semantically sound | Correctness argument in Ch. 10; golden tests | Small-step operational semantics for grade calculus; Lean 4 or Agda | Before M7 full-language traversal fusion |
| Compose fusion is semantically sound | Same | Same | Same |
| Product flattening is semantically sound | Same | Same | Same |
| Grade semiring distributive law holds for all 8 dimensions | Asserted in Ch. 6 and Ch. 15 | Algebraic proof per dimension, especially session and security | Before M7 grade algebra ships |
| Region language over-approximation is conservative | Stated as design intent | Proof that no true alias is reported as non-aliasing | Before M7 traversal alias checking |
| Session type composition is decidable in the narrow scope | Assumed from binary session theory | Mechanical check of the narrow grammar under structural duality | Before M7 session grade ships |
| Fractional ownership semiring is well-formed | Cited from Marshall–Orchard 2024 | Verify the specific composition and partition laws used in Optic match the adopted early carrier model | Before M7 implementation of explicit fractional syntax ships |

#### I.6.1 Why formal verification belongs in the soundness budget

An unsound fusion rewrite is more dangerous than a missing feature because it produces silently wrong programs that pass the type checker. The language's central claim — that fusion is proof-directed rather than heuristic — becomes false if a rewrite is unsound.

The practical approach is not to block all development on formal proofs. It is to treat the proof obligations as explicit tracked debts in the same ledger as the soundness-budget escape hatches. A fusion rule that enters the codebase before its soundness argument is written down is analogous to an `unsafe optic` without a `BoundaryContract`: it is permitted, but it must be flagged and owned.

**Recommended practice:** Each new fusion rule added at M7 and beyond should be accompanied by either (a) a formal proof in the target calculus, or (b) an explicit entry in this table labeled `UNVERIFIED` with the conjectured soundness argument and a tracking issue. The tracking issue remains open until the proof is completed.

#### I.6.2 Target proof framework

The most practical target is Lean 4, for three reasons: it has a strong category-theory library (Mathlib), it is increasingly used for compiler verification, and its proof terms can be extracted and type-checked independently. An alternative is Agda, which has stronger universe polymorphism for the categorical arguments. Either is acceptable; the constraint is that proofs must be machine-checkable by a toolchain that does not require the Optic compiler itself.

The scope for an initial verification effort is deliberately narrow: a small-step operational semantics for the grade calculus (not the full language), covering only the three v0 fusion rules. That is enough to validate the book's core claim — that the v0 optimizer is proof-directed — without requiring a full formal semantics of the surface language.

