use serde::Serialize;
use slang_sema::{EntryPoint, Type};
use slang_source::Span;

#[derive(Clone, Debug, Serialize)]
pub struct Module {
    pub matrix_major: slang_layout::MatrixMajor,
    pub native_16bit_types: bool,
    pub structures: Vec<Struct>,
    pub constant_buffers: Vec<ConstantBuffer>,
    pub globals: Vec<Variable>,
    /// Shared constant-buffer register for global uniform values.
    pub global_uniform_binding: Option<slang_layout::ResourceBinding>,
    /// Occupied bytes in the shared global uniform buffer.
    pub global_uniform_size: Option<u64>,
    /// Flattened shared-buffer declarations when packed aggregates require value views.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_uniform_fields: Option<Vec<Variable>>,
    pub uniform_value_functions: std::collections::BTreeMap<String, String>,
    /// Constant-buffer value layouts, keyed by canonical type names.
    pub uniform_types: std::collections::BTreeMap<String, slang_layout::UniformTypeLayout>,
    pub functions: Vec<Function>,
    pub entry_points: Vec<EntryPoint>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<Variable>,
}

/// A direct cbuffer with anonymous fields visible in the global value scope.
#[derive(Clone, Debug, Serialize)]
pub struct ConstantBuffer {
    pub name: String,
    pub variable_name: String,
    pub fields: Vec<Variable>,
    pub views: Vec<Variable>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Variable {
    pub name: String,
    pub ty: Type,
    pub modifiers: Vec<String>,
    pub semantic: Option<String>,
    pub binding: Option<slang_layout::ResourceBinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniform_layout: Option<slang_layout::UniformVariableLayout>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pack_offset: Option<u64>,
    pub initializer: Option<Expr>,
    pub span: Span,
}

impl Variable {
    /// Classify a global declaration, excluding resource handles and private storage.
    pub fn is_global_uniform(&self) -> bool {
        !self.ty.is_resource()
            && !self
                .modifiers
                .iter()
                .any(|m| matches!(m.as_str(), "static" | "groupshared"))
    }
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
    /// Consecutive statements that do not introduce a lexical scope.
    Sequence(Vec<Stmt>),
    Variable(Variable),
    /// Consecutive declarations without an additional lexical scope.
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
    /// Temporary same-width integer view of writable storage. Effect lowering
    /// preserves its address while applying the viewed type to loads and stores.
    AddressView(Box<Expr>),
    /// Temporary statement-valued expression, eliminated by effect normalization
    /// before scalar SSA and target lowering.
    BlockExpression {
        statements: Vec<Stmt>,
        value: Box<Expr>,
    },
    /// Read an element value from a checked ConstantBuffer resource expression.
    ConstantBufferLoad(Box<Expr>),
    /// Evaluate a checked default in declaration scope before passing its value.
    DeclarationDefault(Box<Expr>),
    Zero,
    Construct(Vec<Expr>),
    Integer(String),
    Float(String),
    Bool(bool),
    Name(String),
    /// A source global reference, distinguished from lexical local bindings.
    GlobalName(String),
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
    /// A selected builtin; user declarations cannot capture its target spelling.
    IntrinsicCall {
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
