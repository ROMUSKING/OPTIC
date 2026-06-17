//! optic-opt — the 3 fusions (ch. 10, M4).
//! Fixed-point driver (≤8 iters): map fusion, compose fusion, product flatten (ch10 order).

use optic_cgir::{verify, CgirGraph, CgirNode, FusionProvenance, FusionReason, NodeId};
use optic_hir as hir;
use optic_syntax::Span;

const MAX_FUSION_ITERS: usize = 8;

/// Run fusion passes until fixed point. Aborts on verify violation (ch10 post-fusion check).
pub fn optimize(mut g: CgirGraph) -> Result<CgirGraph, String> {
    for _ in 0..MAX_FUSION_ITERS {
        let n_before = g.nodes.len();
        g = map_fusion(g);
        verify(&g)?;
        g = compose_fusion(g);
        verify(&g)?;
        g = product_flatten(g);
        verify(&g)?;
        if g.nodes.len() == n_before {
            break;
        }
    }
    verify(&g)?;
    Ok(g)
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

    let mut chain = vec![];
    let mut costate = String::new();
    for &rid in &map_roots {
        if let Some(CgirNode::QueryMap {
            costate: cs,
            optic_name,
            map_param,
            map_body,
            provenance,
            ..
        }) = g.nodes.get(rid as usize)
        {
            if chain.is_empty() {
                costate = cs.clone();
            } else if cs != &costate {
                return g;
            }
            chain.push((
                rid,
                optic_name.clone(),
                map_param.clone(),
                map_body.clone(),
                *provenance,
            ));
        }
    }
    if chain.len() <= 1 {
        return g;
    }

    let fused_id = chain[0].0;
    let param = chain[0].2.clone();
    let span = chain[0].4;
    let mut body = chain[0].3.as_ref().clone();
    for (_, _, inner_param, next_body, _) in chain.iter().skip(1) {
        let params: Vec<&str> = inner_param.split(',').map(str::trim).collect();
        if params.len() > 1 && !matches!(body, hir::HirExpr::Tuple(_, _)) {
            return g;
        }
        body = substitute_all_params(next_body.as_ref(), inner_param, &body);
    }

    if let Some(CgirNode::QueryMap {
        map_param,
        map_body,
        provenance,
        ..
    }) = g.nodes.get_mut(fused_id as usize)
    {
        *map_param = param;
        *map_body = std::sync::Arc::new(body);
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
    if params.len() > 1 && !matches!(repl, hir::HirExpr::Tuple(_, _)) {
        return e.clone();
    }
    let mut out = e.clone();
    if params.len() > 1 {
        if let hir::HirExpr::Tuple(elems, _) = repl {
            for (p, el) in params.iter().zip(elems.iter()) {
                out = substitute_map_body(&out, p, el);
            }
            return out;
        }
    }
    let inner = params.first().copied().unwrap_or("it");
    substitute_map_body(&out, inner, repl)
}

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
        hir::HirExpr::Unsupported { reason, span } => hir::HirExpr::Unsupported {
            reason: reason.clone(),
            span: *span,
        },
    }
}

/// Compose fusion: annotate sequential compose+map queries with FusedLoop (ch10).
fn compose_fusion(mut g: CgirGraph) -> CgirGraph {
    let has_map_root = g
        .roots
        .iter()
        .any(|&r| matches!(g.nodes.get(r as usize), Some(CgirNode::QueryMap { .. })));
    if !has_map_root {
        return g;
    }

    let compose_ids: Vec<NodeId> = g
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(i, n)| {
            if matches!(n, CgirNode::Compose { .. }) {
                Some(i as NodeId)
            } else {
                None
            }
        })
        .collect();
    if compose_ids.is_empty() {
        return g;
    }

    let already_compose_fused = g.provenance_index.values().any(|p| {
        p.reason == FusionReason::ComposeFusion
            && compose_ids.iter().any(|id| p.original_ids.contains(id))
    });
    if already_compose_fused {
        return g;
    }

    let fid: NodeId = g.nodes.len() as u32;
    let span = g
        .nodes
        .get(compose_ids[0] as usize)
        .map(node_provenance)
        .unwrap_or_else(Span::dummy);
    let orig = compose_ids.clone();
    g.nodes.push(CgirNode::FusedLoop {
        id: fid,
        original_ids: orig.clone(),
        costate: "Entities".into(),
        provenance: span,
    });
    g.provenance_index.insert(
        fid,
        FusionProvenance {
            original_ids: orig,
            spans: vec![span],
            reason: FusionReason::ComposeFusion,
        },
    );
    g
}

/// Product flattening: attach FusedLoop for map+product queries (ch10).
fn product_flatten(mut g: CgirGraph) -> CgirGraph {
    let map_root = g
        .roots
        .iter()
        .find(|&&r| matches!(g.nodes.get(r as usize), Some(CgirNode::QueryMap { .. })));
    let Some(&map_root) = map_root else {
        return g;
    };

    let product_ids: Vec<NodeId> = g
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(i, n)| {
            if matches!(n, CgirNode::Product { .. }) {
                Some(i as NodeId)
            } else {
                None
            }
        })
        .collect();
    if product_ids.is_empty() {
        return g;
    }

    let already = g.provenance_index.values().any(|p| {
        p.reason == FusionReason::ProductFlattening
            && product_ids.iter().any(|id| p.original_ids.contains(id))
    });
    if already {
        return g;
    }

    let fid: NodeId = g.nodes.len() as u32;
    let span = g
        .nodes
        .get(product_ids[0] as usize)
        .map(node_provenance)
        .unwrap_or_else(Span::dummy);
    let orig = product_ids.clone();
    g.nodes.push(CgirNode::FusedLoop {
        id: fid,
        original_ids: orig.clone(),
        costate: "Entities".into(),
        provenance: span,
    });
    g.provenance_index.insert(
        fid,
        FusionProvenance {
            original_ids: orig,
            spans: vec![span],
            reason: FusionReason::ProductFlattening,
        },
    );
    g.roots = vec![map_root];
    g
}

