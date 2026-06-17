//! optic-typeck — types, grade inference, alias safety (ch. 9, M2).

use optic_diagnostics::{self as diag, Diagnostic};
use optic_hir::{self as hir, ConcreteGrade, OpticSummary, OwnershipDim, Rational, Region};
use optic_syntax::Span;
use std::collections::HashMap;
use std::sync::Arc;

pub type TypeckResult<T> = Result<T, Vec<Diagnostic>>;

#[derive(Clone, Debug)]
pub struct TypedHir {
    pub items: Vec<hir::HirItem>,
    pub summaries: HashMap<String, Arc<OpticSummary>>,
}

pub fn check(hir: hir::HirProgram) -> TypeckResult<TypedHir> {
    let mut summaries = HashMap::new();
    let mut diags = vec![];
    let mut typed_items = vec![];

    for item in hir.items {
        match item {
            hir::HirItem::Optic { decl, summary } => {
                let inferred = infer_grade_from_summary(&summary);
                if let Some(ann_cache) = hir::annotated_cache_bound(&decl.grade) {
                    if inferred.cache > ann_cache {
                        diags.push(diag::grade_decl_diag(
                            decl.span,
                            vec![decl.grade.span],
                            "declared CacheGrade is tighter than inferred from optic body",
                            serde_json::json!({
                                "annotated": ann_cache,
                                "inferred": inferred.cache,
                                "optic": decl.name.node,
                            }),
                        ));
                    }
                }
                let mut inner = match Arc::try_unwrap(summary) {
                    Ok(s) => s,
                    Err(a) => (*a).clone(),
                };
                inner.get_grade = inferred.clone();
                inner.put_grade = inferred;
                let arc = Arc::new(inner);
                summaries.insert(decl.name.node.clone(), Arc::clone(&arc));
                typed_items.push(hir::HirItem::Optic { decl, summary: arc });
            }
            hir::HirItem::Let {
                name,
                optic,
                summary,
            } => {
                if let hir::HirOptic::Par { lhs, rhs, span } = &optic {
                    match (
                        resolve_summary_for_optic(lhs, &summaries, *span),
                        resolve_summary_for_optic(rhs, &summaries, *span),
                    ) {
                        (Ok(lsum), Ok(rsum)) => {
                            if let Err(d) = alias_safe(&lsum, &rsum) {
                                diags.push(d);
                            }
                        }
                        (Err(d), _) | (_, Err(d)) => diags.push(d),
                    }
                }
                if let hir::HirOptic::Seq { lhs, rhs, span } = &optic {
                    match (
                        resolve_summary_for_optic(lhs, &summaries, *span),
                        resolve_summary_for_optic(rhs, &summaries, *span),
                    ) {
                        (Ok(lsum), Ok(rsum)) => {
                            let combined = sat_add(lsum.get_grade.cache, rsum.get_grade.cache);
                            if let Some(bound) = bound_from_summary(&summary) {
                                if combined > bound {
                                    diags.push(diag::grade_compose_diag(
                                        *span,
                                        compose_related_spans(&optic),
                                        "sequential composition cache exceeds declared bound",
                                        serde_json::json!({
                                            "annotated": bound,
                                            "inferred": combined,
                                            "lhs_cache": lsum.get_grade.cache,
                                            "rhs_cache": rsum.get_grade.cache,
                                            "binding": name,
                                        }),
                                    ));
                                }
                            }
                        }
                        (Err(d), _) | (_, Err(d)) => diags.push(d),
                    }
                }
                let s = summary;
                summaries.insert(name.clone(), Arc::clone(&s));
                typed_items.push(hir::HirItem::Let {
                    name,
                    optic,
                    summary: s,
                });
            }
            hir::HirItem::Query(q) => {
                match resolve_summary_for_optic(&q.optic, &summaries, q.span) {
                    Ok(_) => {}
                    Err(d) => diags.push(d),
                }
                if let hir::HirOptic::Par { lhs, rhs, span } = &q.optic {
                    match (
                        resolve_summary_for_optic(lhs, &summaries, *span),
                        resolve_summary_for_optic(rhs, &summaries, *span),
                    ) {
                        (Ok(lsum), Ok(rsum)) => {
                            if let Err(d) = alias_safe(&lsum, &rsum) {
                                diags.push(d);
                            }
                        }
                        (Err(d), _) | (_, Err(d)) => diags.push(d),
                    }
                }
                match &q.kind {
                    hir::QueryKind::Map { body, .. } => {
                        collect_unsupported_expr(body.as_ref(), &mut diags);
                    }
                    hir::QueryKind::Set { value } => {
                        collect_unsupported_expr(value, &mut diags);
                    }
                    _ => {}
                }
                typed_items.push(hir::HirItem::Query(q));
            }
            other => typed_items.push(other),
        }
    }

    if !diags.is_empty() {
        return Err(diags);
    }
    Ok(TypedHir {
        items: typed_items,
        summaries,
    })
}

