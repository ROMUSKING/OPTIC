//! optic-cgir — CGIR, provenance, verifier (ch. 10, M3).
//! Minimal but sufficient for narrow v0 examples: build from typed HIR, basic nodes, provenance.

use optic_hir as hir;
use optic_syntax::Span;
use std::sync::Arc;

pub type NodeId = u32;

#[derive(Clone, Debug)]
pub struct CgirGraph {
    pub nodes: Vec<CgirNode>,
    pub roots: Vec<NodeId>,
    pub provenance_index: std::collections::BTreeMap<NodeId, FusionProvenance>,
    /// Query-resolvable optic names -> CGIR node id (leaves, products, composes, aliases).
    pub resolved_optics: std::collections::HashMap<String, NodeId>,
    /// Region→column/type map from HIR data declarations (SUG-003).
    pub region_map: hir::RegionMap,
}

#[derive(Clone, Debug)]
pub struct FusionProvenance {
    pub original_ids: Vec<NodeId>,
    pub spans: Vec<Span>,
    pub reason: FusionReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FusionReason {
    /// Pre-fusion graph node from CGIR build (not a rewrite).
    Build,
    MapFusion,
    ComposeFusion,
    ProductFlattening,
}

/// Materialized compose+map loop body (ch10 compose fusion).
#[derive(Clone, Debug)]
pub struct ComposeFusedBody {
    pub compose_id: NodeId,
    pub lhs: NodeId,
    pub rhs: NodeId,
    pub map_param: String,
    pub map_body: Arc<hir::HirExpr>,
}

#[derive(Clone, Debug)]
pub enum CgirNode {
    OpticLeaf {
        id: NodeId,
        name: String,
        costate: String,
        focus: String,
        grade: hir::ConcreteGrade,
        get_fn: String, /* source repr */
        put_fn: String,
        get_param: String,
        get_body: Arc<hir::HirExpr>,
        put_state_param: Option<String>,
        put_value_param: Option<String>,
        put_value_body: Option<Arc<hir::HirExpr>>,
        summary: Arc<hir::OpticSummary>,
        provenance: Span,
    },
    Compose {
        id: NodeId,
        lhs: NodeId,
        rhs: NodeId,
        grade: hir::ConcreteGrade,
        provenance: Span,
    },
    Product {
        id: NodeId,
        lhs: NodeId,
        rhs: NodeId,
        grade: hir::ConcreteGrade,
        alias_safe: bool,
        provenance: Span,
    },
    /// Flattened product chain for codegen (ch10 product_flatten).
    ProductFlat {
        id: NodeId,
        children: Vec<NodeId>,
        grade: hir::ConcreteGrade,
        alias_safe: bool,
        provenance: Span,
    },
    // optic_name instead of optic:NodeId to avoid dangling ids for query refs to lets/composites (per ch10.9)
    QueryGet {
        id: NodeId,
        optic_name: String,
        costate: String,
        cursor: String,
        provenance: Span,
    },
    QuerySet {
        id: NodeId,
        optic_name: String,
        costate: String,
        cursor: String,
        value_repr: String,
        provenance: Span,
    },
    QueryMap {
        id: NodeId,
        optic_name: String,
        costate: String,
        cursor: String,
        map_param: String,
        map_body: std::sync::Arc<hir::HirExpr>,
        provenance: Span,
    },
    FusedLoop {
        id: NodeId,
        original_ids: Vec<NodeId>,
        costate: String,
        provenance: Span,
        /// Present when compose fusion materializes a single-loop body (ch10).
        compose_body: Option<ComposeFusedBody>,
    },
    // reserved for later
}

/// Lower `A >>> B >>> C` to a nested Compose tree (ch8/ch10).
fn build_seq_chain(
    optic: &hir::HirOptic,
    nodes: &mut Vec<CgirNode>,
    optic_leaf_ids: &std::collections::HashMap<String, NodeId>,
    id: &mut NodeId,
    prov: &mut std::collections::BTreeMap<NodeId, FusionProvenance>,
) -> Option<NodeId> {
    match optic {
        hir::HirOptic::Named { name, .. } => optic_leaf_ids.get(name).copied(),
        hir::HirOptic::Seq { lhs, rhs, span } => {
            let lid = build_seq_chain(lhs, nodes, optic_leaf_ids, id, prov)?;
            let rid = build_seq_chain(rhs, nodes, optic_leaf_ids, id, prov)?;
            let cid = *id;
            *id += 1;
            nodes.push(CgirNode::Compose {
                id: cid,
                lhs: lid,
                rhs: rid,
                grade: nodes
                    .get(lid as usize)
                    .map_or_else(default_grade_v0, |n| n.node_grade()),
                provenance: *span,
            });
            prov.insert(
                cid,
                FusionProvenance {
                    original_ids: vec![lid, rid, cid],
                    spans: vec![*span],
                    reason: FusionReason::Build,
                },
            );
            Some(cid)
        }
        _ => None,
    }
}

fn compose_emit_focus_summary<'a>(g: &'a CgirGraph, id: NodeId) -> Option<&'a str> {
    match g.nodes.get(id as usize)? {
        CgirNode::OpticLeaf { summary, .. } => Some(summary.focus.as_str()),
        CgirNode::Compose { rhs, .. } => compose_emit_focus_summary(g, *rhs),
        _ => None,
    }
}

fn compose_recv_costate_summary<'a>(g: &'a CgirGraph, id: NodeId) -> Option<&'a str> {
    match g.nodes.get(id as usize)? {
        CgirNode::OpticLeaf { summary, .. } => Some(summary.costate.as_str()),
        CgirNode::Compose { lhs, .. } => compose_recv_costate_summary(g, *lhs),
        _ => None,
    }
}

fn compose_types_compatible(g: &CgirGraph, lhs: NodeId, rhs: NodeId) -> bool {
    match (
        compose_emit_focus_summary(g, lhs),
        compose_recv_costate_summary(g, rhs),
    ) {
        (Some(lf), Some(rc)) => lf.eq_ignore_ascii_case(rc),
        _ => false,
    }
}

fn hir_expr_unsupported(e: &hir::HirExpr) -> Option<(&Span, String)> {
    match e {
        hir::HirExpr::Unsupported { reason, span } => Some((span, reason.clone())),
        _ => None,
    }
}

fn compose_chain_unsupported_body(
    g: &CgirGraph,
    compose_id: NodeId,
) -> Option<(Span, String, NodeId)> {
    let chain = compose_leaf_chain(g, compose_id)?;
    for &lid in &chain {
        let leaf = g.nodes.get(lid as usize)?;
        if let CgirNode::OpticLeaf {
            name,
            get_body,
            put_value_body,
            ..
        } = leaf
        {
            if let Some((span, reason)) = hir_expr_unsupported(get_body) {
                return Some((*span, format!("optic `{name}` get body: {reason}"), lid));
            }
            if let Some(body) = put_value_body {
                if let Some((span, reason)) = hir_expr_unsupported(body) {
                    return Some((*span, format!("optic `{name}` put body: {reason}"), lid));
                }
            }
        } else {
            return Some((
                Span::dummy(),
                "compose chain must be optic leaves".into(),
                lid,
            ));
        }
    }
    None
}

