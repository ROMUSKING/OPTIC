## 8. HIR, Cursors, Names, and Summaries

### 8.1 Why HIR exists as a distinct phase

The HIR is where the language stops being surface syntax and starts being an analyzable program. This phase has one job: preserve semantic intent while removing superficial ambiguity.

If the compiler skipped directly from AST to type checking or codegen, every later phase would be forced to repeatedly rediscover the same facts: which identifier refers to a named optic, which field belongs to which costate, what the current cursor is, and whether a query chain represents a get, set, or map action.

HIR exists to make those facts explicit once.

That explicitness is also what keeps the later language from requiring a front-end rewrite. `Cursor<S>`, `PathLift`, and `OpticSummary` are not narrow-compiler conveniences. They are the hooks by which later chapters attach richer optic kinds, asymmetric grades, replay metadata, stageability facts, module-interface summaries, and foreign-boundary contracts without changing the basic shape of the front end.

### 8.2 Name resolution order

The narrow compiler resolves identifiers in a fixed order.

1. local variables in scope,
2. named optics,
3. named `data` declarations,
4. built-ins and primitive types.

This is simple, but the simplicity is load-bearing. Deterministic resolution is a prerequisite for deterministic diagnostics, stable summaries, and reproducible code generation.

### 8.3 The cursor model and why it matters

#### 8.3.1 Concrete cursor lowering table

| Surface form | HIR form | Why the rewrite matters |
|---|---|---|
| `s.field[s.id]` | `cursor.arena.field[cursor.id]` | normalizes the base pointer and induction variable |
| `s.id` | `cursor.id` | makes loop-carried identity explicit |
| `s` in read position | `cursor.arena` | allows whole-costate reads without losing the cursor anchor |
| `s.field[s.id] = v` | `cursor.arena.field[cursor.id] = v` | turns update paths into explicit store sites |

All prelude optic bodies are normalized through a cursor.

```text
Cursor<S> = { arena: &mut S, id: usize }
```

A surface access like:

```rust
s.healths[s.id]
```

becomes the HIR shape:

```text
cursor.arena.healths[cursor.id]
```

That normalization does several things at once.

- It makes the induction variable explicit.
- It gives later passes a single access shape to reason about.
- It turns region extraction into a structural walk rather than a syntax-sensitive guess.
- It helps the code generator produce direct index loops without inventing new variables ad hoc.

### 8.4 Query chains as explicit HIR nodes

#### 8.4.1 Core HIR node shapes

```text
HirOptic =
  | Named(name, span)
  | Compose(lhs, rhs, span)
  | Product(lhs, rhs, span)
  | Paren(inner, span)

HirQuery =
  | QueryGet  { costate, optic, cursor, span }
  | QuerySet  { costate, optic, cursor, value, span }
  | QueryMap  { costate, optic, cursor, fn, span }
```

#### 8.4.2 Query lowering sketch

```text
lower_query_chain(base, optic, methods):
  costate = lower_expr(base)
  optic'   = lower_optic(optic)
  cursor   = fresh('cur')
  current  = QuerySeed(costate, optic', cursor)
  for method in methods:
    current = lower_method(current, method)
  return current
```

The book keeps this algorithm in prose because the important point is not the loop itself; it is that the surface chain is lowered once into a shape that every later pass can trust.

The surface query syntax is intentionally pleasant. The HIR form is intentionally blunt.

```rust
entities.query(HealthView).map(|h| h - 10.0)
```

becomes:

```text
QueryMap {
  costate: Var("entities"),
  optic:   OpticRef("HealthView"),
  cursor:  FreshCursor("cur_0"),
  fn:      Closure { param: "h", body: Sub(Var("h"), FloatLit(10.0)) }
}
```

This is a perfect example of the book's general method: surface ergonomics for the human, explicit structure for the compiler.

### 8.5 Why summaries are the compiler's real semantic currency

#### 8.5.1 A concrete summary record

```text
OpticSummary {
  name,
  costate,
  focus,
  lift,
  get_reads,
  put_reads,
  put_writes,
  get_grade,
  put_grade,
  get_determinism,
  put_determinism,
  serializable,
  provenance,
}
```

#### 8.5.2 Why each field exists

| Field | Immediate consumer | Why it cannot be reconstructed cheaply later |
|---|---|---|
| `lift` | composition of nested optics | nested regions become ambiguous after lowering |
| `get_reads` | coeffect judgment and fusion legality | read structure gets flattened by codegen |
| `put_reads` | alias checker | read-for-update hazards are easy to miss later |
| `put_writes` | alias checker and backend stores | store sets must remain explicit |
| `get_grade` / `put_grade` | checker and future asymmetric optics | later host optics need direction-sensitive budgets |
| determinism bits | replay and coinduction gates | nondeterminism is easier to prevent than recover from |

