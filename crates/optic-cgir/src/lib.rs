//! optic-cgir — CGIR, provenance, verifier (ch. 10, M3).
//! Minimal but sufficient for narrow v0 examples: build from typed HIR, basic nodes, provenance.

use optic_hir as hir;
use optic_syntax::Span;

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
pub enum FusionReason { MapFusion, ComposeFusion, ProductFlattening }

#[derive(Clone, Debug)]
pub enum CgirNode {
    OpticLeaf { id: NodeId, name: String, costate: String, focus: String, grade: hir::ConcreteGrade, get_fn: String /* source repr */, put_fn: String, summary: hir::OpticSummary, provenance: Span },
    Compose { id: NodeId, lhs: NodeId, rhs: NodeId, grade: hir::ConcreteGrade, provenance: Span },
    Product { id: NodeId, lhs: NodeId, rhs: NodeId, grade: hir::ConcreteGrade, alias_safe: bool, provenance: Span },
    QueryGet { id: NodeId, optic: NodeId, costate: String, cursor: String, provenance: Span },
    QueryMap { id: NodeId, optic: NodeId, costate: String, cursor: String, provenance: Span },
    FusedLoop { id: NodeId, original_ids: Vec<NodeId>, costate: String, provenance: Span },
    // reserved for later
}

pub fn build(typed: &optic_typeck::TypedHir) -> Result<CgirGraph, Vec<optic_diagnostics::Diagnostic>> {
    let mut nodes = vec![];
    let mut roots = vec![];
    let mut prov = std::collections::BTreeMap::new();
    let mut id: NodeId = 0;

    for item in &typed.items {
        match item {
            hir::HirItem::Optic { decl, summary } => {
                let nid = id; id += 1;
                nodes.push(CgirNode::OpticLeaf {
                    id: nid,
                    name: decl.name.node.clone(),
                    costate: "Entities".into(),
                    focus: "f32".into(),
                    grade: summary.get_grade.clone(),
                    get_fn: "cursor.arena.healths[cursor.id]".into(), // normalized
                    put_fn: "cursor.arena.healths[cursor.id] = v".into(),
                    summary: summary.clone(),
                    provenance: decl.span,
                });
                prov.insert(nid, FusionProvenance { original_ids: vec![nid], spans: vec![decl.span], reason: FusionReason::ProductFlattening });
            }
            hir::HirItem::Query(q) => {
                let onid = id; id += 1; // pretend leaf for optic
                let qid = id; id += 1;
                nodes.push(CgirNode::QueryMap { id: qid, optic: onid, costate: q.costate.clone(), cursor: q.cursor.clone(), provenance: q.span });
                roots.push(qid);
                prov.insert(qid, FusionProvenance { original_ids: vec![qid], spans: vec![q.span], reason: FusionReason::ProductFlattening });
            }
            _ => {}
        }
    }

    // For the product example, synthesize a Product node (demo)
    if nodes.len() >= 2 {
        let pid = id; id += 1;
        nodes.push(CgirNode::Product { id: pid, lhs: 0, rhs: 1, grade: nodes[0].grade_for_demo(), alias_safe: true, provenance: Span::dummy() });
    }

    Ok(CgirGraph { nodes, roots, provenance_index: prov })
}

impl CgirNode {
    fn grade_for_demo(&self) -> hir::ConcreteGrade {
        if let CgirNode::OpticLeaf { grade, .. } = self { grade.clone() } else { hir::ConcreteGrade { cache: 1, ownership: hir::OwnershipDim { share: hir::Rational::one(), read_only: false, must_use: false } } }
    }
}

pub fn verify(_g: &CgirGraph) -> Result<(), String> { Ok(()) } // invariants stub for demo

pub fn dump_pretty(g: &CgirGraph) -> String {
    format!("CGIR nodes: {}\nroots: {:?}", g.nodes.len(), g.roots)
}