/// Resolve a node by its `NodeId` field (not vector index).
pub fn find_node_by_id<'a>(g: &'a CgirGraph, id: NodeId) -> Option<&'a CgirNode> {
    g.nodes.iter().find(|n| node_id(n) == id)
}

/// Left-to-right leaf spine of a (possibly nested) Compose tree.
pub fn compose_leaf_chain(g: &CgirGraph, id: NodeId) -> Option<Vec<NodeId>> {
    match g.nodes.get(id as usize)? {
        CgirNode::OpticLeaf { .. } => Some(vec![id]),
        CgirNode::Compose { lhs, rhs, .. } => {
            let mut chain = compose_leaf_chain(g, *lhs)?;
            chain.extend(compose_leaf_chain(g, *rhs)?);
            Some(chain)
        }
        _ => None,
    }
}

/// Entry (leftmost) leaf of a compose tree.
pub fn compose_entry_leaf(g: &CgirGraph, id: NodeId) -> Option<NodeId> {
    compose_leaf_chain(g, id).and_then(|c| c.first().copied())
}

/// Exit (rightmost) leaf of a compose tree.
pub fn compose_exit_leaf(g: &CgirGraph, id: NodeId) -> Option<NodeId> {
    compose_leaf_chain(g, id).and_then(|c| c.last().copied())
}

fn compose_subtree_ids(g: &CgirGraph, id: NodeId, out: &mut Vec<NodeId>) {
    match g.nodes.get(id as usize) {
        Some(CgirNode::OpticLeaf { .. }) => out.push(id),
        Some(CgirNode::Compose { lhs, rhs, .. }) => {
            compose_subtree_ids(g, *lhs, out);
            compose_subtree_ids(g, *rhs, out);
            out.push(id);
        }
        _ => {}
    }
}

pub fn compose_tree_node_ids(g: &CgirGraph, compose_id: NodeId) -> Vec<NodeId> {
    let mut ids = vec![];
    compose_subtree_ids(g, compose_id, &mut ids);
    ids
}

pub fn build(
    typed: &optic_typeck::TypedHir,
) -> Result<CgirGraph, Vec<optic_diagnostics::Diagnostic>> {
    let mut nodes = vec![];
    let mut roots = vec![];
    let mut prov = std::collections::BTreeMap::new();
    let mut id: NodeId = 0;
    let mut optic_leaf_ids: std::collections::HashMap<String, NodeId> =
        std::collections::HashMap::new();
    let mut resolved_optics: std::collections::HashMap<String, NodeId> =
        std::collections::HashMap::new();
    let mut query_count = 0usize;
    let region_map = match hir::build_region_map(&hir::HirProgram {
        items: typed.items.clone(),
    }) {
        Ok(m) => m,
        Err(rule) => {
            return Err(vec![optic_diagnostics::cgir_diag(
                "CGI-001",
                Span::dummy(),
                &rule,
                serde_json::json!({}),
            )]);
        }
    };

    for item in &typed.items {
        match item {
            hir::HirItem::Optic { decl, summary } => {
                let nid = id;
                id += 1;
                // real from summary per ch10.9 (OpticLeaf from named optic + summary)
                let get_fn = if summary.get_reads.is_empty() {
                    "cursor.arena".into()
                } else {
                    summary
                        .get_reads
                        .iter()
                        .map(|r| format!("cursor.arena.{}[cursor.id]", r))
                        .collect::<Vec<_>>()
                        .join("; ")
                };
                let put_fn = if summary.put_writes.is_empty() {
                    "".into()
                } else {
                    summary
                        .put_writes
                        .iter()
                        .map(|r| format!("cursor.arena.{}[cursor.id] = v", r))
                        .collect::<Vec<_>>()
                        .join("; ")
                };
                let costate = type_expr_name(&decl.costate);
                let focus = type_expr_name(&decl.focus);
                let get = decl.get.as_ref().expect("optic leaf requires get clause");
                let get_param = get.param.node.clone();
                let get_body = Arc::new(validate_optic_body_expr(
                    lower_optic_get_body(&get_param, &get.body),
                    &region_map,
                ));
                let (put_state_param, put_value_param, put_value_body) =
                    if let Some(put) = &decl.put {
                        (
                            Some(put.state_param.node.clone()),
                            Some(put.value_param.node.clone()),
                            Some(Arc::new(validate_optic_body_expr(
                                lower_optic_put_value_body(
                                    &put.state_param.node,
                                    &put.value_param.node,
                                    &put.body,
                                ),
                                &region_map,
                            ))),
                        )
                    } else {
                        (None, None, None)
                    };
                nodes.push(CgirNode::OpticLeaf {
                    id: nid,
                    name: decl.name.node.clone(),
                    costate,
                    focus,
                    grade: summary.get_grade.clone(),
                    get_fn,
                    put_fn,
                    get_param,
                    get_body,
                    put_state_param,
                    put_value_param,
                    put_value_body,
                    summary: Arc::clone(summary), // summary: &Arc from &typed item; cheap vs prior data clone
                    provenance: decl.span,
                });
                optic_leaf_ids.insert(decl.name.node.clone(), nid);
                resolved_optics.insert(decl.name.node.clone(), nid);
                prov.insert(
                    nid,
                    FusionProvenance {
                        original_ids: vec![nid],
                        spans: vec![decl.span],
                        reason: FusionReason::Build,
                    },
                );
            }
            hir::HirItem::Let {
                name,
                optic,
                summary: _,
                ..
            } => match optic {
                hir::HirOptic::Named { name: target, .. } => {
                    if let Some(&nid) = optic_leaf_ids.get(target) {
                        optic_leaf_ids.insert(name.clone(), nid);
                        resolved_optics.insert(name.clone(), nid);
                    }
                }
                hir::HirOptic::Par { lhs, rhs, span } => {
                    let lname = if let hir::HirOptic::Named { name, .. } = &**lhs {
                        name.clone()
                    } else {
                        "lhs".into()
                    };
                    let rname = if let hir::HirOptic::Named { name, .. } = &**rhs {
                        name.clone()
                    } else {
                        "rhs".into()
                    };
                    if let (Some(&lid), Some(&rid)) =
                        (optic_leaf_ids.get(&lname), optic_leaf_ids.get(&rname))
                    {
                        let lsum = typed
                            .summaries
                            .get(&lname)
                            .or_else(|| nodes.get(lid as usize).and_then(|n| n.summary()));
                        let rsum = typed
                            .summaries
                            .get(&rname)
                            .or_else(|| nodes.get(rid as usize).and_then(|n| n.summary()));
                        let alias_ok = match (lsum, rsum) {
                            (Some(l), Some(r)) => optic_typeck::alias_safe(l, r).is_ok(),
                            _ => false,
                        };
                        let pid = id;
                        id += 1;
                        nodes.push(CgirNode::Product {
                            id: pid,
                            lhs: lid,
                            rhs: rid,
                            grade: nodes
                                .get(lid as usize)
                                .map_or_else(default_grade_v0, |n| n.node_grade()),
                            alias_safe: alias_ok,
                            provenance: *span,
                        });
                        optic_leaf_ids.insert(name.clone(), pid);
                        resolved_optics.insert(name.clone(), pid);
                        prov.insert(
                            pid,
                            FusionProvenance {
                                original_ids: vec![lid, rid, pid],
                                spans: vec![*span],
                                reason: FusionReason::Build,
                            },
                        );
                    }
                }
                hir::HirOptic::Seq { .. } => {
                    if let Some(cid) =
                        build_seq_chain(optic, &mut nodes, &optic_leaf_ids, &mut id, &mut prov)
                    {
                        optic_leaf_ids.insert(name.clone(), cid);
                        resolved_optics.insert(name.clone(), cid);
                    }
                }
            },
            hir::HirItem::Query(q) => {
                query_count += 1;
                let optic_name = match &q.optic {
                    hir::HirOptic::Named { name, .. } => name.clone(),
                    hir::HirOptic::Par { .. } => {
                        return Err(vec![optic_diagnostics::cgir_diag(
                            optic_diagnostics::CGIR_UNRESOLVED_OPTIC,
                            q.span,
                            "inline product in query must be bound to a let name",
                            serde_json::json!({ "optic": "par-direct" }),
                        )]);
                    }
                    _ => {
                        return Err(vec![optic_diagnostics::cgir_diag(
                            optic_diagnostics::CGIR_UNRESOLVED_OPTIC,
                            q.span,
                            "unresolved optic in query",
                            serde_json::json!({}),
                        )]);
                    }
                };
                if !resolved_optics.contains_key(&optic_name)
                    && !typed.summaries.contains_key(&optic_name)
                {
                    return Err(vec![optic_diagnostics::cgir_diag(
                        optic_diagnostics::CGIR_UNRESOLVED_OPTIC,
                        q.span,
                        &format!("unresolved optic `{optic_name}` in query"),
                        serde_json::json!({ "optic": optic_name }),
                    )]);
                }
                if let Some(&nid) = resolved_optics.get(&optic_name) {
                    if matches!(nodes.get(nid as usize), Some(CgirNode::Compose { .. })) {
                        let probe = CgirGraph {
                            nodes: nodes.clone(),
                            roots: vec![],
                            provenance_index: prov.clone(),
                            resolved_optics: resolved_optics.clone(),
                            region_map: region_map.clone(),
                        };
                        if let Some((span, reason, leaf_id)) =
                            compose_chain_unsupported_body(&probe, nid)
                        {
                            return Err(vec![optic_diagnostics::cgir_diag(
                                optic_diagnostics::CGIR_UNSUPPORTED_EXPR,
                                span,
                                &format!("unsupported optic body in compose chain — {reason}"),
                                serde_json::json!({
                                    "compose_id": nid,
                                    "leaf_id": leaf_id,
                                    "reason": reason,
                                }),
                            )]);
                        }
                    }
                }
                let qid = id;
                id += 1;
                let node = match &q.kind {
                    hir::QueryKind::Get => CgirNode::QueryGet {
                        id: qid,
                        optic_name,
                        costate: q.costate.clone(),
                        cursor: q.cursor.clone(),
                        provenance: q.span,
                    },
                    hir::QueryKind::Set { value } => CgirNode::QuerySet {
                        id: qid,
                        optic_name,
                        costate: q.costate.clone(),
                        cursor: q.cursor.clone(),
                        value_repr: format_hir_expr(value),
                        provenance: q.span,
                    },
                    hir::QueryKind::Map { param, body } => CgirNode::QueryMap {
                        id: qid,
                        optic_name,
                        costate: q.costate.clone(),
                        cursor: q.cursor.clone(),
                        map_param: param.clone(),
                        map_body: std::sync::Arc::clone(body),
                        provenance: q.span,
                    },
                };
                nodes.push(node);
                roots.push(qid);
                prov.insert(
                    qid,
                    FusionProvenance {
                        original_ids: vec![qid],
                        spans: vec![q.span],
                        reason: FusionReason::Build,
                    },
                );
            }
            _ => {}
        }
    }

    if query_count > 1 {
        return Err(vec![optic_diagnostics::fusion_verify_diag(&format!(
            "v0 supports at most one query root per program (query_count={query_count})"
        ))]);
    }

    Ok(CgirGraph {
        nodes,
        roots,
        provenance_index: prov,
        resolved_optics,
        region_map,
    })
}