Every named optic in the typed HIR carries an `OpticSummary`. Without that object, the compiler has no compact representation of the information later phases actually need.

```text
OpticSummary {
  costate,
  focus,
  lift,
  get_reads,
  put_reads,
  put_writes,
  get_grade,
  put_grade,
  get_determinism,
  put_determinism,
  serializable,
  provenance,
}
```

Each field exists because a later rule depends on it.

- `lift` is required for nested composition.
- `get_reads`, `put_reads`, and `put_writes` are required for alias analysis.
- `get_grade` and `put_grade` are required both for checking and future asymmetric I/O support.
- determinism and serializability are reserved because replay and coinduction will need them later.

This is the point where a future reader should see the blueprint logic explicitly. The summary is intentionally slightly richer than the prelude strictly needs because the later language will reuse exactly this record rather than inventing a second one. Asymmetric `get_grade`/`put_grade`, richer determinism classes, serializability, replay, and boundary summaries are all already visible here as dormant structure rather than future semantic ruptures.

### 8.6 Path lifting and why nested optics are otherwise underspecified

A child optic only knows how to speak about its focus-relative regions. A parent composition needs those regions restated in the source costate. That is the purpose of `PathLift`.

For a field lens focusing `positions` out of `Entities`, the lift maps focus-relative paths back to source paths. Without that operation, summary composition would be approximate in exactly the wrong places: nested writes and read-for-update hazards.

### 8.7 Summary composition sketches

For sequential composition, the key intuition is that a write-back through a nested optic may still need to read the outer focus again.

```text
summary(A >>> B):
  get_reads  = A.get_reads ∪ lift(A, B.get_reads)
  put_reads  = A.put_reads ∪ A.get_reads ∪ lift(A, B.put_reads)
  put_writes = A.put_writes ∪ lift(A, B.put_writes)
```

For product composition, the intuition is simpler: both children speak about the same costate, so the summary is mostly a union plus a later alias-safety check.

```text
summary(A *** B):
  get_reads  = A.get_reads ∪ B.get_reads
  put_reads  = A.put_reads ∪ B.put_reads
  put_writes = A.put_writes ∪ B.put_writes
```

The prose matters here because a raw formula can hide the operational reason. `put_reads` includes `A.get_reads` in the sequential case because rebuilding the enclosing structure is itself a read-for-update hazard.

### 8.8 Transition

Once HIR has explicit names, explicit cursors, and explicit summaries, the type checker can stop guessing and start enforcing. The next chapter explains how grades and alias safety are made algorithmic rather than aspirational.

### 8.9 Detailed implementation reference: HIR lowering, cursor insertion, and summary construction

This supplement makes the cursor model fully operational. It spells out the resolver tables, query-chain lowering, path lifting, and the specific summary-builder rules that later phases rely on.

