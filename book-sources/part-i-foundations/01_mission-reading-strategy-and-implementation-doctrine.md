## 1. Mission, Reading Strategy, and Implementation Doctrine

> **By the end of this chapter, a reader should understand:** the three-layer implementation program and why Layer 1 (the narrow v0) is treated as the permanent semantic foundation rather than a disposable first draft, what the implementation doctrine requires and why each rule is load-bearing, and what the book treats as non-goals of the prelude and why those deferrals are disciplined choices rather than omissions.

### 1.1 The book's stance

The implementation program has three layers.

- **Layer 1: narrow v0.** Parse a small language, infer and check a small grade algebra, produce a clear coalgebra graph IR, run a minimal set of sound fusion rewrites, and transpile to readable Rust.
- **Layer 2: full language.** Add richer optics, more grade dimensions, observability, replay, staging, and a real native backend without weakening the guarantees established in Layer 1.
- **Layer 3: systems ambition.** Self-host the compiler, grow a `no_std` runtime, and use the same costate/optic discipline to build kernel-class systems.

The narrow v0 is not a demo, a toy, or a disposable prototype. It is the permanent semantic foundation. If the language cannot make its basic promises inside that small fragment, then adding more expressive surface features only hides the failure.

### 1.2 How the book builds progressively

A reader who comes from type theory may be tempted to begin with optics, prisms, or the comonadic story. A reader who comes from systems programming may want to begin with cache lines, SIMD, io_uring, or TBAA metadata. An implementer may want to jump directly to the parser, HIR, and codegen sections. All three perspectives are valid. The problem is that none of them, by itself, tells the whole story.

The book therefore moves in a strict progression.

First it explains the pressure exerted by hardware and systems workloads. That pressure explains why the language rejects ambient effects, opaque runtime schedulers, and AoS-by-default layout assumptions. Next it explains the semantic core that compresses those pressures into one uniform model. Only after that does it spell out how to build the compiler and why the compiler artifacts have the shapes they do. The domain chapters come last because they are not the source of the design; they are the tests that the design must ultimately pass.

### 1.3 Implementation doctrine

The doctrine is intentionally strict.

| Rule | Why it exists | What it prevents |
|---|---|---|
| Build in vertical slices | each milestone must end in a runnable system | speculative architecture without working evidence |
| Keep HIR and CGIR explicit | semantic structure must survive until legality is proven | rediscovering optic intent from flattened control flow |
| Reject unsupported features loudly | capability gates are part of correctness | silent fallbacks that destroy trust |
| Erase grades only after checking | grades are contracts, not runtime values | ghost metadata leaking into codegen |
| Prefer readable first codegen | semantic debugging matters more than heroic lowering in v0 | backend cleverness masking front-end mistakes |
| Preserve provenance through fusion | source optics remain the unit of explanation | profiling and debugging collapse after optimization |
| Design diagnostics for agents as well as humans | the compiler is a collaboration tool | noisy, unstable, unranked error output |

Each doctrine item has a type-theoretic side and a machine side. For example, keeping CGIR above SSA is not merely a compiler engineering preference. It is the implementation form of the claim that composition, product, traversal, staging, and coinduction are primary structures of the language rather than patterns to be reconstructed later.

### 1.4 Success criteria

The language has earned the right to expand when the following are all true at the same time.

- The v0 compiler accepts a small but non-trivial example suite end to end.
- The generated Rust is readable enough that an implementer can audit whether the abstraction disappeared.
- Grade errors, alias errors, and type errors are deterministic, structured, and repairable.
- The optimizer performs a small rewrite set with clear proof obligations.
- Benchmarks show that the generated loops are close to the hand-written baselines they are supposed to replace.

If any of those are missing, the book treats that as a language-design failure, not a tooling inconvenience.

### 1.5 Non-goals of the prelude

The prelude is not trying to prove everything.

| Deferred topic | Why it is deferred |
|---|---|
| Full LLVM backend | a readable Rust backend is the semantic microscope for v0 |
| Symbolic grades | concrete bounded grades keep inference tractable and diagnostics crisp |
| General resumptive control effects | continuation machinery should not be introduced before the costate core is trusted |
| GPU and distributed costates | they add locality and scheduling models beyond the first implementation envelope |
| Kernel boot and drivers | kernel-class work is a downstream systems program, not the first proof target |
| Self-hosting | self-hosting should be a result of stability, not a substitute for it |

The important point is not that these are unimportant. It is that they only become believable after the smaller claims have been made true in code.

### 1.6 Transition

The rest of Part I answers the question that every ambitious systems language must answer early: why does the semantic model look the way it does instead of like effect rows, ambient runtime services, traits over iterators, or a conventional borrow-checked SSA compiler? The answer begins with the machine.