fn default_grade_v0() -> hir::ConcreteGrade {
    hir::ConcreteGrade {
        cache: 1,
        ownership: hir::OwnershipDim {
            share: hir::Rational::one(),
            read_only: false,
            must_use: false,
        },
    }
}

fn type_expr_name(te: &optic_syntax::TypeExpr) -> String {
    match te {
        optic_syntax::TypeExpr::Named { name, .. } => name.clone(),
        _ => "unknown".into(),
    }
}

/// Lower optic get/put surface bodies to HirExpr for fusion/codegen (ch10/ch11).
fn lower_optic_get_body(param: &str, body: &optic_syntax::Expr) -> hir::HirExpr {
    lower_optic_body_expr(param, body, false)
}

fn lower_optic_put_value_body(
    state_param: &str,
    value_param: &str,
    body: &optic_syntax::Expr,
) -> hir::HirExpr {
    if let optic_syntax::Expr::Block { stmts, result, .. } = body {
        for stmt in stmts {
            if let Some(v) = extract_assign_value(&stmt.expr, state_param, value_param) {
                return v;
            }
        }
        if let Some(r) = result {
            if let Some(v) = extract_assign_value(r, state_param, value_param) {
                return v;
            }
        }
    }
    if let Some(v) = extract_assign_value(body, state_param, value_param) {
        return v;
    }
    lower_optic_body_expr(value_param, body, true)
}

fn extract_assign_value(
    e: &optic_syntax::Expr,
    _state_param: &str,
    value_param: &str,
) -> Option<hir::HirExpr> {
    if let optic_syntax::Expr::Assign { value, .. } = e {
        Some(lower_optic_body_expr(value_param, value, true))
    } else {
        None
    }
}

fn lower_optic_body_expr(param: &str, e: &optic_syntax::Expr, is_value: bool) -> hir::HirExpr {
    match e {
        optic_syntax::Expr::Binary {
            left,
            op,
            right,
            span,
        } => hir::HirExpr::Bin {
            op: *op,
            left: Box::new(lower_optic_body_expr(param, left, is_value)),
            right: Box::new(lower_optic_body_expr(param, right, is_value)),
            span: *span,
        },
        optic_syntax::Expr::Atom(optic_syntax::AtomExpr::Int(n, sp)) => {
            hir::HirExpr::LitInt(*n, *sp)
        }
        optic_syntax::Expr::Atom(optic_syntax::AtomExpr::Float(f, sp)) => {
            hir::HirExpr::LitFloat(*f, *sp)
        }
        optic_syntax::Expr::Atom(optic_syntax::AtomExpr::Ident(id)) => {
            if id.node == param {
                hir::HirExpr::Var(param.into(), id.span)
            } else {
                hir::HirExpr::Var(id.node.clone(), id.span)
            }
        }
        optic_syntax::Expr::Atom(optic_syntax::AtomExpr::Paren(inner, sp)) => {
            hir::HirExpr::Paren(Box::new(lower_optic_body_expr(param, inner, is_value)), *sp)
        }
        optic_syntax::Expr::Field(fe) => lower_optic_field_expr(param, fe, is_value),
        optic_syntax::Expr::Block { result, .. } => result
            .as_ref()
            .map(|r| lower_optic_body_expr(param, r, is_value))
            .unwrap_or_else(|| hir::HirExpr::Unsupported {
                reason: "empty optic body block".into(),
                span: Span::dummy(),
            }),
        _ => hir::HirExpr::Unsupported {
            reason: "unsupported optic body expression".into(),
            span: Span::dummy(),
        },
    }
}

