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

Appendix B negative (TYP-010): `host_boundary.json` — `unsafe optic` and `extern` rejected before lower/HIR/dump (except `dump-tokens`).

`unsupported_prism.json` / `unsupported_traversal.json` — **GRA-110** witnesses (`CacheGrade<1>` tighter than inferred); prism/traversal surfaces are supported (see `alive_filter.opt`, `all_healths.opt`).

### M8 observability witnesses (OBS-701 / OBS-702)

| Fixture | Code | Example |
|---------|------|---------|
| `unsupported_profile.json` | OBS-701 | `examples/unsupported_profile.opt` |
| `unsupported_replay.json` | OBS-701 | `examples/unsupported_replay.opt` |
| `trailing_tap.json` | OBS-702 | `examples/trailing_tap.opt` |
| `trailing_record.json` | OBS-702 | `examples/trailing_record.opt` |

These are **`opticc check --json` witnesses only** (no CGIR/HIR goldens). `dump-ast` / `dump-hir` reject them with the same OBS gate as `check`.

### M8 positive examples — partial golden policy

Not every M8 positive example has full `tokens/ast/hir/cgir/rust/bench` coverage:

| Example | Goldens present |
|---------|-----------------|
| `tap_health.opt` | tokens, ast, hir, cgir/pre, cgir/post, rust, bench |
| `record_health.opt` | tokens, ast, hir, cgir/pre, cgir/post, rust, bench |
| `tap_record_chain.opt` | tokens, ast, hir, cgir/pre, cgir/post, rust (no bench) |
| `compose_tap.opt` | cgir/pre, cgir/post, rust only (no tokens/ast/hir/bench) |

Hook-string policy and structural limitations: `docs/observability-v0.md`.

`cgi003_traversal_compose.json` — **CGI-003** witness (`traversal_in_compose`) from `compose_traversal.opt`.

`cgi003_prism_compose.json` — **CGI-003** witness (`prism_in_compose`) from `compose_prism.opt`.

Prism e2e positives: `prism_get.opt` (get query), `prism_set.opt` (set query), `partial_prism.opt` (`partial preview` → Option codegen path).

Traversal e2e positives: `traversal_get.opt` (get query), `traversal_set.opt` (set query), `all_healths.opt` (GradedTraversal + map decay; `// optic(traversal):` + `// simd-eligible` in emitted Rust).

CGI-006 witnesses: `cgi006_prism_leaf.json` / `cgi006_traversal_leaf.json` — structured M7 reserved node diagnostics (library/unit test; no `.opt` pipeline example).

CLI binary name: **`opticc`** (book appendix B uses `optic`).

These fixtures are **`opticc check` / `check --json` witnesses only**. They intentionally fail
before a stable CGIR graph exists, so there are no `dump-cgir` goldens under `cgir/pre|post/`
for them. Use `cargo test -p optic-cli diagnostics_json` or `integration` negative tests.

## Verification tips

- Single-crate tests: `cargo test -p <crate> -- --quiet`
- Full workspace with all failures: `cargo test --workspace --no-fail-fast`
- CGIR goldens + verify: `cargo test -p optic-cli golden_cgir`

## 2026-06-20 robustness sync note
debug_assert guards + error hardening added (see PLAN); fixtures unchanged (parity preserved). Empty src/ vestige cleaned. All docs/PLAN/fixtures/README in sync.

## 2026-06-20 continuation
- Parser depth on decls (fn/let/optic/get/put...) + body test.
- Host HIR prep + explicit bypass coverage test (gates unchanged).
- profile/replay CLI arms coverage + runtime stubs (full M8 deferred).
- Sanit enforced on costate + boundary names.
- Harness = cli (env_clear+PATH).
- No clones, redundant asserts cleaned; PLAN/docs match code; goldens untouched.
- This pass: depth threading complete across decls, fusion explicit for obs nodes, harness PATH no-capture match, added sanit asserts; goldens/ behavior preserved.

## 2026-06-21
- Continued: hard guards/scale shared, boundary prep carry+invariants+tests, guard flow exercised (small/empty+helper/decision -- precise in notes), harness match updates, TYP-010 paths explicit; goldens preserved; full verification done. See PLAN.
- Further: explicit match on compile_emit TYP-010 (error path/surface) and build scale guard flow (TypedHir non-exceed checks, automated); docs/PLAN updated; no behavior/golden change.
- 2026-06-21 continuation: build() match decision explicit for real TypedHir (nested_position/records region paths); full sync/verify, goldens untouched.
- 2026-06-21 continuation (facade): explicit `match build_cgir(&TypedHir)` Ok decision arm + guard non-exceed exercised in automated facade check_positive test (earlier facade build match work) using real TypedHir from record_health.opt (Entities data decl + Record hook + region_map paths); also covers via compile_check; nested/records end-to-end in tests+CLI. Goldens for record_health/nested/Transform checked via full golden+rust+integration runs (zero change, parity preserved). Same-pass appends to PLAN/README-IMPLEMENTATION/v0-exec-spec/fixtures/README with precise exercised desc; no lagging phrasing. Followed patterns, no prod .expect, smallest delta.
  - record_health region_map (SoA Entities costate, no record_fields like nested's Transform) distinctly exercises build_region_map + Record creation in build.
- 2026-06-21 continuation: renamed+refactored facade_compile_cgir_*_positive (to before_fusion using record_health) and facade_compile_emit_positive (using nested_position) to explicit match decisions; real data exercised for guards/early-return/emit; goldens for record/nested/Transform checked (zero change, parity); same-pass appends with precise language to all docs/PLAN.

