//! optic-syntax — lexer, tokens, parser, and AST for the narrow v0 surface language.
//!
//! Designed per book ch. 7 + Appendix D (normative EBNF).
//! - Hand-written recursive descent + binding-power parser (no generator).
//! - Longest-match for >>> and *** (indivisible tokens).
//! - Nestable block comments {- -}.
//! - Deterministic recovery to sync points.
//! - Spans on every significant node.
//!
//! The goal is boring, stable, auditable parsing so that every later artifact
//! (HIR, summaries, CGIR, emitted Rust) is reproducible.

pub mod span;
pub mod token;
pub mod lexer;
pub mod ast;
pub mod parser;

pub use span::{SourceId, Span, Spanned};
pub use token::{Token, TokenKind};
pub use lexer::Lexer;
pub use ast::*;
pub use parser::{parse, ParseError};
