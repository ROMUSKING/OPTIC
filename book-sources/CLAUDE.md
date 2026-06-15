# Claude Code wrapper for the Optic Book package

@AGENTS.md
@agent-system/README.md
@agent-system/OPERATING_MODEL.md
@agent-system/MEMORY_MODEL.md

## Claude-specific notes

- Treat `AGENTS.md` as canonical. This file is a loader and wrapper, not a second instruction source.
- Use project subagents from `.claude/agents/` for focused work.
- Claude auto memory is helpful, but the **canonical shared repository memory** lives in `agent-system/memory/` so the system stays cross-tool and reviewable.
- If a specialist agent discovers the need for another specialist, return a delegation request in the task packet or handoff note rather than recursively expanding the task graph implicitly. This keeps the workflow compatible with tools and modes that do not support nested subagent spawning.
