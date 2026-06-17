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
pub mod parser;
pub mod span;
pub mod token;

pub use ast::*;
pub use lexer::Lexer;
pub use parser::{parse, ParseError};
pub use span::{SourceId, Span, Spanned};
pub use token::{Token, TokenKind};

/// Pretty-print AST for `dump-ast` (appendix B, M0 goldens).
pub fn dump_ast(prog: &Program) -> String {
    format!("{:#?}", prog)
}
