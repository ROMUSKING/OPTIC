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

`dump-summary --node` and `dump-cgir --node` both resolve optic/let **names first** (via HIR summaries / `resolved_optics`), then fall back to numeric CGIR node ids.

M7 scaffolding: `PrismLeaf` and `TraversalLeaf` lowered (`m7_reserved=false`). M8 scaffolding: `.tap`/`.record` → `Tap`/`Record` (`m7_reserved=false`) + hooks; profile/replay → **OBS-701** (stubs). See docs. (Narrow v0 core M0-M6 complete.)

2026-06-20 continuation: ... ; PLAN/docs exact match (this task: plan updated vs book/app C/EBNF + actual Vec impl + wontfix for full traverse/profile in narrow).
2026-06-21: hard guards + shared helpers + boundary carry prep + test coverage (build guard via small+helper, TYP-010, harness match) + precise sync; see PLAN.
2026-06-21 further: match (not expect) on compile_emit (surface gate) + build(&TypedHir) (guard checks) for decision paths; continued doc/plan sync.
2026-06-21 continuation: explicit build match decision on real TypedHir from nested/records example; plan/docs synced same pass.
2026-06-21 continuation (facade): added explicit build_cgir match decision coverage in automated facade check_positive test (earlier facade build match work) using real TypedHir from record_health.opt (Entities data decl + Record hook + region_map paths exercised for guard Ok/non-exceed); goldens parity zero-change; same-pass sync to all docs.
2026-06-21 continuation (before_fusion + emit): explicit `match` on compile_cgir before_fusion early-return (record_health: Entities+Record+region_map) + compile_emit Ok (nested_position: Transform/Entities/region); real data; goldens zero-drift; same-pass sync. (See PLAN for full exercised details.)



`CheckOutcome` includes `typed_hir` for downstream tooling.

## Fixtures

Frozen evidence under `fixtures/` (tokens, ast, hir, cgir, rust, diagnostics, bench). Regenerate with `OPTIC_UPDATE_GOLDEN=1` or `opticc snapshot-update --confirm` — see `fixtures/README.md`.

## Book ↔ v0 mapping notes

- Book `TYP-201` (compose focus mismatch) is not separately emitted in v0; optic **body** mismatches use **`TYP-002`**.
- Parser recursion is capped at **`MAX_PARSE_DEPTH = 512`** (security; emits `PAR-001`).

Robustness assertions (CGIR wiring/ProductFlat/grades/regions/codegen/parser, error propagation) + clippy allows (required) added 2026-06-20. Parser depth complete on all decl paths; fusion updated for obs nodes; harness env exact match. Keep in sync with PLAN + v0-executable-spec + fixtures/README. Stray root + empty src cleaned. All asserts have msgs.