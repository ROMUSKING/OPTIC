# GitHub Copilot repository instructions

Use `AGENTS.md` as the canonical repository operating contract.

Repository-specific rules:
- Edit split sources, never `assembled.md` directly.
- Reassemble with `python assemble_book.py assembled.md` when content changes.
- After changing agent files, task packets, or memory files, run `python tools/agent_sync.py`.
- For multi-file or research-heavy changes, follow the work-packet process in `agent-system/TASK_PACKET_TEMPLATE.md`.
- Treat `agent-system/memory/SHARED_MEMORY.md` as the canonical shared repository memory.
- Prefer the smallest possible edit that preserves section numbering, cross-references, and manifest order.
