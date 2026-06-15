## 5. Optics as the Transformation Model

> **By the end of this chapter, a reader should understand:** why the language uses optics as its primary abstraction for data transformation rather than methods, callbacks, iterators, or effect handlers; what the `get`/`put` pair represents operationally and why both halves matter for alias analysis and codegen; how the two composition operators (`>>>` and `***`) preserve structure in ways that a conventional function-call IR would not; and why `Cursor<S>` is the key bridge between the semantic model and the generated machine code.

### 5.1 Why optics are the right abstraction here

An optic is useful in this language only if it is more than a mathematical gesture. It has to survive the trip into compiler machinery and machine code.

#### 5.1.1 The compiler may speak category theory; the user does not have to

The internal justification for the design is unapologetically mathematical: costates, coalgebraic optics, and semiring-structured grades are the most compact way to state the laws the compiler relies on. But the default user-facing vocabulary should stay operational. Most programmers should be able to think in terms of focuses, regions, updates, hot paths, ownership shares, budgets, and replay boundaries without needing to adopt category theory as everyday surface language. The theory remains the compiler's organizing vocabulary; diagnostics and tutorials should default to the machine-facing one.

That is why the book treats optics operationally. A lens-like optic in the prelude is the promise that there exists a stable and inspectable pair of operations:

```text
Lens<S, A> ≈ { get: Cursor<S> -> A, put: (Cursor<S>, A) -> () }
```

That pair is not merely convenient for implementation. It is the executable form of the coalgebraic view. `get` tells the compiler where the focused value comes from. `put` tells the compiler exactly how updates re-enter the surrounding structure. Both pieces matter for alias safety, fusion, diagnostics, and later native-code lowering.

### 5.2 The prelude optic: lens-like only

The first compiler only supports lens-like optics. That choice is not a loss of theoretical ambition. It is a narrowing designed to maximize the ratio of semantic content to implementation risk.

A lens already gives the compiler everything it needs to prove the architecture.

- A focused read path
- A structured write-back path
- Composition laws
- Interaction with ownership and locality
- Enough surface syntax to make realistic examples

Prisms, traversals, folds, setters, and isos all build on the same basic bridge once the lens path is trusted.

### 5.3 Composition as structure, not syntax sugar

The language uses two primitive optic compositions in the core surface.

```text
A >>> B   -- sequential composition
A *** B   -- product over the same costate
```

Sequential composition is where intermediate structure flows. Product composition is where shared-costate parallel legality matters. These are not merely ergonomic operators. They are what let the compiler build a graph-shaped IR that still remembers what the programmer intended.

A conventional compiler that sees only functions and calls must reconstruct adjacency and sharing later. This language keeps them explicit from the start.

### 5.4 The optic varieties beyond v0

The full language adds other optic varieties because they correspond to recurring machine shapes.

| Optic variety | Semantic meaning | Dominant low-level shape |
|---|---|---|
| Lens | exactly one focus | direct load/modify/store |
| Prism | zero or one focus | conditional branch |
| Traversal | many homogeneous foci | bulk loop or SIMD pass |
| Fold | many read-only foci | reduction or scan |
| Setter | many write-only foci | masked bulk update |
| Iso | lossless conversion | representation rewrite or zero-cost coercion |

This table foreshadows the later chapters. The point is not that each optic kind is tied to one instruction. The point is that each kind has a dominant machine interpretation, and the language design follows that fact rather than hiding it.

### 5.5 Why optics instead of ambient handler evidence as the default

A language with evidence-passing handlers or effect rows can express broad classes of effectful computation. That is valuable. But for the class of systems work this language targets, evidence values are often the wrong default implementation form.

The compiler does not want to dynamically look up the evidence for reading `positions[*]`, writing `healths[*]`, or polling a clock field. It wants those as direct structured accesses over `Runtime`. Optics provide the right bridge because they expose both context requirement and update path in a way the compiler can summarize and erase.

This does not mean general handlers are impossible later. It means the default representation of structured stateful work should be optic-shaped, not handler-shaped.

#### 5.5.1 Cross-language comparison: optics versus the usual abstraction shapes

At first encounter, many readers map optics onto a more familiar abstraction they already know. That instinct is useful, but each analogy is incomplete.

| Language or family | Nearest familiar abstraction | Where the analogy helps | Where Optic deliberately goes further |
|---|---|---|---|
| **C++** | methods on structs, iterator/range pipelines, visitors | close to data layout and easy to imagine as inlineable code | optics keep `get` and `put` explicit and summary-friendly, so fusion and alias reasoning operate on structure rather than on conventions spread across methods and templates |
| **Java** | object methods, streams, visitors, framework pipelines | familiar composition vocabulary | typical Java abstractions do not preserve field-level layout and writeback structure as optimizer-facing artifacts |
| **Python** | object updates, comprehensions, decorators, Pandas/NumPy-style transforms | good intuition for "describe the transformation, not the loop" | the transformation remains statically typed and region-aware all the way into code generation instead of collapsing into dynamic dispatch |
| **TypeScript** | array combinators, unions, object-path updates, Rx-style streams | explicit dataflow APIs feel similar on the surface | Optic insists that the same composition survive into backend legality checks, not disappear after type erasure |
| **Rust** | iterator chains, view types, borrowed projections, lens crates | probably the closest mainstream comparison | optics are not just library patterns here; they are the language's primary transformation objects and the things CGIR is built out of |
| **Haskell / optics libraries** | lenses, prisms, traversals, folds | the optic vocabulary is directly shared | the goal is different: in Optic, the abstraction is judged partly by whether it produces strong machine-level consequences such as cache-lawful loops, TBAA, and SIMD eligibility |

The key point is simply that optics are the smallest abstraction that keeps observation, reinsertion, composition, and summary formation in one place.

### 5.6 The cursor as the operational heart of the prelude

Every successful prelude optic body is normalized through one executable form:

```text
Cursor<S> = { arena: &mut S, id: usize }
```

`Cursor<S>` is what makes the semantic story line up with the machine. The arena field is the base pointer. The index is the induction variable. Field access becomes address computation. Region extraction becomes structural and predictable.

That is why the cursor is not a trivial helper. It is the proof-carrying bridge between optic semantics and code generation.

### 5.7 Transition

If optics explain the program's shape, grades explain its contracts. The next chapter turns locality, exclusivity, latency, and replay constraints into static objects the compiler can manipulate.

