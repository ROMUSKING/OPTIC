//! optic-typeck — types, grade inference, alias safety (ch. 9, M2).

use optic_diagnostics::{self as diag, Diagnostic};
use optic_hir::{self as hir, ConcreteGrade, OpticSummary, OwnershipDim, Rational, Region};
use optic_syntax::{self as syn, Span, TypeExpr};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub type TypeckResult<T> = Result<T, Vec<Diagnostic>>;

const PRIMITIVE_TYPES: &[&str] = &["f32", "i32", "u32", "Vec2"];

#[derive(Clone, Debug)]
pub struct TypedHir {
    pub items: Vec<hir::HirItem>,
    pub summaries: HashMap<String, Arc<OpticSummary>>,
}

/// Normalized grade report for `explain-grade` (appendix B).
#[derive(Clone, Debug, Serialize)]
pub struct GradeReport {
    pub optic: String,
    pub declared: GradeSnapshot,
    pub inferred: GradeSnapshot,
    pub regions: RegionsSnapshot,
}

#[derive(Clone, Debug, Serialize)]
pub struct GradeSnapshot {
    pub cache: u8,
    pub cache_source: String,
    pub ownership_share: String,
    pub ownership_alias: Option<String>,
    pub read_only: bool,
    pub must_use: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RegionsSnapshot {
    pub get_reads: Vec<String>,
    pub put_reads: Vec<String>,
    pub put_writes: Vec<String>,
}

/// PathLift / root-path report for `explain-focus` (appendix B).
#[derive(Clone, Debug, Serialize)]
pub struct FocusReport {
    pub node: String,
    pub costate: String,
    pub focus: String,
    pub path_lift_prefix: Vec<String>,
    pub root_path: String,
    pub focus_fields: Vec<String>,
}

/// Reject deferred surface features (prisms, host/foreign boundaries) before M7.
pub fn collect_unsupported_surface(prog: &syn::Program) -> Vec<Diagnostic> {
    let mut diags = vec![];
    for item in &prog.items {
        match item {
            syn::Item::Extern(e) => {
                diags.push(diag::type_unsupported_v0_diag(
                    e.span,
                    "foreign_decl",
                    "foreign `extern` declarations are not supported in narrow v0 (M7+ host boundaries)",
                    Some(&e.name.node),
                ));
            }
            syn::Item::Optic(decl) => {
                if decl.unsafe_boundary {
                    diags.push(diag::type_unsupported_v0_diag(
                        decl.span,
                        "unsafe_optic",
                        "`unsafe optic` host/foreign boundary wrappers are not supported in narrow v0",
                        Some(&decl.name.node),
                    ));
                } else if decl.type_ctor == syn::OpticTypeCtor::GradedPrism
                    || decl.preview.is_some()
                    || decl.review.is_some()
                {
                    diags.push(diag::type_unsupported_v0_diag(
                        decl.span,
                        "prism",
                        "prism syntax (`GradedPrism`, `preview`/`review`) is not supported in narrow v0 (M7+)",
                        Some(&decl.name.node),
                    ));
                } else if decl.type_ctor == syn::OpticTypeCtor::GradedTraversal {
                    diags.push(diag::type_unsupported_v0_diag(
                        decl.span,
                        "traversal",
                        "traversal syntax (`GradedTraversal`) is not supported in narrow v0 (M7+)",
                        Some(&decl.name.node),
                    ));
                }
            }
            _ => {}
        }
    }
    diags
}

/// Run type/grade/alias checking; always returns typed HIR plus collected diagnostics.
pub fn typeck_pass(hir: hir::HirProgram) -> (TypedHir, Vec<Diagnostic>) {
    let mut summaries = HashMap::new();
    let mut diags = vec![];
    let mut typed_items = vec![];
    let known_types = collect_known_types(&hir);
    let region_map = hir::build_region_map(&hir).unwrap_or_default();

    for item in &hir.items {
        match item {
            hir::HirItem::Optic { decl, .. } => {
                diags.extend(validate_optic_types(decl, &known_types));
                diags.extend(validate_grade_syntax(&decl.grade, &decl.name.node));
                diags.extend(check_optic_body_types(decl, &region_map));
            }
            hir::HirItem::Let {
                name,
                ty: Some(ann),
                ..
            } => {
                diags.extend(validate_graded_optic_types(ann, &known_types, name));
                diags.extend(validate_grade_syntax(&ann.grade, name));
            }
            _ => {}
        }
    }

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
                ty,
                span,
                optic,
                summary,
            } => {
                if let hir::HirOptic::Par { lhs, rhs, span } = &optic {
                    match (
                        resolve_summary_for_optic(lhs, &summaries, *span),
                        resolve_summary_for_optic(rhs, &summaries, *span),
                    ) {
                        (Ok(lsum), Ok(rsum)) => {
                            if let Err(d) = par_lift_allowed(&lsum, &rsum, *span) {
                                diags.push(d);
                            }
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
                    ty,
                    span,
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
                            if let Err(d) = par_lift_allowed(&lsum, &rsum, *span) {
                                diags.push(d);
                            }
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

    (
        TypedHir {
            items: typed_items,
            summaries,
        },
        diags,
    )
}

pub fn check(hir: hir::HirProgram) -> TypeckResult<TypedHir> {
    let (typed, diags) = typeck_pass(hir);
    if diags.is_empty() {
        Ok(typed)
    } else {
        Err(diags)
    }
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
    hir::is_subregion(a, b)
}

fn par_lift_allowed(
    lsum: &hir::OpticSummary,
    rsum: &hir::OpticSummary,
    span: Span,
) -> Result<(), Diagnostic> {
    hir::PathLift::pair(&lsum.lift, &rsum.lift)
        .map_err(|rule| diag::unsupported_expr_diag(span, &rule))?;
    Ok(())
}
fn same_partition(a: &str, b: &str) -> bool {
    let a_root = a.split('.').next().unwrap_or(a);
    let b_root = b.split('.').next().unwrap_or(b);
    a_root == b_root
}
fn le_share(a: &Rational, b: &Rational) -> bool {
    a.num as f64 / a.den as f64 + b.num as f64 / b.den as f64 <= 1.0 + 1e-9
}

/// True when any TYP-010 diagnostic is present (strict gate for check/dump paths).
pub fn has_unsupported_surface(diags: &[Diagnostic]) -> bool {
    diags.iter().any(|d| d.code == diag::TYPE_UNSUPPORTED_V0)
}

/// TYP-010 diagnostics that apply to a specific optic/extern name.
pub fn unsupported_for_node(diags: &[Diagnostic], node: &str) -> Vec<Diagnostic> {
    diags
        .iter()
        .filter(|d| unsupported_targets_node(d, node))
        .cloned()
        .collect()
}

pub fn unsupported_targets_node(d: &Diagnostic, node: &str) -> bool {
    d.code == diag::TYPE_UNSUPPORTED_V0
        && d.evidence.get("name").and_then(|v| v.as_str()) == Some(node)
}

fn is_explain_blocking(d: &Diagnostic) -> bool {
    matches!(
        d.code.as_str(),
        diag::TYPE_UNKNOWN
            | diag::TYPE_BODY_MISMATCH
            | diag::TYPE_GRADE_SYNTAX
            | diag::TYPE_BODY_UNINFERABLE
    )
}

fn is_explain_blocking_for_node(d: &Diagnostic, node: &str) -> bool {
    if unsupported_targets_node(d, node) {
        return true;
    }
    is_explain_blocking(d) && diag_targets_node(d, node)
}

fn diag_targets_node(d: &Diagnostic, node: &str) -> bool {
    d.evidence.get("optic").and_then(|v| v.as_str()) == Some(node)
        || d.evidence.get("binding").and_then(|v| v.as_str()) == Some(node)
        || d.evidence.get("name").and_then(|v| v.as_str()) == Some(node)
}

/// Materialize normalized grade; fails if `node` has blocking TYP-* diagnostics.
pub fn explain_grade_with_diags(
    typed: &TypedHir,
    node: &str,
    diags: &[Diagnostic],
) -> TypeckResult<GradeReport> {
    let blocking: Vec<_> = diags
        .iter()
        .filter(|d| is_explain_blocking_for_node(d, node))
        .cloned()
        .collect();
    if !blocking.is_empty() {
        return Err(blocking);
    }
    explain_grade(typed, node)
}

/// Materialize normalized grade for a named optic or let binding.
pub fn explain_grade(typed: &TypedHir, node: &str) -> TypeckResult<GradeReport> {
    for item in &typed.items {
        match item {
            hir::HirItem::Optic { decl, summary } if decl.name.node == node => {
                return Ok(build_grade_report(node, Some(&decl.grade), summary));
            }
            hir::HirItem::Let {
                name,
                ty,
                summary,
                ..
            } if name == node => {
                return Ok(build_grade_report(node, ty.as_ref().map(|t| &t.grade), summary));
            }
            _ => {}
        }
    }
    let mut candidates: Vec<_> = typed.summaries.keys().cloned().collect();
    candidates.sort();
    let span = file_level_span(typed);
    Err(vec![diag::explain_unknown_node_diag(
        span,
        node,
        &candidates,
    )])
}

fn file_level_span(typed: &TypedHir) -> Span {
    for item in &typed.items {
        match item {
            hir::HirItem::Optic { decl, .. } => return decl.span,
            hir::HirItem::Let { span, .. } => return *span,
            _ => {}
        }
    }
    Span::dummy()
}

fn format_root_path(summary: &OpticSummary) -> String {
    let root_var = summary.costate.to_lowercase();
    let region = summary
        .put_writes
        .first()
        .or_else(|| summary.get_reads.first())
        .map(|s| s.as_str())
        .unwrap_or("");
    if region.contains('.') {
        let col = region.split('.').next().unwrap_or(region);
        let nested: Vec<_> = region.split('.').skip(1).collect();
        let mut path = format!("{root_var}.{col}[id]");
        for seg in nested {
            path.push('.');
            path.push_str(seg);
        }
        return path;
    }
    if !summary.lift.prefix.is_empty() {
        let col = if region.is_empty() {
            summary.lift.prefix[0].clone()
        } else {
            region.to_string()
        };
        let tail = if summary.lift.prefix.len() > 1 {
            summary.lift.prefix[1..].join(".")
        } else if region.is_empty() {
            String::new()
        } else {
            summary.lift.prefix.join(".")
        };
        if tail.is_empty() && region == col {
            return format!("{root_var}.{col}");
        }
        if tail.is_empty() {
            return format!("{root_var}.{col}[id]");
        }
        return format!("{root_var}.{col}[id].{tail}");
    }
    if region.is_empty() {
        return format!("{root_var}[id]");
    }
    format!("{root_var}.{region}[id]")
}

fn focus_fields_from_decl(decl: &syn::OpticDecl) -> Vec<String> {
    let mut out = vec![];
    if let Some(get) = decl.get.as_ref() {
        if let Some(path) = focus_field_path_from_surface(&get.param.node, &get.body) {
            out.push(format!("{}.{}", get.param.node, path.join(".")));
        }
    }
    if let Some(put) = &decl.put {
        if let Some(path) = focus_assign_path_surface(&put.state_param.node, &put.body) {
            out.push(format!(
                "{}.{}",
                put.state_param.node,
                path.join(".")
            ));
        }
    }
    out.sort();
    out.dedup();
    out
}

fn focus_field_path_from_surface(param: &str, e: &syn::Expr) -> Option<Vec<String>> {
    match e {
        syn::Expr::Field(fe) => focus_field_path_from_field_surface(param, fe),
        syn::Expr::Block { result, .. } => result
            .as_ref()
            .and_then(|r| focus_field_path_from_surface(param, r)),
        _ => None,
    }
}

fn focus_field_path_from_field_surface(param: &str, fe: &syn::FieldExpr) -> Option<Vec<String>> {
    match fe {
        syn::FieldExpr::FieldAccess { base, field, .. } => {
            let mut path = focus_field_path_from_field_surface(param, base)?;
            path.push(field.node.clone());
            Some(path)
        }
        syn::FieldExpr::Index { .. } => None,
        syn::FieldExpr::Base(syn::AtomExpr::Ident(id), _) if id.node == param => Some(vec![]),
        _ => None,
    }
}

fn focus_assign_path_surface(state_param: &str, e: &syn::Expr) -> Option<Vec<String>> {
    match e {
        syn::Expr::Assign { target, .. } => {
            if let syn::Expr::Field(fe) = target.as_ref() {
                focus_field_path_from_field_surface(state_param, fe)
            } else {
                None
            }
        }
        syn::Expr::Block { stmts, result, .. } => {
            for stmt in stmts {
                if let Some(p) = focus_assign_path_surface(state_param, &stmt.expr) {
                    return Some(p);
                }
            }
            result
                .as_ref()
                .and_then(|r| focus_assign_path_surface(state_param, r))
        }
        _ => None,
    }
}

/// Materialize PathLift / root-path for a named optic or let binding.
pub fn explain_focus(typed: &TypedHir, node: &str) -> TypeckResult<FocusReport> {
    for item in &typed.items {
        match item {
            hir::HirItem::Optic { decl, summary } if decl.name.node == node => {
                return Ok(FocusReport {
                    node: node.into(),
                    costate: summary.costate.clone(),
                    focus: summary.focus.clone(),
                    path_lift_prefix: summary.lift.prefix.clone(),
                    root_path: format_root_path(summary),
                    focus_fields: focus_fields_from_decl(decl),
                });
            }
            hir::HirItem::Let {
                name,
                summary,
                ..
            } if name == node => {
                return Ok(FocusReport {
                    node: node.into(),
                    costate: summary.costate.clone(),
                    focus: summary.focus.clone(),
                    path_lift_prefix: summary.lift.prefix.clone(),
                    root_path: format_root_path(summary),
                    focus_fields: vec![],
                });
            }
            _ => {}
        }
    }
    let mut candidates: Vec<_> = typed.summaries.keys().cloned().collect();
    candidates.sort();
    let span = file_level_span(typed);
    Err(vec![diag::explain_unknown_node_diag(
        span,
        node,
        &candidates,
    )])
}

/// Like `explain_focus` but fails when `node` has blocking TYP-* diagnostics.
pub fn explain_focus_with_diags(
    typed: &TypedHir,
    node: &str,
    diags: &[Diagnostic],
) -> TypeckResult<FocusReport> {
    let blocking: Vec<_> = diags
        .iter()
        .filter(|d| is_explain_blocking_for_node(d, node))
        .cloned()
        .collect();
    if !blocking.is_empty() {
        return Err(blocking);
    }
    explain_focus(typed, node)
}

fn build_grade_report(
    node: &str,
    grade_ann: Option<&syn::GradeExpr>,
    summary: &OpticSummary,
) -> GradeReport {
    let inferred = infer_grade_from_summary(summary);
    let declared = grade_ann.map(|g| declared_grade_snapshot(g, &inferred));
    GradeReport {
        optic: node.into(),
        declared: declared.unwrap_or_else(|| inferred_grade_snapshot(&inferred)),
        inferred: inferred_grade_snapshot(&inferred),
        regions: RegionsSnapshot {
            get_reads: summary.get_reads.clone(),
            put_reads: summary.put_reads.clone(),
            put_writes: summary.put_writes.clone(),
        },
    }
}

fn declared_grade_snapshot(g: &syn::GradeExpr, inferred: &ConcreteGrade) -> GradeSnapshot {
    let declared = hir::extract_grade_from_ann(g);
    let elided = hir::cache_grade_elided(g);
    let ann_cache = hir::annotated_cache_bound(g);
    let cache_source = if elided {
        "elided"
    } else if ann_cache.is_some() {
        "annotation"
    } else {
        "default"
    };
    GradeSnapshot {
        cache: if elided { inferred.cache } else { declared.cache },
        cache_source: cache_source.into(),
        ownership_share: format!(
            "{}/{}",
            declared.ownership.share.num, declared.ownership.share.den
        ),
        ownership_alias: hir::ownership_grade_alias(g),
        read_only: declared.ownership.read_only,
        must_use: declared.ownership.must_use,
    }
}

fn inferred_ownership_alias(g: &ConcreteGrade) -> Option<String> {
    if g.ownership.read_only {
        Some("SharedGrade".into())
    } else if g.ownership.share.num > 0
        && g.ownership.share.den > 0
        && g.ownership.share.num == g.ownership.share.den as i64
    {
        Some("AffineGrade".into())
    } else {
        None
    }
}

fn inferred_grade_snapshot(g: &ConcreteGrade) -> GradeSnapshot {
    GradeSnapshot {
        cache: g.cache,
        cache_source: "inferred".into(),
        ownership_share: format!("{}/{}", g.ownership.share.num, g.ownership.share.den),
        ownership_alias: inferred_ownership_alias(g),
        read_only: g.ownership.read_only,
        must_use: g.ownership.must_use,
    }
}

fn collect_known_types(hir: &hir::HirProgram) -> HashSet<String> {
    let mut out: HashSet<String> = PRIMITIVE_TYPES.iter().map(|s| (*s).into()).collect();
    for item in &hir.items {
        if let hir::HirItem::Data(d) = item {
            out.insert(d.name.node.clone());
        }
    }
    out
}

fn type_expr_span(te: &TypeExpr) -> Span {
    match te {
        TypeExpr::Named { span, .. }
        | TypeExpr::Tuple(_, span)
        | TypeExpr::Soa(_, span)
        | TypeExpr::BitSet(span) => *span,
    }
}

fn type_expr_root_name(te: &TypeExpr) -> Option<String> {
    match te {
        TypeExpr::Named { name, .. } => Some(name.clone()),
        TypeExpr::Tuple(_, span) => Some(format!("tuple@{}", span.start)),
        TypeExpr::Soa(_, span) => Some(format!("SoA@{}", span.start)),
        TypeExpr::BitSet(span) => Some(format!("BitSet@{}", span.start)),
    }
}

fn validate_graded_optic_types(
    ty: &syn::GradeOpticType,
    known: &HashSet<String>,
    binding: &str,
) -> Vec<Diagnostic> {
    let mut out = vec![];
    if let Some(name) = type_expr_root_name(&ty.costate) {
        if !known.contains(&name) && !name.starts_with("tuple@") && !name.starts_with("SoA@") {
            out.push(diag::type_unknown_diag(
                type_expr_span(&ty.costate),
                &name,
                "costate",
                Some(binding),
                true,
            ));
        }
    }
    if let Some(name) = type_expr_root_name(&ty.focus) {
        if !known.contains(&name) && !name.starts_with("tuple@") && !name.starts_with("SoA@") {
            out.push(diag::type_unknown_diag(
                type_expr_span(&ty.focus),
                &name,
                "focus",
                Some(binding),
                true,
            ));
        }
    }
    out
}

fn validate_optic_types(decl: &syn::OpticDecl, known: &HashSet<String>) -> Vec<Diagnostic> {
    let optic = decl.name.node.as_str();
    let mut out = vec![];
    if let Some(name) = type_expr_root_name(&decl.costate) {
        if !known.contains(&name) && !name.starts_with("tuple@") && !name.starts_with("SoA@") {
            out.push(diag::type_unknown_diag(
                type_expr_span(&decl.costate),
                &name,
                "costate",
                Some(optic),
                false,
            ));
        }
    }
    if let Some(name) = type_expr_root_name(&decl.focus) {
        if !known.contains(&name) && !name.starts_with("tuple@") && !name.starts_with("SoA@") {
            out.push(diag::type_unknown_diag(
                type_expr_span(&decl.focus),
                &name,
                "focus",
                Some(optic),
                false,
            ));
        }
    }
    out
}

fn validate_grade_syntax(g: &syn::GradeExpr, optic: &str) -> Vec<Diagnostic> {
    let mut out = vec![];
    for dim in &g.dims {
        match dim {
            syn::GradeDim::Ownership { r: Some(txt), span } => {
                if !valid_ownership_rational(txt) {
                    out.push(diag::type_grade_syntax_diag(
                        *span,
                        &format!("invalid OwnershipGrade rational `{txt}` — expected num/den"),
                        txt,
                        optic,
                    ));
                }
            }
            syn::GradeDim::Named { name, span } => {
                if !matches!(
                    name.as_str(),
                    "LinearGrade" | "AffineGrade" | "SharedGrade"
                ) {
                    out.push(diag::type_grade_syntax_diag(
                        *span,
                        &format!("unknown grade dimension `{name}`"),
                        name,
                        optic,
                    ));
                }
            }
            _ => {}
        }
    }
    out
}

fn valid_ownership_rational(txt: &str) -> bool {
    let Some((a, b)) = txt.split_once('/') else {
        return false;
    };
    let (Ok(n), Ok(d)) = (a.parse::<i64>(), b.parse::<u64>()) else {
        return false;
    };
    n > 0 && d > 0
}

fn check_optic_body_types(decl: &syn::OpticDecl, region_map: &hir::RegionMap) -> Vec<Diagnostic> {
    let mut out = vec![];
    let optic = decl.name.node.as_str();
    let focus = type_expr_root_name(&decl.focus).unwrap_or_else(|| "unknown".into());
    let costate = type_expr_root_name(&decl.costate).unwrap_or_else(|| "unknown".into());
    if let Some(get) = decl.get.as_ref() {
        match infer_surface_expr_type(&get.body, &get.param.node, &costate, region_map) {
            Ok(actual) => {
                if types_differ(&focus, &actual) {
                    out.push(diag::type_body_mismatch_diag(
                        get.span,
                        &focus,
                        &actual,
                        "get",
                        optic,
                    ));
                }
            }
            Err(()) => {
                out.push(diag::type_body_uninferable_diag(get.span, "get", optic));
            }
        }
    }
    if let Some(put) = &decl.put {
        match infer_put_write_target_type(put, &put.state_param.node, &costate, region_map) {
            Ok(actual) => {
                if types_differ(&focus, &actual) {
                    out.push(diag::type_body_mismatch_diag(
                        put.span,
                        &focus,
                        &actual,
                        "put",
                        optic,
                    ));
                }
            }
            Err(()) => out.push(diag::type_body_uninferable_diag(put.span, "put", optic)),
        }
    }
    out
}

fn types_differ(expected: &str, actual: &str) -> bool {
    normalize_type_name(expected) != normalize_type_name(actual)
}

fn normalize_type_name(t: &str) -> String {
    match t {
        "Vec2" => "(f32, f32)".into(),
        other => other.into(),
    }
}

fn infer_put_write_target_type(
    put: &syn::PutClause,
    state_param: &str,
    costate: &str,
    region_map: &hir::RegionMap,
) -> Result<String, ()> {
    let body = &put.body;
    if let syn::Expr::Block { stmts, result, .. } = body {
        for stmt in stmts {
            if let Some(t) = infer_assign_target_type(&stmt.expr, state_param, costate, region_map) {
                return Ok(t);
            }
        }
        if let Some(r) = result {
            if let Some(t) = infer_assign_target_type(r, state_param, costate, region_map) {
                return Ok(t);
            }
        }
    }
    if let Some(t) = infer_assign_target_type(body, state_param, costate, region_map) {
        return Ok(t);
    }
    // Lens put bodies may be pure value expressions (e.g. `v / 2.0`) rather than assignments.
    infer_surface_expr_type(body, &put.value_param.node, costate, region_map)
}

fn infer_assign_target_type(
    e: &syn::Expr,
    state_param: &str,
    costate: &str,
    region_map: &hir::RegionMap,
) -> Option<String> {
    if let syn::Expr::Assign { target, .. } = e {
        if let syn::Expr::Field(fe) = target.as_ref() {
            return infer_field_expr_type(fe, state_param, costate, region_map).ok();
        }
    }
    None
}

fn infer_surface_expr_type(
    e: &syn::Expr,
    param: &str,
    costate: &str,
    region_map: &hir::RegionMap,
) -> Result<String, ()> {
    match e {
        syn::Expr::Atom(syn::AtomExpr::Float(_, _)) => Ok("f32".into()),
        syn::Expr::Atom(syn::AtomExpr::Int(_, _)) => Ok("i32".into()),
        syn::Expr::Atom(syn::AtomExpr::Ident(id)) if id.node == param => {
            Ok(costate.into())
        }
        syn::Expr::Atom(syn::AtomExpr::Tuple(elems, _)) => {
            let parts: Result<Vec<_>, _> = elems
                .iter()
                .map(|el| infer_surface_expr_type(el, param, costate, region_map))
                .collect();
            Ok(format!("({})", parts?.join(", ")))
        }
        syn::Expr::Atom(syn::AtomExpr::Paren(inner, _)) => {
            infer_surface_expr_type(inner, param, costate, region_map)
        }
        syn::Expr::Field(fe) => infer_field_expr_type(fe, param, costate, region_map),
        syn::Expr::Binary { right, .. } => infer_surface_expr_type(right, param, costate, region_map),
        syn::Expr::Block { result, .. } => result
            .as_ref()
            .map(|r| infer_surface_expr_type(r, param, costate, region_map))
            .unwrap_or(Err(())),
        _ => Err(()),
    }
}

fn infer_field_expr_type(
    fe: &syn::FieldExpr,
    param: &str,
    costate: &str,
    region_map: &hir::RegionMap,
) -> Result<String, ()> {
    match fe {
        syn::FieldExpr::Index { base, .. } => {
            if let syn::FieldExpr::FieldAccess {
                base: inner,
                field,
                ..
            } = &**base
            {
                if let syn::FieldExpr::Base(syn::AtomExpr::Ident(id), _) = &**inner {
                    if id.node == param {
                        return column_element_type(&field.node, region_map);
                    }
                }
            }
            Err(())
        }
        syn::FieldExpr::FieldAccess { base, field, .. } => {
            if let Some(mut path) = focus_field_path(param, base) {
                path.push(field.node.clone());
                return nested_field_type(costate, &path, region_map);
            }
            Err(())
        }
        _ => Err(()),
    }
}

fn focus_field_path(param: &str, fe: &syn::FieldExpr) -> Option<Vec<String>> {
    match fe {
        syn::FieldExpr::Base(syn::AtomExpr::Ident(id), _) if id.node == param => Some(vec![]),
        syn::FieldExpr::FieldAccess { base, field, .. } => {
            let mut path = focus_field_path(param, base)?;
            path.push(field.node.clone());
            Some(path)
        }
        _ => None,
    }
}

fn column_element_type(column: &str, region_map: &hir::RegionMap) -> Result<String, ()> {
    region_map
        .columns
        .get(column)
        .and_then(|c| c.element_ty.clone())
        .ok_or(())
}

fn nested_field_type(
    record: &str,
    path: &[String],
    region_map: &hir::RegionMap,
) -> Result<String, ()> {
    if path.is_empty() {
        return Ok(record.into());
    }
    let mut ty = record.to_string();
    for seg in path {
        let fields = region_map.record_fields.get(&ty).ok_or(())?;
        ty = fields.get(seg).ok_or(())?.clone();
    }
    Ok(ty)
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
    fn test_sibling_nested_field_regions_overlap_for_alias() {
        assert!(is_subregion("transforms.position", "transforms.velocity"));
        let left = mk_sum(
            "L",
            vec!["transforms.position"],
            vec!["transforms.position"],
        );
        let right = mk_sum(
            "R",
            vec!["transforms.velocity"],
            vec!["transforms.velocity"],
        );
        assert!(
            alias_safe(&left, &right).is_err(),
            "sibling nested fields must alias-conflict in product"
        );
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
    fn test_par_lift_rejects_non_identity_path_lift() {
        let src = r#"
data Entities { transforms: SoA<Transform>, healths: SoA<f32> }
data Transform { position: Vec2 }
optic TransformView: GradedOptic<Entities, Transform, _> {
    get s=>s.transforms[s.id] put(s,v)=>{s.transforms[s.id]=v}
}
optic PositionField: GradedOptic<Transform, Vec2, _> {
    get t=>t.position put(t,v)=>{t.position=v}
}
optic HealthView: GradedOptic<Entities, f32, _> {
    get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v}
}
let nested = TransformView >>> PositionField;
fn main() { entities.query(nested *** HealthView).map(|(p,h)| (p,h)); }
"#;
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let err = check(hirp).unwrap_err();
        assert!(
            err.iter().any(|d| d.rule.contains("identity PathLift")),
            "expected par lift rejection: {err:?}"
        );
    }

    #[test]
    fn test_typ001_unknown_costate() {
        let src = r#"
data Entities { healths: SoA<f32> }
optic X: GradedOptic<Ghost, f32, _> {
  get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v}
}
fn main() { entities.query(X).get(); }
"#;
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let err = check(hirp).unwrap_err();
        assert!(err.iter().any(|d| d.code == diag::TYPE_UNKNOWN));
    }

    #[test]
    fn test_typ002_get_body_focus_mismatch() {
        let src = r#"
data Entities { healths: SoA<f32>, positions: SoA<Vec2> }
optic Bad: GradedOptic<Entities, f32, _> {
  get s=>s.positions[s.id] put(s,v)=>{s.healths[s.id]=v}
}
fn main() { entities.query(Bad).get(); }
"#;
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let err = check(hirp).unwrap_err();
        let d = err
            .iter()
            .find(|d| d.code == diag::TYPE_BODY_MISMATCH)
            .expect("TYP-002");
        assert_eq!(d.evidence["expected_type"], "f32");
        assert_eq!(d.evidence["actual_type"], "(f32, f32)");
    }

    #[test]
    fn test_typ003_invalid_grade_syntax() {
        let src = r#"
data Entities { healths: SoA<f32> }
optic X: GradedOptic<Entities, f32, OwnershipGrade<not_a_rational> + _> {
  get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v}
}
fn main() { entities.query(X).get(); }
"#;
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let err = check(hirp).unwrap_err();
        assert!(err.iter().any(|d| d.code == diag::TYPE_GRADE_SYNTAX));
    }

    #[test]
    fn test_explain_grade_healthview() {
        let src = include_str!("../../../examples/health_get.opt");
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let (typed, _) = typeck_pass(hirp);
        let report = explain_grade(&typed, "HealthView").expect("explain");
        assert_eq!(report.optic, "HealthView");
        assert_eq!(report.inferred.cache, 2);
        assert_eq!(report.declared.ownership_alias.as_deref(), Some("AffineGrade"));
        assert!(report.regions.get_reads.contains(&"healths".to_string()));
    }

    #[test]
    fn test_explain_grade_despite_gra110() {
        let src = include_str!("../../../examples/invalid_grade.opt");
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let (typed, diags) = typeck_pass(hirp);
        assert!(diags.iter().any(|d| d.code == diag::GRADE_DECL_TIGHT));
        let report =
            explain_grade_with_diags(&typed, "BadCache", &diags).expect("explain despite other errors");
        assert_eq!(report.inferred.cache, 3);
    }

    #[test]
    fn test_explain_grade_fails_typ001_on_target() {
        let src = include_str!("../../../examples/typ001_unknown_type.opt");
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let (typed, diags) = typeck_pass(hirp);
        let err = explain_grade_with_diags(&typed, "GhostView", &diags).unwrap_err();
        assert!(err.iter().any(|d| d.code == diag::TYPE_UNKNOWN));
    }

    #[test]
    fn test_explain_grade_let_binding() {
        let src = include_str!("../../../examples/nested_position.opt");
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let (typed, _) = typeck_pass(hirp);
        let report = explain_grade(&typed, "nested").expect("let binding");
        assert_eq!(report.inferred.cache, 4);
        assert_eq!(report.declared.cache_source, "annotation");
    }

    #[test]
    fn test_collect_unsupported_surface_prism() {
        let src = include_str!("../../../examples/unsupported_prism.opt");
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let diags = collect_unsupported_surface(&prog);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, diag::TYPE_UNSUPPORTED_V0);
        assert_eq!(
            diags[0].evidence["feature"].as_str(),
            Some("prism")
        );
        assert_eq!(diags[0].evidence["name"].as_str(), Some("AliveFilter"));
    }

    #[test]
    fn test_collect_unsupported_surface_host_boundary() {
        let src = include_str!("../../../examples/host_boundary.opt");
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let diags = collect_unsupported_surface(&prog);
        assert_eq!(diags.len(), 2);
        let features: std::collections::HashSet<_> = diags
            .iter()
            .filter_map(|d| d.evidence["feature"].as_str())
            .collect();
        assert!(features.contains("foreign_decl"));
        assert!(features.contains("unsafe_optic"));
    }

    #[test]
    fn test_collect_unsupported_surface_traversal() {
        let src = r#"
data Entities { healths: SoA<f32> }
optic Scan: GradedTraversal<Entities, f32, _> {
    get s => s.healths[s.id]
}
"#;
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let diags = collect_unsupported_surface(&prog);
        assert!(diags.iter().any(|d| d.evidence["feature"].as_str() == Some("traversal")));
    }

    #[test]
    fn test_explain_focus_lenient_when_other_prism_in_file() {
        let src = r#"
data Entities { healths: SoA<f32> }
optic HealthView: GradedOptic<Entities, f32, CacheGrade<2>> {
    get s => s.healths[s.id]
    put (s, v) => { s.healths[s.id] = v }
}
optic AliveFilter: GradedPrism<Entities, f32, CacheGrade<1>> {
    preview s => s.healths[s.id]
    review (s, a) => { s.healths[s.id] = a }
}
"#;
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let unsupported = collect_unsupported_surface(&prog);
        let hirp = optic_hir::lower(prog).expect("lower");
        assert!(
            !hirp
                .items
                .iter()
                .any(|i| matches!(i, hir::HirItem::Optic { decl, .. } if decl.name.node == "AliveFilter")),
            "unsupported prism must not lower into HIR"
        );
        let (typed, diags) = typeck_pass(hirp);
        let mut combined = diags;
        combined.extend(unsupported);
        let report = explain_focus_with_diags(&typed, "HealthView", &combined).expect("lenient");
        assert_eq!(report.root_path, "entities.healths[id]");
    }

    #[test]
    fn test_explain_focus_blocks_typ010_on_target() {
        let src = include_str!("../../../examples/unsupported_prism.opt");
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let unsupported = collect_unsupported_surface(&prog);
        let hirp = optic_hir::lower(prog).expect("lower");
        let (typed, diags) = typeck_pass(hirp);
        let mut combined = diags;
        combined.extend(unsupported);
        let err = explain_focus_with_diags(&typed, "AliveFilter", &combined).unwrap_err();
        assert!(err.iter().any(|d| d.code == diag::TYPE_UNSUPPORTED_V0));
    }

    #[test]
    fn test_explain_focus_healthview() {
        let src = include_str!("../../../examples/health_get.opt");
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let (typed, diags) = typeck_pass(hirp);
        let report = explain_focus_with_diags(&typed, "HealthView", &diags).expect("focus");
        assert_eq!(report.root_path, "entities.healths[id]");
        assert!(report.path_lift_prefix.is_empty());
    }

    #[test]
    fn test_explain_focus_nested_let_binding() {
        let src = include_str!("../../../examples/nested_position.opt");
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let (typed, diags) = typeck_pass(hirp);
        let report = explain_focus_with_diags(&typed, "nested", &diags).expect("nested let focus");
        assert_eq!(report.root_path, "entities.transforms[id].position");
        assert_eq!(report.path_lift_prefix, vec!["position".to_string()]);
        assert_eq!(report.costate, "Entities");
        assert_eq!(report.focus, "Transform");
    }

    #[test]
    fn test_explain_focus_blocks_typ002_on_target() {
        let src = include_str!("../../../examples/typ002_body_mismatch.opt");
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let (typed, diags) = typeck_pass(hirp);
        let err = explain_focus_with_diags(&typed, "BadFocus", &diags).unwrap_err();
        assert!(err.iter().any(|d| d.code == diag::TYPE_BODY_MISMATCH));
    }

    #[test]
    fn test_explain_unknown_node_exp001() {
        let src = include_str!("../../../examples/health_get.opt");
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let (typed, _) = typeck_pass(hirp);
        let err = explain_grade(&typed, "NoSuchOptic").unwrap_err();
        assert!(err.iter().any(|d| d.code == diag::EXPLAIN_UNKNOWN_NODE));
    }

    #[test]
    fn test_typ004_uninferable_get_body() {
        let src = include_str!("../../../examples/typ004_uninferable_body.opt");
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let err = check(hirp).unwrap_err();
        assert!(err.iter().any(|d| d.code == diag::TYPE_BODY_UNINFERABLE));
    }

    #[test]
    fn test_typ001_unknown_type_on_let_binding() {
        let src = r#"
data Entities { healths: SoA<f32> }
optic H: GradedOptic<Entities,f32,_> { get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v} }
let bad: GradedOptic<Ghost,f32,_> = H;
fn main() { entities.query(bad).get(); }
"#;
        let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let err = check(hirp).unwrap_err();
        assert!(err.iter().any(|d| d.code == diag::TYPE_UNKNOWN));
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
