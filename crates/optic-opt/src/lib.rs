//! optic-opt — the 3 fusions (ch. 10, M4).
//! Fixed-point driver (≤8 iters): map fusion, compose fusion, product flatten (ch10 order).

use optic_cgir::{
    compose_leaf_chain, compose_tree_node_ids, node_span, verify_to_diagnostic, CgirGraph,
    CgirNode, ComposeFusedBody, FusionProvenance, FusionReason, NodeId,
};
use optic_diagnostics::Diagnostic;
use optic_hir as hir;
use optic_syntax::Span;

const MAX_FUSION_ITERS: usize = 8;

/// Post-fusion graph plus non-fatal fusion notes (FUS-501 escape, FUS-502 legality skips).
#[derive(Clone, Debug)]
pub struct OptimizeResult {
    pub graph: CgirGraph,
    pub fusion_notes: Vec<Diagnostic>,
}

/// Run fusion passes until fixed point, skipping compose body rewrite (for equivalence tests).
pub fn optimize_without_compose_fusion(g: CgirGraph) -> Result<CgirGraph, String> {
    optimize_without_compose_fusion_reporting(g)
        .map(|r| r.graph)
        .map_err(|d| d.rule.clone())
}

#[allow(clippy::result_large_err)]
fn fusion_verify(g: &CgirGraph) -> Result<(), Diagnostic> {
    verify_to_diagnostic(g)
}

#[allow(clippy::result_large_err)]
pub fn optimize_without_compose_fusion_reporting(
    mut g: CgirGraph,
) -> Result<OptimizeResult, Diagnostic> {
    let mut fusion_notes = vec![];
    for _ in 0..MAX_FUSION_ITERS {
        let n_before = g.nodes.len();
        g = map_fusion(g);
        fusion_verify(&g)?;
        let (g2, flat_notes) = product_flatten(g);
        g = g2;
        fusion_notes.extend(flat_notes);
        fusion_verify(&g)?;
        if g.nodes.len() == n_before {
            break;
        }
    }
    fusion_verify(&g)?;
    debug_assert!(
        verify_to_diagnostic(&g).is_ok(),
        "post-optimize CGIR must satisfy invariants (wiring, ProductFlat, provenance)"
    );
    Ok(OptimizeResult {
        graph: g,
        fusion_notes,
    })
}

/// Run fusion passes until fixed point. Aborts on verify violation (ch10 post-fusion check).
/// debug_assert post-fusion for CGIR invariants added for robustness.
#[allow(clippy::result_large_err)]
pub fn optimize(mut g: CgirGraph) -> Result<OptimizeResult, Diagnostic> {
    let mut fusion_notes = vec![];
    for _ in 0..MAX_FUSION_ITERS {
        let n_before = g.nodes.len();
        g = map_fusion(g);
        fusion_verify(&g)?;
        let (g2, notes) = compose_fusion(g);
        g = g2;
        fusion_notes.extend(notes);
        fusion_verify(&g)?;
        let (g3, flat_notes) = product_flatten(g);
        g = g3;
        fusion_notes.extend(flat_notes);
        fusion_verify(&g)?;
        if g.nodes.len() == n_before {
            break;
        }
    }
    fusion_verify(&g)?;
    debug_assert!(
        verify_to_diagnostic(&g).is_ok(),
        "post-optimize-without CGIR must satisfy invariants"
    );
    Ok(OptimizeResult {
        graph: g,
        fusion_notes,
    })
}

/// Map fusion: collapse chained QueryMap over same costate (CGIR infrastructure; HIR fuses surface chains).
fn map_fusion(mut g: CgirGraph) -> CgirGraph {
    let map_roots: Vec<NodeId> = g
        .roots
        .iter()
        .copied()
        .filter(|&r| matches!(g.nodes.get(r as usize), Some(CgirNode::QueryMap { .. })))
        .collect();
    if map_roots.len() <= 1 {
        return g;
    }

    let mut chain: Vec<(NodeId, String, Span)> = vec![];
    let mut costate = String::new();
    for &rid in &map_roots {
        if let Some(CgirNode::QueryMap {
            costate: cs,
            map_param,
            provenance,
            ..
        }) = g.nodes.get(rid as usize)
        {
            if chain.is_empty() {
                costate = cs.clone();
            } else if cs != &costate {
                return g;
            }
            chain.push((rid, map_param.clone(), *provenance));
        }
    }
    if chain.len() <= 1 {
        return g;
    }

    let fused_id = chain[0].0;
    let param = chain[0].1.clone();
    let span = chain[0].2;
    let first_body = match g.nodes.get(fused_id as usize) {
        Some(CgirNode::QueryMap { map_body, .. }) => map_body.as_ref(),
        _ => return g,
    };
    let mut body = first_body.clone();
    for &(rid, ref inner_param, _) in chain.iter().skip(1) {
        let next_body = match g.nodes.get(rid as usize) {
            Some(CgirNode::QueryMap { map_body, .. }) => map_body.as_ref(),
            _ => return g,
        };
        let params: Vec<&str> = inner_param.split(',').map(str::trim).collect();
        if params.len() > 1 && !matches!(body, hir::HirExpr::Tuple(_, _)) {
            return g;
        }
        body = substitute_all_params(next_body, inner_param, &body);
    }

    if let Some(CgirNode::QueryMap {
        map_param,
        map_body,
        provenance,
        ..
    }) = g.nodes.get_mut(fused_id as usize)
    {
        *map_param = param;
        let slot = std::sync::Arc::make_mut(map_body);
        *slot = body;
        *provenance = span;
    }

    let orig_ids: Vec<NodeId> = chain.iter().map(|(id, ..)| *id).collect();
    g.roots = vec![fused_id];
    g.provenance_index.insert(
        fused_id,
        FusionProvenance {
            original_ids: orig_ids,
            spans: vec![span],
            reason: FusionReason::MapFusion,
        },
    );
    g
}

fn substitute_all_params(e: &hir::HirExpr, param_str: &str, repl: &hir::HirExpr) -> hir::HirExpr {
    let params: Vec<&str> = param_str.split(',').map(str::trim).collect();
    if params.len() > 1 {
        if !matches!(repl, hir::HirExpr::Tuple(_, _)) {
            return e.clone();
        }
        if let hir::HirExpr::Tuple(elems, _) = repl {
            let mut out = e.clone();
            for (p, el) in params.iter().zip(elems.iter()) {
                out = substitute_map_body(&out, p, el);
            }
            return out;
        }
    }
    let inner = params.first().copied().unwrap_or("it");
    substitute_map_body(e, inner, repl)
}

