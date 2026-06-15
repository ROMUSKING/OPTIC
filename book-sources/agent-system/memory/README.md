# Memory Files

This directory contains the canonical, checked-in memory for the repository agent system.

- `SHARED_MEMORY.md` contains stable, repo-wide truths.
- `FAILED_PATCH_RECORDS.md` contains advisory negative knowledge about previously attempted changes.
- `agents/*.md` contains role-specific memory.

These files are intentionally small and should be maintained by task completion, not by copying chat transcripts.

The split between these files mirrors the longer-range graph policy in the book: durable, shared, query-worthy maintenance knowledge is kept close to the semantic center; large transcripts and speculative scratch work are not.
