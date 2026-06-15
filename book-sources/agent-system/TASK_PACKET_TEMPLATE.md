# Task Packet Template

```md
---
id: task-YYYYMMDD-<slug>
summary: one-sentence objective
status: inbox|active|review|done
owner: orchestrator|research-librarian|book-architect|chapter-editor|coherence-auditor|tooling-compatibility|release-assembler|human
requested_by: user|human|agent:<name>
depends_on: []
delegates_to: []
related_failed_patch_records: []
read_set: []
write_set: []
reviewer: coherence-auditor|human
outputs:
  - updated split source files
  - updated compatibility files
  - updated memory
validation:
  - python tools/agent_sync.py
  - python assemble_book.py assembled.md
---

## Objective

## Constraints

## Proposed decomposition

## Notes / findings

## Merge plan

## Completion notes
```

## Rules

- Every packet should have a clear owner.
- `read_set` and `write_set` should be narrow.
- `delegates_to` should be explicit.
- `related_failed_patch_records` should name any caution records that materially shape the task.
- A finished packet should leave behind either merged output or a recorded blocker.
