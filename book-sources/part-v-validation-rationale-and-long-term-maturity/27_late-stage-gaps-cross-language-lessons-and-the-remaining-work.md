## 27. Late-Stage Gaps, Cross-Language Lessons, and the Remaining Work

### 27.1 Why languages often hit their hardest problems after the core seems to work

Early language work is usually dominated by syntax, typing, lowering, and a first convincing example program. Those are real problems, but they are not usually the problems that decide whether a language becomes operationally complete. The later-stage pressures are different.

A language becomes hard in a new way when other people depend on it. At that point the questions are no longer only:

- can the compiler lower this feature correctly,
- can the optimizer justify this rewrite,
- can the runtime make this program fast.

They become:

- what exactly is stable,
- what is allowed to change,
- how separate compilation is represented,
- what a package or binary artifact means,
- how foreign boundaries are audited,
- how a debugger or profiler continues to identify the source program after heavy optimization,
- and how the ecosystem survives a change in runtime or concurrency semantics.

This is where many languages discover that their core design was only half a language. The missing half is not "ergonomics" in the narrow sense. It is the operational contract between the language, the toolchain, other compilers, package managers, foreign code, and long-lived codebases.

The Optic design is already better positioned than most experimental languages because it begins from explicit runtime structure, explicit summaries, explicit grades, and explicit graph-level optimization. But those strengths only help if the language also grows explicit policy for the layers that usually become folklore in other ecosystems.

This chapter therefore does two things.

First, it identifies the main gaps that still need dedicated design work before the language can credibly claim to be fully general.

Second, it uses other ecosystems as cautionary evidence, not as targets for imitation. The point is not to repeat the mistakes of C++, Rust, Java, Python, or TypeScript in a new syntax. The point is to learn which problems refuse to stay postponed.

Selected external reference points are collected in Appendix G so that the comparisons in this chapter stay tied to primary documentation rather than community mythology.

### 27.2 A map of the remaining gaps

From a distance, the remaining work looks like a long backlog. Up close, it is more structured than that. Almost every unresolved item falls into one of four recurring pressures.

**First, the language needs a compatibility story.** A prototype can survive on good will and a fast-moving compiler. A real ecosystem cannot. Module interfaces, editions, target profiles, lock snapshots, and artifact schema versions all belong here. This is the pressure C++, Go, Swift, and TypeScript make impossible to ignore.

**Second, the language needs a complete host-boundary and runtime story.** Kernels, browsers, engines, services, and plugins all eventually force the same questions: what is the memory model, what can unwind, what may callback, which runtime family is assumed, and which unsafe facts are merely local obligations rather than silent global assumptions. This is where Python's runtime shifts, Rust's FFI discipline, and Java's foreign-memory work all become directly relevant.

**Third, the language needs a durable tooling and observability contract.** Provenance through fusion, stable debug identities, profiler keys, crash capsules, and machine-readable diagnostics are not decorative extras. They are what make aggressive optimization socially affordable.

**Fourth, the language needs an ecosystem center of gravity.** A strong standard library split, a canonical package workflow, generated lock artifacts, first-party binding generation, and clear publicity classes for artifacts all belong here. Without that center, the ecosystem learns its own rules by rumor.

The encouraging point is that none of these pressures demand a second semantic core. They all reuse machinery the book already introduced earlier: explicit `Runtime` and `Project` roots, `OpticSummary`, `RegionSet`, `BoundaryContract`, target profiles, provenance-carrying CGIR, staged artifacts, and structured diagnostics. The remaining work is therefore not a search for a new theory. It is the work of extending the existing theory into the places where languages usually become operationally fragile.

#### 27.2.1 Three buckets clarify sequencing

These pressures should not all be attacked at once. The book's own architecture suggests a conservative order.

**Maturity blockers** come first: module-interface artifacts, artifact versioning, edition and migration policy, public-versus-internal artifact classes, and the standard-library/package-workflow center of gravity. Without these, the ecosystem cannot stabilize even if the compiler already works well.

**Systems-completeness blockers** come next: the formal memory model, the unsafe/FFI soundness ledger, profiler and debug provenance, crash capsules, runtime-family declarations, and boundary contracts rich enough for real host interaction. Without these, the language cannot honestly claim kernel-, browser-, database-, or engine-class generality.

**Research-expansion items** can remain later: general resumptions, distributed and GPU costates, stronger symbolic solving, deeper bootstrap compression, and heavier formal verification. These matter for power and elegance, but they do not have to precede ecosystem stability.

That ordering is intentionally conservative. A language can live for a while without captured continuations or distributed costates. It does not live well for long without interface artifacts, migration policy, and an explicit unsafe audit story.

### 27.3 Compatibility has layers, not one promise

One of the most common late-stage mistakes in language development is to say that a language is either "stable" or "unstable" without naming what is actually being stabilized. In practice compatibility has several layers, and they must be separated.

#### 27.3.1 Source compatibility

Source compatibility answers: will a program that compiled yesterday still parse, type-check, and mean the same thing today?

This is where operator precedence, keyword growth, standard-library changes, edition boundaries, and diagnostic-guided migrations live. A language that waits too long to define editions usually ends up with either permanent syntax regrets or ecosystem-wide breakage.

For Optic, source compatibility should be handled with:

- explicit editions,
- edition-aware parsing and reserved-keyword policy,
- compiler-provided migration commands,
- and stability tiers for library APIs and language features.

The current parser architecture, structured diagnostics, and fix ranking already make this plausible. The missing piece is policy, not mechanism.

#### 27.3.2 Package compatibility

Package compatibility answers: can two independently versioned libraries be built, resolved, and installed together in a reproducible way?

This is where generated lock snapshots, semver policy, target profiles, capability requirements, and build reproducibility matter. A language can delay this during the prototype period, but a self-hosting compiler, a browser engine, or a kernel module ecosystem cannot.

#### 27.3.3 ABI compatibility

ABI compatibility answers: can separately compiled binary artifacts call each other without rebuilding from source?

C++ is the canonical warning here. GCC still documents a dual ABI for libstdc++ because old and new implementations of key standard-library types had to coexist for compatibility reasons, and Clang's C++ modules documentation treats consistency requirements and ABI impacts as first-class concerns rather than side notes. This is what happens when source structure, compiled artifacts, and binary interfaces are allowed to drift apart. See Appendix G.2 for the reference points behind this comparison.

Optic should therefore avoid the vague phrase "stable ABI" unless it is immediately qualified. The language needs at least four distinct positions:

1. **stable C-facing boundary ABI** for FFI and plugins,
2. **edition-scoped source stability** for ordinary code,
3. **compiler-internal IR and metadata** that are explicitly unstable unless versioned,
4. **optional native plugin ABI** only after the runtime, memory model, and unwinding policies are frozen enough to deserve it.

The current `BoundaryContract` model already gives the right shape for this. The remaining work is to name which parts are promised, which parts are versioned, and which parts are intentionally unstable.

#### 27.3.4 Runtime compatibility

Runtime compatibility answers: can code built under one scheduler, memory model, or host-boundary policy safely interoperate with code built under another?

This matters more than many language teams expect. If the language eventually supports kernel-class targets, no-std targets, coinductive event loops, plugin loading, managed-runtime bridges, and possibly distributed or GPU targets, then the runtime contract must be versioned just as carefully as the source contract.

A plugin compiled for one memory model or callback policy must not silently load into a host that assumes another.

#### 27.3.5 Semantic compatibility

Semantic compatibility is the hardest layer and the one most likely to be forgotten. It asks whether the language still means the same thing after optimization, staging, fusion, or backend changes.

The book already answers this better than most language plans because provenance, summaries, and translation validation are treated as first-class. The missing piece is to define a stability surface for those artifacts: which provenance identifiers, debug-info mappings, and diagnostic codes are stable enough for tooling and automation to build on.

### 27.4 Five mainstream ecosystems expose five different maturity cliffs

Earlier chapters already used C++, Rust, Java, Python, and TypeScript to justify specific core choices in layout, ownership, boundary contracts, packaging, and diagnostics. At the maturity stage, the useful question is narrower: which late-stage pressure does each ecosystem make impossible to ignore?

C++ shows that modules do not dissolve artifact identity, consistency requirements, or ABI management. That is why Optic needs explicit module interfaces, explicit artifact hashing, and a compatibility policy that distinguishes source stability from binary stability.

Rust shows that layout, ABI choice, and unwind policy are semantic boundary facts rather than implementation footnotes. That is why `BoundaryContract`, layout policy, and unwind matrices must become public-facing parts of the language rather than remaining compiler folklore.

Java shows that foreign access eventually needs a safer, more structured replacement for folklore wrappers. JNI's long shadow is the clearest warning here. That is why foreign memory, binding generation, and callback contracts should stay on the main road of the language, not in an expert annex.

Python shows that packaging and runtime shifts become ecosystem-wide operational events. Reproducible environments, lock artifacts, and concurrency-model changes all spill into build tooling, extension compatibility, and deployment practice. That is why generated lock snapshots, runtime-family declarations, and staged-artifact identity need to arrive early.

