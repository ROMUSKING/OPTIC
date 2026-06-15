# Optic Book Agent Operating Contract

This file is the **canonical, cross-tool instruction hub** for this repository.

If a tool also supports `CLAUDE.md`, `GEMINI.md`, `.github/copilot-instructions.md`, or Kilo-specific files, those files should be treated as thin wrappers around this document and the `agent-system/` directory. Do not fork the operating rules across tools.

## Mission

Maintain the split-source manuscript, the assembly pipeline, and the repository's agent operating system **without creating a second semantic center**.

The repository has two truths:
- **authoritative**: split chapter/appendix/frontmatter files plus `manifest.json`
- **generated**: `assembled.md` and generated agent indexes

## Non-negotiable repository rules

1. **Edit split sources, not the assembled manuscript.**
   - Authoritative files live in the split chapter, appendix, and frontmatter sources.
   - `assembled.md` is generated output.
2. **Reassemble on output.**
   - Run `python assemble_book.py assembled.md` after content updates.
3. **Keep the agent system coherent.**
   - Run `python tools/agent_sync.py` after changing agent files, task packets, or memory files.
4. **Prefer small, typed work packets.**
   - Multi-file or research-heavy tasks should be decomposed into task packets in `agent-system/tasks/`.
5. **Promote stable truths; don't hoard context.**
   - Stable repository facts go to `agent-system/memory/SHARED_MEMORY.md`.
   - Failed but reusable patch knowledge goes to `agent-system/memory/FAILED_PATCH_RECORDS.md`.
   - Role-specific tactics and recent learnings go to `agent-system/memory/agents/<agent>.md`.
6. **One coordinating agent at a time.**
   - The orchestrator or a human acts as the cross-agent scheduler.
   - Specialists may request further delegation, but they should not recursively expand the task graph on their own unless the active tool explicitly supports it and the task packet says so.

## Where to start

Read these in order:
1. `agent-system/README.md`
2. `agent-system/OPERATING_MODEL.md`
3. `agent-system/MEMORY_MODEL.md`
4. `agent-system/TASK_PACKET_TEMPLATE.md`
5. `agent-system/memory/FAILED_PATCH_RECORDS.md` if the task resembles a previously failed change
6. the relevant per-agent memory file in `agent-system/memory/agents/`

## Canonical workflow

### Single-file, local edit
- Use the nearest appropriate specialist.
- Make the change.
- Update only the specialist memory if a reusable local lesson was learned.
- Reassemble if the change touched manuscript content.

### Multi-file or research-heavy edit
- Create or update a task packet from `agent-system/TASK_PACKET_TEMPLATE.md`.
- Let the orchestrator subdivide the work.
- Use specialists for research, editing, coherence review, tooling compatibility, and release assembly.
- Merge results back through the orchestrator or a human maintainer.
- Update shared memory only when a fact becomes repo-wide and stable.

## Agent topology

- `orchestrator` — primary coordinator; decomposes work, claims final responsibility, updates shared state
- `research-librarian` — external research, source harvesting, historical comparison, citation collection
- `book-architect` — chapter placement, section ordering, cross-links, flow
- `chapter-editor` — local prose, tables, examples, copyedits, scoped content changes
- `coherence-auditor` — numbering, duplicated explanations, contradiction checks, unresolved cross-references
- `tooling-compatibility` — AGENTS/CLAUDE/GEMINI/Copilot/Kilo compatibility files, protocol docs, config drift
- `release-assembler` — generated indexes, validation, manifest discipline, single-file reassembly

## Validation

Always prefer the smallest relevant validation step:
- `python tools/agent_sync.py` — validate agent file set and regenerate indexes
- `python assemble_book.py assembled.md` — rebuild the single-file manuscript
- targeted content grep / heading / manifest checks as required by the task packet

## Compatibility policy

- **Codex / AGENTS.md standard**: `AGENTS.md` is the main entry point.
- **Claude Code**: `CLAUDE.md` plus `.claude/agents/`.
- **Gemini CLI**: `GEMINI.md`.
- **VS Code Copilot**: `.github/copilot-instructions.md` plus `.github/instructions/*.instructions.md`.
- **Kilo CLI / extension**: `AGENTS.md`, `CONTEXT.md`, and `.kilo/agents/`.

## Hard prohibitions

- Do not edit `assembled.md` directly.
- Do not fork the agent operating rules separately for each tool.
- Do not store long conversational transcripts as memory.
- Do not turn one failed patch into a permanent ban without evidence, expiry, or supersession.
- Do not let a specialist rewrite repository-wide structure without an orchestrator or human review.
- Do not create new agent roles until the current topology demonstrably fails.
