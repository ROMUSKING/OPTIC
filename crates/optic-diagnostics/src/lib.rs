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

/// Diagnostic codes per book catalog (ch9/ch11).
pub const GRADE_DECL_TIGHT: &str = "GRA-110";
pub const GRADE_COMPOSE_OVER: &str = "GRA-104";
pub const ALIAS_CONFLICT: &str = "ALI-201";
pub const RESOLVE_UNKNOWN: &str = "RES-001";
pub const CGIR_MULTI_QUERY: &str = "CGI-001";
pub const CGIR_UNRESOLVED_OPTIC: &str = "CGI-002";

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
    Diagnostic {
        code: "PAR-001".into(),
        title: "parse error".into(),
        severity: Severity::Error,
        phase: Phase::Parse,
        primary_span: span,
        related_spans: vec![],
        rule: message,
        evidence: json!({}),
        minimal_fix_options: vec![],
        ranked_fixes: vec![],
        confidence: 1.0,
        next_commands: vec![],
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
        next_commands: vec!["optic dump-hir file.opt".into()],
    }
}

pub fn grade_decl_diag(span: Span, rule: &str, evidence: serde_json::Value) -> Diagnostic {
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
        related_spans: vec![],
        rule: rule.into(),
        evidence,
        minimal_fix_options: fixes.clone(),
        ranked_fixes: ranked(&[
            "raise CacheGrade annotation",
            "reduce read/write regions in optic body",
        ]),
        confidence: 0.9,
        next_commands: vec![
            "optic explain GRA-110".into(),
            "optic dump-summary --node ...".into(),
        ],
    }
}

pub fn grade_compose_diag(span: Span, rule: &str, evidence: serde_json::Value) -> Diagnostic {
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
        related_spans: vec![],
        rule: rule.into(),
        evidence,
        minimal_fix_options: fixes.clone(),
        ranked_fixes: ranked(&[
            "raise sequential CacheGrade bound",
            "split composition into separate queries",
        ]),
        confidence: 0.9,
        next_commands: vec![
            "optic explain GRA-104".into(),
            "optic dump-summary --node ...".into(),
        ],
    }
}

pub fn alias_diag(span: Span, regions: &[String], rule: &str) -> Diagnostic {
    Diagnostic {
        code: ALIAS_CONFLICT.into(),
        title: "alias conflict".into(),
        severity: Severity::Error,
        phase: Phase::Alias,
        primary_span: span,
        related_spans: vec![],
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
            "optic explain ALI-201".into(),
            "optic dump-summary --node ...".into(),
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
        next_commands: vec!["optic dump-cgir file.opt --check".into()],
    }
}
