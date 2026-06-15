## 22. Compilers, Graph-Resident Tooling, Self-Hosting, and Governance

### 22.1 Why compilers are a natural target

```rust
optic ConstFold: GradedOptic<Module, Instr, CacheGrade<2> + CompileTimeGrade> {
    get  m => fold_if_constant(m.instructions[m.id])
    put  (m, i) => { m.instructions[m.id] = i }
}
```

Compiler passes are especially revealing because they show that the optic model is not only for physical resources. It is also a disciplined way to talk about IR-to-IR transformation while keeping provenance and legality visible.

A compiler already manipulates explicit graphs and trees, performs staged analysis and transformation, and cares deeply about diagnostics, provenance, and optimization legality. That makes it one of the most natural downstream domains for the language.

Dead-code elimination, constant folding, register allocation, CFG simplification, scheduling, and lowering all fit the optic story surprisingly well once the core calculus is strong enough.

### 22.2 Self-hosting as a late-stage discipline

#### 22.2.1 Differential trust loop

```text
1. compile compiler sources with the Rust seed
2. compile the same sources with the Optic compiler
3. compare diagnostics JSON on green and red suites
4. compare generated Rust or native output on canonical examples
5. compare benchmark drift against seed tolerances
```

Self-hosting becomes meaningful only when this loop is boring and repeatable.

The book treats self-hosting as a long-range systems milestone, not a branding event. The right sequence remains:

1. build and freeze the Rust prelude compiler,
2. move front-end pieces into the language,
3. move type checking, summaries, and CGIR machinery into the language,
4. validate against the Rust seed with differential tests,
5. only then move the rest of the compiler under self-hosting discipline.

This is one of the places where the book's governance stance matters. A self-hosted compiler that is not yet reproducible, diagnosable, and benchmarkable is not progress. It is a new source of ambiguity.

#### 22.2.2 What should become reusable libraries, and what must stay compiler-specific

A self-hosted compiler should not be understood as "ordinary user code plus the standard library". That model either bloats the standard library with compiler internals or forces every compiler component to reinvent the same substrate privately. The healthier split has three rings.

| Ring | Typical contents | Stability expectation | Why it belongs there |
|---|---|---|---|
| core standard library | collections, text/bytes primitives, paths, numeric utilities, `Option`/`Result`, general optic combinators, target-neutral concurrency helpers | language-level and broadly stable | ordinary programs should use these without inheriting compiler-version coupling |
| first-party toolchain support libraries | spans and file maps, interning, stable hashing, index arenas, mmap/journal graph-store primitives, module-interface codecs, diagnostics schema/renderers, target-profile descriptions, object/debug-info emitters, package-resolution engine, graph-protocol libraries | toolchain-stable, may evolve with compiler editions | reusable by the compiler, language server, package manager, alternate front ends, and analysis tools without freezing language law |
| compiler-private implementation | grammar tables, HIR/CGIR schemas, resolver, type rules, grade solver, summary builder, alias checker, fusion legality, backend legality, edition migrators, invalidation policy | compiler-private | these components embody the language's semantic authority and must remain free to evolve with the compiler |

The practical conclusion is subtle but important. A **large share of the infrastructure by code volume** may eventually live in reusable first-party libraries, while the **semantic authority of the compiler** should remain concentrated in compiler-specific crates. That is the right balance. It keeps the self-hosted compiler from degenerating into a pile of bespoke utilities, but it also avoids freezing optimizer laws or IR shapes as if they were standard-library promises.

A good promotion rule is conservative: move a component from compiler-private to first-party library only after at least two independent consumers need it and its contract can be described without reference to one particular compiler pass. Generic graph stores, span/file-map utilities, stable hashing, artifact codecs, and diagnostic schemas often meet that bar. HIR node sets, grade-law tables, or rewrite legality checks usually do not.

This split should become explicit in the long-range governance material and is summarized again in Appendix I together with the soundness-budget ledger and artifact-publicity classes.

#### 22.2.3 Toolchains are foreign-boundary systems too

Compilers often look self-contained on paper. In practice they are some of the most boundary-heavy programs in a systems stack. They parse files, cache artifacts, call assemblers and linkers, embed debuggers and profilers, interface with platform SDKs, consume foreign optimization libraries, and increasingly cooperate with editors, language servers, package managers, and AI agents.

That makes the compiler domain a good confirmation of the earlier interlude. The same boundary model that explains MMIO and graphics callbacks also explains toolchain reality.

