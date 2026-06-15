# Optic Book Split Sources

This directory contains the book split into front matter, part-intro files, chapter files, and appendix files.

- `00_frontmatter.md` contains the title page, abstract, preface, introduction, reading guide, reader orientation, and contents.
- `part-*/00_part_intro.md` contains the part heading plus the part-level transition prose.
- `part-*/*.md` files contain one numbered chapter each.
- `appendices/00_appendices_intro.md` contains the appendices heading and bridge paragraph.
- `appendices/appendix-*.md` files contain one appendix each.
- `manifest.json` records the assembly order.
- `assemble_book.py` reassembles the split sources into a single Markdown file without changing content.

The files are stored as exact text slices of the assembled manuscript so reassembly is lossless.

## Repository agent operating system

This package now also contains a cross-tool, self-maintaining repository agent system.

Canonical files:
- `AGENTS.md` — canonical instruction hub
- `CLAUDE.md`, `GEMINI.md`, `CONTEXT.md` — tool wrappers
- `.github/copilot-instructions.md` and `.github/instructions/*.instructions.md` — Copilot-compatible instructions
- `.claude/agents/*.md` — Claude Code project subagents
- `.kilo/agents/*.md` — Kilo-compatible custom agents
- `agent-system/` — operating model, memory model, task packets, generated indexes
- `tools/agent_sync.py` — validates the agent system and regenerates indexes

Suggested maintenance sequence:
1. edit split sources and/or agent-system files
2. run `python tools/agent_sync.py`
3. run `python assemble_book.py assembled.md`
