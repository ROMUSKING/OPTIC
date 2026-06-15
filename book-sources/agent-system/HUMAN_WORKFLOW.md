# Human Workflow

This repository agent system is designed for human-led development as much as for tool-led automation.

## Human role

Humans remain responsible for:
- setting intent,
- approving broad structural changes,
- resolving conflicting specialist outputs,
- deciding when a fact deserves promotion into shared memory,
- and accepting final release artifacts.

## Suggested human workflow

1. Start with or update a task packet.
2. Decide whether the task is single-scope or multi-scope.
3. Use the orchestrator pattern even if you are the only human involved.
4. Let specialists work on bounded subproblems.
5. Run coherence review before considering the task complete.
6. Reassemble the manuscript and review generated indexes.

## Why keep the protocol even for humans?

Because the same failure modes appear in human teams:
- context drift,
- contradictory local edits,
- duplicated explanations,
- and stale operational knowledge.

The agent system is meant to reduce those failures for both humans and tools.