TypeScript shows two things at once: a language can openly admit a soundness trade, and a compiler eventually has to own the project graph if it wants to scale. That is why Optic should keep its soundness budget explicit and should treat project-graph identity as part of the language architecture rather than as an implementation convenience.

The point of this comparison set is not to restate earlier cross-language material in full. It is to make one maturity claim precise: once a language has real users, the pressure points stop being optional. Optic's job is not to avoid them. It is to meet them with the same explicitness it already applies to costates, optics, grades, and provenance.

### 27.5 Modules, separate compilation, and artifact identity are still underspecified

The current book is strong on AST, HIR, CGIR, summaries, and generated-code artifacts. It is comparatively thin on the artifact that sits between "one file compiled" and "whole program linked": the module interface artifact.

That gap needs to be closed before the language grows a serious library ecosystem.

#### 27.5.1 What a module artifact should contain

A module artifact should not be only a list of exported names. For this language, it should also carry enough static structure that downstream compilation can continue to work at the language's natural abstraction level.

A plausible artifact should contain:

- exported type signatures,
- exported optic signatures,
- serialized `OpticSummary` data,
- dependency fingerprints,
- edition and target-profile requirements,
- boundary-contract metadata for exported foreign items,
- and enough provenance to make diagnostics deterministic across separate compilation.

This is not "too much metadata". It is exactly the information the compiler already knows and already uses. Refusing to serialize it would only force later recompilation or re-inference.

#### 27.5.2 What the build graph should key on

The current book already uses content-hash ideas for node identity and cache reuse. The same principle should be extended to modules.

A build artifact key should at least include:

- source content hash,
- edition,
- target profile,
- enabled capability set,
- transitive imported interface hashes,
- build-host policy for staged artifacts,
- and optimizer profile when it affects generated interface facts.

That last point matters because staged computation and foreign-boundary summaries can affect what downstream crates are allowed to assume.

#### 27.5.3 Why this belongs under the current model

This does not require inventing a second compiler architecture. It is a direct reuse of the existing summary discipline:

- HIR already identifies the public semantic surface,
- `OpticSummary` already describes the legal read/write and grade behavior of exported optics,
- CGIR hashes already provide stable shape information,
- diagnostics already want stable ids and spans.

The missing work is therefore serialization policy, artifact versioning, and invalidation rules.

#### 27.5.4 Public, toolchain-stable, and internal artifacts must be distinguished

The module-interface story also needs a publicity policy. A mature toolchain emits many serialized artifacts, but not all of them should become ecosystem promises.

The clean split is:

- **public stable artifacts**: module interfaces, generated lock snapshots, registry publication records, and any replay or benchmark capsule formats users exchange directly;
- **toolchain-stable artifacts**: diagnostics schemas, graph-protocol revisions, selected build-plan materializations, and debug/performance sidecars consumed by first-party tools;
- **internal artifacts**: HIR caches, CGIR caches, solver traces, invalidation journals, and provisional optimizer notes.

That distinction feeds directly into invalidation rules. Public artifacts participate in compatibility promises. Toolchain-stable artifacts participate in version negotiation and migration tooling. Internal artifacts may be dropped and regenerated whenever the compiler improves.

### 27.6 Packaging, reproducible environments, and binary distribution need their own chapter because they will not stay small

Packaging is where many technically strong languages discover that their users do not actually consume "the language". They consume a stack of tools: dependency resolution, build planning, target selection, generated bindings, binary artifacts, native dependencies, deployment conventions, and the metadata that makes all of those reproducible.

The important refinement, given the language's compile-time execution model, is that Optic does **not** need a second handwritten manifest or build DSL as the source of truth. The package and workspace description can live in the language itself as native declarations evaluated over `BuildRuntime`. What the ecosystem still needs are **compiler-generated artifacts** that serialize the results of that native description.

That distinction is worth making explicit:

- **authoring surface:** native `package`, `workspace`, and staged `build_plan` declarations in Optic source;
- **generated artifacts:** dependency lock snapshots, module-interface summaries, staged-artifact manifests, registry metadata, and binary-distribution metadata emitted by the toolchain.

This is a better fit for the rest of the architecture because it keeps packaging under the same typed model as the compiler proper. Dependency solving, target-profile selection, native-library discovery, binding generation, and artifact planning become ordinary staged optic graphs. They can be cached, diagnosed, and audited using the same `BuildRuntime`, `TargetProfile`, `BoundaryContract`, provenance, and `CompileTimeGrade` machinery that already exists elsewhere in the book.

#### 27.6.1 What is the canonical unit of publication?

For Optic there are still at least four plausible publication units:

- source packages,
- precompiled module/interface artifacts,
- native libraries or executables,
- staged data artifacts such as generated tables or pre-specialized graph fragments.

The language should support all four, but it should define one canonical **package identity** that describes them together. That identity should be derived from native declarations plus generated artifact hashes, not from a separately maintained manifest language.

#### 27.6.2 What belongs in native package/workspace declarations?

At minimum, the native build surface should be able to express:

- edition,
- package version,
- target-profile constraints,
- capability requirements,
- required runtime family,
- dependency requirements,
- boundary-contract exports,
- and whether staged artifacts are included, derived, or forbidden.

These facts belong in the source language because they influence staging, target checking, FFI legality, cache keys, and code generation. Keeping them outside the language would force the compiler to recover information that could simply be stated once.

#### 27.6.3 What is the lock story if there is no user-authored manifest file?

A language that wants reproducible builds still needs a canonical resolved snapshot. The difference is that Optic's lock state should be **generated**, not hand-authored.

A good Optic lock artifact would record:

- exact package versions,
- exact target profile and capability profile,
- exact native or staged artifact hashes,
- exact foreign-binding generator versions where generated bindings affect ABI,
- and enough build-input hashes to reproduce `BuildRuntime` decisions.

That artifact may still be stored on disk and checked into repositories when teams want that workflow. But it is no longer the semantic source of the build. It is the compiler's canonical serialization of a native staged decision.

#### 27.6.4 Why this still belongs under the current model

The package system should not invent its own notion of capability or target. It should reuse:

- target profiles from the backend chapters,
- boundary contracts from the unsafe/FFI interlude,
- staged artifact ids from the compile-time execution chapter,
- module-interface summaries derived from `OpticSummary`,
- and diagnostic identifiers from the agent-facing tooling chapters.

That reuse is the difference between a coherent language toolchain and an ecosystem that quietly forks its own metadata models.

#### 27.6.5 Native packaging example

```rust
pub let package = Package {
    name: "browser.layout",
    edition: Edition::current(),
    targets: [Target::linux_x86_64_musl(), Target::wasm32_wasi()],
    deps: [
        Dependency::registry("css.selectors").compatible("2.1"),
        Dependency::registry("gfx.raster").exact("5.0.3"),
    ],
    runtime_family: RuntimeFamily::native(),
    capabilities: [BuildCap::read_path("assets/fonts"), BuildCap::env("SDKROOT")],
    experimental: [],
};

pub fn build_plan(rt: &[SharedGrade] BuildRuntime) -> BuildPlan {
    rt.query(
        ResolveDependencies
        >>> BuildModuleInterfaces
        >>> StageSelectorAutomata
        >>> PlanArtifacts
    ).get()
}
```

This example is deliberately ordinary. A package is a value. A build plan is a staged graph. The compiler materializes the generated lock snapshot, module-interface artifacts, and staged-artifact manifests from those declarations instead of asking the user to restate the same information in another language.


#### 27.6.6 The init path should be part of the package and tooling contract

If native `package`, `workspace`, and `build_plan` declarations are the canonical authored surface, then the first-party package tool should make those declarations the default output of project creation rather than leaving new users to write them from scratch or to copy stale boilerplate.

That is why the language should treat `optic init` as more than a convenience command. It is the first ecosystem-scale decision point where the toolchain can help the user state the facts that later chapters keep insisting must be explicit anyway:

- runtime family,
- target profile,
- initial `AppWorld` shape,
- expected host capabilities and foreign boundaries,
- workspace topology,
- and the desired replay/benchmark/tooling posture.

A graph-first, wizard-driven init flow also prevents an especially common maturity failure: the ecosystem improvises several incompatible “normal” project layouts before the language has said what the canonical one is. In other words, a good init tool is part of closure, not just onboarding.

### 27.7 Runtime-model changes are ecosystem changes

The concurrency and runtime chapter already explains how grades, `HostContext`, and `ControlRuntime` provide the language with a coherent way to talk about event loops, parallel products, and liveness. What is still missing is an explicit policy for runtime-model change.

The reason this matters is visible in Python's current free-threading work. Making the GIL optional is not just "a performance feature". It changes ABI expectations, extension requirements, and the practical meaning of thread safety for user code. Python's own documentation and PEP 703 treat those consequences as real migration issues, not incidental fallout. Appendix G.5 provides the concrete Python references.

For Optic, the corresponding rule should be:

> Any change to the memory model, callback model, scheduler semantics, or host-boundary semantics is a compatibility event.

That implies several concrete policies.

