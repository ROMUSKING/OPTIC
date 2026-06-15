# Part II — The Narrow v0 Compiler

Part II turns the semantic argument into a compiler that can be falsified. Each chapter introduces one new compiler artifact—grammar, HIR, summaries, checker facts, CGIR, fusion, and code generation—and insists that the artifact remain inspectable, deterministic, and evidence-bearing. The aim is not feature breadth; it is to make the core abstraction executable and auditable.

The important reading rule for this part is that none of these artifacts is merely local to the narrow compiler. Each one is also a **zero-cost structural hook** for later features. The hand-written parser is the future edition and migration surface. `OpticSummary` is the carrier that later holds asymmetric I/O grades, richer determinism classes, replay flags, and foreign-boundary facts. The abstract `GradeConstraintSolver` seam is the future insertion point for symbolic/Z3-backed reasoning. CGIR is the place where traversals, prisms, staging, coinduction, and proof-carrying rewrites can later enter without a semantic rupture. The Rust backend is the reference path against which LLVM, replay, debug provenance, and self-hosting will later be validated.

Read in that light, Part II is not a detour away from the grander systems ambitions. It is the place where the architecture proves that it already contains the attachment points those ambitions will need.
