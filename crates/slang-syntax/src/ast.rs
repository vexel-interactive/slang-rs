use serde::Serialize;
use slang_source::Span;

#[derive(Clone, Debug, Default, Serialize)]
pub struct Module {
    pub declarations: Vec<Declaration>,
}

#[derive(Clone, Debug, Serialize)]
pub enum Declaration {
    Struct(Struct),
    Function(Box<Function>),
    Global(Variable),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TypeRef {
    pub name: String,
    pub arguments: Vec<TypeRef>,
    pub array: Option<u32>,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct Attribute {
    pub name: String,
    pub arguments: Vec<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<Variable>,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct Function {
    pub name: String,
    pub return_type: TypeRef,
    pub parameters: Vec<Variable>,
    pub attributes: Vec<Attribute>,
    pub modifiers: Vec<String>,
    pub semantic: Option<String>,
    pub body: Option<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct Variable {
    pub name: String,
    pub ty: TypeRef,
    pub modifiers: Vec<String>,
    pub semantic: Option<String>,
    pub initializer: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub enum StmtKind {
    Block(Vec<Stmt>),
    Variable(Variable),
    Expression(Expr),
    Return(Option<Expr>),
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    While {
        condition: Expr,
        body: Box<Stmt>,
    },
    For {
        initializer: Box<Stmt>,
        condition: Option<Expr>,
        increment: Option<Expr>,
        body: Box<Stmt>,
    },
    Break,
    Continue,
    Empty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ExprId(pub usize);

#[derive(Clone, Debug, Serialize)]
pub struct Expr {
    pub id: ExprId,
    pub kind: ExprKind,
    pub span: Span,
    #[serde(skip)]
    pub(crate) height: usize,
}

#[derive(Clone, Debug, Serialize)]
pub enum ExprKind {
    Integer(String),
    Float(String),
    Bool(bool),
    String(String),
    Name(String),
    Unary {
        op: String,
        value: Box<Expr>,
    },
    Binary {
        op: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
    },
    Member {
        base: Box<Expr>,
        name: String,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Conditional {
        condition: Box<Expr>,
        then_value: Box<Expr>,
        else_value: Box<Expr>,
    },
    Postfix {
        op: String,
        value: Box<Expr>,
    },
}
