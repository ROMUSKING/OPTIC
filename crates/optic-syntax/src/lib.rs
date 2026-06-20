//! optic-syntax — lexer, tokens, parser, and AST for the narrow v0 surface language.
//!
//! Designed per book ch. 7 + Appendix D (normative EBNF).
//! - Hand-written recursive descent + binding-power parser (no generator).
//! - Longest-match for >>> and *** (indivisible tokens).
//! - Nestable block comments {- -}.
//! - Deterministic recovery to sync points.
//! - Spans on every significant node.

pub mod ast;
pub mod lexer;
pub mod obs;
pub mod parser;
pub mod span;
pub mod token;

pub use obs::{decode_obs_hook_string_lit, validate_obs_hook_label, MAX_OBS_HOOK_LABEL_BYTES};

pub use ast::*;
pub use lexer::Lexer;
pub use parser::{parse, ParseError, ParseErrorKind};
pub use span::{SourceId, Span, Spanned};
pub use token::{Token, TokenKind};

/// Pretty-print AST for `dump-ast` (appendix B, M0 goldens).
pub fn dump_ast(prog: &Program) -> String {
    format!("{:#?}", prog)
}

/// Deterministic token dump for M0 goldens (kind + span only).
pub fn dump_tokens(src: &str, source_id: SourceId) -> String {
    let mut out = String::new();
    for t in Lexer::new(src, source_id).lex() {
        use std::fmt::Write;
        let _ = writeln!(out, "{:?} {:?}", t.kind, t.span);
    }
    out
}

#[cfg(test)]
mod golden_tests {
    use super::*;
    use std::path::PathBuf;

