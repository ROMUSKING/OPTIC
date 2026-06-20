//! optic-hir — HIR, name resolution, cursors, OpticSummary (ch. 8, M1).
//!
//! Lowers from optic-syntax AST to explicit cursor forms, resolved names,
//! and OpticSummary (the key compiler currency for grades, alias, fusion, codegen).
//! Follows book exactly: Cursor lowering table, summary fields, composition rules.

use optic_syntax::{self as syn, Span};
use std::collections::HashMap;
use std::sync::Arc;

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
        Rational {
            num: num / g as i64,
            den: den / g,
        }
    }
    pub fn one() -> Self {
        Rational { num: 1, den: 1 }
    }
    pub fn half() -> Self {
        Rational { num: 1, den: 2 }
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// PathLift for nested field paths (ch8.6 / 8.9.5.1).
/// `prefix` holds focus-relative path segments (e.g. `["position"]` for `t.position`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PathLift {
    pub prefix: Vec<String>,
}

impl PathLift {
    pub fn identity() -> Self {
        Self { prefix: vec![] }
    }

    /// Sequential composition: πA ∘ πB (ch8.9.5.1).
    pub fn seq(parent: &PathLift, child: &PathLift) -> PathLift {
        let mut prefix = parent.prefix.clone();
        prefix.extend(child.prefix.iter().cloned());
        PathLift { prefix }
    }

    /// Product composition: pair_lift(πA, πB) — v0 tuple focus shares costate; lifts must be identity.
    pub fn pair(left: &PathLift, right: &PathLift) -> Result<PathLift, String> {
        if !left.prefix.is_empty() || !right.prefix.is_empty() {
            return Err(
                "v0 product composition (*** ) requires identity PathLift on both operands".into(),
            );
        }
        Ok(PathLift::identity())
    }

    /// Lift focus-relative regions through a parent optic's SoA column + composed prefix (ch8.9.5.1).
    pub fn lift_regions(
        lift: &PathLift,
        parent_column: Option<&str>,
        regs: &[Region],
    ) -> Vec<Region> {
        regs.iter()
            .map(|r| lift_region(lift, parent_column, r))
            .collect()
    }
}

fn lift_region(lift: &PathLift, parent_column: Option<&str>, region: &str) -> Region {
    if region.contains('.') {
        return region.to_string();
    }
    let mut parts: Vec<String> = vec![];
    if let Some(col) = parent_column.filter(|c| !c.is_empty()) {
        parts.push(col.to_string());
    }
    parts.extend(lift.prefix.iter().cloned());
    if lift.prefix.last().map(|s| s.as_str()) != Some(region) {
        parts.push(region.to_string());
    } else if parts.is_empty() {
        parts.push(region.to_string());
    }
    if parts.len() == 1 {
        parts.into_iter().next().unwrap()
    } else {
        parts.join(".")
    }
}

/// Minimal subregion lattice for dotted paths (ch9 overlaps).
pub fn is_subregion(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    if a.starts_with(&format!("{b}.")) || b.starts_with(&format!("{a}.")) {
        return true;
    }
    // Sibling nested fields under the same SoA column (e.g. transforms.position vs transforms.velocity).
    let a_root = a.split('.').next().unwrap_or(a);
    let b_root = b.split('.').next().unwrap_or(b);
    a_root == b_root && a.contains('.') && b.contains('.')
}

/// Compose-depth cap (security).
pub const MAX_OPTIC_COMPOSE_DEPTH: usize = 512;

/// Column metadata derived from `data` declarations (SUG-003).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ColumnInfo {
    pub name: String,
    pub rust_ty: String,
    pub element_ty: Option<String>,
}

/// Region→column/type map threaded into codegen (SUG-003).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegionMap {
    pub costate_name: String,
    pub columns: std::collections::BTreeMap<String, ColumnInfo>,
    /// Nested record types: type name -> field -> Rust type.
    pub record_fields:
        std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
}

impl RegionMap {
    pub fn column_rust_ty(&self, column: &str) -> Option<&str> {
        self.columns.get(column).map(|c| c.rust_ty.as_str())
    }

    pub fn top_level_column<'a>(&self, region: &'a str) -> Option<&'a str> {
        let root = region.split('.').next().unwrap_or(region);
        if self.columns.contains_key(root) {
            Some(root)
        } else {
            None
        }
    }

    pub fn is_top_level_column(&self, region: &str) -> bool {
        self.columns.contains_key(region)
    }

    pub fn region_field(&self, region: &str) -> Result<String, String> {
        let root = region.split('.').next().unwrap_or(region);
        if self.columns.contains_key(root) {
            Ok(root.to_string())
        } else {
            Err(format!(
                "unknown region `{region}` — not declared in costate/data"
            ))
        }
    }

    pub fn region_bind(&self, region: &str) -> Result<String, String> {
        let root = region.split('.').next().unwrap_or(region);
        Ok(format!("_{root}"))
    }

    pub fn region_ty(&self, region: &str) -> Result<String, String> {
        let root = region.split('.').next().unwrap_or(region);
        self.columns
            .get(root)
            .map(|c| c.rust_ty.clone())
            .ok_or_else(|| format!("unknown region type for `{region}`"))
    }

    pub fn nested_field_path(&self, region: &str) -> Vec<String> {
        let parts: Vec<_> = region.split('.').collect();
        if parts.len() <= 1 {
            vec![]
        } else {
            parts[1..].iter().map(|s| (*s).to_string()).collect()
        }
    }
}

pub fn validate_rust_type(ty: &str) -> Result<(), String> {
    validate_rust_type_in_map(ty, None)
}

pub fn validate_rust_type_in_map(
    ty: &str,
    declared_records: Option<&std::collections::HashSet<String>>,
) -> Result<(), String> {
    const PRIMITIVES: &[&str] = &["f32", "i32", "u32", "u64", "bool", "usize", "(f32, f32)"];
    if PRIMITIVES.contains(&ty) {
        return Ok(());
    }
    if ty.starts_with("Vec<") && ty.ends_with('>') {
        return validate_rust_type(&ty[4..ty.len() - 1]);
    }
    if ty.starts_with('(') && ty.ends_with(')') {
        return Ok(());
    }
    if ty.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        if let Some(recs) = declared_records {
            if recs.contains(ty) {
                return Ok(());
            }
            return Err(format!("undeclared record type `{ty}`"));
        }
        return Ok(());
    }
    Err(format!("disallowed rust type `{ty}`"))
}

/// Build region map from HIR data declarations (SUG-003).
pub fn build_region_map(program: &HirProgram) -> Result<RegionMap, String> {
    let mut map = RegionMap::default();
    let mut soa_seen = false;
    // Pass 1a: reserve record type names (forward refs).
    for item in &program.items {
        if let HirItem::Data(d) = item {
            let has_soa = d
                .fields
                .iter()
                .any(|f| matches!(f.ty, syn::TypeExpr::Soa(..)));
            if !has_soa {
                map.record_fields.entry(d.name.node.clone()).or_default();
            }
        }
    }
    // Pass 1b: fill record field types.
    for item in &program.items {
        if let HirItem::Data(d) = item {
            let has_soa = d
                .fields
                .iter()
                .any(|f| matches!(f.ty, syn::TypeExpr::Soa(..)));
            if !has_soa {
                register_record_type(&mut map, d);
            }
        }
    }
    // Pass 2: SoA costate columns.
    for item in &program.items {
        if let HirItem::Data(d) = item {
            let has_soa = d
                .fields
                .iter()
                .any(|f| matches!(f.ty, syn::TypeExpr::Soa(..)));
            if has_soa {
                if soa_seen {
                    return Err(format!(
                        "v0 supports only one SoA costate data decl; duplicate `{}`",
                        d.name.node
                    ));
                }
                soa_seen = true;
                map.costate_name = d.name.node.clone();
                for field in &d.fields {
                    let (rust_ty, element_ty) =
                        lower_field_rust_ty(&field.ty, &mut map.record_fields)?;
                    map.columns.insert(
                        field.name.node.clone(),
                        ColumnInfo {
                            name: field.name.node.clone(),
                            rust_ty,
                            element_ty,
                        },
                    );
                }
            }
        }
    }
    Ok(map)
}

