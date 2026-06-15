//! optic-typeck — types, grade inference, alias safety (ch. 9, M2 "the hardest").
//! Uses OpticSummary from hir. Conservative field-root regions. Produces structured diags.

use optic_hir::{self as hir, ConcreteGrade, OwnershipDim, OpticSummary, Rational, Region};
use optic_diagnostics::{self as diag, Diagnostic, Phase};
use std::collections::HashMap;

pub type TypeckResult<T> = Result<T, Vec<Diagnostic>>;

/// Minimal typed form (attach summaries + grades).
#[derive(Clone, Debug)]
pub struct TypedHir {
    pub items: Vec<hir::HirItem>,
    pub summaries: HashMap<String, OpticSummary>,
}

/// Main entry (M2).
pub fn check(hir: hir::HirProgram) -> TypeckResult<TypedHir> {
    let mut summaries = HashMap::new();
    let mut diags = vec![];
    let mut typed_items = vec![];

    for item in hir.items {
        match &item {
            hir::HirItem::Optic { decl, summary } => {
                // grade inference / check from ann or body (simplified bottom-up)
                let inferred = infer_grade_from_summary(summary);
                let mut s = summary.clone();
                s.get_grade = inferred.clone();
                s.put_grade = inferred;
                summaries.insert(decl.name.node.clone(), s.clone());
                typed_items.push(hir::HirItem::Optic { decl: decl.clone(), summary: s });
            }
            hir::HirItem::Let { name, optic: _, summary } => {
                let s = summary.clone();
                summaries.insert(name.clone(), s.clone());
                typed_items.push(item.clone());
            }
            hir::HirItem::Query(q) => {
                // for top level queries, check alias if product involved (demo)
                if let hir::HirOptic::Par { lhs, rhs, .. } = &q.optic {
                    let lsum = resolve_summary_for_optic(lhs, &summaries);
                    let rsum = resolve_summary_for_optic(rhs, &summaries);
                    if let Err(d) = alias_safe(&lsum, &rsum) {
                        diags.push(d);
                    }
                }
                typed_items.push(item.clone());
            }
            _ => typed_items.push(item.clone()),
        }
    }

    if !diags.is_empty() {
        return Err(diags);
    }
    Ok(TypedHir { items: typed_items, summaries })
}

fn infer_grade_from_summary(s: &OpticSummary) -> ConcreteGrade {
    // body-driven: cache ~ |reads ∪ writes| sat ; ownership from ann or default Affine
    let cache = (s.get_reads.len() + s.put_writes.len()).min(255) as u8;
    let own = if s.name.as_deref() == Some("HealthView") || s.name.as_deref() == Some("PositionView") {
        OwnershipDim { share: Rational::one(), read_only: false, must_use: false } // Affine for example
    } else {
        OwnershipDim { share: Rational::one(), read_only: false, must_use: false }
    };
    ConcreteGrade { cache, ownership: own }
}

fn resolve_summary_for_optic(o: &hir::HirOptic, env: &HashMap<String, OpticSummary>) -> OpticSummary {
    match o {
        hir::HirOptic::Named { name, .. } => env.get(name).cloned().unwrap_or_else(|| default_sum(name)),
        hir::HirOptic::Seq { lhs, .. } | hir::HirOptic::Par { lhs, .. } => resolve_summary_for_optic(lhs, env),
    }
}

fn default_sum(n: &str) -> OpticSummary {
    OpticSummary {
        name: Some(n.into()),
        costate: "Entities".into(),
        focus: "f32".into(),
        lift: hir::PathLift::default(),
        get_reads: vec![],
        put_reads: vec![],
        put_writes: vec![],
        get_grade: ConcreteGrade { cache: 1, ownership: OwnershipDim { share: Rational::one(), read_only: false, must_use: false } },
        put_grade: ConcreteGrade { cache: 1, ownership: OwnershipDim { share: Rational::one(), read_only: false, must_use: false } },
        get_determinism: hir::Determinism::Pure,
        put_determinism: hir::Determinism::Pure,
        serializable: true,
        provenance: optic_syntax::Span::dummy(),
    }
}

/// Exact alias checker from ch9 (M2 gate).
pub fn alias_safe(left: &OpticSummary, right: &OpticSummary) -> Result<(), Diagnostic> {
    let left_eff: Vec<_> = left.get_reads.iter().chain(&left.put_reads).chain(&left.put_writes).cloned().collect();
    let right_eff: Vec<_> = right.get_reads.iter().chain(&right.put_reads).chain(&right.put_writes).cloned().collect();

    for (l, r) in overlapping_pairs(&left.put_writes, &right_eff) {
        if left.get_grade.ownership.read_only && right.get_grade.ownership.read_only {
            if le_share(&left.get_grade.ownership.share, &right.get_grade.ownership.share) {
                continue;
            }
        }
        // v0: same costate product acts as "partition family" for the demo
        if same_partition(&l, &r) {
            if le_share(&left.get_grade.ownership.share, &right.get_grade.ownership.share) { continue; }
        }
        return Err(diag::alias_diag(left.provenance, &vec![l.clone(), r.clone()], "put_writes overlaps effective region (including put_reads hazard)"));
    }

    for (r, l) in overlapping_pairs(&right.put_writes, &left_eff) {
        if right.get_grade.ownership.read_only && left.get_grade.ownership.read_only {
            if le_share(&right.get_grade.ownership.share, &left.get_grade.ownership.share) { continue; }
        }
        if same_partition(&r, &l) {
            if le_share(&right.get_grade.ownership.share, &left.get_grade.ownership.share) { continue; }
        }
        return Err(diag::alias_diag(right.provenance, &vec![r.clone(), l.clone()], "put_writes overlaps effective region (including put_reads hazard)"));
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

fn is_subregion(a: &str, b: &str) -> bool { a == b } // v0 field roots only
fn same_partition(a: &str, b: &str) -> bool { a == b } // v0: treat same field root product as partitionable for demo
fn le_share(a: &Rational, b: &Rational) -> bool { a.num as f64 / a.den as f64 + b.num as f64 / b.den as f64 <= 1.0 + 1e-9 }
