# Golden fixtures (appendix B / M0–M6)

Committed snapshots for lexer, AST, CGIR, diagnostics, and bench baselines.

## Layout

- `tokens/` — `dump-tokens` output per example
- `ast/` — `dump-ast` output per example
- `cgir/pre/` — pre-fusion `dump-cgir`
- `cgir/post/` — post-fusion `dump-cgir`
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

## Verification tips

- Single-crate tests: `cargo test -p <crate> -- --quiet`
- Full workspace with all failures: `cargo test --workspace --no-fail-fast`
- CGIR goldens + verify: `cargo test -p optic-cli golden_cgir`