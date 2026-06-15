## Appendix H — Compiler Graph Store, Projection Filesystem, and Tool Protocol Reference

This appendix keeps the graph-resident compiler and tooling architecture in reference form. It does not introduce a second system. It restates, in compact operational terms, how the book's existing ideas—explicit costates, projections, `BuildRuntime`, `Project`, and structured host boundaries—fit together in storage, filesystem views, and direct tool protocols.

It should also be read as the local-storage view of the larger ecosystem-graph formulation from §4.6.1: the mapped project graph is the checked workspace slice, not the entire universe of packages and artifacts.

### H.1 Core idea in one sentence

The graph-store architecture only remains coherent if this sentence stays true all the way through implementation, tooling, and self-hosting.

```text
CompilerGraph is the authoritative build-time costate; files, diagnostics, interfaces, and generated outputs are projections; tools talk to the graph directly through a revisioned protocol.
```

### H.2 Comparative shorthand: semantic image, semantic identity, projections, and derivations

Chapter 4 does the full narrative work. This appendix keeps the short mnemonic close to the operational details.

- **Smalltalk** contributes the lesson that tools should live over the authoritative semantic state, not over stale reconstructions.
- **Unison** contributes the lesson that semantic identity should be stronger than filename identity where meaning is stable.
- **Projectional systems such as MPS** contribute the lesson that one semantic structure can support many coherent views.
- **Graph-first build systems such as Nix** contribute the lesson that build closure and derived artifacts should be explicit and reproducible.

The Optic design keeps all four lessons but refuses to let any single one dominate. The project graph is a semantic image, not a mutable heap image; it uses content addressing where meaning is stable and revisioned ids where operation is hot; it treats text as a first-class projection rather than an obsolete compatibility layer; and it keeps native build declarations inside the language rather than inventing a second authored DSL.

### H.3 Minimal persistent layout

| Region | Responsibility | Notes |
|---|---|---|
| superblock | schema version, workspace identity, root offsets, current revision | fixed offset, one page |
| journal | append-only transaction log and commit markers | source of crash recovery between checkpoints |
| text arena | authored UTF-8 chunks and stable rope pieces | writable through patch transactions |
| interner arena | symbols, paths, canonical strings | read-heavy, deduplicated |
| syntax/HIR/CGIR arenas | packed nodes and edge payloads | generation-counted ids recommended |
| summary/diagnostic tables | hot fixed-size facts | dense, cache-friendly |
| projection table | maps projection ids to graph regions and policies | some writable, most read-only |
| watch table | subscriptions and invalidation cursors | per tool session |
| artifact index | references to sidecar CAS blobs and materializations | large blobs stay out of the hot graph |
| experimental arena | direct internal lanes first (`sep`, `memory`), then proof witnesses, geometric kernels, ultrametric indexes, sheaf/topos records, and stability analyses | graph-native, opt-in, and explicitly provisional |
| agent memory arena | durable decision records, task state, advisory failed-patch records | small, typed, queryable maintenance knowledge stays near the graph; large diffs and transcripts remain sidecar |

### H.4 Projection classes

| Projection class | Typical payload | Writable? | Reinsertion rule |
|---|---|---|---|
| authored text | UTF-8 source module | yes | patch text region and invalidate dependent summaries |
| structured syntax | AST / HIR pretty or serialized view | usually no | derived |
| semantic graph | CGIR / summary / provenance view | no | derived |
| diagnostics | current structured diagnostic stream | no | derived |
| build roots | package / workspace / target / build-plan values | selectively | structured edit over build-root region |
| generated artifact | interface summary, emitted source, object reference | no | materialize or regenerate |

### H.5 Filesystem projection guidelines

Projection filesystems are valuable adapters, but they are not the semantic center. Use them for:

- editor and shell compatibility,
- human inspection,
- gradual adoption,
- and simple read-mostly tooling.

Do not require them for:

- primary incremental compilation,
- semantic refactoring,
- graph queries,
- batched edits,
- or AI-agent repair loops.

### H.5.1 LSP and ordinary editor adapters are first-party compatibility surfaces

The direct graph protocol is semantically primary, but ordinary editor support is not optional. The toolchain should ship a first-party LSP adapter that translates between graph-native revisions and the file-oriented expectations of existing editors, code review tools, and patch workflows. Projection filesystems remain useful compatibility adapters, but the language should not require a bespoke graph-native editor before ordinary development becomes practical.

### H.6 Direct protocol request families

| Family | Representative operations |
|---|---|
| session | open workspace, negotiate schema/protocol version, declare capabilities |
| snapshot | pin graph revision, open read snapshot, release snapshot |
| patch | apply text patch, replace build-root value, submit batched edits |
| init/template | start init wizard, submit structured intent answers, preview scaffold diff, commit generated project roots |
| query | fetch diagnostics, summaries, HIR, CGIR, provenance, target profile |
| query-experimental | fetch direct internal witnesses first (separation/resource and weak-memory), then proof witnesses, geometry kernels, ultrametric indexes, sheaf/topos records, and stability analyses |
| query-agent-memory | fetch decision records, task records, and advisory failed-patch records |
| watch | subscribe to invalidation stream, changed projections, or diagnostics deltas |
| stage/build | evaluate package roots, build plans, stageable subgraphs, artifact plans |
| materialize | export projection to ordinary files, emit generated artifact, write reproducible snapshot |
| explain | explain failed fusion, failed staging, failed grade bound, failed boundary contract, or why a similar patch failed earlier |

The ordering in `query-experimental` is normative rather than cosmetic. Clients should treat the direct internal lanes as the first place to look for answers to memory, provenance, and reorder-legality questions, then try the simpler sidecar outputs attached to those nodes, and only then fall through to richer proof or categorical tracks.

### H.7 New diagnostic families worth reserving

| Prefix | Meaning |
|---|---|
| `GRF` | graph-file format, schema, or recovery issues |
| `PRJ` | projection read/write policy violations |
| `IPC` | direct protocol version, capability, or revision mismatch |
| `VFS` | projection filesystem adapter failure or degraded mode |
| `INI` | init wizard, intent validation, or template-generation failure |
| `XPR` | experimental-lane schema, feature-line, or promotion-policy violations |

### H.8 Staged-build consequence

Once the compiler graph is authoritative, native package declarations and build plans do not need a second handwritten config language. The authored build roots live in graph-backed source modules, and lock snapshots, module interfaces, build plans, and generated manifests become graph projections or sidecar materializations.

This is the cleanest way to keep build, compilation, tooling, and self-hosting under one model. A graph-first `optic init` naturally belongs in the same family: it is simply the earliest project transaction over those build-root regions.


### H.9 Repository agent systems are ordinary graph clients

A repository-local agent operating system should be treated as one more first-party client of the graph protocol rather than as a parallel toolchain. That means task packets, context indexes, shared-memory updates, and failed-patch records should reuse the same revision, capability, and materialization discipline as other graph consumers. Appendix K records the file-level form used in the split book package.