/// Map-fusion substitution (ch10 compose body rewrite).
///
/// Differs from [`hir::substitute_hir_var`] on `FocusField`: here a matching `param`
/// replaces the entire `FocusField` node with `repl` (tuple projection / bind splice).
/// `substitute_hir_var` only renames the `param` ident when `repl` is a `Var`.
fn substitute_map_body(e: &hir::HirExpr, name: &str, repl: &hir::HirExpr) -> hir::HirExpr {
    match e {
        hir::HirExpr::CursorField {
            cursor,
            field: _,
            span: _,
        }
        | hir::HirExpr::CursorIndex {
            cursor,
            field: _,
            span: _,
        } => {
            if cursor == name {
                repl.clone()
            } else {
                e.clone()
            }
        }
        hir::HirExpr::LitInt(n, sp) => hir::HirExpr::LitInt(*n, *sp),
        hir::HirExpr::LitFloat(f, sp) => hir::HirExpr::LitFloat(*f, *sp),
        hir::HirExpr::Var(v, sp) if v == name => repl.clone(),
        hir::HirExpr::Var(v, sp) => hir::HirExpr::Var(v.clone(), *sp),
        hir::HirExpr::Bin {
            op,
            left,
            right,
            span,
        } => hir::HirExpr::Bin {
            op: *op,
            left: Box::new(substitute_map_body(left, name, repl)),
            right: Box::new(substitute_map_body(right, name, repl)),
            span: *span,
        },
        hir::HirExpr::Paren(inner, sp) => {
            hir::HirExpr::Paren(Box::new(substitute_map_body(inner, name, repl)), *sp)
        }
        hir::HirExpr::Tuple(elems, sp) => hir::HirExpr::Tuple(
            elems
                .iter()
                .map(|el| substitute_map_body(el, name, repl))
                .collect(),
            *sp,
        ),
        hir::HirExpr::TupleProj { base, index, span } => hir::HirExpr::TupleProj {
            base: Box::new(substitute_map_body(base, name, repl)),
            index: *index,
            span: *span,
        },
        hir::HirExpr::FocusField { param, path, span } => {
            if param == name {
                repl.clone()
            } else {
                hir::HirExpr::FocusField {
                    param: param.clone(),
                    path: path.clone(),
                    span: *span,
                }
            }
        }
        hir::HirExpr::Unsupported { reason, span } => hir::HirExpr::Unsupported {
            reason: reason.clone(),
            span: *span,
        },
    }
}

/// Compose fusion: merge QueryMap(Compose(A,B), f) into a single FusedLoop body (ch10).
fn compose_fusion(mut g: CgirGraph) -> (CgirGraph, Vec<Diagnostic>) {
    let map_root = g
        .roots
        .iter()
        .find(|&&r| matches!(g.nodes.get(r as usize), Some(CgirNode::QueryMap { .. })));
    let Some(&map_root) = map_root else {
        return (g, vec![]);
    };

    let Some(CgirNode::QueryMap {
        optic_name,
        costate,
        map_param,
        map_body,
        provenance: map_span,
        ..
    }) = g.nodes.get(map_root as usize).cloned()
    else {
        return (g, vec![]);
    };

    let Some(&compose_id) = g.resolved_optics.get(&optic_name) else {
        return (g, vec![]);
    };
    if !matches!(
        g.nodes.get(compose_id as usize),
        Some(CgirNode::Compose { .. })
    ) {
        return (g, vec![]);
    };

    let Some(compose_node) = g.nodes.get(compose_id as usize) else {
        return (g, vec![]);
    };
    let compose_span = node_span(compose_node);
    let span = map_span.merge(compose_span);

    if let Some(note) =
        compose_fusion_block_note(&g, compose_id, map_body.as_ref(), &map_param, span)
    {
        return (g, vec![note]);
    }

    let Some(chain) = compose_leaf_chain(&g, compose_id) else {
        return (g, vec![]);
    };
    if chain.len() < 2 {
        return (g, vec![]);
    }
    debug_assert!(chain.len() >= 2, "compose chain len >=2 post guard");
    let entry = chain[0];
    let exit = chain[chain.len() - 1];

    let already = g
        .provenance_index
        .values()
        .any(|p| p.reason == FusionReason::ComposeFusion && p.original_ids.contains(&compose_id));
    if already {
        return (g, vec![]);
    }

    let fid: NodeId = g.nodes.len() as u32;
    let mut orig = compose_tree_node_ids(&g, compose_id);
    if !orig.contains(&map_root) {
        orig.push(map_root);
    }
    g.nodes.push(CgirNode::FusedLoop {
        id: fid,
        original_ids: orig.clone(),
        costate: costate.clone(),
        provenance: span,
        compose_body: Some(ComposeFusedBody {
            compose_id,
            lhs: entry,
            rhs: exit,
            map_param: map_param.clone(),
            map_body: std::sync::Arc::clone(&map_body),
        }),
    });
    g.provenance_index.insert(
        fid,
        FusionProvenance {
            original_ids: orig,
            spans: vec![span],
            reason: FusionReason::ComposeFusion,
        },
    );
    g.roots = vec![fid];
    (g, vec![])
}

fn compose_fusion_block_note(
    g: &CgirGraph,
    compose_id: NodeId,
    map_body: &hir::HirExpr,
    map_param: &str,
    span: Span,
) -> Option<Diagnostic> {
    if let Some((reason, leaf_id)) = optic_cgir::compose_chain_forbidden_leaf(g, compose_id) {
        let msg = match reason {
            "prism_in_compose" => "compose fusion blocked — compose chain contains PrismLeaf",
            "traversal_in_compose" => {
                "compose fusion blocked — compose chain contains TraversalLeaf"
            }
            _ => "compose fusion blocked — compose chain contains unsupported leaf",
        };
        return Some(optic_diagnostics::fusion_compose_legality_diag(
            span,
            msg,
            serde_json::json!({
                "reason": reason,
                "compose_id": compose_id,
                "leaf_id": leaf_id,
            }),
        ));
    }
    let chain = compose_leaf_chain(g, compose_id)?;
    if chain.len() < 2 {
        return Some(optic_diagnostics::fusion_compose_legality_diag(
            span,
            "compose fusion blocked — compose chain must contain at least two optic leaves",
            serde_json::json!({
                "reason": "non_leaf_chain",
                "compose_id": compose_id,
                "leaves": chain.len()
            }),
        ));
    }
    for w in chain.windows(2) {
        let (Some(lhs_sum), Some(rhs_sum)) = (
            g.nodes.get(w[0] as usize).and_then(leaf_summary),
            g.nodes.get(w[1] as usize).and_then(leaf_summary),
        ) else {
            return Some(optic_diagnostics::fusion_compose_legality_diag(
                span,
                "compose fusion blocked — compose chain must be optic leaves",
                serde_json::json!({
                    "reason": "non_leaf_child",
                    "lhs": w[0],
                    "rhs": w[1]
                }),
            ));
        };
        if !types_compatible(lhs_sum, rhs_sum) {
            return Some(optic_diagnostics::fusion_compose_legality_diag(
                span,
                "compose fusion blocked — intermediate focus/costate mismatch",
                serde_json::json!({
                    "reason": "focus_costate_mismatch",
                    "lhs_focus": lhs_sum.focus,
                    "rhs_costate": rhs_sum.costate,
                }),
            ));
        }
    }
    for &lid in &chain {
        let Some(sum) = g.nodes.get(lid as usize).and_then(leaf_summary) else {
            return Some(optic_diagnostics::fusion_compose_legality_diag(
                span,
                "compose fusion blocked — compose chain must be optic leaves",
                serde_json::json!({ "reason": "non_leaf_child", "leaf": lid }),
            ));
        };
        if sum.get_determinism != hir::Determinism::Pure
            || sum.put_determinism != hir::Determinism::Pure
        {
            return Some(optic_diagnostics::fusion_compose_legality_diag(
                span,
                "compose fusion blocked — optic determinism must be Pure",
                serde_json::json!({
                    "reason": "impurity",
                    "leaf": lid,
                    "get_det": format!("{:?}", sum.get_determinism),
                    "put_det": format!("{:?}", sum.put_determinism),
                }),
            ));
        }
    }
    debug_assert!(chain.len() >= 2, "compose chain len>=2");
    let entry = chain[0];
    let exit = chain[chain.len() - 1];
    if intermediate_escapes_query(g, entry, exit, map_body, map_param) {
        return Some(optic_diagnostics::fusion_compose_blocked_diag(
            span,
            "compose fusion blocked — intermediate value escapes",
            serde_json::json!({
                "reason": "escape",
                "map_param": map_param,
            }),
        ));
    }
    None
}

