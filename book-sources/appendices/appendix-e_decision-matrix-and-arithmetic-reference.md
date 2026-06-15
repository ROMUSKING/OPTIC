## Appendix E — Decision Matrix and Arithmetic Reference

This appendix condenses the book's most reused quantitative and decision-level facts into one working reference. It is intentionally compact: the main text gives the arguments, while this appendix keeps the constants, default assumptions, loop-shape rules, and maintenance tests close at hand for implementation work.

### E.1 Decision matrix

- **Runtime root:** explicit `Runtime`; requires region-rooted summaries; pays off as explicit host-versus-semantic separation; primarily touches `RES-*`, `OBS-*`, and `KRN-*` diagnostics.
- **State/context model:** optic over costate; requires `OpticSummary`; pays off as direct loops and address arithmetic; primarily touches `TYP-*` and `GRA-*`.
- **Error model:** prisms and typed results; requires branch nodes and prism summaries; pays off as branch hints and mask lowering; primarily touches `OPT-*` and future prism diagnostics.
- **Iteration model:** traversal; requires traversal-legality flags; pays off as SIMD and dense loop formation; primarily touches `VEC-*` and `FUS-*`.
- **Infinite behavior:** coinduction; requires coinductive CGIR nodes; pays off as event loops and queue-aware lowering; primarily touches `OPT-*` and future liveness diagnostics.
- **Specialization:** staging; requires `Stage` nodes and a specialization cache; pays off as monomorphic hot code; primarily touches `FUS-*` and `PER-*`.
- **Ownership:** graded dimension; requires an ownership-aware alias checker; pays off as lock-free or lock-minimized proven-exclusive paths; primarily touches `ALI-*` and `GRA-*`.
- **Alias metadata:** region-based TBAA; requires region trees; pays off as better reordering and vectorization; primarily touches `CGI-*` and `LLV-*`.
- **Backend bring-up:** Rust first, then LLVM; requires a translation-validation harness; pays off as faster semantic debugging; primarily touches `COD-*` and `LLV-*`.
- **Diagnostics:** stable machine-readable schema; requires structured evidence and ranked fixes; pays off as agent efficiency and reproducibility; touches every diagnostic family.

### E.2 Hardware constants and default assumptions

- **Cache line size:** 64 bytes.
- **Page size:** 4 KiB.
- **AVX2 vector width:** 256 bits.
- **AVX-512 vector width:** 512 bits.
- **Common scalar sizes:** `u8=1`, `u32=4`, `u64=8`, `f32=4`, `f64=8`.
- **`Vec2<f32>` stride:** 8 bytes.
- **Padded `Vec3<f32>` stride:** usually 16 bytes in SIMD-friendly layouts.

These defaults are not global truths. They are the assumed baseline for the first target profiles and for the book's arithmetic examples.

### E.3 Grade defaults by domain

- **Kernels:** ownership, latency, blocking, DMA/MMIO, and liveness.
- **Browsers:** cache, staging, traversal, and latency.
- **Databases:** I/O, cache, transaction/session, and staging.
- **Games:** cache, traversal/SIMD, parallel ownership, and latency.
- **Compilers:** compile-time, cache, staging, and provenance.
- **Services:** latency, bandwidth, coinductive liveness, and replay determinism.

### E.4 Loop-shape cookbook

- **Single lens map:** one scalar SoA loop.
- **Product of lenses:** one multi-load/store loop.
- **Traversal with pure arithmetic:** vector loop plus scalar tail.
- **Prism plus traversal:** branchy loop, or a mask plus compact pass when profitable.
- **Composed lens chain:** one fused loop with register-resident intermediate.
- **Coinductive pipeline:** ring or event loop with explicit yield points.
- **Staged operator graph:** one monomorphic specialized loop or function.

### E.5 Maintenance rule

When a future revision proposes a new abstraction, it should be added to this appendix only after the following are written down explicitly:

1. its semantic obligation,
2. its compiler artifact,
3. its legality condition,
4. its machine consequence,
5. the rejected alternative.

If any of those are missing, the feature is not ready for the core language.

### E.6 Closing note

The design is ambitious, but the book's real argument is modest: keep the semantic core small enough that it can be proven in code, then let every later feature justify itself by the same standard.

That is the discipline that makes the full language credible. It is also the discipline that makes the path to kernels, browsers, databases, games, compilers, and a self-hosted ecosystem believable rather than theatrical.

---

