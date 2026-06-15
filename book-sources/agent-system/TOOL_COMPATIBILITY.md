# Tool Compatibility

## Canonical rule

`AGENTS.md` is the canonical instruction hub.

Other files exist for compatibility and should wrap, narrow, or point back to `AGENTS.md` rather than fork the operating contract.

## Tool matrix

| Tool | Primary files in this repository | Notes |
|---|---|---|
| Claude Code | `CLAUDE.md`, `.claude/agents/*.md`, `AGENTS.md` | use project subagents and repo-local memory files; treat Claude auto memory as secondary |
| OpenAI Codex | `AGENTS.md` | use well-scoped work packets; parallelize by separate tasks when appropriate |
| Gemini CLI | `GEMINI.md`, `AGENTS.md` | GEMINI.md is a concise wrapper; detailed rules stay centralized |
| GitHub Copilot / VS Code Copilot | `.github/copilot-instructions.md`, `.github/instructions/*.instructions.md`, `AGENTS.md` | path-specific instructions keep context narrow |
| Kilo CLI / Kilo extension | `AGENTS.md`, `CONTEXT.md`, `.kilo/agents/*.md` | Kilo-specific agents mirror the canonical role set |

## Design consequences

- The canonical system must stay understandable without any one vendor’s proprietary feature.
- Role definitions should be expressible as markdown + frontmatter where possible.
- Context should be layered: canonical hub, path-specific instructions, role-specific agents, explicit memory.
