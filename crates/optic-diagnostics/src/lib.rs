//! optic-diagnostics — structured diagnostics (ch. 11 + appendix A/F).

use optic_syntax::Span;
use serde::Serialize;
use serde_json::json;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Note,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Parse,
    Resolve,
    Type,
    Grade,
    Alias,
    Cgir,
    Fusion,
    Codegen,
}

#[derive(Clone, Debug, Serialize)]
pub struct RankedFix {
    pub description: String,
    pub confidence: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub title: String,
    pub severity: Severity,
    pub phase: Phase,
    pub primary_span: Span,
    pub related_spans: Vec<Span>,
    pub rule: String,
    pub evidence: serde_json::Value,
    pub minimal_fix_options: Vec<String>,
    pub ranked_fixes: Vec<RankedFix>,
    pub confidence: f32,
    pub next_commands: Vec<String>,
}

pub fn emit_human(d: &Diagnostic) -> String {
    format!(
        "{} [{}] {}: {} (at {:?})\n  evidence: {}\n  fix: {:?}\n  next: {:?}",
        d.code,
        serde_json::to_string(&d.phase).unwrap_or_else(|_| "?".into()),
        d.title,
        d.rule,
        d.primary_span,
        d.evidence,
        d.minimal_fix_options,
        d.next_commands
    )
}

pub fn to_json(d: &Diagnostic) -> String {
    serde_json::to_string_pretty(d).unwrap_or_else(|_| "{}".into())
}

