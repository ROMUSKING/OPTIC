# Repository Agent Operating System

This directory defines a **cross-tool, self-maintaining system of coding agents** for the split Optic implementation-book package.

It is designed around a small set of principles drawn from current agentic coding practice:
- context is finite and should be curated rather than accumulated blindly;
- specialized agents are useful when they isolate context, permissions, and responsibilities;
- well-scoped tasks parallelize better than monolithic prompts;
- memory helps only when it is selective, compact, and explicitly maintained;
- the same system should help both **agents** and **humans** collaborate with the codebase.

The system is intentionally conservative. It does **not** rely on hidden long chats as the repository memory. Instead it keeps:
- one canonical instruction hub (`AGENTS.md`),
- one explicit operating model,
- one shared memory ledger,
- one per-agent memory ledger,
- one failed-patch caution ledger,
- one work-packet template,
- one generated context index,
- and tool-specific wrappers for Claude Code, Codex, Gemini CLI, GitHub Copilot, and Kilo.

## Directory map

- `OPERATING_MODEL.md` — the overall coordination model
- `MEMORY_MODEL.md` — shared vs per-agent memory rules
- `TASK_PACKET_TEMPLATE.md` — the unit of delegated work
- `HUMAN_WORKFLOW.md` — how humans drive and review the system
- `TOOL_COMPATIBILITY.md` — tool-by-tool file and precedence map
- `RESEARCH_BASIS.md` — the external research and vendor guidance this system is based on
- `memory/SHARED_MEMORY.md` — canonical stable repository truths
- `memory/FAILED_PATCH_RECORDS.md` — advisory negative knowledge about previously attempted changes
- `memory/agents/*.md` — role-specific memory ledgers
- `tasks/` — inbox/active/review/done work packet flow
- `generated/` — generated indexes; do not hand-edit
- `registry.json` — canonical agent registry consumed by maintenance tooling

## Operating stance

Treat this system as a **repository operating system for collaboration**:
- the orchestrator manages task decomposition and handoff,
- specialists work in bounded scopes,
- memories are maintained explicitly,
- generated indexes are refreshed automatically,
- and the final single-file book is always rebuilt from split sources.

The entire system is meant to evolve with the codebase. If roles, memory files, or compatibility files drift, the maintenance script should expose that drift quickly.

The repository-local files are a practical stand-in for the longer-range graph-native memory policy described in the book: keep durable, shared, query-worthy knowledge close to the semantic center; keep scratchpads and long transcripts out of the authoritative memory surface.