- Plugin and dynamic-library metadata must declare the runtime model they were built against.
- The standard library must not ship two subtly incompatible "official" event models without making the distinction explicit in types and native package declarations.
- The memory model for atomics, volatility, address spaces, and callbacks must be frozen before a plugin ABI is promised.
- Tooling should surface runtime-model mismatches as first-class diagnostics rather than opaque linker or loader failures.

The good news is that the current architecture is already explicit enough to carry these facts. The missing work is to elevate them from internal assumptions to public contract.

### 27.8 Soundness budgets and escape hatches must stay explicit

The language cannot remain pleasant if every hard boundary forces the user into theorem-prover mode. It also cannot remain analyzable if convenience features silently bypass the model.

TypeScript is a useful contrast because it explicitly documents its choice to permit some unsound behavior for usability and JavaScript compatibility. That honesty is valuable even if the language's tradeoffs are different from Optic's. Appendix G.6 points to the relevant handbook section.

The corresponding rule for Optic should be:

> Ordinary optic composition remains sound by default. Any escape from that regime must be named, localized, and auditable.

Under the current model that means:

- no silent weakening of alias safety,
- no invisible fallback from checked grades to unchecked ones,
- no hidden optimizer assumptions across foreign boundaries,
- no unchecked control effects smuggled through ordinary lens/traversal syntax,
- no unsound subtyping rule that can be triggered accidentally.

The language already has the right raw materials for explicit escape hatches:

- `unsafe optic`,
- `BoundaryContract`,
- gradual grades,
- explicit stageability judgments,
- explicit address-space types,
- and explicit capability gating.

The missing work is editorial and procedural. The book should eventually include a short "soundness budget" chapter that states which escape hatches exist, why they exist, what proofs they bypass, and what audit obligations they create.

#### 27.8.1 A published soundness-budget ledger should become part of the toolchain contract

The missing soundness-budget chapter should not be only reflective prose. It should be a compact ledger with one row per sanctioned escape hatch.

| Escape hatch | What proof it bypasses | Why it exists | Required local contract | Diagnostic family | Audit owner |
|---|---|---|---|---|---|
| `unsafe optic` | local alias/completeness proof | raw hardware or foreign interaction | `BoundaryContract` + safety clauses | `UNS-*` | subsystem owner |
| `extern` item | language-internal ABI/layout proof | legacy interop | ABI/layout/unwind contract | `FFI-*` | binding owner |
| volatile/MMIO | ordinary reordering assumptions | devices and control registers | address-space + volatility contract | `MMIO-*` | device owner |
| inline assembly | optimizer visibility of effects | privileged or target-specific instructions | clobber/fence/privilege contract | `ASM-*` | platform owner |
| staged host access | hermetic compile-time guarantee | controlled build integration | capability declaration + determinism class | `STG-*` | build owner |
| gradual grade `?` | static precision of one grade dimension | adoption path | observation wrapper + repair flow | `GRD-*` | local module owner |

That ledger should live close to the compiler and tooling, not only in a design appendix. The compiler can then emit diagnostics that quote the exact ledger entry and tell the user which proof obligation is now manual rather than automatic. Appendix I records the shape such a ledger should take.

### 27.9 Tooling, migration, and deprecation need a public policy, not only a good compiler

A language can survive a rough optimizer more easily than it can survive a chaotic migration story.

Once other teams and codebases depend on the language, the painful questions change again:

- how are keywords added,
- how are operators retired or reinterpreted,
- how are diagnostics kept stable enough for tooling,
- what kind of autofixes are promised,
- whether formatter output is stable across editions,
- and whether a machine-readable representation of the source or summaries is treated as public or internal.

The current book already does unusually well on diagnostics. That is a strong base. The missing layer is policy.

A practical mature-language policy for Optic should include:

1. an edition mechanism,
2. a documented deprecation window,
3. a compiler `fix` mode keyed by stable diagnostics,
4. a formatter with explicitly versioned stability expectations,
5. a linter/profile system that reuses diagnostic ids instead of inventing a second namespace,
6. and a statement about which machine-readable artifacts are considered stable enough for tools.

The user's earlier suggestion that the book may eventually be easier to maintain in JSON points to the same underlying pressure. Tooling ecosystems grow around structured data. If Optic wants to be friendly to editors, refactoring tools, package tooling, and coding agents, it should eventually define stable schemas for at least:

- diagnostics,
- module interface summaries,
- benchmark/regression metadata,
- and perhaps selected HIR/CGIR export views.

#### 27.9.1 A concrete public policy is better than "we'll be careful"

A credible first compatibility and migration policy could be stated very plainly.

- new hard keywords first land as warnings or contextual keywords, then become hard only at an edition boundary;
- operator precedence may not change within an edition line;
- formatter output is stable within an edition and target-independent except for explicit line-ending policy;
- `optic fix`, `optic migrate`, and edition-lint modes are first-party tools rather than community afterthoughts;
- every machine-readable artifact is labeled `public-stable`, `toolchain-stable`, or `internal`;
- diagnostic codes remain stable across editions unless the underlying rule disappears entirely, in which case the old code remains reserved with migration notes.

These rules are not glamorous, but they prevent the ecosystem from learning compatibility policy by rumor.

### 27.10 Debuggability, crash reporting, and profiler identity still need dedicated treatment

The book already insists that provenance survive fusion, which is the right foundation. What it does not yet describe in enough detail is the full debug/profiling/crash pipeline for a mature toolchain.

The missing questions include:

- how fused loops map back to source lines and optic names in DWARF or equivalent debug info,
- how staged artifacts preserve source identity,
- how panic/unwind/fault boundaries report boundary contracts,
- how the compiler can package a minimal reproducer when a crash crosses multiple fused nodes,
- and how benchmark or profiler samples remain comparable across editions and backends.

These are late-stage issues because they matter most once optimization becomes aggressive and third-party tooling begins to depend on the optimizer's output. They are also exactly the place where the current graph-first architecture is strongest. Source mapping should be easier here than in conventional compilers because the optimizer never had to rediscover the user's structure after lowering.

That strength is worth turning into a dedicated chapter later. The book should eventually contain a chapter on:

- debug info strategy,
- profiler attribution,
- crash capsules,
- stage artifact provenance,
- and translation validation for optimized code.

#### 27.10.1 Minimum mature debugging and profiling contract

Before the language can call itself operationally mature, the toolchain should guarantee at least the following.

- fused loops retain a logical callsite chain or provenance table in debug info;
- staged artifacts record their origin module, stage key, and source provenance;
- crash reports can emit a minimal reproducible capsule keyed by interface hashes and fused-node provenance;
- benchmark baselines and profiler samples are keyed by semantic `PerfKey`, target profile, and backend family rather than by raw symbol names;
- and translation-validation failures between backends preserve the same source-facing ids.

This is what makes optimization socially affordable. Engineers only accept aggressive fusion and staging when those transformations remain explainable after a production crash or regression.

### 27.11 The standard library, runtime surface, and package tool need an explicit center of gravity

Languages often fragment not because the core language is weak, but because the first wave of users grows several incompatible unofficial answers to the same operational question.

Examples include:

- multiple competing package workflows,
- multiple incompatible async runtimes,
- multiple partially overlapping FFI wrapper conventions,
- several no-std or freestanding subsets,
- incompatible plugin-loading models,
- and ad hoc generated-binding tools.

The current book already proposes a tiny `optic-runtime` for the prelude and explicit host-costate modeling for the rest. That is a good start. But the language will still need to say which tools and runtime surfaces are "canonical enough" that ecosystem code can depend on them.

The goal is not to monopolize experimentation. The goal is to prevent structural fragmentation in the areas where interoperability matters most.

For Optic, the most important candidates for a canonical first-party story are:

- native package declaration format and generated lock snapshot format,
- module interface artifact format,
- standard diagnostics schema,
- boundary-contract declaration and binding-generation pipeline,
- replay and benchmark artifact format,
- and the hosted versus `no_std` runtime split.

#### 27.11.1 The center of gravity should be organized in rings

The language does not need one enormous blessed platform. It does need a clear center of gravity.

A useful division is:

- **core standard library** for universally available language-facing types and algorithms;
- **runtime families** for hosted, `no_std`/freestanding, and later specialized embeddings, with explicit compatibility matrices;
- **first-party toolchain ecosystem** for package workflow, interface artifacts, binding generation, replay and benchmark formats, graph protocol, and plugin policy.

The key governance rule is that code should not need to guess which ring a dependency belongs to. Hosted versus freestanding, canonical package workflow, plugin loading, and async/coinductive runtime discipline must all have clear first-party answers even if alternative experiments remain possible.

### 27.12 What the current model still needs before it is operationally complete

By this point the remaining work is no longer mysterious. It falls into a small number of concrete chapters, schemas, and policies that the language still needs to write down. The list below is best read as a sequence of maturity passes, not as a pile of disconnected future work.

#### 27.12.1 Sequencing relative to the milestone ladder

