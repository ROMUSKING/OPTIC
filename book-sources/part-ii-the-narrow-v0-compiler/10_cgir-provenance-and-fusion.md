## 10. CGIR, Provenance, and Fusion

### 10.1 Why CGIR exists above SSA

SSA is excellent once the compiler has already committed to a lower-level control-flow structure. It is a poor place to *discover* optic composition, product adjacency, or provenance. CGIR exists because the language wants its own structure to remain explicit until that structure has paid out its optimization and diagnostic value.

Seen historically, this is a deliberate defense against a recurring compiler failure mode: lower away the language’s structure too early, then spend the rest of the optimizer trying to reconstruct what the source already knew. CGIR is a recovered paradigm in that sense. It keeps the graph-shaped structure that older optimizer pipelines often flattened into generic control flow before fusion legality, provenance, and domain intent had been fully harvested.

In other words: CGIR is not a fancy pretty-printer for the AST. It is the implementation form of the language.

### 10.2 Core node families

#### 10.2.1 A concrete CGIR catalog

```text
CgirNode =
  | OpticLeaf  { id, name, costate, focus, summary, get_fn, put_fn, provenance }
  | Compose    { id, lhs, rhs, grade, provenance }
  | Product    { id, lhs, rhs, grade, alias_safe, provenance }
  | QueryGet   { id, optic, costate, cursor, provenance }
  | QuerySet   { id, optic, costate, cursor, value, provenance }
  | QueryMap   { id, optic, costate, cursor, map_fn, provenance }
  | FusedLoop  { id, original_ids, costate, body, provenance }
  | Tap(...) | Coinductive(...) | Stage(...) | Record(...)   // reserved in v0
```

#### 10.2.2 Invariants the verifier must enforce

| Invariant | Consequence if violated |
|---|---|
| node ids are unique | snapshots and provenance become unstable |
| `Compose.lhs.focus == Compose.rhs.costate` | fusion may synthesize invalid loops |
| `Product` children share costate type | generated loop would read different arenas as one |
| `alias_safe` is true before codegen | product lowering becomes unsound |
| fused loops name at least two originals | provenance is lying about optimization |

The prelude CGIR needs only a small node catalog, but each node must carry real semantic weight.

| Node family | Purpose |
|---|---|
| `OpticLeaf` | named, summarized optic definition |
| `Compose` | sequential optic composition |
| `Product` | same-costate product composition |
| `QueryGet`, `QuerySet`, `QueryMap` | action roots over a costate |
| `FusedLoop` | post-optimization materialized loop body |

Reserved future nodes such as `Tap`, `Coinductive`, `Stage`, and `Record` should exist in the type universe even if the prelude rejects them in source. Reserving structure early avoids a later IR rupture.

### 10.3 Construction rules and why they are bottom-up

CGIR is built bottom-up from typed HIR because the legality checks are compositional.

- A named optic becomes an `OpticLeaf` populated from its summary.
- `A >>> B` becomes `Compose(A, B)` after checking focus-costate compatibility.
- `A *** B` becomes `Product(A, B)` after checking shared costate type and alias safety.
- Query actions become graph roots because they define how the optic graph is actually run.

This bottom-up shape mirrors the book's broader design rule: do not flatten structure earlier than necessary.

### 10.4 Provenance is a semantic requirement, not a debugging afterthought

One of the critique's strongest warnings was that aggressive fusion can destroy debuggability if provenance is treated casually. This book accepts that warning as a design requirement.

Every node in CGIR must have non-dummy provenance. Every fused node must carry the union of original node ids and spans. Every codegen path must preserve enough of that provenance that a profiler or diagnostic can still name the source optics involved.

The reason this is so important is simple: if the language's unit of reasoning is the optic, then the implementation must preserve optics as the unit of explanation even after it stops preserving them as separate runtime loops.

### 10.5 The three prelude fusion passes

#### 10.5.1 Fixed-point driver

```text
optimize(graph):
  changed = true
  iters = 0
  while changed and iters < 8:
    changed = false
    graph, c1 = map_fusion(graph)
    graph, c2 = compose_fusion(graph)
    graph, c3 = product_flatten(graph)
    changed = c1 or c2 or c3
    verify(graph)
    iters += 1
  return graph
```

#### 10.5.2 Why the pass order is deliberate

Map fusion runs first because it removes trivial intermediate structure without changing graph edges. Compose fusion then sees cleaner escape patterns. Product flattening runs last because it is primarily a canonicalization pass for the backend.

The prelude optimizer stays intentionally small.