fn node_provenance(n: &CgirNode) -> Span {
    match n {
        CgirNode::OpticLeaf { provenance, .. }
        | CgirNode::Compose { provenance, .. }
        | CgirNode::Product { provenance, .. }
        | CgirNode::QueryGet { provenance, .. }
        | CgirNode::QuerySet { provenance, .. }
        | CgirNode::QueryMap { provenance, .. }
        | CgirNode::FusedLoop { provenance, .. } => *provenance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use optic_hir::{HirExpr, OwnershipDim, Rational};
    use optic_syntax::{BinOp, Span};
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn empty_resolved() -> std::collections::HashMap<String, NodeId> {
        std::collections::HashMap::new()
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
                CgirNode::OpticLeaf {
                    id: 0,
                    name: "H".into(),
                    costate: "E".into(),
                    focus: "f32".into(),
                    grade: sum.get_grade.clone(),
                    get_fn: "".into(),
                    put_fn: "".into(),
                    summary: Arc::clone(&sum),
                    provenance: Span::dummy(),
                },
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
        };
        let out = optimize(g).expect("optimize should verify");
        assert_eq!(out.roots, vec![1]);
        if let Some(CgirNode::QueryMap { map_body, .. }) = out.nodes.get(1) {
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
        };
        assert!(optimize(g).is_err());
    }

    #[test]
    fn compose_fusion_adds_fused_loop_provenance() {
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
        resolved.insert("seq".into(), 1);
        let g = CgirGraph {
            nodes: vec![
                CgirNode::OpticLeaf {
                    id: 0,
                    name: "A".into(),
                    costate: "E".into(),
                    focus: "f32".into(),
                    grade: sum.get_grade.clone(),
                    get_fn: "".into(),
                    put_fn: "".into(),
                    summary: Arc::clone(&sum),
                    provenance: Span::dummy(),
                },
                CgirNode::Compose {
                    id: 1,
                    lhs: 0,
                    rhs: 0,
                    grade: sum.get_grade.clone(),
                    provenance: Span::dummy(),
                },
                CgirNode::QueryMap {
                    id: 2,
                    optic_name: "seq".into(),
                    costate: "entities".into(),
                    cursor: "c".into(),
                    map_param: "x".into(),
                    map_body: Arc::new(HirExpr::Var("x".into(), Span::dummy())),
                    provenance: Span::dummy(),
                },
            ],
            roots: vec![2],
            provenance_index: BTreeMap::new(),
            resolved_optics: resolved,
        };
        let out = optimize(g).expect("compose fusion should verify");
        assert!(out
            .nodes
            .iter()
            .any(|n| matches!(n, CgirNode::FusedLoop { .. })));
        assert!(out.provenance_index.values().any(|p| {
            p.reason == FusionReason::ComposeFusion || p.reason == FusionReason::ProductFlattening
        }));
    }

    #[test]
    fn product_flatten_adds_fused_loop_for_product_map() {
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
                CgirNode::OpticLeaf {
                    id: 0,
                    name: "H".into(),
                    costate: "E".into(),
                    focus: "f32".into(),
                    grade: sum.get_grade.clone(),
                    get_fn: "".into(),
                    put_fn: "".into(),
                    summary: Arc::clone(&sum),
                    provenance: Span::dummy(),
                },
                CgirNode::OpticLeaf {
                    id: 1,
                    name: "H2".into(),
                    costate: "E".into(),
                    focus: "f32".into(),
                    grade: sum.get_grade.clone(),
                    get_fn: "".into(),
                    put_fn: "".into(),
                    summary: Arc::clone(&sum),
                    provenance: Span::dummy(),
                },
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
        };
        let out = optimize(g).expect("product flatten should verify");
        assert!(out
            .nodes
            .iter()
            .any(|n| matches!(n, CgirNode::FusedLoop { .. })));
        assert!(out
            .provenance_index
            .values()
            .any(|p| { p.reason == FusionReason::ProductFlattening }));
    }

    #[test]
    fn orphan_query_map_fails_verify() {
        let body = HirExpr::Var("x".into(), Span::dummy());
        let mut resolved = std::collections::HashMap::new();
        resolved.insert("H".into(), 0);
        let g = CgirGraph {
            nodes: vec![
                CgirNode::OpticLeaf {
                    id: 0,
                    name: "H".into(),
                    costate: "E".into(),
                    focus: "f32".into(),
                    grade: hir::ConcreteGrade {
                        cache: 1,
                        ownership: OwnershipDim {
                            share: Rational::one(),
                            read_only: false,
                            must_use: false,
                        },
                    },
                    get_fn: "".into(),
                    put_fn: "".into(),
                    summary: Arc::new(hir::OpticSummary {
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
                    provenance: Span::dummy(),
                },
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
        };
        assert!(verify(&g).is_err(), "orphan QueryMap must fail verify");
    }
}
