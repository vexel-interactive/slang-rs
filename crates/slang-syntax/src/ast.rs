use serde::Serialize;
use slang_source::Span;

#[derive(Clone, Debug, Default, Serialize)]
pub struct Module {
    pub module_declared: bool,
    pub name: Option<String>,
    pub implementation: Option<String>,
    pub imports: Vec<Import>,
    pub file_includes: Vec<FileInclude>,
    pub declarations: Vec<Declaration>,
    /// Direct cbuffer declarations after their fields and hidden handles are normalized.
    pub constant_buffers: Vec<ConstantBufferInfo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConstantBufferInfo {
    pub name: String,
    pub variable_name: String,
    pub element_type: String,
    pub members: Vec<ConstantBufferMember>,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConstantBufferMember {
    pub name: String,
    pub variable_name: String,
    pub static_storage: bool,
    pub span: Span,
}

impl Module {
    pub fn constant_buffer_member(&self, span: Span) -> Option<&ConstantBufferMember> {
        self.constant_buffers
            .iter()
            .flat_map(|b| &b.members)
            .find(|m| m.span == span)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Import {
    /// Normalized file request, including the .slang extension.
    pub path: String,
    pub exported: bool,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct FileInclude {
    pub path: String,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub enum Declaration {
    TypeAlias(TypeAlias),
    Struct(Struct),
    Function(Box<Function>),
    Global(Variable),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TypeRef {
    pub name: String,
    pub arguments: Vec<TypeArgument>,
    /// Array dimensions from outermost to innermost.
    pub arrays: Vec<u32>,
    pub span: Span,
}

/// A type application can contain both types and integer shape arguments.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum TypeArgument {
    Type(TypeRef),
    Integer { value: u32, span: Span },
}

#[derive(Clone, Debug, Serialize)]
pub struct Attribute {
    pub name: String,
    pub arguments: Vec<Expr>,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct TypeAlias {
    pub name: String,
    pub target: TypeRef,
    pub modifiers: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct Struct {
    pub name: String,
    pub modifiers: Vec<String>,
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
    pub register: Option<RegisterAnnotation>,
    pub initializer: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct RegisterAnnotation {
    pub binding: slang_layout::RegisterBinding,
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
    Variable(Box<Variable>),
    /// A declaration group introduces bindings sequentially in the current scope.
    Variables(Vec<Variable>),
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
    Discard,
    Empty,
}

#[derive(Clone, Debug, Serialize)]
pub struct SwitchCase {
    /// `None` denotes the default label; consecutive labels have empty bodies.
    pub value: Option<Expr>,
    pub statements: Vec<Stmt>,
    pub span: Span,
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
    InitializerList(Vec<Expr>),
    Integer(String),
    Float(String),
    Bool(bool),
    String(String),
    Name(String),
    /// A numeric type application used as a constructor callee.
    TypeApplication(Box<TypeRef>),
    Unary {
        op: String,
        value: Box<Expr>,
    },
    Cast {
        target: Box<TypeRef>,
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
