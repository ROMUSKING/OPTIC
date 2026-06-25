//! AST for v0 surface (close to EBNF in appendix D + ch. 7).
//! Spans are on almost everything for provenance.

use crate::span::{Span, Spanned};

pub type Ident = Spanned<String>;

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpticTypeCtor {
    GradedOptic,
    GradedPrism,
    GradedTraversal,
}

impl OpticDecl {
    /// True when the surface form is deferred to M7+ (unsafe boundary, host/foreign).
    pub fn is_unsupported_v0(&self) -> bool {
        self.unsafe_boundary
    }

    /// True when the optic is a graded prism (preview/review surface).
    pub fn is_prism(&self) -> bool {
        self.type_ctor == OpticTypeCtor::GradedPrism
    }

    /// True when the optic is a graded traversal (get/put surface; bulk/SIMD metadata).
    pub fn is_traversal(&self) -> bool {
        self.type_ctor == OpticTypeCtor::GradedTraversal
    }
}

#[derive(Debug, Clone)]
pub enum Item {
    Data(DataDecl),
    Optic(Box<OpticDecl>),
    Extern(ExternDecl),
    Let(LetBinding),
    Fn(FnDecl),
    /// Top-level expr stmt (for demo/scripts; EBNF items are decls but examples use bare queries)
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct DataDecl {
    pub name: Ident,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: Ident,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeExpr {
    Soa(Box<TypeExpr>, Span),
    BitSet(Span),
    Tuple(Vec<TypeExpr>, Span),
    Named {
        name: String,
        args: Vec<TypeExpr>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub struct OpticDecl {
    pub name: Ident,
    pub type_ctor: OpticTypeCtor,
    pub unsafe_boundary: bool,
    pub costate: TypeExpr,
    pub focus: TypeExpr,
    pub grade: GradeExpr,
    pub get: Option<GetClause>,
    pub put: Option<PutClause>,
    pub preview: Option<GetClause>,
    pub review: Option<PutClause>,
    /// M7 Phase 1 skeleton fields (reusing GetClause/PutClause per plan). Full enforcement Phase 2.
    pub traverse: Option<GetClause>,
    pub update: Option<PutClause>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExternDecl {
    pub abi: String,
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct GetClause {
    pub param: Ident,
    pub body: Expr,
    /// When true (via `partial preview`), preview is treated as `Option<focus>` in codegen.
    pub partial: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct PutClause {
    pub state_param: Ident,
    pub value_param: Ident,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct GradeExpr {
    pub dims: Vec<GradeDim>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum GradeDim {
    Cache { n: Option<u32>, span: Span },        // None = _
    Ownership { r: Option<String>, span: Span }, // rational or _
    Named { name: String, span: Span },          // LinearGrade etc or _
    BranchBias { bias: String, span: Span },     // Likely | Unlikely | Unknown (M7)
    Infer(Span),
}

#[derive(Debug, Clone)]
pub struct LetBinding {
    pub name: Ident,
    pub ty: Option<GradeOpticType>, // optional annotation
    pub value: OpticExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct GradeOpticType {
    pub costate: TypeExpr,
    pub focus: TypeExpr,
    pub grade: GradeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: Ident,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub target: Option<Ident>, // for assignments in blocks
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Expr {
    QueryChain(QueryChain),
    Field(FieldExpr),
    Atom(AtomExpr),
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
        span: Span,
    },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },
    Block {
        stmts: Vec<Stmt>,
        result: Option<Box<Expr>>,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Debug, Clone)]
pub struct QueryChain {
    pub base: Box<Expr>,
    pub optic: OpticExpr,
    pub methods: Vec<QueryMethod>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum QueryMethod {
    Get(Span),
    Set(Expr, Span),
    Map(Closure, Span),
    /// M8 observability tap (book ch14.5); lowered to CGIR `Tap`.
    Tap(String, Span),
    /// M8 observability record (book ch14.5); lowered to CGIR `Record`.
    Record(String, Span),
    /// Deferred observability surface (profile/replay); rejected via OBS-701 in v0.
    Profile(String, Span),
    Replay(String, Span),
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub params: Vec<Ident>,
    pub body: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum OpticExpr {
    Atom(OpticAtom),
    Seq {
        left: Box<OpticExpr>,
        right: Box<OpticExpr>,
        span: Span,
    },
    Par {
        left: Box<OpticExpr>,
        right: Box<OpticExpr>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum OpticAtom {
    Named(Ident),
    Paren(Box<OpticExpr>, Span),
}

#[derive(Debug, Clone)]
pub enum FieldExpr {
    Base(AtomExpr, Span),
    FieldAccess {
        base: Box<FieldExpr>,
        field: Ident,
        span: Span,
    },
    Index {
        base: Box<FieldExpr>,
        index: Box<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum AtomExpr {
    Ident(Ident),
    Int(i64, Span),
    Float(f64, Span),
    Tuple(Vec<Expr>, Span),
    Paren(Box<Expr>, Span),
    // block handled at Expr level
}

#[derive(Debug, Clone)]
pub struct FieldAccess {/* used in lowering */}