- an external assembler or linker is an ABI boundary with determinism and staging consequences;
- LLVM or platform SDK bindings are foreign libraries whose summaries should remain visible to diagnostics and replay tooling;
- editor and LSP integration is a callback-heavy host interface rather than a separate semantic universe;
- AI-assisted repair loops rely on the same structured diagnostics whether the failing boundary is a parser rule, an optimizer invariant, or an FFI contract.

A compiler written in the language should therefore not need a privileged "metacompiler" escape hatch. It should exercise the same boundary discipline the language expects of kernels, services, and engines.

### 22.3 The project graph as the compiler and tooling system's primary costate

A mature compiler eventually starts to look like a database whether it admits this or not. It stores source text, syntax trees, typed IR, summaries, module interfaces, package resolution, diagnostics, generated artifacts, benchmark baselines, runtime blueprints, and invalidation metadata. If those are all kept in separate places, the toolchain spends an increasing fraction of its complexity budget just translating among its own partial copies of the world.

The better long-range architecture is to make the **project graph** primary and to treat the compiler graph as one hot projection of it.

```text
ProjectRuntime = ProjectGraph × CompilerHost × SessionState
ProjectGraph   = TextArena × SyntaxArena × HirArena × SummaryTable × CgirArena × ArtifactIndex × RuntimeBlueprintIndex × ProjectionTable × WatchTable
CompilerGraph  = ProjectGraph ⊳ compiler view
```

This reuses the language's existing conceptual center instead of inventing a second one. The project graph is just another explicit costate. Parser, resolver, type checker, optimizer, serializer, materializer, package planner, and runtime-blueprint generator are all optics over that costate. Source files, HIR views, CGIR views, diagnostics, lock snapshots, generated outputs, and target runtimes are projections.

#### 22.3.1 Why a single memory-mapped graph file is attractive

The compiler's hot working set is metadata-heavy and pointer-rich. A memory-mapped project graph file with compiler-controlled layout gives three benefits that align well with the rest of the language design.

First, it gives the compiler stable identities and revisions across process restarts without rebuilding the entire world from text on every invocation. Second, it keeps hot summaries and edge tables in one place with explicit layout, which is much closer to the language's data-oriented ethos than a maze of process-local hash maps and cache directories. Third, it makes snapshots, replay, and direct tool attachment much simpler because every participant can agree on one project-graph revision.

The storage model should stay conservative and deterministic.

- one serial commit path,
- many readers,
- append-oriented mutation,
- fixed-format hot records,
- explicit schema versioning,
- periodic compaction,
- and crash recovery driven by journal + checkpoint rather than best-effort cache regeneration.

That discipline is especially valuable for a self-hosted compiler, where the compiler's own internal state becomes part of the language's operational contract.

#### 22.3.2 The graph should remain hotter than the artifacts

A project graph is not a generic blob store. Its job is to keep the metadata and dependency structure hot. Large immutable outputs are better referenced than embedded.

| Keep graph-native | Usually sidecar / content-addressed |
|---|---|
| source text chunks, interned strings, spans, syntax/HIR/CGIR nodes, summaries, diagnostics, module-interface summaries, watch state, runtime blueprints, target/profile metadata | object files, archives, debug symbol bundles, large baked assets, generated tables too large for hot metadata |

This keeps the mapped file small enough to stay operationally pleasant while preserving one authoritative graph.

#### 22.3.3 Many IRs stay manageable because they are projections, not rival truths

At this point in the book, the reader has seen a growing list of representations: source text, syntax, HIR, typed HIR, summary tables, CGIR, fused CGIR, interface artifacts, generated Rust, LLVM, benchmark identities, debug metadata, and project-graph projections. That can look like architectural drift unless the book says plainly what the relationship is.

The relationship is the same one introduced in Chapter 4:

> the compiler has many representations, but one authoritative graph.

The compiler therefore should not attempt to squash all phases into one mega-IR. Each phase has a different invariant and a different audience.

- source text preserves authored structure and comments,
- HIR preserves name and cursor meaning,
- summaries preserve legality facts,
- CGIR preserves optimization structure,
- backend forms preserve target-facing execution shape,
- interface and artifact forms preserve distribution and reuse boundaries.

