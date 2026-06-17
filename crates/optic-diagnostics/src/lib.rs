//! optic-diagnostics — structured diagnostics (ch. 11 + app A/F).
//! code/phase/span/rule/evidence/minimal_fix/next_commands ; human + json.

use optic_syntax::Span;
use serde::Serialize; // Deserialize unused (no de from json yet; kept struct serializable per ch11)
use serde_json::json;

#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub code: String, // e.g. "ALI-101", "GRA-104"
    pub phase: Phase,
    pub primary_span: Span,
    pub rule: String,
    pub evidence: serde_json::Value,
    pub minimal_fix_options: Vec<String>,
    pub next_commands: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
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

pub fn emit_human(d: &Diagnostic) -> String {
    format!(
        "{} [{}]: {} (at {:?})\n  evidence: {}\n  fix: {:?}\n  next: {:?}",
        d.code,
        format!("{:?}", d.phase),
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

/// Common codes for v0 (from book examples + ch11).
pub const ALIAS_CONFLICT: &str = "ALI-101";
pub const GRADE_CACHE_OVER: &str = "GRA-104";

pub fn grade_diag(span: Span, rule: &str, evidence: serde_json::Value) -> Diagnostic {
    Diagnostic {
        code: GRADE_CACHE_OVER.into(),
        phase: Phase::Grade,
        primary_span: span,
        rule: rule.into(),
        evidence,
        minimal_fix_options: vec![
            "raise CacheGrade annotation".into(),
            "reduce read/write regions in optic body".into(),
        ],
        next_commands: vec![
            "optic explain GRA-104".into(),
            "optic dump-summary --node ...".into(),
        ],
    }
}

pub fn alias_diag(span: Span, regions: &[String], rule: &str) -> Diagnostic {
    Diagnostic {
        code: ALIAS_CONFLICT.into(),
        phase: Phase::Alias,
        primary_span: span,
        rule: rule.into(),
        evidence: json!({ "overlapping_regions": regions }),
        minimal_fix_options: vec![
            "split product into sequential passes".into(),
            "use read-only grades if possible".into(),
        ],
        next_commands: vec![
            "optic explain ALI-101".into(),
            "optic dump-summary --node ...".into(),
        ],
    }
}
