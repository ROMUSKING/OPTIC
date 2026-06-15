# Memory Model

## 1. Goals

Repository memory should be:
- explicit,
- reviewable,
- selective,
- compact,
- and tool-agnostic.

## 2. Memory layers

### Shared memory
`memory/SHARED_MEMORY.md`

Use for facts that are:
- stable across many tasks,
- useful to more than one role,
- and harmful to rediscover repeatedly.

Examples:
- split sources are authoritative,
- `assembled.md` is generated,
- manifest order controls assembly,
- agent-sync must be run after agent-system edits,
- the canonical role map.

### Failed-patch records
`memory/FAILED_PATCH_RECORDS.md`

Use for:
- previously attempted fixes or rewrites that failed in a reusable way,
- benchmark or test regressions worth warning about later,
- reviewer rejections that should make future agents more cautious,
- and provenance-linked negative knowledge that is too important to leave in old task packets.

A failed-patch record should be:
- evidence-backed,
- explicitly scoped to a revision or file/node set,
- advisory rather than permanent,
- and superseded or expired when later work makes it obsolete.

### Per-agent memory
`memory/agents/<agent>.md`

Use for:
- recurring mistakes for that role,
- local tactics,
- recent unresolved concerns,
- role-specific checklists.

Examples:
- the coherence auditor’s common drift patterns,
- the release assembler’s validation sequence,
- the research librarian’s citation hygiene reminders.

## 3. Promotion and recording rules

Promote a fact from per-agent memory to shared memory only when:
- it applies across roles,
- it has survived at least two tasks,
- and it changes how the repository should normally be edited or validated.

Record a failed patch when:
- the same style of change is likely to be retried later,
- the failure has concrete evidence (diagnostic, benchmark, review note, or test),
- and the warning is strong enough to influence future repair ranking or human review.

## 4. Compaction rule

Do not let memory grow as a transcript.

A memory entry should be:
- one durable fact,
- one short tactic,
- or one explicit unresolved risk.

If an item is task-specific and expired, remove it or move it to the finished work packet rather than keeping it in memory.

## 5. Context-budget rule

Keep memory deliberately small.

Recommended soft limits:
- shared memory: ~200 lines max
- per-agent memory: ~120 lines max

The maintenance script checks these limits and reports drift.

## 6. Canonicality rule

Auto-memory features in specific tools may exist, but the canonical cross-tool memory for this repository is the checked-in `agent-system/memory/` tree.


## 7. Graph-native policy

The long-range target is to keep durable shared memory graph-native. In this repository package, checked-in markdown files play that role. Treat them as the durable semantic subset of agent memory, not as a place to dump transcripts or speculative scratch work.