What keeps this practical is shared identity and provenance. Node ids, region ids, type ids, artifact keys, and revision ids belong to the `Project` graph, not to one local pass. That is why self-hosting, distributed build, debugger attribution, and coding-agent workflows can all speak about “the same node” even when that node is being viewed through different projections.

#### 22.3.4 Compiler passes should be described as graph transactions

Earlier chapters already describe parser, lowering, summary construction, fusion, and artifact emission as optics over compiler-owned costates. The full project model makes the next step natural: each pass is a **transaction over `Project.build`**.

That means a pass has five recognizable stages.

1. select the affected graph region,
2. query the graph and compute derived facts,
3. synthesize replacements or derived projections,
4. validate the relevant invariants,
5. commit a new project revision.

This description is not just philosophical. It clarifies how several other parts of the language fit together.

- **incremental compilation** becomes dependency- and region-aware transaction invalidation;
- **distributed build** becomes scheduling over subgraph closures;
- **debugging and profiling** can attach to revisions and provenance sets rather than to transient pass-local data structures;
- **self-hosting** becomes more tractable because compiler passes are ordinary staged graph programs rather than privileged compiler-only magic.

#### 22.3.5 Why the direct protocol and the native query subset belong together

The direct graph protocol should therefore not be understood as merely a replacement for file watching. It is the transport for three closely related things:

- read-mostly project queries expressed in the ordinary optic subset,
- mutating compiler/tool transactions over `Project.build`,
- and projection/materialization requests for text, interfaces, artifacts, and crash or benchmark capsules.

This is one reason the book continues to insist that the projection filesystem is useful but secondary. Files are excellent human surfaces. Transactions, semantic queries, and provenance-aware revisions are better expressed over the graph directly.

### 22.4 Projection filesystems: useful, but secondary

Once the compiler graph is authoritative, a file tree becomes one compatibility surface rather than the semantic center.

A projection filesystem can mount selected views of the graph for tools that expect files:

```text
/optic/src/...          authored source text projection
/optic/hir/...          pretty or structured HIR projection
/optic/cgir/...         graph projection
/optic/diag/...         current diagnostics projection
/optic/pkg/...          package/workspace/build projection
/optic/gen/...          generated artifacts and interfaces
/optic/tool/<name>/...  tool-specific projections
```

This is a strong idea because it preserves compatibility with editors, grep-like tools, indexers, diff tools, and existing file-oriented workflows while keeping the graph itself primary.

But the filesystem surface should remain **secondary** for two reasons.

First, file protocols are weak. A write to a text file does not say whether the caller wants an atomic text patch, a semantic rename, a projection re-materialization, or a transaction over several related regions. Second, user-space filesystem layers inevitably add mediation overhead and liveness hazards that do not belong on the compiler's critical path.

So the right stance is:

- projection filesystem for compatibility and human convenience,
- direct graph protocol for primary tool integration,
- and ordinary exported text materialization for environments where mounting is unavailable or undesirable.

#### 22.4.1 Writable versus derived projections

Not every projection should be writable.

- authored text projections are writable;
- selected build-root values may be writable through structured edits;
- HIR, CGIR, diagnostics, and most summary projections are derived and read-only;
- generated outputs are re-materialized, not hand-edited.

This matters because the optic analogy should remain honest. A writable projection must have a clear reinsertion path. A read-only diagnostic stream does not.

### 22.5 Direct tool protocol over the graph

The primary integration surface for editors, language servers, build tools, test runners, indexers, and coding agents should be a concrete protocol over the compiler graph.

A good default is a local binary protocol over a Unix domain socket or platform equivalent, with batched requests and explicit graph revisions. The important design choice is not the exact wire format. It is the transactional model.

#### 22.5.1 Protocol principles

1. **Serial mutation, cheap reads.** One ordered commit path keeps the graph deterministic. Readers attach to revisions or snapshots.
2. **Batch by default.** A tool should be able to submit a text patch, ask for updated diagnostics, request affected summaries, and subscribe to a watch in one batch.
3. **Stable identities.** Nodes, projections, and revisions need durable ids so tools can correlate changes across requests.
4. **Capability-aware access.** A formatter, an indexer, and an AI agent do not need identical write powers.
5. **Projection-aware replies.** Tools can ask for text, structured data, materialized artifacts, or graph-native handles.

#### 22.5.2 Representative request families

