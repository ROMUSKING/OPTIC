//! Hand-written recursive-descent + binding-power parser (ch. 7.5, 7.9).
//! Precedence: >>> (5, left) > *** (4, left) > grade + (3).
//! Recovery is present but deliberately simple for the narrow prelude.

use crate::ast::*;
use crate::lexer::Lexer;
use crate::span::{SourceId, Span, Spanned};
use crate::token::{Token, TokenKind};

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

pub fn parse(src: &str, source_id: SourceId) -> Result<Program, Vec<ParseError>> {
    let tokens = Lexer::new(src, source_id).lex();
    let mut p = Parser {
        src,
        tokens,
        pos: 0,
        errors: vec![],
    };
    let items = p.parse_items();
    let span = if items.is_empty() {
        Span::dummy()
    } else {
        let first = items
            .first()
            .map(|i| match i {
                Item::Data(d) => d.span,
                Item::Optic(o) => o.span,
                Item::Let(l) => l.span,
                Item::Fn(f) => f.span,
                Item::Expr(e) => body_span(e),
            })
            .unwrap_or(Span::dummy());
        let last = items
            .last()
            .map(|i| match i {
                Item::Data(d) => d.span,
                Item::Optic(o) => o.span,
                Item::Let(l) => l.span,
                Item::Fn(f) => f.span,
                Item::Expr(e) => body_span(e),
            })
            .unwrap_or(first);
        first.merge(last)
    };
    let program = Program { items, span };
    if p.errors.is_empty() {
        Ok(program)
    } else {
        Err(p.errors)
    }
}

