//! `optic` — stable narrow-v0 compiler facade (M6 library API).
//!
//! Re-exports pipeline entrypoints for embedding without depending on each crate.
//!
//! # Example using re-exported scale limit
//! ```
//! use optic::MAX_CGIR_NODES_V0;
//! assert!(MAX_CGIR_NODES_V0 >= 4096);
//! ```

pub use optic_cgir::{
    build as build_cgir, dump_node_pretty, find_node_by_id, is_m7_reserved, leaf_summary_by_id,
    m7_reserved_kind, node_span, node_summary, resolve_cgir_node, scale_limit_err_string,
    verify_to_diagnostic, CgirGraph, ResolveCgirNodeError, MAX_CGIR_NODES_V0, MAX_NODE_NAME_BYTES,
};
pub use optic_codegen_rust::emit as emit_rust;
pub use optic_diagnostics::Diagnostic;
pub use optic_hir::{lower, ConcreteGrade, HirProgram, OpticSummary};
pub use optic_opt::optimize;
pub use optic_syntax::{parse, ParseErrorKind, Program, SourceId, Span};
pub use optic_typeck::{
    check, collect_unsupported_surface, explain_focus, explain_focus_with_diags, explain_grade,
    explain_grade_with_diags, has_unsupported_surface, infer_grade_from_summary, typeck_pass,
    unsupported_for_node, FocusReport, GradeReport, TypedHir,
};

// The MAX_CGIR_NODES_V0 is re-exported via the optic_cgir pub use above (for embedders/harnesses; see doctest example in module docs).

/// Default source size cap (matches CLI appendix B guard).
pub const DEFAULT_MAX_SOURCE_BYTES: u64 = 4 * 1024 * 1024;

/// Outcome of a full `check` pipeline (typeck through codegen dry-run).
#[derive(Debug)]
pub struct CheckOutcome {
    pub typed_hir: TypedHir,
    pub fusion_notes: Vec<Diagnostic>,
}

/// Derive a stable `SourceId` from a filesystem path (for spans in file-based APIs).
pub fn source_id_from_path(path: &std::path::Path) -> SourceId {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.to_string_lossy().hash(&mut h);
    SourceId((h.finish() & 0x7fff_ffff) as u32 + 1)
}

/// CGIR build outcome (optionally pre- or post-fusion).
#[derive(Debug)]
pub struct CgirOutcome {
    pub graph: CgirGraph,
    pub fusion_notes: Vec<Diagnostic>,
}

fn lower_to_diags(errs: Vec<optic_syntax::ParseError>) -> Vec<Diagnostic> {
    errs.into_iter()
        .map(|e| {
            if let Some(optic_syntax::ParseErrorKind::DuplicateSoaCostate { costate }) = &e.kind {
                optic_diagnostics::hir_duplicate_soa_diag(e.span, costate, &e.message)
            } else {
                optic_diagnostics::resolve_diag(e.span, e.message)
            }
        })
        .collect()
}

/// Parse source to AST; returns structured diagnostics on failure.
pub fn parse_src(src: &str) -> Result<Program, Vec<Diagnostic>> {
    parse_src_with_id(src, SourceId(1))
}

/// Parse with an explicit `SourceId` (e.g. from `source_id_from_path`).
pub fn parse_src_with_id(src: &str, source_id: SourceId) -> Result<Program, Vec<Diagnostic>> {
    parse(src, source_id).map_err(|errs| {
        errs.into_iter()
            .map(|e| optic_diagnostics::parse_diag(e.span, e.message))
            .collect()
    })
}

/// Reject files with any deferred surface feature (same gate as `compile_check`).
pub fn reject_unsupported_surface(prog: &Program) -> Result<(), Vec<Diagnostic>> {
    let diags = optic_typeck::collect_unsupported_surface(prog);
    if optic_typeck::has_unsupported_surface(&diags) {
        Err(diags)
    } else {
        Ok(())
    }
}

/// True when diagnostics include OBS-701/OBS-702 observability surface rejections.
pub fn has_unsupported_observability(diags: &[Diagnostic]) -> bool {
    optic_typeck::has_unsupported_observability(diags)
}

/// Parse → lower; returns structured diagnostics on failure.
pub fn lower_src(src: &str) -> Result<HirProgram, Vec<Diagnostic>> {
    let prog = parse_src(src)?;
    reject_unsupported_surface(&prog)?;
    lower(prog).map_err(lower_to_diags)
}

/// Parse and return deterministic AST dump text.
pub fn dump_ast_src(src: &str) -> Result<String, Vec<Diagnostic>> {
    let prog = parse_src(src)?;
    reject_unsupported_surface(&prog)?;
    Ok(optic_syntax::dump_ast(&prog))
}

/// Parse → lower and return deterministic HIR dump text.
pub fn dump_hir_src(src: &str) -> Result<String, Vec<Diagnostic>> {
    Ok(optic_hir::dump_hir(&lower_src(src)?))
}

fn check_source_bytes(src: &str, max_bytes: u64) -> Result<(), Vec<Diagnostic>> {
    if src.len() as u64 > max_bytes {
        Err(vec![optic_diagnostics::parse_diag(
            Span::dummy(),
            format!("source exceeds {max_bytes} byte limit"),
        )])
    } else {
        Ok(())
    }
}