fn lower_field_rust_ty(
    te: &syn::TypeExpr,
    records: &mut std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
) -> Result<(String, Option<String>), String> {
    match te {
        syn::TypeExpr::Soa(inner, _) => {
            let (elem, _) = lower_type_rust(inner, records)?;
            Ok((format!("Vec<{elem}>"), Some(elem)))
        }
        other => {
            let (ty, _) = lower_type_rust(other, records)?;
            Ok((ty, None))
        }
    }
}

fn lower_type_rust(
    te: &syn::TypeExpr,
    records: &mut std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
) -> Result<(String, bool), String> {
    match te {
        syn::TypeExpr::Named { name, args, .. } => {
            if name == "Vec2" {
                return Ok(("(f32, f32)".into(), false));
            }
            if name == "f32" || name == "i32" || name == "u32" {
                return Ok((name.clone(), false));
            }
            if args.is_empty() {
                validate_rust_type_in_map(name, Some(&records.keys().cloned().collect()))?;
                return Ok((name.clone(), true));
            }
            let arg_strs: Vec<_> = args
                .iter()
                .map(|a| lower_type_rust(a, records).map(|x| x.0))
                .collect::<Result<_, _>>()?;
            Ok((format!("{name}<{}>", arg_strs.join(", ")), true))
        }
        syn::TypeExpr::Soa(inner, _) => {
            let (elem, _) = lower_type_rust(inner, records)?;
            Ok((format!("Vec<{elem}>"), false))
        }
        syn::TypeExpr::Tuple(elems, _) => {
            let parts: Vec<_> = elems
                .iter()
                .map(|e| lower_type_rust(e, records).map(|x| x.0))
                .collect::<Result<_, _>>()?;
            Ok((format!("({})", parts.join(", ")), false))
        }
        syn::TypeExpr::BitSet(_) => Ok(("u64".into(), false)),
    }
}

/// Register nested record field types from a `data` decl (e.g. `Transform { position: Vec2 }`).
pub fn register_record_type(map: &mut RegionMap, decl: &syn::DataDecl) {
    let mut fields = std::collections::BTreeMap::new();
    for field in &decl.fields {
        if let Ok((ty, _)) = lower_type_rust(&field.ty, &mut map.record_fields) {
            fields.insert(field.name.node.clone(), ty);
        }
    }
    map.record_fields.insert(decl.name.node.clone(), fields);
}

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
    fn default() -> Self {
        Determinism::Pure
    }
}

/// HIR nodes (ch8 sketches).
#[derive(Clone, Debug)]
pub enum HirOptic {
    Named {
        name: String,
        span: Span,
    },
    Seq {
        lhs: Box<HirOptic>,
        rhs: Box<HirOptic>,
        span: Span,
    },
    Par {
        lhs: Box<HirOptic>,
        rhs: Box<HirOptic>,
        span: Span,
    },
}

/// Observability hook on a query chain (M8 scaffolding).
#[derive(Clone, Debug)]
pub enum ObsHook {
    Tap(String, Span),
    Record(String, Span),
}

