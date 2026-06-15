---
applyTo: "AGENTS.md,CLAUDE.md,GEMINI.md,CONTEXT.md,agent-system/**/*.md,.claude/**/*.md,.kilo/**/*.md,.github/copilot-instructions.md,tools/agent_sync.py"
---

Agent-system rules:
- `AGENTS.md` is canonical; tool-specific files should wrap or narrow it, not contradict it.
- Keep role definitions crisp and non-overlapping.
- The orchestrator owns cross-agent decomposition.
- Shared memory holds repo-wide stable truths; per-agent memory holds role-local tactics and recent learnings.
- Favor concise, high-signal guidance over large monolithic manuals.

- Durable failed-patch knowledge belongs in `agent-system/memory/FAILED_PATCH_RECORDS.md` when it has evidence and future reuse value.
- Do not treat failed-patch records as permanent bans; require evidence, expiry, or supersession.
