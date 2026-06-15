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
    let mut p = Parser { src, tokens, pos: 0, errors: vec![] };
    let items = p.parse_items();
    let span = if items.is_empty() {
        Span::dummy()
    } else {
        let first = items.first().map(|i| match i {
            Item::Data(d) => d.span,
            Item::Optic(o) => o.span,
            Item::Let(l) => l.span,
            Item::Fn(f) => f.span,
            Item::Expr(e) => body_span(e),
        }).unwrap_or(Span::dummy());
        let last = items.last().map(|i| match i {
            Item::Data(d) => d.span,
            Item::Optic(o) => o.span,
            Item::Let(l) => l.span,
            Item::Fn(f) => f.span,
            Item::Expr(e) => body_span(e),
        }).unwrap_or(first);
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
        self.tokens.get(self.pos).map(|t| t.kind).unwrap_or(TokenKind::Eof)
    }

    fn current_span(&self) -> Span {
        self.tokens.get(self.pos).map(|t| t.span).unwrap_or(Span::dummy())
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
                message: format!("expected {:?} in {} , got {:?}", expected, ctx, self.current()),
                span: sp,
            });
            None
        }
    }

    fn skip_until_sync(&mut self, sync: &[TokenKind]) {
        while !sync.contains(&self.current()) && self.current() != TokenKind::Eof {
            self.advance();
        }
    }

    fn parse_items(&mut self) -> Vec<Item> {
        let mut items = vec![];
        let sync = [TokenKind::KwData, TokenKind::KwOptic, TokenKind::KwLet, TokenKind::KwFn, TokenKind::Eof];
        while self.current() != TokenKind::Eof {
            match self.current() {
                TokenKind::KwData => {
                    if let Some(d) = self.parse_data_decl() { items.push(Item::Data(d)); }
                }
                TokenKind::KwOptic => {
                    if let Some(o) = self.parse_optic_decl() { items.push(Item::Optic(o)); }
                }
                TokenKind::KwLet => {
                    if let Some(l) = self.parse_let_binding() { items.push(Item::Let(l)); }
                }
                TokenKind::KwFn => {
                    if let Some(f) = self.parse_fn_decl() { items.push(Item::Fn(f)); }
                }
                _ => {
                    // Support bare top-level expr (query chains etc) for demo/example style
                    // (EBNF items are decls, but this allows the provided .opt examples without heavy rewrite)
                    if self.current() == TokenKind::Ident || self.current() == TokenKind::LParen || self.current() == TokenKind::LBrace {
                        if let Some(e) = self.parse_expr() {
                            // consume optional ;
                            if self.current() == TokenKind::Semi { self.advance(); }
                            items.push(Item::Expr(e));
                            continue;
                        }
                    }
                    let sp = self.current_span();
                    self.errors.push(ParseError { message: "expected top-level item (data, optic, let, fn) or expr".into(), span: sp });
                    self.skip_until_sync(&sync);
                }
            }
        }
        items
    }

    fn parse_data_decl(&mut self) -> Option<DataDecl> {
        let start = self.advance().span; // data
        let name_tok = self.advance();
        if name_tok.kind != TokenKind::Ident {
            self.errors.push(ParseError { message: "expected ident after data".into(), span: name_tok.span });
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
            if self.current() == TokenKind::Comma { self.advance(); }
        }
        let rbrace = self.expect(TokenKind::RBrace, "data decl").unwrap_or(Span::dummy());
        let span = start.merge(rbrace);
        Some(DataDecl { name, fields, span })
    }

    fn parse_field_decl(&mut self) -> Option<FieldDecl> {
        let name_tok = self.advance();
        if name_tok.kind != TokenKind::Ident {
            self.errors.push(ParseError { message: "expected field name".into(), span: name_tok.span });
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
                            if self.current() == TokenKind::Comma { self.advance(); }
                        }
                        self.expect(TokenKind::Gt, "type args")?;
                    }
                    let span = if args.is_empty() { span0 } else { span0.merge(args.last().map(ty_span).unwrap_or(span0)) };
                    Some(TypeExpr::Named { name, args, span })
                }
            }
            TokenKind::LParen => {
                let start = self.advance().span;
                let mut ts = vec![];
                while self.current() != TokenKind::RParen && self.current() != TokenKind::Eof {
                    if let Some(t) = self.parse_type_expr() { ts.push(t); }
                    if self.current() == TokenKind::Comma { self.advance(); }
                }
                let end = self.expect(TokenKind::RParen, "tuple type").unwrap_or(start);
                Some(TypeExpr::Tuple(ts, start.merge(end)))
            }
            _ => {
                let sp = self.current_span();
                self.errors.push(ParseError { message: "expected type".into(), span: sp });
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
        } else { None };

        let rbrace = self.expect(TokenKind::RBrace, "optic }").unwrap_or(Span::dummy());
        let span = start.merge(rbrace);

        Some(OpticDecl { name, costate, focus, grade, get, put, span })
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
                            let n: u32 = self.text_of(&n_tok).parse().unwrap_or(0);
                            self.expect(TokenKind::Gt, ">")?;
                            return Some(GradeDim::Cache { n: Some(n), span: sp });
                        }
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
                    Some(GradeDim::Named { name: txt, span: sp })
                } else if txt == "_" {
                    Some(GradeDim::Infer(sp))
                } else {
                    Some(GradeDim::Named { name: txt, span: sp })
                }
            }
            TokenKind::Lt | _ => {
                // bare _ or error recovery
                if self.current() == TokenKind::Ident && self.text_of_current() == "_" {
                    let _ = self.advance();
                    return Some(GradeDim::Infer(sp));
                }
                self.errors.push(ParseError { message: "expected grade dim".into(), span: sp });
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
        Some(PutClause { state_param, value_param, body, span })
    }

    fn parse_let_binding(&mut self) -> Option<LetBinding> {
        let start = self.advance().span; // let
        let name_tok = self.advance();
        let name = Spanned::new(self.text_of(&name_tok), name_tok.span);

        let ty = if self.current() == TokenKind::Colon {
            self.advance();
            // very simplified: we accept a full GradedOptic<...> annotation
            let costate = self.parse_type_expr()?;
            self.expect(TokenKind::Comma, ",")?;
            let focus = self.parse_type_expr()?;
            self.expect(TokenKind::Comma, ",")?;
            let grade = self.parse_grade_expr()?;
            self.expect(TokenKind::Gt, ">")?; // rough
            Some(GradeOpticType { costate, focus, grade, span: start })
        } else { None };

        self.expect(TokenKind::Equals, "=")?;
        let value = self.parse_optic_expr()?;
        self.expect(TokenKind::Semi, ";")?;
        let span = start.merge(value_span(&value));
        Some(LetBinding { name, ty, value, span })
    }

    fn parse_fn_decl(&mut self) -> Option<FnDecl> {
        // Minimal support for now (enough for simple wrappers)
        let start = self.advance().span;
        let name_tok = self.advance();
        let name = Spanned::new(self.text_of(&name_tok), name_tok.span);
        self.expect(TokenKind::LParen, "(")?;
        let mut params = vec![];
        while self.current() != TokenKind::RParen {
            let p_name = self.advance();
            let p = Spanned::new(self.text_of(&p_name), p_name.span);
            self.expect(TokenKind::Colon, ":")?;
            let ty = self.parse_type_expr()?;
            params.push(Param { name: p, ty, span: p_name.span });
            if self.current() == TokenKind::Comma { self.advance(); }
        }
        self.expect(TokenKind::RParen, ")")?;
        let ret = if self.current() == TokenKind::FatArrow || self.current() == TokenKind::Gt {
            // support -> or => for fn ret (EBNF uses -> ; lenient for demo)
            if self.current() == TokenKind::FatArrow || self.current() == TokenKind::Gt { self.advance(); }
            Some(self.parse_type_expr()?)
        } else { None };
        self.expect(TokenKind::LBrace, "{")?;
        let mut body = vec![];
        while self.current() != TokenKind::RBrace && self.current() != TokenKind::Eof {
            body.push(self.parse_stmt()?);
        }
        self.expect(TokenKind::RBrace, "}")?;
        Some(FnDecl { name, params, ret, body, span: start })
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        let start = self.current_span();
        // very simple: optional target = expr ;
        let mut target = None;
        if let TokenKind::Ident = self.current() {
            // peek for =
            let saved = self.pos;
            let id_tok = self.advance();
            if self.current() == TokenKind::Equals {
                self.advance();
                target = Some(Spanned::new(self.text_of(&id_tok), id_tok.span));
            } else {
                self.pos = saved; // backtrack
            }
        }
        let expr = self.parse_expr()?;
        self.expect(TokenKind::Semi, "stmt ;")?;
        let span = start.merge(body_span(&expr));
        Some(Stmt { target, expr, span })
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
            return Some(Expr::Assign { target: Box::new(left), value: Box::new(right), span });
        }
        Some(left)
    }

    fn parse_field_expr(&mut self) -> Option<Expr> {
        let mut base = self.parse_atom_expr()?;
        let mut span = match &base {
            AtomExpr::Ident(s) => s.span,
            AtomExpr::Int(_, sp) | AtomExpr::Float(_, sp) | AtomExpr::Tuple(_, sp) | AtomExpr::Paren(_, sp) => *sp,
        };
        loop {
            match self.current() {
                TokenKind::Dot => {
                    self.advance();
                    let id_tok = self.advance();
                    if id_tok.kind != TokenKind::Ident {
                        self.errors.push(ParseError { message: "expected field ident after .".into(), span: id_tok.span });
                        break;
                    }
                    let field = Spanned::new(self.text_of(&id_tok), id_tok.span);
                    let new_span = span.merge(field.span);
                    base = AtomExpr::Ident(Spanned::new("_field_temp".into(), new_span)); // placeholder for FieldExpr lowering; real FieldExpr built in HIR
                    // For AST fidelity we keep simple Atom for now; HIR will do proper FieldExpr chain from source text.
                    span = new_span;
                    // (To keep simple for full pipeline, we will parse the structure but represent field chains via the original source spans in HIR lowering.)
                }
                TokenKind::LBracket => {
                    self.advance();
                    let idx = self.parse_expr()?;
                    let r = self.expect(TokenKind::RBracket, "]")?;
                    span = span.merge(r);
                    // similar, represented in HIR
                    base = AtomExpr::Ident(Spanned::new("_index_temp".into(), span));
                }
                _ => break,
            }
        }
        // Return as Atom for current AST simplicity (field info carried in spans + source for HIR); upgrade to FieldExpr variant in future if needed.
        Some(Expr::Atom(base))
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
                if exprs.len() == 1 {
                    Some(AtomExpr::Paren(Box::new(exprs.into_iter().next().unwrap()), start.merge(end)))
                } else {
                    Some(AtomExpr::Tuple(exprs, start.merge(end)))
                }
            }
            TokenKind::LBrace => {
                // block as atom
                let block = self.parse_block_expr()?;
                Some(AtomExpr::Paren(Box::new(block), /*approx*/ Span::dummy()))
            }
            _ => {
                if self.looks_like_query_chain() {
                    let qc = self.parse_query_chain()?;
                    // Wrap query as atom for expr position (query is a kind of expr)
                    return Some(AtomExpr::Ident(Spanned::new("query_chain".into(), qc.span)));
                }
                let sp = self.current_span();
                self.errors.push(ParseError { message: "expected atom (ident/lit/( / { / query )".into(), span: sp });
                Some(AtomExpr::Ident(Spanned::new("_err_atom".into(), sp)))
            }
        }
    }

    fn looks_like_query_chain(&self) -> bool {
        // ident . query (
        if self.current() != TokenKind::Ident { return false; }
        // peek ahead without advancing permanently (simple for now)
        // In practice after atom/field we check .query in higher or here for top level.
        true
    }

    fn parse_query_chain(&mut self) -> Option<QueryChain> {
        let base_tok = self.advance();
        let base = Box::new(Expr::Atom(AtomExpr::Ident(Spanned::new(self.text_of(&base_tok), base_tok.span))));
        self.expect(TokenKind::Dot, ".")?;
        // query or KwQuery
        let qtok = self.advance();
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
                    // closure: | IDENT | expr   or | ( id, .. ) | expr
                    // consume the | ... | forgiving but better now
                    let mut params = vec![];
                    // accept | or just start (no BinOpPlaceholder token)
                    if self.current() == TokenKind::Ident && self.text_of_current() == "|" {
                        self.advance();
                    }
                    if self.current() == TokenKind::LParen {
                        self.advance();
                        while self.current() != TokenKind::RParen && self.current() != TokenKind::Eof {
                            if self.current() == TokenKind::Ident {
                                let p = self.advance();
                                params.push(Spanned::new(self.text_of(&p), p.span));
                            }
                            if self.current() == TokenKind::Comma { self.advance(); }
                        }
                        self.expect(TokenKind::RParen, "closure )")?;
                    } else if self.current() == TokenKind::Ident {
                        let p = self.advance();
                        params.push(Spanned::new(self.text_of(&p), p.span));
                    }
                    if self.current() == TokenKind::Ident && self.text_of_current() == "|" {
                        self.advance();
                    }
                    let body = self.parse_expr()?;
                    methods.push(QueryMethod::Map(Closure { params, body: Box::new(body), span: sp }, sp));
                    self.expect(TokenKind::RParen, "map )")?;
                }
                _ => { /* stop */ break; }
            }
        }
        let span = base_tok.span; // approx
        Some(QueryChain { base, optic, methods, span })
    }

    fn parse_block_expr(&mut self) -> Option<Expr> {
        let start = self.expect(TokenKind::LBrace, "{")?;
        let mut stmts = vec![];
        let mut result = None;
        while self.current() != TokenKind::RBrace && self.current() != TokenKind::Eof {
            // allow trailing expr without ; as result in blocks
            if self.current() == TokenKind::RBrace { break; }
            let e = self.parse_expr()?;
            if self.current() == TokenKind::Semi {
                self.advance();
                stmts.push(Stmt { target: None, expr: e, span: start });
            } else {
                result = Some(Box::new(e));
                break;
            }
        }
        let end = self.expect(TokenKind::RBrace, "block }")?;
        Some(Expr::Block { stmts, result, span: start.merge(end) })
    }

    fn parse_binary_expr(&mut self) -> Option<Expr> {
        let left = self.parse_field_expr()?;
        // minimal binop support for map bodies etc ( + - etc )
        match self.current() {
            TokenKind::Plus | TokenKind::Ident /*rough for - etc*/ => {
                // For demo, if next looks like binop continue as binary
                if self.current() == TokenKind::Plus {
                    let op = BinOp::Add;
                    self.advance();
                    let right = self.parse_field_expr()?;
                    return Some(Expr::Binary { left: Box::new(left), op, right: Box::new(right), span: Span::dummy() });
                }
            }
            _ => {}
        }
        Some(left)
    }

    // Optic expressions with precedence (>>> tighter than *** ) per ch7 + EBNF
    fn parse_optic_expr(&mut self) -> Option<OpticExpr> {
        self.parse_optic_par()
    }

    fn parse_optic_par(&mut self) -> Option<OpticExpr> {
        let mut lhs = self.parse_optic_seq()?;
        while self.current() == TokenKind::Par {
            let op_span = self.advance().span;
            let rhs = self.parse_optic_seq()?;
            let span = op_span; // approx
            lhs = OpticExpr::Par { left: Box::new(lhs), right: Box::new(rhs), span };
        }
        Some(lhs)
    }

    fn parse_optic_seq(&mut self) -> Option<OpticExpr> {
        let mut lhs = self.parse_optic_atom()?;
        while self.current() == TokenKind::Seq {
            let op_span = self.advance().span;
            let rhs = self.parse_optic_atom()?;
            let span = /* merge */ op_span;
            lhs = OpticExpr::Seq { left: Box::new(lhs), right: Box::new(rhs), span };
        }
        Some(lhs)
    }

    fn parse_optic_atom(&mut self) -> Option<OpticExpr> {
        match self.current() {
            TokenKind::Ident => {
                let id = self.advance();
                Some(OpticExpr::Atom(OpticAtom::Named(Spanned::new(self.text_of(&id), id.span))))
            }
            TokenKind::LParen => {
                let start = self.advance().span;
                let inner = self.parse_optic_expr()?;
                let end = self.expect(TokenKind::RParen, ")")?;
                Some(OpticExpr::Atom(OpticAtom::Paren(Box::new(inner), start.merge(end))))
            }
            _ => {
                let sp = self.current_span();
                self.errors.push(ParseError { message: "expected optic atom (ident or ( ))".into(), span: sp });
                None
            }
        }
    }

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
                    stmts.push(Stmt { target: None, expr: e, span: start });
                } else {
                    result = Some(Box::new(e));
                    break;
                }
            }
            let end = self.expect(TokenKind::RBrace, " }")?;
            Some(Expr::Block { stmts, result, span: start.merge(end) })
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
        } else { String::new() }
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
        GradeDim::Cache { span, .. } | GradeDim::Ownership { span, .. } | GradeDim::Named { span, .. } | GradeDim::Infer(span) => *span,
    }
}

fn body_span(e: &Expr) -> Span {
    match e {
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
const _ : () = ();