The HIR phase performs name resolution, cursor insertion, query chain desugaring, and optic summary computation. It must not touch types or grades (those are `optic-typeck`'s job).

#### 8.9.1 Name resolution

##### 8.9.1.1 Resolver data structures

A minimal but robust resolver for v0 uses three explicit maps plus a scope stack:

```text
GlobalOptics   : Symbol -> OpticId
GlobalData     : Symbol -> DataId
BuiltinTable   : Symbol -> BuiltinId
LocalScopes    : Vec<HashMap<Symbol, LocalId>>
```

Resolution should never be "best effort". Every successful resolution produces a specific symbol class (`Local`, `NamedOptic`, `Data`, `Builtin`) and that class is recorded in the HIR node. This prevents later passes from reparsing or guessing symbol meaning.

##### 8.9.1.2 Resolver algorithm

```text
resolve_ident(name):
  for scope in LocalScopes from innermost to outermost:
    if name in scope: return Local(scope[name])
  if name in GlobalOptics: return NamedOptic(GlobalOptics[name])
  if name in GlobalData:   return Data(GlobalData[name])
  if name in BuiltinTable: return Builtin(BuiltinTable[name])
  emit RES-101(name)
  return ErrorSymbol
```

Determinism requires duplicate declarations to be rejected at insertion time. Shadowing inside closures is fine; ambiguous globals are not.

Resolution order for `IDENT` tokens:
1. Local variables in the current closure scope
2. Named optics in the current file
3. Named costates (`data` declarations) in the current file
4. Imported symbols (future; not v0)
5. Built-in types (`SoA`, `BitSet`, `Vec2`, `f32`, etc.)

Unresolved identifiers emit `RES-101`. Unresolved field names on a known costate emit `RES-111`.

Resolution must be deterministic: if two definitions share a name in the same scope, the second is an error, not a shadowing.

#### 8.9.2 Query chain lowering

Surface syntax:

```rust
entities.query(HealthView).map(|h| h - 10.0)
```

HIR form:

```text
QueryMap {
  costate: Var("entities"),
  optic:   OpticRef("HealthView"),
  cursor:  FreshCursor("cur_0"),
  fn:      Closure { param: "h", body: Sub(Var("h"), FloatLit(10.0)) },
  span:    ...,
}
```

The HIR makes the cursor explicit. Every query form introduces a fresh cursor name to make codegen straightforward.

##### 8.9.2.1 Query-chain lowering algorithm

```text
lower_query_chain(base_expr, optic_expr, methods):
  base   = lower_expr(base_expr)
  optic  = lower_optic_expr(optic_expr)
  cursor = fresh_symbol('cur')

  current = QuerySeed { costate: base, optic: optic, cursor: cursor }
  for m in methods:
    match m:
      get()   -> current = QueryGet { ...from current... }
      set(v)  -> current = QuerySet { ...from current..., value: lower_expr(v) }
      map(cl) -> current = QueryMap { ...from current..., fn: lower_closure(cl) }
  return current
```

The lowering is intentionally left-associated over the method chain so that source order is preserved exactly in spans and diagnostics. Any syntactic sugar that changes source ordering should be delayed until after type checking, not hidden inside the HIR builder.

#### 8.9.3 Cursor insertion rules

| Surface form | HIR cursor form |
|-------------|----------------|
| `s.healths[s.id]` in get body | `cursor.arena.healths[cursor.id]` |
| `s.healths[s.id] = v` in put body | `cursor.arena.healths[cursor.id] = v` |
| `s.id` | `cursor.id` |
| `s` (whole costate in get) | `cursor.arena` (read-only reference) |

The compiler should detect and reject any attempt to store or alias the cursor itself — it is a temporary view, not a first-class value in v0.

#### 8.9.4 HIR optic node shapes

```text
HirOptic =
  | Named(name: Symbol, span: Span)
  | Compose(lhs: HirOptic, rhs: HirOptic, span: Span)    -- >>>
  | Product(lhs: HirOptic, rhs: HirOptic, span: Span)    -- ***
  | Paren(inner: HirOptic, span: Span)                   -- for provenance
```

The HIR preserves parentheses as `Paren` nodes for error attribution. They are stripped during CGIR construction after operator precedence is confirmed.

#### 8.9.5 Summary computation

##### 8.9.5.1 Path-lift-aware composition rules

Simple unions are not precise enough once nested field composition matters. The implementation should compute summary composition with explicit path lifting.

For a summary:

```text
Ω = ⟨π, Rg, Rp, W, Gg, Gp, Dg, Dp, Ξ⟩
```

where `π` is the `PathLift`, the composition rules are:

```text
summary(A >>> B) =
  lift       = πA ∘ πB
  get_reads  = A.get_reads ∪ πA(B.get_reads)
  put_reads  = A.put_reads ∪ A.get_reads ∪ πA(B.put_reads)
  put_writes = A.put_writes ∪ πA(B.put_writes)
  get_grade  = combine_seq(A.get_grade, B.get_grade)
  put_grade  = combine_seq(A.get_grade, combine_seq(B.put_grade, A.put_grade))
  get_det    = join_det(A.get_det, B.get_det)
  put_det    = join_det(A.get_det, join_det(B.put_det, A.put_det))
  serializable = A.serializable && B.serializable
```

The extra `A.get_reads` term in `put_reads` is the important one: to put through a composed optic, the outer optic often needs to re-read the enclosing focus in order to rebuild it.

For product:

```text
summary(A *** B) =
  lift       = pair_lift(πA, πB)
  get_reads  = A.get_reads ∪ B.get_reads
  put_reads  = A.put_reads ∪ B.put_reads
  put_writes = A.put_writes ∪ B.put_writes   -- legality checked separately
  get_grade  = combine_par(A.get_grade, B.get_grade)
  put_grade  = combine_par(A.put_grade, B.put_grade)
  get_det    = join_det(A.get_det, B.get_det)
  put_det    = join_det(A.put_det, B.put_det)
  serializable = A.serializable && B.serializable
```

##### 8.9.5.2 Summary builder algorithm

```text
build_summary(named_optic):
  get_reads  = collect_regions(named_optic.get_body, mode='read')
  put_reads  = collect_regions(named_optic.put_body, mode='read')
  put_writes = collect_regions(named_optic.put_body, mode='write')
  get_grade  = infer_get_grade(get_reads)
  put_grade  = infer_put_grade(put_reads, put_writes)
  lift       = infer_lift(named_optic.focus_path)
  return OpticSummary(...)
```

This builder is the compiler's first real abstraction barrier. Every later legality decision assumes the summary is correct.

---