/// Reject whole-column SoA access (`s.healths`) and unindexed multi-segment paths (`s.transforms.position`).
fn validate_optic_body_expr(expr: hir::HirExpr, region_map: &hir::RegionMap) -> hir::HirExpr {
    if let hir::HirExpr::FocusField { param, path, span } = expr {
        if path.is_empty() {
            return hir::HirExpr::FocusField { param, path, span };
        }
        let first = &path[0];
        if region_map.is_top_level_column(first) {
            if path.len() == 1 {
                return hir::HirExpr::Unsupported {
                    reason: format!("unsupported whole-column access `.{}` without index", first),
                    span,
                };
            }
            return hir::HirExpr::Unsupported {
                reason: format!(
                    "unsupported unindexed nested access `{}` — index SoA column first",
                    path.join(".")
                ),
                span,
            };
        }
        hir::HirExpr::FocusField { param, path, span }
    } else {
        expr
    }
}

fn focus_field_path(param: &str, fe: &optic_syntax::FieldExpr) -> Option<Vec<String>> {
    match fe {
        optic_syntax::FieldExpr::Base(optic_syntax::AtomExpr::Ident(id), _) => {
            if id.node == param {
                Some(vec![])
            } else {
                None
            }
        }
        optic_syntax::FieldExpr::FieldAccess { base, field, .. } => {
            let mut path = focus_field_path(param, base)?;
            path.push(field.node.clone());
            Some(path)
        }
        _ => None,
    }
}

fn lower_optic_field_expr(
    param: &str,
    fe: &optic_syntax::FieldExpr,
    _is_value: bool,
) -> hir::HirExpr {
    match fe {
        optic_syntax::FieldExpr::Index { base, span, .. } => {
            if let optic_syntax::FieldExpr::FieldAccess {
                base: inner, field, ..
            } = &**base
            {
                if let optic_syntax::FieldExpr::Base(optic_syntax::AtomExpr::Ident(id), _) =
                    &**inner
                {
                    if id.node == param {
                        return hir::HirExpr::CursorIndex {
                            cursor: "cursor".into(),
                            field: field.node.clone(),
                            span: *span,
                        };
                    }
                }
            }
            hir::HirExpr::Unsupported {
                reason: "unsupported field[index] in optic body".into(),
                span: *span,
            }
        }
        optic_syntax::FieldExpr::FieldAccess { base, field, span } => {
            if let Some(mut path) = focus_field_path(param, base) {
                path.push(field.node.clone());
                return hir::HirExpr::FocusField {
                    param: param.into(),
                    path,
                    span: *span,
                };
            }
            hir::HirExpr::Unsupported {
                reason: format!("unsupported field access `.{}` in optic body", field.node),
                span: *span,
            }
        }
        optic_syntax::FieldExpr::Base(atom, span) => match atom {
            optic_syntax::AtomExpr::Ident(id) => {
                if id.node == param {
                    hir::HirExpr::Var(param.into(), id.span)
                } else {
                    hir::HirExpr::Var(id.node.clone(), id.span)
                }
            }
            _ => hir::HirExpr::Unsupported {
                reason: "unsupported atom in optic field expr".into(),
                span: *span,
            },
        },
    }
}

impl CgirNode {
    fn node_grade(&self) -> hir::ConcreteGrade {
        match self {
            CgirNode::OpticLeaf { grade, .. }
            | CgirNode::Compose { grade, .. }
            | CgirNode::Product { grade, .. }
            | CgirNode::ProductFlat { grade, .. } => grade.clone(),
            _ => default_grade_v0(),
        }
    }

    fn summary(&self) -> Option<&Arc<hir::OpticSummary>> {
        if let CgirNode::OpticLeaf { summary, .. } = self {
            Some(summary)
        } else {
            None
        }
    }
}

pub fn format_hir_expr(e: &hir::HirExpr) -> String {
    match e {
        hir::HirExpr::LitInt(n, _) => n.to_string(),
        hir::HirExpr::LitFloat(f, _) => {
            if f.fract() == 0.0 {
                format!("{f:.1}")
            } else {
                f.to_string()
            }
        }
        hir::HirExpr::Var(v, _) => v.clone(),
        hir::HirExpr::Bin {
            op, left, right, ..
        } => {
            let op_s = match op {
                optic_syntax::BinOp::Add => "+",
                optic_syntax::BinOp::Sub => "-",
                optic_syntax::BinOp::Mul => "*",
                optic_syntax::BinOp::Div => "/",
                optic_syntax::BinOp::Lt => "<",
                optic_syntax::BinOp::Gt => ">",
                optic_syntax::BinOp::Le => "<=",
                optic_syntax::BinOp::Ge => ">=",
            };
            format!(
                "{} {} {}",
                format_hir_expr(left),
                op_s,
                format_hir_expr(right)
            )
        }
        hir::HirExpr::Paren(inner, _) => format!("({})", format_hir_expr(inner)),
        hir::HirExpr::Tuple(elems, _) => {
            let parts: Vec<_> = elems.iter().map(format_hir_expr).collect();
            format!("({})", parts.join(", "))
        }
        hir::HirExpr::TupleProj { base, index, .. } => {
            format!("{}.{}", format_hir_expr(base), index)
        }
        hir::HirExpr::FocusField { param, path, .. } => {
            format!("{param}.{}", path.join("."))
        }
        hir::HirExpr::Unsupported { reason, .. } => format!("/* unsupported: {reason} */"),
        _ => "/* unsupported cursor form */".into(),
    }
}