| Request family | Purpose |
|---|---|
| session / capability negotiation | open workspace, negotiate protocol version, declare requested powers |
| text and patch operations | read or patch authored text projections |
| semantic queries | fetch HIR/CGIR/summaries/diagnostics/provenance |
| build and staging queries | evaluate package roots, build plans, stageable subgraphs, artifact plans |
| watch and invalidation | subscribe to revision deltas, changed projections, diagnostic updates |
| materialization | export source view, interface artifact, generated output, or reproducible workspace snapshot |
| explanation | ask why a node is dynamic, why a fusion failed, why a grade bound was violated |

#### 22.5.3 Why the protocol should be primary even if a projection filesystem exists

A direct protocol can express operations that files cannot express cleanly: graph revision selection, semantic explanation, batched transactions, stable node ids, structured diagnostics deltas, materialization policies, and provenance queries. It is also the natural place for AI-agent-friendly operations such as "apply this patch, tell me what invalidated, and return ranked next repairs".

That is why the projection filesystem should be treated as an adapter, not as the foundation.

### 22.6 The init tool should be graph-first and agent-aware

The first contact most users have with a language toolchain is not the optimizer or the debugger. It is the command that creates the project skeleton. That command therefore deserves architectural seriousness.

For Optic, the equivalent of `cargo init` or `go mod init` should not be a thin file copier wrapped around a handful of inert text templates. It should be a **guided project transaction** that creates the initial `Project`, `BuildRuntime`, `RuntimeBlueprint`, and `AppWorld` regions in a form the rest of the toolchain can immediately understand.

A first-party `optic init` should therefore do more than choose a package name. In human-oriented wizard mode it should help the author describe:

- the intended domain or blueprint family,
- the initial package/workspace topology,
- the expected `RuntimeFamily` and target profiles,
- the initial `AppWorld` hot data layout,
- the major host or foreign boundaries,
- the desired diagnostics, replay, and benchmark posture,
- and whether the project begins as a library, service, tool, mixed workspace, or subsystem.

The output of that interaction should be native Optic declarations and graph regions, not a second configuration vocabulary. At minimum the tool should be able to generate:

- `package` and `workspace` declarations,
- an initial `RuntimeBlueprint`,
- one or more `data AppWorld` skeletons,
- boundary-contract stubs for expected interop,
- benchmark and replay scaffolding with semantic `PerfKey`s,
- and procedural domain templates parameterized by the answers the user just gave.

This is where the earlier architecture pays off. The same domain blueprints that later guide optimizations and agent workflows can guide initialization. A service template, a browser-subsystem template, and a compiler-tooling template do not need separate template languages. They need one generator that emits ordinary language values and projections.

Just as importantly, the init path should be available in both **human mode** and **agent mode**. Interactive wizarding is the right default because most first projects are underspecified. But a headless mode should accept structured intent data and use the same graph transaction internally, so automation does not become a second, weaker entry point into the ecosystem.

The design consequence is broader than ergonomics. If the init tool emits native declarations, benchmark scaffolds, boundary stubs, and runtime-family metadata from the beginning, then large-codebase agents, package tools, and later self-hosted compiler components start from typed project intent instead of trying to infer it from boilerplate. That is exactly the kind of early structure that makes the rest of the language easier to automate without adding another special-purpose DSL.

### 22.7 Coding-agent-friendly diagnostics as a design inheritance

Compilers are also where the agent-friendly diagnostic architecture returns as a systems advantage. If the language is to host its own compiler and broader ecosystem tooling, its diagnostic story must already be good enough to support iterative automated development.

That is why the earlier discipline around stable codes, structured evidence, and ranked repairs should not be thought of as only "tooling". It is part of the language's self-hosting readiness.

### 22.7.1 A repository agent operating system should be treated as a first-party graph client

Once the project graph, direct protocol, init flow, and diagnostic schema exist, a repository-local agent operating system stops looking like an external convenience layer and starts looking like another serious graph client. The same graph that supports editors, debuggers, package tools, and self-hosted compiler passes can support a coordinating agent, a small family of specialists, explicit work packets, shared and role-local memory, and generated context indexes. The important design rule is that this agent layer should **reuse** the graph-native tooling story rather than inventing a second semantics for automation.

That is why the repository package accompanying this book ships a canonical `AGENTS.md`, tool-specific wrappers, specialized agent files, explicit memory ledgers, and a maintenance script that regenerates context indexes from checked-in state. The files are not an afterthought. They are a worked example of how graph-native tooling, agent-oriented diagnostics, and direct protocol thinking can be turned into a repeatable collaboration system for both humans and coding agents. Appendix K records that operating system in compact reference form.


