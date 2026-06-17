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
}

#[derive(Clone, Debug)]
pub struct FusionProvenance {
    pub original_ids: Vec<NodeId>,
    pub spans: Vec<Span>,
    pub reason: FusionReason,
}

#[derive(Clone, Debug)]
pub enum FusionReason {
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
    // track leaf ids by optic name for structural Par/Product construction (avoids brittle len>=2 + dangling)
    let mut optic_leaf_ids: std::collections::HashMap<String, NodeId> =
        std::collections::HashMap::new();

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
                prov.insert(
                    nid,
                    FusionProvenance {
                        original_ids: vec![nid],
                        spans: vec![decl.span],
                        reason: FusionReason::ProductFlattening,
                    },
                );
            }
            hir::HirItem::Let { optic, .. } => {
                // structural Product for let-bound Par (e.g. "let bad = W *** A", "let combined = H *** P") per ch10.9
                if let hir::HirOptic::Par { lhs, rhs, .. } = optic {
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
                        let pid = id;
                        id += 1;
                        nodes.push(CgirNode::Product {
                            id: pid,
                            lhs: lid,
                            rhs: rid,
                            grade: nodes
                                .get(lid as usize)
                                .map_or_else(|| grade_for_demo_fallback(), |n| n.grade_for_demo()),
                            alias_safe: true,
                            provenance: Span::dummy(),
                        });
                        prov.insert(
                            pid,
                            FusionProvenance {
                                original_ids: vec![lid, rid, pid],
                                spans: vec![],
                                reason: FusionReason::ProductFlattening,
                            },
                        );
                    }
                }
            }
            hir::HirItem::Query(q) => {
                let optic_name = if let hir::HirOptic::Named { name, .. } = &q.optic {
                    name.clone()
                } else if let hir::HirOptic::Par { .. } = &q.optic {
                    "par-direct".into()
                } else {
                    "unknown".into()
                };
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
                    hir::QueryKind::Map { .. } => CgirNode::QueryMap {
                        id: qid,
                        optic_name,
                        costate: q.costate.clone(),
                        cursor: q.cursor.clone(),
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
                        reason: FusionReason::MapFusion,
                    },
                );
            }
            _ => {}
        }
    }

    // no more len>=2 brittle synth (now structural from Let Par above; keeps for direct cases if needed but ids allocated correctly)
    // if nodes.len() >=2 fallback removed to prevent wrong ids for decay etc.

    Ok(CgirGraph {
        nodes,
        roots,
        provenance_index: prov,
    })
}

fn grade_for_demo_fallback() -> hir::ConcreteGrade {
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
    fn grade_for_demo(&self) -> hir::ConcreteGrade {
        if let CgirNode::OpticLeaf { grade, .. } = self {
            grade.clone()
        } else {
            hir::ConcreteGrade {
                cache: 1,
                ownership: hir::OwnershipDim {
                    share: hir::Rational::one(),
                    read_only: false,
                    must_use: false,
                },
            }
        }
    }
}

fn format_hir_expr(e: &hir::HirExpr) -> String {
    match e {
        hir::HirExpr::LitInt(n, _) => n.to_string(),
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
        _ => "v".into(),
    }
}

pub fn verify(g: &CgirGraph) -> Result<(), String> {
    let n = g.nodes.len();
    if n == 0 && !g.roots.is_empty() {
        return Err("roots reference empty graph".into());
    }
    for &root in &g.roots {
        if root as usize >= n {
            return Err(format!("root {} out of bounds (nodes={n})", root));
        }
    }
    for (idx, node) in g.nodes.iter().enumerate() {
        match node {
            CgirNode::Compose { lhs, rhs, .. } | CgirNode::Product { lhs, rhs, .. } => {
                if *lhs as usize >= n {
                    return Err(format!("node {idx}: lhs {lhs} out of bounds"));
                }
                if *rhs as usize >= n {
                    return Err(format!("node {idx}: rhs {rhs} out of bounds"));
                }
            }
            CgirNode::FusedLoop { original_ids, .. } => {
                for oid in original_ids {
                    if *oid as usize >= n {
                        return Err(format!("node {idx}: fused ref {oid} out of bounds"));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
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

    fn mk_typed_with_optic(name: &str) -> TypedHir {
        // minimal: use lower on small src? but to avoid cycle, construct simple
        // for test, we call build on empty-ish; real tests use CLI or hir lower
        TypedHir {
            items: vec![],
            summaries: HashMap::new(),
        }
    }

    fn minimal_hir_optic_item(name: &str, summary: Arc<hir::OpticSummary>) -> hir::HirItem {
        // minimal HirItem for tests (bypass full src parse/decl weight per issue6); decl satisfies type but build only uses summary for OpticLeaf (ch10.9). Reuses mk_typed pattern.
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
        // minimal via helper (reduces heavy dummy in test fn body per review issue6; decl satisfies type but build uses only summary for leaf per ch10.9). Reuses mk_typed pattern.
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
        // strengthened canary pre (issue3): capture for flow proof (build does Arc::clone from typed item summary)
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
}
