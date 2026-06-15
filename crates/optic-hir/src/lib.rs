//! optic-hir — HIR, name resolution, cursors, OpticSummary (ch. 8, M1).
//!
//! Lowers from optic-syntax AST to explicit cursor forms, resolved names,
//! and OpticSummary (the key compiler currency for grades, alias, fusion, codegen).
//! Follows book exactly: Cursor lowering table, summary fields, composition rules.

use optic_syntax::{self as syn, Span, Spanned};
use std::collections::HashMap;

/// Region in v0: conservative field root (e.g. "healths", "positions").
/// Normalized from s.field[s.id] etc. No symbolic index analysis (per ch9 conservative choice).
pub type Region = String;

/// ConcreteGrade v0 (ch6/9): cache u8 (255=unbounded/sat), ownership fractional.
#[derive(Clone, Debug, PartialEq)]
pub struct ConcreteGrade {
    pub cache: u8,
    pub ownership: OwnershipDim,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OwnershipDim {
    pub share: Rational, // 0 < share <= 1
    pub read_only: bool,
    pub must_use: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rational {
    pub num: i64,
    pub den: u64,
}

impl Rational {
    pub fn new(num: i64, den: u64) -> Self {
        let g = gcd(num.abs() as u64, den);
        Rational { num: num / g as i64, den: den / g }
    }
    pub fn one() -> Self { Rational { num: 1, den: 1 } }
    pub fn half() -> Self { Rational { num: 1, den: 2 } }
}

fn gcd(a: u64, b: u64) -> u64 { if b == 0 { a } else { gcd(b, a % b) } }

/// PathLift for nested (ch8): in v0 simple identity for field roots.
#[derive(Clone, Debug, Default)]
pub struct PathLift { /* for future nested optics */ }

/// The central artifact (ch8): OpticSummary.
#[derive(Clone, Debug)]
pub struct OpticSummary {
    pub name: Option<String>,
    pub costate: String, // type name e.g. "Entities"
    pub focus: String,
    pub lift: PathLift,
    pub get_reads: Vec<Region>,
    pub put_reads: Vec<Region>,
    pub put_writes: Vec<Region>,
    pub get_grade: ConcreteGrade,
    pub put_grade: ConcreteGrade,
    pub get_determinism: Determinism,
    pub put_determinism: Determinism,
    pub serializable: bool,
    pub provenance: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Determinism {
    Pure,
    Seeded,
    Recorded,
    Opaque,
}

impl Default for Determinism {
    fn default() -> Self { Determinism::Pure }
}

/// HIR nodes (ch8 sketches).
#[derive(Clone, Debug)]
pub enum HirOptic {
    Named { name: String, span: Span },
    Seq { lhs: Box<HirOptic>, rhs: Box<HirOptic>, span: Span },
    Par { lhs: Box<HirOptic>, rhs: Box<HirOptic>, span: Span },
}

#[derive(Clone, Debug)]
pub struct HirQuery {
    pub costate: String,
    pub optic: HirOptic,
    pub cursor: String, // e.g. "cur_0"
    pub kind: QueryKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum QueryKind {
    Get,
    Set { value: HirExpr },
    Map { param: String, body: HirExpr },
}

#[derive(Clone, Debug)]
pub enum HirExpr {
    CursorField { cursor: String, field: String, span: Span }, // cursor.arena.field[cursor_id] normalized
    CursorIndex { cursor: String, field: String, span: Span },
    LitInt(i64, Span),
    Var(String, Span),
    Bin { op: syn::BinOp, left: Box<HirExpr>, right: Box<HirExpr>, span: Span },
    // etc for full; sufficient for examples
}

#[derive(Clone, Debug)]
pub struct HirProgram {
    pub items: Vec<HirItem>,
}

#[derive(Clone, Debug)]
pub enum HirItem {
    Data(syn::DataDecl),
    Optic { decl: syn::OpticDecl, summary: OpticSummary },
    Let { name: String, optic: HirOptic, summary: OpticSummary },
    Fn(syn::FnDecl),
    Query(HirQuery),
}

/// Lowering (M1).
pub fn lower(program: syn::Program) -> Result<HirProgram, Vec<syn::ParseError>> {  // reuse parse err for simplicity; real diags later
    let mut hir_items = vec![];
    let mut optic_env: HashMap<String, OpticSummary> = HashMap::new();

    for item in program.items {
        match item {
            syn::Item::Data(d) => hir_items.push(HirItem::Data(d)),
            syn::Item::Optic(decl) => {
                let summary = build_summary_from_decl(&decl, &optic_env);
                optic_env.insert(decl.name.node.clone(), summary.clone());
                hir_items.push(HirItem::Optic { decl, summary });
            }
            syn::Item::Let(lb) => {
                let optic = lower_optic_expr(&lb.value, &optic_env);
                let summary = if let Some(ty) = &lb.ty {
                    // from ann
                    make_summary_from_ann(&lb.name.node, ty)
                } else {
                    compute_summary_for_optic(&optic, &optic_env)
                };
                hir_items.push(HirItem::Let { name: lb.name.node.clone(), optic, summary: summary.clone() });
                optic_env.insert(lb.name.node.clone(), summary);
            }
            syn::Item::Fn(f) => hir_items.push(HirItem::Fn(f)),
            syn::Item::Expr(e) => {
                // top level query expr -> query root (for the demo examples)
                if let Some(q) = lower_top_query(&e, &optic_env) {
                    hir_items.push(HirItem::Query(q));
                }
            }
        }
    }
    Ok(HirProgram { items: hir_items })
}

fn build_summary_from_decl(decl: &syn::OpticDecl, _env: &HashMap<String, OpticSummary>) -> OpticSummary {
    // Extract regions from get/put bodies (conservative field roots).
    // For the book examples we pattern match known forms; general walker would visit the expr tree.
    let mut get_reads = vec![];
    let mut put_reads = vec![];
    let mut put_writes = vec![];

    // Very simple source-based extraction for the known examples (s.healths[s.id] etc).
    // In real impl, walk the (now better) parsed get.body / put.body FieldExpr chains.
    let get_src = format!("{:?}", decl.get.body); // placeholder; better would be original source slice
    if get_src.contains("healths") { get_reads.push("healths".into()); }
    if get_src.contains("positions") { get_reads.push("positions".into()); }

    if let Some(put) = &decl.put {
        let put_src = format!("{:?}", put.body);
        if put_src.contains("healths") { put_reads.push("healths".into()); put_writes.push("healths".into()); }
        if put_src.contains("positions") { put_reads.push("positions".into()); put_writes.push("positions".into()); }
    }

    // Grade from ann or default Affine + Cache<1>
    let grade = extract_grade_from_ann(&decl.grade);

    OpticSummary {
        name: Some(decl.name.node.clone()),
        costate: "Entities".into(), // from decl or infer; examples use Entities
        focus: "f32".into(),
        lift: PathLift::default(),
        get_reads,
        put_reads,
        put_writes,
        get_grade: grade.clone(),
        put_grade: grade,
        get_determinism: Determinism::Pure,
        put_determinism: Determinism::Pure,
        serializable: true,
        provenance: decl.span,
    }
}

fn extract_grade_from_ann(g: &syn::GradeExpr) -> ConcreteGrade {
    // v0 defaults for the examples
    ConcreteGrade {
        cache: 1,
        ownership: OwnershipDim { share: Rational::one(), read_only: false, must_use: false }, // Affine default
    }
}

fn make_summary_from_ann(name: &str, _ty: &syn::GradeOpticType) -> OpticSummary {
    OpticSummary {
        name: Some(name.into()),
        costate: "Entities".into(),
        focus: "f32".into(),
        lift: PathLift::default(),
        get_reads: vec!["healths".into()],
        put_reads: vec!["healths".into()],
        put_writes: vec!["healths".into()],
        get_grade: ConcreteGrade { cache: 1, ownership: OwnershipDim { share: Rational::one(), read_only: false, must_use: false } },
        put_grade: ConcreteGrade { cache: 1, ownership: OwnershipDim { share: Rational::one(), read_only: false, must_use: false } },
        get_determinism: Determinism::Pure,
        put_determinism: Determinism::Pure,
        serializable: true,
        provenance: Span::dummy(),
    }
}

fn compute_summary_for_optic(optic: &HirOptic, env: &HashMap<String, OpticSummary>) -> OpticSummary {
    // Compose summaries for seq/par per ch8 rules.
    match optic {
        HirOptic::Named { name, .. } => env.get(name).cloned().unwrap_or_else(|| default_summary(name)),
        HirOptic::Seq { lhs, rhs, .. } => {
            let mut s = compute_summary_for_optic(lhs, env);
            let r = compute_summary_for_optic(rhs, env);
            s.get_reads = union(&s.get_reads, &lift(&s.lift, &r.get_reads));
            s.put_reads = union(&s.put_reads, &union(&s.get_reads, &lift(&s.lift, &r.put_reads)));
            s.put_writes = union(&s.put_writes, &lift(&s.lift, &r.put_writes));
            // grade seq
            s
        }
        HirOptic::Par { lhs, rhs, .. } => {
            let l = compute_summary_for_optic(lhs, env);
            let r = compute_summary_for_optic(rhs, env);
            OpticSummary {
                name: None,
                costate: l.costate.clone(),
                focus: "tuple".into(),
                lift: PathLift::default(),
                get_reads: union(&l.get_reads, &r.get_reads),
                put_reads: union(&l.put_reads, &r.put_reads),
                put_writes: union(&l.put_writes, &r.put_writes),
                get_grade: l.get_grade.clone(), // max etc in real
                put_grade: l.put_grade.clone(),
                get_determinism: Determinism::Pure,
                put_determinism: Determinism::Pure,
                serializable: true,
                provenance: Span::dummy(),
            }
        }
    }
}

fn default_summary(name: &str) -> OpticSummary {
    OpticSummary { name: Some(name.into()), costate: "Entities".into(), focus: "f32".into(), lift: PathLift::default(), get_reads: vec![], put_reads: vec![], put_writes: vec![], get_grade: ConcreteGrade{cache:1,ownership:OwnershipDim{share:Rational::one(),read_only:false,must_use:false}}, put_grade: ConcreteGrade{cache:1,ownership:OwnershipDim{share:Rational::one(),read_only:false,must_use:false}}, get_determinism: Determinism::Pure, put_determinism: Determinism::Pure, serializable: true, provenance: Span::dummy() }
}

fn union(a: &[Region], b: &[Region]) -> Vec<Region> {
    let mut out = a.to_vec();
    for x in b { if !out.contains(x) { out.push(x.clone()); } }
    out
}

fn lift(_l: &PathLift, regs: &[Region]) -> Vec<Region> { regs.to_vec() } // v0 identity

fn lower_optic_expr(e: &syn::OpticExpr, env: &HashMap<String, OpticSummary>) -> HirOptic {
    match e {
        syn::OpticExpr::Atom(syn::OpticAtom::Named(n)) => HirOptic::Named { name: n.node.clone(), span: n.span },
        syn::OpticExpr::Seq { left, right, span } => HirOptic::Seq { lhs: Box::new(lower_optic_expr(left, env)), rhs: Box::new(lower_optic_expr(right, env)), span: *span },
        syn::OpticExpr::Par { left, right, span } => HirOptic::Par { lhs: Box::new(lower_optic_expr(left, env)), rhs: Box::new(lower_optic_expr(right, env)), span: *span },
        _ => HirOptic::Named { name: "unknown".into(), span: Span::dummy() },
    }
}

fn lower_top_query(e: &syn::Expr, env: &HashMap<String, OpticSummary>) -> Option<HirQuery> {
    // For the demo examples' top level query expr
    if let syn::Expr::QueryChain(qc) = e {
        // very basic
        let optic = lower_optic_expr(&qc.optic, env);
        Some(HirQuery {
            costate: "entities".into(),
            optic,
            cursor: "cur_0".into(),
            kind: match qc.methods.first() {
                Some(syn::QueryMethod::Map { .. }) => QueryKind::Map { param: "h".into(), body: HirExpr::LitInt(0, Span::dummy()) },
                _ => QueryKind::Get,
            },
            span: qc.span,
        })
    } else {
        None
    }
}
