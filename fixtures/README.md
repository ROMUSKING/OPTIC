# Golden fixtures (appendix B / M0–M6)

Committed snapshots for lexer, AST, CGIR, diagnostics, and bench baselines.

## Layout

- `tokens/` — `dump-tokens` output per example
- `ast/` — `dump-ast` output per example
- `cgir/pre/` — pre-fusion `dump-cgir` (raw CGIR after build, before `optic-opt`)
- `cgir/post/` — post-fusion `dump-cgir` (after map→compose→product passes)
  - `health_decay` pre/post are often identical: map-chain fusion happens at HIR, so CGIR sees a single `QueryMap` already
  - `health_position` post materializes nested product as `ProductFlat([leaf_ids…])`; compare pre vs post to see flattening (no provenance-only `FusedLoop`)
  - `compose_decay` post adds `FusedLoop` with materialized compose body (`compose=0,1`); compare pre vs post for ch10 compose fusion
  - `FusedLoop.original_ids` is an intentional **superset** of the book triple `[A.id, B.id, QueryMap.id]`: compose fusion records the full compose subtree plus the `QueryMap` root (and may include nested `Compose` node ids). Product flatten records **leaf optic ids** plus the enclosing **product node id** (not the query/map root). Downstream tools should treat `original_ids` as provenance closure, not an exact manuscript triple.
- `hir/` — `dump-hir` snapshots
- `diagnostics/` — human `.txt` and JSON `check --json` witnesses
- `bench/` — `opticc bench --update` wall-time baselines
- `rust/` — emitted Rust shape references

## Update workflow

```bash
cargo run -p optic-cli -- snapshot-update --confirm
# or manually:
OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-syntax golden_
OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-cli golden_cgir
OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-hir golden_hir
OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-codegen-rust golden_rust
cargo run -p optic-cli -- bench --update
```

Review diffs before committing.

## CLI commands (v0)

| Command | Notes |
|---------|-------|
| `opticc check` | Full pipeline; rejects TYP-010 |
| `opticc dump-ast` / `dump-hir` / `dump-summary` / `dump-cgir` | Same TYP-010 gate as `check` |
| `opticc dump-summary --node NAME` | Optic/let name lookup (**precedence over numeric**) |
| `opticc dump-summary --node N` | Numeric CGIR node id (fallback when name not found) |
| `opticc dump-cgir --node NAME` | Optic/let name via `resolved_optics` (**precedence over numeric**) |
| `opticc dump-cgir --node N` | Numeric CGIR node id (fallback when name not found) |
| `opticc explain-focus` / `explain-grade` | Per-node lenience for other items' errors |
| `opticc doctor [file]` / `bench [file]` | Optional single-file mode |

## Negative examples (`invalid_*`, `parse_error.opt`, `cgi003_*`, `cgi004_*`, `cgi005_*`, `res001_*`, `typ*`, `unsupported_prism.opt`, `unsupported_traversal.opt`, `host_boundary.opt`)

TYP witnesses (`typ001_unknown_type`, `typ001_unknown_focus`, `typ002_body_mismatch`, `typ002_put_mismatch`, `typ003_*`, `typ004_*`) live under `fixtures/diagnostics/typ*.json`. Regenerate with:

```bash
OPTIC_UPDATE_GOLDEN=1 cargo test -p optic-cli diagnostics_json
```

Explain-grade JSON: `fixtures/diagnostics/explain_grade_*.json` (success: HealthView, BadCache, nested let; errors: EXP-001, TYP-002/003/004 target blocking).

Explain-focus JSON: `fixtures/diagnostics/explain_focus_*.json` (success: HealthView, nested let; errors: EXP-001, TYP-002/010 target blocking).

Appendix B negative (TYP-010): `unsupported_traversal.json`, `host_boundary.json` — traversal, `unsafe optic`, and `extern` rejected before lower/HIR/dump (except `dump-tokens`).

`unsupported_prism.json` — **GRA-110** witness (`CacheGrade<1>` tighter than inferred cache for preview+review regions); prism surface is supported (see `alive_filter.opt`).

`cgi003_prism_compose.json` — **CGI-003** witness (`prism_in_compose`) from `compose_prism.opt`.

Prism e2e positives: `prism_get.opt` (get query), `prism_set.opt` (set query), `partial_prism.opt` (`partial preview` → Option codegen path).

CGI-006 witness: `cgi006_prism_leaf.json` — structured M7 reserved node diagnostic (library/unit test; no `.opt` pipeline example).

CLI binary name: **`opticc`** (book appendix B uses `optic`).

These fixtures are **`opticc check` / `check --json` witnesses only**. They intentionally fail
before a stable CGIR graph exists, so there are no `dump-cgir` goldens under `cgir/pre|post/`
for them. Use `cargo test -p optic-cli diagnostics_json` or `integration` negative tests.

## Verification tips

- Single-crate tests: `cargo test -p <crate> -- --quiet`
- Full workspace with all failures: `cargo test --workspace --no-fail-fast`
- CGIR goldens + verify: `cargo test -p optic-cli golden_cgir`