fn compose_related_spans(optic: &hir::HirOptic) -> Vec<Span> {
    match optic {
        hir::HirOptic::Seq { lhs, rhs, span } => {
            let mut out = vec![*span];
            if let hir::HirOptic::Named { span, .. } = &**lhs {
                out.push(*span);
            }
            if let hir::HirOptic::Named { span, .. } = &**rhs {
                out.push(*span);
            }
            out
        }
        other => vec![optic_span(other)],
    }
}

fn optic_span(o: &hir::HirOptic) -> Span {
    match o {
        hir::HirOptic::Named { span, .. }
        | hir::HirOptic::Seq { span, .. }
        | hir::HirOptic::Par { span, .. } => *span,
    }
}

fn collect_unsupported_expr(e: &hir::HirExpr, diags: &mut Vec<Diagnostic>) {
    match e {
        hir::HirExpr::Unsupported { reason, span } => {
            diags.push(diag::unsupported_expr_diag(*span, reason));
        }
        hir::HirExpr::Bin { left, right, .. } => {
            collect_unsupported_expr(left, diags);
            collect_unsupported_expr(right, diags);
        }
        hir::HirExpr::Paren(inner, _) => collect_unsupported_expr(inner, diags),
        hir::HirExpr::Tuple(elems, _) => {
            for el in elems {
                collect_unsupported_expr(el, diags);
            }
        }
        hir::HirExpr::TupleProj { base, .. } => collect_unsupported_expr(base, diags),
        _ => {}
    }
}

/// ch9.9.3: cache = sat_add(reads, writes) with separate body counts.
pub fn infer_grade_from_summary(s: &OpticSummary) -> ConcreteGrade {
    let reads = distinct_count(&s.get_reads) as u8;
    let writes = distinct_count(&s.put_writes) as u8;
    let cache = sat_add(reads, writes);
    let has_put = !s.put_writes.is_empty();
    let put_reads_set: std::collections::HashSet<_> = s.put_reads.iter().collect();
    let put_writes_set: std::collections::HashSet<_> = s.put_writes.iter().collect();
    let overlap_put = put_reads_set.intersection(&put_writes_set).next().is_some();
    let own = if !has_put {
        OwnershipDim {
            share: Rational::one(),
            read_only: true,
            must_use: false,
        }
    } else if !overlap_put {
        OwnershipDim {
            share: Rational::one(),
            read_only: false,
            must_use: false,
        }
    } else {
        OwnershipDim {
            share: Rational::one(),
            read_only: false,
            must_use: false,
        }
    };
    ConcreteGrade {
        cache,
        ownership: own,
    }
}

fn distinct_count(regs: &[Region]) -> usize {
    let mut seen = std::collections::HashSet::new();
    for r in regs {
        seen.insert(r.as_str());
    }
    seen.len()
}

fn bound_from_summary(s: &OpticSummary) -> Option<u8> {
    Some(s.get_grade.cache).filter(|&c| c != 255)
}

pub fn resolve_summary_for_optic(
    o: &hir::HirOptic,
    env: &HashMap<String, Arc<OpticSummary>>,
    span: Span,
) -> Result<Arc<OpticSummary>, Diagnostic> {
    match o {
        hir::HirOptic::Named { name, .. } => env
            .get(name)
            .cloned()
            .ok_or_else(|| diag::resolve_diag(span, format!("unknown optic `{name}`"))),
        hir::HirOptic::Seq { lhs, rhs, .. } | hir::HirOptic::Par { lhs, rhs, .. } => {
            let l = resolve_summary_for_optic(lhs, env, span)?;
            let r = resolve_summary_for_optic(rhs, env, span)?;
            Ok(Arc::new(compose_summary_arc(&l, &r, o)))
        }
    }
}

