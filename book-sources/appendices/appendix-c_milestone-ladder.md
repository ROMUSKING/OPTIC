## Appendix C — Milestone Ladder

This appendix turns the milestone story into a compact control surface. The ladder is capability-based, not calendar-based, and each gate is written as a claim that must be made true in repository evidence rather than by schedule pressure alone.

| Milestone | Gate | Primary risk |
|---|---|---|
| M0 — lexer and parser | deterministic parsing, all syntax errors collected in one pass, token and AST snapshots committed | operator tokenization (`>>>`, `***` as indivisible tokens); recovery producing spurious later errors |
| M1 — HIR | names resolve, query chains lower to explicit cursors, summaries exist for all named optics | `PathLift` producing incomplete region sets for nested optics; `put_reads` field populated incorrectly |
| M2 — typed HIR | types, concrete grades, and alias rules enforced with stable diagnostics | alias checker false negatives on `put_reads` conflicts; grade composition arithmetic saturating incorrectly at composition boundaries |
| M3 — CGIR | graph construction deterministic, invariant checker passes, dump output stable | provenance fields (`original_ids`) not threaded through node construction from day one |
| M4 — minimal fusion | map fusion, compose fusion, and product flattening sound and provenance-preserving | rewrite introducing a fusion that is semantically wrong but fast; soundness argument must be written before merge |
| M5 — Rust backend | generated Rust compiles, acceptance examples pass, runtime crate complete, **benchmark baselines committed** | benchmark baselines not committed at M5 makes "benchmarks green" at M6 meaningless |
| M6 — prelude release | benchmarks green within tolerance, diagnostics stable, fixtures frozen, **agent-repair loop validated** | agent requires more than one pass to resolve a grade or alias diagnostic — missing evidence field |
| M7 — full language begins | prisms, traversals, richer grades, coinduction on top of stable M6; **ownership/session/asymmetry decisions already fixed** | implementation must use the fractional carrier, explicit asymmetric syntax, and the narrowed session scope from day one of M7 |
| M8 — observability and replay | tap, record, profile, time and RNG injection, replay hooks | time injection not threaded from `HostContextLite` from M0 — requires retroactive refactor |
| M9 — self-hosting ladder | mixed seed/self-hosted compiler with differential validation; **translation-validation harness operational** | self-hosted output diverges in grade or alias diagnostics from seed without a harness to detect it |
| M10 — kernel-class ladder | `no_std` runtime and staged kernel-domain progression; **memory model chapter written** | kernel targets claim correctness without a stated memory model for atomics, volatility, and callbacks |

### C.1 Critical path notes

The notes below are the places where sequencing matters most. They are included here so that the ladder remains a management tool rather than a decorative roadmap.

**The alias checker is the hardest part of M2.** The acceptance test `invalid_alias.opt` must be committed and passing before M2 is declared. The checker must be exercised against: deeply nested products, self-referential costates, compositions where the conflict is in `put_reads` rather than `put_writes`, and `Linear` optic double-use. These are the cases most likely to produce false negatives.

**Provenance must be designed into M3, not retrofitted at M8.** Once the CGIR graph is constructed without `original_ids` on every node, adding them requires touching every pass that creates or rewrites nodes. That is an M4–M5 disruption if left to M8.

**Benchmark baselines must be committed at M5, not M6.** The M6 gate "benchmarks green within tolerance" is only meaningful if the baselines existed before M6 work began. A baseline committed on the same day as the gate check is a benchmark regression waiting to happen at M7.

**The front-loaded ownership/asymmetry/session decisions (§27.19) must already be reflected in the implementation by M7.** Any M7 code written against a different carrier or syntax regime will require avoidable rework. They should therefore be treated as M6 exit criteria in practice.

**Before M9:** Edition policy, module interface format, and conformance suite must exist. Self-hosting is only meaningful if there is a canonical artifact to validate against.

**Before M10:** Formal memory model, debug/profiling provenance strategy, and runtime-family policy. A kernel-class target without a stated memory model is an unsafe library, not a language.

The milestone ladder is one of the book's governance tools. It prevents the project from confusing architectural ambition with implementation readiness.

---

