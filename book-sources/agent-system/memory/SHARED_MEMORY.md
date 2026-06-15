# Shared Repository Memory

## Canonical truths

- The split source files are authoritative.
- `assembled.md` is generated and should not be edited directly.
- `manifest.json` is the assembly order authority.
- `AGENTS.md` is the canonical cross-tool instruction hub.
- Tool-specific wrappers (`CLAUDE.md`, `GEMINI.md`, Copilot instructions, Kilo files) should not fork policy.
- After agent-system edits, run `python tools/agent_sync.py`.
- After manuscript edits, run `python assemble_book.py assembled.md`.

## Stable workflow truths

- Multi-file or research-heavy work should use a task packet.
- Shared memory holds stable repo truths; per-agent memory holds local tactics.
- The orchestrator or a human is the default cross-agent scheduler.

- Durable, shared maintenance knowledge should be kept in checked-in memory files; scratch notes and long transcripts should not.
- Failed patch knowledge is advisory and lives in `memory/FAILED_PATCH_RECORDS.md`, with evidence and expiry rather than permanent bans.
