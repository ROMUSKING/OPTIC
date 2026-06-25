//! Hand-written lexer per book ch. 7.9 (longest-match, nested block comments, spans).
//! "The longest-match rule is non-negotiable."

use crate::span::{SourceId, Span};
use crate::token::{Token, TokenKind};

pub struct Lexer<'src> {
    src: &'src str,
    pos: usize,
    source_id: SourceId,
    tokens: Vec<Token>,
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src str, source_id: SourceId) -> Self {
        Lexer {
            src,
            pos: 0,
            source_id,
            tokens: Vec::new(),
        }
    }

    pub fn lex(mut self) -> Vec<Token> {
        let mut iter_guard = 0usize;
        while self.pos < self.src.len() {
            iter_guard += 1;
            debug_assert!(
                iter_guard < self.src.len().saturating_add(2),
                "lexer must strictly advance pos (guard against no-progress; +2 per R7)"
            );
            // whitespace is ignored (per app D D.1 and ch7.9 disambiguation)
            if self.current_char().is_whitespace() {
                self.advance();
                continue;
            }
            let start = self.pos;
            let ch = self.current_char();

            // Longest-match multi-char operators first (critical)
            if self.starts_with(">>>") {
                let end = self.pos + 3;
                self.emit(TokenKind::Seq, start, end);
                self.pos = end;
                continue;
            }
            if self.starts_with("***") {
                let end = self.pos + 3;
                self.emit(TokenKind::Par, start, end);
                self.pos = end;
                continue;
            }
            if self.starts_with("=>") {
                let end = self.pos + 2;
                self.emit(TokenKind::FatArrow, start, end);
                self.pos = end;
                continue;
            }
            if self.starts_with("<=") {
                let end = self.pos + 2;
                self.emit(TokenKind::Le, start, end);
                self.pos = end;
                continue;
            }
            if self.starts_with(">=") {
                let end = self.pos + 2;
                self.emit(TokenKind::Ge, start, end);
                self.pos = end;
                continue;
            }

            // Comments (discarded, never tokens)
            if self.starts_with("--") {
                self.skip_line_comment();
                continue;
            }
            if self.starts_with("{-") {
                self.skip_block_comment_nested();
                continue;
            }

            if ch == '"' {
                self.scan_string_literal(start);
                continue;
            }

            if is_ident_start(ch) {
                self.scan_ident_or_keyword(start);
                continue;
            }

            if ch.is_ascii_digit() {
                self.scan_number_literal(start);
                continue;
            }

            // Single-char punctuation or error
            self.scan_single_char_punct_or_error(start);
        }

        // Always emit a final Eof for convenience (span is zero-width at end)
        let end = self.src.len() as u32;
        self.tokens.push(Token::new(
            TokenKind::Eof,
            Span::new(self.source_id, end, end),
        ));
        self.tokens
    }

    fn current_char(&self) -> char {
        self.src[self.pos..].chars().next().unwrap_or('\0')
    }

    fn starts_with(&self, s: &str) -> bool {
        self.src[self.pos..].starts_with(s)
    }

    fn emit(&mut self, kind: TokenKind, start: usize, end: usize) {
        let span = Span::new(self.source_id, start as u32, end as u32);
        self.tokens.push(Token::new(kind, span));
    }

    fn advance(&mut self) {
        if self.pos < self.src.len() {
            let ch = self.current_char();
            self.pos += ch.len_utf8();
        }
    }

    fn scan_ident_or_keyword(&mut self, start: usize) {
        while self.pos < self.src.len() {
            let ch = self.current_char();
            if is_ident_char(ch) {
                self.advance();
            } else {
                break;
            }
        }
        let end = self.pos;
        let text = &self.src[start..end];
        let kind = match text {
            "data" => TokenKind::KwData,
            "optic" => TokenKind::KwOptic,
            "unsafe" => TokenKind::KwUnsafe,
            "extern" => TokenKind::KwExtern,
            "get" => TokenKind::KwGet,
            "put" => TokenKind::KwPut,
            "preview" => TokenKind::KwPreview,
            "partial" => TokenKind::KwPartial,
            "review" => TokenKind::KwReview,
            "traverse" => TokenKind::KwTraverse,
            "update" => TokenKind::KwUpdate,
            "let" => TokenKind::KwLet,
            "fn" => TokenKind::KwFn,
            "query" => TokenKind::KwQuery,
            _ => TokenKind::Ident,
        };
        self.emit(kind, start, end);
    }

    fn scan_string_literal(&mut self, start: usize) {
        self.advance(); // opening "
        while self.pos < self.src.len() {
            let ch = self.current_char();
            if ch == '"' {
                self.advance();
                break;
            }
            if ch == '\\' {
                self.advance();
                if self.pos < self.src.len() {
                    self.advance();
                }
                continue;
            }
            self.advance();
        }
        self.emit(TokenKind::StringLit, start, self.pos);
    }

    fn scan_number_literal(&mut self, start: usize) {
        let mut saw_dot = false;
        while self.pos < self.src.len() {
            let ch = self.current_char();
            if ch.is_ascii_digit() {
                self.advance();
            } else if ch == '.' && !saw_dot {
                // peek next to decide float vs range etc. For v0 simple.
                let next_pos = self.pos + 1;
                if next_pos < self.src.len()
                    && self.src[next_pos..]
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_digit())
                {
                    saw_dot = true;
                    self.advance(); // consume .
                                    // continue to consume digits
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        let end = self.pos;
        let kind = if saw_dot {
            TokenKind::FloatLit
        } else {
            TokenKind::IntLit
        };
        self.emit(kind, start, end);
    }

    fn scan_single_char_punct_or_error(&mut self, start: usize) {
        let ch = self.current_char();
        let kind = match ch {
            ':' => TokenKind::Colon,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semi,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            '.' => TokenKind::Dot,
            '+' => TokenKind::Plus,
            '<' => TokenKind::Lt,
            '>' => TokenKind::Gt,
            '*' => TokenKind::Star, // lone * is invalid per book; we emit and let parser error
            '=' => TokenKind::Equals,
            '|' => TokenKind::Pipe,
            '-' => TokenKind::Minus,
            '/' => TokenKind::Slash,
            _ => TokenKind::Error,
        };
        self.advance();
        let end = self.pos;
        self.emit(kind, start, end);
    }

    fn skip_line_comment(&mut self) {
        // already saw --
        self.pos += 2;
        while self.pos < self.src.len() {
            if self.current_char() == '\n' {
                self.advance();
                return;
            }
            self.advance();
        }
    }

    fn skip_block_comment_nested(&mut self) {
        // already saw {-
        let mut depth = 1u32;
        self.pos += 2;
        while self.pos < self.src.len() && depth > 0 {
            if self.starts_with("{-") {
                depth += 1;
                self.pos += 2;
            } else if self.starts_with("-}") {
                depth -= 1;
                self.pos += 2;
            } else {
                self.advance();
            }
        }
        // if unclosed, we just stop (parser will see Eof and diagnose)
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
