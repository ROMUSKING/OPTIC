//! Tokens for the v0 surface (ch. 7.9).
//! Longest-match decisions are made in the lexer.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokenKind {
    // Literals & idents
    Ident,
    IntLit,
    FloatLit,

    // Keywords (priority over ident)
    KwData,
    KwOptic,
    KwUnsafe,
    KwExtern,
    KwGet,
    KwPut,
    KwPreview,
    KwReview,
    KwLet,
    KwFn,
    KwQuery, // for the .query( form; not strictly needed
    StringLit,
    // methods in chains are recognized in parser

    // Operators (indivisible)
    Seq,      // >>>
    Par,      // ***
    FatArrow, // =>
    Le,
    Ge, // <= >=  (for future exprs; book has < > etc in bin_op)

    // Punctuation
    Colon,
    Comma,
    Semi,
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Dot,
    Plus, // grade +
    Lt,
    Gt,
    Star, // only appears inside *** ; lone * is error per book
    Equals,
    Pipe,  // | for closures per app D "closure ::= '|' ..."
    Minus, // - for binop per app D
    Slash, // / for binop per app D

    // Comments are never emitted as tokens (lexed away)
    // Eof is implicit
    Eof,

    // Error token for recovery / diagnostics
    Error,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub span: crate::span::Span,
    // For idents/lits we store the text slice (or intern later).
    // For now the parser re-slices the source using the span.
}

impl Token {
    pub fn new(kind: TokenKind, span: crate::span::Span) -> Self {
        Token { kind, span }
    }
}
