## Appendix J — Native Project Queries, Graph Transactions, and the Semantic Query Engine

This appendix records the graph-query and graph-transaction story in reference form. It does not introduce a second language. Instead, it shows how the authoritative `Project` graph becomes queryable, rewritable, and distributable while still reusing the ordinary optic model established in the main text.

Under the closure rule in §27.17 and the standing proposal review in Chapter 28, native project queries remain a **core** facility precisely because they reuse ordinary optics; the richer internal QIR remains an implementation detail and therefore never becomes a second user-facing language.

### J.1 One semantic graph, many projections

The query system makes sense only if it inherits the same architectural rule the rest of the compiler already follows: one authoritative graph, many projections, no second semantic center. Chapter 4 established that graph-as-costate principle at the scale of the whole project; this appendix turns that architectural claim into a query and transaction reference.

The project graph is the semantic database of the language. Source text, syntax, HIR, summaries, CGIR, interfaces, diagnostics, artifacts, benchmark metadata, and runtime blueprints are not rival truths. They are projections over one durable graph revision.

The critical rule is therefore:

> if a fact matters to legality, optimization, diagnostics, staging, build planning, or reproducibility, it should be queryable from the Project graph as structured data.

That includes at least:

- node kind and stable ids,
- `OpticSummary` fields,
- `RegionSet` memberships,
- grades and bounds,
- determinism and stageability,
- provenance and fused ancestry,
- boundary contracts,
- dependency edges,
- interface and artifact identity,
- diagnostic records and ranked repairs.

### J.2 Native Project queries as an ordinary optic subset

The source-facing query model should remain inside the language. That includes the durable, shared subset of maintenance knowledge: decisions, validated repair records, and advisory failed-patch records should be queryable through ordinary project roots rather than hidden behind tool-specific history panes.

Project queries are therefore expressed as ordinary optic programs rooted at stable graph projections such as:

- `ProjectSource`
- `ProjectSyntax`
- `ProjectHir`
- `ProjectSummaries`
- `ProjectCgir`
- `ProjectInterfaces`
- `ProjectDiagnostics`
- `ProjectArtifacts`
- `ProjectDependencies`
- `ProjectExperimental`
- `ProjectAgentMemory`
- `ProjectFailedPatches`

A representative query:

A repository-local agent operating system can use exactly the same subset. In practice that means task decomposition, coherence checks, fix synthesis, and release assembly can all be expressed as ordinary project queries plus graph transactions rather than as a second automation DSL. The split book package that accompanies this manuscript uses that idea directly; Appendix K records the resulting file-level operating model.

```rust
pub fn fusion_candidates(p: &[SharedGrade] Project) -> List<NodeId> {
    p.query(
        ProjectCgir
        >>> Nodes
        >>> Filter(|n| n.kind == NodeKind::Compose)
        >>> Filter(|n| !n.flags.fusion_blocked)
        >>> Filter(|n| n.summary.get_grade.cache < 8)
        >>> Map(|n| n.id)
    ).get()
}
```

This subset should remain:

- typed,
- deterministic,
- mostly read-only,
- bounded enough for compile-time use,
- and ordinary in its surface rules.

It should not become a second full user-facing query language with an unrelated parser, type checker, and migration story.

### J.3 Read-only queries versus graph transactions

The previous section describes read-mostly project inspection. Compiler phases and tool actions need one extra capability: they must commit graph revisions.

The right model is:

> read-mostly project queries and mutating compiler passes reuse the same optic substrate, but mutating phases are graph transactions rather than plain queries.

A graph transaction has the common shape:

```text
select graph regions
  -> analyze and derive facts
  -> synthesize replacements or materialized projections
  -> validate invariants
  -> commit a new revision
```

Examples:

| Transaction | Reads | Writes |
|---|---|---|
| parser | text projection | syntax projection |
| HIR lowering | syntax projection | HIR projection |
| summary builder | typed HIR | summary table |
| CGIR construction | HIR + summaries | CGIR projection |
| fusion | CGIR | fused CGIR + provenance links |
| interface emission | summaries + CGIR + target profile | interface artifacts |
| build planning | package/workspace roots + target profile | build-plan artifacts |

This is the compact operational summary of the compiler pipeline as described throughout the book. The same model also covers project initialization: `optic init` should be understood as an early graph transaction that writes build roots, runtime blueprints, `AppWorld` scaffolds, and procedural template projections rather than as a separate boilerplate generator.

### J.4 Internal QIR remains an implementation detail

The internal query engine may still lower project queries and graph transactions into a richer internal Query IR.

A practical internal shape is:

```text
QIR ::= Scan(source)
      | Filter(q, predicate)
      | Project(q, fields)
      | Join(q1, q2, condition)
      | Paths(root, condition)
      | Explain(node)
      | Why(node, property)
      | Repair(goal, node)
```

QIR exists so the compiler and tools can optimize queries, use indexes, prune paths, batch requests, and synthesize fixes efficiently. Experimental roots such as `ProjectExperimental` lower through the same mechanism; they do not get a second query language or a special non-graph protocol.

The book's design rule is that QIR should remain **internal**. The user-facing model is still the ordinary optic subset over `Project` roots. That prevents the language from drifting into “normal code plus an unrelated query language.”

### J.5 Query execution and indexes

The semantic query engine should execute against graph-native indexes rather than against repeated full scans wherever possible.

Useful indexes include:

- `RegionIndex` — region → readers and writers,
- `GradeIndex` — grade buckets, bounds, and violations,
- `KindIndex` — node kind → node ids,
- `DependencyIndex` — node → dependents and closure roots,
- `ProvenanceIndex` — source span → nodes,
- `ArtifactIndex` — artifact key → graph region and projection metadata.

A minimal optimizer should perform:

1. predicate pushdown,
2. projection pruning,
3. index selection,
4. path pruning,
5. limit-aware early termination,
6. cached query reuse by `(query, revision)` hash.

This makes coding-agent workflows and IDE integration fast enough to rely on the graph directly rather than on repeated text scraping.

### J.6 Query-to-fix synthesis

Because diagnostics already carry structured evidence and ranked repairs, the next step is to let the toolchain synthesize fixes from graph facts rather than only from hand-written diagnostic templates.

A repair request is best understood as a constrained graph transformation:

```text
repair { goal grade.cache <= 4; node pipeline }
```

becomes:

- identify the grade-producing subgraph,
- enumerate minimal legal transformations,
- score them by locality, safety, and churn,
- emit a graph patch plus projection edits,
- revalidate against the original constraint.

Typical repair families:

- split a composition,
- insert a stage boundary,
- reorder a product,
- relax a declared bound,
- convert a writer to read-only,
- wrap an unsafe boundary behind a safer optic.

The important language-wide point is that fix synthesis remains graph-native and provenance-aware. It does not guess from text alone.

### J.7 External frontends target the graph, not a parallel IR

The same project graph can host external-language ingestion, experimental mathematics arenas, agent memory, and advisory failed-patch knowledge without creating a second query language.

The rule is:

> external languages may project into the Project graph, but they do not redefine its guarantees.

That yields three extension roles.

1. **Frontends** — C/C++, Rust, or other languages lower into HIR/CGIR plus conservative summaries.
2. **Backends/projections** — CGIR lowers to Rust, LLVM, or other artifact families.
3. **Tooling clients** — editors, profilers, debuggers, and agents query the graph directly.

A frontend contract should guarantee:

- conservative `RegionSet` completeness,
- explicit determinism classification,
- stable provenance mapping,
- a declared capability tier (`opaque`, `regions-only`, `summary-complete`, and so on).

This is what allows mixed-language projects to participate in one semantic toolchain rather than merely coexisting beside it.

### J.8 Distributed build over graph transactions

Once the project graph is canonical, distributed compilation also becomes graph-shaped rather than file-shaped.

A build task is the transitive closure of a subgraph rooted at one exported node or artifact request:

```text
BuildTask = { root, closure, summaries, target_profile, artifact_key }
```

The scheduler operates over the dependency DAG of these tasks. Cache keys are computed from:

- subgraph hash,
- interface hashes,
- target profile,
- relevant staged inputs,
- runtime-family and capability assumptions.

This gives language-agnostic distributed builds because all frontends converge to the same graph substrate.

### J.9 Persistent graph store and direct protocol

The project graph should remain persistent and cheaply queryable. The graph-store design in Appendix H already motivates a single mmap-friendly authoritative file with append-oriented mutation, journaled commits, and fixed hot records. That same structure is what makes native project queries and graph transactions practical.

The direct protocol should therefore be the primary tool surface, with the projection filesystem remaining a compatibility adapter.

Representative protocol families:

- session and capability negotiation,
- read/query over a chosen graph revision,
- patch text or patch graph transaction,
- explain / why / repair,
- subscribe to revision deltas,
- materialize projections or reproducible capsules.

The projection filesystem remains valuable for grep, diff, and file-oriented editors, but the semantic center stays with the graph and the direct protocol.

### J.10 One-sentence summary

The long-range toolchain direction can be stated compactly:

> the Project graph is not only the compiler's internal state; it is the language's semantic database, query surface, transactional build graph, and tooling protocol root.

