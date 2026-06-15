## Appendix A — Diagnostic Catalog

This appendix keeps the diagnostic catalog explicit and stable. The catalog is intentionally load-bearing: both humans and coding agents need a durable reference for the compiler's failure modes, and later migration or automation work depends on those codes remaining a reliable protocol rather than a prose convenience.

The codes are theorem-stable, but the preferred rendering should default to machine-facing language. When the internal rule is about semiring composition, coalgebraic structure, or phase legality, the default message should still tell the reader what collided, overflowed, widened, or crossed a forbidden boundary in operational terms.

| Code | Phase | Meaning | Preferred first action |
|---|---|---|---|
| `PAR-010` | parse | Mixed `>>>` and `***` without parentheses | Add parentheses according to the fixed precedence rule |
| `PAR-011` | parse | `>>>` used where `***` was likely intended, or vice versa | Re-check pipeline grouping intent |
| `PAR-020` | parse | Unexpected token in optic declaration | Compare against the nearest valid optic form |
| `PAR-021` | parse | Missing `=>` in a `get` or `put` clause | Insert `=>` after the parameter list |
| `PAR-030` | parse | Unterminated block expression or block comment | Add the missing terminator |
| `RES-101` | resolve | Unknown symbol | Check declarations, shadowing, and spelling |
| `RES-111` | resolve | Unknown field on costate | Inspect the `data` declaration and field spelling |
| `RES-121` | resolve | Optic used before declaration in an order-sensitive context | Reorder or introduce a declaration boundary |
| `TYP-201` | type | Focus type mismatch in composition | Dump summaries for both optics |
| `TYP-211` | type | Costate type mismatch in product | Ensure both sides of `***` share the same costate |
| `TYP-221` | type | Unsupported optic kind in the prelude | Rewrite to lens form or gate for a later milestone |
| `TYP-231` | type | Map closure parameter type mismatch | Check the focus type of the optic |
| `GRA-101` | grade | Grade dimension missing and inference unavailable | Add an explicit grade annotation |
| `GRA-104` | grade | Sequential composition exceeds declared cache bound | Split the pipeline or relax the bound |
| `GRA-110` | grade | Declared grade is tighter than inferred grade | Tighten the body or relax the declaration |
| `GRA-121` | grade | Ownership grade incompatible with composition or context | Re-check exclusivity assumptions |
| `GRA-131` | grade | Unbounded grade used in a bounded context | Bound the body or remove the bound |
| `ALI-201` | alias | Write/write conflict in product | Separate the product or make one side read-only |
| `ALI-211` | alias | Write/read conflict in product | Make the writer exclusive or the reader strictly read-only |
| `ALI-221` | alias | Linear optic reused after consumption | Ensure the linear resource is used exactly once |
| `OPT-301` | feature | Prism syntax used in v0 | Rewrite to a supported lens-like form |
| `OPT-311` | feature | Traversal syntax used in v0 | Rewrite to the supported prelude subset |
| `OPT-321` | feature | `.coinductive()` or `.drive()` used in v0 | Remove or gate for the full language |
| `OPT-331` | feature | `.record()` or `.replay()` used in v0 | Remove or gate for the observability milestone |
| `OPT-341` | feature | `stage` keyword used in v0 | Remove or gate for the staging milestone |
| `CGI-410` | CGIR | CGIR invariant failure during construction | Dump HIR and the failing node; likely compiler bug |
| `CGI-420` | CGIR | Cycle detected in the CGIR graph | Treat as compiler bug |
| `FUS-501` | fusion | Fusion blocked because an intermediate escapes | Introduce a stage boundary or keep the unfused form |
| `FUS-511` | fusion | Compose fusion blocked by nondeterministic or unsupported shape | Separate the stage or mark as approximate later |
| `FUS-521` | fusion | Fused form regressed benchmark performance | File a performance bug and disable the rewrite for that case |
| `COD-601` | codegen | Rust generation failed for an unsupported shape | Dump fused CGIR; likely compiler bug |
| `COD-611` | codegen | Generated Rust failed `cargo check` | Dump fused CGIR; compiler bug until disproven |
| `STG-101` | staging | Candidate static graph reads a runtime-only region | Move the read behind a residual boundary or route it through `BuildRuntime` |
| `STG-111` | staging | Compile-time host access is not reproducible or not whitelisted | Declare the input explicitly in `BuildHostContext` |
| `STG-121` | staging | Compile-time recursion or traversal lacks a proven bound | Add a bound or residualize the computation |
| `STG-131` | staging | Staged result exceeds code-size or artifact-size budget | Keep structure staged but residualize bulky data |
| `STG-141` | staging | Compile-time work exceeds declared budget | Split specialization, cache earlier, or relax the budget |
| `CTX-101` | context | Focused path is ambiguous in the current root | Disambiguate the focused path or make the root path explicit |
| `CTX-111` | context | Focused form crosses a forbidden build/runtime boundary | Re-scope the focus or split the computation across the correct root |
| `CTX-121` | context | No zero-cost focus path exists for the requested view | Fall back to the explicit root path or refactor the surrounding optic |
| `CTX-131` | context | Focused form would widen alias, determinism, or boundary assumptions illegally | Keep the explicit path or tighten the local region/boundary declaration |
| `GRD-151` | grade | Elided or partially specified grade cannot be reconstructed exactly | Make the missing dimensions explicit or switch to gradual `?` |
| `GRD-161` | grade | Observed development-time grade diverges from the declared or inferred contract | Accept the suggested grade, tighten the implementation, or declare a wider bound |
| `GRD-171` | grade | Elided grade would widen an exported or persisted contract illegally | Spell out the hidden dimensions explicitly at the boundary |
| `OBS-701` | observability | Debug/trace optic used without a matching grade or mode | Add the correct observability grade in the full language |
| `BTS-801` | bootstrap | Self-hosted output diverged from the seed compiler | Run differential harness and compare diagnostics |
| `KRN-901` | kernel | Kernel target used an unsupported host dependency | Replace with a target-approved runtime optic |
| `INI-101` | init | Project intent is incomplete or contradictory | Supply or revise runtime family, target, or domain answers in the wizard or intent file |
| `INI-111` | init | Selected template conflicts with declared target/runtime/capability set | Choose a compatible blueprint or adjust the declared constraints |
| `INI-121` | init | Procedural scaffold generation could not synthesize required build roots or boundary stubs | Accept the suggested missing fields or switch to manual graph-root authoring |

The diagnostic record should be emitted in both human-readable and JSON forms. A good machine-facing record includes:

- stable code,
- phase,
- severity,
- primary and related spans,
- violated rule,
- evidence object,
- ranked local fix options,
- preferred fix,
- next commands.

That structure is what lets coding agents repair locally instead of resorting to whole-file rewrites.

---

