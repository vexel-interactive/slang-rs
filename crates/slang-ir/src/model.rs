use serde::Serialize;
use slang_sema::{EntryPoint, Type};
use slang_source::Span;

#[derive(Clone, Debug, Serialize)]
pub struct Module {
    pub structures: Vec<Struct>,
    pub globals: Vec<Variable>,
    pub functions: Vec<Function>,
    pub entry_points: Vec<EntryPoint>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<Variable>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Variable {
    pub name: String,
    pub ty: Type,
    pub modifiers: Vec<String>,
    pub semantic: Option<String>,
    pub initializer: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct Function {
    pub name: String,
    pub return_type: Type,
    pub parameters: Vec<Variable>,
    pub modifiers: Vec<String>,
    pub semantic: Option<String>,
    pub body: Option<Stmt>,
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
    DoWhile {
        body: Box<Stmt>,
        condition: Expr,
    },
    Switch {
        selector: Expr,
        cases: Vec<SwitchCase>,
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

#[derive(Clone, Debug, Serialize)]
pub struct Expr {
    pub ty: Type,
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct SwitchCase {
    pub value: Option<Expr>,
    pub statements: Vec<Stmt>,
}

#[derive(Clone, Debug, Serialize)]
pub enum ExprKind {
    Integer(String),
    Float(String),
    Bool(bool),
    Name(String),
    Unary {
        op: String,
        value: Box<Expr>,
    },
    Cast {
        value: Box<Expr>,
    },
    Binary {
        op: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        name: String,
        arguments: Vec<Expr>,
    },
    MethodCall {
        receiver: Box<Expr>,
        name: String,
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
