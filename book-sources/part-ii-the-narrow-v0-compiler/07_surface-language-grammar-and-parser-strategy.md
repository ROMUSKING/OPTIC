## 7. Surface Language, Grammar, and Parser Strategy

### 7.1 Why grammar matters more than it first appears

A language this structured cannot afford a vague parser. If two implementations disagree on token boundaries, precedence, or recovery points, every later artifact becomes unstable: AST fixtures, HIR shapes, diagnostics, CGIR snapshots, and codegen.

That is why the grammar chapter is not a formality. It is the first point where the book's insistence on determinism becomes operational.

### 7.2 The surface forms the prelude supports

The prelude surface is intentionally small.

- `data` declarations
- `optic` declarations with `get` and optional `put`
- `let` bindings for optic expressions
- `fn` declarations for small wrappers and tests
- optic composition with `>>>` and `***`
- query chains using `.query(...).get()`, `.set(...)`, and `.map(...)`

This subset is large enough to write meaningful examples and small enough to parse without ambiguous hidden desugarings.

### 7.3 Lexer rules that must be fixed early

The lexer must commit to a few facts up front.

| Token decision | Why it matters |
|---|---|
| `>>>` is one token | composition precedence depends on it |
| `***` is one token | product parsing depends on it |
| block comments are nestable | generated and hand-written examples both need safe commenting |
| keywords outrank identifiers | deterministic parsing and diagnostics |
| a lone `*` is invalid in surface syntax | avoids confusing arithmetic/operator overlap in v0 |

The longest-match rule is non-negotiable. It is the difference between a stable composition grammar and a parser that is forced to patch over tokenization mistakes later.

### 7.4 Precedence and why it mirrors the machine story

The prelude uses one strong precedence choice:

```text
>>> binds tighter than ***
```

That is not arbitrary. Sequential composition is the closer analogue to direct dataflow or function application. Product composition is the looser analogue to parallel juxtaposition over a shared costate. The tighter binding of `>>>` makes mixed expressions read in the same direction that the compiler will later build the graph.

The parser should still emit a targeted diagnostic if a mixed unparenthesized expression is likely to be confusing. The point is not merely to parse; it is to keep the source language auditable.

### 7.5 Parser architecture

The narrow grammar is best served by a hand-written recursive-descent parser with a small Pratt parser for optic expressions.

That choice is justified by three constraints.

- The grammar is not large enough to justify a heavy generated parser.
- Error recovery and spans matter more than grammar compression.
- Optic composition precedence is simpler to express as binding power than as left-factored productions.

#### 7.5.1 Binding-power sketch

```text
parse_optic_expr(min_bp = 0):
  lhs = parse_optic_atom()
  while next token is >>> or ***:
    read op
    if lbp(op) < min_bp: stop
    rhs = parse_optic_expr(rbp(op))
    lhs = make_node(op, lhs, rhs)
  return lhs
```

The full normative EBNF is placed in Appendix D so the main body can explain why the grammar is shaped this way without drowning the reader in productions.

### 7.6 Recovery is part of correctness

Agent-facing compilation changes the bar for parsing. A parser that stops at the first error is not merely inconvenient; it makes automated repair loops far less efficient.

The prelude parser therefore recovers to synchronization points.

| Context | Recovery tokens |
|---|---|
| top level | `data`, `optic`, `let`, `fn`, EOF |
| optic body | `get`, `put`, `}`, EOF |
| type position | `,`, `>`, `)`, `]`, `=`, `{`, EOF |
| query chain | `.get`, `.set`, `.map`, `;`, `}`, EOF |

The recovery design fits the rest of the compiler doctrine: produce as much deterministic evidence as possible from one run.

### 7.7 A concrete example and why it parses cleanly

```rust
data Entities {
    healths: SoA<f32>,
    positions: SoA<Vec2>,
}

optic HealthView: GradedOptic<Entities, f32, CacheGrade<1> + AffineGrade> {
    get  s      => s.healths[s.id]
    put  (s, v) => { s.healths[s.id] = v }
}

let player_view = HealthView *** PositionView;
```

This example is deliberately small. It exercises the core top-level forms, both field and index syntax, optic composition, and grade annotation without yet involving the later compiler phases. A parser that cannot make this example boring will make every later chapter fragile.

### 7.8 Transition

The parser's job is to preserve structure and spans. The next phase must resolve names, normalize field access through cursors, and compute the summaries that every later legality rule depends on.

### 7.9 Detailed implementation reference: concrete grammar, lexer behavior, and parser recovery

