## Appendix K — Repository Agent Operating System and Cross-Tool Compatibility

This appendix documents the repository-local agent operating system shipped with the split book package. It is not a second semantics for the language. It is a practical, cross-tool collaboration layer built from the same architectural convictions as the rest of the manuscript: one canonical center of truth, explicit summaries and identities, explicit task boundaries, explicit memory, explicit generated artifacts, and explicit validation.

### K.1 Why the package ships an agent operating system at all

The main text argues that large codebases become fragile when too much knowledge is left ambient. That argument applies just as much to the maintenance of the book package itself. The package therefore ships a small operating system for coding agents and human collaborators so that:

- instructions do not fragment across tools,
- work decomposition is explicit,
- context and memory are bounded and reviewable,
- and the single-file release artifact is always regenerated from authoritative split sources.

### K.2 Canonical file roles

| File or directory | Role | Authority level |
|---|---|---|
| `AGENTS.md` | canonical cross-tool instruction hub | canonical |
| `CLAUDE.md` | Claude wrapper and loader | wrapper |
| `GEMINI.md` | Gemini wrapper | wrapper |
| `CONTEXT.md` | Kilo-compatible wrapper | wrapper |
| `.github/copilot-instructions.md` | repository-wide Copilot guidance | wrapper |
| `.github/instructions/*.instructions.md` | path-specific Copilot narrowing | wrapper |
| `.claude/agents/*.md` | Claude project subagents | tool-specific agent files |
| `.kilo/agents/*.md` | Kilo custom agents | tool-specific agent files |
| `agent-system/registry.json` | canonical role registry | canonical |
| `agent-system/memory/SHARED_MEMORY.md` | stable shared repository memory | canonical |
| `agent-system/memory/agents/*.md` | role-specific memory | canonical but local in scope |
| `agent-system/tasks/` | explicit task graph | canonical workflow state |
| `agent-system/generated/*.md` | generated context and compatibility indexes | generated |
| `tools/agent_sync.py` | validates and regenerates the system indexes | generated-artifact maintenance tool |

### K.3 Agent topology

| Agent | Main responsibility | Normal write surface | Typical delegation targets |
|---|---|---|---|
| `orchestrator` | decompose, sequence, merge, and close tasks | task packets, shared memory, broad coordinating edits | every specialist |
| `research-librarian` | gather external evidence and historical comparisons | task packets, Appendix G-related material | none by default |
| `book-architect` | chapter placement, ordering, and cross-link structure | frontmatter, chapter structure, manifest | editor, auditor |
| `chapter-editor` | local prose, tables, examples, and bounded chapter edits | split source chapters and appendices | auditor |
| `coherence-auditor` | numbering, duplication, contradiction, and drift review | task notes, local memory, targeted correction suggestions | none by default |
| `tooling-compatibility` | instruction files and tool-specific wrappers | AGENTS / CLAUDE / GEMINI / Copilot / Kilo files | release-assembler |
| `release-assembler` | generated indexes, validation, and final assembly | generated context indexes, `assembled.md` | none by default |

### K.4 Task packets are the unit of delegated work

The operating system uses one explicit work unit: the task packet. A task packet names the objective, owner, dependencies, read/write set, delegated subtasks, validation steps, and expected outputs.

Packets move through four states:

1. `inbox` — captured but not decomposed,
2. `active` — claimed and subdivided,
3. `review` — waiting on coherence or human approval,
4. `done` — merged and archived.

This makes multi-agent work visible and reviewable instead of burying it in chat history.

### K.5 Shared memory versus per-agent memory

The memory system mirrors the findings summarized in Chapter 28 and Appendix G: context helps only when it stays selective.

- **Shared memory** is for stable repo-wide truths that should affect many future tasks.
- **Per-agent memory** is for role-specific tactics, recurring pitfalls, and unresolved concerns.
- **Failed-patch records** are advisory negative knowledge: they capture a previously attempted change, the evidence that it failed, and whether the warning is still current.

