# Optic Narrow v0 — Implementation Guide

This repository implements the **narrow v0** compiler described in the Optic Language Implementation Book (`book-sources/`, assembled in `book-sources/assembled.md`).

## Quick start

```bash
cargo build -p optic-cli
cargo run -p optic-cli -- check examples/health_get.opt
cargo test --workspace --no-fail-fast
```

The CLI binary is **`opticc`** (appendix B names it `optic`).

## Architecture

| Crate | Milestone | Role |
|-------|-----------|------|
| `optic-syntax` | M0 | Lexer, parser, AST |
| `optic-hir` | M1 | Lowering, `OpticSummary`, region map |
| `optic-typeck` | M2 | Types, grades, alias check |
| `optic-cgir` | M3 | CGIR graph + verifier |
| `optic-opt` | M4 | Map / compose / product fusion |
| `optic-codegen-rust` | M5 | Rust emission |
| `optic-runtime` | M5 | `Cursor` + SoA harness |
| `optic-diagnostics` | M6 | Structured diagnostic catalog |
| `optic` | M6 | Stable library facade |
| `optic-cli` | M6 | `opticc` command surface |

Pipeline: `.opt → parse → lower → typeck → CGIR → optimize → verify → emit Rust`

## Key documents

- **[PLAN.md](PLAN.md)** — milestone status, diagnostic catalog, gate checklist
- **[docs/v0-executable-spec.md](docs/v0-executable-spec.md)** — API, CLI, fixtures, verification
- **[fixtures/README.md](fixtures/README.md)** — golden update workflow

## Library API (`crates/optic`)

Embedding helpers: `compile_check`, `compile_check_from_path`, `compile_emit`, `explain_grade_from_src`, `explain_focus_from_src`, `parse_src`, `lower_src`, `dump_ast_src`, `dump_hir_src`, `collect_unsupported_surface`.

`dump-summary --node` resolves optic/let **names first**, then falls back to numeric CGIR node ids. `dump-cgir --node` is numeric-only.

`CheckOutcome` includes `typed_hir` for downstream tooling.

## Fixtures

Frozen evidence under `fixtures/` (tokens, ast, hir, cgir, rust, diagnostics, bench). Regenerate with `OPTIC_UPDATE_GOLDEN=1` or `opticc snapshot-update --confirm` — see `fixtures/README.md`.

## Book ↔ v0 mapping notes

- Book `TYP-201` (compose focus mismatch) is not separately emitted in v0; optic **body** mismatches use **`TYP-002`**.
- Parser recursion is capped at **`MAX_PARSE_DEPTH = 512`** (security; emits `PAR-001`).