/// Read a source file with a byte cap (matches CLI `read_source` guard).
pub fn read_source_capped(
    path: &std::path::Path,
    max_bytes: u64,
) -> Result<String, Vec<Diagnostic>> {
    use std::io::Read;
    let f = std::fs::File::open(path).map_err(|e| {
        vec![optic_diagnostics::parse_diag(
            Span::dummy(),
            format!("open {}: {e}", path.display()),
        )]
    })?;
    let mut buf = Vec::new();
    f.take(max_bytes.saturating_add(1))
        .read_to_end(&mut buf)
        .map_err(|e| {
            vec![optic_diagnostics::parse_diag(
                Span::dummy(),
                format!("read {}: {e}", path.display()),
            )]
        })?;
    if buf.len() as u64 > max_bytes {
        return Err(vec![optic_diagnostics::parse_diag(
            Span::dummy(),
            format!("source {} exceeds {max_bytes} byte limit", path.display()),
        )]);
    }
    String::from_utf8(buf).map_err(|e| {
        vec![optic_diagnostics::parse_diag(
            Span::dummy(),
            format!("utf8 decode {}: {e}", path.display()),
        )]
    })
}

/// Parse → lower → typeck (shared by compile_* helpers).
fn compile_through_check(
    src: &str,
    max_bytes: u64,
    source_id: SourceId,
) -> Result<TypedHir, Vec<Diagnostic>> {
    check_source_bytes(src, max_bytes)?;
    let prog = parse(src, source_id).map_err(|errs| {
        errs.into_iter()
            .map(|e| optic_diagnostics::parse_diag(e.span, e.message))
            .collect::<Vec<_>>()
    })?;
    let unsupported = optic_typeck::collect_unsupported_surface(&prog);
    if optic_typeck::has_unsupported_surface(&unsupported) {
        return Err(unsupported);
    }
    let hir = optic_hir::lower(prog).map_err(lower_to_diags)?;
    let (typed, diags) = typeck_pass(hir);
    if diags.is_empty() {
        Ok(typed)
    } else {
        Err(diags)
    }
}

/// Parse → HIR → type/grade/alias → CGIR → fusion → verify → codegen dry-run.
pub fn compile_check(src: &str) -> Result<CheckOutcome, Vec<Diagnostic>> {
    compile_check_with_limit(src, DEFAULT_MAX_SOURCE_BYTES)
}

pub fn compile_check_with_limit(
    src: &str,
    max_bytes: u64,
) -> Result<CheckOutcome, Vec<Diagnostic>> {
    compile_check_with_limit_and_id(src, max_bytes, SourceId(1))
}

fn compile_check_with_limit_and_id(
    src: &str,
    max_bytes: u64,
    source_id: SourceId,
) -> Result<CheckOutcome, Vec<Diagnostic>> {
    let typed = compile_through_check(src, max_bytes, source_id)?;
    let cg = build_cgir(&typed)?;
    let outcome = optimize(cg).map_err(|d| vec![d])?;
    optic_cgir::verify_to_diagnostic(&outcome.graph).map_err(|d| vec![d])?;
    debug_assert!(
        !outcome.graph.nodes.is_empty() || outcome.graph.roots.is_empty(),
        "CGIR structure post-verify"
    );
    debug_assert!(
        outcome.graph.region_map.costate_name.is_empty()
            || !outcome.graph.region_map.columns.is_empty(),
        "region map consistency when present"
    );
    emit_rust(&outcome.graph, "optic_runtime")
        .map_err(|e| vec![optic_diagnostics::codegen_failed_diag(&e)])?;
    Ok(CheckOutcome {
        typed_hir: typed,
        fusion_notes: outcome.fusion_notes,
    })
}

/// Read a `.opt` file and run `compile_check` with a path-derived `SourceId`.
pub fn compile_check_from_path(path: &std::path::Path) -> Result<CheckOutcome, Vec<Diagnostic>> {
    compile_check_from_path_with_limit(path, DEFAULT_MAX_SOURCE_BYTES)
}

pub fn compile_check_from_path_with_limit(
    path: &std::path::Path,
    max_bytes: u64,
) -> Result<CheckOutcome, Vec<Diagnostic>> {
    let src = read_source_capped(path, max_bytes)?;
    let source_id = source_id_from_path(path);
    compile_check_with_limit_and_id(&src, max_bytes, source_id)
}

/// Build CGIR (and optionally run fusion). Post-fusion graphs are verified like `compile_check`.
pub fn compile_cgir(src: &str, before_fusion: bool) -> Result<CgirOutcome, Vec<Diagnostic>> {
    compile_cgir_with_limit(src, before_fusion, DEFAULT_MAX_SOURCE_BYTES)
}

pub fn compile_cgir_with_limit(
    src: &str,
    before_fusion: bool,
    max_bytes: u64,
) -> Result<CgirOutcome, Vec<Diagnostic>> {
    let typed = compile_through_check(src, max_bytes, SourceId(1))?;
    let cg = build_cgir(&typed)?;
    if before_fusion {
        // Note: build() now enforces MAX_CGIR_NODES_V0 (protects pre-fusion dump paths too; early return exercised in facade test).
        return Ok(CgirOutcome {
            graph: cg,
            fusion_notes: vec![],
        });
    }
    let outcome = optimize(cg).map_err(|d| vec![d])?;
    optic_cgir::verify_to_diagnostic(&outcome.graph).map_err(|d| vec![d])?;
    Ok(CgirOutcome {
        graph: outcome.graph,
        fusion_notes: outcome.fusion_notes,
    })
}

/// Full pipeline through Rust emission (includes CGIR verify).
pub fn compile_emit(src: &str) -> Result<String, Vec<Diagnostic>> {
    compile_emit_with_limit(src, DEFAULT_MAX_SOURCE_BYTES)
}