pub fn verify(g: &CgirGraph) -> Result<(), String> {
    let n = g.nodes.len();
    if n == 0 && !g.roots.is_empty() {
        return Err("roots reference empty graph".into());
    }
    if g.roots.len() > 1 {
        return Err(format!(
            "v0 expects at most one root, found {}",
            g.roots.len()
        ));
    }
    let resolved = &g.resolved_optics;
    for &root in &g.roots {
        if root as usize >= n {
            return Err(format!("root {root} out of bounds (nodes={n})"));
        }
    }
    for (idx, node) in g.nodes.iter().enumerate() {
        match node {
            CgirNode::Compose { lhs, rhs, .. } => {
                if *lhs as usize >= n || *rhs as usize >= n {
                    return Err(format!("node {idx}: compose edge out of bounds"));
                }
                if *lhs == idx as u32 || *rhs == idx as u32 {
                    return Err(format!("node {idx}: self-referential compose edge"));
                }
                if !compose_types_compatible(g, *lhs, *rhs) {
                    let lf = compose_emit_focus_summary(g, *lhs)
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("unknown@node:{lhs}"));
                    let rc = compose_recv_costate_summary(g, *rhs)
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("unknown@node:{rhs}"));
                    return Err(format!(
                        "node {idx}: compose type wiring invalid (focus {lf} != costate {rc})"
                    ));
                }
            }
            CgirNode::Product {
                lhs,
                rhs,
                alias_safe,
                ..
            } => {
                if *lhs as usize >= n || *rhs as usize >= n {
                    return Err(format!("node {idx}: product edge out of bounds"));
                }
                if *lhs == idx as u32 || *rhs == idx as u32 {
                    return Err(format!("node {idx}: self-referential product edge"));
                }
                if !alias_safe {
                    return Err(format!("node {idx}: product alias_safe is false"));
                }
            }
            CgirNode::ProductFlat {
                children,
                alias_safe,
                ..
            } => {
                if children.len() < 2 {
                    return Err(format!(
                        "node {idx}: ProductFlat requires at least two children"
                    ));
                }
                let mut seen = std::collections::HashSet::new();
                for &cid in children {
                    if cid as usize >= n {
                        return Err(format!("node {idx}: ProductFlat child {cid} out of bounds"));
                    }
                    if cid == idx as u32 {
                        return Err(format!("node {idx}: self-referential ProductFlat child"));
                    }
                    if !seen.insert(cid) {
                        return Err(format!(
                            "node {idx}: ProductFlat child {cid} appears more than once"
                        ));
                    }
                    if !matches!(g.nodes.get(cid as usize), Some(CgirNode::OpticLeaf { .. })) {
                        return Err(format!(
                            "node {idx}: ProductFlat child {cid} must be OpticLeaf"
                        ));
                    }
                }
                if !alias_safe {
                    return Err(format!("node {idx}: ProductFlat alias_safe is false"));
                }
            }
            CgirNode::FusedLoop {
                id,
                original_ids,
                compose_body,
                ..
            } => {
                if g.provenance_index.get(id).is_none() {
                    return Err(format!(
                        "node {idx}: FusedLoop missing provenance_index entry"
                    ));
                }
                if original_ids.len() < 2 {
                    return Err(format!(
                        "node {idx}: FusedLoop original_ids must list at least two nodes"
                    ));
                }
                for oid in original_ids {
                    if *oid as usize >= n {
                        return Err(format!("node {idx}: fused ref {oid} out of bounds"));
                    }
                }
                if let Some(body) = compose_body {
                    if body.compose_id as usize >= n
                        || body.lhs as usize >= n
                        || body.rhs as usize >= n
                    {
                        return Err(format!("node {idx}: compose fused body edge out of bounds"));
                    }
                    let Some(CgirNode::Compose { .. }) = g.nodes.get(body.compose_id as usize)
                    else {
                        return Err(format!(
                            "node {idx}: compose_body.compose_id {} is not a Compose node",
                            body.compose_id
                        ));
                    };
                    let entry = compose_entry_leaf(g, body.compose_id).ok_or_else(|| {
                        format!(
                            "node {idx}: compose_body compose_id {} has no entry leaf",
                            body.compose_id
                        )
                    })?;
                    let exit = compose_exit_leaf(g, body.compose_id).ok_or_else(|| {
                        format!(
                            "node {idx}: compose_body compose_id {} has no exit leaf",
                            body.compose_id
                        )
                    })?;
                    if body.lhs != entry || body.rhs != exit {
                        return Err(format!(
                            "node {idx}: compose_body entry/exit ({entry},{exit}) != stored ({},{})",
                            body.lhs, body.rhs
                        ));
                    }
                    let chain = compose_leaf_chain(g, body.compose_id).ok_or_else(|| {
                        format!(
                            "node {idx}: compose_body compose_id {} is not a leaf spine",
                            body.compose_id
                        )
                    })?;
                    if chain.len() < 2 {
                        return Err(format!(
                            "node {idx}: compose fused body requires at least two leaves"
                        ));
                    }
                    for w in chain.windows(2) {
                        if let (
                            Some(CgirNode::OpticLeaf { summary: lf, .. }),
                            Some(CgirNode::OpticLeaf { summary: rc, .. }),
                        ) = (g.nodes.get(w[0] as usize), g.nodes.get(w[1] as usize))
                        {
                            if !lf.focus.eq_ignore_ascii_case(&rc.costate) {
                                return Err(format!(
                                    "node {idx}: compose fusion chain wiring invalid (focus {} != costate {})",
                                    lf.focus, rc.costate
                                ));
                            }
                        } else {
                            return Err(format!(
                                "node {idx}: compose fused chain must be OpticLeaf nodes"
                            ));
                        }
                    }
                }
            }
            CgirNode::QueryGet { optic_name, .. }
            | CgirNode::QuerySet { optic_name, .. }
            | CgirNode::QueryMap { optic_name, .. } => {
                if optic_name == "par-direct" || optic_name == "unknown" {
                    return Err(format!("node {idx}: unresolved query optic `{optic_name}`"));
                }
                if !resolved.contains_key(optic_name) {
                    return Err(format!(
                        "node {idx}: query optic `{optic_name}` not in resolved_optics"
                    ));
                }
            }
            _ => {}
        }
    }
    verify_acyclic(g)?;
    // Reachability: orphan QueryMaps are invalid unless superseded by fusion provenance.
    let mut superseded = std::collections::HashSet::new();
    for prov in g.provenance_index.values() {
        for &oid in &prov.original_ids {
            superseded.insert(oid);
        }
    }
    let mut live = std::collections::HashSet::new();
    for &r in &g.roots {
        mark_reachable(g, r, &mut live);
    }
    for (i, node) in g.nodes.iter().enumerate() {
        let id = i as NodeId;
        if matches!(node, CgirNode::OpticLeaf { .. }) {
            live.insert(id);
        }
        if !live.contains(&id) {
            if matches!(node, CgirNode::QueryMap { .. }) && !superseded.contains(&id) {
                return Err(format!(
                    "node {id}: unreachable orphan QueryMap after fusion"
                ));
            }
            if let CgirNode::FusedLoop {
                id: fid,
                compose_body: Some(_),
                ..
            } = node
            {
                return Err(format!(
                    "node {fid}: unreachable FusedLoop with materialized compose body"
                ));
            }
        }
    }
    Ok(())
}

fn cgir_structural_children(node: &CgirNode) -> Vec<NodeId> {
    match node {
        CgirNode::Compose { lhs, rhs, .. } => vec![*lhs, *rhs],
        CgirNode::Product { lhs, rhs, .. } => vec![*lhs, *rhs],
        CgirNode::ProductFlat { children, .. } => children.clone(),
        CgirNode::FusedLoop {
            original_ids,
            compose_body,
            ..
        } => {
            let mut out = original_ids.clone();
            if let Some(body) = compose_body {
                out.push(body.compose_id);
                out.push(body.lhs);
                out.push(body.rhs);
            }
            out
        }
        _ => vec![],
    }
}