The first wave belongs before M7: compatibility policy, module-interface artifacts, and native package declarations with generated lock snapshots. Those three items determine how the language will grow a library ecosystem, and they are harder to retrofit than they look.

The second wave belongs before M9: conformance and multi-implementation discipline. Self-hosting only becomes meaningful once the seed compiler and the self-hosted compiler can be compared against the same artifact schemas, diagnostics, and benchmark keys.

The third wave belongs before M10: the formal memory model, the debug/profiler/crash-provenance story, and the runtime-family/plugin policy. Kernel-class claims and aggressive backend optimization are simply too strong to make without those contracts in place.

Beyond that lies full maturity work: binding generation, standard-library governance, and the long tail of conformance infrastructure. Those are not optional, but they benefit from a real ecosystem already existing.

#### 27.12.2 A. Compatibility and edition policy

This work should define the edition mechanism, the deprecation window, the migration-tool contract, and the stability tiers for language and library features. The compiler already has the technical ingredients — edition-aware parsing, structured diagnostics, and fix-oriented tooling. What is missing is a public policy that tells users when and how change is allowed.

#### 27.12.3 B. Module interface and separate-compilation artifacts

This work should define the serialized interface artifact, the dependency and invalidation rules that govern it, the artifact-hashing policy, and the conditions under which a target or runtime-profile change forces rebuild. The current model already has the right raw material in `OpticSummary`, provenance, CGIR identity, and target profiles; it now needs a stable wire format and versioning discipline.

#### 27.12.4 C. Native package declarations, generated lock snapshots, and binary artifact story

This work should define the canonical native package/workspace surface, the generated lock artifact, staged and binary artifact metadata, supply-chain provenance hooks, and system-package integration policy. Because the build language is native Optic code, the real design job is not inventing another manifest DSL. It is defining the generated artifacts that serialize the results of that code in a reproducible way.

#### 27.12.5 D. Formal memory model chapter

This chapter needs to freeze pointer provenance, volatile and atomic semantics, fence and barrier rules, callback and interrupt visibility, and the stability policy for those rules across editions and runtime families. Until this exists, claims about kernels, plugins, DMA, or aggressive low-level optimization remain necessarily incomplete.

#### 27.12.6 E. Debug/profiling/crash provenance chapter

This chapter should define the debug-info mapping policy, profiler attribution format, crash capsules, and minimized reproducible failure bundles. The book already has the right conceptual foundation in provenance-preserving CGIR. What is missing is a stable external contract for the debugging and profiling ecosystem that grows around it.

#### 27.12.7 F. Foreign binding-generation chapter

This work should cover header and schema import, generated binding provenance, boundary-contract inference and override rules, regeneration workflows, and how generated bindings interact with lock snapshots and module interfaces. Hand-authored `extern` blocks are a good start, but not a complete long-term interop story.

#### 27.12.8 G. Conformance and multi-implementation discipline

This work should define conformance suites, normative artifact schemas, translation-validation expectations, and an implementation-divergence policy. The language's self-hosting ambition, multi-backend ambition, and tooling ambition all depend on this becoming explicit earlier than feels comfortable.

#### 27.12.9 H. Runtime-family and plugin policy

This chapter should define runtime-family identifiers, plugin ABI/runtime declarations, managed-runtime bridge guarantees, and callback contracts by runtime family. Once hosted, freestanding, kernel, event-loop, and managed embeddings all exist, the language needs a public way to say which world a package or artifact assumes.

#### 27.12.10 I. Standard library scope and governance

This work should fix the criteria for first-party libraries, the split between core and batteries, the stability promises by crate or package tier, and the deprecation/removal process. Without that center of gravity, the ecosystem will eventually rediscover several incompatible “normal” ways to solve the same operational problem.

#### 27.12.11 J. A practical research ladder for v1 stabilization

Not every unresolved semantic question needs the richest available mathematics first. For v1 the language should prefer the **smallest theory that closes the operational gap** while still fitting the existing costate/optic/summary architecture.

The direct experimental candidates are:

- **resource and separation reasoning** for ownership, provenance, unsafe boundaries, and local alias proofs;
- **weak-memory and ordering semantics** for atomics, fences, volatility, DMA visibility, and backend reorder legality.

These are more direct answers to the current memory-model backlog than starting with sheaf, topos, or HoTT machinery. They fit the present architecture with fewer new concepts because they already speak in terms of resources, accesses, obligations, and reorderings.

Alongside those direct lanes, the language should keep several **simpler sidecars** available while the core stabilizes:

- **typestate and explicit protocol automata** as a narrower first answer before richer session or sheaf machinery;
- **TLA+ and Alloy** as external specification/model-checking companions for callback, protocol, scheduler, and distributed-runtime design;
- **abstract interpretation** as a conservative optimizer and checker companion before proof-heavier machinery is justified.

The richer experimental lanes — `std.experimental.proof`, `std.experimental.sheaf`, `std.experimental.topos`, and `std.experimental.dynamics` — remain valuable, but they should compete against these smaller answers rather than being assumed to be the first step.

A good working rule for the first stabilized line of the language is therefore:

- keep **one direct internal stack** for the memory-model backlog — access-based regions, fractional/resource ownership, weak-memory legality, and proof/translation witnesses;
- keep **one sidecar stack** for protocol and approximation work — typestate/protocol automata, TLA+, Alloy, and abstract interpretation;
- treat richer proof, sheaf, topos, and dynamics lanes as **second-wave refinements** that must show what they can answer that the two simpler stacks cannot.

This is the smallest coherent research stack that still aligns with the rest of the architecture. It avoids forcing the v1 compiler, package tool, and runtime-family policy to absorb every available theory before the narrower answers have been exhausted.

It also makes one limit explicit: **no single experimental lane answers all of the remaining obligations**. The language still needs a stack, not a silver bullet. The direct lanes answer the memory-model core, the sidecars answer many protocol and approximation questions cheaply, and the richer second-wave tracks remain available when the first two layers stop being enough.

### 27.13 More mainstream languages still expose different failure surfaces

Section 27.4 already covered the five ecosystems most repeatedly used earlier in the book. This section adds a different set of lessons: runtime-family clarity, binary-evolution policy, compile-time latency discipline, and mixed-language adoption pressure. The question stays the same — which pressure became non-negotiable once a language stopped being a research artifact and became an ecosystem? — but the examples are chosen to add new constraints rather than to repeat the earlier comparison set.

#### 27.13.1 A compact map before the detailed discussion

The additional languages in this section matter because they each make a different maturity pressure impossible to ignore.

Go foregrounds the value of one compiler-owned workflow and a narrow, explicit compatibility promise. Swift makes binary evolution and module stability impossible to treat as afterthoughts. Kotlin shows that gradual interop has to feel routine if a new language wants adoption inside existing codebases. C# and .NET force runtime-family diversity into the open. Node.js and Elixir demonstrate that a runtime model can become part of a language's public identity rather than just a library choice. Julia turns compile-time latency into a product issue. Scala reminds us that macros, plugins, and migration boundaries can quietly become a second language if they are not constrained early.

The detailed subsections that follow draw out those pressures one by one. The goal is not breadth for its own sake. It is to widen the chapter's evidence base without repeating the same earlier comparison logic in a new table.

#### 27.13.2 Go: compatibility promises and one-tool workflows reduce ambient uncertainty

Go is an especially useful contrast because many of its strengths are social-architectural rather than type-theoretic. The language and toolchain deliberately narrowed the number of moving pieces that ordinary developers had to reason about: a source-compatibility promise under Go 1, one primary `go` command, and a module system owned by the toolchain rather than delegated entirely to third-party convention.

The key lesson is not that Optic should become minimal in the Go sense. The key lesson is that a language earns trust when it makes the boundary between "the language" and "the build/package tool" unusually crisp.

For Optic, the analogous guardrails are:

- the compiler should own module interface artifacts, not merely consume ad hoc metadata files,
- package identity and checksums should be compiler-visible facts, not only package-manager facts,
- and the compatibility promise should be written down in layers.

Go is also a warning against a common mistake. The Go 1 promise is source-level; it is not a blanket promise of compiled binary compatibility. That separation is healthy. Optic should make the same distinction explicit instead of allowing source compatibility, serialized module-interface compatibility, plugin ABI compatibility, and runtime-family compatibility to blur into one marketing phrase.

Cross-reference this directly with §§27.3, 25.5, and 25.6. The book already argues that compatibility has layers and that package metadata needs its own chapter. Go strengthens that argument by showing that explicitness itself is a large part of the ergonomic win.

#### 27.13.3 Swift: ABI, module stability, and library evolution became separate jobs

Swift is the clearest industrial example of a language discovering, in public, that source language design is only one part of ecosystem stability. ABI stability, module stability, and library evolution had to be named separately because they solve different problems:

- ABI stability answers whether binaries compiled with different compiler/runtime versions can interoperate.
- Module stability answers whether one compiler can understand another compiler's module artifact.
- Library evolution answers how a library changes without forcing all clients to rebuild.

The important point for Optic is not the specific Swift mechanism. It is the deeper systems lesson:

> once binaries, plugins, drivers, or separately shipped libraries matter, resilience is a costed feature, not a free slogan.

Swift is explicit that library-evolution support changes performance characteristics and even some source-language behavior around exhaustiveness. That is the kind of honesty Optic will need once it moves from "compile everything together" to plugin ecosystems, driver surfaces, or binary-distributed runtime libraries.

The current model already has the right seeds for this:

- `TargetProfile`
- `BoundaryContract`
- staged and target-specific artifacts
- explicit provenance and summaries

What is missing is a first-class *resilience policy*. The day-0 rule should be: if a package wants a stable binary surface, it opts into a stronger and more expensive contract. Otherwise the compiler is free to keep the stronger whole-program assumptions that make the generated code better.

#### 27.13.4 Kotlin: adoption accelerates when interop is gradual, not all-or-nothing

Kotlin's most important non-theoretical lesson is that a language can gain real adoption by making coexistence with legacy systems feel routine. Kotlin's Java interop is not treated as a side door. Mixed Kotlin/Java builds, API-shaping rules for Java-facing Kotlin code, and migration tutorials are all first-class parts of the official story.

That matters directly to Optic's full-generality goal. If the language is supposed to grow into kernels, browsers, compilers, games, databases, and services, it will spend a long time embedded in mixed environments:

- kernels with C and assembly,
- engines with C++ and scripting,
- enterprise systems with Java and .NET islands,
- existing Rust/C libraries,
- vendor SDKs,
- and generated or imported bindings.

The guardrail is straightforward:

> mixed-language builds and generated foreign bindings must be part of the happy path, not a heroic path.

This reinforces the earlier boundary-contract work rather than replacing it. `BoundaryContract` stays the semantic core. The extra requirement is operational: the package tool and module system must make "Optic plus legacy code" feel like one supported mode of development rather than an unofficial workaround.

#### 27.13.5 C# and .NET: one language may need several runtime families at once

The .NET ecosystem is useful because it demonstrates that a successful language may need to inhabit multiple deployment families simultaneously:

- JIT-compiled, managed, runtime-hosted execution,
- ahead-of-time native deployment,
- restricted environments where JIT is disallowed,
- and generated interop code that moves work from runtime to build time.

For Optic, this is a direct confirmation of a theme already emerging elsewhere in the book: runtime family must be a named part of the architecture. A language that wants to span hosted services, freestanding utilities, kernels, tooling, compile-time graph evaluation, and embedding into managed runtimes cannot pretend these are all one operational environment.

The current model can absorb this with minimal extension if the team keeps reusing existing concepts:

- extend `TargetProfile` with `RuntimeFamily`,
- keep `BuildRuntime` distinct from ordinary `Runtime`,
- and let `BoundaryContract` describe whether a foreign boundary is hosted, freestanding, managed, callback-heavy, unwind-capable, or AOT-only.

The deeper lesson from .NET is that interop generation is not a secondary tool once performance and deployment diversity matter. It becomes part of the compiler story.

#### 27.13.6 Node.js / JavaScript: the event loop becomes part of the language's practical identity

Node.js is valuable here not because Optic should imitate JavaScript, but because Node makes one fact unusually obvious: the runtime's concurrency shape becomes one of the language's public design decisions. The event loop is not merely a library convention. It governs what kinds of workloads are natural, which kinds of bugs dominate production, and how users reason about blocking.

That observation strengthens the earlier claim that Optic should never speak about "async" as if it were one thing. The language's own concepts already point in a better direction:

- `Coinductive` nodes,
- `LivenessGrade`,
- `BlockingGrade`,
- `HostContext`,
- and explicit driver/runtime families.

The right extension is therefore not a new async syntax regime. It is a clearer operational taxonomy: event-loop runtime, work-stealing shared-memory runtime, isolated-process runtime, freestanding/kernel runtime, and build-time runtime. Different domains will use different families, but they should all remain visible in the same artifact model.

#### 27.13.7 Elixir: isolated lightweight processes show that runtime semantics can be a product feature

Elixir and the BEAM ecosystem show the opposite pole from Node.js. Instead of a single-threaded event loop with kernel offload, the runtime is built around isolated lightweight processes, message passing, supervision, and fault containment.

The relevant lesson for Optic is not that it should turn into an actor language. It is that runtime semantics can be so central that they become part of the language's market identity. Once that happens, leaving runtime family implicit becomes dangerous. Users start writing systems that depend on hidden operational assumptions.

Optic should therefore treat runtime family the same way it treats ownership and host boundaries: explicit, typed, and inspectable. A coinductive driver that targets an event loop is not the same thing as a process-supervision runtime, even if both are expressible in the same surface language.

#### 27.13.8 Julia: compile-time latency is a product issue, not a side note

Julia is an especially important comparison for Optic because it validates the book's recent shift toward explicit compile-time execution. Julia's own documentation now treats package images, ahead-of-time compilation, and time-to-first-execution as first-class engineering topics. That is exactly the mindset Optic should adopt early.

If Optic intends to make staging and compile-time graph evaluation central, then three things need to be true from day 0:

1. compile-time work is represented explicitly (`BuildRuntime`, stageability judgment, compile-time artifacts),
2. compile-time cost is budgeted explicitly (`CompileTimeGrade`, stage boundaries, cache keys),
3. and compiled/staged results are cacheable as ordinary artifacts, not hidden compiler ephemera.

This is one of the strongest arguments for keeping the macro-free design. A second meta-language would make compile-time work harder to budget, cache, explain, and attribute. Ordinary optic graphs over `BuildRuntime` keep it under the same model as the rest of the language.

#### 27.13.9 Scala: migration cliffs often come from macros, plugins, and ecosystem coupling

Scala is already discussed earlier in this part of the book, but it belongs in this broader comparison set as well because it is one of the clearest examples of a language whose core ideas were strong enough for major adoption in some domains, while ecosystem evolution still had to contend with macro libraries, compiler plugins, and migration prerequisites.

The architectural lesson is straightforward:

> metaprogramming power should not be allowed to become a second, unstable compatibility surface by accident.

For Optic, that means the default metaprogramming path should remain staged graph transformation over explicit IR-like structures. If the language later admits plugins or richer compile-time APIs, they should ride on versioned, constrained schemas rather than compiler-internal structures.

### 27.14 Languages with strong ideas that stayed niche still matter

It is easy to learn the wrong lesson from niche languages. The wrong lesson is that their ideas were bad. The more useful lesson is usually that the ideas were real, but the surrounding adoption costs stayed too high for broad uptake.

That is exactly why they are valuable to study.

They often reveal failure modes earlier and more starkly than mainstream languages do.

#### 27.14.1 D: a systems language should not need a second personality to interoperate cleanly

D is a particularly important cautionary example for Optic because it lives close to the same aspiration space: systems programming, native code, performance, safer subsets, and C interop.

Its `@safe`/`@trusted`/`@system` split is useful. Its BetterC subset is also revealing for a different reason. The official documentation is explicit that linking D into C is not straightforward, and that BetterC exists as a restricted subset precisely to avoid depending on the D runtime. The documentation also warns that BetterC use can lead to cryptic compiler or linker errors when libraries assume more runtime support than the subset provides.

The design lesson for Optic is strong:

> do not let hosted Optic, freestanding Optic, kernel Optic, and interop Optic become four different personalities of the language.

The current architecture can avoid that if it keeps using:

- `TargetProfile`,
- `RuntimeFamily`,
- `BoundaryContract`,
- `unsafe optic`,
- and capability-gated host costates.

That is the minimal-extension way to absorb the hard realities D exposes without inventing a separate BetterOptic dialect.

#### 27.14.2 Nim: pleasant syntax does not rescue weak artifact and package discipline

Nim is a useful caution because it shows that a friendly surface language and straightforward C interop do not automatically produce a broadly trusted operational ecosystem. Nimble is practical and lightweight, but its own guide still reflects a world where external VCS tools are assumed and where package resolution behavior follows repository tags and, in their absence, even the latest commit. That is workable for an enthusiast ecosystem. It is a weak default for a language that wants kernel-class tooling, long-lived services, or reproducible industrial builds.

The relevant Optic lesson is not about syntax at all. It is about artifact determinism:

- package resolution must be reproducible,
- lock/checksum behavior must be part of the standard workflow,
- and generated bindings or foreign summaries must participate in that same artifact identity.

This fits naturally with the current model if module interface artifacts, staged artifacts, and boundary-generated code all share the same hash/manifest discipline.

#### 27.14.3 Pony: very strong concurrency semantics can narrow a language's perceived domain

Pony's actor model and reference capabilities are intellectually impressive. The language promises data-race freedom, no locks, and capability-checked concurrency without runtime overhead. But the tutorial also makes clear just how much of the language's everyday mental model is organized around actors, asynchronous behaviours, and a capability lattice with several modes.

The lesson is not that Pony's model is wrong. The lesson is that a language can ask too much cognitive buy-in up front if its most unusual subsystem becomes the default way to think about all programs.

Optic should avoid that trap. It already has a better route available:

- keep `Runtime` and optics as the general base model,
- treat actor/process/event-loop/shared-memory execution as runtime-family choices,
- and keep advanced concurrency structure visible where it matters without requiring every introductory program to internalize the whole concurrency calculus.

#### 27.14.4 Idris: proof power is real, but proof burden changes the social surface of the language

Idris is a classic example of a language whose ideas have influenced the broader field without itself becoming a mainstream implementation language. The official tutorial and docs foreground dependent types, totality, and the fact that only total functions are safe to evaluate during type checking. The official site also still describes Idris 2 as a work in progress.

Those are not flaws. They are signals of what kind of ecosystem burden a dependently typed language carries:

- proof obligations are real work,
- compile-time execution must be tightly controlled,
- and tool maturity matters enormously because the type checker is no longer just checking simple shape compatibility.

The Optic lesson is not to abandon expressive static reasoning. It is to stratify it.

Routine programming should stay inside a fast, predictable, executable core. Stronger proof-oriented modes should be opt-in and local: gradual grades, checked unsafe wrappers, staged graph proofs, or dedicated verification-oriented crates and chapters.

#### 27.14.5 ATS: theoretical power alone does not produce a large ecosystem

ATS is another language that deserves respect rather than dismissal. Its own description emphasizes dependent types, linear types, formal specification, and C-level efficiency. That is an extraordinary technical package.

It is also a warning. A language can be extremely powerful and still remain niche if:

- ordinary programming feels like theorem-guided programming too early,
- the tooling surface is too specialized,
- or the migration and interop story asks too much up-front expertise.

For Optic, this reinforces the narrow-v0-first doctrine. The language should not make users pay proof-theory prices in the common case if its public ambition is broad systems adoption.

#### 27.14.6 These niche cases should be read as design warnings, not as dismissals

D, Nim, Pony, Idris, and ATS did not "lose" because their ideas were empty. Many of their ideas continue to shape other languages.

The more accurate interpretation is this:

- D warns about split language personalities.
- Nim warns about package and artifact looseness.
- Pony warns about making one advanced runtime model the default way to think.
- Idris warns about proof burden and tool maturity.
- ATS warns that raw expressiveness is not a substitute for adoption architecture.

Optic can benefit from all of them if it converts those warnings into day-0 constraints rather than into retrospective regrets.

### 27.15 The recurring reasons strong languages stall or fracture

Looking across both mainstream and niche languages, a surprisingly small set of recurring patterns appears.

#### 27.15.1 Pattern 1: too many new mental models arrive at once

When a language asks users to adopt a new memory model, new concurrency model, new package model, new macro model, and new proof model at the same time, even strong ideas become socially expensive. Pony, Idris, and ATS reveal this starkly. Scala also exposed a milder form of the same problem around macros and contextual abstractions.

**Optic guardrail:** keep the prelude psychologically small even if the long-term architecture is large. Advanced grades, coinduction, richer optics, and proof-oriented staging should layer on top of one executable core rather than replacing it.

#### 27.15.2 Pattern 2: interop exists, but only through a second personality of the language

D's BetterC story is the clearest warning. As soon as a language needs a special freestanding/interoperable subset, users start learning two models and toolchains start encoding special cases.

**Optic guardrail:** model all hostile boundaries through `BoundaryContract`, `unsafe optic`, `TargetProfile`, and `RuntimeFamily` rather than through a second sublanguage.

#### 27.15.3 Pattern 3: package and artifact determinism arrive late

Python had to grow standard environment and lockfile work. Nimble exposes the cost of a lighter package discipline. Go demonstrates the opposite: the module file is part of the ordinary compiler/tool story.

**Optic guardrail:** ship one canonical native workspace/package declaration surface, one compiler-generated lock/checksum story, and one compiler-visible module-interface artifact before the ecosystem improvises its own.

#### 27.15.4 Pattern 4: the runtime family remains implicit for too long

Node.js, Elixir, Python free-threading, and .NET Native AOT all show that runtime-model changes are not merely implementation details. They alter libraries, deployment, interop, debugging, and performance expectations.

**Optic guardrail:** runtime family must appear in native package/workspace declarations, generated module-interface artifacts, diagnostics, and target profiles.

#### 27.15.5 Pattern 5: compile-time work is treated as free until users revolt

Julia's package images and AOT systems make the issue explicit. Scala's macro and plugin migration pain is a related story from the metaprogramming side.

**Optic guardrail:** treat compile time and cold-start latency as budgeted resources. `BuildRuntime`, stageability, compile-time caches, and `CompileTimeGrade` already point in that direction and should remain central.

#### 27.15.6 Pattern 6: compatibility is promised too vaguely

Go shows the value of a narrow, explicit source compatibility promise. Swift shows how ABI, module, and library evolution become separate operational concerns. C++ shows how modules do not erase ABI or build consistency.

**Optic guardrail:** define compatibility as a matrix, not a slogan:

- syntax/edition compatibility,
- source compatibility,
- module-interface compatibility,
- package/build reproducibility,
- binary/plugin ABI compatibility,
- runtime-family compatibility.

#### 27.15.7 Pattern 7: macros, plugins, and code generation quietly become a second language

Scala is the strongest warning here, but many ecosystems run into the same issue. Once code generation depends on unstable internal compiler APIs, the language acquires a shadow language that is hard to migrate and hard to specify.

**Optic guardrail:** keep compile-time programming inside ordinary staged optic graphs as long as possible. If plugins exist later, make them schema-driven and versioned.

#### 27.15.8 Pattern 8: the standard library and package ecosystem develop without a center of gravity

Many ecosystems eventually accumulate multiple partially incompatible "normal" ways to solve the same runtime, package, async, FFI, or plugin problem.

**Optic guardrail:** name the first-party center early: native workspace/package declarations, generated lock artifacts, diagnostics schema, boundary contracts, replay artifacts, and runtime-family declarations.

### 27.16 Architecting against those failures from day 0

The most useful response is not a warning list. It is a concrete set of architectural rules that translate the warnings into design constraints.

The next section closes that argument by turning those rules into a hard classification scheme for future growth.

#### 27.16.1 Keep one semantic center

The language should continue to treat `Runtime`, `HostContext`, `ControlRuntime`, `BuildRuntime`, optics, summaries, and boundary contracts as one connected model.

That single-model discipline is the main protection against D-style split subsets, macro shadow-languages, and ad hoc runtime families.

#### 27.16.2 Make compatibility a typed matrix, not a promise paragraph

Add explicit fields to native package/workspace declarations and generated module-interface artifacts for at least:

- `edition`,
- `target_profile`,
- `runtime_family`,
- `interface_schema_version`,
- `boundary_contract_schema_version`,
- and package checksum / artifact identity.

This is the minimal extension that absorbs Go's compatibility clarity, Swift's resilience distinction, and C++'s module/ABI separation.

#### 27.16.3 Treat module interfaces as serialized summaries, not opaque compiler trivia

The current architecture already revolves around `OpticSummary`, `RegionSet`, provenance, target profiles, and boundary contracts. That is enough to define a stable module-interface artifact.

Such an artifact should serialize:

- exported type and optic signatures,
- grades and constraints,
- region and boundary summaries,
- runtime-family declarations,
- target-profile assumptions,
- and artifact hashes of staged/generated subgraphs.

That is the direct way to prevent separate-compilation drift from becoming folklore.

#### 27.16.4 Keep interop on the main road

Interoperability should be built around one official pipeline:

1. import or declare foreign surfaces,
2. infer or author `BoundaryContract`s,
3. generate wrappers/bindings,
4. wrap unsafe leaves with safe optics,
5. publish the resulting summaries in the same module-interface artifact.

This is how Optic can borrow the good lesson from Kotlin and Java/Panama without turning into a language where unsafe interop lives outside the model.

#### 27.16.5 Do not create a BetterOptic subset

Hosted service builds, `no_std` utilities, kernel code, plugins, and build-time graph execution should all stay in the same language.

The legal differences should come from:

- `TargetProfile`,
- `RuntimeFamily`,
- capability-gated host costates,
- and `BoundaryContract` restrictions.

This is the clean way to incorporate D's warning while reusing existing concepts.

#### 27.16.6 Make package resolution and artifact identity reproducible from the beginning

The package tool should not be a later add-on. It should understand:

- native package/workspace declarations,
- generated lock snapshots and checksums,
- module-interface artifacts,
- generated bindings,
- staged artifacts,
- and target/runtime-family splits.

That is how Optic avoids recreating Python's and Nim's later packaging pressure in a new form.

#### 27.16.7 Treat compile-time work as a resource with budgets and caches

The existing compile-time execution model should be deepened, not replaced.

Specifically:

- `BuildRuntime` remains explicit,
- stageability remains a judgment over ordinary graphs,
- compile-time caches remain artifactized,
- and `CompileTimeGrade` remains the place where cold-start and build-time cost become visible.

This is the architectural response to Julia's lesson and to macro/plugin ecosystems that become opaque performance sinks.

#### 27.16.8 Keep advanced static reasoning opt-in and layered