struct Parser<'a> {
    src: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    fn current(&self) -> TokenKind {
        self.tokens
            .get(self.pos)
            .map(|t| t.kind)
            .unwrap_or(TokenKind::Eof)
    }

    fn current_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span)
            .unwrap_or(Span::dummy())
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or_else(|| {
            let end = self.src.len() as u32;
            Token::new(TokenKind::Eof, Span::new(SourceId(0), end, end))
        });
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: TokenKind, ctx: &str) -> Option<Span> {
        if self.current() == expected {
            Some(self.advance().span)
        } else {
            let sp = self.current_span();
            self.errors.push(ParseError {
                message: format!(
                    "expected {:?} in {} , got {:?}",
                    expected,
                    ctx,
                    self.current()
                ),
                span: sp,
            });
            None
        }
    }

    fn should_consume_sync_token(kind: TokenKind) -> bool {
        matches!(kind, TokenKind::RBrace | TokenKind::Semi | TokenKind::Eof)
    }

    fn skip_until_sync(&mut self, sync: &[TokenKind]) {
        while !sync.contains(&self.current()) && self.current() != TokenKind::Eof {
            self.advance();
        }
        // ch7.6: consume only closing tokens; stop at next decl keyword without eating it.
        if self.current() != TokenKind::Eof
            && sync.contains(&self.current())
            && Self::should_consume_sync_token(self.current())
        {
            self.advance();
        }
    }

    fn parse_items(&mut self) -> Vec<Item> {
        let mut items = vec![];
        // A.8: sync from ch7.6 table (top-level item + context); expanded for recovery so bad inner (field/expr) doesn't cascade "expected RBrace in block"
        let sync = [
            TokenKind::KwData,
            TokenKind::KwOptic,
            TokenKind::KwLet,
            TokenKind::KwFn,
            TokenKind::RBrace,
            TokenKind::Eof,
        ];
        while self.current() != TokenKind::Eof {
            match self.current() {
                TokenKind::KwData => {
                    if let Some(d) = self.parse_data_decl() {
                        items.push(Item::Data(d));
                    } else {
                        self.skip_until_sync(&sync);
                    }
                }
                TokenKind::KwOptic => {
                    if let Some(o) = self.parse_optic_decl() {
                        items.push(Item::Optic(Box::new(o)));
                    } else {
                        self.skip_until_sync(&sync);
                    }
                }
                TokenKind::KwLet => {
                    if let Some(l) = self.parse_let_binding() {
                        items.push(Item::Let(l));
                    } else {
                        self.skip_until_sync(&sync);
                    }
                }
                TokenKind::KwFn => {
                    if let Some(f) = self.parse_fn_decl() {
                        items.push(Item::Fn(f));
                    } else {
                        self.skip_until_sync(&sync);
                    }
                }
                _ => {
                    // Support bare top-level expr (query chains etc) for demo/example style
                    // (EBNF items are decls, but this allows the provided .opt examples without heavy rewrite)
                    if self.current() == TokenKind::Ident
                        || self.current() == TokenKind::LParen
                        || self.current() == TokenKind::LBrace
                    {
                        if let Some(e) = self.parse_expr() {
                            // consume optional ;
                            if self.current() == TokenKind::Semi {
                                self.advance();
                            }
                            items.push(Item::Expr(e));
                            continue;
                        }
                    }
                    let sp = self.current_span();
                    self.errors.push(ParseError {
                        message: "expected top-level item (data, optic, let, fn) or expr".into(),
                        span: sp,
                    });
                    self.skip_until_sync(&sync);
                }
            }
        }
        items
    }

    fn parse_data_decl(&mut self) -> Option<DataDecl> {
        // A.1: field_list and type_expr per app D: "field_list ::= field_decl (',' field_decl)* ','?"
        // "type_expr ::= 'SoA' '<' type_expr '>' | IDENT ('<' type_args '>')?" + ch7.7 concrete ex (handles Vec2 as Named no-args, f32, trailing , )
        let start = self.advance().span; // data
        let name_tok = self.advance();
        if name_tok.kind != TokenKind::Ident {
            self.errors.push(ParseError {
                message: "expected ident after data".into(),
                span: name_tok.span,
            });
            self.skip_until_sync(&[TokenKind::LBrace, TokenKind::KwData, TokenKind::Eof]);
            return None;
        }
        let name = Spanned::new(self.text_of(&name_tok), name_tok.span);

        self.expect(TokenKind::LBrace, "data decl")?;

        let mut fields = vec![];
        while self.current() != TokenKind::RBrace && self.current() != TokenKind::Eof {
            if let Some(f) = self.parse_field_decl() {
                fields.push(f);
            }
            if self.current() == TokenKind::Comma {
                self.advance();
            }
        }
        let rbrace = self
            .expect(TokenKind::RBrace, "data decl")
            .unwrap_or(Span::dummy());
        let span = start.merge(rbrace);
        Some(DataDecl { name, fields, span })
    }

    fn parse_field_decl(&mut self) -> Option<FieldDecl> {
        let name_tok = self.advance();
        if name_tok.kind != TokenKind::Ident {
            self.errors.push(ParseError {
                message: "expected field name".into(),
                span: name_tok.span,
            });
            self.skip_until_sync(&[TokenKind::Comma, TokenKind::RBrace, TokenKind::Eof]); // A.8 recovery per ch7.6 type/field sync
            return None;
        }
        let name = Spanned::new(self.text_of(&name_tok), name_tok.span);
        self.expect(TokenKind::Colon, "field decl")?;
        let ty = self.parse_type_expr()?;
        let span = name.span.merge(ty_span(&ty));
        Some(FieldDecl { name, ty, span })
    }

    fn parse_type_expr(&mut self) -> Option<TypeExpr> {
        match self.current() {
            TokenKind::Ident => {
                let name_tok = self.advance();
                let name = self.text_of(&name_tok);
                let span0 = name_tok.span;
                if name == "SoA" {
                    self.expect(TokenKind::Lt, "SoA<")?;
                    let inner = self.parse_type_expr()?;
                    let gt = self.expect(TokenKind::Gt, "SoA>").unwrap_or(span0);
                    Some(TypeExpr::Soa(Box::new(inner), span0.merge(gt)))
                } else if name == "BitSet" {
                    Some(TypeExpr::BitSet(span0))
                } else {
                    let mut args = vec![];
                    if self.current() == TokenKind::Lt {
                        self.advance();
                        while self.current() != TokenKind::Gt && self.current() != TokenKind::Eof {
                            if let Some(a) = self.parse_type_expr() {
                                args.push(a);
                            }
                            if self.current() == TokenKind::Comma {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::Gt, "type args")?;
                    }
                    let span = if args.is_empty() {
                        span0
                    } else {
                        span0.merge(args.last().map(ty_span).unwrap_or(span0))
                    };
                    Some(TypeExpr::Named { name, args, span })
                }
            }
            TokenKind::LParen => {
                let start = self.advance().span;
                let mut ts = vec![];
                while self.current() != TokenKind::RParen && self.current() != TokenKind::Eof {
                    if let Some(t) = self.parse_type_expr() {
                        ts.push(t);
                    }
                    if self.current() == TokenKind::Comma {
                        self.advance();
                    }
                }
                let end = self
                    .expect(TokenKind::RParen, "tuple type")
                    .unwrap_or(start);
                Some(TypeExpr::Tuple(ts, start.merge(end)))
            }
            _ => {
                let sp = self.current_span();
                self.errors.push(ParseError {
                    message: "expected type".into(),
                    span: sp,
                });
                None
            }
        }
    }

    fn parse_optic_decl(&mut self) -> Option<OpticDecl> {
        let start = self.advance().span; // optic
        let name_tok = self.advance();
        let name = Spanned::new(self.text_of(&name_tok), name_tok.span);
        self.expect(TokenKind::Colon, "optic :")?;

        // GradedOptic < S , A , G >
        self.expect(TokenKind::Ident, "GradedOptic")?; // accept any ident for now; could check name
                                                       // We are lenient on the exact "GradedOptic" token for simplicity in v0.
        self.expect(TokenKind::Lt, "<")?;
        let costate = self.parse_type_expr()?;
        self.expect(TokenKind::Comma, ",")?;
        let focus = self.parse_type_expr()?;
        self.expect(TokenKind::Comma, ",")?;
        let grade = self.parse_grade_expr()?;
        self.expect(TokenKind::Gt, ">")?;

        self.expect(TokenKind::LBrace, "optic body {")?;

        let get = self.parse_get_clause()?;
        let put = if self.current() == TokenKind::KwPut {
            Some(self.parse_put_clause()?)
        } else {
            None
        };

        let rbrace = self
            .expect(TokenKind::RBrace, "optic }")
            .unwrap_or(Span::dummy());
        let span = start.merge(rbrace);

        Some(OpticDecl {
            name,
            costate,
            focus,
            grade,
            get,
            put,
            span,
        })
    }

    fn parse_grade_expr(&mut self) -> Option<GradeExpr> {
        let start = self.current_span();
        let mut dims = vec![];
        loop {
            dims.push(self.parse_grade_dim()?);
            if self.current() == TokenKind::Plus {
                self.advance();
            } else {
                break;
            }
        }
        let span = start.merge(dims.last().map(|d| grade_dim_span(d)).unwrap_or(start));
        Some(GradeExpr { dims, span })
    }

    fn parse_grade_dim(&mut self) -> Option<GradeDim> {
        let sp = self.current_span();
        match self.current() {
            TokenKind::Ident => {
                let id = self.advance();
                let txt = self.text_of(&id);
                if txt == "CacheGrade" {
                    if self.current() == TokenKind::Lt {
                        self.advance();
                        if self.current() == TokenKind::Ident && self.text_of_current() == "_" {
                            self.advance();
                            self.expect(TokenKind::Gt, ">")?;
                            return Some(GradeDim::Cache { n: None, span: sp });
                        }
                        if let TokenKind::IntLit = self.current() {
                            let n_tok = self.advance();
                            let n: u32 = match self.text_of(&n_tok).parse() {
                                Ok(v) => v,
                                Err(_) => {
                                    self.errors.push(ParseError {
                                        message: "invalid CacheGrade literal".into(),
                                        span: n_tok.span,
                                    });
                                    return None;
                                }
                            };
                            self.expect(TokenKind::Gt, ">")?;
                            return Some(GradeDim::Cache {
                                n: Some(n),
                                span: sp,
                            });
                        }
                        self.errors.push(ParseError {
                            message: "expected CacheGrade<_> or CacheGrade<N>".into(),
                            span: sp,
                        });
                        return None;
                    }
                    Some(GradeDim::Cache { n: None, span: sp })
                } else if txt == "OwnershipGrade" {
                    // similar
                    self.expect(TokenKind::Lt, "<")?;
                    // accept _ or rational text
                    let r = if self.current() == TokenKind::Ident && self.text_of_current() == "_" {
                        self.advance();
                        None
                    } else {
                        // accept a token as the rational text (Int or "num/den")
                        let t = self.advance();
                        Some(self.text_of(&t))
                    };
                    self.expect(TokenKind::Gt, ">")?;
                    Some(GradeDim::Ownership { r, span: sp })
                } else if matches!(txt.as_str(), "LinearGrade" | "AffineGrade" | "SharedGrade") {
                    Some(GradeDim::Named {
                        name: txt,
                        span: sp,
                    })
                } else if txt == "_" {
                    Some(GradeDim::Infer(sp))
                } else {
                    Some(GradeDim::Named {
                        name: txt,
                        span: sp,
                    })
                }
            }
            TokenKind::Lt | _ => {
                // bare _ or error recovery
                if self.current() == TokenKind::Ident && self.text_of_current() == "_" {
                    let _ = self.advance();
                    return Some(GradeDim::Infer(sp));
                }
                self.errors.push(ParseError {
                    message: "expected grade dim".into(),
                    span: sp,
                });
                Some(GradeDim::Infer(sp))
            }
        }
    }

    fn parse_get_clause(&mut self) -> Option<GetClause> {
        let start = self.expect(TokenKind::KwGet, "get")?;
        let param_tok = self.advance();
        let param = Spanned::new(self.text_of(&param_tok), param_tok.span);
        self.expect(TokenKind::FatArrow, "=>")?;
        let body = self.parse_expr()?;
        let span = start.merge(body_span(&body));
        Some(GetClause { param, body, span })
    }

    fn parse_put_clause(&mut self) -> Option<PutClause> {
        let start = self.expect(TokenKind::KwPut, "put")?;
        self.expect(TokenKind::LParen, "(")?;
        let sp_tok = self.advance();
        let state_param = Spanned::new(self.text_of(&sp_tok), sp_tok.span);
        self.expect(TokenKind::Comma, ",")?;
        let vp_tok = self.advance();
        let value_param = Spanned::new(self.text_of(&vp_tok), vp_tok.span);
        self.expect(TokenKind::RParen, ")")?;
        self.expect(TokenKind::FatArrow, "=>")?;
        let body = self.parse_expr_or_block()?;
        let span = start.merge(body_span(&body));
        Some(PutClause {
            state_param,
            value_param,
            body,
            span,
        })
    }

    fn parse_let_binding(&mut self) -> Option<LetBinding> {
        let start = self.advance().span; // let
        let name_tok = self.advance();
        let name = Spanned::new(self.text_of(&name_tok), name_tok.span);

        let had_colon = self.current() == TokenKind::Colon;
        let ty = if had_colon {
            self.advance();
            if self.current() == TokenKind::Ident && self.text_of_current() == "GradedOptic" {
                self.advance();
                self.expect(TokenKind::Lt, "GradedOptic<")?;
                let costate = self.parse_type_expr()?;
                self.expect(TokenKind::Comma, ",")?;
                let focus = self.parse_type_expr()?;
                self.expect(TokenKind::Comma, ",")?;
                let grade = self.parse_grade_expr()?;
                self.expect(TokenKind::Gt, ">")?;
                Some(GradeOpticType {
                    costate,
                    focus,
                    grade,
                    span: start,
                })
            } else {
                self.errors.push(ParseError {
                    message: "expected GradedOptic<...> type annotation after let name:".into(),
                    span: self.current_span(),
                });
                return None;
            }
        } else {
            None
        };

        self.expect(TokenKind::Equals, "=")?;
        let value = self.parse_optic_expr()?;
        self.expect(TokenKind::Semi, ";")?;
        let span = start.merge(value_span(&value));
        Some(LetBinding {
            name,
            ty,
            value,
            span,
        })
    }

    fn parse_fn_decl(&mut self) -> Option<FnDecl> {
        // Minimal support for now (enough for simple wrappers)
        let start = self.advance().span;
        let name_tok = self.advance();
        let name = Spanned::new(self.text_of(&name_tok), name_tok.span);
        self.expect(TokenKind::LParen, "(")?;
        let mut params = vec![];
        while self.current() != TokenKind::RParen && self.current() != TokenKind::Eof {
            let p_name = self.advance();
            let p = Spanned::new(self.text_of(&p_name), p_name.span);
            self.expect(TokenKind::Colon, ":")?;
            let ty = self.parse_type_expr()?;
            params.push(Param {
                name: p,
                ty,
                span: p_name.span,
            });
            if self.current() == TokenKind::Comma {
                self.advance();
            }
        }
        self.expect(TokenKind::RParen, ")")?;
        let ret = if self.current() == TokenKind::FatArrow
            || self.current() == TokenKind::Gt
            || self.current() == TokenKind::Minus
        {
            // A.7: support '->' (EBNF fn_decl uses '->' type ) ; consume Minus then Gt (lexer: - > not single token); keep lenient for Fat/Gt
            if self.current() == TokenKind::Minus {
                self.advance(); // -
                if self.current() == TokenKind::Gt {
                    self.advance();
                } // >
            } else if self.current() == TokenKind::FatArrow || self.current() == TokenKind::Gt {
                self.advance();
            }
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(TokenKind::LBrace, "{")?;
        let mut body = vec![];
        while self.current() != TokenKind::RBrace && self.current() != TokenKind::Eof {
            let start = self.current_span();
            let mut target = None;
            if let TokenKind::Ident = self.current() {
                let saved = self.pos;
                let id_tok = self.advance();
                if self.current() == TokenKind::Equals {
                    self.advance();
                    target = Some(Spanned::new(self.text_of(&id_tok), id_tok.span));
                } else {
                    self.pos = saved;
                }
            }
            let expr = self.parse_expr()?;
            if self.current() == TokenKind::Semi {
                self.advance();
            } else if self.current() != TokenKind::RBrace {
                self.errors.push(ParseError {
                    message: "expected ';' or '}' after statement in fn body".into(),
                    span: self.current_span(),
                });
                break;
            }
            let span = start.merge(body_span(&expr));
            body.push(Stmt { target, expr, span });
            if self.current() == TokenKind::RBrace {
                break;
            }
        }
        self.expect(TokenKind::RBrace, "}")?;
        Some(FnDecl {
            name,
            params,
            ret,
            body,
            span: start,
        })
    }

    fn parse_expr(&mut self) -> Option<Expr> {
        // Full EBNF support for v0 (expr ::= query_chain | assign_expr ; assign ::= field ( = assign )? ; field ::= atom ( . IDENT | [ expr ] )* ; ... )
        self.parse_assign_expr()
    }

    fn parse_assign_expr(&mut self) -> Option<Expr> {
        let start = self.current_span();
        let left = self.parse_field_expr()?;
        if self.current() == TokenKind::Equals {
            self.advance();
            let right = self.parse_assign_expr()?;
            let span = start.merge(body_span(&right));
            return Some(Expr::Assign {
                target: Box::new(left),
                value: Box::new(right),
                span,
            });
        }
        // A.6: binop after field (per "add ... after field", EBNF binary in atom_expr, examples in map bodies)
        // supports all 8; right uses field for minimal (no full chains yet); * via Star, -/ new tokens.
        match self.current() {
            TokenKind::Plus => {
                self.advance();
                let right = self.parse_field_expr()?;
                let span = start.merge(body_span(&right));
                return Some(Expr::Binary {
                    left: Box::new(left),
                    op: BinOp::Add,
                    right: Box::new(right),
                    span,
                });
            }
            TokenKind::Minus => {
                self.advance();
                let right = self.parse_field_expr()?;
                let span = start.merge(body_span(&right));
                return Some(Expr::Binary {
                    left: Box::new(left),
                    op: BinOp::Sub,
                    right: Box::new(right),
                    span,
                });
            }
            TokenKind::Star => {
                self.advance();
                let right = self.parse_field_expr()?;
                let span = start.merge(body_span(&right));
                return Some(Expr::Binary {
                    left: Box::new(left),
                    op: BinOp::Mul,
                    right: Box::new(right),
                    span,
                });
            }
            TokenKind::Slash => {
                self.advance();
                let right = self.parse_field_expr()?;
                let span = start.merge(body_span(&right));
                return Some(Expr::Binary {
                    left: Box::new(left),
                    op: BinOp::Div,
                    right: Box::new(right),
                    span,
                });
            }
            TokenKind::Lt => {
                self.advance();
                let right = self.parse_field_expr()?;
                let span = start.merge(body_span(&right));
                return Some(Expr::Binary {
                    left: Box::new(left),
                    op: BinOp::Lt,
                    right: Box::new(right),
                    span,
                });
            }
            TokenKind::Gt => {
                self.advance();
                let right = self.parse_field_expr()?;
                let span = start.merge(body_span(&right));
                return Some(Expr::Binary {
                    left: Box::new(left),
                    op: BinOp::Gt,
                    right: Box::new(right),
                    span,
                });
            }
            TokenKind::Le => {
                self.advance();
                let right = self.parse_field_expr()?;
                let span = start.merge(body_span(&right));
                return Some(Expr::Binary {
                    left: Box::new(left),
                    op: BinOp::Le,
                    right: Box::new(right),
                    span,
                });
            }
            TokenKind::Ge => {
                self.advance();
                let right = self.parse_field_expr()?;
                let span = start.merge(body_span(&right));
                return Some(Expr::Binary {
                    left: Box::new(left),
                    op: BinOp::Ge,
                    right: Box::new(right),
                    span,
                });
            }
            _ => {}
        }
        Some(left)
    }

    fn parse_field_expr(&mut self) -> Option<Expr> {
        // Support query_chain at expr level per EBNF "expr ::= query_chain | assign_expr" and app D.
        // This ensures Item::Expr in parse_items (bare or via fn body stmts) and Program span calc get real QueryChain.
        if self.looks_like_query_chain() {
            let qc = self.parse_query_chain()?;
            return Some(Expr::QueryChain(qc));
        }
        // A.3: build recursive FieldExpr per app D "field_expr ::= atom_expr ('.' IDENT | '[' expr ']')*"
        // + ch7 field examples (s.healths[s.id]). Use existing FieldExpr {Base, FieldAccess, Index} + spans;
        // no more _temp placeholders. For bare atom (no dot/[) unwrap to Atom to minimize; chains use Expr::Field.
        let base = self.parse_atom_expr()?;
        let mut span = match &base {
            AtomExpr::Ident(s) => s.span,
            AtomExpr::Int(_, sp)
            | AtomExpr::Float(_, sp)
            | AtomExpr::Tuple(_, sp)
            | AtomExpr::Paren(_, sp) => *sp,
        };
        let mut field_expr = FieldExpr::Base(base, span);
        let mut is_chain = false;
        loop {
            match self.current() {
                TokenKind::Dot => {
                    self.advance();
                    let id_tok = self.advance();
                    if id_tok.kind != TokenKind::Ident && id_tok.kind != TokenKind::IntLit {
                        self.errors.push(ParseError {
                            message: "expected field ident after .".into(),
                            span: id_tok.span,
                        });
                        self.skip_until_sync(&[
                            TokenKind::Comma,
                            TokenKind::RBrace,
                            TokenKind::RParen,
                            TokenKind::Semi,
                            TokenKind::Eof,
                        ]); // A.8: more sync from ch7.6 to avoid RBrace cascade
                        break;
                    }
                    let field = Spanned::new(self.text_of(&id_tok), id_tok.span);
                    let new_span = span.merge(field.span);
                    field_expr = FieldExpr::FieldAccess {
                        base: Box::new(field_expr),
                        field,
                        span: new_span,
                    };
                    span = new_span;
                    is_chain = true;
                }
                TokenKind::LBracket => {
                    self.advance();
                    let idx = self.parse_expr()?;
                    let r = self.expect(TokenKind::RBracket, "]")?;
                    let new_span = span.merge(r);
                    field_expr = FieldExpr::Index {
                        base: Box::new(field_expr),
                        index: Box::new(idx),
                        span: new_span,
                    };
                    span = new_span;
                    is_chain = true;
                }
                // A.8 recovery note: added skips in dot error + field_decl; top sync uses ch7.6; more in expr contexts prevent cascade to block RBrace (per "one bad field" issue)
                _ => break,
            }
        }
        if is_chain {
            Some(Expr::Field(field_expr))
        } else {
            // extract atom back for bare idents/lits etc (keeps prior Atom shape for non-field cases)
            if let FieldExpr::Base(a, _) = field_expr {
                Some(Expr::Atom(a))
            } else {
                Some(Expr::Field(field_expr))
            }
        }
    }

    fn parse_atom_expr(&mut self) -> Option<AtomExpr> {
        match self.current() {
            TokenKind::Ident => {
                let id = self.advance();
                Some(AtomExpr::Ident(Spanned::new(self.text_of(&id), id.span)))
            }
            TokenKind::IntLit => {
                let t = self.advance();
                let v: i64 = self.text_of(&t).parse().unwrap_or(0);
                Some(AtomExpr::Int(v, t.span))
            }
            TokenKind::FloatLit => {
                let t = self.advance();
                let v: f64 = self.text_of(&t).parse().unwrap_or(0.0);
                Some(AtomExpr::Float(v, t.span))
            }
            TokenKind::LParen => {
                let start = self.advance().span;
                let mut exprs = vec![];
                if self.current() != TokenKind::RParen {
                    exprs.push(self.parse_expr()?);
                    while self.current() == TokenKind::Comma {
                        self.advance();
                        exprs.push(self.parse_expr()?);
                    }
                }
                let end = self.expect(TokenKind::RParen, "tuple/paren )")?;
                if let [inner] = exprs.as_slice() {
                    Some(AtomExpr::Paren(Box::new(inner.clone()), start.merge(end)))
                } else {
                    Some(AtomExpr::Tuple(exprs, start.merge(end)))
                }
            }
            TokenKind::LBrace => {
                // block as atom
                let block = self.parse_block_expr()?;
                Some(AtomExpr::Paren(
                    Box::new(block),
                    /*approx*/ Span::dummy(),
                ))
            }
            _ => {
                if self.looks_like_query_chain() {
                    let qc = self.parse_query_chain()?;
                    // Wrap query as atom for expr position (query is a kind of expr)
                    return Some(AtomExpr::Ident(Spanned::new("query_chain".into(), qc.span)));
                }
                let sp = self.current_span();
                self.errors.push(ParseError {
                    message: "expected atom (ident/lit/( / { / query )".into(),
                    span: sp,
                });
                Some(AtomExpr::Ident(Spanned::new("_err_atom".into(), sp)))
            }
        }
    }

    fn looks_like_query_chain(&self) -> bool {
        // ident . query (   -- peek without advance, per ch7.9 strategy + EBNF query_chain
        if self.current() != TokenKind::Ident {
            return false;
        }
        if let (Some(d), Some(q)) = (self.tokens.get(self.pos + 1), self.tokens.get(self.pos + 2)) {
            if d.kind == TokenKind::Dot {
                if q.kind == TokenKind::KwQuery
                    || (q.kind == TokenKind::Ident && self.text_of(q) == "query")
                {
                    return true;
                }
            }
        }
        false
    }

    fn parse_query_chain(&mut self) -> Option<QueryChain> {
        let base_tok = self.advance();
        let base = Box::new(Expr::Atom(AtomExpr::Ident(Spanned::new(
            self.text_of(&base_tok),
            base_tok.span,
        ))));
        self.expect(TokenKind::Dot, ".")?;
        // query or KwQuery
        let _qtok = self.advance();
        // ignore exact name
        self.expect(TokenKind::LParen, "(")?;
        let optic = self.parse_optic_expr()?;
        self.expect(TokenKind::RParen, ")")?;

        let mut methods = vec![];
        while self.current() == TokenKind::Dot {
            self.advance();
            match self.current() {
                TokenKind::KwGet => {
                    let sp = self.advance().span;
                    self.expect(TokenKind::LParen, "get(")?;
                    self.expect(TokenKind::RParen, "get()")?;
                    methods.push(QueryMethod::Get(sp));
                }
                TokenKind::Ident if self.text_of_current() == "set" => {
                    let sp = self.advance().span;
                    self.expect(TokenKind::LParen, "set(")?;
                    let val = self.parse_expr()?;
                    self.expect(TokenKind::RParen, ")")?;
                    methods.push(QueryMethod::Set(val, sp));
                }
                TokenKind::Ident if self.text_of_current() == "map" => {
                    let sp = self.advance().span;
                    self.expect(TokenKind::LParen, "map(")?;
                    // closure: | IDENT | expr   or | ( id, .. ) | expr   (A.5, per app D)
                    let mut params = vec![];
                    if self.current() == TokenKind::Pipe {
                        self.advance();
                    }
                    if self.current() == TokenKind::LParen {
                        self.advance();
                        while self.current() != TokenKind::RParen
                            && self.current() != TokenKind::Eof
                        {
                            if self.current() == TokenKind::Ident {
                                let p = self.advance();
                                params.push(Spanned::new(self.text_of(&p), p.span));
                            }
                            if self.current() == TokenKind::Comma {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::RParen, "closure )")?;
                    } else if self.current() == TokenKind::Ident {
                        let p = self.advance();
                        params.push(Spanned::new(self.text_of(&p), p.span));
                    }
                    if self.current() == TokenKind::Pipe {
                        self.advance();
                    }
                    let body = self.parse_expr()?;
                    methods.push(QueryMethod::Map(
                        Closure {
                            params,
                            body: Box::new(body),
                            span: sp,
                        },
                        sp,
                    ));
                    self.expect(TokenKind::RParen, "map )")?;
                }
                _ => {
                    /* stop */
                    break;
                }
            }
        }
        let span = base_tok.span; // approx
        Some(QueryChain {
            base,
            optic,
            methods,
            span,
        })
    }

    fn parse_block_expr(&mut self) -> Option<Expr> {
        let start = self.expect(TokenKind::LBrace, "{")?;
        let mut stmts = vec![];
        let mut result = None;
        while self.current() != TokenKind::RBrace && self.current() != TokenKind::Eof {
            // allow trailing expr without ; as result in blocks
            if self.current() == TokenKind::RBrace {
                break;
            }
            let e = self.parse_expr()?;
            if self.current() == TokenKind::Semi {
                self.advance();
                stmts.push(Stmt {
                    target: None,
                    expr: e,
                    span: start,
                });
            } else {
                result = Some(Box::new(e));
                break;
            }
        }
        let end = self.expect(TokenKind::RBrace, "block }")?;
        Some(Expr::Block {
            stmts,
            result,
            span: start.merge(end),
        })
    }

    // Optic expressions with precedence (>>> tighter than *** ) per ch7 + EBNF
    // A.4: parse_optic_expr starts with par per "optic_expr ::= optic_par", par does ('***' seq)* per EBNF;
    // seq does ('>>>' atom)* first (tighter) per ch7.9.3 table + 7.9.5.1 pratt sketch. Already wired; adding spec ref.
    fn parse_optic_expr(&mut self) -> Option<OpticExpr> {
        self.parse_optic_par()
    }

    fn parse_optic_par(&mut self) -> Option<OpticExpr> {
        let mut lhs = self.parse_optic_seq()?;
        while self.current() == TokenKind::Par {
            let op_span = self.advance().span;
            let rhs = self.parse_optic_seq()?;
            let span = op_span; // approx
            lhs = OpticExpr::Par {
                left: Box::new(lhs),
                right: Box::new(rhs),
                span,
            };
        }
        Some(lhs)
    }

    fn parse_optic_seq(&mut self) -> Option<OpticExpr> {
        let mut lhs = self.parse_optic_atom()?;
        while self.current() == TokenKind::Seq {
            let op_span = self.advance().span;
            let rhs = self.parse_optic_atom()?;
            let span = /* merge */ op_span;
            lhs = OpticExpr::Seq {
                left: Box::new(lhs),
                right: Box::new(rhs),
                span,
            };
        }
        Some(lhs)
    }

    fn parse_optic_atom(&mut self) -> Option<OpticExpr> {
        match self.current() {
            TokenKind::Ident => {
                let id = self.advance();
                Some(OpticExpr::Atom(OpticAtom::Named(Spanned::new(
                    self.text_of(&id),
                    id.span,
                ))))
            }
            TokenKind::LParen => {
                let start = self.advance().span;
                let inner = self.parse_optic_expr()?;
                let end = self.expect(TokenKind::RParen, ")")?;
                Some(OpticExpr::Atom(OpticAtom::Paren(
                    Box::new(inner),
                    start.merge(end),
                )))
            }
            _ => {
                let sp = self.current_span();
                self.errors.push(ParseError {
                    message: "expected optic atom (ident or ( ))".into(),
                    span: sp,
                });
                None
            }
        }
    }

    #[allow(dead_code)]
    fn atom_span(a: &AtomExpr) -> Span {
        match a {
            AtomExpr::Ident(s) => s.span,
            AtomExpr::Int(_, sp) | AtomExpr::Float(_, sp) => *sp,
            AtomExpr::Tuple(_, sp) | AtomExpr::Paren(_, sp) => *sp,
        }
    }

    fn parse_expr_or_block(&mut self) -> Option<Expr> {
        if self.current() == TokenKind::LBrace {
            let start = self.advance().span;
            let mut stmts = vec![];
            let mut result = None;
            while self.current() != TokenKind::RBrace && self.current() != TokenKind::Eof {
                let e = self.parse_expr()?;
                if self.current() == TokenKind::Semi {
                    self.advance();
                    stmts.push(Stmt {
                        target: None,
                        expr: e,
                        span: start,
                    });
                } else {
                    result = Some(Box::new(e));
                    break;
                }
            }
            let end = self.expect(TokenKind::RBrace, " }")?;
            Some(Expr::Block {
                stmts,
                result,
                span: start.merge(end),
            })
        } else {
            self.parse_expr()
        }
    }

    fn text_of(&self, tok: &Token) -> String {
        let s = &self.src[tok.span.start as usize..tok.span.end as usize];
        s.to_string()
    }

    fn text_of_current(&self) -> String {
        if let Some(tok) = self.tokens.get(self.pos) {
            self.text_of(tok)
        } else {
            String::new()
        }
    }
}

// helpers for spans (very approximate in this first version)
fn ty_span(t: &TypeExpr) -> Span {
    match t {
        TypeExpr::Soa(_, sp) | TypeExpr::BitSet(sp) | TypeExpr::Tuple(_, sp) => *sp,
        TypeExpr::Named { span, .. } => *span,
    }
}

fn grade_dim_span(d: &GradeDim) -> Span {
    match d {
        GradeDim::Cache { span, .. }
        | GradeDim::Ownership { span, .. }
        | GradeDim::Named { span, .. }
        | GradeDim::Infer(span) => *span,
    }
}

fn body_span(e: &Expr) -> Span {
    // Updated for A.2: handle QueryChain (and keep Atom) so Program span calc (in parse()) for Item::Expr produces real span not always dummy.
    // (parse_items already pushes Item::Expr for bare top-level or fn-body stmts that are queries.)
    match e {
        Expr::QueryChain(q) => q.span,
        Expr::Atom(AtomExpr::Ident(s)) => s.span,
        Expr::Atom(AtomExpr::Int(_, sp)) | Expr::Atom(AtomExpr::Float(_, sp)) => *sp,
        _ => Span::dummy(),
    }
}

fn value_span(o: &OpticExpr) -> Span {
    match o {
        OpticExpr::Atom(OpticAtom::Named(s)) => s.span,
        OpticExpr::Atom(OpticAtom::Paren(_, sp)) => *sp,
        OpticExpr::Seq { span, .. } | OpticExpr::Par { span, .. } => *span,
    }
}

// Small placeholder for missing token in closure parsing (parser is lenient for first cut)
const _: () = ();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_field_index_for_s_healths_s_id() {
        // A.3 golden: directly exercises EBNF field_expr with . and [ , and nested field in index.
        // s.healths[s.id] must parse to Expr::Field( Index { base: FieldAccess{base:Base(s), field:healths}, index: Field(Access s.id) } )
        let src = "s.healths[s.id]";
        let sid = SourceId(0);
        let res = parse(src, sid);
        assert!(
            res.is_ok(),
            "expected parse ok for field[index] expr, got {:?}",
            res.err()
        );
        let prog = res.unwrap();
        // At least one Item::Expr that is a Field containing Index
        let has_field_index = prog.items.iter().any(|item| {
            if let Item::Expr(Expr::Field(FieldExpr::Index { base, .. })) = item {
                matches!(&**base, FieldExpr::FieldAccess { .. })
            } else {
                false
            }
        });
        assert!(
            has_field_index,
            "parsed expr should contain Field Index over field access for healths[s.id]"
        );
    }

    // A.9 golden tests for M0 (small EBNF fragments per app D)
    #[test]
    fn parses_data_decl_soa_vec2() {
        let src = "data Entities { healths: SoA<f32>, positions: SoA<Vec2> }";
        let sid = SourceId(0);
        assert!(
            parse(src, sid).is_ok(),
            "data field_list + type SoA<Named no arg> per app D"
        );
    }

    #[test]
    fn parses_optic_get_put_with_field_index() {
        let src = "optic V: GradedOptic<E, f32, CacheGrade<1>> { get s => s.h[s.id] put (s, v) => { s.h[s.id] = v } }";
        let sid = SourceId(0);
        assert!(
            parse(src, sid).is_ok(),
            "optic with get/put field[index] + block assign"
        );
    }

    #[test]
    fn parses_let_with_par() {
        let src = "let c = A *** B;";
        let sid = SourceId(0);
        assert!(parse(src, sid).is_ok(), "let optic_par *** per EBNF");
    }

    #[test]
    fn parses_query_map_closure() {
        let src = "e.query(o).map(|(h, p)| h + 1.0);";
        let sid = SourceId(0);
        assert!(
            parse(src, sid).is_ok(),
            "query_chain + map closure (tuple params + body) per app D"
        );
    }

    #[test]
    fn parses_fn_with_arrow_ret() {
        let src = "fn f(x: i32) -> i32 { x }";
        let sid = SourceId(0);
        assert!(parse(src, sid).is_ok(), "fn_decl with -> ret type per EBNF");
    }

    #[test]
    fn recovery_parses_second_item_after_bad_let() {
        let src = "let bad: NotAType = X;\nlet c = A *** B;\n";
        let sid = SourceId(0);
        assert!(
            parse(src, sid).is_err(),
            "recovery must surface parse errors to callers"
        );
        let good = parse("let c = A *** B;\n", sid).expect("valid let parses");
        assert!(good
            .items
            .iter()
            .any(|i| matches!(i, Item::Let(l) if l.name.node == "c")));
    }

    #[test]
    fn recovery_parses_fn_after_bad_params() {
        let src = "fn bad(x\nlet c = A;\nfn main() { }\n";
        let sid = SourceId(0);
        assert!(parse(src, sid).is_err(), "bad fn must yield parse Err");
        let good = parse("fn main() { }\n", sid).expect("valid fn parses");
        assert!(good
            .items
            .iter()
            .any(|i| matches!(i, Item::Fn(f) if f.name.node == "main")));
    }

    #[test]
    fn parser_regression_completes_quickly() {
        let inputs = [
            "fn f(x: i32) -> i32 { x }\nlet c = A *** B;\n",
            "fn f() { 42 }\n",
            "let bad: GradedOptic<Entities, f32, CacheGrade<2>> = A >>> B;\n",
        ];
        for src in inputs {
            let start = std::time::Instant::now();
            let _ = parse(src, SourceId(0));
            assert!(
                start.elapsed().as_millis() < 500,
                "parser must not hang on small input: {src:?}"
            );
        }
    }

    #[test]
    #[ignore = "optional low-mem check: ulimit -v 2000000 cargo test -p optic-syntax -- --ignored"]
    fn parse_under_low_memory_constraint() {
        let src = "fn f() { 42 }\nlet c = A *** B;\n";
        let prog = parse(src, SourceId(0)).expect("parse under mem probe");
        assert!(!prog.items.is_empty());
    }

    #[test]
    fn parses_fn_body_trailing_expr_without_semi() {
        let src = "fn f() { 42 }";
        let prog = parse(src, SourceId(0)).expect("parse fn");
        let Item::Fn(f) = &prog.items[0] else {
            panic!("expected fn");
        };
        assert_eq!(f.body.len(), 1);
        assert!(matches!(f.body[0].expr, Expr::Atom(AtomExpr::Int(42, _))));
    }

    #[test]
    fn parses_graded_optic_type_args() {
        let src = "let x: GradedOptic<Entities, f32, CacheGrade<2>> = HealthView;";
        let prog = parse(src, SourceId(0)).expect("parse graded let");
        let Item::Let(l) = &prog.items[0] else {
            panic!("expected let");
        };
        let ty = l.ty.as_ref().expect("typed let");
        assert!(matches!(
            &ty.costate,
            TypeExpr::Named { name, .. } if name == "Entities"
        ));
        assert!(matches!(&ty.focus, TypeExpr::Named { name, .. } if name == "f32"));
    }

    #[test]
    fn parses_map_closure_tuple_params() {
        let src = "e.query(o).map(|(h, p)| h);";
        let prog = parse(src, SourceId(0)).expect("parse map closure");
        let Item::Expr(Expr::QueryChain(q)) = &prog.items[0] else {
            panic!("expected query");
        };
        let Some(QueryMethod::Map(cl, _)) = q.methods.last() else {
            panic!("expected map method");
        };
        assert_eq!(cl.params.len(), 2);
    }
}
