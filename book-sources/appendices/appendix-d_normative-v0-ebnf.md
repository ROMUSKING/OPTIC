## Appendix D — Normative v0 EBNF

This appendix is normative rather than illustrative. The parser for the prelude must implement the following grammar exactly, and any divergence between implementation and grammar should be treated as a compiler defect rather than a user-level ambiguity.

```ebnf
program         ::= item* EOF
item            ::= data_decl
                  | optic_decl
                  | let_binding
                  | fn_decl

data_decl       ::= 'data' IDENT '{' field_list? '}'
field_list      ::= field_decl (',' field_decl)* ','?
field_decl      ::= IDENT ':' type_expr

optic_decl      ::= 'optic' IDENT ':' optic_type_ann '{' optic_body '}'
optic_type_ann  ::= ('GradedOptic' | 'GradedPrism' | 'GradedTraversal') '<' type_expr ',' type_expr ',' grade_ann '>'
grade_ann       ::= grade_dim ('+' grade_dim)*
                  | '_'
grade_dim       ::= 'CacheGrade' '<' INT_LIT '>'
                  | 'OwnershipGrade' '<' rational_lit '>'
                  | 'LinearGrade'
                  | 'AffineGrade'
                  | 'SharedGrade'
                  | 'CacheGrade' '<' '_' '>'
                  | 'BranchBias' '<' IDENT '>'   -- M7: Likely|Unlikely|Unknown
optic_body      ::= get_clause put_clause?
                  | preview_clause review_clause?
                  | traverse_clause update_clause?
-- Phase 1 skeleton: parser permissive (mixed/legacy ok); ctor-specific strict in Track1 Phase 2. EBNF full set (BranchBias IDENT token, lax).
get_clause      ::= 'get' IDENT '=>' expr
put_clause      ::= 'put' '(' IDENT ',' IDENT ')' '=>' (expr | block_expr)
preview_clause  ::= ['partial'] 'preview' IDENT '=>' expr
review_clause   ::= 'review' '(' IDENT ',' IDENT ')' '=>' (expr | block_expr)
traverse_clause ::= 'traverse' IDENT '=>' expr
update_clause   ::= 'update' '(' IDENT ',' IDENT ')' '=>' (expr | block_expr)
block_expr      ::= '{' stmt* expr? '}'
stmt            ::= (IDENT '=')? expr ';'

optic_expr      ::= optic_par
optic_par       ::= optic_seq ('***' optic_seq)*
optic_seq       ::= optic_atom ('>>>' optic_atom)*
optic_atom      ::= IDENT
                  | '(' optic_expr ')'

let_binding     ::= 'let' IDENT ('=' optic_expr
                  | ':' optic_type_ann '=' optic_expr) ';'

fn_decl         ::= 'fn' IDENT '(' param_list? ')' ('->' type_expr)? '{'
                    stmt* expr? '}'
param_list      ::= param (',' param)*
param           ::= IDENT ':' type_expr

query_chain     ::= expr '.query(' optic_expr ')' query_method+
query_method    ::= '.get()'
                  | '.set(' expr ')'
                  | '.map(' closure ')'
closure         ::= '|' IDENT '|' expr
                  | '|' '(' IDENT (',' IDENT)* ')' '|' expr

expr            ::= query_chain
                  | assign_expr
assign_expr     ::= field_expr (('=' assign_expr) | )
field_expr      ::= atom_expr ('.' IDENT | '[' expr ']')*
atom_expr       ::= IDENT
                  | INT_LIT
                  | FLOAT_LIT
                  | '(' expr (',' expr)* ')'
                  | '(' expr ')'
                  | block_expr
                  | binary_expr

binary_expr     ::= atom_expr bin_op atom_expr
bin_op          ::= '+' | '-' | '*' | '/' | '<' | '>' | '<=' | '>='

type_expr       ::= 'SoA' '<' type_expr '>'
                  | 'BitSet'
                  | '(' type_expr (',' type_expr)+ ')'
                  | IDENT ('<' type_args '>')?
type_args       ::= type_expr (',' type_expr)*
rational_lit    ::= INT_LIT '/' INT_LIT | INT_LIT

IDENT           ::= [a-zA-Z][a-zA-Z0-9_]*
INT_LIT         ::= [0-9]+
FLOAT_LIT       ::= [0-9]+ '.' [0-9]+
RATIONAL_LIT    ::= [0-9]+ '/' [0-9]+
COMMENT         ::= '--' [^\n]* '\n'
BLOCK_COMMENT   ::= '{-' (BLOCK_COMMENT | [^-] | '-' [^}])* '-}'
```

### D.1 Disambiguation notes

- `>>>` is a single token, not three `>` tokens.
- `***` is a single token, not three `*` tokens.
- a lone `*` is invalid in the surface language.
- `{-` starts a nestable block comment even inside expressions.
- whitespace is ignored except inside future literal forms.

---


### D.2 Reserved experimental keyword and namespace roots

The contextual keyword `experimental` is reserved for post-v0 package/workspace/build declarations even though the narrow compiler does not implement those declarations yet. Feature names such as `sep`, `memory`, `proof`, `geo`, `ultra`, `sheaf`, `topos`, `dynamics`, and `nonstandard` are **not** reserved as global hard keywords; they are reserved as namespace segments under `std.experimental.*`. In the current roadmap, `sep` and `memory` are the primary direct internal lanes for the open memory-model questions; the others remain second-wave or domain-oriented lanes. TLA+, Alloy, typestate, and abstract interpretation remain external sidecars rather than reserved namespace segments. This keeps the experimentation lane explicit without polluting ordinary source syntax or prematurely promoting every research answer into the language surface.