That smallness should not be misread as an attempt to replace a mature SSA optimizer. CGIR’s job is narrower and more specific: preserve optic structure long enough to justify structural fusion, provenance retention, and summary-driven legality. Once that work is complete, the native backend is expected to take over ordinary scalar and target-level optimization rather than forcing CGIR to imitate decades of SSA engineering.

#### 10.5.3 Map fusion

Chained pure maps over the same query root collapse into one map. The gain is reduced intermediate value traffic and clearer generated code.

#### 10.5.4 Compose fusion

When an intermediate focus does not escape, a sequential composition can collapse into one loop body. This is the first place where the language demonstrates a real zero-cost abstraction claim.

#### 10.5.5 Product flattening

Nested products normalize into a flatter internal representation so codegen does not spend its budget on nested tuple noise.

### 10.6 Why fixed-point optimization is enough in v0

The prelude optimizer should be a tiny fixed-point engine over a very small pass set. That is the right level of ambition.

A big rewrite system would create two problems at once.

- It would make the legality story harder to audit.
- It would make performance regressions harder to localize.

By contrast, a fixed pass order over a small node set gives the project something it badly needs early: predictable diffs.

### 10.7 A representative fused loop story

Suppose the source says:

```rust
entities.query(HealthView *** PositionView).map(|(h, p)| damage(h, p));
```

CGIR first makes the product explicit. Fusion then proves that both fields can be read in one pass and the update can be written back without alias conflict. The backend no longer sees "a library abstraction over two optics". It sees a single loop with two field loads, one transformation, and two stores.

That is the whole point of keeping CGIR above SSA long enough for the optic structure to matter.

### 10.8 Transition

Once CGIR is fused and canonicalized, the last prelude question is whether the backend can make the abstraction disappear in readable code. The answer must be yes before the language moves on.

### 10.9 Detailed implementation reference: CGIR catalog, construction, and verifier rules

The next sections are the precise node-level reference for CGIR. They specify graph container shape, node variants, builder behavior, and the invariants that `optic dump-cgir --check` must enforce.

The Coalgebra Graph IR is the compiler's optimization boundary. Every optimization pass operates on the CGIR and must preserve the provenance links that connect each node to source spans.

#### 10.9.1 Design principles

- **Explicit provenance on every node.** A node without a source span is a compiler bug.
- **No implicit sharing.** Each CGIR node has a stable `NodeId: u32` assigned at construction. Nodes are not deduplicated silently.
- **Immutable construction.** The initial CGIR is constructed from typed HIR and is never mutated in place. Each optimization pass produces a new CGIR from the old one.
- **Inspectable at every stage.** `optic dump-cgir --node X` must work on both pre- and post-fusion CGIR.

#### 10.9.2 Node catalog

##### 10.9.2.1 Supporting types and graph container

The node catalog is easier to implement if the graph container itself is explicit.

```rust
pub type NodeId = u32;

pub struct CgirGraph {
    pub nodes: IndexVec<NodeId, CgirNode>,
    pub roots: Vec<NodeId>,
    pub provenance_index: BTreeMap<NodeId, FusionProvenance>,
}

pub struct FusionProvenance {
    pub original_ids: Vec<NodeId>,
    pub spans: Vec<SourceSpan>,
    pub reason: FusionReason,
}

pub enum FusionReason {
    MapFusion,
    ComposeFusion,
    ProductFlattening,
}

pub enum Determinism {
    Pure,
    Seeded,
    Recorded,
    Opaque,
}
```

Even in v0, storing a structured determinism enum is worth it. The prelude may only exercise `Pure` and `Opaque`, but reserving the full shape now prevents a later refactor when replay and recorded inputs appear.

##### 10.9.2.2 Canonical node-shape rules

A CGIR builder should enforce the following canonical forms before optimization begins:

- `Paren` nodes from HIR do not survive into CGIR; their spans are merged into child provenance.
- Query nodes are graph roots; they are never nested under other query nodes.
- `Compose` and `Product` nodes refer only to non-query child nodes.
- `OpticLeaf.get_fn` and `put_fn` contain only normalized cursor forms, not arbitrary source syntax.
- Reserved future nodes may exist in the enum but are never constructed in v0.

Canonical forms shrink the optimizer surface area and make diff-based debugging much easier.