    fn examples_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
    }

    fn fixture_path(subdir: &str, name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../fixtures/{subdir}/{name}"))
    }

    fn assert_golden(subdir: &str, example: &str, actual: &str) {
        let path = fixture_path(subdir, &format!("{example}.txt"));
        if std::env::var("OPTIC_UPDATE_GOLDEN").is_ok() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create fixture dir");
            }
            std::fs::write(&path, actual).expect("write golden");
        }
        let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
            panic!(
                "missing golden {} (run with OPTIC_UPDATE_GOLDEN=1)",
                path.display()
            )
        });
        assert_eq!(actual, expected, "golden mismatch for {example}");
    }

    #[test]
    fn golden_tokens_health_decay() {
        let src = std::fs::read_to_string(examples_dir().join("health_decay.opt")).expect("read");
        let actual = dump_tokens(&src, SourceId(1));
        assert_golden("tokens", "health_decay", &actual);
    }

    #[test]
    fn golden_ast_health_decay() {
        let src = std::fs::read_to_string(examples_dir().join("health_decay.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        let actual = dump_ast(&prog);
        assert_golden("ast", "health_decay", &actual);
    }

    #[test]
    fn golden_tokens_health_position() {
        let src =
            std::fs::read_to_string(examples_dir().join("health_position.opt")).expect("read");
        let actual = dump_tokens(&src, SourceId(1));
        assert_golden("tokens", "health_position", &actual);
    }

    #[test]
    fn golden_ast_health_position() {
        let src =
            std::fs::read_to_string(examples_dir().join("health_position.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        let actual = dump_ast(&prog);
        assert_golden("ast", "health_position", &actual);
    }

    #[test]
    fn golden_tokens_nested_position() {
        let src =
            std::fs::read_to_string(examples_dir().join("nested_position.opt")).expect("read");
        assert_golden("tokens", "nested_position", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_nested_position() {
        let src =
            std::fs::read_to_string(examples_dir().join("nested_position.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "nested_position", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_nested_field_triple() {
        let src =
            std::fs::read_to_string(examples_dir().join("nested_field_triple.opt")).expect("read");
        assert_golden(
            "tokens",
            "nested_field_triple",
            &dump_tokens(&src, SourceId(1)),
        );
    }

    #[test]
    fn golden_ast_nested_field_triple() {
        let src =
            std::fs::read_to_string(examples_dir().join("nested_field_triple.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "nested_field_triple", &dump_ast(&prog));
    }

    /// Full positive suite; one example per test keeps memory bounded (PLAN §5).
    #[test]
    fn golden_tokens_health_get() {
        let src = std::fs::read_to_string(examples_dir().join("health_get.opt")).expect("read");
        assert_golden("tokens", "health_get", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_tokens_health_set() {
        let src = std::fs::read_to_string(examples_dir().join("health_set.opt")).expect("read");
        assert_golden("tokens", "health_set", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_health_get() {
        let src = std::fs::read_to_string(examples_dir().join("health_get.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "health_get", &dump_ast(&prog));
    }

    #[test]
    fn golden_ast_health_set() {
        let src = std::fs::read_to_string(examples_dir().join("health_set.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "health_set", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_alive_filter() {
        let src = std::fs::read_to_string(examples_dir().join("alive_filter.opt")).expect("read");
        assert_golden("tokens", "alive_filter", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_alive_filter() {
        let src = std::fs::read_to_string(examples_dir().join("alive_filter.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "alive_filter", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_prism_get() {
        let src = std::fs::read_to_string(examples_dir().join("prism_get.opt")).expect("read");
        assert_golden("tokens", "prism_get", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_prism_get() {
        let src = std::fs::read_to_string(examples_dir().join("prism_get.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "prism_get", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_prism_set() {
        let src = std::fs::read_to_string(examples_dir().join("prism_set.opt")).expect("read");
        assert_golden("tokens", "prism_set", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_prism_set() {
        let src = std::fs::read_to_string(examples_dir().join("prism_set.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "prism_set", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_partial_prism() {
        let src = std::fs::read_to_string(examples_dir().join("partial_prism.opt")).expect("read");
        assert_golden("tokens", "partial_prism", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_partial_prism() {
        let src = std::fs::read_to_string(examples_dir().join("partial_prism.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "partial_prism", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_all_healths() {
        let src = std::fs::read_to_string(examples_dir().join("all_healths.opt")).expect("read");
        assert_golden("tokens", "all_healths", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_all_healths() {
        let src = std::fs::read_to_string(examples_dir().join("all_healths.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "all_healths", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_traversal_get() {
        let src = std::fs::read_to_string(examples_dir().join("traversal_get.opt")).expect("read");
        assert_golden("tokens", "traversal_get", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_traversal_get() {
        let src = std::fs::read_to_string(examples_dir().join("traversal_get.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "traversal_get", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_traversal_set() {
        let src = std::fs::read_to_string(examples_dir().join("traversal_set.opt")).expect("read");
        assert_golden("tokens", "traversal_set", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_traversal_set() {
        let src = std::fs::read_to_string(examples_dir().join("traversal_set.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "traversal_set", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_compose_prism() {
        let src = std::fs::read_to_string(examples_dir().join("compose_prism.opt")).expect("read");
        assert_golden("tokens", "compose_prism", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_compose_prism() {
        let src = std::fs::read_to_string(examples_dir().join("compose_prism.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "compose_prism", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_tap_health() {
        let src = std::fs::read_to_string(examples_dir().join("tap_health.opt")).expect("read");
        assert_golden("tokens", "tap_health", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_tap_health() {
        let src = std::fs::read_to_string(examples_dir().join("tap_health.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "tap_health", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_record_health() {
        let src = std::fs::read_to_string(examples_dir().join("record_health.opt")).expect("read");
        assert_golden("tokens", "record_health", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_record_health() {
        let src = std::fs::read_to_string(examples_dir().join("record_health.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "record_health", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_tap_record_chain() {
        let src =
            std::fs::read_to_string(examples_dir().join("tap_record_chain.opt")).expect("read");
        assert_golden(
            "tokens",
            "tap_record_chain",
            &dump_tokens(&src, SourceId(1)),
        );
    }

    #[test]
    fn golden_ast_tap_record_chain() {
        let src =
            std::fs::read_to_string(examples_dir().join("tap_record_chain.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "tap_record_chain", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_unsupported_profile() {
        let src =
            std::fs::read_to_string(examples_dir().join("unsupported_profile.opt")).expect("read");
        assert_golden(
            "tokens",
            "unsupported_profile",
            &dump_tokens(&src, SourceId(1)),
        );
    }

    #[test]
    fn golden_ast_unsupported_profile() {
        let src =
            std::fs::read_to_string(examples_dir().join("unsupported_profile.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "unsupported_profile", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_unsupported_replay() {
        let src =
            std::fs::read_to_string(examples_dir().join("unsupported_replay.opt")).expect("read");
        assert_golden(
            "tokens",
            "unsupported_replay",
            &dump_tokens(&src, SourceId(1)),
        );
    }

    #[test]
    fn golden_ast_unsupported_replay() {
        let src =
            std::fs::read_to_string(examples_dir().join("unsupported_replay.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "unsupported_replay", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_trailing_tap() {
        let src = std::fs::read_to_string(examples_dir().join("trailing_tap.opt")).expect("read");
        assert_golden("tokens", "trailing_tap", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_trailing_tap() {
        let src = std::fs::read_to_string(examples_dir().join("trailing_tap.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "trailing_tap", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_invalid_grade() {
        let src = std::fs::read_to_string(examples_dir().join("invalid_grade.opt")).expect("read");
        assert_golden("tokens", "invalid_grade", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_tokens_invalid_alias() {
        let src = std::fs::read_to_string(examples_dir().join("invalid_alias.opt")).expect("read");
        assert_golden("tokens", "invalid_alias", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_invalid_grade() {
        let src = std::fs::read_to_string(examples_dir().join("invalid_grade.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "invalid_grade", &dump_ast(&prog));
    }

    #[test]
    fn golden_ast_invalid_alias() {
        let src = std::fs::read_to_string(examples_dir().join("invalid_alias.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "invalid_alias", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_grade_mismatch() {
        let src = std::fs::read_to_string(examples_dir().join("grade_mismatch.opt")).expect("read");
        assert_golden("tokens", "grade_mismatch", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_grade_mismatch() {
        let src = std::fs::read_to_string(examples_dir().join("grade_mismatch.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "grade_mismatch", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_compose_decay() {
        let src = std::fs::read_to_string(examples_dir().join("compose_decay.opt")).expect("read");
        assert_golden("tokens", "compose_decay", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_compose_decay() {
        let src = std::fs::read_to_string(examples_dir().join("compose_decay.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "compose_decay", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_compose_triple() {
        let src = std::fs::read_to_string(examples_dir().join("compose_triple.opt")).expect("read");
        assert_golden("tokens", "compose_triple", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_ast_compose_triple() {
        let src = std::fs::read_to_string(examples_dir().join("compose_triple.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "compose_triple", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_unsupported_prism() {
        let src =
            std::fs::read_to_string(examples_dir().join("unsupported_prism.opt")).expect("read");
        assert_golden(
            "tokens",
            "unsupported_prism",
            &dump_tokens(&src, SourceId(1)),
        );
    }

    #[test]
    fn golden_ast_unsupported_prism() {
        let src =
            std::fs::read_to_string(examples_dir().join("unsupported_prism.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "unsupported_prism", &dump_ast(&prog));
    }

    #[test]
    fn golden_tokens_host_boundary() {
        let src = std::fs::read_to_string(examples_dir().join("host_boundary.opt")).expect("read");
        assert_golden("tokens", "host_boundary", &dump_tokens(&src, SourceId(1)));
    }

    #[test]
    fn golden_tokens_unsupported_traversal() {
        let src = std::fs::read_to_string(examples_dir().join("unsupported_traversal.opt"))
            .expect("read");
        assert_golden(
            "tokens",
            "unsupported_traversal",
            &dump_tokens(&src, SourceId(1)),
        );
    }

    #[test]
    fn golden_ast_unsupported_traversal() {
        let src = std::fs::read_to_string(examples_dir().join("unsupported_traversal.opt"))
            .expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "unsupported_traversal", &dump_ast(&prog));
    }

    #[test]
    fn golden_ast_host_boundary() {
        let src = std::fs::read_to_string(examples_dir().join("host_boundary.opt")).expect("read");
        let prog = parse(&src, SourceId(1)).expect("parse");
        assert_golden("ast", "host_boundary", &dump_ast(&prog));
    }
}