### 22.7.2 Durable agent memory belongs in the graph; scratch memory does not

The repository agent operating system is strongest when it shares the same semantic center as the compiler and tooling. That does **not** mean every trace of agent activity should become graph truth. It means the durable, shared, query-worthy subset of maintenance knowledge should be represented alongside the rest of the project graph.

Good graph-native candidates include:

- accepted architectural decisions,
- validated diagnostic-repair records,
- benchmark and replay explanations,
- task state that spans revisions,
- boundary exceptions and local audit notes,
- and other facts that later humans or agents genuinely need to query.

Poor graph-native candidates include raw scratchpads, long transcripts, speculative notes, and tool-specific hidden state. Those belong in sidecars or private tool memory because the project graph should remain a typed semantic database, not a dump of every conversation the repository ever triggered.

### 22.7.3 Failed patches are first-class negative knowledge

One category deserves explicit treatment: prior failed patches. Large codebases waste enormous effort retrying similar bad ideas because the only record of failure lives in old chat threads, abandoned branches, or one person's memory.

The right response is to treat failed patches as **typed negative knowledge**. A failed patch record should say what was attempted, which nodes or regions it touched, which revision and target profile it was evaluated against, which evidence proved the failure, and whether the result is still believed to matter. It should never become a blanket “never do this again” rule.

That gives the toolchain and future agents a much better default. Query→fix synthesis can de-rank repair candidates that already failed for the same reason, while human review can inspect the evidence rather than trusting folklore. Appendix K records the repository-local file-level version of this idea; the long-range `ProjectGraph` version is to keep the durable summary in the graph and the large patch diff or transcript in sidecar storage.

### 22.8 Governance and invariants

The book closes the main text with a governance point because ambitious systems languages fail as often by losing their discipline as by losing their semantics.

Three governance rules matter most.

- No feature enters the language without a lowering story.
- No optimization enters the compiler without a provenance and benchmark story.
- No milestone is declared complete without repository evidence.

A mature language also needs explicit policy for the parts that are easy to postpone and hard to retrofit: editions, source compatibility, package compatibility, module artifacts, binary interfaces, migration tooling, and debugger/profiler stability through optimization. Those pressures are common late-stage failure points in other ecosystems, and they are gathered into a concrete maturity chapter in Chapter 27 and a standing proposal checklist in Chapter 28.

### 22.9 Final transition

The appendices that follow are not leftovers. They are the working reference material that turns the book's conceptual narrative back into day-to-day implementation practice.

---

### 22.10 Detailed implementation reference: compiler passes as optics over explicit IR costates

Compilers are not merely a future bootstrap target; they are also one of the clearest demonstrations that the optic model handles richly structured transformation systems. The detailed examples below restore that perspective.

A compiler is a program that transforms programs. Every compiler pass is naturally an optic over a typed IR costate.

#### 22.9.1 The Compiler IR as Costate

```rust
data Module {
    functions:    SoA<FunctionDef>,
    instructions: SoA<Instr>,         -- flat instruction arena
    types:        SoA<TypeInfo>,
    dominators:   SoA<DomTreeNode>,   -- pre-computed for analysis passes
    use_def:      SoA<UseDefChain>,   -- pre-computed use-def chains
    metadata:     SoA<PassMetadata>,  -- per-pass scratch storage
}
```

#### 22.9.2 Dead Code Elimination as a Traversal + Prism

```rust
optic DeadInstr: GradedPrism<Module, Instr,
    CacheGrade<2> + CompileTimeGrade>
{
    preview m => {
        let instr = m.instructions[m.id];
        if m.use_def[m.id].uses.is_empty() && instr.has_no_side_effects() {
            Some(instr)
        } else {
            None
        }
    }
    review  instr => instr
}

-- DCE pass:
module
    .query(AllInstrs *** DeadInstr)
    .map(|(all, dead)| all.remove(dead))
    .drive();
```

#### 22.9.3 Constant Folding as Lens + Map

```rust
optic ConstFold: GradedOptic<Module, Instr, CacheGrade<2> + CompileTimeGrade> {
    get  m => {
        let instr = m.instructions[m.id];
        match instr {
            Instr::BinOp(op, Const(a), Const(b)) => Instr::Const(eval_op(op, a, b)),
            other => other,
        }
    }
    put  (m, folded) => { m.instructions[m.id] = folded; }
}
```