The language should welcome richer grade dimensions, verification-oriented stages, and stronger proof machinery later, but ordinary production programming should still be possible within the executable core.

That means:

- fast prelude diagnostics,
- gradual grades where appropriate,
- local unsafe/boundary obligations,
- and specialized proof-heavy workflows that do not infect the everyday build.

This is how Optic can benefit from Idris-, ATS-, and Pony-like lessons without inheriting their adoption friction.

The practical corollary is that the v1 research program should prefer **smaller direct theories first**: resource/separation reasoning and weak-memory operational models before richer categorical machinery for the same questions, typestate before fully general protocol theories where that suffices, and sidecar specification/model-checking tools before new core syntax when the latter would only duplicate the former.

#### 27.16.9 Constrain metaprogramming and compiler extension points early

The safest default remains:

- stage ordinary optic graphs,
- serialize structured artifacts,
- avoid freeform AST-rewriting macros,
- and keep any future plugin system schema-driven and editioned.

This is the cleanest response to Scala-style migration cliffs and to the general tendency of macros to become a second unstable language.

#### 27.16.10 Make runtime families explicit everywhere they matter

A package, module artifact, benchmark, replay artifact, and diagnostic should all be able to say which runtime family they assume.

That includes at least:

- build-time runtime,
- hosted shared-memory runtime,
- event-loop/coinductive runtime,
- isolated-process/supervision runtime,
- freestanding runtime,
- kernel runtime,
- managed-host embedding runtime.

This is how Node.js, Elixir, .NET, and Python's runtime shifts are converted into a reusable Optic rule.

#### 27.16.11 Design migration tools before the first ecosystem fracture

The compiler already has strong diagnostics. That foundation should grow into:

- edition-aware rewrites,
- schema migrations for module artifacts,
- package-declaration schema migrations,
- generated-binding regeneration workflows,
- and source-format stability rules.

Go, Swift, and Scala all suggest the same thing from different angles: migration cannot remain improvised once the language is real.

#### 27.16.12 Publish the conformance and artifact story earlier than feels necessary

Because Optic wants to be self-hosting, performance-sensitive, and eventually multi-backend, it should publish conformance suites, artifact schemas, and translation-validation expectations earlier than most ecosystems do.

This is not bureaucracy. It is the only reliable way to stop later disagreements from becoming silent language forks.

#### 27.16.13 Stress-test domain collisions before v1 claims harden

A language that aims at kernels, browsers, databases, games, services, and compilers should not rely only on isolated domain benchmarks when deciding whether a feature is ready. Mixed-domain collision suites — bounded render loops fed by network ingress, transactional wrappers over legacy engines, experimental kernels embedded in ordinary traversals — are better proofs that the single-graph architecture is really holding.

#### 27.16.14 Experimental mathematics must remain subordinate to the ordinary summary path

Experimental tracks are valuable only if they enrich the existing architecture rather than fork it. The acceptance test for any mathematical extension is therefore whether it still lowers into ordinary summaries, grades, boundary contracts, and target-profile assumptions. If it needs a parallel legality pipeline, it belongs in research infrastructure, not in the language roadmap.

### 27.17 Closing the Design: Core, Boundary, or Never

By this point the book has enough architecture that future growth should stop being discussed as if every new use case deserves a new core mechanism. The language will stay coherent only if it becomes harder, not easier, to grow the surface irresponsibly. The right closure rule is therefore simple: every future demand must be classified as **core**, **boundary**, or **never in core**.

That closure rule now has one deliberate safety valve: a **quarantined experimental lane**. Experimental mathematics, proof machinery, and domain-specific research ideas may be implemented and benchmarked under the reserved `experimental` contextual keyword, the `std.experimental` namespace root, and the graph-native `ExperimentalArena`. They do **not** become part of the ordinary core surface merely because they are implemented. The point of the lane is to keep research inside the same semantic graph while keeping promotion into core rare and reviewable.

#### 27.17.1 What belongs in core

Core features are the ones that must participate directly in the language's ordinary semantic pipeline:

- explicit `Runtime`, `BuildRuntime`, and `Project` roots,
- optics and ordinary staged evaluation,
- `OpticSummary`, `RegionSet`, grades, and determinism classes,
- CGIR and provenance,
- native package/workspace declarations,
- ordinary optic-based queries over `Project`,
- and optics-native domain libraries such as `std.solver.*` when they lower through the ordinary summary and staging pipeline.

A feature belongs in core only when an existing optic, staged build graph, boundary contract, or generated artifact cannot already express the use case without losing semantic truth. The goal is not minimalism for its own sake. The goal is to keep the semantic center small enough that every core feature still enjoys the full summary, legality, provenance, and backend story.

#### 27.17.2 What belongs at the boundary

Boundary features are fully supported, but they are supported as explicit edges of the model rather than as new centers of it. They include foreign ABIs, raw hardware access, managed-runtime bridges, plugin ABIs, registry and package fetches, generated bindings, projection filesystems, direct graph protocols, and imported external package interfaces.

Their common rule is that they must pass through `BoundaryContract`, generated interface artifacts, runtime-family declarations, capability declarations, imported-front-end lowering, or compiler-owned materializations. The language covers the use case, but it does not multiply the number of semantic worlds in which that use case can be expressed.

#### 27.17.3 Permanent “never in core” table

| Tempting feature or promise | Why it is attractive | Why it stays out of the language core | Supported instead via |
|---|---|---|---|
| a second authored build/config language | familiarity from TOML/YAML/CMake-style ecosystems | creates a second semantic center and duplicates build facts the compiler already knows | native `package` / `workspace` declarations plus generated lock, interface, and build artifacts |
| a primary syntax-rewriting macro system | short-term expressive power and ecosystem experimentation | rewrites syntax rather than semantic graph structure and tends to become a shadow language | staged optic graphs, generated artifacts, and later schema-driven plugin points |
| ambient exceptions as the default control model | terse error propagation and legacy familiarity | hides control edges from CGIR, replay, provenance, and staged legality checks | prisms, `Result`, `Option`, and a possible later explicit control layer |
| a BetterOptic-style hosted/freestanding/kernel dialect split | makes hard domains feel easier by narrowing semantics | fractures the language into personalities and duplicates runtime/toolchain policy | `RuntimeFamily`, `TargetProfile`, capability-gated host regions, and `BoundaryContract` |
| a vague blanket “stable ABI” promise | sounds ecosystem-friendly | source, module, runtime, and plugin compatibility are different contracts with different costs | explicit C-facing ABI guarantees first, opt-in plugin/runtime-family contracts later |
| multiple blessed package workflows | social pressure from different communities | fragments ecosystem identity, caching, and artifact schemas | one first-party native package/workspace/build path with generated artifacts |
| unstructured `unsafe` | convenience at hard boundaries | destroys summary completeness and auditability | `unsafe optic`, `BoundaryContract`, and the published soundness-budget ledger |
| a separate user-facing query language for `Project` | query ergonomics and tooling power | recreates a second meta-language beside ordinary code | ordinary optic-based project queries with internal QIR lowering |
| a core-language solver DSL | dense equation syntax and domain familiarity | creates a second semantic center for numerical work and duplicates staging, summaries, and artifact rules | optics-native solver libraries plus imported or embedded modeling frontends that lower into the same graph |
| plugins over unstable internal compiler IRs | fast experimentation | creates a shadow compiler API that is harder to migrate than the core language | versioned graph protocol, schema-driven tool/plugin surfaces, generated interface artifacts |

#### 27.17.4 Support use cases, not surface imitation

This is the practical feature-admission test.

1. Does the proposal unlock a genuinely new use case?
2. If so, can that use case already be covered by ordinary optics, staged build-time execution, boundary contracts, or generated artifacts?
3. If yes, do not add a new core feature.
4. If no, either admit it into core with a full summary/lowering story or keep it explicitly at the boundary.

The language should therefore measure itself against the use cases of C++, Rust, Go, Java, Python, TypeScript, and the more niche languages discussed later in this chapter without inheriting their surplus surface area. Cover the use case; do not inherit the extra semantic center that originally carried it.
A solver stack is a good concrete example. The language should absolutely cover imported equation systems, generated stencils, Jacobian builders, sparse kernels, and target-aware stepping loops. It does not follow that the core language needs a second solver calculus. In Optic, the default answer should remain optics-native solver libraries plus staged artifact generation, with any denser notation treated as a frontend rather than as a second semantics.

#### 27.17.5 Why this matters for the whole ecosystem

This closure rule does more than control feature creep. It also explains how the whole language ecosystem can live inside one larger costate graph. Registries, external packages, interface artifacts, generated bindings, replay capsules, benchmark baselines, plugin descriptors, and tool protocols all become ordinary regions, projections, or boundary edges of the larger semantic graph precisely because the language refuses to treat each new pressure as grounds for a new sublanguage. Saying “never in core” is the positive act that makes one ecosystem graph rich enough to host all of them.

### 27.18 Why these gaps still fit the current architecture

The strongest conclusion of this chapter is not that the language needs a second theoretical center. It is that the current center is strong enough to carry the missing work if the team keeps reusing it.

