## 28. Feature Admission Checklist and Coding-Agent Failure Modes

This final chapter turns the maturity analysis into a standing review instrument. The earlier chapters identified the recurring late-stage pressures — compatibility, boundary discipline, runtime-family coherence, artifact identity, provenance, and toolchain policy. Those pressures become useful only if future feature proposals are forced to answer them in the same order every time.

The chapter therefore has two tasks. First, it defines a repeatable checklist for deciding whether a proposal belongs in the language core, in the boundary lane, in a quarantined experimental lane, or nowhere in the language at all. Second, it grounds that checklist in a practical observation from the current generation of coding agents: large codebases fail not only because models lack raw coding ability, but because repository knowledge is hard to retrieve, stale guidance crowds out relevant context, multi-repository or multi-artifact work exceeds the default tool envelope, and verification remains expensive and fragile. Those failures are not separate from language design. They are exactly the kind of operational pressure that either gets absorbed into the language's explicit model or later returns as feature creep.

### 28.1 Why a standing checklist belongs in the book

A strong language design can still decay if every later proposal is argued from scratch. The natural failure pattern is familiar: one proposal is allowed because it seems convenient, the next because it looks compatible with the first, and eventually a second semantic center appears by accumulation rather than by declaration.

The book's architecture already has the right counterweight. It insists on one semantic center, one authored language, one summary model, one graph-shaped compiler story, and one explicit boundary lane for realities that the core language should not pretend away. What it still needs is a permanent review surface that makes those commitments operational.

The checklist below should therefore be treated as part of the language's governance model. A proposal that cannot answer these questions has not matured enough to be accepted, no matter how attractive the feature looks in isolation.

### 28.2 The feature-admission checklist at a glance

| Review question | Why it matters | Typical outcomes |
|---|---|---|
| What exact use case is unlocked? | prevents surface imitation of another language without a new operational gain | keep, defer, or reject |
| Which existing lane should carry it: core, boundary, generated artifact, internal toolchain, or quarantined experimental? | prevents second-language creep while still giving research a disciplined home | core / boundary / generated / internal / experimental |
| What earlier artifact proves it is legal? | stops the compiler from rediscovering meaning heuristically | summary, grade, boundary contract, target profile, interface artifact |
| What is the smallest theory or sidecar that already answers the question? | prevents over-theorized core growth and keeps v1 implementation tractable | resource logic / weak memory / typestate / abstract interpretation / TLA+ / Alloy / none |
| What mixed-domain collision should this feature survive? | prevents isolated examples from disguising architectural fractures | render + network, storage + legacy FFI, experimental kernel + ordinary traversal, or an explicitly justified alternative |
| Does it lower through the ordinary summary and boundary path? | prevents experimental mathematics or domain frontends from creating a second rulebook | yes with `OpticSummary`/`BoundaryContract`/CGIR, or reject / quarantine |
| What stable identity does its output have? | makes incremental builds and reproducibility possible | node hash, artifact key, interface hash, revision id |
| Can the ergonomic goal be met by checked focusing/elision or grade inference instead of a new ambient mechanism? | prevents prop-drilling fixes or resource-default conveniences from turning into hidden semantics | focused path, inferred grade, or explicit rejection |
| What compatibility surface does it affect? | keeps source, interface, ABI, and runtime promises separate | edition, interface schema, runtime family, plugin ABI |
| What proof or guarantee does it bypass, if any? | turns unsoundness into a budget instead of folklore | soundness-ledger entry or rejection |
| What provenance survives after optimization and lowering? | keeps debugging, profiling, and agents grounded | source span, node id, fused provenance set, PerfKey |
| What is the smallest rollback path if the feature proves too costly? | prevents irreversible accretion | deprecate, gate, boundary-only, internal-only |

This table is intentionally compact. The longer sections that follow explain how each question should be applied.

Two of the checklist questions deserve special emphasis. First, a proposal should be able to name at least one **mixed-domain collision** it survives without inventing special escape rules, because a systems language proves its extensibility when unlike subsystems meet. Second, experimental mathematics and imported domain frontends are welcome only when they remain subordinate to the ordinary summary, grade, provenance, and boundary pipeline.