fn acyclic_dfs(
    g: &CgirGraph,
    n: usize,
    u: usize,
    state: &mut [u8],
) -> Result<(), String> {
    if state[u] == 1 {
        return Err(format!("node {u}: cycle in structural CGIR edges"));
    }
    if state[u] == 2 {
        return Ok(());
    }
    state[u] = 1;
    if let Some(node) = g.nodes.get(u) {
        for child in cgir_structural_children(node) {
            let c = child as usize;
            if c < n {
                acyclic_dfs(g, n, c, state)?;
            }
        }
    }
    state[u] = 2;
    Ok(())
}

/// ch10 structural acyclicity over Compose/Product/ProductFlat/FusedLoop edges.
fn verify_acyclic(g: &CgirGraph) -> Result<(), String> {
    let n = g.nodes.len();
    let mut state = vec![0u8; n];
    for i in 0..n {
        if state[i] == 0 {
            acyclic_dfs(g, n, i, &mut state)?;
        }
    }
    Ok(())
}

fn mark_reachable(g: &CgirGraph, id: NodeId, live: &mut std::collections::HashSet<NodeId>) {
    if !live.insert(id) {
        return;
    }
    if let Some(node) = g.nodes.get(id as usize) {
        match node {
            CgirNode::Compose { lhs, rhs, .. } => {
                mark_reachable(g, *lhs, live);
                mark_reachable(g, *rhs, live);
            }
            CgirNode::Product { lhs, rhs, .. } => {
                mark_reachable(g, *lhs, live);
                mark_reachable(g, *rhs, live);
            }
            CgirNode::ProductFlat { children, .. } => {
                for &cid in children {
                    mark_reachable(g, cid, live);
                }
            }
            CgirNode::FusedLoop { original_ids, .. } => {
                for &oid in original_ids {
                    mark_reachable(g, oid, live);
                }
            }
            CgirNode::QueryGet { optic_name, .. }
            | CgirNode::QuerySet { optic_name, .. }
            | CgirNode::QueryMap { optic_name, .. } => {
                if let Some(&nid) = g.resolved_optics.get(optic_name) {
                    mark_reachable(g, nid, live);
                }
            }
            _ => {}
        }
    }
}

pub fn node_id(node: &CgirNode) -> NodeId {
    match node {
        CgirNode::OpticLeaf { id, .. }
        | CgirNode::Compose { id, .. }
        | CgirNode::Product { id, .. }
        | CgirNode::ProductFlat { id, .. }
        | CgirNode::QueryGet { id, .. }
        | CgirNode::QuerySet { id, .. }
        | CgirNode::QueryMap { id, .. }
        | CgirNode::FusedLoop { id, .. } => *id,
    }
}

