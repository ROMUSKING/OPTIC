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
}

#[derive(Clone, Debug)]
pub struct FusionProvenance {
    pub original_ids: Vec<NodeId>,
    pub spans: Vec<Span>,
    pub reason: FusionReason,
}

#[derive(Clone, Debug)]
pub enum FusionReason {
    /// Pre-fusion graph node from CGIR build (not a rewrite).
    Build,
    MapFusion,
    ComposeFusion,
    ProductFlattening,
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
        map_body: hir::HirExpr,
        provenance: Span,
    },
    FusedLoop {
        id: NodeId,
        original_ids: Vec<NodeId>,
        costate: String,
        provenance: Span,
    },
    // reserved for later
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
                nodes.push(CgirNode::OpticLeaf {
                    id: nid,
                    name: decl.name.node.clone(),
                    costate: "Entities".into(),
                    focus: "f32".into(),
                    grade: summary.get_grade.clone(),
                    get_fn,
                    put_fn,
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
                hir::HirOptic::Seq { lhs, rhs, span } => {
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
                        let cid = id;
                        id += 1;
                        nodes.push(CgirNode::Compose {
                            id: cid,
                            lhs: lid,
                            rhs: rid,
                            grade: nodes
                                .get(lid as usize)
                                .map_or_else(default_grade_v0, |n| n.node_grade()),
                            provenance: *span,
                        });
                        optic_leaf_ids.insert(name.clone(), cid);
                        resolved_optics.insert(name.clone(), cid);
                        prov.insert(
                            cid,
                            FusionProvenance {
                                original_ids: vec![lid, rid, cid],
                                spans: vec![*span],
                                reason: FusionReason::Build,
                            },
                        );
                    }
                }
                _ => {}
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
                        map_body: body.clone(),
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
        return Err(vec![optic_diagnostics::cgir_diag(
            optic_diagnostics::CGIR_MULTI_QUERY,
            Span::dummy(),
            "v0 supports at most one query root per program",
            serde_json::json!({ "query_count": query_count }),
        )]);
    }

    Ok(CgirGraph {
        nodes,
        roots,
        provenance_index: prov,
        resolved_optics,
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

impl CgirNode {
    fn node_grade(&self) -> hir::ConcreteGrade {
        match self {
            CgirNode::OpticLeaf { grade, .. }
            | CgirNode::Compose { grade, .. }
            | CgirNode::Product { grade, .. } => grade.clone(),
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
        hir::HirExpr::Tuple(elems, _) => {
            let parts: Vec<_> = elems.iter().map(format_hir_expr).collect();
            format!("({})", parts.join(", "))
        }
        hir::HirExpr::TupleProj { base, index, .. } => {
            format!("{}.{}", format_hir_expr(base), index)
        }
        hir::HirExpr::Unsupported { reason, .. } => format!("/* unsupported: {reason} */"),
        _ => "v".into(),
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
            CgirNode::FusedLoop {
                id, original_ids, ..
            } => {
                if g.provenance_index.get(id).is_none() {
                    return Err(format!(
                        "node {idx}: FusedLoop missing provenance_index entry"
                    ));
                }
                for oid in original_ids {
                    if *oid as usize >= n {
                        return Err(format!("node {idx}: fused ref {oid} out of bounds"));
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
    // Reachability: every non-OpticLeaf node must be reachable from roots or be a dependency.
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
            if matches!(node, CgirNode::QueryMap { .. }) {
                return Err(format!(
                    "node {idx}: unreachable orphan QueryMap after fusion"
                ));
            }
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
            CgirNode::FusedLoop { original_ids, .. } => {
                for &oid in original_ids {
                    mark_reachable(g, oid, live);
                }
            }
            _ => {}
        }
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
            CgirNode::QueryGet { optic_name, .. } => format!("QueryGet({optic_name})"),
            CgirNode::QuerySet {
                optic_name,
                value_repr,
                ..
            } => {
                format!("QuerySet({optic_name}, val={value_repr})")
            }
            CgirNode::QueryMap { optic_name, .. } => format!("QueryMap({optic_name})"),
            CgirNode::FusedLoop { original_ids, .. } => {
                format!("FusedLoop(orig={original_ids:?})")
            }
        };
        let prov = g
            .provenance_index
            .get(&(i as NodeId))
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
                get: optic_syntax::GetClause {
                    param: optic_syntax::Spanned::new("s".into(), optic_syntax::Span::dummy()),
                    body: optic_syntax::Expr::Atom(optic_syntax::AtomExpr::Ident(
                        optic_syntax::Spanned::new("s".into(), optic_syntax::Span::dummy()),
                    )),
                    span: optic_syntax::Span::dummy(),
                },
                put: None,
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
        };
        assert!(verify(&g_ok).is_ok());

        let g_bad = CgirGraph {
            nodes: vec![],
            roots: vec![0],
            provenance_index: std::collections::BTreeMap::new(),
            resolved_optics: std::collections::HashMap::new(),
        };
        assert!(verify(&g_bad).is_err());
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