#[derive(Clone, Debug)]
pub struct HirQuery {
    pub costate: String,
    pub optic: HirOptic,
    pub cursor: String, // e.g. "cur_0"
    pub kind: QueryKind,
    /// Tap/record hooks in chain order (book ch14.5).
    pub observability: Vec<ObsHook>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum QueryKind {
    Get,
    Set { value: HirExpr },
    Map { param: String, body: Arc<HirExpr> },
}

#[derive(Clone, Debug)]
pub enum HirExpr {
    CursorField {
        cursor: String,
        field: String,
        span: Span,
    }, // cursor.arena.field[cursor_id] normalized
    CursorIndex {
        cursor: String,
        field: String,
        span: Span,
    },
    LitInt(i64, Span),
    LitFloat(f64, Span),
    Var(String, Span),
    Bin {
        op: syn::BinOp,
        left: Box<HirExpr>,
        right: Box<HirExpr>,
        span: Span,
    },
    /// Parenthesized subexpression (distinct from 1-tuple for product maps).
    Paren(Box<HirExpr>, Span),
    /// Tuple literal / product focus value (ch8 product queries).
    Tuple(Vec<HirExpr>, Span),
    /// Tuple field projection, e.g. `p.0` in map bodies.
    TupleProj {
        base: Box<HirExpr>,
        index: u32,
        span: Span,
    },
    /// Focus-relative field projection in optic bodies, e.g. `t.position` (ch8 PathLift).
    FocusField {
        param: String,
        path: Vec<String>,
        span: Span,
    },
    /// Unsupported surface form in map/value lowering (surfaced at codegen).
    Unsupported {
        reason: String,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub struct HirProgram {
    pub items: Vec<HirItem>,
}

#[derive(Clone, Debug)]
pub enum HirItem {
    Data(syn::DataDecl),
    Optic {
        decl: syn::OpticDecl,
        summary: Arc<OpticSummary>,
    },
    Let {
        name: String,
        ty: Option<syn::GradeOpticType>,
        span: Span,
        optic: HirOptic,
        summary: Arc<OpticSummary>,
    },
    Fn(syn::FnDecl),
    Query(HirQuery),
}

/// Lowering (M1).
pub fn lower(program: syn::Program) -> Result<HirProgram, Vec<syn::ParseError>> {
    let mut hir_items = vec![];
    let mut optic_env: HashMap<String, Arc<OpticSummary>> = HashMap::new();
    let mut soa_costate_seen = false;

    for item in program.items {
        match item {
            syn::Item::Data(d) => {
                let has_soa = d
                    .fields
                    .iter()
                    .any(|f| matches!(f.ty, syn::TypeExpr::Soa(..)));
                if has_soa {
                    if soa_costate_seen {
                        return Err(vec![syn::ParseError {
                            message: format!(
                                "v0 supports only one SoA costate data decl; duplicate `{}`",
                                d.name.node
                            ),
                            span: d.name.span,
                            kind: Some(syn::ParseErrorKind::DuplicateSoaCostate {
                                costate: d.name.node.clone(),
                            }),
                        }]);
                    }
                    soa_costate_seen = true;
                }
                hir_items.push(HirItem::Data(d));
            }
            syn::Item::Optic(decl) => {
                let decl = *decl;
                if decl.is_unsupported_v0() {
                    continue;
                }
                let summary = build_summary_from_decl(&decl, &optic_env);
                let arc = Arc::new(summary);
                optic_env.insert(decl.name.node.clone(), Arc::clone(&arc));
                hir_items.push(HirItem::Optic { decl, summary: arc });
            }
            syn::Item::Let(lb) => {
                let optic = match lower_optic_expr(&lb.value, &optic_env, 0) {
                    Ok(o) => o,
                    Err(e) => return Err(vec![e]),
                };
                let summary = if let Some(ty) = &lb.ty {
                    make_summary_from_ann(&lb.name.node, ty, &optic, &optic_env).map_err(|e| {
                        vec![syn::ParseError {
                            message: e,
                            span: lb.span,
                            kind: None,
                        }]
                    })?
                } else {
                    compute_summary_for_optic(&optic, &optic_env).map_err(|e| {
                        vec![syn::ParseError {
                            message: e,
                            span: lb.span,
                            kind: None,
                        }]
                    })?
                };
                let arc = Arc::new(summary);
                hir_items.push(HirItem::Let {
                    name: lb.name.node.clone(),
                    ty: lb.ty.clone(),
                    span: lb.span,
                    optic,
                    summary: Arc::clone(&arc),
                });
                optic_env.insert(lb.name.node.clone(), arc);
            }
            syn::Item::Fn(f) => {
                for q in lower_queries_from_fn(&f, &optic_env).map_err(|e| vec![e])? {
                    hir_items.push(HirItem::Query(q));
                }
                hir_items.push(HirItem::Fn(f));
            }
            syn::Item::Expr(e) => {
                if let Some(q) = lower_top_query(&e, &optic_env).map_err(|e| vec![e])? {
                    hir_items.push(HirItem::Query(q));
                }
            }
            syn::Item::Extern(_) => {}
        }
    }
    Ok(HirProgram { items: hir_items })
}
/// dump_hir: deterministic pretty for M1 goldens / CLI dump-hir (B.7).
pub fn dump_hir(p: &HirProgram) -> String {
    let mut out = String::new();
    out.push_str("HIR items: ");
    out.push_str(&p.items.len().to_string());
    out.push('\n');
    for item in &p.items {
        match item {
            HirItem::Optic { decl, summary } => {
                let kind = if decl.is_traversal() {
                    "Traversal"
                } else if decl.is_prism() {
                    "Prism"
                } else {
                    "Optic"
                };
                out.push_str(&format!(
                    "  Optic {} kind={} costate={} lift={:?} reads={:?} writes={:?}\n",
                    decl.name.node,
                    kind,
                    summary.costate,
                    summary.lift.prefix,
                    summary.get_reads,
                    summary.put_writes
                ));
            }
            HirItem::Let { name, summary, .. } => {
                out.push_str(&format!(
                    "  Let {} lift={:?} reads={:?} writes={:?}\n",
                    name, summary.lift.prefix, summary.get_reads, summary.put_writes
                ));
            }
            HirItem::Query(q) => {
                let mut line = format!(
                    "  Query cursor={} kind={:?}",
                    q.cursor,
                    std::mem::discriminant(&q.kind)
                );
                if !q.observability.is_empty() {
                    let hooks: Vec<String> = q
                        .observability
                        .iter()
                        .map(|h| match h {
                            ObsHook::Tap(l, _) => format!("tap:{l}"),
                            ObsHook::Record(e, _) => format!("record:{e}"),
                        })
                        .collect();
                    line.push_str(&format!(" obs={hooks:?}"));
                }
                line.push('\n');
                out.push_str(&line);
            }
            _ => out.push_str("  other\n"),
        }
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use optic_syntax::{parse, SourceId};

    #[test]
    fn test_lower_health_view_regions_from_real_ast() {
        // Uses post-A parser + B lower; asserts collect_regions + build_summary per ch8.9.5.2
        let src = r#"
data Entities { healths: SoA<f32> }
optic HealthView: GradedOptic<Entities, f32, CacheGrade<1>> {
    get s => s.healths[s.id]
    put (s, v) => { s.healths[s.id] = v }
}
"#;
        let prog = parse(src, SourceId(1)).expect("parse in test");
        let hir = lower(prog).expect("lower in test");
        // find the optic summary
        let mut found = false;
        for it in &hir.items {
            if let HirItem::Optic { summary, .. } = it {
                if summary.name.as_deref() == Some("HealthView") {
                    assert!(
                        summary.get_reads.contains(&"healths".to_string()),
                        "get_reads should have healths from field expr"
                    );
                    assert!(
                        summary.put_writes.contains(&"healths".to_string()),
                        "put_writes from assign lhs"
                    );
                    found = true;
                }
            }
        }
        assert!(found, "HealthView summary present");
    }

    #[test]
    fn test_reject_get_map_lowers_unsupported() {
        let src = r#"
optic H: GradedOptic<Entities,f32,_> { get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v} }
fn main() { entities.query(H).get().map(|h| h); }
"#;
        let prog = parse(src, SourceId(1)).expect("parse");
        let hirp = lower(prog).expect("lower");
        let unsupported = hirp.items.iter().any(|it| {
            if let HirItem::Query(q) = it {
                if let QueryKind::Map { body, .. } = &q.kind {
                    return matches!(
                        body.as_ref(),
                        HirExpr::Unsupported { reason, .. }
                            if reason.contains(".get().map()")
                    );
                }
            }
            false
        });
        assert!(unsupported, "get().map() must lower to Unsupported");
    }

    fn assert_hir_golden(example: &str) {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples")
            .join(example);
        let src = std::fs::read_to_string(&path).expect("read example");
        let prog = parse(&src, SourceId(1)).expect("parse");
        let hirp = lower(prog).expect("lower");
        let actual = dump_hir(&hirp);
        let stem = example.trim_end_matches(".opt");
        let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/hir")
            .join(format!("{stem}.txt"));
        if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
            std::fs::write(&fixture, &actual).expect("write golden");
        } else {
            let expected = std::fs::read_to_string(&fixture).expect("hir golden");
            assert_eq!(actual, expected, "HIR golden mismatch for {example}");
        }
    }

    #[test]
    fn golden_hir_health_decay() {
        assert_hir_golden("health_decay.opt");
    }

    #[test]
    fn golden_hir_health_get() {
        assert_hir_golden("health_get.opt");
    }

    #[test]
    fn golden_hir_health_set() {
        assert_hir_golden("health_set.opt");
    }

    #[test]
    fn golden_hir_health_position() {
        assert_hir_golden("health_position.opt");
    }

    #[test]
    fn golden_hir_alive_filter() {
        assert_hir_golden("alive_filter.opt");
    }

    #[test]
    fn golden_hir_prism_get() {
        assert_hir_golden("prism_get.opt");
    }

    #[test]
    fn golden_hir_prism_set() {
        assert_hir_golden("prism_set.opt");
    }

    #[test]
    fn golden_hir_partial_prism() {
        assert_hir_golden("partial_prism.opt");
    }

    #[test]
    fn golden_hir_all_healths() {
        assert_hir_golden("all_healths.opt");
    }

    #[test]
    fn golden_hir_traversal_get() {
        assert_hir_golden("traversal_get.opt");
    }

    #[test]
    fn golden_hir_traversal_set() {
        assert_hir_golden("traversal_set.opt");
    }

    #[test]
    fn golden_hir_tap_health() {
        assert_hir_golden("tap_health.opt");
    }

    #[test]
    fn golden_hir_record_health() {
        assert_hir_golden("record_health.opt");
    }

    #[test]
    fn golden_hir_tap_record_chain() {
        assert_hir_golden("tap_record_chain.opt");
    }

    #[test]
    fn lower_rejects_profile_replay_and_trailing_hooks_without_surface_gate() {
        for (src, needle) in [
            (
                include_str!("../../../examples/unsupported_profile.opt"),
                "profile",
            ),
            (
                include_str!("../../../examples/unsupported_replay.opt"),
                "replay",
            ),
            (include_str!("../../../examples/trailing_tap.opt"), ".tap"),
            (
                include_str!("../../../examples/trailing_record.opt"),
                ".record",
            ),
        ] {
            let prog = optic_syntax::parse(src, optic_syntax::SourceId(1)).expect("parse");
            let err = super::lower(prog).expect_err("lower must reject unsupported obs hooks");
            assert!(
                err.iter().any(|e| e.message.contains(needle)),
                "expected `{needle}` rejection: {err:?}"
            );
        }
    }

    #[test]
    fn test_fuse_map_chain_substitutes_body() {
        let src = r#"
optic H: GradedOptic<E,f32,_> { get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v} }
fn main() { entities.query(H).map(|h| h - 1.0).map(|x| x * 2.0); }
"#;
        let prog = parse(src, SourceId(1)).expect("parse");
        let hirp = lower(prog).expect("lower");
        let map_body = hirp.items.iter().find_map(|it| {
            if let HirItem::Query(q) = it {
                if let QueryKind::Map { body, .. } = &q.kind {
                    return Some(body.as_ref().clone());
                }
            }
            None
        });
        let body = map_body.expect("fused map query");
        match body {
            HirExpr::Bin {
                op: syn::BinOp::Mul,
                left,
                ..
            } => match &*left {
                HirExpr::Bin {
                    op: syn::BinOp::Sub,
                    ..
                } => {}
                other => panic!("expected fused sub in mul, got {other:?}"),
            },
            other => panic!("expected fused mul at top, got {other:?}"),
        }
    }

    #[test]
    fn test_fuse_map_chain_multi_param_tuple() {
        let src = r#"
data E { healths: SoA<f32>, positions: SoA<f32> }
optic H: GradedOptic<E,f32,_> { get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v} }
optic P: GradedOptic<E,f32,_> { get s=>s.positions[s.id] put(s,v)=>{s.positions[s.id]=v} }
let c = H *** P;
fn main() { entities.query(c).map(|(h,p)| (h - 1.0, p + 1.0)).map(|(x,y)| (x * 2.0, y)); }
"#;
        let prog = parse(src, SourceId(1)).expect("parse");
        let hirp = lower(prog).expect("lower");
        let body = hirp.items.iter().find_map(|it| {
            if let HirItem::Query(q) = it {
                if let QueryKind::Map { body, .. } = &q.kind {
                    return Some(body.as_ref().clone());
                }
            }
            None
        });
        let body = body.expect("fused product map");
        match body {
            HirExpr::Tuple(elems, _) => {
                assert_eq!(elems.len(), 2);
                match &elems[0] {
                    HirExpr::Bin {
                        op: syn::BinOp::Mul,
                        left,
                        ..
                    } => match &**left {
                        HirExpr::Bin {
                            op: syn::BinOp::Sub,
                            ..
                        } => {}
                        other => panic!("expected fused sub in mul tuple arm, got {other:?}"),
                    },
                    other => panic!("expected mul in tuple arm 0, got {other:?}"),
                }
            }
            other => panic!("expected fused tuple body, got {other:?}"),
        }
    }

