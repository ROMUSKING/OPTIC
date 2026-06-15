## Appendix B — Command Surface and Repository Layout

This appendix gathers the operational surfaces an implementer, tool author, or coding agent will touch most often: the command set, the repository shape, and the minimum bootstrap layout for the first compiler milestones. The aim is practical clarity rather than narrative development.

### B.1 Command surface

| Command | Purpose |
|---|---|
| `optic init --wizard` | Guided project creation: collect project intent, generate native package/workspace/build roots, runtime blueprints, AppWorld scaffolds, and initial benchmark/diagnostic artifacts |
| `optic init --template NAME` | Fast-path project creation from a parameterized domain blueprint |
| `optic init --intent FILE` | Headless project creation from structured intent data |
| `optic init --preview` | Show the graph and file projections that would be created without committing them |
| `optic lsp serve` | Serve the first-party LSP adapter over the graph protocol |
| `optic materialize [PATH]` | Materialize graph-backed source or generated projections into an ordinary file tree |
| `optic sync-projections` | Force projection refresh between graph and writable text surfaces |
| `optic check file.opt` | Parse, resolve, type-check, and emit diagnostics |
| `optic check file.opt --json` | Machine-readable diagnostics |
| `optic explain CODE` | Show the rule, examples, and likely repairs for a diagnostic |
| `optic explain-focus file.opt --node NAME` | Show the explicit root-path form of a focused/elided expression |
| `optic explain-grade file.opt --node NAME` | Show the normalized grade after inference or partial elision for one optic |
| `optic explain-grade file.opt --node NAME` | Show the normalized grade after inference or partial elision |
| `optic dump-ast file.opt` | Print AST |
| `optic dump-hir file.opt` | Print resolved HIR and summaries |
| `optic dump-summary file.opt --node NAME` | Show one optic summary |
| `optic dump-cgir file.opt` | Print post-fusion CGIR |
| `optic dump-cgir file.opt --before-fusion` | Print pre-fusion CGIR |
| `optic dump-cgir file.opt --node N` | Print one node and its provenance |
| `optic dump-cgir file.opt --check` | Run invariant checks on CGIR |
| `optic transpile file.opt` | Emit Rust |
| `optic doctor file.opt` | Run consistency checks and suggest next actions |
| `optic bench file.opt` | Run benchmark harness and compare to baselines |
| `optic snapshot-update --confirm` | Update golden fixtures after review |
| `optic bootstrap` | Later: compare seed and self-hosted output |
| `optic experiment list` | Show available experimental lanes and package-level experiment declarations |
| `optic experiment doctor` | Check experimental-artifact schemas, witness freshness, and promotion blockers |

### B.2 Repository layout

```text
optic/
  Cargo.toml
  crates/
    optic-cli/
    optic-diagnostics/
    optic-syntax/
    optic-ast/
    optic-hir/
    optic-typeck/
    optic-cgir/
    optic-opt/
    optic-codegen-rust/
    optic-runtime/
    optic-tests/
  docs/
    implementation-book.md
    v0-executable-spec.md
    observability-v0.md
    effect-coeffect-v0.md
  examples/
    health_get.opt
    health_set.opt
    health_decay.opt
    health_position.opt
    nested_position.opt
    invalid_grade.opt
    invalid_alias.opt
    unsupported_prism.opt
    grade_mismatch.opt
    host_boundary.opt
  fixtures/
    tokens/
    ast/
    hir/
    cgir/
      pre/
      post/
    rust/
    diagnostics/
    bench/
      baselines/
```

### B.3 Minimal file-level bootstrap plan for M0–M2

The narrow compiler should treat M0–M2 as a concrete repository bring-up, not just as milestone labels. This subsection is intentionally practical: it names the first files that should exist and the first responsibilities they should carry.

#### B.3.1 M0 — parser and lexer

```text
crates/optic-syntax/
  tokens.rs
  lexer.rs
  parser.rs
  span.rs
```

Primary tasks:
- longest-match tokenization for `>>>` / `***`,
- nested block comments,
- deterministic recovery,
- token and AST fixtures.

#### B.3.2 M1 — HIR and cursor normalization

```text
crates/optic-hir/
  hir.rs
  resolver.rs
  lower.rs
  cursor.rs
```

Primary tasks:
- name resolution,
- query-chain lowering,
- cursor insertion,
- `PathLift` construction.

#### B.3.3 M2 — summaries, grades, and alias checking

```text
crates/optic-summary/
  infer.rs
  regions.rs
  grade.rs
  determinism.rs

crates/optic-typeck/
  checker.rs
  alias.rs
```

Primary tasks:
- sound `OpticSummary` inference,
- concrete cache grade inference,
- fractional ownership carrier plus named aliases,
- alias conflict diagnostics,
- `invalid_alias.opt` and grade-bound tests.

The important point is that M2 is the semantic hinge. If summary inference or alias checking is wrong, the rest of the compiler only becomes a faster way to be wrong.

### B.4 Agent workflow discipline

A coding agent working in this repository should normally proceed in this order.

1. parse errors,
2. resolve errors,
3. type errors,
4. grade and alias errors,
5. HIR and CGIR inspection,
6. codegen inspection,
7. benchmark review.

This order matters because later phases often depend on earlier-phase structure being clean.

---