/// Canonical type-name equality for compose wiring (OpticSummary focus/costate).
fn types_compatible(lhs: &hir::OpticSummary, rhs: &hir::OpticSummary) -> bool {
    lhs.focus.eq_ignore_ascii_case(&rhs.costate)
}

/// Basic escape check (ch10): map body references the compose intermediate outside map_param.
fn intermediate_escapes_query(
    g: &CgirGraph,
    _lhs: NodeId,
    rhs: NodeId,
    map_body: &hir::HirExpr,
    map_param: &str,
) -> bool {
    let (leaf_param, focus) = match g.nodes.get(rhs as usize) {
        Some(CgirNode::OpticLeaf {
            get_param, focus, ..
        }) => (get_param.as_str(), focus.as_str()),
        Some(CgirNode::PrismLeaf {
            preview_param,
            focus,
            ..
        }) => (preview_param.as_str(), focus.as_str()),
        Some(CgirNode::TraversalLeaf {
            get_param, focus, ..
        }) => (get_param.as_str(), focus.as_str()),
        _ => return false,
    };
    // Rhs param names the intermediate flowing through compose (e.g. `x` in `get x => ...`).
    if leaf_param != map_param && hir::hir_expr_refs_var(map_body, leaf_param) {
        return true;
    }
    // Conservative: reject map bodies that reference the declared focus type name as an ident.
    if focus != map_param && hir::hir_expr_refs_var(map_body, focus) {
        return true;
    }
    false
}

fn leaf_summary(n: &CgirNode) -> Option<&hir::OpticSummary> {
    n.summary().map(|s| s.as_ref())
}

/// Collect leaf ids from a (possibly nested) Product tree in left-to-right order.
fn flatten_product_children(g: &CgirGraph, id: NodeId) -> Result<Vec<NodeId>, String> {
    match g.nodes.get(id as usize) {
        Some(CgirNode::Product { lhs, rhs, .. }) => {
            let mut out = flatten_product_children(g, *lhs)?;
            out.extend(flatten_product_children(g, *rhs)?);
            Ok(out)
        }
        Some(CgirNode::ProductFlat { children, .. }) => Ok(children.clone()),
        Some(CgirNode::OpticLeaf { .. })
        | Some(CgirNode::PrismLeaf { .. })
        | Some(CgirNode::TraversalLeaf { .. }) => Ok(vec![id]),
        Some(CgirNode::Tap { .. }) | Some(CgirNode::Record { .. }) => Err(format!(
            "flatten_product_children: node {id} is Tap/Record (observability orphan; not a product leaf)"
        )),
        // catch-all for future CGIR variants (prevents "fusion not updated" regressions; exhaust over known leaves + products)
        Some(_) => Err(format!(
            "flatten_product_children: node {id} is not Product/ProductFlat/OpticLeaf/PrismLeaf/TraversalLeaf"
        )),
        None => Err(format!("flatten_product_children: node {id} out of bounds")),
    }
}

fn product_is_nested(g: &CgirGraph, lhs: NodeId, rhs: NodeId) -> bool {
    matches!(
        g.nodes.get(lhs as usize),
        Some(CgirNode::Product { .. } | CgirNode::ProductFlat { .. })
    ) || matches!(
        g.nodes.get(rhs as usize),
        Some(CgirNode::Product { .. } | CgirNode::ProductFlat { .. })
    )
}

