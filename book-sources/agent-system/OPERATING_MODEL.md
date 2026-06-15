# Operating Model

## 1. Why this system exists

Large, evolving codebases fail when every task is handled through one growing conversation or one giant manual. The repository therefore uses a **supervisor + specialists + explicit work packets + explicit memory** design.

This design is compatible with:
- tools that support subagents and separate contexts directly,
- tools that only support one active agent but can follow structured task packets,
- and human-driven software development where the same decomposition still improves coherence.

## 2. Core topology

### Orchestrator
The orchestrator is the only role that should perform broad task decomposition, arbitration, and merge-level decisions by default.

Responsibilities:
- interpret the user request,
- decide whether the task is single-scope or multi-scope,
- create or update the work packet,
- delegate to specialists,
- merge outputs,
- request coherence review,
- request release assembly.

### Specialists
Specialists are narrow and context-bounded.

- `research-librarian` — external research, source gathering, historical checks
- `book-architect` — chapter placement, structure, flow, cross-references
- `chapter-editor` — local content edits and stylistic tightening
- `coherence-auditor` — contradiction, numbering, duplication, and drift review
- `tooling-compatibility` — instruction files, protocol docs, and tool adapters
- `release-assembler` — manifest discipline, generated indexes, and final assembly

## 3. Mutual delegation without chaos

The system supports **mutual delegation**, but not unrestricted recursive spawning.

The rule is:
- a specialist may request another specialist,
- but the request should usually be surfaced through the orchestrator or a documented handoff note,
- unless the active tool explicitly supports nested subagents safely and the work packet allows it.

This keeps the workflow compatible across tools that differ in subagent support.

## 4. Work packet lifecycle

Every multi-file or research-heavy task should use a packet.

Lifecycle:
1. `inbox` — request captured, not yet decomposed
2. `active` — orchestrator has claimed and subdivided the task
3. `review` — changes complete; waiting for coherence audit or human review
4. `done` — merged into repository state and memories updated

A packet should identify:
- objective,
- owner,
- delegated subtasks,
- touched files,
- acceptance checks,
- and output expectations.

## 5. Parallelization rules

Parallel work is encouraged when:
- subtasks are structurally independent,
- file ownership is disjoint,
- and the merge step is clear.

Good parallel splits:
- research vs local prose edit,
- chapter edit vs compatibility-file update,
- content patch vs coherence audit.

Bad parallel splits:
- two agents editing the same chapter section blindly,
- simultaneous renumbering and cross-reference rewriting without shared packet state,
- multiple agents all updating the shared memory file without serialization.

## 6. Coherence loop

Every non-trivial task should end with this loop:
1. specialist output completed
2. orchestrator merges or sequences outputs
3. coherence-auditor checks numbering, references, duplicated explanation, and package drift
4. release-assembler regenerates indexes and the assembled manuscript
5. shared memory, failed-patch records, and per-agent memory are updated if needed

## 7. Human-driven extrapolation

This same system is meant to scale to human teams.

Humans map naturally onto the same roles:
- product/editorial lead as orchestrator,
- researcher,
- architect,
- section editor,
- release/operations owner,
- reviewer.

The point is not to automate humans away. It is to make the collaboration protocol explicit enough that humans and agents can share it.


## 8. Failed-patch review loop

Before proposing a broad rewrite or retrying a previously difficult repair, the orchestrator or specialist should check `agent-system/memory/FAILED_PATCH_RECORDS.md` for similar failures. If a new attempt fails in a reusable way, record:
- what was attempted,
- what evidence proved failure,
- whether the warning should expire or await revalidation,
- and which later patch superseded it if one exists.

This keeps negative knowledge concise, reviewable, and reusable without turning it into a permanent ban list.