Promotion rule: move a fact from per-agent memory to shared memory only when it is stable, cross-role, and likely to matter again.

Compaction rule: memory files are not transcripts. They should contain durable facts and concise tactics, not full historical conversation.

#### K.5.1 Durable graph-worthy memory versus sidecar scratch

The operating system should mirror the long-range `ProjectGraph` policy described in the main text.

- Keep **durable, semantic, shared** maintenance knowledge close to the authoritative repository memory.
- Keep **large diffs, transcripts, speculative notes, and ranking caches** outside the authoritative memory surface.

In the split package this means checked-in markdown ledgers stand in for the future graph-native memory arena. Later, the same categories can be moved into an actual `AgentMemoryArena` without changing the policy.

#### K.5.2 Failed-patch record schema

A failed patch should be remembered as a caution record, not a ban. A useful minimal schema is:

```text
FailedPatchRecord {
  id,
  goal,
  target_nodes_or_files,
  patch_fingerprint,
  attempted_at_revision,
  outcome,
  reason_class,
  evidence_refs,
  superseded_by,
  revalidate_after,
  status,
}
```

Good outcome classes include: `TypeRejected`, `AliasRejected`, `TestFailed`, `PerfRegression`, `HumanRejected`, and `Superseded`. Good reason classes include: `Unsound`, `Incomplete`, `Policy`, `Regression`, and `Duplicate`.

The important rule is that failed-patch records are **advisory**. Future agents and reviewers should use them to de-rank similar proposals and inspect prior evidence, not to turn one bad attempt into an eternal prohibition.

### K.6 Mutual delegation and subdivision rules

The system supports mutual delegation, but in a disciplined way. A specialist may request another specialist, yet broad cross-agent scheduling should normally be routed through the orchestrator or a human coordinator. This keeps the system compatible with tools that differ in their support for nested subagents or multi-agent teams.

The guiding rule is simple: let specialists stay narrow, and let the orchestrator own graph-wide coordination.

### K.7 Extrapolation to human-driven development

The same structure improves ordinary human software development. Most successful engineering teams already converge on:

- a coordinator or lead,
- specialists,
- bounded work packets,
- explicit review,
- and short written memory that survives handoff.

The repository agent operating system is therefore not a robot-only layer. It is a codification of good large-codebase practice that both humans and agents can follow.

### K.8 Generated indexes and self-maintenance

The package includes a maintenance script, `tools/agent_sync.py`, that validates the file set and regenerates:

- `agent-system/generated/context-index.md`
- `agent-system/generated/tool-matrix.md`

This keeps the system self-maintaining in a narrow, reviewable sense: the canonical role registry is checked in, the generated indexes are reproducible, and drift between the registry, memory files, and tool wrappers becomes visible immediately.

### K.9 Cross-tool compatibility matrix

| Tool | Canonical files used in this package |
|---|---|
| Claude Code | `CLAUDE.md`, `.claude/agents/*.md`, `AGENTS.md` |
| OpenAI Codex | `AGENTS.md` |
| Gemini CLI | `GEMINI.md`, `AGENTS.md` |
| GitHub Copilot / VS Code Copilot | `.github/copilot-instructions.md`, `.github/instructions/*.instructions.md`, `AGENTS.md` |
| Kilo CLI / extension | `AGENTS.md`, `CONTEXT.md`, `.kilo/agents/*.md` |

The compatibility rule is that these files should form a **wrapper stack**, not several competing centers of truth.

### K.10 Operating rules worth keeping short and permanent

- Edit split sources, not `assembled.md`.
- Reassemble on output.
- Run `python tools/agent_sync.py` after changing the agent system.
- Use task packets for multi-file or research-heavy work.
- Keep memory compact and explicit.
- Keep `AGENTS.md` canonical and wrappers thin.
- Let the orchestrator or a human coordinator own wide merges and role arbitration.