pub fn dump_pretty(g: &CgirGraph) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "CGIR ({} nodes, roots {:?})\n",
        g.nodes.len(),
        g.roots
    ));
    for (i, node) in g.nodes.iter().enumerate() {
        let kind = match node {
            CgirNode::OpticLeaf { name, .. } => format!("OpticLeaf({name})"),
            CgirNode::Compose { lhs, rhs, .. } => format!("Compose({lhs},{rhs})"),
            CgirNode::Product {
                lhs,
                rhs,
                alias_safe,
                ..
            } => {
                format!("Product({lhs},{rhs},alias_safe={alias_safe})")
            }
            CgirNode::ProductFlat {
                children,
                alias_safe,
                ..
            } => {
                format!("ProductFlat({children:?},alias_safe={alias_safe})")
            }
            CgirNode::QueryGet { optic_name, .. } => format!("QueryGet({optic_name})"),
            CgirNode::QuerySet {
                optic_name,
                value_repr,
                ..
            } => {
                format!("QuerySet({optic_name}, val={value_repr})")
            }
            CgirNode::QueryMap { optic_name, .. } => format!("QueryMap({optic_name})"),
            CgirNode::FusedLoop {
                original_ids,
                compose_body,
                ..
            } => {
                if let Some(b) = compose_body {
                    format!(
                        "FusedLoop(orig={original_ids:?},compose={},{})",
                        b.lhs, b.rhs
                    )
                } else {
                    format!("FusedLoop(orig={original_ids:?})")
                }
            }
        };
        let nid = node_id(node);
        let prov = g
            .provenance_index
            .get(&nid)
            .map(|p| format!("{:?}", p.reason))
            .unwrap_or_else(|| "none".into());
        out.push_str(&format!("  [{i}] {kind}  provenance={prov}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use optic_hir as hir;
    use optic_typeck::TypedHir;
    use std::collections::HashMap;

    fn mk_typed_with_optic(_name: &str) -> TypedHir {
        // minimal: use lower on small src? but to avoid cycle, construct simple
        // for test, we call build on empty-ish; real tests use CLI or hir lower
        TypedHir {
            items: vec![],
            summaries: HashMap::new(),
        }
    }

    fn minimal_hir_optic_item(name: &str, summary: Arc<hir::OpticSummary>) -> hir::HirItem {
        hir::HirItem::Optic {
            decl: optic_syntax::OpticDecl {
                name: optic_syntax::Spanned::new(name.into(), optic_syntax::Span::dummy()),
                type_ctor: optic_syntax::OpticTypeCtor::GradedOptic,
                unsafe_boundary: false,
                costate: optic_syntax::TypeExpr::Named {
                    name: "E".into(),
                    args: vec![],
                    span: optic_syntax::Span::dummy(),
                },
                focus: optic_syntax::TypeExpr::Named {
                    name: "f32".into(),
                    args: vec![],
                    span: optic_syntax::Span::dummy(),
                },
                grade: optic_syntax::GradeExpr {
                    dims: vec![],
                    span: optic_syntax::Span::dummy(),
                },
                get: Some(optic_syntax::GetClause {
                    param: optic_syntax::Spanned::new("s".into(), optic_syntax::Span::dummy()),
                    body: optic_syntax::Expr::Atom(optic_syntax::AtomExpr::Ident(
                        optic_syntax::Spanned::new("s".into(), optic_syntax::Span::dummy()),
                    )),
                    span: optic_syntax::Span::dummy(),
                }),
                put: None,
                preview: None,
                review: None,
                span: optic_syntax::Span::dummy(),
            },
            summary,
        }
    }

    #[test]
    fn test_build_basic() {
        let t = mk_typed_with_optic("H");
        let g = build(&t).expect("build");
        assert!(g.nodes.is_empty());
    }

    #[test]
    fn test_validate_optic_body_expr_soa_denylist() {
        let mut map = hir::RegionMap::default();
        map.columns.insert(
            "healths".into(),
            hir::ColumnInfo {
                name: "healths".into(),
                rust_ty: "Vec<f32>".into(),
                element_ty: Some("f32".into()),
            },
        );
        map.columns.insert(
            "transforms".into(),
            hir::ColumnInfo {
                name: "transforms".into(),
                rust_ty: "Vec<Transform>".into(),
                element_ty: Some("Transform".into()),
            },
        );
        let allowed = validate_optic_body_expr(
            hir::HirExpr::FocusField {
                param: "t".into(),
                path: vec!["position".into()],
                span: Span::dummy(),
            },
            &map,
        );
        assert!(matches!(allowed, hir::HirExpr::FocusField { .. }));
        let denied_col = validate_optic_body_expr(
            hir::HirExpr::FocusField {
                param: "s".into(),
                path: vec!["healths".into()],
                span: Span::dummy(),
            },
            &map,
        );
        assert!(matches!(denied_col, hir::HirExpr::Unsupported { .. }));
        let denied_nested = validate_optic_body_expr(
            hir::HirExpr::FocusField {
                param: "s".into(),
                path: vec!["transforms".into(), "position".into()],
                span: Span::dummy(),
            },
            &map,
        );
        assert!(matches!(denied_nested, hir::HirExpr::Unsupported { .. }));
    }

    #[test]
    fn test_verify_product_flat_invariants() {
        let sum = Arc::new(hir::OpticSummary {
            name: Some("H".into()),
            costate: "E".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec![],
            get_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: hir::OwnershipDim {
                    share: hir::Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: hir::OwnershipDim {
                    share: hir::Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: Span::dummy(),
        });
        let leaf = CgirNode::OpticLeaf {
            id: 0,
            name: "H".into(),
            costate: "E".into(),
            focus: "f32".into(),
            grade: sum.get_grade.clone(),
            get_fn: "".into(),
            put_fn: "".into(),
            get_param: "s".into(),
            get_body: Arc::new(hir::HirExpr::Var("s".into(), Span::dummy())),
            put_state_param: None,
            put_value_param: None,
            put_value_body: None,
            summary: sum,
            provenance: Span::dummy(),
        };
        let cases: Vec<(CgirGraph, &str)> = vec![
            (
                CgirGraph {
                    nodes: vec![
                        leaf.clone(),
                        CgirNode::ProductFlat {
                            id: 1,
                            children: vec![0],
                            grade: default_grade_v0(),
                            alias_safe: true,
                            provenance: Span::dummy(),
                        },
                    ],
                    roots: vec![1],
                    provenance_index: Default::default(),
                    resolved_optics: Default::default(),
                    region_map: Default::default(),
                },
                "at least two children",
            ),
            (
                CgirGraph {
                    nodes: vec![
                        leaf.clone(),
                        leaf.clone(),
                        CgirNode::ProductFlat {
                            id: 2,
                            children: vec![0, 0],
                            grade: default_grade_v0(),
                            alias_safe: true,
                            provenance: Span::dummy(),
                        },
                    ],
                    roots: vec![2],
                    provenance_index: Default::default(),
                    resolved_optics: Default::default(),
                    region_map: Default::default(),
                },
                "more than once",
            ),
            (
                CgirGraph {
                    nodes: vec![
                        leaf.clone(),
                        CgirNode::Product {
                            id: 1,
                            lhs: 0,
                            rhs: 0,
                            grade: default_grade_v0(),
                            alias_safe: true,
                            provenance: Span::dummy(),
                        },
                        CgirNode::ProductFlat {
                            id: 2,
                            children: vec![0, 1],
                            grade: default_grade_v0(),
                            alias_safe: true,
                            provenance: Span::dummy(),
                        },
                    ],
                    roots: vec![2],
                    provenance_index: Default::default(),
                    resolved_optics: Default::default(),
                    region_map: Default::default(),
                },
                "must be OpticLeaf",
            ),
        ];
        for (g, needle) in cases {
            let err = verify(&g).expect_err("verify should fail");
            assert!(err.contains(needle), "expected `{needle}` in `{err}`");
        }
    }

    #[test]
    fn test_arc_summary_in_optic_leaf() {
        // ch10.9: Arc<OpticSummary> shared from typed HIR into OpticLeaf without deep clone.
        let arc_sum = std::sync::Arc::new(hir::OpticSummary {
            name: Some("H".into()),
            costate: "Entities".into(),
            focus: "f32".into(),
            lift: hir::PathLift::default(),
            get_reads: vec!["healths".into()],
            put_reads: vec![],
            put_writes: vec![],
            get_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: hir::OwnershipDim {
                    share: hir::Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            put_grade: hir::ConcreteGrade {
                cache: 1,
                ownership: hir::OwnershipDim {
                    share: hir::Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            },
            get_determinism: hir::Determinism::Pure,
            put_determinism: hir::Determinism::Pure,
            serializable: true,
            provenance: optic_syntax::Span::dummy(),
        });
        let mut items = vec![];
        items.push(minimal_hir_optic_item("H", std::sync::Arc::clone(&arc_sum)));
        let typed = optic_typeck::TypedHir {
            items,
            summaries: std::collections::HashMap::new(),
        };
        let g = build(&typed).expect("build");
        let leaf_sum_ref = g
            .nodes
            .iter()
            .find_map(|n| {
                if let CgirNode::OpticLeaf { summary, .. } = n {
                    Some(summary)
                } else {
                    None
                }
            })
            .expect("has OpticLeaf");
        // ptr_eq proves Arc sharing from typed item summary into OpticLeaf (ch10.9).
        assert!(
            std::sync::Arc::ptr_eq(&arc_sum, leaf_sum_ref),
            "Arc ptr_eq sharing from input summary to OpticLeaf"
        );
        assert!(std::sync::Arc::strong_count(leaf_sum_ref) >= 1);
        assert!(leaf_sum_ref.get_reads.contains(&"healths".to_string()));
    }

    #[test]
    fn test_integration_large_n_lower_check_build_arc_capacity() {
        // End-to-end: parse -> lower -> check -> build (N=8 optics, ch10.9 Arc flow).
        let mut src =
            "data E { r0: SoA<f32>, r1: SoA<f32>, r2: SoA<f32>, r3: SoA<f32>, r4: SoA<f32> }\n"
                .to_string();
        for i in 0..8 {
            let r = i % 5;
            src.push_str(&format!(
                r#"optic H{i}: GradedOptic<E,f32,_> {{ get s=>s.r{r}[s.id] put(s,v)=>{{s.r{r}[s.id]=v}} }}
"#
            ));
        }
        src.push_str("let c = H0 *** H1;\nfn main() { entities.query(c).map(|(a,b)|a+b); }\n");
        let prog =
            optic_syntax::parse(&src, optic_syntax::SourceId(1)).expect("parse for integration");
        let hirp = optic_hir::lower(prog).expect("lower for integration");
        let typed = optic_typeck::check(hirp).expect("check for integration");
        let h0_pre = typed
            .summaries
            .get("H0")
            .map(|s| std::sync::Arc::strong_count(s))
            .unwrap_or(0);
        let g = build(&typed).expect("build for integration");
        // verify Arc summaries flowed to leaves (sharing), dedup (unique r's), pipeline produced graph with optics
        let mut leaf_count = 0;
        let mut unique_r = std::collections::HashSet::new();
        for n in &g.nodes {
            if let CgirNode::OpticLeaf {
                name: leaf_name,
                summary,
                ..
            } = n
            {
                leaf_count += 1;
                assert!(
                    std::sync::Arc::strong_count(summary) >= 1,
                    "Arc in leaf post full pipeline"
                );
                if leaf_name == "H0" {
                    if let Some(h0) = typed.summaries.get("H0") {
                        assert!(
                            std::sync::Arc::ptr_eq(h0, summary),
                            "ptr_eq sharing from typed summary to leaf (full pipeline flow)"
                        );
                        assert!(
                            std::sync::Arc::strong_count(summary) >= h0_pre,
                            "strong count delta post build clone"
                        );
                    }
                }
                for r in &summary.get_reads {
                    unique_r.insert(r.clone());
                }
            }
        }
        assert!(
            leaf_count >= 8,
            "large-N optics lowered/checked/built to leaves"
        );
        assert!(
            unique_r.len() <= 5,
            "dedup exercised end-to-end in pipeline (overlaps)"
        );
        assert!(!g.nodes.is_empty());
    }

    #[test]
    fn test_verify_ok_and_fail() {
        let g_ok = CgirGraph {
            nodes: vec![CgirNode::OpticLeaf {
                id: 0,
                name: "H".into(),
                costate: "E".into(),
                focus: "f".into(),
                grade: default_grade_v0(),
                get_fn: "".into(),
                put_fn: "".into(),
                get_param: "s".into(),
                get_body: std::sync::Arc::new(hir::HirExpr::Var(
                    "s".into(),
                    optic_syntax::Span::dummy(),
                )),
                put_state_param: None,
                put_value_param: None,
                put_value_body: None,
                summary: std::sync::Arc::new(hir::OpticSummary {
                    name: Some("H".into()),
                    costate: "E".into(),
                    focus: "f32".into(),
                    lift: hir::PathLift::default(),
                    get_reads: vec!["healths".into()],
                    put_reads: vec![],
                    put_writes: vec![],
                    get_grade: default_grade_v0(),
                    put_grade: default_grade_v0(),
                    get_determinism: hir::Determinism::Pure,
                    put_determinism: hir::Determinism::Pure,
                    serializable: true,
                    provenance: optic_syntax::Span::dummy(),
                }),
                provenance: optic_syntax::Span::dummy(),
            }],
            roots: vec![],
            provenance_index: std::collections::BTreeMap::new(),
            resolved_optics: [("H".into(), 0)].into_iter().collect(),
            region_map: hir::RegionMap::default(),
        };
        assert!(verify(&g_ok).is_ok());

        let g_bad = CgirGraph {
            nodes: vec![],
            roots: vec![0],
            provenance_index: std::collections::BTreeMap::new(),
            resolved_optics: std::collections::HashMap::new(),
            region_map: hir::RegionMap::default(),
        };
        assert!(verify(&g_bad).is_err());
    }

    #[test]
    fn test_build_chained_seq_compose_tree() {
        let src = std::fs::read_to_string(format!(
            "{}/../../examples/compose_triple.opt",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("read");
        let prog = optic_syntax::parse(&src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let typed = optic_typeck::check(hirp).expect("check");
        let g = build(&typed).expect("build");
        let compose_count = g
            .nodes
            .iter()
            .filter(|n| matches!(n, CgirNode::Compose { .. }))
            .count();
        assert_eq!(compose_count, 2, "A>>>B>>>C must produce two Compose nodes");
        verify(&g).expect("chained compose should verify wiring");
    }

    #[test]
    fn test_verify_rejects_structural_cycle() {
        let grade = default_grade_v0();
        let make_leaf = |id: u32, name: &str| {
            let summary = Arc::new(hir::OpticSummary {
                name: Some(name.into()),
                costate: "E".into(),
                focus: "E".into(),
                lift: hir::PathLift::default(),
                get_reads: vec!["h".into()],
                put_reads: vec![],
                put_writes: vec!["h".into()],
                get_grade: grade.clone(),
                put_grade: grade.clone(),
                get_determinism: hir::Determinism::Pure,
                put_determinism: hir::Determinism::Pure,
                serializable: true,
                provenance: optic_syntax::Span::dummy(),
            });
            CgirNode::OpticLeaf {
                id,
                name: name.into(),
                costate: "E".into(),
                focus: "E".into(),
                grade: grade.clone(),
                get_fn: String::new(),
                put_fn: String::new(),
                get_param: "s".into(),
                get_body: Arc::new(hir::HirExpr::LitInt(0, optic_syntax::Span::dummy())),
                put_state_param: None,
                put_value_param: None,
                put_value_body: None,
                summary,
                provenance: optic_syntax::Span::dummy(),
            }
        };
        let g = CgirGraph {
            nodes: vec![
                make_leaf(0, "A"),
                make_leaf(1, "B"),
                CgirNode::Compose {
                    id: 2,
                    lhs: 0,
                    rhs: 3,
                    grade: grade.clone(),
                    provenance: optic_syntax::Span::dummy(),
                },
                CgirNode::Compose {
                    id: 3,
                    lhs: 2,
                    rhs: 1,
                    grade,
                    provenance: optic_syntax::Span::dummy(),
                },
            ],
            roots: vec![2],
            provenance_index: Default::default(),
            region_map: Default::default(),
            resolved_optics: [("A".into(), 0), ("B".into(), 1)]
                .into_iter()
                .collect(),
        };
        let err = verify(&g).expect_err("cycle must fail verify");
        assert!(
            err.contains("cycle in structural CGIR edges"),
            "expected acyclicity error: {err}"
        );
    }

    #[test]
    fn test_verify_accepts_let_alias_decay() {
        let src = std::fs::read_to_string(format!(
            "{}/../../examples/health_decay.opt",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("read");
        let prog = optic_syntax::parse(&src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let typed = optic_typeck::check(hirp).expect("check");
        let g = build(&typed).expect("build");
        assert!(g.resolved_optics.contains_key("decay"));
        verify(&g).expect("decay alias should verify");
    }

    #[test]
    fn test_build_populates_region_map_from_nested_position() {
        let src = std::fs::read_to_string(format!(
            "{}/../../examples/nested_position.opt",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("read nested_position.opt");
        let prog = optic_syntax::parse(&src, optic_syntax::SourceId(1)).expect("parse");
        let hirp = optic_hir::lower(prog).expect("lower");
        let typed = optic_typeck::check(hirp).expect("check");
        let g = build(&typed).expect("build");
        assert_eq!(g.region_map.costate_name, "Entities");
        assert!(g.region_map.columns.contains_key("transforms"));
        assert!(g.region_map.record_fields.contains_key("Transform"));
        assert_eq!(
            g.region_map.columns["transforms"].element_ty.as_deref(),
            Some("Transform")
        );
    }

    #[test]
    fn test_query_get_set_pipeline() {
        for (file, kind) in [
            ("health_get.opt", "QueryGet"),
            ("health_set.opt", "QuerySet"),
        ] {
            let src = std::fs::read_to_string(format!(
                "{}/../../examples/{file}",
                env!("CARGO_MANIFEST_DIR")
            ))
            .expect("read example");
            let prog = optic_syntax::parse(&src, optic_syntax::SourceId(1)).expect("parse");
            let hirp = optic_hir::lower(prog).expect("lower");
            let typed = optic_typeck::check(hirp).expect("check");
            let g = build(&typed).expect("build");
            assert!(g.nodes.iter().any(|n| {
                match n {
                    CgirNode::QueryGet { optic_name, .. } if kind == "QueryGet" => {
                        optic_name == "HealthView"
                    }
                    CgirNode::QuerySet { optic_name, .. } if kind == "QuerySet" => {
                        optic_name == "HealthView"
                    }
                    _ => false,
                }
            }));
        }
    }
}