The main chapter introduced the narrative and the design constraints. The following material is the normative algorithmic supplement: exact token behavior, operator precedence, recovery discipline, and grammar fragments that keep independent implementations compatible.

A concrete, implementable grammar is essential for the prelude. Without it, parser behavior varies across implementations and makes golden AST tests fragile.

#### 7.9.1 Character classes

```
letter     ::= [a-zA-Z]
digit      ::= [0-9]
ident_char ::= letter | digit | '_'
```

#### 7.9.2 Token types

##### 7.9.2.1 Longest-match lexer algorithm

The lexer must implement longest-match tokenization for multi-character operators before falling back to single-character punctuation. This is not optional because the grammar relies on `>>>` and `***` being indivisible tokens.

```text
scan_token(i):
  if src[i..].starts_with('>>>'): emit(SEQ, span(i,i+3)); return i+3
  if src[i..].starts_with('***'): emit(PAR, span(i,i+3)); return i+3
  if src[i..].starts_with('=>'):  emit(FAT_ARROW, span(i,i+2)); return i+2
  if src[i..].starts_with('<='):  emit(LE, span(i,i+2)); return i+2
  if src[i..].starts_with('>='):  emit(GE, span(i,i+2)); return i+2
  if src[i..].starts_with('--'):  skip_line_comment()
  if src[i..].starts_with('{-'):  skip_block_comment_nested()
  if is_ident_start(src[i]):      scan_identifier_or_keyword()
  if is_digit(src[i]):            scan_number_literal()
  else                            scan_single_char_punct_or_error()
```

Nested block comments must be handled with a depth counter:

```text
skip_block_comment_nested():
  depth = 1
  while depth > 0:
    if next == '{-': depth += 1
    elif next == '-}': depth -= 1
    elif EOF: emit(PAR-030); break
    advance()
```

The implementation should resist the temptation to use ad hoc regex splitting. Nested comments, longest-match operators, and faithful spans are simpler to reason about in a single deterministic scanner.

| Token | Pattern | Notes |
|-------|---------|-------|
| `IDENT` | `letter ident_char*` | Keywords take priority |
| `INT_LIT` | `digit+` | Unsigned, fits u64 |
| `KW_DATA` | `data` | Keyword |
| `KW_OPTIC` | `optic` | Keyword |
| `KW_GET` | `get` | Keyword |
| `KW_PUT` | `put` | Keyword |
| `KW_LET` | `let` | Keyword |
| `KW_FN` | `fn` | Keyword |
| `KW_QUERY` | `query` (method form) | |
| `KW_MAP` | `map` | |
| `KW_GET_M` | `get` (method form) | |
| `KW_SET` | `set` | |
| `SEQ` | `>>>` | Sequence composition operator |
| `PAR` | `***` | Parallel product operator |
| `FAT_ARROW` | `=>` | Body separator |
| `COLON` | `:` | |
| `COMMA` | `,` | |
| `SEMI` | `;` | |
| `LBRACE` | `{` | |
| `RBRACE` | `}` | |
| `LPAREN` | `(` | |
| `RPAREN` | `)` | |
| `LBRACKET` | `[` | |
| `RBRACKET` | `]` | |
| `DOT` | `.` | |
| `PLUS` | `+` | Grade union in type position |
| `LT` | `<` | Grade parameter open |
| `GT` | `>` | Grade parameter close |
| `STAR` | `*` | (only `***` in surface; single `*` is not a valid operator) |
| `EQUALS` | `=` | |
| `COMMENT` | `--` to end-of-line | Discarded |
| `BLOCK_COMMENT` | `{- … -}` | Nestable; discarded |

#### 7.9.3 Operator precedence (binding tightest to loosest)

| Level | Operator | Associativity | Description |
|-------|----------|--------------|-------------|
| 5 | `>>>` | left | Sequential composition |
| 4 | `***` | left | Parallel product |
| 3 | type `+` | left | Grade union in type annotations |

Without parentheses, `A *** B >>> C` parses as `A *** (B >>> C)` because `>>>` binds tighter. The parser must enforce this and emit `PAR-010` when ambiguous forms appear without the required parentheses.

**Rationale for this precedence:** `>>>` is analogous to function application / sequential pipe (tight), while `***` is analogous to parallel juxtaposition (loose). Arithmetic analogy: `*` is `>>>`, `+` is `***`.

#### 7.9.4 v0 surface grammar (EBNF, canonical form)

Full EBNF is reproduced in Appendix E. The key productions:

```ebnf
program         ::= item* EOF
item            ::= data_decl | optic_decl | let_binding | fn_decl

data_decl       ::= 'data' IDENT '{' (field_decl (',' field_decl)* ','?)? '}'
field_decl      ::= IDENT ':' type_expr

optic_decl      ::= 'optic' IDENT ':' optic_type '{' optic_body '}'
optic_type      ::= 'GradedOptic' '<' type_expr ',' type_expr ',' grade_expr '>'
grade_expr      ::= grade_dim ('+' grade_dim)*
grade_dim       ::= 'CacheGrade' '<' INT_LIT '>'
                  | 'OwnershipGrade' '<' rational_lit '>'
                  | 'LinearGrade' | 'AffineGrade' | 'SharedGrade'
                  | '_'                                    (* infer *)
optic_body      ::= get_clause put_clause?
get_clause      ::= 'get' IDENT '=>' expr
put_clause      ::= 'put' '(' IDENT ',' IDENT ')' '=>' (expr | block_expr)
block_expr      ::= '{' stmt* '}'
stmt            ::= expr ';'

optic_expr      ::= optic_atom (optic_op optic_atom)*
optic_op        ::= '>>>' | '***'
optic_atom      ::= IDENT | '(' optic_expr ')'

let_binding     ::= 'let' IDENT '=' optic_expr ';'

query_chain     ::= expr '.query(' optic_expr ')' ('.' query_method)*
query_method    ::= 'get' '(' ')'
                  | 'set' '(' expr ')'
                  | 'map' '(' closure ')'

expr            ::= query_chain
                  | field_access
                  | index_expr
                  | tuple_expr
                  | IDENT
                  | INT_LIT
                  | '(' expr ')'
field_access    ::= expr '.' IDENT
index_expr      ::= expr '[' expr ']'
tuple_expr      ::= '(' expr (',' expr)+ ')'

type_expr       ::= IDENT ('<' type_expr (',' type_expr)* '>')?
                  | '(' type_expr (',' type_expr)* ')'
                  | 'SoA' '<' type_expr '>'
                  | 'BitSet'
```

#### 7.9.5 Parser strategy

##### 7.9.5.1 Pratt parser skeleton for optic expressions

The optic algebra is the one place where binding power genuinely matters. A Pratt parser makes the rule visible in code.

```text
parse_optic_expr(min_bp = 0):
  lhs = parse_optic_atom()
  while let op = peek_infix_optic_op():
    (lbp, rbp) = binding_power(op)
    if lbp < min_bp: break
    advance()
    rhs = parse_optic_expr(rbp)
    lhs = make_infix_optic_node(op, lhs, rhs)
  return lhs

binding_power('>>>') = (50, 51)
binding_power('***') = (40, 41)
```

This directly encodes the rule that `>>>` binds tighter than `***`.

##### 7.9.5.2 Error recovery sets by context

Recovery works best when synchronization tokens are specific to the parser context rather than globally fixed.

| Context | Sync tokens |
|---------|-------------|
| top-level item | `data`, `optic`, `let`, `fn`, EOF |
| inside optic body | `get`, `put`, `}`, EOF |
| inside type annotation | `,`, `>`, `)`, `]`, `=`, `{`, EOF |
| inside query chain | `.get`, `.set`, `.map`, `;`, `}`, EOF |

The parser should attach a recovery note to the diagnostic whenever tokens are skipped. That gives both humans and agents a concrete explanation for why later syntax may have been interpreted more weakly after the first error.

The v0 parser should be a hand-written recursive-descent parser with Pratt-style operator parsing for optic expressions. Rationale:

- Hand-written parsers produce better error messages than generated ones
- Pratt parsing handles the `>>>` / `***` precedence table cleanly
- The grammar is LL(2) everywhere except the `get`/`put` clause distinction (one token lookahead to the `=>` suffices)

**Error recovery strategy:** At any parse error, the parser should:
1. Emit a `PAR-0xx` diagnostic with the offending span
2. Consume tokens until it finds a synchronization token: `optic`, `data`, `let`, `fn`, `}`, or EOF
3. Resume parsing from the next top-level item

This guarantees that a file with N syntax errors produces N diagnostics rather than stopping at the first.

#### 7.9.6 Span representation

Every AST node must carry a `Span { file: FileId, start: u32, end: u32 }`. Byte offsets into the source file are preferred over line/column (line/column is computed on demand for display). The file must never be implicit; multi-file projects must always carry `FileId`.

---