### 28.3 Semantic and compiler questions every proposal must answer

#### 28.3.1 What genuinely new use case is unlocked?

A proposal should begin with a use case, not with a borrowed surface feature. The right opening question is not “which mainstream language already has this syntax?” but “what important program shape or system boundary remains impossible or unreasonably awkward under the current model?”

This is the chapter's most important guardrail. If the proposal only makes an existing use case feel more familiar to users of another language, then the default answer should be “not in core.” The current architecture already covers a wide range of use cases through ordinary optics, staged build-time execution, boundary contracts, and generated artifacts. A proposal that does not unlock new operational territory must justify why a second mechanism is worth the long-term compatibility cost.

#### 28.3.2 Which lane carries it: core, boundary, generated artifact, internal toolchain, or quarantined experimental work?

Every proposal should be classified immediately into one of five lanes.

- **Core** means ordinary authored source, ordinary optic semantics, ordinary summaries, and ordinary compile-time and runtime behavior.
- **Boundary** means the use case is fully supported, but it crosses a hostile ABI, hardware, runtime-family, or capability edge and therefore rides through `BoundaryContract`, `unsafe optic`, target/runtime declarations, or generated bindings.
- **Generated artifact** means the feature should exist as compiler-emitted material — lock snapshots, interface files, generated bindings, crash capsules, replay traces, benchmark metadata — rather than as a second authored surface.
- **Internal toolchain** means the capability is real but should remain compiler- or protocol-facing rather than becoming source syntax.
- **Experimental** means the idea is important enough to implement and test, but not yet stable enough for the ordinary language contract. Experimental work enters only through the contextual keyword `experimental`, the `std.experimental` namespace root, and graph-native experimental arenas or artifacts.

The point of this early classification is not bureaucracy. It is to prevent every ecosystem pressure from being interpreted as evidence that the core surface language must grow.
A practical example is solver design. Equation-heavy notation, imported model formats, generated stencils, and symbolic preprocessing may all be valuable. The checklist should still ask whether those needs can be carried by optics-native solver libraries, staged artifacts, and imported frontends before admitting a new solver-specific core syntax.

#### 28.3.2.1 If the idea is still research, quarantine it early

A mathematically interesting idea does not have to jump directly into the core language in order to be taken seriously. The review question should therefore be explicit: can the proposal live under the experimental lane first? If the answer is yes, the default should be to keep it there until it has: (1) a stable summary form, (2) a clear legality rule, (3) a measurable backend or tooling consequence, and (4) a credible rollback path.

This is especially important for proof-oriented, geometric, ultrametric, sheaf-like, topos-like, or dynamical-system-inspired features. Those ideas may well matter to the language, but the burden is to show which layer they refine — proof artifacts, domain numerics, graph retrieval, coinductive stability, or distributed consistency — before they are allowed to reshape ordinary user code.

#### 28.3.2.3 Prefer the smallest theory that closes the operational gap

The review process should ask one question earlier and more bluntly than most language teams do: could a **simpler theory or an external sidecar** answer the same problem with less permanent surface-area cost?

For Optic this matters directly. The open memory-model questions are often better served first by resource/separation reasoning and weak-memory operational models than by richer proof or categorical machinery. Protocol and callback-state questions are often better served first by typestate or explicit protocol automata than by full session or sheaf machinery. Some scheduler, distributed-runtime, and plugin questions are often better explored first in TLA+ or Alloy than as new language features. Conservative optimizer questions are often better answered first through abstract interpretation than through proof-heavy machinery.

The practical rule is simple: if a narrower theory or sidecar already answers the operational question, the burden shifts to the richer proposal to show what new capability it adds beyond elegance.

#### 28.3.2.4 Prefer a decision ladder over a feature pile

For proposals touching memory, boundaries, protocols, or optimizer legality, the review should ask for an explicit ladder:

1. what is the **direct internal lane** that fits the existing model best;
2. what is the **simpler sidecar** that could answer the question without enlarging the language;
3. what is the **richer second-wave theory** that should only be tried if the first two are insufficient.