#### 22.9.4 Register Allocation as Optic Over Live Ranges

Register allocation is the assignment of virtual registers (unbounded) to physical registers (bounded, 16 on x86-64). The optic model:

```rust
data LiveRangeArena {
    ranges:    SoA<LiveRange>,    -- (virtual_reg, start_point, end_point)
    conflicts: SoA<ConflictSet>,  -- which virtual regs interfere
    colors:    SoA<Option<Reg>>,  -- assigned physical register (None = spill)
}

optic ColorRegister: GradedPrism<LiveRangeArena, Reg,
    CacheGrade<3> + CompileTimeGrade>
{
    preview lra => {
        let range = lra.ranges[lra.id];
        let available = ALL_REGS - lra.conflicts[lra.id].used_regs();
        available.first()
    }
    review  reg => reg
}
```

When `ColorRegister.preview` returns `None`, the register is spilled to the stack. The `None` path is itself an optic:

```rust
optic SpillToStack: GradedOptic<LiveRangeArena, StackSlot, CacheGrade<1>> {
    get  lra => lra.stack_frame.allocate_slot(sizeof(lra.ranges[lra.id].type_))
    put  (lra, slot) => { lra.colors[lra.id] = None; lra.stack_slots[lra.id] = slot; }
}

let alloc_pass = ColorRegister >>> SpillToStack;
-- If ColorRegister succeeds (Some(reg)), SpillToStack is skipped (prism law)
-- If ColorRegister fails (None), SpillToStack handles the spill
```

This is register allocation expressed as composable optics. The grade `CompileTimeGrade` ensures the entire allocation pass runs at compile time, not at runtime of the compiled program.

---

### 22.11 Detailed implementation reference: self-hosting ladder and differential trust loop

The self-hosting plan belongs next to the tooling chapter because it is a discipline of validation, not just an implementation milestone. The following material records the explicit trust-chain and bootstrap-ladder guidance needed to keep that discipline honest.

#### 22.10.1 Bootstrapping ladder

| Stage | Compiler impl | Output |
|-------|--------------|--------|
| S0 | Rust prelude compiler | Rust source |
| S1 | Optic front-end libraries compiled by Rust | Rust source |
| S2 | Optic parser, HIR, diagnostics written in Optic | Rust source |
| S3 | Optic type checker, CGIR, optimizer written in Optic | Rust source |
| S4 | Mixed: most passes in Optic, small Rust shell | Rust + native |
| S5 | Fully self-hosted with native backend underway | Native + Rust fallback |

#### 22.10.2 Compiler passes as optics

Every compiler pass is itself an optic over a compiler costate:

| Pass | Costate | Focus |
|------|---------|-------|
| Parser | `SourceFile` | `Ast` |
| HIR lowering | `HirArena` | `HirItem` |
| Type check | `TypeckCtx` | `TypedHirItem` |
| CGIR construction | `CgirGraph` | `CgirNode` |
| Fusion | `CgirGraph` | `CgirNode` (rewrite) |
| Codegen | `RustAstArena` | `RustItem` |

This is only valuable once the prelude IR and summary machinery are robust. Do not force early self-description.

#### 22.10.3 Trust chain

- Reproducible builds of the Rust prelude compiler
- Golden snapshot suites shared across Rust and Optic implementations
- Differential testing between Rust-hosted and Optic-hosted compilers
- Stable diagnostic codes across both implementations
- A frozen bootstrap seed for release lines

#### 22.10.4 Self-hosting exit criteria

##### 22.10.4.1 Differential validation loop

Self-hosting should be treated as a translation-validation problem, not as a declaration of maturity. The minimum practical loop is:

```text
1. compile compiler sources with seed Rust implementation
2. compile same sources with Optic implementation
3. compare diagnostics JSON for known-good and known-bad suites
4. compare generated Rust for canonical examples
5. compare benchmark deltas against seed tolerances
6. only then consider the Optic compiler a valid successor for that revision
```

This keeps self-hosting grounded in observable equivalence rather than in the prestige of saying the compiler is written in its own language.

The compiler may be called self-hosting only when:

- It can compile itself from a clean checkout
- The generated compiler passes the same regression and benchmark suite as the seed
- Diagnostics remain stable enough for coding agents to work against either implementation
- The trusted Rust shell is small, audited, and shrinking

---