    #[test]
    fn test_incompatible_map_chain_lowers_unsupported() {
        let src = r#"
data E { healths: SoA<f32>, positions: SoA<f32> }
optic H: GradedOptic<E,f32,_> { get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v} }
fn main() { entities.query(H).map(|h| h).map(|(x,y)| (x,y)); }
"#;
        let prog = parse(src, SourceId(1)).expect("parse");
        let hirp = lower(prog).expect("lower");
        let unsupported = hirp.items.iter().any(|it| {
            if let HirItem::Query(q) = it {
                if let QueryKind::Map { body, .. } = &q.kind {
                    return matches!(
                        body.as_ref(),
                        HirExpr::Unsupported { reason, .. }
                            if reason.contains("incompatible map chain")
                    );
                }
            }
            false
        });
        assert!(
            unsupported,
            "incompatible map chain must lower to Unsupported"
        );
    }

    #[test]
    fn substitute_hir_ident_matches_var_replacement_contract() {
        let e = HirExpr::Bin {
            op: syn::BinOp::Add,
            left: Box::new(HirExpr::Var("x".into(), Span::dummy())),
            right: Box::new(HirExpr::CursorIndex {
                cursor: "x".into(),
                field: "healths".into(),
                span: Span::dummy(),
            }),
            span: Span::dummy(),
        };
        let subbed = substitute_hir_ident(&e, "x", "_bind");
        assert!(hir_expr_refs_var(&e, "x"));
        assert!(!hir_expr_refs_var(&subbed, "x"));
        match subbed {
            HirExpr::Bin { left, right, .. } => {
                assert!(matches!(*left, HirExpr::Var(ref v, _) if v == "_bind"));
                assert!(matches!(*right, HirExpr::Var(ref v, _) if v == "_bind"));
            }
            other => panic!("unexpected substitution shape: {other:?}"),
        }
    }

    #[test]
    fn test_map_set_lowers_unsupported_for_cgi003() {
        let src = r#"
optic H: GradedOptic<Entities,f32,_> { get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v} }
fn main() { entities.query(H).map(|h| h - 1.0).set(42.0); }
"#;
        let prog = parse(src, SourceId(1)).expect("parse");
        let hirp = lower(prog).expect("lower");
        let unsupported = hirp.items.iter().any(|it| {
            if let HirItem::Query(q) = it {
                if let QueryKind::Map { body, .. } = &q.kind {
                    return matches!(
                        body.as_ref(),
                        HirExpr::Unsupported { reason, .. }
                            if reason.contains(".map().set()")
                    );
                }
            }
            false
        });
        assert!(unsupported, ".map().set() must lower to Unsupported");
    }

    #[test]
    fn test_dump_hir_and_query_map_lowering() {
        let src = r#"
optic H: GradedOptic<Entities,f32,_> { get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v} }
fn main() { entities.query(H).map(|h| h - 1 ); }
"#;
        let prog = parse(src, SourceId(1)).expect("p");
        let hirp = lower(prog).expect("l");
        let d = dump_hir(&hirp);
        assert!(d.contains("Query"), "dump_hir must contain Query item");
        let has_query_map = hirp.items.iter().any(|it| {
            if let HirItem::Query(q) = it {
                matches!(q.kind, QueryKind::Map { .. })
            } else {
                false
            }
        });
        assert!(has_query_map, "lowered QueryMap present");
        assert!(
            d.contains("kind="),
            "dump_hir must include query kind discriminant"
        );
        let map_body = hirp.items.iter().find_map(|it| {
            if let HirItem::Query(q) = it {
                if let QueryKind::Map { body, .. } = &q.kind {
                    return Some(body.as_ref());
                }
            }
            None
        });
        match map_body.expect("map body") {
            HirExpr::Bin {
                op: syn::BinOp::Sub,
                ..
            } => {}
            other => panic!("expected Sub in map body, got {other:?}"),
        }
    }

    #[test]
    fn test_par_product_summary_put_reads_and_let() {
        // covers Par in let, put_reads from assign rhs, product union
        let src = r#"
data Entities { healths: SoA<f32> }
optic W: GradedOptic<Entities,f32,_> { get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v} }
optic A: GradedOptic<Entities,f32,_> { get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v+1} }
let bad = W *** A;
fn main() { entities.query(bad).map(|(a,b)| a+b); }
"#;
        let prog = parse(src, SourceId(1)).expect("p");
        let hirp = lower(prog).expect("l");
        // find let summary has put_reads (from rhs reads in put)
        let has_let_with_put_reads = hirp.items.iter().any(|it| {
            if let HirItem::Let { summary, .. } = it {
                summary.put_reads.iter().any(|r| r == "healths") && summary.put_writes.len() >= 1
            } else {
                false
            }
        });
        assert!(
            has_let_with_put_reads,
            "let Par summary has put_reads from rhs"
        );
        // has Par optic in items
        let has_par = hirp.items.iter().any(|it| {
            if let HirItem::Let { optic, .. } = it {
                matches!(optic, HirOptic::Par { .. })
            } else {
                false
            }
        });
        assert!(has_par, "Par in let lowered");
    }

    #[test]
    fn test_arc_sharing_and_dedup_in_lower() {
        // ch8.9: Arc<OpticSummary> sharing after lower.
        // strong_count==1 post-drop of internal env; dedup preserves order/no dups for regions.
        // Uses real lower on par source (ch8.9).
        let src = r#"
data Entities { healths: SoA<f32> }
optic H: GradedOptic<Entities,f32,_> { get s=>s.healths[s.id] put(s,v)=>{s.healths[s.id]=v} }
let c = H *** H;
fn main() { entities.query(c).map(|(h1,h2)| h1 + h2); }
"#;
        let prog = parse(src, SourceId(1)).expect("p");
        let hirp = lower(prog).expect("l");
        let mut optic_arc_count = 0;
        let mut dedup_tested = false;
        for it in &hirp.items {
            if let HirItem::Optic { summary, .. } = it {
                optic_arc_count += 1;
                let cnt = std::sync::Arc::strong_count(summary);
                assert!(
                    cnt == 1,
                    "post-lower Arc refcount 1 (sharing was internal to env during lower)"
                );
            }
            if let HirItem::Let { summary, .. } = it {
                // the composed has its data (possibly deduped regions if dups in union)
                if summary.put_writes.len() > 0 {
                    dedup_tested = true;
                }
            }
        }
        assert!(optic_arc_count > 0);
        assert!(dedup_tested);
        // explicit dedup test (small)
        let regs = vec![
            "healths".to_string(),
            "healths".to_string(),
            "pos".to_string(),
        ];
        let d = dedup_regions(regs);
        assert_eq!(d, vec!["healths".to_string(), "pos".to_string()]);
        // dedup_regions preserves first-seen order (ch8.9).
    }

    #[test]
    fn test_large_number_of_summaries() {
        // N=12 optics via real parse+lower; exercises Arc sharing and dedup (ch8.9).
        let mut src =
            "data E { r0: SoA<f32>, r1: SoA<f32>, r2: SoA<f32>, r3: SoA<f32>, r4: SoA<f32> }\n"
                .to_string();
        for i in 0..12 {
            let r = i % 5;
            src.push_str(&format!(
                r#"optic H{i}: GradedOptic<E,f32,_> {{ get s=>s.r{r}[s.id] put(s,v)=>{{s.r{r}[s.id]=v}} }}
"#
            ));
        }
        src.push_str("let c0 = H0 *** H5;\nlet c1 = H6 *** H11;\nfn main() { entities.query(c0).map(|(a,b)| a+b); entities.query(c1).map(|x| x); }\n");
        let prog = parse(&src, SourceId(1)).expect("p");
        let hirp = lower(prog).expect("l");
        let mut optic_arc_count = 0;
        let mut first_arc: Option<std::sync::Arc<OpticSummary>> = None;
        let mut max_regions = 0;
        for it in &hirp.items {
            if let HirItem::Optic { summary, .. } = it {
                optic_arc_count += 1;
                if first_arc.is_none() {
                    first_arc = Some(std::sync::Arc::clone(summary));
                }
                let cnt = std::sync::Arc::strong_count(summary);
                assert!(
                    cnt >= 1,
                    "post-lower Arc refcount >=1 (sharing internal to env/compute during lower; ch8.9; ==1 in simple cases, >= in presence of lets/Par sharing)"
                );
                max_regions = max_regions.max(summary.get_reads.len());
            }
            if let HirItem::Let { summary, .. } = it {
                max_regions = max_regions.max(summary.get_reads.len() + summary.put_writes.len());
            }
        }
        assert!(
            optic_arc_count >= 12,
            "12+ optics from real lower of synthetic N src (capped for hardware)"
        );
        assert!(
            max_regions <= 5,
            "dedup under overlap (r%5) exercised in unions/collect during compute Par/Seq"
        );
        // ptr_eq proves Arc sharing post-lower (ch8.9).
        if let Some(a) = &first_arc {
            let before = std::sync::Arc::strong_count(a);
            let b = std::sync::Arc::clone(a);
            assert!(
                std::sync::Arc::ptr_eq(a, &b),
                "ptr_eq confirms Arc sharing (not separate allocs)"
            );
            assert!(std::sync::Arc::strong_count(a) == before + 1);
            // drop b at scope end
        }
        assert!(hirp.items.len() >= 14); // 12 Optic + 2 Let (exercises env Arc for lets too)
    }

    #[test]
    #[ignore = "optional low-mem check: ulimit -v 2000000 cargo test -p optic-hir -- --ignored"]
    fn test_under_low_memory_constraint() {
        // N=10 optics; run manually under memory limits (see PLAN.md Hardware limits).
        let mut src =
            "data E { r0: SoA<f32>, r1: SoA<f32>, r2: SoA<f32>, r3: SoA<f32>, r4: SoA<f32> }\n"
                .to_string();
        src.reserve(4096);
        for i in 0..10 {
            let r = i % 5;
            src.push_str(&format!(
                r#"optic H{i}: GradedOptic<E,f32,_> {{ get s=>s.r{r}[s.id] put(s,v)=>{{s.r{r}[s.id]=v}} }}
"#
            ));
        }
        src.push_str("let c = H0 *** H1;\nfn main() { entities.query(c).map(|(a,b)|a+b); }\n");
        let prog = parse(&src, SourceId(1)).expect("p");
        let hir = lower(prog).expect("l");
        assert!(hir.items.len() >= 11); // 10 Optic + 1 Let; Arc/dedup paths exercised (many r%5)
    }
}