- Compatibility policy reuses editions, target profiles, and boundary contracts.
- Module artifacts reuse summaries, provenance, and graph hashes.
- Packaging reuses target profiles, staged artifact ids, and diagnostics.
- Runtime coherence reuses `Runtime`, `HostContext`, `ControlRuntime`, and grades.
- Soundness boundaries reuse `unsafe optic` and `BoundaryContract`.
- Debuggability reuses provenance-preserving CGIR and stable diagnostic ids.

That reuse matters. Many languages become internally inconsistent when they solve late-stage problems with ad hoc subsystems that do not share the same vocabulary as the core language. The Optic design has a chance to avoid that, but only if it keeps refusing the temptation to solve operational maturity with hidden mechanisms.

### 27.19 Implementation priorities and remaining empirical questions

This section gathers the considerations that belong in the book but do not fit neatly under any single gap category. They arise directly from the question: given the current specification, what are the next concrete decisions and risks?

#### 27.19.1 The compiler critical path

The language has a good specification and a publishable book. The most important next step is M0–M6: building the narrow v0 compiler that makes the book's claims real rather than plausible. Without running code, every subsequent claim is design fiction.

The five milestones have a specific risk ordering.

**M0–M1 (parser and HIR)** are mostly mechanical. The risk is grade inference diverging from what the specification says. The concrete two-dimensional arithmetic is simple, but the interaction between inferred grades and declared bounds at composition sites needs test coverage from day one — not as a later clean-up pass.

**M2 (typed HIR)** contains the hardest v0 problem: the alias checker. Write/write and write/read conflicts in product compositions are the first place where the theory must become executable code and produce deterministic diagnostics. The `invalid_alias.opt` acceptance example must be committed and passing before M2 is declared. The alias checker must be tested against pathological cases — deeply nested products, self-referential costates, and compositions where the conflict is in the `put_reads` field rather than `put_writes` — because `put_reads` conflicts are the most common false negative.

**M3–M4 (CGIR and fusion)** introduce the risk of optimization incorrectness. The three fusion passes must have algebraic soundness arguments written down before they are merged, not after. Provenance — the `original_ids` mechanism that keeps fused loops traceable back to source optics — is easy to underspecify and very hard to retrofit. It must be part of the fusion pass specification from the beginning, not added when debugging becomes difficult.

**M5–M6 (Rust backend and prelude release)** introduce a different risk: the generated code looks correct but is not what the benchmark baselines should be. Benchmark baselines must be committed before M6 is declared, not measured afterward. The benchmark harness should record the semantic `PerfKey` (optic name and composition shape), the target profile, and the tolerance band. Without committed baselines, "benchmarks green" has no meaning.

#### 27.19.2 Three front-loaded decisions that unblock M7

These questions are no longer left as open design suspense. They are fixed early because each one changes artifacts that the implementation should not have to retrofit later.

**Decision 1: fractional ownership is the underlying carrier from the beginning.**

The architecture now adopts fractional ownership early, with `SharedGrade`, `AffineGrade`, and `LinearGrade` retained as named surface aliases. This is the right compromise. It front-loads the hard carrier decision, preserves a friendly surface for most programs, and avoids building an alias checker that would later need a semantic transplant.

The main payoff is compositional parallel-product proofs for partition-shaped programs. Two secondary gains follow: field-level lending and stronger proof transport through composition. The book is now explicit that the multicore payoff lands later, but the *carrier decision* belongs early.

**Decision 2: asymmetric I/O optics use an explicit surface type.**

The surface form is fixed as `AsymmetricGradedOptic<S, A, G_get, G_put>`, with ordinary `GradedOptic<S, A, G>` remaining as sugar for the symmetric case. The composition laws are fixed in Chapter 15 and no longer left for a later design note.

**Decision 3: the first session-type scope is intentionally narrow.**

The first session-type release is binary, linear, non-delegating, and checked at optic boundaries by structural duality. This is enough for the network and protocol use cases that motivate the feature, while keeping the checker and diagnostics tractable.

The broader lesson is that the language should front-load decisions whenever they affect long-lived carriers, summary formats, or artifact schemas. That is the point where design indecision becomes retrofit cost.

#### 27.19.3 Formal properties that remain unverified

The book makes claims about fusion soundness, grade algebra well-formedness, and alias safety completeness that are asserted but not machine-checked. This is not unusual for a first publishable edition of a language specification, but the claims should be explicitly tagged as conjectures with stated proof obligations rather than allowed to harden into assumed facts.

**Fusion soundness.** The three v0 fusion passes (map fusion, compose fusion, product flattening) are presented with correctness arguments but no formal proof. For the prelude, thorough golden-test coverage is sufficient to proceed. For the full language — where traversal fusion, prism fusion, and staged fusion add significantly more complex rewrite rules — an unsound rewrite is a serious risk. The target for formal verification is Lean 4 or Agda, operating over a small-step operational semantics for the grade calculus. This should be pursued as a background obligation alongside M7, not deferred to post-M10.

**Grade algebra well-formedness.** The product of eight semirings must satisfy the semiring axioms for each dimension and the distributive law for the product. The less obvious dimensions — session types under sequential composition, security lattice under parallel product — need case-by-case checks. An error in the distributive law for any dimension would make grade normal forms non-unique, which would invalidate the claim that fusion legality is proof-directed rather than heuristic.

**Alias safety completeness.** The conservative region language is intentionally an over-approximation. The open question is whether it produces too many false-positive alias conflicts in realistic programs. This requires empirical measurement against real systems code — game engine ECS loops, database query plans, kernel page table walkers — not just the nine acceptance test cases. A false positive rate above roughly 5% on realistic programs would suggest the region language needs refinement before the full language ships.

#### 27.19.4 Three tooling investments that compound early

These are not blocking for any milestone, but they pay dividends across every subsequent session of work. The cost of establishing them during M5–M6 is low; the cost of retrofitting them during M9 is high.

**A structured benchmark harness with committed baselines.** The benchmark format should record: the semantic `PerfKey` (optic name and composition shape, not a raw symbol name), the target profile (architecture, ISA extensions, cache configuration), the backend family (Rust transpiler vs LLVM), and a tolerance band for pass/fail. Without committed baselines, benchmark regressions are invisible until they accumulate into production failures. This harness should be part of M5 release gates, not an M6 add-on.

**Agent-facing diagnostic validation before M6.** The structured diagnostic schema — stable codes, evidence objects, ranked fixes, next-command suggestions — is already specified. Before M6 is declared, the schema should be validated against a real automated repair loop: an agent given a grade violation, an alias conflict, and a type mismatch, allowed to resolve each in one pass using only the structured JSON output with no additional context. If any of the three requires more than one pass, the evidence field for that diagnostic code is missing something specific. That specific thing should be added before M6, not filed as a future enhancement.

**Translation-validation harness between Rust and LLVM backends.** Before the LLVM backend is authoritative, the project needs a way to verify that LLVM-generated code is semantically equivalent to Rust-transpiled code for the same source program. The harness should: compare diagnostic JSON for the known-good acceptance suite between both backends; compare generated loop shapes for canonical examples; compare benchmark deltas against seed tolerances. Without this harness, LLVM codegen cannot be trusted to maintain the semantic invariants the book claims it preserves, and the claim that the Rust backend is a "semantic microscope" becomes unfalsifiable once the LLVM backend is the default.

### 27.20 Summary: the road to a working language

By the end of this chapter, the remaining work should feel less like an unbounded backlog and more like a sequence of obligations that follow directly from the architecture the book has already chosen.

The first obligation is still the narrow compiler: M0–M6 must turn the core thesis into running code, with the alias checker at M2 as the hardest immediate risk. The second is to implement against the now-front-loaded M7 decisions — fractional ownership as the underlying carrier, explicit asymmetric-grade syntax, and a narrow first session-type scope — so the full-language work does not begin from an unstable summary format. The third is to publish the ecosystem-facing contracts early: module interfaces, edition policy, and native package plus lock artifacts before the first library culture improvises its own rules.

After that, the priorities become the things that keep a promising language from becoming a durable one: benchmark and translation-validation discipline, a published soundness budget, an explicit memory model, runtime-family and plugin policy, and the debugging/profiling contracts that make optimized code explainable. Formal verification, richer session and security grades, and later research expansions still matter, but they no longer look like the main gate between the current project and a working language.

That is the final shape of the argument. The language does not need a new semantic center to mature. It needs the courage to keep extending the current one — explicit runtime roots, explicit optics, explicit summaries, explicit grades, explicit boundary contracts, explicit graph artifacts, and explicit policy — into the places where other languages most often become vague.

### 27.21 Transition to the standing checklist chapter

The maturity chapter ends by naming the pressures and the remaining obligations. The next chapter turns those pressures into a standing review discipline: a feature-admission checklist that future proposals must answer in one place, plus a summary of what current coding agents still struggle with in large codebases and why those struggles matter to Optic's architecture.

That ordering is deliberate. The language first identifies the late-stage risks, then it fixes the review questions that stop those risks from reappearing as ad hoc exceptions.

