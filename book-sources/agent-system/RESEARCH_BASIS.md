# Research Basis

This operating system is informed by a small set of public, durable lessons.

## Context management

- Anthropic's context engineering guidance: context is finite; long tasks need compaction, structured notes, and often multi-agent decomposition.
- SWE-ContextBench: experience reuse helps when retrieval and summarization are selective.

## Multi-agent topology

- LangChain's multi-agent guidance: use specialized components for context management, distributed development, and parallelization; choose subagents, handoffs, or router patterns according to task shape.
- OpenAI Codex guidance: parallel, well-scoped tasks work well; AGENTS.md improves navigation and testing discipline.

## Tool compatibility

- GitHub Copilot supports repository instructions, path-specific instructions, and agent instruction files.
- Claude Code supports `CLAUDE.md`, repo/user/project subagents, and separate context windows.
- Gemini CLI supports hierarchical `GEMINI.md` context files.
- Kilo supports `AGENTS.md`, `CLAUDE.md`, `CONTEXT.md`, and project-local custom agents.

## Extrapolation to human software development

The same structure improves human collaboration because humans also suffer from:
- stale project memory,
- overloaded working context,
- and unclear task ownership.

This system therefore treats agents and humans as users of one shared operating protocol.