In current Optic terms, that usually means:
- `std.experimental.sep` before richer provenance or ownership theories,
- `std.experimental.memory` before broader categorical or proof-heavy concurrency foundations,
- typestate, TLA+, Alloy, or abstract interpretation before new protocol or optimizer features,
- and only then `proof`, `sheaf`, `topos`, or `dynamics` as possible deeper refinements.

A proposal that skips this ladder should be presumed too expensive for v1 unless it can show that the smaller answers are already inadequate.

In practical review terms, the committee or maintainer should be able to answer three yes/no questions before approving the work:

- have the direct internal lanes already been tried or clearly ruled out,
- have the smaller sidecars already been tried or clearly ruled out,
- and does the richer proposal preserve the one-summary, one-graph architecture instead of introducing a shadow semantics?

If any of those answers is still "no", the proposal belongs in research notes rather than in the design freeze.

#### 28.3.3 What earlier artifact proves it is legal?

A feature belongs in the language only if the compiler can point to a specific earlier artifact that justifies using it later. In Optic, those artifacts are already known: `OpticSummary`, `RegionSet`, grade expressions, boundary contracts, interface artifacts, target profiles, runtime families, and graph revisions.

If a proposal cannot say which artifact carries its legality proof, then the backend or tooling will eventually have to recover the feature heuristically from lowered code or opaque metadata. That is exactly the failure mode the book has been trying to avoid from the beginning.

#### 28.3.4 What stable identity does the feature's output have?

A large language ecosystem lives or dies by stable identities. A feature that produces code, metadata, interfaces, staged artifacts, replay traces, bindings, or plans must say what identifies those things across revisions.

That identity might be a node hash, a revision id, an interface checksum, a staged-artifact key, or a `PerfKey`. The important point is that the answer cannot be “whatever file happened to be written by the current compiler build.” If it is, incremental compilation, reproducibility, and multi-tool reasoning will all become fragile later.

### 28.4 Compatibility, boundary, and rollback questions

#### 28.4.1 Which compatibility layer does it affect?

Every feature proposal must state explicitly whether it affects:

- source/edition compatibility,
- module-interface compatibility,
- package/build reproducibility,
- binary or plugin ABI,
- runtime-family compatibility,
- or only internal toolchain state.

This is how the language avoids the most common maturity mistake: saying “stable” without naming what is stabilized.

#### 28.4.2 What proof or guarantee does it bypass?

Some features really are escape hatches. That does not disqualify them, but it does change the review rule. If a proposal weakens a proof, it must enter the soundness-budget ledger with:

- the proof it bypasses,
- the local contract that replaces it,
- the diagnostic family attached to it,
- and the audit owner responsible for it.

If the proposal cannot be localized this way, it should not be admitted.

#### 28.4.3 What provenance survives after optimization?

A feature that is semantically acceptable but destroys provenance is still too expensive for a language that wants graph-native debugging, profiling, and coding-agent workflows. Every proposal should therefore answer:

- what source identities survive into CGIR,
- what fused or staged ancestry survives lowering,
- what debug/perf identity survives backend emission,
- and how a crash or benchmark result would still be attributed to the feature.

#### 28.4.4 What is the rollback path?

Every feature should have a stated retreat path. If it turns out to be too costly, too unstable, or too socially fragmenting, can it be:

- demoted from core to boundary,
- demoted from authored source to generated artifact,
- kept as internal tooling only,
- or edition-gated and eventually deprecated?

If the answer is no, the admission bar should be correspondingly higher.

### 28.5 Permanent “never in core” checklist

The closure rule from Chapter 27 becomes more useful when phrased as a standing negative table rather than a one-time design note.

