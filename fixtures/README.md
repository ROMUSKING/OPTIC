# Golden fixtures (appendix B / M0–M6)

Committed snapshots for lexer, AST, CGIR, diagnostics, and bench baselines.

## Layout

- `tokens/` — `dump-tokens` output per example
- `ast/` — `dump-ast` output per example
- `cgir/pre/` — pre-fusion `dump-cgir` (raw CGIR after build, before `optic-opt`)
- `cgir/post/` — post-fusion `dump-cgir` (after map→compose→product passes)
  - `health_decay` pre/post are often identical: map-chain fusion happens at HIR, so CGIR sees a single `QueryMap` already
  - `health_position` post adds `FusedLoop` provenance for product flatten; compare pre vs post to see fusion annotations
- `hir/` — `dump-hir` snapshots
- `diagnostics/` — human `.txt` and JSON `check --json` witnesses
- `bench/` — `optic bench --update` wall-time baselines
- `rust/` — emitted Rust shape references

## Update workflow

```bash
cargo run -p optic-cli -- snapshot-update --confirm
# or manually:
OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-syntax golden_
OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-cli golden_cgir
cargo run -p optic-cli -- bench --update
```

Review diffs before committing.

## Negative examples (`invalid_*`, `parse_error.opt`, `cgi003_*`, `cgi004_*`, `cgi005_*`, `res001_*`)

These fixtures are **`optic check` / `check --json` witnesses only**. They intentionally fail
before a stable CGIR graph exists, so there are no `dump-cgir` goldens under `cgir/pre|post/`
for them. Use `cargo test -p optic-cli diagnostics_json` or `integration` negative tests.

## Verification tips

- Single-crate tests: `cargo test -p <crate> -- --quiet`
- Full workspace with all failures: `cargo test --workspace --no-fail-fast`
- CGIR goldens + verify: `cargo test -p optic-cli golden_cgir`