pub fn diagnostics_to_json(diags: &[Diagnostic]) -> String {
    serde_json::to_string_pretty(&json!({ "ok": false, "diagnostics": diags }))
        .unwrap_or_else(|_| r#"{"ok":false,"diagnostics":[]}"#.into())
}

pub fn check_ok_json(notes: &[Diagnostic]) -> String {
    serde_json::to_string_pretty(&json!({ "ok": true, "notes": notes }))
        .unwrap_or_else(|_| r#"{"ok":true,"notes":[]}"#.into())
}

/// Diagnostic codes per book catalog (ch9/ch11).
pub const GRADE_DECL_TIGHT: &str = "GRA-110";
pub const GRADE_COMPOSE_OVER: &str = "GRA-104";
pub const ALIAS_CONFLICT: &str = "ALI-201";
pub const RESOLVE_UNKNOWN: &str = "RES-001";
pub const HIR_DUPLICATE_SOA: &str = "HIR-101";
pub const CGIR_MULTI_QUERY: &str = "CGI-001";
pub const CGIR_UNRESOLVED_OPTIC: &str = "CGI-002";
pub const CGIR_UNSUPPORTED_EXPR: &str = "CGI-003";
pub const CGIR_VERIFY_FAILED: &str = "CGI-004";
pub const FUS_COMPOSE_BLOCKED: &str = "FUS-501";
pub const FUS_COMPOSE_LEGALITY_BLOCKED: &str = "FUS-502";
pub const TYPE_UNKNOWN: &str = "TYP-001";
pub const TYPE_BODY_MISMATCH: &str = "TYP-002";
pub const TYPE_GRADE_SYNTAX: &str = "TYP-003";
pub const TYPE_BODY_UNINFERABLE: &str = "TYP-004";
pub const TYPE_UNSUPPORTED_V0: &str = "TYP-010";
pub const EXPLAIN_UNKNOWN_NODE: &str = "EXP-001";

fn ranked(fixes: &[&str]) -> Vec<RankedFix> {
    fixes
        .iter()
        .enumerate()
        .map(|(i, f)| RankedFix {
            description: (*f).into(),
            confidence: 1.0 - (i as f32 * 0.1),
        })
        .collect()
}

pub fn parse_diag(span: Span, message: String) -> Diagnostic {
    let fixes = vec![
        "fix syntax near primary span".into(),
        "compare with appendix D EBNF".into(),
    ];
    Diagnostic {
        code: "PAR-001".into(),
        title: "parse error".into(),
        severity: Severity::Error,
        phase: Phase::Parse,
        primary_span: span,
        related_spans: vec![],
        rule: message,
        evidence: json!({}),
        minimal_fix_options: fixes.clone(),
        ranked_fixes: ranked(&[
            "fix syntax near primary span",
            "compare with appendix D EBNF",
        ]),
        confidence: 1.0,
        next_commands: vec!["opticc dump-tokens file.opt".into()],
    }
}

pub fn hir_duplicate_soa_diag(span: Span, costate: &str, rule: &str) -> Diagnostic {
    Diagnostic {
        code: HIR_DUPLICATE_SOA.into(),
        title: "duplicate SoA costate data declaration".into(),
        severity: Severity::Error,
        phase: Phase::Resolve,
        primary_span: span,
        related_spans: vec![],
        rule: rule.into(),
        evidence: json!({ "costate": costate }),
        minimal_fix_options: vec!["merge columns into a single SoA costate data decl".into()],
        ranked_fixes: ranked(&[
            "merge columns into a single SoA costate data decl",
            "remove the duplicate data declaration",
        ]),
        confidence: 1.0,
        next_commands: vec!["opticc dump-hir file.opt".into()],
    }
}

pub fn resolve_diag(span: Span, message: String) -> Diagnostic {
    Diagnostic {
        code: RESOLVE_UNKNOWN.into(),
        title: "name resolution failed".into(),
        severity: Severity::Error,
        phase: Phase::Resolve,
        primary_span: span,
        related_spans: vec![],
        rule: message.clone(),
        evidence: json!({ "name": message }),
        minimal_fix_options: vec!["check optic spelling".into()],
        ranked_fixes: ranked(&["declare the optic before use"]),
        confidence: 0.95,
        next_commands: vec!["opticc dump-hir file.opt".into()],
    }
}

pub fn grade_decl_diag(
    span: Span,
    related_spans: Vec<Span>,
    rule: &str,
    evidence: serde_json::Value,
) -> Diagnostic {
    let fixes = vec![
        "raise CacheGrade annotation".into(),
        "reduce read/write regions in optic body".into(),
    ];
    Diagnostic {
        code: GRADE_DECL_TIGHT.into(),
        title: "declared grade tighter than inferred".into(),
        severity: Severity::Error,
        phase: Phase::Grade,
        primary_span: span,
        related_spans,
        rule: rule.into(),
        evidence,
        minimal_fix_options: fixes.clone(),
        ranked_fixes: ranked(&[
            "raise CacheGrade annotation",
            "reduce read/write regions in optic body",
        ]),
        confidence: 0.9,
        next_commands: vec![
            "opticc explain GRA-110".into(),
            "opticc dump-summary --node ...".into(),
        ],
    }
}

pub fn grade_compose_diag(
    span: Span,
    related_spans: Vec<Span>,
    rule: &str,
    evidence: serde_json::Value,
) -> Diagnostic {
    let fixes = vec![
        "raise sequential CacheGrade bound".into(),
        "split composition into separate queries".into(),
    ];
    Diagnostic {
        code: GRADE_COMPOSE_OVER.into(),
        title: "sequential composition exceeds cache bound".into(),
        severity: Severity::Error,
        phase: Phase::Grade,
        primary_span: span,
        related_spans,
        rule: rule.into(),
        evidence,
        minimal_fix_options: fixes.clone(),
        ranked_fixes: ranked(&[
            "raise sequential CacheGrade bound",
            "split composition into separate queries",
        ]),
        confidence: 0.9,
        next_commands: vec![
            "opticc explain GRA-104".into(),
            "opticc dump-summary --node ...".into(),
        ],
    }
}

pub fn codegen_failed_diag(rule: &str) -> Diagnostic {
    Diagnostic {
        code: "CGI-005".into(),
        title: "codegen failed".into(),
        severity: Severity::Error,
        phase: Phase::Codegen,
        primary_span: Span::dummy(),
        related_spans: vec![],
        rule: rule.into(),
        evidence: json!({}),
        minimal_fix_options: vec!["fix map body tuple arity or unsupported forms".into()],
        ranked_fixes: ranked(&["fix map body tuple arity or unsupported forms"]),
        confidence: 1.0,
        next_commands: vec!["opticc transpile file.opt".into()],
    }
}

pub fn alias_diag(
    span: Span,
    related_spans: Vec<Span>,
    regions: &[String],
    rule: &str,
) -> Diagnostic {
    Diagnostic {
        code: ALIAS_CONFLICT.into(),
        title: "alias conflict".into(),
        severity: Severity::Error,
        phase: Phase::Alias,
        primary_span: span,
        related_spans,
        rule: rule.into(),
        evidence: json!({ "conflicting_regions": regions }),
        minimal_fix_options: vec![
            "split product into sequential passes".into(),
            "use read-only grades if possible".into(),
        ],
        ranked_fixes: ranked(&[
            "split product into sequential passes",
            "use read-only grades if possible",
        ]),
        confidence: 0.95,
        next_commands: vec![
            "opticc explain ALI-201".into(),
            "opticc dump-summary --node ...".into(),
        ],
    }
}

pub fn unsupported_expr_diag(span: Span, reason: &str) -> Diagnostic {
    Diagnostic {
        code: CGIR_UNSUPPORTED_EXPR.into(),
        title: "unsupported expression in query body".into(),
        severity: Severity::Error,
        phase: Phase::Type,
        primary_span: span,
        related_spans: vec![],
        rule: reason.into(),
        evidence: json!({ "reason": reason }),
        minimal_fix_options: vec!["simplify map/set value expression".into()],
        ranked_fixes: ranked(&["simplify map/set value expression"]),
        confidence: 0.95,
        next_commands: vec!["opticc dump-hir file.opt".into()],
    }
}

pub fn fusion_compose_blocked_diag(
    span: Span,
    rule: &str,
    evidence: serde_json::Value,
) -> Diagnostic {
    Diagnostic {
        code: FUS_COMPOSE_BLOCKED.into(),
        title: "compose fusion blocked — intermediate escapes".into(),
        severity: Severity::Note,
        phase: Phase::Fusion,
        primary_span: span,
        related_spans: vec![],
        rule: rule.into(),
        evidence,
        minimal_fix_options: vec!["introduce a stage boundary or keep the unfused form".into()],
        ranked_fixes: ranked(&[
            "introduce a stage boundary or keep the unfused form",
            "opticc dump-cgir file.opt --before-fusion",
        ]),
        confidence: 0.9,
        next_commands: vec!["opticc dump-cgir file.opt --before-fusion".into()],
    }
}

pub fn fusion_compose_legality_diag(
    span: Span,
    rule: &str,
    evidence: serde_json::Value,
) -> Diagnostic {
    Diagnostic {
        code: FUS_COMPOSE_LEGALITY_BLOCKED.into(),
        title: "compose fusion blocked — legality precondition".into(),
        severity: Severity::Note,
        phase: Phase::Fusion,
        primary_span: span,
        related_spans: vec![],
        rule: rule.into(),
        evidence,
        minimal_fix_options: vec![
            "fix focus/costate wiring or optic purity".into(),
            "keep the unfused compose form".into(),
        ],
        ranked_fixes: ranked(&[
            "fix focus/costate wiring or optic purity",
            "keep the unfused compose form",
            "opticc dump-cgir file.opt --before-fusion",
        ]),
        confidence: 0.9,
        next_commands: vec!["opticc dump-cgir file.opt --before-fusion".into()],
    }
}

pub fn fusion_verify_diag(rule: &str) -> Diagnostic {
    Diagnostic {
        code: CGIR_VERIFY_FAILED.into(),
        title: "fusion or CGIR verify failed".into(),
        severity: Severity::Error,
        phase: Phase::Fusion,
        primary_span: Span::dummy(),
        related_spans: vec![],
        rule: rule.into(),
        evidence: json!({}),
        minimal_fix_options: vec!["opticc dump-cgir file.opt --check".into()],
        ranked_fixes: ranked(&["opticc dump-cgir file.opt --check"]),
        confidence: 1.0,
        next_commands: vec!["opticc dump-cgir file.opt --check".into()],
    }
}

pub fn type_unknown_diag(
    span: Span,
    type_name: &str,
    role: &str,
    node: Option<&str>,
    binding: bool,
) -> Diagnostic {
    let mut evidence = json!({ "type_name": type_name, "role": role });
    if let Some(n) = node {
        if binding {
            evidence["binding"] = json!(n);
        } else {
            evidence["optic"] = json!(n);
        }
    }
    Diagnostic {
        code: TYPE_UNKNOWN.into(),
        title: "unknown type".into(),
        severity: Severity::Error,
        phase: Phase::Type,
        primary_span: span,
        related_spans: vec![],
        rule: format!("type `{type_name}` is not declared in this program"),
        evidence,
        minimal_fix_options: vec![
            "declare a data type with this name".into(),
            "use a primitive type (f32, i32, u32, Vec2)".into(),
        ],
        ranked_fixes: ranked(&[
            "declare a data type with this name",
            "use a primitive type (f32, i32, u32, Vec2)",
        ]),
        confidence: 0.95,
        next_commands: vec![
            "opticc explain TYP-001".into(),
            "opticc dump-hir file.opt".into(),
        ],
    }
}

pub fn type_body_mismatch_diag(
    span: Span,
    expected: &str,
    actual: &str,
    clause: &str,
    optic: &str,
) -> Diagnostic {
    Diagnostic {
        code: TYPE_BODY_MISMATCH.into(),
        title: "type mismatch in optic body".into(),
        severity: Severity::Error,
        phase: Phase::Type,
        primary_span: span,
        related_spans: vec![],
        rule: format!("{clause} body type `{actual}` does not match declared focus `{expected}`"),
        evidence: json!({
            "expected_type": expected,
            "actual_type": actual,
            "clause": clause,
            "optic": optic,
        }),
        minimal_fix_options: vec![
            "change declared focus type to match the body".into(),
            "adjust get/put body to return the declared focus type".into(),
        ],
        ranked_fixes: ranked(&[
            "change declared focus type to match the body",
            "adjust get/put body to return the declared focus type",
        ]),
        confidence: 0.95,
        next_commands: vec![
            "opticc explain TYP-002".into(),
            "edit declared focus type or get/put body in source".into(),
        ],
    }
}

pub fn type_body_uninferable_diag(span: Span, clause: &str, optic: &str) -> Diagnostic {
    Diagnostic {
        code: TYPE_BODY_UNINFERABLE.into(),
        title: "cannot infer optic body type".into(),
        severity: Severity::Error,
        phase: Phase::Type,
        primary_span: span,
        related_spans: vec![],
        rule: format!("{clause} body uses a form the type checker cannot infer in v0"),
        evidence: json!({ "clause": clause, "optic": optic }),
        minimal_fix_options: vec![
            "use indexed SoA access or simple field projection".into(),
            "simplify the optic body expression".into(),
        ],
        ranked_fixes: ranked(&[
            "use indexed SoA access or simple field projection",
            "simplify the optic body expression",
        ]),
        confidence: 0.9,
        next_commands: vec![
            "opticc explain TYP-004".into(),
            "opticc dump-hir file.opt".into(),
        ],
    }
}

pub fn type_unsupported_v0_diag(
    span: Span,
    feature: &str,
    detail: &str,
    name: Option<&str>,
) -> Diagnostic {
    let mut evidence = json!({ "feature": feature, "detail": detail });
    if let Some(n) = name {
        evidence["name"] = json!(n);
    }
    Diagnostic {
        code: TYPE_UNSUPPORTED_V0.into(),
        title: "unsupported in narrow v0".into(),
        severity: Severity::Error,
        phase: Phase::Type,
        primary_span: span,
        related_spans: vec![],
        rule: detail.into(),
        evidence,
        minimal_fix_options: vec![
            "use lens-like GradedOptic forms supported in v0".into(),
            "defer prisms and host boundaries to M7+".into(),
        ],
        ranked_fixes: ranked(&[
            "use lens-like GradedOptic forms supported in v0",
            "defer prisms and host boundaries to M7+",
        ]),
        confidence: 1.0,
        next_commands: vec![
            "opticc explain TYP-010".into(),
            "opticc check file.opt --json".into(),
        ],
    }
}

pub fn explain_unknown_node_diag(span: Span, node: &str, candidates: &[String]) -> Diagnostic {
    Diagnostic {
        code: EXPLAIN_UNKNOWN_NODE.into(),
        title: "explain: unknown optic or let name".into(),
        severity: Severity::Error,
        phase: Phase::Type,
        primary_span: span,
        related_spans: vec![],
        rule: format!("no optic or let binding named `{node}`"),
        evidence: json!({ "node": node, "candidates": candidates }),
        minimal_fix_options: vec!["check optic or let binding spelling".into()],
        ranked_fixes: ranked(&["check optic or let binding spelling"]),
        confidence: 1.0,
        next_commands: vec![
            "opticc dump-hir file.opt".into(),
            "opticc dump-summary file.opt".into(),
        ],
    }
}

pub fn explain_grade_ok_json(report: &serde_json::Value) -> String {
    serde_json::to_string_pretty(&json!({ "ok": true, "grade": report }))
        .unwrap_or_else(|_| r#"{"ok":true,"grade":{}}"#.into())
}

pub fn explain_focus_ok_json(report: &serde_json::Value) -> String {
    serde_json::to_string_pretty(&json!({ "ok": true, "focus": report }))
        .unwrap_or_else(|_| r#"{"ok":true,"focus":{}}"#.into())
}

pub fn type_grade_syntax_diag(
    span: Span,
    detail: &str,
    fragment: &str,
    optic: &str,
) -> Diagnostic {
    Diagnostic {
        code: TYPE_GRADE_SYNTAX.into(),
        title: "invalid grade annotation syntax".into(),
        severity: Severity::Error,
        phase: Phase::Type,
        primary_span: span,
        related_spans: vec![],
        rule: detail.into(),
        evidence: json!({ "fragment": fragment, "optic": optic }),
        minimal_fix_options: vec![
            "use CacheGrade<N>, LinearGrade, AffineGrade, SharedGrade, or OwnershipGrade<num/den>".into(),
            "use `_` for inferrable dimensions".into(),
        ],
        ranked_fixes: ranked(&[
            "use CacheGrade<N>, LinearGrade, AffineGrade, SharedGrade, or OwnershipGrade<num/den>",
            "use `_` for inferrable dimensions",
        ]),
        confidence: 0.95,
        next_commands: vec![
            "opticc explain TYP-003".into(),
            "fix grade annotation syntax in optic declaration".into(),
        ],
    }
}

pub fn cgir_diag(code: &str, span: Span, rule: &str, evidence: serde_json::Value) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        title: "CGIR invariant violation".into(),
        severity: Severity::Error,
        phase: Phase::Cgir,
        primary_span: span,
        related_spans: vec![],
        rule: rule.into(),
        evidence,
        minimal_fix_options: vec![],
        ranked_fixes: vec![],
        confidence: 1.0,
        next_commands: vec!["opticc dump-cgir file.opt --check".into()],
    }
}
