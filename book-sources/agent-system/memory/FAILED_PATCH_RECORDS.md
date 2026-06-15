# Failed Patch Records

This file holds **advisory negative knowledge** for the repository agent system.

Use it when a previously attempted change is important enough that future agents or humans should be warned before retrying a similar idea.

## Policy

A failed-patch record should be:
- evidence-backed,
- scoped to a revision, target file set, or semantic area,
- concise,
- advisory rather than permanent,
- and superseded or expired when later work makes it obsolete.

Do **not** use this file for:
- raw transcripts,
- speculative worries without evidence,
- or blanket prohibitions with no revalidation path.

## Record template

```md
### FP-YYYYMMDD-<slug>
- status: tentative | confirmed | obsolete | superseded
- goal: one-sentence objective
- area: chapter / appendix / agent-system / tooling / semantic area
- attempted_at_revision: <commit-ish or release tag>
- patch_fingerprint: <short hash or description>
- outcome: TypeRejected | AliasRejected | TestFailed | PerfRegression | HumanRejected | Superseded
- reason_class: Unsound | Incomplete | Policy | Regression | Duplicate
- evidence:
  - diagnostic code / benchmark key / review note / task packet
- caution:
  - what future work should be wary of
- revalidate_after: <condition or date>
- superseded_by: <record or successful patch, if any>
```

## Records

_No durable failed-patch records have been captured yet._
