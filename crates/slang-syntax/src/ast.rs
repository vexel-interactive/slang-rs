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
    pub static_members: Vec<StaticMemberInfo>,
    /// Source member names and owners of flattened struct and alias declarations.
    pub nested_types: Vec<NestedTypeInfo>,
    /// Concrete type arguments visible in each instantiated declaration context.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub generic_instances: Vec<GenericInstanceInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub function_applications: Vec<FunctionApplicationInfo>,
    /// Constructor templates retain synthesized memberwise candidates where eligible.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub generic_constructors: Vec<GenericConstructorInfo>,
    /// Inferred overload groups keyed by concrete owner and semantic argument types.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub constructor_applications: Vec<ConstructorApplicationInfo>,
    /// Generic calls retain the interface requirement selected on opaque parameters.
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub constraint_calls: std::collections::BTreeMap<ExprId, ConstraintCallInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub constraint_variables: Vec<ConstraintVariableInfo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConstraintVariableInfo {
    pub span: Span,
    pub call: ConstraintCallInfo,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConstraintCallInfo {
    pub owner: TypeRef,
    pub interface: TypeRef,
    pub requirement: Function,
    pub declaration_module: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct GenericConstructorInfo {
    pub owner: String,
    pub function: String,
    pub synthesize_members: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConstructorApplicationInfo {
    pub owner: String,
    pub argument_types: Vec<String>,
    pub function: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct FunctionApplicationInfo {
    pub name: String,
    pub candidates: Vec<FunctionCandidateInfo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FunctionCandidateInfo {
    pub function: String,
    pub default_type_arguments: usize,
    pub generic: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct GenericInstanceInfo {
    pub context: usize,
    pub type_name: String,
    pub template_name: String,
    pub parameter_count: usize,
    pub arguments: Vec<TypeRef>,
}

#[derive(Clone, Debug, Serialize)]
pub struct NestedTypeInfo {
    pub owner: String,
    pub name: String,
    pub type_name: String,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct StaticMemberInfo {
    pub owner: String,
    pub name: String,
    pub variable_name: String,
    pub span: Span,
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
    pub pack_offset: Option<PackOffsetAnnotation>,
    pub span: Span,
}

impl Module {
    pub fn static_member(&self, span: Span) -> Option<&StaticMemberInfo> {
        self.static_members.iter().find(|m| m.span == span)
    }
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
    Interface(Interface),
    Function(Box<Function>),
    Global(Variable),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TypeRef {
    pub name: String,
    /// A qualified owner carrying its own generic arguments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<Box<TypeRef>>,
    #[serde(skip_serializing_if = "is_false")]
    pub application: bool,
    /// An implicit receiver or constructor result synthesized by the parser.
    #[serde(skip_serializing_if = "is_false")]
    pub owner_ref: bool,
    /// A constructor or cast originating from a generic type parameter.
    #[serde(skip_serializing_if = "is_false")]
    pub parameter_ref: bool,
    pub arguments: Vec<TypeArgument>,
    /// Storage qualification; numeric value types retain their algebraic shape.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matrix_major: Option<slang_layout::MatrixMajor>,
    /// Array dimensions from outermost to innermost.
    pub arrays: Vec<Expr>,
    pub span: Span,
}

fn is_false(value: &bool) -> bool {
    !value
}

#[derive(Clone, Debug, Serialize)]
pub struct GenericParameter {
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<TypeRef>,
    pub default: Option<TypeRef>,
    pub span: Span,
}

/// A type application can contain both types and integer shape arguments.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum TypeArgument {
    Type(TypeRef),
    Integer { value: u32, span: Span },
    Expression(Box<Expr>),
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub generic_parameters: Vec<GenericParameter>,
    pub target: TypeRef,
    pub modifiers: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct Struct {
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub generic_parameters: Vec<GenericParameter>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bases: Vec<TypeRef>,
    pub modifiers: Vec<String>,
    pub fields: Vec<Variable>,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct Interface {
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub generic_parameters: Vec<GenericParameter>,
    pub bases: Vec<TypeRef>,
    pub requirements: Vec<Function>,
    pub modifiers: Vec<String>,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize)]
pub struct Function {
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub generic_parameters: Vec<GenericParameter>,
    /// Owning type for a normalized user-defined __init declaration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constructor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<StructMethod>,
    pub return_type: TypeRef,
    pub parameters: Vec<Variable>,
    pub attributes: Vec<Attribute>,
    pub modifiers: Vec<String>,
    pub semantic: Option<String>,
    pub body: Option<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StructMethod {
    pub owner: String,
    pub name: String,
    pub is_static: bool,
}

impl Function {
    pub fn owner_type(&self) -> Option<&str> {
        self.constructor
            .as_deref()
            .or_else(|| self.method.as_ref().map(|m| m.owner.as_str()))
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Variable {
    pub name: String,
    pub ty: TypeRef,
    pub modifiers: Vec<String>,
    pub semantic: Option<String>,
    pub register: Option<RegisterAnnotation>,
    pub pack_offset: Option<PackOffsetAnnotation>,
    pub initializer: Option<Expr>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct RegisterAnnotation {
    pub binding: slang_layout::RegisterBinding,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct PackOffsetAnnotation {
    pub offset: u64,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Expr {
    pub id: ExprId,
    pub kind: ExprKind,
    pub span: Span,
    /// Parentheses prevent parser-time literal folding by an enclosing prefix
    /// operator, even though they are transparent to ordinary type checking.
    pub parenthesized: bool,
    #[serde(skip)]
    pub(crate) height: usize,
}

impl Expr {
    /// Interpret a named expression application as a qualified type reference.
    pub fn type_reference(&self) -> Option<TypeRef> {
        let (name, base) = match &self.kind {
            ExprKind::Name(name) => (name.clone(), None),
            ExprKind::TypeApplication(ty) => return Some(*ty.clone()),
            ExprKind::Member { base, name, .. } => {
                (name.clone(), Some(Box::new(base.type_reference()?)))
            }
            ExprKind::GenericApplication { callee, arguments } => {
                let mut ty = callee.type_reference()?;
                ty.arguments = arguments.clone();
                ty.application = true;
                ty.span = self.span;
                return Some(ty);
            }
            _ => return None,
        };
        Some(TypeRef {
            name,
            base,
            application: false,
            owner_ref: false,
            parameter_ref: false,
            arguments: vec![],
            matrix_major: None,
            arrays: vec![],
            span: self.span,
        })
    }

    /// Construct a nonnegative integer expression for a synthesized array dimension.
    pub fn integer(id: ExprId, value: u32, span: Span) -> Self {
        Self {
            id,
            kind: ExprKind::Integer(value.to_string()),
            span,
            parenthesized: false,
            height: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ExprKind {
    InitializerList(Vec<Expr>),
    Integer(String),
    Float(String),
    Bool(bool),
    String(String),
    Name(String),
    /// A type application used as a constructor or qualified lookup base.
    TypeApplication(Box<TypeRef>),
    /// Explicit arguments applied to a member callee, retaining its receiver.
    GenericApplication {
        callee: Box<Expr>,
        arguments: Vec<TypeArgument>,
    },
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
        /// `::` looks up only static members, even when the base is a value.
        static_lookup: bool,
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