pub fn compile_emit_with_limit(src: &str, max_bytes: u64) -> Result<String, Vec<Diagnostic>> {
    let outcome = compile_cgir_with_limit(src, false, max_bytes)?;
    emit_rust(&outcome.graph, "optic_runtime")
        .map_err(|e| vec![optic_diagnostics::codegen_failed_diag(&e)])
}

/// Parse → partial check → explain normalized grade for a named optic or let binding.
/// Succeeds even when other items in the file have grade/type errors.
pub fn explain_grade_from_src(src: &str, node: &str) -> Result<GradeReport, Vec<Diagnostic>> {
    explain_grade_from_src_with_limit(src, node, DEFAULT_MAX_SOURCE_BYTES)
}

pub fn explain_grade_from_src_with_limit(
    src: &str,
    node: &str,
    max_bytes: u64,
) -> Result<GradeReport, Vec<Diagnostic>> {
    check_source_bytes(src, max_bytes)?;
    let prog = parse(src, SourceId(1)).map_err(|errs| {
        errs.into_iter()
            .map(|e| optic_diagnostics::parse_diag(e.span, e.message))
            .collect::<Vec<_>>()
    })?;
    let unsupported = optic_typeck::collect_unsupported_surface(&prog);
    let hir = optic_hir::lower(prog).map_err(lower_to_diags)?;
    let (typed, diags) = typeck_pass(hir);
    let mut combined = diags;
    combined.extend(unsupported);
    explain_grade_with_diags(&typed, node, &combined)
}

/// Parse → partial check → explain PathLift / root-path for a named optic or let binding.
pub fn explain_focus_from_src(src: &str, node: &str) -> Result<FocusReport, Vec<Diagnostic>> {
    explain_focus_from_src_with_limit(src, node, DEFAULT_MAX_SOURCE_BYTES)
}

pub fn explain_focus_from_src_with_limit(
    src: &str,
    node: &str,
    max_bytes: u64,
) -> Result<FocusReport, Vec<Diagnostic>> {
    check_source_bytes(src, max_bytes)?;
    let prog = parse(src, SourceId(1)).map_err(|errs| {
        errs.into_iter()
            .map(|e| optic_diagnostics::parse_diag(e.span, e.message))
            .collect::<Vec<_>>()
    })?;
    let unsupported = optic_typeck::collect_unsupported_surface(&prog);
    let hir = optic_hir::lower(prog).map_err(lower_to_diags)?;
    let (typed, diags) = typeck_pass(hir);
    let mut combined = diags;
    combined.extend(unsupported);
    explain_focus_with_diags(&typed, node, &combined)
}