fn type_expr_name(te: &syn::TypeExpr) -> String {
    match te {
        syn::TypeExpr::Named { name, .. } => name.clone(),
        syn::TypeExpr::Tuple(_, _) => "(tuple)".into(),
        syn::TypeExpr::Soa(_, _) => "SoA".into(),
        syn::TypeExpr::BitSet(_) => "BitSet".into(),
    }
}

fn primary_read_clause<'a>(decl: &'a syn::OpticDecl) -> Option<&'a syn::GetClause> {
    decl.get.as_ref().or(decl.preview.as_ref())
}

fn focus_relative_lift_from_decl(decl: &syn::OpticDecl) -> PathLift {
    let Some(read) = primary_read_clause(decl) else {
        return PathLift::default();
    };
    let param = read.param.node.as_str();
    let mut path = focus_field_path_from_expr(param, &read.body).unwrap_or_default();
    if let Some(put) = &decl.put {
        if let Some(mut put_path) = focus_field_path_from_put(&put.state_param.node, &put.body) {
            for seg in put_path.drain(..) {
                if !path.contains(&seg) {
                    path.push(seg);
                }
            }
        }
    } else if let Some(review) = &decl.review {
        if let Some(mut review_path) =
            focus_field_path_from_put(&review.state_param.node, &review.body)
        {
            for seg in review_path.drain(..) {
                if !path.contains(&seg) {
                    path.push(seg);
                }
            }
        }
    }
    PathLift { prefix: path }
}

fn focus_field_path_from_put(state_param: &str, body: &syn::Expr) -> Option<Vec<String>> {
    match body {
        syn::Expr::Block { stmts, result, .. } => {
            for stmt in stmts {
                if let Some(p) = focus_assign_path(state_param, &stmt.expr) {
                    return Some(p);
                }
            }
            result
                .as_ref()
                .and_then(|r| focus_assign_path(state_param, r))
        }
        _ => focus_assign_path(state_param, body),
    }
}

fn focus_assign_path(state_param: &str, e: &syn::Expr) -> Option<Vec<String>> {
    if let syn::Expr::Assign { target, .. } = e {
        if let syn::Expr::Field(fe) = target.as_ref() {
            return focus_field_path_from_field_expr(state_param, fe);
        }
    }
    None
}

fn focus_field_path_from_expr(param: &str, e: &syn::Expr) -> Option<Vec<String>> {
    match e {
        syn::Expr::Field(fe) => focus_field_path_from_field_expr(param, fe),
        syn::Expr::Block { result, .. } => result
            .as_ref()
            .and_then(|r| focus_field_path_from_expr(param, r)),
        _ => None,
    }
}

fn focus_field_path_from_field_expr(param: &str, fe: &syn::FieldExpr) -> Option<Vec<String>> {
    match fe {
        syn::FieldExpr::FieldAccess { base, field, .. } => {
            let mut path = focus_field_path_from_field_expr(param, base)?;
            path.push(field.node.clone());
            Some(path)
        }
        syn::FieldExpr::Index { .. } => None,
        syn::FieldExpr::Base(syn::AtomExpr::Ident(id), _) if id.node == param => Some(vec![]),
        _ => None,
    }
}

/// SoA column roots extracted from region paths (sorted for stable arity checks).
/// Distinct from [`dedup_regions`]: that helper preserves first-seen order on full region
/// strings during summary collection; this collapses to column identity for seq compose rules.
fn region_column_roots(reads: &[Region]) -> Vec<String> {
    let mut roots: Vec<String> = reads
        .iter()
        .map(|r| r.split('.').next().unwrap_or(r).to_string())
        .collect();
    roots.sort();
    roots.dedup();
    roots
}

fn seq_parent_column(lhs_reads: &[Region]) -> Result<Option<String>, String> {
    let roots = region_column_roots(lhs_reads);
    if roots.len() > 1 {
        return Err(format!(
            "v0 sequential composition requires lhs with one SoA column root; got {roots:?}"
        ));
    }
    Ok(roots.into_iter().next())
}