| Candidate | Why it keeps reappearing | Why it still stays out of core | Supported instead via |
|---|---|---|---|
| a second authored build/config language | familiarity from TOML/YAML/CMake ecosystems | duplicates facts already present in `Project` and creates a second semantic center | native `package` / `workspace` declarations plus generated lock, interface, and build artifacts |
| a primary syntax-rewriting macro system | expressive short-term escape hatch | grows into a shadow language and weakens graph-level staging and provenance | staged optic graphs, generated artifacts, and schema-driven plugin points |
| ambient exceptions as the ordinary control model | terse error propagation and legacy familiarity | hides control edges from CGIR, replay, and provenance | prisms, `Result`, `Option`, and any later control layer kept explicit |
| a BetterOptic hosted/freestanding/kernel dialect split | makes hard domains feel easier by narrowing semantics | fractures the language into personalities and duplicates policy | `RuntimeFamily`, `TargetProfile`, capability-gated host regions, and `BoundaryContract` |
| a blanket “stable ABI” slogan | sounds ecosystem-friendly | source, interface, runtime, and plugin compatibility are distinct contracts | explicit C-facing ABI first, opt-in plugin/runtime contracts later |
| multiple blessed package workflows | community pressure from different domains | fragments caching, artifact schemas, and social defaults | one first-party native package/workspace/build path |
| unstructured `unsafe` | convenience at hard boundaries | destroys summary completeness and auditability | `unsafe optic`, `BoundaryContract`, and the published soundness-budget ledger |
| a separate user-facing query language for `Project` | query ergonomics and tooling power | recreates a second meta-language beside ordinary code | ordinary optic-based project queries with internal QIR lowering |
| plugins over unstable internal compiler IRs | rapid experimentation | creates a shadow compiler API that is harder to migrate than the language core | versioned graph protocol, schema-driven tool/plugin surfaces, generated interface artifacts |

The table should be treated as a standing rebuttal to feature creep. A proposal that matches one of these rows starts from “no” and must prove that it does not in fact recreate the excluded pattern under a new name.
The same test should be applied to mathematically rich proposals. Hyperreal or nonstandard-analysis ideas, for example, may be extremely valuable as analysis artifacts, solver-generation helpers, or approximation witnesses. They should still enter through the quarantined experimental lane first rather than by immediately changing the core numeric tower.

### 28.6 What coding agents still struggle with in large codebases

The current generation of coding agents is already useful, but the public evidence points to a set of recurring weak points that matter directly to language and compiler architecture.

#### 28.6.1 Context is scarce, and monolithic guidance rots

Anthropic's engineering guidance on context engineering is explicit that context is finite, that models show “context rot” as token counts rise, and that long-horizon tasks need compaction, structured notes, and often multi-agent decomposition rather than one ever-growing conversation. OpenAI's own engineering notes on Codex report a parallel lesson: a single giant `AGENTS.md` failed because it crowded out task-relevant information, decayed quickly, and was hard to verify mechanically. Those two sources point to the same operational conclusion: repository knowledge must be structured, queryable, and freshness-aware instead of poured into one static manual.

#### 28.6.2 Repository boundaries and tool envelopes are still narrow

GitHub's own documentation for Copilot coding agent states that, by default, the agent only accesses context in the current repository, cannot modify multiple repositories in one run, and opens one pull request per assigned task. It also documents compatibility limits around rulesets, hosted-runner assumptions, and content exclusions. These are sensible safety constraints, but they expose a real implementation problem for large systems: many important changes cross repository, artifact, or policy boundaries that the default agent envelope does not naturally see.

#### 28.6.3 Experience reuse is fragile and poor retrieval can make agents worse

SWE-ContextBench was proposed precisely because repository-level software engineering is not a sequence of independent tasks. Its abstract reports that the ability to accumulate, retrieve, and apply prior experience across related tasks had been under-measured, and that correctly selected summarized experience improves accuracy while reducing runtime and token cost. Just as important, incorrectly selected or unfiltered experience gives limited or negative benefit. In other words, memory helps only when its retrieval and summarization are right.

#### 28.6.4 Real software-engineering tasks remain difficult, and environment variability matters

OpenAI's SWE-Lancer benchmark reports that frontier models still fail on the majority of real-world freelance software engineering tasks. The same benchmark was later updated to remove the requirement for internet connectivity during execution specifically because it was a major source of evaluation variability. This is a useful caution: repository reasoning is hard enough even before hidden environment dependence is added on top.

