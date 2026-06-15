---
applyTo: "manifest.json,assemble_book.py,assembled.md,README.md"
---

Assembly rules:
- `manifest.json` is authoritative for assembly order.
- `assemble_book.py` should remain lossless and deterministic.
- `assembled.md` is generated output; never treat it as the source of truth.
- If a new chapter or appendix is added, update both the manifest and the frontmatter contents list.