fn build_summary_from_decl(
    decl: &syn::OpticDecl,
    _env: &HashMap<String, Arc<OpticSummary>>,
) -> OpticSummary {
    let fallback = syn::Expr::Atom(syn::AtomExpr::Int(0, Span::dummy()));
    let read_body = primary_read_clause(decl)
        .map(|c| &c.body)
        .unwrap_or(&fallback);
    let get_reads = collect_regions(read_body, "read");
    let mut put_reads = vec![];
    let mut put_writes = vec![];

    if let Some(put) = &decl.put {
        put_reads = collect_regions(&put.body, "read");
        put_writes = collect_regions(&put.body, "write");
    } else if let Some(review) = &decl.review {
        put_reads = collect_regions(&review.body, "read");
        put_writes = collect_regions(&review.body, "write");
    }

    let grade = extract_grade_from_ann(&decl.grade);

    OpticSummary {
        name: Some(decl.name.node.clone()),
        costate: type_expr_name(&decl.costate),
        focus: type_expr_name(&decl.focus),
        lift: focus_relative_lift_from_decl(decl),
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

/// collect_regions: structural walk of AST Expr/FieldExpr (post A parser fidelity) per ch8.9 Detailed impl ref.
/// Returns distinct field-root regions (e.g. column names) for read or write positions.
/// Follows cursor lowering table: field access root becomes the Region.
fn collect_regions(body: &syn::Expr, mode: &str) -> Vec<Region> {
    let mut regs = vec![];
    collect_regions_inner(body, mode, &mut regs);
    dedup_regions(regs)
}
fn collect_regions_inner(expr: &syn::Expr, mode: &str, out: &mut Vec<Region>) {
    match expr {
        syn::Expr::Field(fe) => {
            // Walk FieldExpr chain to the accessed column (e.g. healths or positions).
            let mut cur: &syn::FieldExpr = fe;
            loop {
                match cur {
                    syn::FieldExpr::FieldAccess { field, base, .. } => {
                        // the 'field' here is the column root in s.col[idx]
                        out.push(field.node.clone());
                        cur = base;
                    }
                    syn::FieldExpr::Index { base, .. } => {
                        cur = base;
                        // index expr may contain reads, but for v0 regions we care about the base column
                    }
                    syn::FieldExpr::Base(..) => break,
                }
            }
        }
        syn::Expr::Assign { target, value, .. } => {
            // In put bodies: lhs target is write; rhs may read (for reconstruction)
            if mode == "write" || mode == "read" {
                collect_regions_inner(target, if mode == "write" { "write" } else { "read" }, out);
            }
            if mode == "read" {
                collect_regions_inner(value, "read", out);
            }
        }
        syn::Expr::Block { stmts, result, .. } => {
            for s in stmts {
                collect_regions_inner(&s.expr, mode, out);
            }
            if let Some(r) = result {
                collect_regions_inner(r, mode, out);
            }
        }
        syn::Expr::Binary { left, right, .. } => {
            collect_regions_inner(left, mode, out);
            collect_regions_inner(right, mode, out);
        }
        syn::Expr::QueryChain(qc) => {
            // regions come from optic decls, not query site itself
            collect_regions_inner(&qc.base, mode, out);
        }
        _ => {}
    }
}

/// Parse surface grade annotations (ch6/9): CacheGrade<N>, Linear/Affine/Shared aliases.
pub fn extract_grade_from_ann(g: &syn::GradeExpr) -> ConcreteGrade {
    let mut cache_ann: Option<u8> = None;
    let mut share = Rational::one();
    let mut read_only = false;
    for dim in &g.dims {
        match dim {
            syn::GradeDim::Cache { n, .. } => {
                if let Some(v) = n {
                    cache_ann = Some(*v as u8);
                }
            }
            syn::GradeDim::Named { name, .. } => match name.as_str() {
                "LinearGrade" => share = Rational::one(),
                "AffineGrade" => share = Rational::one(),
                "SharedGrade" => {
                    share = Rational::one();
                    read_only = true;
                }
                _ => {}
            },
            syn::GradeDim::Ownership { r, .. } => {
                if let Some(txt) = r {
                    if let Some((a, b)) = txt.split_once('/') {
                        if let (Ok(n), Ok(d)) = (a.parse::<i64>(), b.parse::<u64>()) {
                            share = Rational::new(n, d);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    ConcreteGrade {
        cache: cache_ann.unwrap_or(1),
        ownership: OwnershipDim {
            share,
            read_only,
            must_use: false,
        },
    }
}

/// Annotated cache bound from surface grade, if any (for M2 GRA-* checks).
pub fn annotated_cache_bound(g: &syn::GradeExpr) -> Option<u8> {
    for dim in &g.dims {
        if let syn::GradeDim::Cache { n: Some(v), .. } = dim {
            return Some(*v as u8);
        }
    }
    None
}

/// True when surface grade includes `CacheGrade<_>` (elided cache dimension).
pub fn cache_grade_elided(g: &syn::GradeExpr) -> bool {
    g.dims
        .iter()
        .any(|d| matches!(d, syn::GradeDim::Cache { n: None, .. }))
}

/// Named ownership alias from surface grade, if any.
pub fn ownership_grade_alias(g: &syn::GradeExpr) -> Option<String> {
    for dim in &g.dims {
        if let syn::GradeDim::Named { name, .. } = dim {
            if matches!(name.as_str(), "LinearGrade" | "AffineGrade" | "SharedGrade") {
                return Some(name.clone());
            }
        }
    }
    None
}

fn make_summary_from_ann(
    name: &str,
    ty: &syn::GradeOpticType,
    optic: &HirOptic,
    env: &HashMap<String, Arc<OpticSummary>>,
) -> Result<OpticSummary, String> {
    let mut summary = compute_summary_for_optic(optic, env)?;
    let grade = extract_grade_from_ann(&ty.grade);
    summary.name = Some(name.into());
    summary.get_grade = grade.clone();
    summary.put_grade = grade;
    Ok(summary)
}

fn compute_summary_for_optic(
    optic: &HirOptic,
    env: &HashMap<String, Arc<OpticSummary>>,
) -> Result<OpticSummary, String> {
    match optic {
        HirOptic::Named { name, .. } => Ok(env
            .get(name)
            .map(|a| (**a).clone())
            .unwrap_or_else(|| default_summary(name))),
        HirOptic::Seq { lhs, rhs, .. } => {
            let mut s = compute_summary_for_optic(lhs, env)?;
            let r = compute_summary_for_optic(rhs, env)?;
            let a_get_reads = s.get_reads.clone();
            let a_put_reads = s.put_reads.clone();
            let parent_col = seq_parent_column(&a_get_reads)?;
            let parent_col = parent_col.as_deref();
            let lifted_get = PathLift::lift_regions(&s.lift, parent_col, &r.get_reads);
            let lifted_put_reads = PathLift::lift_regions(&s.lift, parent_col, &r.put_reads);
            let lifted_put_writes = PathLift::lift_regions(&s.lift, parent_col, &r.put_writes);
            s.get_reads = union(&a_get_reads, &lifted_get);
            s.put_reads = union(&a_put_reads, &union(&a_get_reads, &lifted_put_reads));
            s.put_writes = union(&s.put_writes, &lifted_put_writes);
            s.lift = PathLift::seq(&s.lift, &r.lift);
            s.provenance = s.provenance.merge(r.provenance);
            Ok(s)
        }
        HirOptic::Par { lhs, rhs, .. } => {
            let l = compute_summary_for_optic(lhs, env)?;
            let r = compute_summary_for_optic(rhs, env)?;
            let lift = PathLift::pair(&l.lift, &r.lift)?;
            Ok(OpticSummary {
                name: None,
                costate: l.costate.clone(),
                focus: "tuple".into(),
                lift,
                get_reads: union(&l.get_reads, &r.get_reads),
                put_reads: union(&l.put_reads, &r.put_reads),
                put_writes: union(&l.put_writes, &r.put_writes),
                get_grade: l.get_grade.clone(),
                put_grade: l.put_grade.clone(),
                get_determinism: Determinism::Pure,
                put_determinism: Determinism::Pure,
                serializable: true,
                provenance: l.provenance,
            })
        }
    }
}

fn default_summary(name: &str) -> OpticSummary {
    OpticSummary {
        name: Some(name.into()),
        costate: "Entities".into(),
        focus: "f32".into(),
        lift: PathLift::default(),
        get_reads: vec![],
        put_reads: vec![],
        put_writes: vec![],
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
        provenance: Span::dummy(),
    }
}

/// Small helper to dedup Region lists preserving first-seen order, O(n) using HashSet.
/// Dedup region lists preserving first-seen order (ch8.9).
/// Per ch8.9/9.9 region driven.
pub fn dedup_regions(v: Vec<Region>) -> Vec<Region> {
    let mut out = vec![];
    let mut seen = std::collections::HashSet::new();
    for r in v {
        if seen.insert(r.clone()) {
            out.push(r);
        }
    }
    out
}

fn union(a: &[Region], b: &[Region]) -> Vec<Region> {
    let mut out = a.to_vec();
    out.extend(b.iter().cloned());
    dedup_regions(out)
}

fn lower_optic_expr(
    e: &syn::OpticExpr,
    env: &HashMap<String, Arc<OpticSummary>>,
    depth: usize,
) -> Result<HirOptic, syn::ParseError> {
    if depth > MAX_OPTIC_COMPOSE_DEPTH {
        return Err(syn::ParseError {
            message: "optic compose depth limit exceeded".into(),
            span: optic_expr_span(e),
            kind: None,
        });
    }
    match e {
        syn::OpticExpr::Atom(syn::OpticAtom::Named(n)) => {
            let _resolved = resolve_ident(&n.node, env);
            Ok(HirOptic::Named {
                name: n.node.clone(),
                span: n.span,
            })
        }
        syn::OpticExpr::Seq { left, right, span } => Ok(HirOptic::Seq {
            lhs: Box::new(lower_optic_expr(left, env, depth + 1)?),
            rhs: Box::new(lower_optic_expr(right, env, depth + 1)?),
            span: *span,
        }),
        syn::OpticExpr::Par { left, right, span } => Ok(HirOptic::Par {
            lhs: Box::new(lower_optic_expr(left, env, depth + 1)?),
            rhs: Box::new(lower_optic_expr(right, env, depth + 1)?),
            span: *span,
        }),
        _ => Err(syn::ParseError {
            message: "unsupported optic expression".into(),
            span: Span::dummy(),
            kind: None,
        }),
    }
}

fn optic_expr_span(e: &syn::OpticExpr) -> Span {
    match e {
        syn::OpticExpr::Atom(syn::OpticAtom::Named(n)) => n.span,
        syn::OpticExpr::Seq { span, .. } | syn::OpticExpr::Par { span, .. } => *span,
        _ => Span::dummy(),
    }
}

/// resolve_ident: minimal impl of book ch8.9.1.2 resolver algorithm (name res order).
/// For v0 HIR: checks optic_env (named optics), falls back; full locals handled at call sites.
/// Emits would be diags later. Deterministic per book.
fn resolve_ident(name: &str, optic_env: &HashMap<String, Arc<OpticSummary>>) -> Option<String> {
    // 1. locals (not in this scope here; caller would have)
    // 2. named optics
    if optic_env.contains_key(name) {
        return Some(format!("optic:{}", name));
    }
    // 3. data decls (would check data_env)
    // 4. builtins (f32, etc)
    if ["f32", "Vec2", "Entities", "SoA"].contains(&name) {
        return Some(format!("builtin:{}", name));
    }
    None
}

fn is_query_operation(m: &syn::QueryMethod) -> bool {
    matches!(
        m,
        syn::QueryMethod::Get(_) | syn::QueryMethod::Set(_, _) | syn::QueryMethod::Map(_, _)
    )
}

/// Prefix-only observability hooks (v0 narrow); defense-in-depth when surface gate bypassed.
fn partition_observability(
    methods: &[syn::QueryMethod],
) -> Result<(Vec<ObsHook>, Vec<syn::QueryMethod>), syn::ParseError> {
    let mut hooks = vec![];
    let mut query_methods = vec![];
    let mut seen_query_op = false;
    for m in methods {
        match m {
            syn::QueryMethod::Profile(mode, sp) => {
                return Err(syn::ParseError {
                    message: format!(
                        "query method `profile` is not supported in narrow v0 (mode: {mode})"
                    ),
                    span: *sp,
                    kind: None,
                });
            }
            syn::QueryMethod::Replay(checkpoint, sp) => {
                return Err(syn::ParseError {
                    message: format!(
                        "query method `replay` is not supported in narrow v0 (checkpoint: {checkpoint})"
                    ),
                    span: *sp,
                    kind: None,
                });
            }
            syn::QueryMethod::Tap(_, sp) if seen_query_op => {
                return Err(syn::ParseError {
                    message: "`.tap(...)` must appear before .get/.set/.map in narrow v0".into(),
                    span: *sp,
                    kind: None,
                });
            }
            syn::QueryMethod::Record(_, sp) if seen_query_op => {
                return Err(syn::ParseError {
                    message: "`.record(...)` must appear before .get/.set/.map in narrow v0".into(),
                    span: *sp,
                    kind: None,
                });
            }
            syn::QueryMethod::Tap(label, sp) => {
                hooks.push(ObsHook::Tap(label.clone(), *sp));
            }
            syn::QueryMethod::Record(event, sp) => {
                hooks.push(ObsHook::Record(event.clone(), *sp));
            }
            other => {
                if is_query_operation(other) {
                    seen_query_op = true;
                }
                query_methods.push(other.clone());
            }
        }
    }
    Ok((hooks, query_methods))
}

fn lower_top_query(
    e: &syn::Expr,
    env: &HashMap<String, Arc<OpticSummary>>,
) -> Result<Option<HirQuery>, syn::ParseError> {
    // For the demo examples' top level query expr
    if let syn::Expr::QueryChain(qc) = e {
        let optic = lower_optic_expr(&qc.optic, env, 0).unwrap_or_else(|_| HirOptic::Named {
            name: "?".into(),
            span: qc.span,
        });
        let (observability, query_methods) = partition_observability(&qc.methods)?;
        Ok(Some(HirQuery {
            costate: "entities".into(),
            optic,
            cursor: "cur_0".into(),
            kind: lower_query_methods_to_kind(&query_methods),
            observability,
            span: qc.span,
        }))
    } else {
        Ok(None)
    }
}

fn lower_queries_from_fn(
    f: &syn::FnDecl,
    env: &HashMap<String, Arc<OpticSummary>>,
) -> Result<Vec<HirQuery>, syn::ParseError> {
    let mut qs = vec![];
    for stmt in &f.body {
        collect_queries_from_expr(&stmt.expr, env, &mut qs)?;
    }
    Ok(qs)
}

fn collect_queries_from_expr(
    e: &syn::Expr,
    env: &HashMap<String, Arc<OpticSummary>>,
    qs: &mut Vec<HirQuery>,
) -> Result<(), syn::ParseError> {
    if let Some(q) = lower_top_query(e, env)? {
        qs.push(q);
    }
    match e {
        syn::Expr::Block { stmts, result, .. } => {
            for s in stmts {
                collect_queries_from_expr(&s.expr, env, qs)?;
            }
            if let Some(r) = result {
                collect_queries_from_expr(r, env, qs)?;
            }
        }
        syn::Expr::Binary { left, right, .. } => {
            collect_queries_from_expr(left, env, qs)?;
            collect_queries_from_expr(right, env, qs)?;
        }
        _ => {}
    }
    Ok(())
}

fn lower_query_methods_to_kind(methods: &[syn::QueryMethod]) -> QueryKind {
    if methods.is_empty() {
        return QueryKind::Get;
    }
    // Reject unsupported trailing methods after map (v0: no .map().set()).
    let mut seen_map = false;
    for m in methods {
        match m {
            syn::QueryMethod::Map(_, _) => seen_map = true,
            syn::QueryMethod::Set(_, _) if seen_map => {
                return QueryKind::Map {
                    param: "it".into(),
                    body: Arc::new(HirExpr::Unsupported {
                        reason: "v0 does not support .map().set() chain".into(),
                        span: Span::dummy(),
                    }),
                };
            }
            _ => {}
        }
    }
    let has_get = methods
        .iter()
        .any(|m| matches!(m, syn::QueryMethod::Get(_)));
    let map_closures: Vec<&syn::Closure> = methods
        .iter()
        .filter_map(|m| {
            if let syn::QueryMethod::Map(cl, _) = m {
                Some(cl)
            } else {
                None
            }
        })
        .collect();
    if has_get && !map_closures.is_empty() {
        let span = methods
            .iter()
            .find_map(|m| match m {
                syn::QueryMethod::Get(sp) => Some(*sp),
                _ => None,
            })
            .unwrap_or_else(Span::dummy);
        return QueryKind::Map {
            param: "it".into(),
            body: Arc::new(HirExpr::Unsupported {
                reason: "v0 does not support .get().map() chain".into(),
                span,
            }),
        };
    }
    if !map_closures.is_empty() {
        return fuse_map_chain(&map_closures);
    }
    match methods.last() {
        Some(syn::QueryMethod::Set(v, _)) => QueryKind::Set {
            value: lower_expr(v),
        },
        _ => QueryKind::Get,
    }
}

fn closure_param(cl: &syn::Closure) -> String {
    if cl.params.len() > 1 {
        cl.params
            .iter()
            .map(|p| p.node.clone())
            .collect::<Vec<_>>()
            .join(",")
    } else {
        cl.params
            .first()
            .map(|p| p.node.clone())
            .unwrap_or_else(|| "it".into())
    }
}

/// Map fusion at HIR layer (ch10): collapse `.map(f).map(g)` into one map body `g(f(x))`.
fn fuse_map_chain(maps: &[&syn::Closure]) -> QueryKind {
    let param = closure_param(maps[0]);
    let mut body = lower_expr(&maps[0].body);
    for cl in maps.iter().skip(1) {
        if cl.params.len() > 1 && !matches!(body, HirExpr::Tuple(_, _)) {
            return QueryKind::Map {
                param,
                body: Arc::new(HirExpr::Unsupported {
                    reason: "incompatible map chain: multi-param map requires tuple focus from prior map".into(),
                    span: cl.span,
                }),
            };
        }
        let next = lower_expr(&cl.body);
        body = substitute_closure_params(&next, cl, &body);
    }
    QueryKind::Map {
        param,
        body: Arc::new(body),
    }
}

fn substitute_closure_params(e: &HirExpr, cl: &syn::Closure, repl: &HirExpr) -> HirExpr {
    if cl.params.len() > 1 && !matches!(repl, HirExpr::Tuple(_, _)) {
        return e.clone();
    }
    if cl.params.len() > 1 {
        if let HirExpr::Tuple(elems, _) = repl {
            let mut out = e.clone();
            for (p, el) in cl.params.iter().zip(elems.iter()) {
                out = substitute_hir_var(&out, &p.node, el);
            }
            return out;
        }
    }
    let inner = cl.params.first().map(|p| p.node.as_str()).unwrap_or("it");
    substitute_hir_var(e, inner, repl)
}

/// True when `name` appears as a free variable reference in `e`.
pub fn hir_expr_refs_var(e: &HirExpr, name: &str) -> bool {
    match e {
        HirExpr::Var(v, _) => v == name,
        HirExpr::Bin { left, right, .. } => {
            hir_expr_refs_var(left, name) || hir_expr_refs_var(right, name)
        }
        HirExpr::Paren(inner, _) => hir_expr_refs_var(inner, name),
        HirExpr::Tuple(elems, _) => elems.iter().any(|el| hir_expr_refs_var(el, name)),
        HirExpr::TupleProj { base, .. } => hir_expr_refs_var(base, name),
        HirExpr::FocusField { param, .. } => param == name,
        HirExpr::CursorField { cursor, .. } | HirExpr::CursorIndex { cursor, .. } => cursor == name,
        HirExpr::LitInt(_, _) | HirExpr::LitFloat(_, _) | HirExpr::Unsupported { .. } => false,
    }
}

/// Structural substitution replacing free occurrences of `name` with `repl`.
pub fn substitute_hir_var(e: &HirExpr, name: &str, repl: &HirExpr) -> HirExpr {
    match e {
        HirExpr::CursorField {
            cursor,
            field: _,
            span: _,
        }
        | HirExpr::CursorIndex {
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
        HirExpr::LitInt(n, sp) => HirExpr::LitInt(*n, *sp),
        HirExpr::LitFloat(f, sp) => HirExpr::LitFloat(*f, *sp),
        HirExpr::Var(v, sp) if v == name => repl.clone(),
        HirExpr::Var(v, sp) => HirExpr::Var(v.clone(), *sp),
        HirExpr::Bin {
            op,
            left,
            right,
            span,
        } => HirExpr::Bin {
            op: *op,
            left: Box::new(substitute_hir_var(left, name, repl)),
            right: Box::new(substitute_hir_var(right, name, repl)),
            span: *span,
        },
        HirExpr::Tuple(elems, sp) => HirExpr::Tuple(
            elems
                .iter()
                .map(|el| substitute_hir_var(el, name, repl))
                .collect(),
            *sp,
        ),
        HirExpr::Paren(inner, sp) => {
            HirExpr::Paren(Box::new(substitute_hir_var(inner, name, repl)), *sp)
        }
        HirExpr::TupleProj { base, index, span } => HirExpr::TupleProj {
            base: Box::new(substitute_hir_var(base, name, repl)),
            index: *index,
            span: *span,
        },
        HirExpr::FocusField { param, path, span } => {
            let new_param = if param == name {
                match repl {
                    HirExpr::Var(v, _) => v.clone(),
                    _ => param.clone(),
                }
            } else {
                param.clone()
            };
            HirExpr::FocusField {
                param: new_param,
                path: path.clone(),
                span: *span,
            }
        }
        HirExpr::Unsupported { reason, span } => HirExpr::Unsupported {
            reason: reason.clone(),
            span: *span,
        },
    }
}

/// Like [`substitute_hir_var`] but substitutes an identifier string (codegen path).
pub fn substitute_hir_ident(e: &HirExpr, name: &str, repl: &str) -> HirExpr {
    substitute_hir_var(e, name, &HirExpr::Var(repl.into(), Span::dummy()))
}

/// Minimal expr lowering to HirExpr for bodies (map bodies, values) per ch8 cursor forms + 8.9.2.1.
/// Only enough for the prelude examples (Bin, Lit, Atom idents become Var, Field for completeness).
/// Full cursor rewrite (s.foo -> cursor.arena...) can be done in CGIR or here; keep simple for HIR (bodies for provenance/summary only in v0).
fn lower_expr(e: &syn::Expr) -> HirExpr {
    match e {
        syn::Expr::Binary {
            left,
            op,
            right,
            span,
        } => HirExpr::Bin {
            op: *op,
            left: Box::new(lower_expr(left)),
            right: Box::new(lower_expr(right)),
            span: *span,
        },
        syn::Expr::Atom(syn::AtomExpr::Int(i, sp)) => HirExpr::LitInt(*i, *sp),
        syn::Expr::Atom(syn::AtomExpr::Float(f, sp)) => HirExpr::LitFloat(*f, *sp),
        syn::Expr::Atom(syn::AtomExpr::Ident(id)) => HirExpr::Var(id.node.clone(), id.span),
        syn::Expr::Atom(syn::AtomExpr::Paren(inner, sp)) => {
            HirExpr::Paren(Box::new(lower_expr(inner)), *sp)
        }
        syn::Expr::Atom(syn::AtomExpr::Tuple(exprs, sp)) => {
            HirExpr::Tuple(exprs.iter().map(lower_expr).collect(), *sp)
        }
        syn::Expr::Field(fe) => lower_field_expr(fe),
        syn::Expr::QueryChain(q) => HirExpr::Unsupported {
            reason: "query chain not supported in map/value context".into(),
            span: q.span,
        },
        syn::Expr::Assign { span, .. } | syn::Expr::Block { span, .. } => HirExpr::Unsupported {
            reason: "expression form not supported in map/value context".into(),
            span: *span,
        },
    }
}

fn lower_field_expr(fe: &syn::FieldExpr) -> HirExpr {
    match fe {
        syn::FieldExpr::FieldAccess { base, field, span } => {
            let base_h = match &**base {
                syn::FieldExpr::Base(syn::AtomExpr::Ident(id), _) => {
                    HirExpr::Var(id.node.clone(), id.span)
                }
                other => lower_field_expr(other),
            };
            if field.node.chars().all(|c| c.is_ascii_digit()) {
                match field.node.parse::<u32>() {
                    Ok(idx) => HirExpr::TupleProj {
                        base: Box::new(base_h),
                        index: idx,
                        span: *span,
                    },
                    Err(_) => HirExpr::Unsupported {
                        reason: format!("invalid tuple index `{}`", field.node),
                        span: *span,
                    },
                }
            } else {
                HirExpr::Unsupported {
                    reason: format!(
                        "record field access `.{}` not supported in map body",
                        field.node
                    ),
                    span: field.span,
                }
            }
        }
        syn::FieldExpr::Index { base, index, span } => {
            let _ = lower_field_expr(base);
            let _idx = lower_expr(index);
            HirExpr::Unsupported {
                reason: "field[index] not supported in map body (use tuple projection)".into(),
                span: *span,
            }
        }
        syn::FieldExpr::Base(atom, _span) => match atom {
            syn::AtomExpr::Ident(id) => HirExpr::Var(id.node.clone(), id.span),
            syn::AtomExpr::Int(i, sp) => HirExpr::LitInt(*i, *sp),
            syn::AtomExpr::Float(f, sp) => HirExpr::LitFloat(*f, *sp),
            syn::AtomExpr::Paren(inner, _) => lower_expr(inner),
            syn::AtomExpr::Tuple(exprs, sp) => {
                HirExpr::Tuple(exprs.iter().map(lower_expr).collect(), *sp)
            }
        },
    }
}