### 28.7 What the architecture should do in response

The point of the previous section is not to chase the weaknesses of today's agents reactively. It is to read those weaknesses as design pressure on the language itself.

A language that wants to cooperate well with coding agents at large-codebase scale should therefore front-load the following architectural responses.

1. **Repository knowledge must be the system of record.** The earlier `ProjectGraph` decision is not just elegant; it is the right answer to stale manuals and context crowding. Agents should query graph-native summaries, interfaces, diagnostics, and provenance rather than consume one monolithic instruction file.
2. **The build graph and artifact graph must be semantically visible.** Multi-repository or multi-artifact work should be represented in the graph rather than disappearing into external scripts. This is what lets the toolchain grow beyond single-repository task envelopes without inventing a second language.
3. **Summaries and provenance must be stable enough to serve as retrieval anchors.** A coding agent needs something better than fuzzy search. `OpticSummary`, `BoundaryContract`, `RegionSet`, stable node ids, and `PerfKey`-style semantic identities are the natural retrieval units.
4. **Diagnostics must remain structured and replayable.** Large-codebase work fails expensively when the agent cannot tell whether it broke source compatibility, interface compatibility, runtime-family assumptions, or artifact identity. The diagnostic schema already points in the right direction; it should remain a first-class protocol.
5. **Experience reuse must be selective, not ambient.** The lesson from SWE-ContextBench is that summaries and prior traces help only when they are retrieved and compressed well. That is an argument for queryable project graphs and explicit artifact classes, not for giant persistent transcripts.
6. **Environment dependence must be explicit.** The SWE-Lancer update is a reminder that hidden external dependencies make evaluation and repair brittle. Build-time capabilities, runtime-family declarations, and target profiles should therefore stay explicit and hashable.

These responses are not “AI features” layered onto the language. They are reasons to keep `Project`, native package declarations, graph-native tooling, and optic-based project queries central.

### 28.7.1 From agent failure modes to a repository agent operating system

The architectural response should therefore not stop at better prompts or larger context windows. A language project that expects serious agent participation should ship a **repository agent operating system**: one coordinating role, a small number of sharply bounded specialists, explicit work packets, explicit shared memory, explicit per-agent memory, and a generated context index derived from checked-in state rather than from one long transcript.

This is the smallest structure that directly answers the documented weak points. Context crowding is handled by specialist isolation and compact memory. Cross-file drift is handled by an orchestrator and coherence review. Verification remains explicit because the task packet names the expected validation steps. Experience reuse becomes selective because stable lessons are promoted into shared memory while temporary findings remain attached to the task that discovered them.

### 28.7.2 Canonical hub, thin wrappers, explicit memory

The repository agent system should also follow the same closure rule the language applies elsewhere: one semantic center, many adapters. For agent tooling that means one canonical instruction hub (`AGENTS.md`) plus thin wrappers for tool-specific entry points such as `CLAUDE.md`, `GEMINI.md`, GitHub Copilot instruction files, and Kilo agent files. The wrappers should narrow or point back to the canonical contract, not fork it.

The same rule applies to memory. Auto-memory features in individual tools can help, but the canonical repository memory should remain checked in, reviewable, and tool-agnostic: one shared memory file for stable repo-wide truths and one bounded memory file per agent role. That keeps the system portable across tools and legible to human maintainers.

### 28.7.3 The extrapolation to human-driven software development is direct

This structure is useful not because software development is becoming less human, but because large development efforts already have the same shape when they work well: one coordinator, several specialists, explicit work packets, explicit review, and short written memory that survives personnel changes. The repository agent operating system is therefore best understood as a codification of good software-development practice that happens to be executable by coding agents as well as humans.

Appendix K records one concrete file-level realization of this idea for the split book package: role files, memory ledgers, task packet templates, compatibility wrappers, and a small maintenance script that keeps the context index in sync with the checked-in system definition.


### 28.7.4 Repository memory should be typed, selective, and queryable