fn compose_summary_arc(l: &OpticSummary, r: &OpticSummary, o: &hir::HirOptic) -> OpticSummary {
    match o {
        hir::HirOptic::Seq { span, .. } => {
            let mut s = l.clone();
            s.get_reads = union(&s.get_reads, &r.get_reads);
            s.put_reads = union(&s.put_reads, &union(&s.get_reads, &r.put_reads));
            s.put_writes = union(&s.put_writes, &r.put_writes);
            s.get_grade.cache = sat_add(l.get_grade.cache, r.get_grade.cache);
            s.put_grade.cache = sat_add(l.put_grade.cache, r.put_grade.cache);
            s.provenance = *span;
            s
        }
        hir::HirOptic::Par { span, .. } => OpticSummary {
            name: None,
            costate: l.costate.clone(),
            focus: "tuple".into(),
            lift: hir::PathLift::default(),
            get_reads: union(&l.get_reads, &r.get_reads),
            put_reads: union(&l.put_reads, &r.put_reads),
            put_writes: union(&l.put_writes, &r.put_writes),
            get_grade: l.get_grade.clone(),
            put_grade: l.put_grade.clone(),
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: *span,
        },
        _ => l.clone(),
    }
}

fn union(a: &[Region], b: &[Region]) -> Vec<Region> {
    hir::dedup_regions(a.iter().chain(b.iter()).cloned().collect())
}

/// Exact alias checker from ch9 (M2 gate).
pub fn alias_safe(left: &OpticSummary, right: &OpticSummary) -> Result<(), Diagnostic> {
    let left_eff: Vec<_> = left
        .get_reads
        .iter()
        .chain(&left.put_reads)
        .chain(&left.put_writes)
        .cloned()
        .collect();
    let right_eff: Vec<_> = right
        .get_reads
        .iter()
        .chain(&right.put_reads)
        .chain(&right.put_writes)
        .cloned()
        .collect();

    for (l, r) in overlapping_pairs(&left.put_writes, &right_eff) {
        if left.get_grade.ownership.read_only && right.get_grade.ownership.read_only {
            if le_share(
                &left.get_grade.ownership.share,
                &right.get_grade.ownership.share,
            ) {
                continue;
            }
        }
        if same_partition(&l, &r) {
            if le_share(
                &left.get_grade.ownership.share,
                &right.get_grade.ownership.share,
            ) {
                continue;
            }
        }
        return Err(diag::alias_diag(
            left.provenance,
            vec![left.provenance, right.provenance],
            &vec![l.clone(), r.clone()],
            "put_writes overlaps effective region (including put_reads hazard)",
        ));
    }

    for (r, l) in overlapping_pairs(&right.put_writes, &left_eff) {
        if right.get_grade.ownership.read_only && left.get_grade.ownership.read_only {
            if le_share(
                &right.get_grade.ownership.share,
                &left.get_grade.ownership.share,
            ) {
                continue;
            }
        }
        if same_partition(&r, &l) {
            if le_share(
                &right.get_grade.ownership.share,
                &left.get_grade.ownership.share,
            ) {
                continue;
            }
        }
        return Err(diag::alias_diag(
            right.provenance,
            vec![left.provenance, right.provenance],
            &vec![r.clone(), l.clone()],
            "put_writes overlaps effective region (including put_reads hazard)",
        ));
    }
    Ok(())
}

fn overlapping_pairs(writes: &[Region], eff: &[Region]) -> Vec<(Region, Region)> {
    let mut out = vec![];
    for w in writes {
        for e in eff {
            if w == e || is_subregion(w, e) || is_subregion(e, w) {
                out.push((w.clone(), e.clone()));
            }
        }
    }
    out
}

fn is_subregion(a: &str, b: &str) -> bool {
    a == b
}
fn same_partition(a: &str, b: &str) -> bool {
    a == b
}
fn le_share(a: &Rational, b: &Rational) -> bool {
    a.num as f64 / a.den as f64 + b.num as f64 / b.den as f64 <= 1.0 + 1e-9
}

