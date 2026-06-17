//! optic-opt — the 3 fusions (ch. 10, M4).
//! Fixed-point driver. For demo, mark product as fused into FusedLoop.

use optic_cgir::{CgirGraph, CgirNode, FusionReason, NodeId}; // FusionProvenance unused (direct in opt)

pub fn optimize(mut g: CgirGraph) -> CgirGraph {
    // simple: if there is a Product, replace with a FusedLoop carrying provenance
    let has_product = g
        .nodes
        .iter()
        .any(|n| matches!(n, CgirNode::Product { .. }));
    if has_product {
        let fid: NodeId = g.nodes.len() as u32;
        let orig: Vec<NodeId> = (0..g.nodes.len() as u32).collect();
        g.nodes.push(CgirNode::FusedLoop {
            id: fid,
            original_ids: orig.clone(),
            costate: "Entities".into(),
            provenance: optic_syntax::Span::dummy(),
        });
        if let Some(p) = g.provenance_index.get_mut(&0) {
            p.original_ids = orig;
            p.reason = FusionReason::ProductFlattening;
        }
        g.roots = vec![fid];
    }
    g
}