```text
CgirNode =
  -- Leaf nodes
  | OpticLeaf {
      id:         NodeId,
      name:       Symbol,
      costate:    TypeRef,
      focus:      TypeRef,
      grade:      ConcreteGrade,
      get_fn:     CgirExpr,
      put_fn:     CgirExpr,
      summary:    OpticSummary,
      provenance: SourceSpan,
    }

  -- Composition nodes
  | Compose {
      id:         NodeId,
      lhs:        NodeId,
      rhs:        NodeId,
      grade:      ConcreteGrade,       -- semiring product of lhs and rhs grades
      provenance: SourceSpan,
    }
  | Product {
      id:         NodeId,
      lhs:        NodeId,
      rhs:        NodeId,
      grade:      ConcreteGrade,       -- semiring sum of lhs and rhs grades
      alias_safe: bool,                -- set by alias checker, never assumed
      provenance: SourceSpan,
    }

  -- Query nodes (attached to a costate variable)
  | QueryGet {
      id:         NodeId,
      optic:      NodeId,
      costate:    CgirExpr,            -- the arena variable
      cursor:     Symbol,              -- fresh cursor name
      provenance: SourceSpan,
    }
  | QuerySet {
      id:         NodeId,
      optic:      NodeId,
      costate:    CgirExpr,
      cursor:     Symbol,
      value:      CgirExpr,
      provenance: SourceSpan,
    }
  | QueryMap {
      id:         NodeId,
      optic:      NodeId,
      costate:    CgirExpr,
      cursor:     Symbol,
      map_fn:     CgirExpr,            -- (focus) -> focus
      provenance: SourceSpan,
    }

  -- Fusion artifacts (only appear post-optimization)
  | FusedLoop {
      id:           NodeId,
      original_ids: Vec<NodeId>,       -- all nodes that were fused into this
      costate:      CgirExpr,
      body:         CgirExpr,          -- the merged loop body
      provenance:   FusionProvenance,  -- span union of all originals
    }

  -- Reserved future nodes (present in node type but rejected in v0)
  | Tap(...)         -- observability tap (reserved)
  | Coinductive(...) -- reactive loop (reserved)
  | Stage(...)       -- partial evaluation boundary (reserved)
  | Record(...)      -- DST recording (reserved)
```

Any attempt to construct a reserved node in v0 emits `OPT-3xx` and halts CGIR construction.

#### 10.9.3 CGIR expressions (`CgirExpr`)

```text
CgirExpr =
  | Var(Symbol)
  | FieldAccess(CgirExpr, Symbol)         -- expr.field
  | IndexAccess(CgirExpr, CgirExpr)       -- expr[idx]
  | CursorField(Symbol, CursorField)      -- cursor.arena | cursor.id
  | Assign(CgirExpr, CgirExpr)           -- lhs = rhs (in put bodies)
  | Call(Symbol, Vec<CgirExpr>)          -- helper call
  | Tuple(Vec<CgirExpr>)                 -- (a, b, ...)
  | TupleGet(CgirExpr, usize)            -- expr.N
  | Lit(Literal)                          -- integer/float literal
  | BinOp(BinOp, CgirExpr, CgirExpr)
  | Closure(Vec<Symbol>, Box<CgirExpr>)  -- |params| body
  | Block(Vec<CgirStmt>, Box<CgirExpr>)  -- { stmts; expr }
```

#### 10.9.4 CGIR construction rules

##### 10.9.4.1 Builder driver and invalid-node propagation

```text
build_cgir(typed_hir_program):
  graph = empty_graph()
  for item in typed_hir_program.items:
    if item is named optic:
      build_named_optic(graph, item)
    elif item is let-bound optic expression:
      build_root_expr(graph, item.expr)
    elif item is query root:
      root_id = build_query_root(graph, item)
      graph.roots.push(root_id)
  return graph

build_root_expr(graph, expr):
  match expr:
    Named(name)      -> make_leaf(graph, summary_table[name])
    Compose(l, r)    -> build_compose(graph, build_root_expr(graph,l), build_root_expr(graph,r))
    Product(l, r)    -> build_product(graph, build_root_expr(graph,l), build_root_expr(graph,r))
    ErrorExpr        -> make_invalid(graph)
```

Every builder entry point returns either a valid `NodeId` or an `Invalid` placeholder node that carries the emitted diagnostic id. Invalid nodes allow the compiler to continue collecting errors in a single run while still preventing optimization and codegen from operating on broken graphs.

CGIR is constructed from typed HIR in a single bottom-up traversal. Construction rules:

```text
construct(HirOptic::Named(name)) ->
  OpticLeaf { ... fields from OpticSummary table ... }

construct(HirOptic::Compose(lhs, rhs)) ->
  let l = construct(lhs)
  let r = construct(rhs)
  -- type check: l.focus == r.costate, else TYP-201
  let grade = combine_seq(l.grade, r.grade)
  -- grade bound check: if grade > declared bound, emit GRA-104
  Compose { lhs: l.id, rhs: r.id, grade, ... }

construct(HirOptic::Product(lhs, rhs)) ->
  let l = construct(lhs)
  let r = construct(rhs)
  -- costate must match: l.costate == r.costate, else TYP-201
  let grade = combine_par(l.grade, r.grade)
  let safe = alias_check(l.summary, r.summary)
  if !safe: emit ALI-201
  Product { lhs: l.id, rhs: r.id, grade, alias_safe: safe, ... }
```

If any construction step fails, the CGIR node is marked `Invalid` and construction continues to collect further errors. An `Invalid` node must never be passed to the optimizer or codegen.

#### 10.9.5 CGIR invariants

##### 10.9.5.1 Verifier algorithm

The invariant checker should be a first-class command, not an internal assertion bundle. A simple verifier pass is enough for v0:

```text
verify(graph):
  check_unique_node_ids(graph)
  check_root_ids_exist(graph)
  dfs_check_acyclic(graph)
  for node in graph.nodes:
    match node:
      Compose(lhs, rhs):
        require focus(lhs) == costate(rhs) else CGI-410
      Product(lhs, rhs, alias_safe):
        require costate(lhs) == costate(rhs) else CGI-410
        require alias_safe == true else CGI-410
      FusedLoop(original_ids):
        require len(original_ids) >= 2 else CGI-410
      _:
        pass
    require provenance(node) != Span::DUMMY else CGI-410
```

The verifier is the compiler's internal consistency oracle. Any pass that produces a graph that fails `verify` has not merely discovered a user error; it has violated a compiler contract.

These invariants must be checked by `optic-cgir`'s invariant checker (`optic dump-cgir --check`):

1. Every node has a unique `NodeId`.
2. Every `Compose` node's `lhs.focus == rhs.costate`.
3. Every `Product` node's `lhs.costate == rhs.costate`.
4. No `Invalid` nodes are present after type checking completes.
5. `alias_safe` on every `Product` node is `true` (alias checker ran and passed).
6. No cycles in the node reference graph.
7. Every `FusedLoop` node lists at least two `original_ids`.
8. `provenance` is never `Span::DUMMY` on a non-fused node.

---

### 10.10 Detailed implementation reference: fusion laws, fixed-point driver, and provenance obligations

Fusion is where the book’s promises become executable. The material below makes every rewrite concrete: pattern, precondition, rewrite shape, provenance retention, and the conditions under which a blocked fusion becomes a deliberate diagnostic instead of a silent miss.

The v0 optimizer implements exactly three fusion passes, applied in order. Each pass has named preconditions; if any precondition fails, the pass is skipped for that node and emits a `FUS-5xx` diagnostic if the failure was unexpected.

#### 10.10.1 Pass order

```text
Pass 1: Map fusion          (eliminates intermediate focus values between chained maps)
Pass 2: Compose fusion      (merges sequential optic loops into a single loop body)
Pass 3: Product flattening  (normalizes nested products into flat tuples)
```

Passes are applied to a post-type-check CGIR and produce a new CGIR. The original is preserved for provenance and `dump-cgir --before-fusion` inspection.

#### 10.10.2 Pass 1: Map fusion

##### 10.10.2.1 Map-fusion algorithm

```text
map_fusion(graph):
  changed = false
  for root in graph.roots:
    walk postorder(root):
      if node matches QueryMap(QueryMap(seed, f), g)
         and same_costate_and_optic(seed)
         and pure(f) and pure(g)
         and not captures_escape(f, g):
           replace node with QueryMap(seed, compose_closures(f, g))
           record_fusion(MapFusion, originals=[inner, outer])
           changed = true
  return (graph, changed)
```

Purity here means: no arena mutation, no host calls, no opaque builtins. The prelude should be explicit about this rather than silently assuming lambdas are harmless.

**Pattern:**

```text
QueryMap(QueryMap(costate, optic, cursor_a, f), optic, cursor_b, g)
```

**Precondition:** Both maps apply to the same optic on the same costate, and neither map body captures variables that escape into the outer context.

**Rewrite:**

```text
QueryMap(costate, optic, cursor_c, |x| g(f(x)))
```

**Provenance:** The `FusedLoop` node carries both original `QueryMap` spans. The generated Rust loop gets a `// fused: [Map1, Map2]` comment.

**Rewrite rule statement (for the test suite):**

```text
-- Map fusion law:
query(o).map(f).map(g)  ≡  query(o).map(x => g(f(x)))
-- provided f and g are pure (no arena side effects)
```