The research on coding-agent failure modes points toward a precise architectural response: the repository should preserve the small subset of maintenance knowledge that is durable enough to matter again, and it should preserve it as structured data rather than as one long transcript.

For Optic, the right default is to keep that durable subset inside the same semantic world as the compiler graph whenever possible. That means accepted decisions, validated repair records, benchmark explanations, runtime-family and boundary notes, and long-lived task state should all be eligible for graph-native representation. They become another query surface over `Project`, not a second, informal knowledge base.

The corresponding negative rule is just as important. Scratch notes, speculative search trails, giant chat logs, and opaque embedding caches should not become authoritative graph truth. They may help a local tool, but they do not deserve the same trust class as source, summaries, diagnostics, or interfaces.

### 28.7.5 Failed patches are advisory negative knowledge, not bans

Large codebases accumulate negative knowledge whether they admit it or not. A patch was tried. It broke alias safety, regressed a benchmark, failed translation validation, or was rejected because it weakened a boundary contract. If that information is lost, future agents and humans will eventually rediscover the same failure the expensive way.

The language and toolchain should therefore support a typed failed-patch record with at least: goal, target nodes or regions, patch fingerprint, attempted revision, target profile, outcome class, reason class, evidence references, supersession link, and expiry or revalidation policy.

The design principle is caution without taboo. A failed-patch record is a warning backed by provenance and evidence. It is not a permanent prohibition. Future agents should use it to de-rank similar proposals and to inspect prior evidence before retrying them, not to freeze the design around stale mistakes.

### 28.8 A concrete review procedure for future proposals

A proposal review should be short, repeatable, and ruthless enough that the language does not drift.

#### Step 1 — classify the proposal

State in one sentence:
- the use case,
- the lane (`core`, `boundary`, `generated artifact`, `internal toolchain`),
- and the compatibility layers affected.

If that sentence cannot be written clearly, the proposal is not ready.

#### Step 2 — answer the checklist

Every proposal document should answer at least these questions directly:

1. What exact use case is unlocked?
2. Which lane carries it?
3. What earlier artifact proves it is legal?
4. What stable identity does its output have?
5. Which compatibility surface does it affect?
6. What proof, if any, does it bypass?
7. What provenance survives optimization and lowering?
8. What is the rollback path if the proposal proves too costly?

#### Step 3 — classify the outcome

A proposal should end in exactly one of these outcomes.

- **Accept into core** — if the use case is new, the summary/lowering story is explicit, and no second semantic center is introduced.
- **Support at the boundary** — if the use case is real but belongs behind `BoundaryContract`, `unsafe optic`, generated bindings, runtime-family declarations, or explicit capabilities.
- **Emit as generated artifact** — if the need is real but should live as compiler-emitted data rather than authored syntax.
- **Keep internal** — if the mechanism is useful for the compiler or tooling but should not become part of the language surface.
- **Accept into the experimental lane** — if the idea is promising, measurable, and worth implementing, but still too unstable for the ordinary language contract.
- **Reject / never in core** — if the proposal recreates one of the permanently excluded patterns.

#### Step 4 — add a regression anchor

Every accepted proposal should add one new lasting anchor to the repository:
- a fixture,
- a benchmark,
- an artifact-schema example,
- a diagnostic case,
- or a translation-validation check.

That is how the checklist remains part of the implementation rather than a policy memo.

### 28.9 Final closing argument

The purpose of this chapter is not to freeze the language out of fear. It is to freeze the *review discipline* so the language can continue to grow without becoming structurally incoherent.

The recurring lesson from both mature ecosystems and current coding-agent behavior is the same: large systems fail when they rely on hidden context, vague compatibility promises, unversioned boundaries, and too many unofficial ways to say the same thing. Optic already has a stronger starting point than most languages because it treats runtime roots, summaries, grades, boundary contracts, graph revisions, and generated artifacts as explicit semantic objects. The checklist simply makes that advantage harder to lose.

### 28.10 Transition to the appendices

The appendices that follow are now easier to read in operational terms. They are not just loose reference material. They are the concrete schemas, ladders, contracts, and query surfaces that support the review discipline established in this chapter.