/// Product flattening: rewrite nested Product chains to ProductFlat (ch10.10.4).
fn product_flatten(mut g: CgirGraph) -> (CgirGraph, Vec<Diagnostic>) {
    let mut notes = vec![];
    let mut changed = true;
    while changed {
        changed = false;
        let candidates: Vec<(
            usize,
            NodeId,
            NodeId,
            NodeId,
            hir::ConcreteGrade,
            bool,
            Span,
        )> = g
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| {
                if let CgirNode::Product {
                    id,
                    lhs,
                    rhs,
                    grade,
                    alias_safe,
                    provenance,
                } = n
                {
                    // Two-pass loop: rewrite nested Product chains, then leaf-level products.
                    if product_is_nested(&g, *lhs, *rhs) {
                        return Some((i, *id, *lhs, *rhs, grade.clone(), *alias_safe, *provenance));
                    }
                }
                None
            })
            .collect();

        for (idx, id, lhs, rhs, grade, alias_safe, provenance) in candidates {
            if matches!(g.nodes.get(idx), Some(CgirNode::ProductFlat { .. })) {
                continue;
            }
            let children = match flatten_product_children(&g, lhs) {
                Ok(mut out) => match flatten_product_children(&g, rhs) {
                    Ok(rhs_children) => {
                        out.extend(rhs_children);
                        out
                    }
                    Err(e) => {
                        notes.push(optic_diagnostics::fusion_verify_diag(&format!(
                            "product flatten skipped for node {id}: {e}"
                        )));
                        continue;
                    }
                },
                Err(e) => {
                    notes.push(optic_diagnostics::fusion_verify_diag(&format!(
                        "product flatten skipped for node {id}: {e}"
                    )));
                    continue;
                }
            };
            if children.len() < 2 {
                continue;
            }
            let mut original_ids = children.clone();
            original_ids.push(id);
            g.nodes[idx] = CgirNode::ProductFlat {
                id,
                children,
                grade,
                alias_safe,
                provenance,
            };
            g.provenance_index.insert(
                id,
                FusionProvenance {
                    original_ids,
                    spans: vec![provenance],
                    reason: FusionReason::ProductFlattening,
                },
            );
            changed = true;
        }
    }

    // Normalize leaf-level Product nodes used by query optics to ProductFlat.
    let optic_products: Vec<NodeId> = g
        .resolved_optics
        .values()
        .copied()
        .filter(|&nid| matches!(g.nodes.get(nid as usize), Some(CgirNode::Product { .. })))
        .collect();
    for pid in optic_products {
        let idx = pid as usize;
        let Some(CgirNode::Product {
            id,
            lhs,
            rhs,
            grade,
            alias_safe,
            provenance,
        }) = g.nodes.get(idx).cloned()
        else {
            continue;
        };
        let children = match flatten_product_children(&g, lhs) {
            Ok(mut out) => match flatten_product_children(&g, rhs) {
                Ok(rhs_children) => {
                    out.extend(rhs_children);
                    out
                }
                Err(e) => {
                    notes.push(optic_diagnostics::fusion_verify_diag(&format!(
                        "product flatten skipped for optic product {id}: {e}"
                    )));
                    continue;
                }
            },
            Err(e) => {
                notes.push(optic_diagnostics::fusion_verify_diag(&format!(
                    "product flatten skipped for optic product {id}: {e}"
                )));
                continue;
            }
        };
        if children.len() < 2 {
            continue;
        }
        let mut original_ids = children.clone();
        original_ids.push(id);
        g.nodes[idx] = CgirNode::ProductFlat {
            id,
            children,
            grade,
            alias_safe,
            provenance,
        };
        g.provenance_index.insert(
            id,
            FusionProvenance {
                original_ids,
                spans: vec![provenance],
                reason: FusionReason::ProductFlattening,
            },
        );
    }
    (g, notes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use optic_cgir::verify;
    use optic_hir::{HirExpr, OwnershipDim, Rational};
    use optic_syntax::{BinOp, Span};
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn empty_resolved() -> std::collections::HashMap<String, NodeId> {
        std::collections::HashMap::new()
    }

    fn mk_leaf(
        id: NodeId,
        name: &str,
        costate: &str,
        focus: &str,
        sum: Arc<hir::OpticSummary>,
    ) -> CgirNode {
        CgirNode::OpticLeaf {
            id,
            name: name.into(),
            costate: costate.into(),
            focus: focus.into(),
            grade: sum.get_grade.clone(),
            get_fn: "".into(),
            put_fn: "".into(),
            get_param: "s".into(),
            get_body: Arc::new(HirExpr::Var("s".into(), Span::dummy())),
            put_state_param: None,
            put_value_param: None,
            put_value_body: None,
            summary: sum,
            provenance: Span::dummy(),
        }
    }

    #[test]
    fn map_fusion_substitutes_chained_bodies() {
        let body1 = HirExpr::Bin {
            op: BinOp::Sub,
            left: Box::new(HirExpr::Var("h".into(), Span::dummy())),
            right: Box::new(HirExpr::LitFloat(1.0, Span::dummy())),
            span: Span::dummy(),
        };
        let body2 = HirExpr::Bin {
            op: BinOp::Mul,
            left: Box::new(HirExpr::Var("x".into(), Span::dummy())),
            right: Box::new(HirExpr::LitFloat(2.0, Span::dummy())),
            span: Span::dummy(),
        };
        let sum = Arc::new(hir::OpticSummary {
            name: Some("H".into()),
            costate: "E".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let mut resolved = std::collections::HashMap::new();
        resolved.insert("H".into(), 0);
        let g = CgirGraph {
            nodes: vec![
                mk_leaf(0, "H", "E", "f32", Arc::clone(&sum)),
                CgirNode::QueryMap {
                    id: 1,
                    optic_name: "H".into(),
                    costate: "entities".into(),
                    cursor: "c".into(),
                    map_param: "h".into(),
                    map_body: std::sync::Arc::new(body1),
                    provenance: Span::dummy(),
                },
                CgirNode::QueryMap {
                    id: 2,
                    optic_name: "H".into(),
                    costate: "entities".into(),
                    cursor: "c".into(),
                    map_param: "x".into(),
                    map_body: std::sync::Arc::new(body2),
                    provenance: Span::dummy(),
                },
            ],
            roots: vec![1, 2],
            provenance_index: BTreeMap::new(),
            resolved_optics: resolved,
            region_map: hir::RegionMap::default(),
        };
        let out = optimize(g).expect("optimize should verify");
        assert_eq!(out.graph.roots, vec![1]);
        if let Some(CgirNode::QueryMap { map_body, .. }) = out.graph.nodes.get(1) {
            match map_body.as_ref() {
                HirExpr::Bin {
                    op: BinOp::Mul,
                    left,
                    ..
                } => match &**left {
                    HirExpr::Bin { op: BinOp::Sub, .. } => {}
                    other => panic!("expected fused sub inside mul, got {other:?}"),
                },
                other => panic!("expected fused mul body, got {other:?}"),
            }
        } else {
            panic!("missing fused QueryMap");
        }
    }

    #[test]
    fn optimize_aborts_on_invalid_graph() {
        let g = CgirGraph {
            nodes: vec![],
            roots: vec![0],
            provenance_index: BTreeMap::new(),
            resolved_optics: empty_resolved(),
            region_map: hir::RegionMap::default(),
        };
        assert!(optimize(g).is_err());
    }

    #[test]
    fn compose_fusion_fuses_nested_compose_chain() {
        let sum = Arc::new(hir::OpticSummary {
            name: Some("X".into()),
            costate: "f32".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let mut resolved = std::collections::HashMap::new();
        resolved.insert("pipe".into(), 4);
        let g = CgirGraph {
            nodes: vec![
                mk_leaf(0, "A", "Entities", "f32", Arc::clone(&sum)),
                mk_leaf(1, "B", "f32", "f32", Arc::clone(&sum)),
                mk_leaf(2, "C", "f32", "f32", Arc::clone(&sum)),
                CgirNode::Compose {
                    id: 3,
                    lhs: 0,
                    rhs: 1,
                    grade: sum.get_grade.clone(),
                    provenance: Span::dummy(),
                },
                CgirNode::Compose {
                    id: 4,
                    lhs: 3,
                    rhs: 2,
                    grade: sum.get_grade.clone(),
                    provenance: Span::dummy(),
                },
                CgirNode::QueryMap {
                    id: 5,
                    optic_name: "pipe".into(),
                    costate: "entities".into(),
                    cursor: "c".into(),
                    map_param: "x".into(),
                    map_body: Arc::new(HirExpr::Var("x".into(), Span::dummy())),
                    provenance: Span::dummy(),
                },
            ],
            roots: vec![5],
            provenance_index: BTreeMap::new(),
            resolved_optics: resolved,
            region_map: hir::RegionMap::default(),
        };
        let out = optimize(g).expect("nested compose fusion");
        let fused = out
            .graph
            .nodes
            .iter()
            .find_map(|n| match n {
                CgirNode::FusedLoop { compose_body, .. } => compose_body.as_ref(),
                _ => None,
            })
            .expect("fused body");
        assert_eq!(fused.lhs, 0, "entry leaf");
        assert_eq!(fused.rhs, 2, "exit leaf");
        assert_eq!(fused.compose_id, 4, "root compose");
        assert!(out.fusion_notes.is_empty());
    }

    #[test]
    fn compose_fusion_adds_fused_loop_provenance() {
        let sum_a = Arc::new(hir::OpticSummary {
            name: Some("A".into()),
            costate: "Entities".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let sum_b = Arc::new(hir::OpticSummary {
            name: Some("B".into()),
            costate: "f32".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec![],
            put_reads: vec![],
            put_writes: vec![],
            get_grade: sum_a.get_grade.clone(),
            put_grade: sum_a.put_grade.clone(),
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let mut resolved = std::collections::HashMap::new();
        resolved.insert("seq".into(), 2);
        let g = CgirGraph {
            nodes: vec![
                mk_leaf(0, "A", "Entities", "f32", Arc::clone(&sum_a)),
                mk_leaf(1, "B", "f32", "f32", Arc::clone(&sum_b)),
                CgirNode::Compose {
                    id: 2,
                    lhs: 0,
                    rhs: 1,
                    grade: sum_a.get_grade.clone(),
                    provenance: Span::dummy(),
                },
                CgirNode::QueryMap {
                    id: 3,
                    optic_name: "seq".into(),
                    costate: "entities".into(),
                    cursor: "c".into(),
                    map_param: "x".into(),
                    map_body: Arc::new(HirExpr::Var("x".into(), Span::dummy())),
                    provenance: Span::dummy(),
                },
            ],
            roots: vec![3],
            provenance_index: BTreeMap::new(),
            resolved_optics: resolved,
            region_map: hir::RegionMap::default(),
        };
        let out = optimize(g).expect("compose fusion should verify");
        assert_eq!(out.graph.roots, vec![4]);
        let fused = out.graph.nodes.iter().find_map(|n| {
            if let CgirNode::FusedLoop { compose_body, .. } = n {
                compose_body.as_ref()
            } else {
                None
            }
        });
        assert!(fused.is_some(), "compose fusion must materialize body");
        assert!(out
            .graph
            .provenance_index
            .values()
            .any(|p| { p.reason == FusionReason::ComposeFusion }));
    }

    #[test]
    fn compose_fusion_blocked_on_impurity() {
        let sum_rhs = Arc::new(hir::OpticSummary {
            name: Some("B".into()),
            costate: "f32".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec![],
            put_reads: vec![],
            put_writes: vec![],
            get_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Opaque,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let sum_lhs = Arc::new(hir::OpticSummary {
            name: Some("A".into()),
            costate: "Entities".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: sum_rhs.get_grade.clone(),
            put_grade: sum_rhs.put_grade.clone(),
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let mut resolved = std::collections::HashMap::new();
        resolved.insert("seq".into(), 2);
        let g = CgirGraph {
            nodes: vec![
                mk_leaf(0, "A", "Entities", "f32", Arc::clone(&sum_lhs)),
                mk_leaf(1, "B", "f32", "f32", Arc::clone(&sum_rhs)),
                CgirNode::Compose {
                    id: 2,
                    lhs: 0,
                    rhs: 1,
                    grade: sum_lhs.get_grade.clone(),
                    provenance: Span::dummy(),
                },
                CgirNode::QueryMap {
                    id: 3,
                    optic_name: "seq".into(),
                    costate: "entities".into(),
                    cursor: "c".into(),
                    map_param: "x".into(),
                    map_body: Arc::new(HirExpr::Var("x".into(), Span::dummy())),
                    provenance: Span::dummy(),
                },
            ],
            roots: vec![3],
            provenance_index: BTreeMap::new(),
            resolved_optics: resolved,
            region_map: hir::RegionMap::default(),
        };
        let out = optimize(g).expect("optimize should still verify unfused compose");
        assert_eq!(out.graph.roots, vec![3]);
        assert!(
            out.fusion_notes
                .iter()
                .any(|d| d.code == optic_diagnostics::FUS_COMPOSE_LEGALITY_BLOCKED),
            "impurity must emit FUS-502"
        );
    }

    #[test]
    fn compose_fusion_blocked_on_intermediate_escape() {
        let sum_rhs = Arc::new(hir::OpticSummary {
            name: Some("B".into()),
            costate: "f32".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec![],
            put_reads: vec![],
            put_writes: vec![],
            get_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let sum_lhs = Arc::new(hir::OpticSummary {
            name: Some("A".into()),
            costate: "Entities".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: sum_rhs.get_grade.clone(),
            put_grade: sum_rhs.put_grade.clone(),
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let mut resolved = std::collections::HashMap::new();
        resolved.insert("seq".into(), 2);
        let mut b_leaf = mk_leaf(1, "B", "f32", "f32", Arc::clone(&sum_rhs));
        if let CgirNode::OpticLeaf { get_param, .. } = &mut b_leaf {
            *get_param = "x".into();
        }
        let g = CgirGraph {
            nodes: vec![
                mk_leaf(0, "A", "Entities", "f32", Arc::clone(&sum_lhs)),
                b_leaf,
                CgirNode::Compose {
                    id: 2,
                    lhs: 0,
                    rhs: 1,
                    grade: sum_lhs.get_grade.clone(),
                    provenance: Span::dummy(),
                },
                CgirNode::QueryMap {
                    id: 3,
                    optic_name: "seq".into(),
                    costate: "entities".into(),
                    cursor: "c".into(),
                    map_param: "y".into(),
                    map_body: Arc::new(HirExpr::Bin {
                        op: BinOp::Add,
                        left: Box::new(HirExpr::Var("y".into(), Span::dummy())),
                        right: Box::new(HirExpr::Var("x".into(), Span::dummy())),
                        span: Span::dummy(),
                    }),
                    provenance: Span::dummy(),
                },
            ],
            roots: vec![3],
            provenance_index: BTreeMap::new(),
            resolved_optics: resolved,
            region_map: hir::RegionMap::default(),
        };
        let out = optimize(g).expect("optimize should still verify unfused compose");
        assert_eq!(out.graph.roots, vec![3]);
        assert!(
            out.fusion_notes
                .iter()
                .any(|d| d.code == optic_diagnostics::FUS_COMPOSE_BLOCKED),
            "intermediate escape must emit FUS-501"
        );
    }

    #[test]
    fn compose_types_compatible_uses_summary_not_display_strings() {
        let sum_a = Arc::new(hir::OpticSummary {
            name: Some("A".into()),
            costate: "Entities".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let sum_b = Arc::new(hir::OpticSummary {
            name: Some("B".into()),
            costate: "f32".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec![],
            put_reads: vec![],
            put_writes: vec![],
            get_grade: sum_a.get_grade.clone(),
            put_grade: sum_a.put_grade.clone(),
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let mut lhs = mk_leaf(0, "A", "Entities", "FLOAT", Arc::clone(&sum_a));
        let mut rhs = mk_leaf(1, "B", "f32_alias", "f32", Arc::clone(&sum_b));
        if let CgirNode::OpticLeaf { focus, costate, .. } = &mut lhs {
            *focus = "FLOAT".into();
            *costate = "Entities".into();
        }
        if let CgirNode::OpticLeaf { costate, .. } = &mut rhs {
            *costate = "f32_alias".into();
        }
        assert!(types_compatible(
            lhs_summary(&lhs).expect("lhs summary"),
            lhs_summary(&rhs).expect("rhs summary")
        ));
        let mut bad_rhs_sum = (*sum_b).clone();
        bad_rhs_sum.costate = "Entities".into();
        let bad_rhs = mk_leaf(2, "C", "Entities", "f32", Arc::new(bad_rhs_sum));
        assert!(!types_compatible(
            lhs_summary(&lhs).expect("lhs summary"),
            lhs_summary(&bad_rhs).expect("bad summary")
        ));
    }

    fn lhs_summary(n: &CgirNode) -> Option<&hir::OpticSummary> {
        leaf_summary(n)
    }

    #[test]
    fn compose_fusion_blocked_on_focus_mismatch() {
        let sum = Arc::new(hir::OpticSummary {
            name: Some("A".into()),
            costate: "E".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let mut resolved = std::collections::HashMap::new();
        resolved.insert("seq".into(), 2);
        let g = CgirGraph {
            nodes: vec![
                mk_leaf(0, "A", "Entities", "f32", Arc::clone(&sum)),
                mk_leaf(1, "B", "Entities", "f32", Arc::clone(&sum)),
                CgirNode::Compose {
                    id: 2,
                    lhs: 0,
                    rhs: 1,
                    grade: sum.get_grade.clone(),
                    provenance: Span::dummy(),
                },
                CgirNode::QueryMap {
                    id: 3,
                    optic_name: "seq".into(),
                    costate: "entities".into(),
                    cursor: "c".into(),
                    map_param: "x".into(),
                    map_body: Arc::new(HirExpr::Var("x".into(), Span::dummy())),
                    provenance: Span::dummy(),
                },
            ],
            roots: vec![3],
            provenance_index: BTreeMap::new(),
            resolved_optics: resolved,
            region_map: hir::RegionMap::default(),
        };
        let err = optimize(g).expect_err("invalid compose wiring must fail verify");
        assert!(
            err.rule.contains("compose type wiring invalid"),
            "focus mismatch caught by CGIR verify: {}",
            err.rule
        );
    }

    #[test]
    fn product_flatten_materializes_product_flat_children() {
        let sum = Arc::new(hir::OpticSummary {
            name: Some("H".into()),
            costate: "E".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let mut resolved = std::collections::HashMap::new();
        resolved.insert("par".into(), 2);
        let g = CgirGraph {
            nodes: vec![
                mk_leaf(0, "H", "E", "f32", Arc::clone(&sum)),
                mk_leaf(1, "H2", "E", "f32", Arc::clone(&sum)),
                CgirNode::Product {
                    id: 2,
                    lhs: 0,
                    rhs: 1,
                    grade: sum.get_grade.clone(),
                    alias_safe: true,
                    provenance: Span::dummy(),
                },
                CgirNode::QueryMap {
                    id: 3,
                    optic_name: "par".into(),
                    costate: "entities".into(),
                    cursor: "c".into(),
                    map_param: "h,p".into(),
                    map_body: Arc::new(HirExpr::Tuple(
                        vec![
                            HirExpr::Var("h".into(), Span::dummy()),
                            HirExpr::Var("p".into(), Span::dummy()),
                        ],
                        Span::dummy(),
                    )),
                    provenance: Span::dummy(),
                },
            ],
            roots: vec![3],
            provenance_index: BTreeMap::new(),
            resolved_optics: resolved,
            region_map: hir::RegionMap::default(),
        };
        let out = optimize(g).expect("product flatten should verify");
        let flat = out
            .graph
            .nodes
            .iter()
            .find_map(|n| {
                if let CgirNode::ProductFlat { children, .. } = n {
                    Some(children.clone())
                } else {
                    None
                }
            })
            .expect("ProductFlat node");
        assert_eq!(flat, vec![0, 1]);
        assert!(out
            .graph
            .provenance_index
            .values()
            .any(|p| p.reason == FusionReason::ProductFlattening));
        debug_assert!(
            verify_to_diagnostic(&out.graph).is_ok(),
            "post ProductFlat positive"
        );
    }

    #[test]
    fn orphan_query_map_fails_verify() {
        let body = HirExpr::Var("x".into(), Span::dummy());
        let mut resolved = std::collections::HashMap::new();
        resolved.insert("H".into(), 0);
        let g = CgirGraph {
            nodes: vec![
                mk_leaf(
                    0,
                    "H",
                    "E",
                    "f32",
                    Arc::new(hir::OpticSummary {
                        name: Some("H".into()),
                        costate: "E".into(),
                        focus: "f32".into(),
                        lift: hir::PathLift::default(),
                        get_reads: vec!["healths".into()],
                        put_reads: vec![],
                        put_writes: vec!["healths".into()],
                        get_grade: hir::ConcreteGrade {
                            cache: 1,
                            ownership: OwnershipDim {
                                share: Rational::one(),
                                read_only: false,
                                must_use: false,
                            },
                        },
                        put_grade: hir::ConcreteGrade {
                            cache: 1,
                            ownership: OwnershipDim {
                                share: Rational::one(),
                                read_only: false,
                                must_use: false,
                            },
                        },
                        get_determinism: hir::Determinism::Pure,
                        put_determinism: hir::Determinism::Pure,
                        serializable: true,
                        provenance: Span::dummy(),
                    }),
                ),
                CgirNode::QueryMap {
                    id: 1,
                    optic_name: "H".into(),
                    costate: "entities".into(),
                    cursor: "c".into(),
                    map_param: "x".into(),
                    map_body: Arc::new(body.clone()),
                    provenance: Span::dummy(),
                },
                CgirNode::QueryMap {
                    id: 2,
                    optic_name: "H".into(),
                    costate: "entities".into(),
                    cursor: "c".into(),
                    map_param: "y".into(),
                    map_body: Arc::new(body),
                    provenance: Span::dummy(),
                },
            ],
            roots: vec![1],
            provenance_index: BTreeMap::new(),
            resolved_optics: resolved,
            region_map: hir::RegionMap::default(),
        };
        assert!(verify(&g).is_err(), "orphan QueryMap must fail verify");
    }

    #[test]
    fn flatten_product_children_rejects_invalid_child_kind() {
        let g = CgirGraph {
            nodes: vec![
                CgirNode::Product {
                    id: 0,
                    lhs: 1,
                    rhs: 2,
                    grade: hir::ConcreteGrade {
                        cache: 1,
                        ownership: OwnershipDim {
                            share: Rational::one(),
                            read_only: false,
                            must_use: false,
                        },
                    },
                    alias_safe: true,
                    provenance: Span::dummy(),
                },
                CgirNode::QueryMap {
                    id: 1,
                    optic_name: "H".into(),
                    costate: "e".into(),
                    cursor: "c".into(),
                    map_param: "x".into(),
                    map_body: Arc::new(HirExpr::Var("x".into(), Span::dummy())),
                    provenance: Span::dummy(),
                },
                mk_leaf(
                    2,
                    "H",
                    "E",
                    "f32",
                    Arc::new(hir::OpticSummary {
                        name: Some("H".into()),
                        costate: "E".into(),
                        focus: "f32".into(),
                        lift: hir::PathLift::default(),
                        get_reads: vec!["healths".into()],
                        put_reads: vec![],
                        put_writes: vec!["healths".into()],
                        get_grade: hir::ConcreteGrade {
                            cache: 1,
                            ownership: OwnershipDim {
                                share: Rational::one(),
                                read_only: false,
                                must_use: false,
                            },
                        },
                        put_grade: hir::ConcreteGrade {
                            cache: 1,
                            ownership: OwnershipDim {
                                share: Rational::one(),
                                read_only: false,
                                must_use: false,
                            },
                        },
                        get_determinism: hir::Determinism::Pure,
                        put_determinism: hir::Determinism::Pure,
                        serializable: true,
                        provenance: Span::dummy(),
                    }),
                ),
            ],
            roots: vec![0],
            provenance_index: BTreeMap::new(),
            resolved_optics: empty_resolved(),
            region_map: hir::RegionMap::default(),
        };
        let err = flatten_product_children(&g, 0).expect_err("invalid child");
        assert!(err.contains("not Product/ProductFlat/OpticLeaf"));
    }

    #[test]
    fn flatten_product_children_collects_three_leaf_chain() {
        let sum = Arc::new(hir::OpticSummary {
            name: Some("L".into()),
            costate: "E".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let g = CgirGraph {
            nodes: vec![
                mk_leaf(0, "A", "E", "f32", Arc::clone(&sum)),
                mk_leaf(1, "B", "E", "f32", Arc::clone(&sum)),
                mk_leaf(2, "C", "E", "f32", Arc::clone(&sum)),
                CgirNode::Product {
                    id: 3,
                    lhs: 0,
                    rhs: 1,
                    grade: sum.get_grade.clone(),
                    alias_safe: true,
                    provenance: Span::dummy(),
                },
                CgirNode::Product {
                    id: 4,
                    lhs: 3,
                    rhs: 2,
                    grade: sum.get_grade.clone(),
                    alias_safe: true,
                    provenance: Span::dummy(),
                },
            ],
            roots: vec![4],
            provenance_index: BTreeMap::new(),
            resolved_optics: empty_resolved(),
            region_map: hir::RegionMap::default(),
        };
        let children = flatten_product_children(&g, 4).expect("three leaves");
        assert_eq!(children, vec![0, 1, 2]);
    }

    #[test]
    fn product_flatten_records_note_on_invalid_child() {
        let sum_a = Arc::new(hir::OpticSummary {
            name: Some("A".into()),
            costate: "E".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let sum_b = Arc::new(hir::OpticSummary {
            name: Some("B".into()),
            costate: "f32".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: sum_a.get_grade.clone(),
            put_grade: sum_a.put_grade.clone(),
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let g = CgirGraph {
            nodes: vec![
                mk_leaf(0, "A", "E", "f32", Arc::clone(&sum_a)),
                mk_leaf(1, "B", "E", "f32", Arc::clone(&sum_a)),
                mk_leaf(2, "C", "f32", "f32", Arc::clone(&sum_b)),
                mk_leaf(3, "D", "f32", "f32", sum_b),
                CgirNode::Product {
                    id: 4,
                    lhs: 0,
                    rhs: 1,
                    grade: sum_a.get_grade.clone(),
                    alias_safe: true,
                    provenance: Span::dummy(),
                },
                CgirNode::Compose {
                    id: 5,
                    lhs: 2,
                    rhs: 3,
                    grade: sum_a.get_grade.clone(),
                    provenance: Span::dummy(),
                },
                CgirNode::Product {
                    id: 6,
                    lhs: 4,
                    rhs: 5,
                    grade: sum_a.get_grade.clone(),
                    alias_safe: true,
                    provenance: Span::dummy(),
                },
            ],
            roots: vec![6],
            provenance_index: BTreeMap::new(),
            resolved_optics: empty_resolved(),
            region_map: hir::RegionMap::default(),
        };
        let out =
            optimize_without_compose_fusion_reporting(g).expect("optimize with skipped flatten");
        assert!(
            out.fusion_notes
                .iter()
                .any(|n| n.rule.contains("product flatten skipped")),
            "expected fusion note on flatten skip, got {:?}",
            out.fusion_notes
        );
        assert!(matches!(out.graph.nodes[6], CgirNode::Product { .. }));
    }

    #[test]
    fn substitute_map_body_agrees_with_hir_var_without_focus_field() {
        let e = HirExpr::Bin {
            op: BinOp::Add,
            left: Box::new(HirExpr::Var("x".into(), Span::dummy())),
            right: Box::new(HirExpr::CursorIndex {
                cursor: "x".into(),
                field: "healths".into(),
                span: Span::dummy(),
            }),
            span: Span::dummy(),
        };
        let repl = HirExpr::Var("_bind".into(), Span::dummy());
        let map_sub = substitute_map_body(&e, "x", &repl);
        let hir_sub = hir::substitute_hir_var(&e, "x", &repl);
        assert_eq!(format!("{map_sub:?}"), format!("{hir_sub:?}"));
    }

    #[test]
    fn substitute_map_body_replaces_focus_field_whole_node() {
        let e = HirExpr::FocusField {
            param: "x".into(),
            path: vec!["healths".into()],
            span: Span::dummy(),
        };
        let repl = HirExpr::Var("_bind".into(), Span::dummy());
        let map_sub = substitute_map_body(&e, "x", &repl);
        assert!(matches!(map_sub, HirExpr::Var(ref v, _) if v == "_bind"));
        let hir_sub = hir::substitute_hir_var(&e, "x", &repl);
        assert!(matches!(hir_sub, HirExpr::FocusField { .. }));
    }

    fn mk_traversal(
        id: NodeId,
        name: &str,
        costate: &str,
        focus: &str,
        sum: Arc<hir::OpticSummary>,
    ) -> CgirNode {
        CgirNode::TraversalLeaf {
            id,
            name: name.into(),
            costate: costate.into(),
            focus: focus.into(),
            grade: sum.get_grade.clone(),
            get_fn: String::new(),
            set_fn: String::new(),
            get_param: "s".into(),
            get_body: Arc::new(HirExpr::CursorIndex {
                cursor: "cursor".into(),
                field: "healths".into(),
                span: Span::dummy(),
            }),
            set_state_param: Some("s".into()),
            set_value_param: Some("v".into()),
            set_value_body: Some(Arc::new(HirExpr::Var("v".into(), Span::dummy()))),
            summary: sum,
            provenance: Span::dummy(),
            m7_reserved: false,
        }
    }

    fn mk_prism(
        id: NodeId,
        name: &str,
        costate: &str,
        focus: &str,
        sum: Arc<hir::OpticSummary>,
    ) -> CgirNode {
        CgirNode::PrismLeaf {
            id,
            name: name.into(),
            costate: costate.into(),
            focus: focus.into(),
            grade: sum.get_grade.clone(),
            preview_fn: String::new(),
            review_fn: String::new(),
            preview_param: "s".into(),
            preview_body: Arc::new(HirExpr::CursorIndex {
                cursor: "cursor".into(),
                field: "healths".into(),
                span: Span::dummy(),
            }),
            preview_returns_option: false,
            preview_wrap_some: false,
            review_state_param: Some("s".into()),
            review_value_param: Some("a".into()),
            review_value_body: Some(Arc::new(HirExpr::Var("a".into(), Span::dummy()))),
            summary: sum,
            provenance: Span::dummy(),
            m7_reserved: false,
        }
    }

    #[test]
    fn prism_query_map_unchanged_by_compose_fusion() {
        let sum = Arc::new(hir::OpticSummary {
            name: Some("AliveFilter".into()),
            costate: "Entities".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: hir::ConcreteGrade {
                cache: 2,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 2,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let mut resolved = std::collections::HashMap::new();
        resolved.insert("AliveFilter".into(), 0);
        let g = CgirGraph {
            nodes: vec![
                mk_prism(0, "AliveFilter", "Entities", "f32", Arc::clone(&sum)),
                CgirNode::QueryMap {
                    id: 1,
                    optic_name: "AliveFilter".into(),
                    costate: "entities".into(),
                    cursor: "c".into(),
                    map_param: "h".into(),
                    map_body: Arc::new(HirExpr::Var("h".into(), Span::dummy())),
                    provenance: Span::dummy(),
                },
            ],
            roots: vec![1],
            provenance_index: BTreeMap::new(),
            resolved_optics: resolved,
            region_map: hir::RegionMap::default(),
        };
        let out = optimize(g).expect("prism map should optimize");
        assert_eq!(out.graph.roots, vec![1]);
        assert!(out.fusion_notes.is_empty());
    }

    #[test]
    fn compose_fusion_blocked_on_prism_in_compose() {
        let sum_lhs = Arc::new(hir::OpticSummary {
            name: Some("H".into()),
            costate: "Entities".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: hir::ConcreteGrade {
                cache: 2,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 2,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let sum_prism = Arc::new(hir::OpticSummary {
            name: Some("P".into()),
            costate: "f32".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: sum_lhs.get_grade.clone(),
            put_grade: sum_lhs.put_grade.clone(),
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let mut resolved = std::collections::HashMap::new();
        resolved.insert("seq".into(), 2);
        let g = CgirGraph {
            nodes: vec![
                mk_leaf(0, "H", "Entities", "f32", Arc::clone(&sum_lhs)),
                mk_prism(1, "P", "f32", "f32", Arc::clone(&sum_prism)),
                CgirNode::Compose {
                    id: 2,
                    lhs: 0,
                    rhs: 1,
                    grade: sum_lhs.get_grade.clone(),
                    provenance: Span::dummy(),
                },
                CgirNode::QueryMap {
                    id: 3,
                    optic_name: "seq".into(),
                    costate: "entities".into(),
                    cursor: "c".into(),
                    map_param: "h".into(),
                    map_body: Arc::new(HirExpr::Var("h".into(), Span::dummy())),
                    provenance: Span::dummy(),
                },
            ],
            roots: vec![3],
            provenance_index: BTreeMap::new(),
            resolved_optics: resolved,
            region_map: hir::RegionMap::default(),
        };
        let out = optimize(g).expect("compose+prism should still verify unfused");
        assert_eq!(out.graph.roots, vec![3]);
        assert!(
            out.fusion_notes.iter().any(|d| {
                d.code == optic_diagnostics::FUS_COMPOSE_LEGALITY_BLOCKED
                    && d.evidence["reason"] == "prism_in_compose"
            }),
            "compose+prism must emit FUS-502 prism_in_compose"
        );
    }

    #[test]
    fn compose_fusion_blocked_on_traversal_in_compose() {
        let sum_lhs = Arc::new(hir::OpticSummary {
            name: Some("H".into()),
            costate: "Entities".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: hir::ConcreteGrade {
                cache: 2,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 2,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let sum_traversal = Arc::new(hir::OpticSummary {
            name: Some("T".into()),
            costate: "f32".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: sum_lhs.get_grade.clone(),
            put_grade: sum_lhs.put_grade.clone(),
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let mut resolved = std::collections::HashMap::new();
        resolved.insert("seq".into(), 2);
        let g = CgirGraph {
            nodes: vec![
                mk_leaf(0, "H", "Entities", "f32", Arc::clone(&sum_lhs)),
                mk_traversal(1, "T", "f32", "f32", Arc::clone(&sum_traversal)),
                CgirNode::Compose {
                    id: 2,
                    lhs: 0,
                    rhs: 1,
                    grade: sum_lhs.get_grade.clone(),
                    provenance: Span::dummy(),
                },
                CgirNode::QueryMap {
                    id: 3,
                    optic_name: "seq".into(),
                    costate: "entities".into(),
                    cursor: "c".into(),
                    map_param: "h".into(),
                    map_body: Arc::new(HirExpr::Var("h".into(), Span::dummy())),
                    provenance: Span::dummy(),
                },
            ],
            roots: vec![3],
            provenance_index: BTreeMap::new(),
            resolved_optics: resolved,
            region_map: hir::RegionMap::default(),
        };
        let out = optimize(g).expect("compose+traversal should still verify unfused");
        assert_eq!(out.graph.roots, vec![3]);
        assert!(
            out.fusion_notes.iter().any(|d| {
                d.code == optic_diagnostics::FUS_COMPOSE_LEGALITY_BLOCKED
                    && d.evidence["reason"] == "traversal_in_compose"
            }),
            "compose+traversal must emit FUS-502 traversal_in_compose"
        );
    }

    #[test]
    fn flatten_product_children_accepts_prism_leaf() {
        let sum = Arc::new(hir::OpticSummary {
            name: Some("P".into()),
            costate: "Entities".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let g = CgirGraph {
            nodes: vec![
                mk_prism(0, "P", "Entities", "f32", Arc::clone(&sum)),
                mk_leaf(1, "H", "Entities", "f32", sum),
                CgirNode::Product {
                    id: 2,
                    lhs: 0,
                    rhs: 1,
                    grade: hir::ConcreteGrade {
                        cache: 1,
                        ownership: OwnershipDim {
                            share: Rational::one(),
                            read_only: false,
                            must_use: false,
                        },
                    },
                    alias_safe: true,
                    provenance: Span::dummy(),
                },
            ],
            roots: vec![2],
            provenance_index: BTreeMap::new(),
            resolved_optics: empty_resolved(),
            region_map: hir::RegionMap::default(),
        };
        let children = flatten_product_children(&g, 2).expect("prism in product");
        assert_eq!(children, vec![0, 1]);
        debug_assert!(true, "M7 prism leaf positive construction explicit here + ProductFlat support in verify (R5)");
    }

    #[test]
    fn flatten_product_children_accepts_traversal_leaf() {
        let sum = Arc::new(hir::OpticSummary {
            name: Some("T".into()),
            costate: "Entities".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec!["healths".into()],
            get_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: OwnershipDim {
                    share: Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let g = CgirGraph {
            nodes: vec![
                mk_traversal(0, "T", "Entities", "f32", Arc::clone(&sum)),
                mk_leaf(1, "H", "Entities", "f32", sum),
                CgirNode::Product {
                    id: 2,
                    lhs: 0,
                    rhs: 1,
                    grade: hir::ConcreteGrade {
                        cache: 1,
                        ownership: OwnershipDim {
                            share: Rational::one(),
                            read_only: false,
                            must_use: false,
                        },
                    },
                    alias_safe: true,
                    provenance: Span::dummy(),
                },
            ],
            roots: vec![2],
            provenance_index: BTreeMap::new(),
            resolved_optics: empty_resolved(),
            region_map: hir::RegionMap::default(),
        };
        let children = flatten_product_children(&g, 2).expect("traversal in product");
        assert_eq!(children, vec![0, 1]);
    }
}