#### 10.10.3 Pass 2: Compose fusion

##### 10.10.3.1 Compose-fusion algorithm

```text
compose_fusion(graph):
  changed = false
  for root in graph.roots:
    walk postorder(root):
      if node matches QueryMap(costate, Compose(a,b), map_fn)
         and deterministic(a) and deterministic(b)
         and not intermediate_escapes(node)
         and compatible_for_single_loop(a,b):
           fused = make_fused_loop(costate, [a,b,node], synthesize_fused_body(a,b,map_fn))
           replace node with fused
           changed = true
  return (graph, changed)
```

`synthesize_fused_body` may still introduce a register-resident temporary for the outer focus. Fusion only promises elimination of *heap* or *loop-level* intermediates, not a mystical absence of temporaries altogether.

**Pattern:**

```text
QueryMap(costate, Compose(A, B), cursor, f)
```

**Precondition:**
1. Neither A nor B is `Invalid`.
2. `A.focus == B.costate` (type compatibility; already checked).
3. The intermediate focus value (A's output / B's input) does not escape into any outer binding.
4. Both A and B have `determinism = Deterministic`.

**Rewrite:**

```text
FusedLoop {
  original_ids: [A.id, B.id, QueryMap.id],
  body: |cursor| {
    let intermediate = A.get(cursor);
    let result = B.get(intermediate);   -- B operates on intermediate, not cursor directly
    let updated = f(result);
    B.put(cursor, updated);
    A.put(cursor, intermediate);        -- put flows right-to-left
  }
}
```

This is the core DoD optimization: two optics that would require two loop passes are merged into one.

**Fusion blocked:** If the intermediate value does escape (captured in a `let` that is used outside the query chain), emit `FUS-501`:

```text
note[FUS-501]: compose fusion blocked — intermediate value escapes
  introduce a stage boundary or capture the value explicitly
```

#### 10.10.4 Pass 3: Product flattening

##### 10.10.4.1 Product-flattening algorithm

```text
product_flatten(graph):
  changed = false
  for node in graph.nodes:
    if node matches Product(Product(a,b), c):
      replace node with ProductFlat([a,b,c])
      changed = true
    elif node matches Product(a, Product(b,c)):
      replace node with ProductFlat([a,b,c])
      changed = true
  return (graph, changed)
```

The point of flattening is not aesthetic. A flat product lowers to fewer nested tuples, fewer tuple projections, and more direct register allocation in the generated loop body.

**Pattern:**

```text
Product(Product(A, B), C)   or   Product(A, Product(B, C))
```

**Precondition:** All three optics share the same costate type.

**Rewrite:** Normalize to a left-leaning flat `Product` chain:

```text
Product_flat(A, B, C)   -- single flat product node
```

This reduces the number of nested tuple operations in the generated Rust.

#### 10.10.5 Provenance preservation rules

##### 10.10.5.1 Optimizer fixed-point driver

The optimizer should be implemented as a small deterministic fixed-point engine with explicit pass order and iteration caps.

```text
optimize(graph):
  g = graph
  changed = true
  iters = 0
  while changed and iters < 8:
    changed = false
    for pass in [map_fusion, compose_fusion, product_flatten]:
      (g2, changed_pass) = pass(g)
      g = g2
      changed = changed or changed_pass
    verify(g)
    iters += 1
  return g
```

The cap is not about hiding bugs; it is about making optimization behavior reproducible. If a rewrite system does not converge quickly on these tiny graph shapes, the rewrite laws themselves need attention.

##### 10.10.5.2 Why the pass order is semantically sensible

- Map fusion comes first because it reduces noise without changing the graph's structural edges.
- Compose fusion comes second because fewer intermediate map nodes means fewer apparent escape paths.
- Product flattening comes last because it is a shape-normalization pass whose result is easiest to reason about after other local simplifications have already happened.

This ordering is not sacred forever, but it is a clean v0 default with predictable diffs.

Every fusion pass must:

1. Carry all original `NodeId`s in the `FusedLoop.original_ids` list.
2. Carry the span union of all fused nodes as the new `provenance`.
3. Store the pre-fusion CGIR snapshot in the diagnostics context (accessible via `dump-cgir --before-fusion`).
4. Name the generated loop variable after the outermost optic in the fusion.
5. Emit `// optic(fused): [A, B, C]` as a comment in the Rust output.

Provenance must survive the full pipeline: fusion → codegen → benchmark. A profiler report must be traceable back to a named optic, even if that optic was fused.

---