#[cfg(test)]
#[allow(clippy::assertions_on_constants, clippy::single_element_loop)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // Selection for explicit `match` (vs .expect) on positive Results: targeted only to before_fusion early-return,
    // compile_emit Ok, and inner build_cgir decisions using real TypedHir from record/health + this-run health_get/set/alive/all_healths/tap (per cgir scale test
    // + harness patterns + prior facade continuation + additional cgir scale guard decision coverage). Other .expect kept for smallest delta / existing style.
    // Some Err paths use explicit match for decision coverage (e.g. TYP-010 in host_boundary); most negative tests
    // use .unwrap_err() for conciseness.
    // 2026-06-21: additional compile/build paths (from_path health_get, alive/all_healths prism/trav, query health_get/set, tap_record) converted to match exercising guards.
    // Err arms for new matches: covered by existing dedicated reject tests (facade_rejects_*, cgir build_err cases, unwrap_err tests); no bloat added.
    // 2026-06-21 continuation (this run): codegen helper assert_rust_golden + cgir integration large-N converted build to explicit match (real golden fixtures coverage via helper + synthetic capacity path); follows prior match harness/doctor style. (Note: facade compile_* outer matches transitively exercise inner build guards per compile_through_check path.)

    // Note: other facade tests use .expect/.unwrap_err for setup (pre-existing, test-only; prod paths use Result+match).
    // parse/lower/check .expect in cgir query/tap tests are setup-only (boilerplate common to all real-TypedHir tests; build decision is the exercised guard path per PLAN).
    // other bare evidence[] / .any() (e.g. TYPE_UNKNOWN inline + CGIR_UNSUPPORTED_EXPR/GRADE_COMPOSE_OVER + !any absences) left per smallest (PAR-001 hardened this pass; remaining left; no new coverage/tests added per smallest; see 2026 sub + PLAN).
    // Mixed styles + cross-refs (unknown-costate here; focus via CLI/json; typeck .any left): see facade_explain_grade_fails_typ001_on_target.
    fn example_src(name: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples")
            .join(name);
        std::fs::read_to_string(path).expect("read example")
    }

    fn count_ast_items(prog: &Program) -> (usize, usize, usize) {
        let mut optics = 0usize;
        let mut lets = 0usize;
        let mut data = 0usize;
        for item in &prog.items {
            match item {
                optic_syntax::Item::Optic(_) => optics += 1,
                optic_syntax::Item::Let(_) => lets += 1,
                optic_syntax::Item::Data(_) => data += 1,
                _ => {}
            }
        }
        (optics, lets, data)
    }

    /// PLAN §4: parse → dump_ast → re-parse same source yields stable AST shape/counts.
    #[test]
    fn smoke_ast_roundtrip_stable_item_counts() {
        for name in [
            "health_get.opt",
            "compose_triple.opt",
            "nested_position.opt",
        ] {
            let src = example_src(name);
            let prog1 = parse(&src, SourceId(1)).expect("parse");
            let dump1 = optic_syntax::dump_ast(&prog1);
            let prog2 = parse(&src, SourceId(1)).expect("re-parse");
            let dump2 = optic_syntax::dump_ast(&prog2);
            assert_eq!(
                dump1, dump2,
                "{name}: dump_ast must be stable across re-parse"
            );
            assert_eq!(
                count_ast_items(&prog1),
                count_ast_items(&prog2),
                "{name}: AST item counts must match"
            );
        }
    }

    /// Double parse produces identical AST dumps on positive goldens.
    /// Full `dump_hir` equality is asserted on `health_get.opt` (canonical); other examples
    /// only check optic-name sets and item counts to keep the smoke suite fast.
    #[test]
    fn smoke_parse_deterministic_on_positive_goldens() {
        for name in [
            "health_get.opt",
            "health_set.opt",
            "health_decay.opt",
            "health_position.opt",
            "compose_decay.opt",
            "compose_triple.opt",
            "nested_position.opt",
            "nested_field_triple.opt",
        ] {
            let src = example_src(name);
            let prog1 = parse(&src, SourceId(1)).expect("parse");
            let prog2 = parse(&src, SourceId(1)).expect("re-parse");
            assert_eq!(
                optic_syntax::dump_ast(&prog1),
                optic_syntax::dump_ast(&prog2),
                "{name}: AST dump must be deterministic"
            );
            let hir1 = lower(prog1).expect("lower");
            let hir2 = lower(prog2).expect("re-lower");
            let names1: HashSet<_> = hir1
                .items
                .iter()
                .filter_map(|i| match i {
                    optic_hir::HirItem::Optic { decl, .. } => Some(decl.name.node.clone()),
                    _ => None,
                })
                .collect();
            let names2: HashSet<_> = hir2
                .items
                .iter()
                .filter_map(|i| match i {
                    optic_hir::HirItem::Optic { decl, .. } => Some(decl.name.node.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(
                names1, names2,
                "{name}: optic names must match across lowers"
            );
            assert_eq!(
                hir1.items.len(),
                hir2.items.len(),
                "{name}: HIR item count stable"
            );
            if name == "health_get.opt" {
                assert_eq!(
                    optic_hir::dump_hir(&hir1),
                    optic_hir::dump_hir(&hir2),
                    "{name}: full HIR dump must be deterministic"
                );
            }
        }
    }

    /// Summary region roots must be declared SoA columns (conservative subset property).
    #[test]
    fn smoke_summary_regions_subset_of_declared_columns() {
        for name in [
            "health_get.opt",
            "health_set.opt",
            "health_decay.opt",
            "health_position.opt",
            "compose_decay.opt",
            "compose_triple.opt",
            "nested_position.opt",
            "nested_field_triple.opt",
        ] {
            let src = example_src(name);
            let hir = lower(parse(&src, SourceId(1)).expect("parse")).expect("lower");
            let map = optic_hir::build_region_map(&hir).expect("region map");
            let columns: HashSet<_> = map.columns.keys().cloned().collect();
            for item in &hir.items {
                let summary = match item {
                    optic_hir::HirItem::Optic { summary, .. } => summary,
                    optic_hir::HirItem::Let { summary, .. } => summary,
                    _ => continue,
                };
                for reg in summary
                    .get_reads
                    .iter()
                    .chain(&summary.put_reads)
                    .chain(&summary.put_writes)
                {
                    let root = reg.split('.').next().unwrap_or(reg.as_str());
                    if columns.contains(root) {
                        continue;
                    }
                    if summary.lift.prefix.iter().any(|p| p == reg) {
                        continue;
                    }
                    if reg.contains('.') && columns.contains(root) {
                        continue;
                    }
                    assert!(
                        false,
                        "{name}: region `{reg}` root `{root}` not in declared columns {:?} nor lift {:?}",
                        columns,
                        summary.lift.prefix
                    );
                }
            }
        }
    }

    #[test]
    fn facade_compile_check_positive() {
        // use record example to exercise real TypedHir (data Entities + Record hook + region_map) through build guards (match binds + asserts non-empty)
        let src = example_src("record_health.opt");
        // explicit match (not .expect) for full compile_check Ok decision on primary records/region_map fixture (exercises complete path + internal build guards/early+final scale); record_health is the canonical for Entities/Record
        let o = match compile_check(&src) {
            Ok(o) => o,
            Err(e) => panic!("compile_check must Ok for record_health.opt (real TypedHir non-exceed guard): {e:?}"),
        };
        debug_assert!(
            !o.typed_hir.items.is_empty(),
            "post-success CGIR/facade assert"
        );
        // inner build_cgir match for explicit decision coverage on TypedHir (post compile_check); follows harness style (now both outer compile + inner explicit)
        let g = match build_cgir(&o.typed_hir) {
            Ok(g) => g,
            Err(e) => {
                panic!(
                    "build must Ok for record_health.opt (real TypedHir non-exceed guard): {e:?}"
                )
            }
        };
        assert!(!g.nodes.is_empty());
    }

    #[test]
    fn facade_compile_emit_positive() {
        // explicit match (not .expect) for compile_emit Ok decision + post-assert; use nested to exercise real nested/Transform data path through emit (via compile_cgir(false); CGIR-build layer)
        // health_decay covered via golden_rust + execution + smoke lists (385/443).
        let src = example_src("nested_position.opt");
        let out = match compile_emit(&src) {
            Ok(o) => o,
            Err(e) => {
                panic!("compile_emit must Ok for nested_position.opt (real non-exceed): {e:?}")
            }
        };
        assert!(out.contains("run_example"));
    }

    #[test]
    fn facade_compile_cgir_before_fusion_positive() {
        // use record example to exercise before_fusion early return (build + direct return, no optimize) decision match on real TypedHir (data Entities + Record hook + region_map)
        // explicit match (not .expect) follows exact match-not-expect + post-asserts (pre-existing nodes; new fusion_notes contract for early return) from cgir scale tests + module selection criteria (token/AST/HIR -> CGIR-build)
        let src = example_src("record_health.opt");
        let outcome = match compile_cgir(&src, true) {
            Ok(o) => o,
            Err(e) => panic!("compile_cgir before_fusion must Ok for record_health.opt (real TypedHir non-exceed guard): {e:?}"),
        };
        assert!(!outcome.graph.nodes.is_empty());
        assert!(outcome.fusion_notes.is_empty()); // before_fusion early-return contract: fusion_notes must be empty
    }

    #[test]
    fn facade_explain_grade_invalid_grade_file() {
        let src = example_src("invalid_grade.opt");
        let report = explain_grade_from_src(&src, "BadCache").expect("explain despite GRA-110");
        assert_eq!(report.optic, "BadCache");
        assert_eq!(report.inferred.cache, 3);
    }

    #[test]
    fn facade_explain_grade_let_binding() {
        let src = example_src("nested_position.opt");
        let report = explain_grade_from_src(&src, "nested").expect("let binding");
        assert_eq!(report.optic, "nested");
        assert_eq!(report.inferred.cache, 4);
    }

    #[test]
    fn facade_explain_grade_fails_typ001_on_target() {
        let src = example_src("typ001_unknown_type.opt");
        let err = explain_grade_from_src(&src, "GhostView").unwrap_err();
        // explicit find.expect("TYP-001") on real typ001_unknown_type.opt fixture (terse harness style; explain-grade error path per self-host prep)
        err.iter().find(|d| d.code == "TYP-001").expect("TYP-001");
    }

    #[test]
    fn facade_explain_grade_inferred_affine_alias() {
        let src = example_src("health_get.opt");
        let report = explain_grade_from_src(&src, "HealthView").expect("explain");
        assert_eq!(
            report.inferred.ownership_alias.as_deref(),
            Some("AffineGrade")
        );
    }

    #[test]
    fn facade_compile_check_from_path_positive() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/health_get.opt");
        // explicit match (not .expect) for compile path decision on real fixture (exercises full compile through build guards; post token/AST/HIR -> CGIR-build)
        let outcome = match compile_check_from_path(&path) {
            Ok(o) => o,
            Err(e) => panic!(
                "compile_check_from_path must Ok for health_get.opt (real non-exceed): {e:?}"
            ),
        };
        assert!(!outcome.typed_hir.items.is_empty());
    }

    #[test]
    fn facade_rejects_obs701_on_compile_check() {
        for (name, method) in [
            ("unsupported_profile.opt", "profile"),
            ("unsupported_replay.opt", "replay"),
        ] {
            let src = example_src(name);
            let err = compile_check(&src).unwrap_err();
            // explicit find.expect("OBS-701") on real unsupported_*.opt fixture (terse harness style; compile_check error path per self-host prep)
            // compact single-line let-d retained (established short facade precedent matching TYP-010 compile/OBS-702; typeck splits on diag::; post-fmt clean; no bloat)
            let d = err.iter().find(|d| d.code == "OBS-701").expect("OBS-701");
            // explicit get+and_then for evidence (no value asserts added per smallest); mirrors GRA-110/TYP-002 hardened style (host-boundary TYP-010 precedent in typeck/cli)
            // (absent-key / non-str boundary for hardened assert left per smallest; main fixtures always provide key)
            assert_eq!(
                d.evidence.get("method").and_then(|v| v.as_str()),
                Some(method)
            );
            assert!(has_unsupported_observability(&err)); // pre-existing helper (family presence); find already asserts specific code per smallest
        }
    }

    #[test]
    fn facade_rejects_obs702_on_compile_check() {
        for name in ["trailing_tap.opt", "trailing_record.opt"] {
            let src = example_src(name);
            let err = compile_check(&src).unwrap_err();
            // explicit find.expect("OBS-702") on real trailing_*.opt fixture (terse harness style; compile_check error path per self-host prep)
            err.iter().find(|d| d.code == "OBS-702").expect("OBS-702");
            assert!(has_unsupported_observability(&err)); // pre-existing helper (family presence); find already asserts specific code per smallest
        }
    }

    #[test]
    fn facade_rejects_obs701_on_dump_hir_and_ast() {
        for name in ["unsupported_profile.opt", "unsupported_replay.opt"] {
            let src = example_src(name);
            let hir_err = dump_hir_src(&src).unwrap_err();
            // explicit find.expect("OBS-701") on real unsupported_*.opt fixture (terse harness style; dump_hir error path per self-host prep)
            hir_err
                .iter()
                .find(|d| d.code == "OBS-701")
                .expect("OBS-701");
            let ast_err = dump_ast_src(&src).unwrap_err();
            // explicit find.expect("OBS-701") on real unsupported_*.opt fixture (terse harness style; dump_ast error path per self-host prep)
            ast_err
                .iter()
                .find(|d| d.code == "OBS-701")
                .expect("OBS-701");
        }
    }

    #[test]
    fn facade_rejects_obs702_on_dump_hir_and_ast() {
        for name in ["trailing_tap.opt", "trailing_record.opt"] {
            let src = example_src(name);
            let hir_err = dump_hir_src(&src).unwrap_err();
            // explicit find.expect("OBS-702") on real trailing_*.opt fixture (terse harness style; dump_hir error path per self-host prep)
            hir_err
                .iter()
                .find(|d| d.code == "OBS-702")
                .expect("OBS-702");
            let ast_err = dump_ast_src(&src).unwrap_err();
            // explicit find.expect("OBS-702") on real trailing_*.opt fixture (terse harness style; dump_ast error path per self-host prep)
            ast_err
                .iter()
                .find(|d| d.code == "OBS-702")
                .expect("OBS-702");
        }
    }

    #[test]
    fn facade_rejects_typ010_on_compile_check() {
        let src = example_src("host_boundary.opt");
        let err = compile_check(&src).unwrap_err();
        // explicit find.expect("TYP-010") on real host_boundary fixture (terse harness style; direct boundary coverage per self-host prep)
        err.iter().find(|d| d.code == "TYP-010").expect("TYP-010");
    }

    #[test]
    fn facade_rejects_typ010_on_dump_hir_and_ast() {
        let src = example_src("host_boundary.opt");
        let hir_err = dump_hir_src(&src).unwrap_err();
        // explicit find.expect("TYP-010") on real host_boundary.opt fixture (terse harness style; dump_hir error path per self-host prep)
        hir_err
            .iter()
            .find(|d| d.code == "TYP-010")
            .expect("TYP-010");
        let ast_err = dump_ast_src(&src).unwrap_err();
        // explicit find.expect("TYP-010") on real host_boundary.opt fixture (terse harness style; dump_ast error path per self-host prep)
        ast_err
            .iter()
            .find(|d| d.code == "TYP-010")
            .expect("TYP-010");
    }

    #[test]
    fn hir_direct_lower_unsafe_optic_prep_path() {
        // exercises HIR lowering prep for unsafe+extern (full carry); gates still reject in facade/compile paths (self-host bootstrap support, ch22/appI)
        // (documents direct API delta vs prior silent drop; no golden impact)
        let src = example_src("host_boundary.opt");
        let prog = parse(&src, SourceId(1)).expect("parse host_boundary directly");
        let hir = lower(prog).expect("lower unsafe optic for prep path coverage");
        let has_unsafe = hir.items.iter().any(|item| {
            if let optic_hir::HirItem::Optic { decl, .. } = item {
                decl.unsafe_boundary
            } else {
                false
            }
        });
        assert!(
            has_unsafe,
            "unsafe optic must now lower to HirItem::Optic (boundary lowering prep)"
        );
        // explicit extern carry for HIR full (self-host bootstrap S0->S1 prep per ch22/app I/PLAN; externs now visible to tooling sources; no behavior/golden change)
        let has_extern = hir
            .items
            .iter()
            .any(|item| matches!(item, optic_hir::HirItem::Extern(_)));
        assert!(
            has_extern,
            "extern must lower to HirItem::Extern for full boundary HIR (host/foreign support)"
        );
        // explicit dump_hir coverage for Extern arm on real host_boundary (addresses test gap; dump includes "  Extern host_helper abi=C"; no golden impact)
        let dumped = optic_hir::dump_hir(&hir);
        assert!(
            dumped.contains("Extern host_helper"),
            "dump_hir must cover new Extern arm (real fixture prep)"
        );
        // typeck_pass bypass coverage: Extern reaches TypedHir (falls to other=> push); unsafe optic body checked but gate not re-run here
        let (typed, _diags) = typeck_pass(hir);
        let has_extern_typed = typed
            .items
            .iter()
            .any(|item| matches!(item, optic_hir::HirItem::Extern(_)));
        assert!(
            has_extern_typed,
            "extern must reach TypedHir (prep for self-host boundary sources)"
        );
        // gate still works for compile + emit (explicit TYP-010 path test; both use match for error-expecting facade calls per harness/doctor pattern; early surface gate in compile_through_check)
        let err = match compile_check(&src) {
            Err(e) => e,
            Ok(_) => panic!("TYP-010 expected on compile_check for host_boundary"),
        };
        // explicit find.expect("TYP-010") on real host_boundary fixture (terse harness style; direct boundary coverage per self-host prep)
        err.iter().find(|d| d.code == "TYP-010").expect("TYP-010");
        // exercises compile_emit error return (early TYP-010 surface gate before build_cgir/scale guards) + matches harness style
        let emit_err = match compile_emit(&src) {
            Err(e) => e,
            Ok(_) => panic!("TYP-010 expected on compile_emit for host_boundary"),
        };
        emit_err
            .iter()
            .find(|d| d.code == "TYP-010")
            .expect("TYP-010");
    }

    #[test]
    fn facade_compile_check_alive_filter_prism() {
        let src = example_src("alive_filter.opt");
        // explicit match for compile path (prism scaffolding real data; exercises build guards/Ok decision in CGIR-build layer)
        let outcome = match compile_check(&src) {
            Ok(o) => o,
            Err(e) => panic!("compile_check must Ok for alive_filter.opt (real non-exceed): {e:?}"),
        };
        assert!(
            outcome
                .typed_hir
                .items
                .iter()
                .any(|i| matches!(i, optic_hir::HirItem::Optic { decl, .. } if decl.name.node == "AliveFilter")),
            "AliveFilter prism must be in typed HIR"
        );
    }

    #[test]
    fn facade_compile_check_all_healths_traversal() {
        let src = example_src("all_healths.opt");
        let outcome = match compile_check(&src) {
            Ok(o) => o,
            Err(e) => panic!("compile_check must Ok for all_healths.opt (real non-exceed): {e:?}"),
        }; // explicit match (compile path exercising build guards)
        assert!(
            outcome
                .typed_hir
                .items
                .iter()
                .any(|i| matches!(i, optic_hir::HirItem::Optic { decl, .. } if decl.name.node == "AllHealths" && decl.is_traversal())),
            "AllHealths traversal must be in typed HIR"
        );
        let emitted = match compile_emit(&src) {
            Ok(e) => e,
            Err(e) => panic!("compile_emit must Ok for all_healths.opt (real non-exceed): {e:?}"),
        };
        assert!(emitted.contains("// optic(traversal): AllHealths"));
        assert!(emitted.contains("// simd-eligible"));
    }

    #[test]
    fn facade_rejects_oversized_source() {
        let huge = "x".repeat((DEFAULT_MAX_SOURCE_BYTES + 1) as usize);
        let err = compile_check(&huge).unwrap_err();
        // explicit find.expect("PAR-001") on oversized synthetic (terse harness style; compile_check error path per self-host prep)
        err.iter().find(|d| d.code == "PAR-001").expect("PAR-001");
    }

    #[test]
    fn facade_rejects_oversized_file_from_path() {
        let path = std::env::temp_dir().join(format!("optic_huge_{}.opt", std::process::id()));
        let huge = "x".repeat((DEFAULT_MAX_SOURCE_BYTES + 1) as usize);
        std::fs::write(&path, &huge).expect("write huge.opt");
        let err = compile_check_from_path(&path).unwrap_err();
        let _ = std::fs::remove_file(&path);
        // explicit find.expect("PAR-001") on oversized synthetic via from_path (terse harness style; compile_check_from_path error path per self-host prep)
        err.iter().find(|d| d.code == "PAR-001").expect("PAR-001");
    }

    #[test]
    fn agent_repair_smoke_frozen_json_witnesses() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/diagnostics");
        let check_cases = [
            (
                "invalid_grade.json",
                "GRA-110",
                &["annotated", "inferred", "optic"][..],
            ),
            (
                "invalid_alias.json",
                "ALI-201",
                &["conflicting_regions"][..],
            ),
            (
                "typ001_unknown_type.json",
                "TYP-001",
                &["type_name", "role", "optic"][..],
            ),
            (
                "typ001_unknown_focus.json",
                "TYP-001",
                &["type_name", "role", "optic"][..],
            ),
            (
                "typ002_body_mismatch.json",
                "TYP-002",
                &["expected_type", "actual_type", "optic"][..],
            ),
            (
                "typ002_put_mismatch.json",
                "TYP-002",
                &["expected_type", "actual_type", "optic"][..],
            ),
            (
                "typ003_grade_syntax.json",
                "TYP-003",
                &["fragment", "optic"][..],
            ),
            (
                "typ003_unknown_dim.json",
                "TYP-003",
                &["fragment", "optic"][..],
            ),
            (
                "typ004_uninferable_body.json",
                "TYP-004",
                &["clause", "optic"][..],
            ),
            (
                "unsupported_prism.json",
                "GRA-110",
                &["annotated", "inferred", "optic"][..],
            ),
            (
                "unsupported_traversal.json",
                "GRA-110",
                &["annotated", "inferred", "optic"][..],
            ),
            ("host_boundary.json", "TYP-010", &["feature", "detail"][..]),
            (
                "unsupported_profile.json",
                "OBS-701",
                &["method", "milestone"][..],
            ),
            (
                "unsupported_replay.json",
                "OBS-701",
                &["method", "milestone"][..],
            ),
            ("trailing_tap.json", "OBS-702", &["method", "milestone"][..]),
            (
                "trailing_record.json",
                "OBS-702",
                &["method", "milestone"][..],
            ),
            (
                "cgi006_tap_stub.json",
                "CGI-006",
                &["kind", "node_id", "reason", "milestone"][..],
            ),
            (
                "cgi006_record_stub.json",
                "CGI-006",
                &["kind", "node_id", "reason", "milestone"][..],
            ),
            (
                "cgi006_prism_leaf.json",
                "CGI-006",
                &["kind", "node_id", "reason", "milestone"][..],
            ),
            (
                "cgi006_traversal_leaf.json",
                "CGI-006",
                &["kind", "node_id", "reason", "milestone"][..],
            ),
            (
                "cgi003_prism_compose.json",
                "CGI-003",
                &["compose_id", "leaf_id", "reason"][..],
            ),
            (
                "cgi003_traversal_compose.json",
                "CGI-003",
                &["compose_id", "leaf_id", "reason"][..],
            ),
        ];
        for (file, code, evidence_keys) in check_cases {
            let path = dir.join(file);
            let raw = std::fs::read_to_string(&path).expect("read witness");
            let v: serde_json::Value = serde_json::from_str(&raw).expect("parse json");
            let diags = v["diagnostics"].as_array().expect("diagnostics array");
            let d = diags
                .iter()
                .find(|d| d["code"].as_str() == Some(code))
                .unwrap_or_else(|| panic!("missing {code} in {file}"));
            let fixes = d["ranked_fixes"].as_array().expect("ranked_fixes");
            assert!(!fixes.is_empty(), "{code} must have ranked_fixes");
            for key in evidence_keys {
                assert!(
                    d["evidence"].get(key).is_some(),
                    "{code} evidence must include {key}"
                );
            }
        }
        let explain_grade_cases = [
            "explain_grade_healthview.json",
            "explain_grade_badcache.json",
            "explain_grade_nested.json",
            "explain_grade_unknown_node.json",
            "explain_grade_typ002_fail.json",
            "explain_grade_typ003_fail.json",
            "explain_grade_typ004_fail.json",
        ];
        for file in explain_grade_cases {
            let path = dir.join(file);
            let raw = std::fs::read_to_string(&path).expect("read explain-grade witness");
            let v: serde_json::Value = serde_json::from_str(&raw).expect("parse json");
            if file.contains("unknown_node") || file.contains("_fail") {
                let diags = v["diagnostics"].as_array().expect("diagnostics array");
                assert!(!diags.is_empty(), "{file} must have diagnostics");
            } else {
                assert_eq!(v["ok"], true, "{file} must be success envelope");
                assert!(v.get("grade").is_some(), "{file} must include grade");
            }
        }
        let explain_focus_cases = [
            ("explain_focus_healthview.json", true),
            ("explain_focus_nested.json", true),
            ("explain_focus_alive_filter.json", true),
            ("explain_focus_all_healths.json", true),
            ("explain_focus_unknown_node.json", false),
            ("explain_focus_typ002_fail.json", false),
            ("explain_focus_typ010_fail.json", false),
        ];
        for (file, success) in explain_focus_cases {
            let path = dir.join(file);
            let raw = std::fs::read_to_string(&path).expect("read explain-focus witness");
            let v: serde_json::Value = serde_json::from_str(&raw).expect("parse json");
            if success {
                assert_eq!(v["ok"], true, "{file} must be success envelope");
                assert!(v.get("focus").is_some(), "{file} must include focus");
            } else {
                let diags = v["diagnostics"].as_array().expect("diagnostics array");
                assert!(!diags.is_empty(), "{file} must have diagnostics");
            }
        }
    }

    #[test]
    fn optimize_prism_leaf_graph_maps_cgi006() {
        use optic_cgir::{CgirGraph, CgirNode};
        use optic_hir::{Determinism, HirExpr, OpticSummary, OwnershipDim, PathLift, Rational};
        use optic_syntax::Span;
        use std::sync::Arc;

        let summary = Arc::new(OpticSummary {
            name: Some("AliveFilter".into()),
            costate: "Entities".into(),
            focus: "f32".into(),
            lift: PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec![],
            get_grade: optic_hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: optic_hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: Determinism::Pure,
            put_determinism: Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let g = CgirGraph {
            nodes: vec![CgirNode::PrismLeaf {
                id: 0,
                name: "AliveFilter".into(),
                costate: "Entities".into(),
                focus: "f32".into(),
                grade: summary.get_grade.clone(),
                preview_fn: String::new(),
                review_fn: String::new(),
                preview_param: "s".into(),
                preview_body: Arc::new(HirExpr::LitInt(1, Span::dummy())),
                preview_returns_option: false,
                preview_wrap_some: false,
                review_state_param: None,
                review_value_param: None,
                review_value_body: None,
                summary,
                provenance: Span::dummy(),
                m7_reserved: true,
            }],
            roots: vec![0],
            provenance_index: Default::default(),
            resolved_optics: Default::default(),
            region_map: Default::default(),
        };
        let diag = optimize(g).expect_err("optimize must fail verify on PrismLeaf");
        assert_eq!(diag.code, optic_diagnostics::CGIR_M7_RESERVED);
        assert_eq!(diag.evidence["kind"], "PrismLeaf");
    }

    #[test]
    fn facade_compile_emit_alive_filter() {
        let src = example_src("alive_filter.opt");
        let rust = match compile_emit(&src) {
            Ok(r) => r,
            Err(e) => panic!("compile_emit must Ok for alive_filter.opt (real non-exceed): {e:?}"),
        }; // explicit match for compile_emit decision (real prism path)
        assert!(rust.contains("run_example"));
        assert!(!rust.contains("if let Some"));
    }

    #[test]
    fn facade_explain_focus_alive_filter() {
        let src = example_src("alive_filter.opt");
        let report = explain_focus_from_src(&src, "AliveFilter").expect("prism focus report");
        assert_eq!(report.node, "AliveFilter");
        assert_eq!(report.root_path, "entities.healths[id]");
    }

    #[test]
    fn lower_let_unknown_named_res001() {
        // smallest inline coverage for new explicit Err arm in compute_summary_for_optic (Named unknown during let optic-expr lower); exercises lower→lower_to_diags→RES-001. Uses unified message.
        let src = r#"
data E { h: SoA<f32> }
optic A: GradedOptic<E, f32, _> { get s => s.h[s.id] put (s, v) => { s.h[s.id] = v } }
let bad = A >>> Missing;
fn main() { entities.query(bad).get(); }
"#;
        let errs = lower_src(src).expect_err("lower must fail for unknown Named in let optic-expr");
        assert!(
            errs.iter()
                .any(|d| d.code == "RES-001" && d.rule.contains("unknown optic `Missing`")),
            "must be RES-001 with unified message from hir compute path"
        );
        // 1-line extension for annotated-let parity (make_summary_from_ann path to compute Err)
        let src_ann = r#"
data E { h: SoA<f32> }
optic A: GradedOptic<E, f32, _> { get s => s.h[s.id] put (s, v) => { s.h[s.id] = v } }
let bad: GradedOptic<E, f32, _> = A >>> MissingAnn;
fn main() { entities.query(bad).get(); }
"#;
        let e2 = lower_src(src_ann).expect_err("ann let");
        assert!(e2
            .iter()
            .any(|d| d.code == "RES-001" && d.rule.contains("unknown optic `MissingAnn`")));
    }
}