/// sat_add per ch9.9.2: u8 sat at 255
pub fn sat_add(x: u8, y: u8) -> u8 {
    if x == 255 || y == 255 || x as u16 + y as u16 > 254 {
        255
    } else {
        x + y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use optic_hir::{Determinism, OpticSummary, PathLift};

    fn mk_sum(name: &str, reads: Vec<&str>, writes: Vec<&str>) -> OpticSummary {
        OpticSummary {
            name: Some(name.into()),
            costate: "Entities".into(),
            focus: "f32".into(),
            lift: PathLift::default(),
            get_reads: reads.into_iter().map(|s| s.to_string()).collect(),
            put_reads: vec![],
            put_writes: writes.into_iter().map(|s| s.to_string()).collect(),
            get_grade: ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: ConcreteGrade {
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
            provenance: optic_syntax::Span::dummy(),
        }
    }

    #[test]
    fn test_sat_add_and_infer_ch939() {
        assert_eq!(sat_add(254, 2), 255);
        assert_eq!(sat_add(1, 1), 2);
        let s = mk_sum("H", vec!["healths"], vec!["healths"]);
        let g = infer_grade_from_summary(&s);
        assert_eq!(
            g.cache, 2,
            "ch9.9.3: sat_add(reads, writes) for same-field get+put"
        );
    }

    #[test]
    fn test_gra110_healthview_cachegrade1_rejects() {
        let src = r#"
data Entities { healths: SoA<f32> }
optic HealthView: GradedOptic<Entities,f32,CacheGrade<1>> {
  get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v}
}
fn main() { entities.query(HealthView).get(); }
"#;
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let err = check(hirp).unwrap_err();
        assert!(err.iter().any(|d| d.code == diag::GRADE_DECL_TIGHT));
        let d = err
            .iter()
            .find(|d| d.code == diag::GRADE_DECL_TIGHT)
            .unwrap();
        assert_eq!(d.evidence["inferred"], 2);
        assert_eq!(d.evidence["annotated"], 1);
    }

    #[test]
    fn test_gra110_invalid_grade_multi_region() {
        let src = include_str!("../../../examples/invalid_grade.opt");
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let err = check(hirp).unwrap_err();
        assert!(err.iter().any(|d| d.code == diag::GRADE_DECL_TIGHT));
    }

    #[test]
    fn test_cgi003_rejects_incompatible_map_chain() {
        let src = r#"
data E { healths: SoA<f32>, positions: SoA<f32> }
optic H: GradedOptic<E,f32,_> { get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v} }
fn main() { entities.query(H).map(|h| h).map(|(x,y)| (x,y)); }
"#;
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let err = check(hirp).unwrap_err();
        assert!(err.iter().any(|d| d.code == diag::CGIR_UNSUPPORTED_EXPR));
    }

    #[test]
    fn test_cgi003_rejects_unsupported_map_body() {
        let src = r#"
optic H: GradedOptic<Entities,f32,_> { get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v} }
fn main() { entities.query(H).get().map(|h| h); }
"#;
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let err = check(hirp).unwrap_err();
        assert!(err.iter().any(|d| d.code == diag::CGIR_UNSUPPORTED_EXPR));
    }

    #[test]
    fn test_gra104_seq_compose_bound() {
        let src = r#"
data E { h: SoA<f32> }
optic A: GradedOptic<E,f32,_> { get s=>s.h[s.id] put(s,v)=>{s.h[s.id]=v} }
optic B: GradedOptic<E,f32,_> { get s=>s.h[s.id] put(s,v)=>{s.h[s.id]=v} }
let bad: GradedOptic<E,f32,CacheGrade<2>> = A >>> B;
fn main() { entities.query(bad).map(|x| x); }
"#;
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let err = check(hirp).unwrap_err();
        assert!(err.iter().any(|d| d.code == diag::GRADE_COMPOSE_OVER));
    }

    #[test]
    fn test_alias_safe_put_reads_hazard() {
        let l = mk_sum("W", vec!["healths"], vec!["healths"]);
        let r = mk_sum("A", vec!["healths"], vec!["healths"]);
        assert!(alias_safe(&l, &r).is_err());
        let d = alias_safe(&l, &r).unwrap_err();
        assert_eq!(d.code, diag::ALIAS_CONFLICT);
        assert!(d.evidence.get("conflicting_regions").is_some());
    }

    #[test]
    fn test_resolve_unknown_optic_errors() {
        let o = hir::HirOptic::Named {
            name: "Missing".into(),
            span: Span::dummy(),
        };
        let env = HashMap::new();
        let err = resolve_summary_for_optic(&o, &env, Span::dummy()).unwrap_err();
        assert_eq!(err.code, diag::RESOLVE_UNKNOWN);
    }

    #[test]
    fn test_arc_in_check_and_resolve() {
        let src = r#"
data Entities { healths: SoA<f32> }
optic H: GradedOptic<Entities,f32,_> { get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v} }
let c = H;
fn main() { entities.query(c).map(|h| h - 1); }
"#;
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("p");
        let hirp = optic_hir::lower(prog).expect("l");
        let typed = check(hirp).expect("check");
        assert!(!typed.summaries.is_empty());
    }
}
