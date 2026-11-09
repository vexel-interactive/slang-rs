use crate::{
    visit::{Visitor, walk_expression},
    *,
};
use slang_sema::{CheckedModule, Type, span_key};
use slang_source::{Diagnostic, Span};
use std::collections::{BTreeMap, BTreeSet};

mod arguments;
mod control;

const MAX_INLINE_NODES: usize = 65_536;
const MAX_INLINE_DEPTH: usize = 32;

/// Inline explicitly requested value functions before scalar SSA.
/// Capture storage is declared at function scope. Linear bodies use comma
/// expressions; control-flow bodies use statement expressions normalized at
/// their source evaluation points, including conditional arms.
pub(crate) fn lower(module: &mut Module, checked: &CheckedModule) -> Result<(), Diagnostic> {
    let forced: BTreeSet<_> = checked
        .module
        .declarations
        .iter()
        .filter_map(|d| {
            let slang_syntax::Declaration::Function(function) = d else {
                return None;
            };
            function
                .attributes
                .iter()
                .any(|a| a.name == "ForceInline")
                .then(|| {
                    checked
                        .function_names
                        .get(&span_key(function.span))
                        .cloned()
                        .unwrap_or_else(|| function.name.clone())
                })
        })
        .collect();
    if forced.is_empty() {
        return Ok(());
    }
    let names = crate::visit::names(module);
    let mut prefix = "_slang_inline_".to_owned();
    while names.iter().any(|name| name.starts_with(&prefix)) {
        prefix.push('_');
    }
    let mut lowerer = Lowerer {
        templates: module
            .functions
            .iter()
            .filter(|f| forced.contains(&f.name) && f.body.is_some())
            .map(|f| (f.name.clone(), f.clone()))
            .collect(),
        forced,
        prefix,
        counter: 0,
        declarations: vec![],
        active: vec![],
        remaining: MAX_INLINE_NODES,
    };
    // Global expansion lives in an initializer function, never in file-scope
    // statements. DXC evaluates that function in the shader's private state.
    for global in &mut module.globals {
        if let Some(value) = &mut global.initializer {
            let mut requests = Requests {
                forced: &lowerer.forced,
                found: false,
            };
            requests.expression(value)?;
            if !requests.found {
                continue;
            }
            lowerer.remaining = MAX_INLINE_NODES;
            lowerer.expression(value)?;
            let name = format!("{}global_init_{}", lowerer.prefix, module.functions.len());
            let mut statements = std::mem::take(&mut lowerer.declarations);
            statements.push(Stmt {
                kind: StmtKind::Return(Some(value.clone())),
                span: value.span,
            });
            module.functions.push(Function {
                name: name.clone(),
                return_type: value.ty.clone(),
                parameters: vec![],
                modifiers: vec![],
                semantic: None,
                body: Some(crate::effects::block(statements, value.span)),
                span: value.span,
            });
            value.kind = ExprKind::Call {
                name,
                arguments: vec![],
            };
        }
    }
    for function in &mut module.functions {
        if let Some(body) = &mut function.body {
            lowerer.active.push(function.name.clone());
            lowerer.remaining = MAX_INLINE_NODES;
            lowerer.statement(body)?;
            lowerer.active.pop();
            if !lowerer.declarations.is_empty() {
                let mut declarations = std::mem::take(&mut lowerer.declarations);
                if let StmtKind::Block(values) = &mut body.kind {
                    declarations.append(values);
                } else {
                    declarations.push(body.clone());
                }
                body.kind = StmtKind::Block(declarations);
            }
        }
    }
    // Initializer functions may themselves return fixed arrays.
    crate::array_result::lower(module)?;
    crate::effects::lower(module, checked)
}

struct Requests<'a> {
    forced: &'a BTreeSet<String>,
    found: bool,
}
impl Visitor for Requests<'_> {
    fn expression(&mut self, value: &mut Expr) -> Result<(), Diagnostic> {
        if matches!(&value.kind, ExprKind::Call { name, .. } if self.forced.contains(name)) {
            self.found = true;
        }
        walk_expression(self, value)
    }
}

struct Lowerer {
    templates: BTreeMap<String, Function>,
    forced: BTreeSet<String>,
    prefix: String,
    counter: usize,
    declarations: Vec<Stmt>,
    active: Vec<String>,
    remaining: usize,
}

impl Lowerer {
    fn storage(&mut self, variable: &Variable, span: Span) -> Expr {
        let name = format!("{}{}", self.prefix, self.counter);
        self.counter += 1;
        let mut storage = variable.clone();
        storage.name = name.clone();
        storage.initializer = None;
        storage.semantic = None;
        storage.binding = None;
        storage.uniform_layout = None;
        storage.pack_offset = None;
        storage
            .modifiers
            .retain(|m| matches!(m.as_str(), "row_major" | "column_major"));
        self.declarations.push(Stmt {
            kind: StmtKind::Variable(storage),
            span,
        });
        Expr {
            kind: ExprKind::Name(name),
            ty: variable.ty.clone(),
            span,
        }
    }

    fn assign(&self, left: Expr, right: Expr, span: Span) -> Expr {
        let right = if left.ty == right.ty {
            right
        } else {
            Expr {
                ty: left.ty.clone(),
                span,
                kind: ExprKind::Cast {
                    value: Box::new(right),
                },
            }
        };
        Expr {
            ty: left.ty.clone(),
            span,
            kind: ExprKind::Binary {
                op: "=".into(),
                left: Box::new(left),
                right: Box::new(right),
            },
        }
    }

    fn expand(&mut self, name: &str, arguments: &[Expr], span: Span) -> Result<Expr, Diagnostic> {
        if self.active.len() >= MAX_INLINE_DEPTH || self.active.iter().any(|active| active == name)
        {
            return Err(unsupported(
                span,
                "recursive or excessively nested ForceInline expansion",
            ));
        }
        let function =
            self.templates.get(name).cloned().ok_or_else(|| {
                unsupported(span, "ForceInline requires an available function body")
            })?;
        if !(storable(&function.return_type) || function.return_type == Type::Void)
            || function.parameters.iter().any(|p| !storable(&p.ty))
        {
            return Err(unsupported(
                span,
                "ForceInline non-value signatures are not implemented",
            ));
        }
        let body = function.body.as_ref().unwrap();
        let StmtKind::Block(statements) = &body.kind else {
            return Err(unsupported(
                span,
                "ForceInline requires a linear function body",
            ));
        };
        let linear_result = statements.last().and_then(|statement| {
            if let StmtKind::Return(Some(result)) = &statement.kind {
                Some(result)
            } else {
                None
            }
        });
        let linear = linear_result.is_some()
            && !function.parameters.iter().any(arguments::output)
            && statements[..statements.len() - 1].iter().all(|statement| {
                matches!(
                    statement.kind,
                    StmtKind::Variable(_)
                        | StmtKind::Variables(_)
                        | StmtKind::Expression(_)
                        | StmtKind::Empty
                )
            });
        if !linear {
            return self.expand_control(&function, arguments, span);
        }
        let Some(Stmt {
            kind: StmtKind::Return(Some(result)),
            ..
        }) = statements.last()
        else {
            return Err(unsupported(
                span,
                "ForceInline requires a final numeric return",
            ));
        };
        if arguments.len() != function.parameters.len() {
            return Err(unsupported(
                span,
                "ForceInline argument count does not match checked parameters",
            ));
        }
        self.active.push(name.into());
        let mut values = vec![];
        let mut bindings = BTreeMap::new();
        // Capture even unused arguments, in source argument order.
        for (parameter, argument) in function.parameters.iter().zip(arguments) {
            let local = self.storage(parameter, span);
            values.push(self.assign(local.clone(), argument.clone(), span));
            bindings.insert(parameter.name.clone(), local);
        }
        for statement in &statements[..statements.len() - 1] {
            match &statement.kind {
                StmtKind::Variable(variable) => {
                    self.local(variable, &mut bindings, &mut values, span)?
                }
                StmtKind::Variables(variables) => {
                    for variable in variables {
                        self.local(variable, &mut bindings, &mut values, span)?;
                    }
                }
                StmtKind::Expression(value) => values.push(capture(value, &bindings)?),
                StmtKind::Empty => {}
                _ => {
                    return Err(unsupported(
                        span,
                        "ForceInline control flow and nested declaration scopes are not implemented",
                    ));
                }
            }
        }
        let mut result = capture(result, &bindings)?;
        if result.ty != function.return_type {
            result = Expr {
                ty: function.return_type.clone(),
                span,
                kind: ExprKind::Cast {
                    value: Box::new(result),
                },
            };
        }
        values.push(result);
        let mut result = sequence(values, span);
        // Expanded callees may themselves contain requested inline calls.
        self.expression(&mut result)?;
        self.active.pop();
        Ok(result)
    }

    fn local(
        &mut self,
        variable: &Variable,
        bindings: &mut BTreeMap<String, Expr>,
        values: &mut Vec<Expr>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if !storable(&variable.ty)
            || variable
                .modifiers
                .iter()
                .any(|m| matches!(m.as_str(), "static" | "groupshared" | "volatile"))
        {
            return Err(unsupported(
                span,
                "ForceInline local storage must be an ordinary value",
            ));
        }
        let local = self.storage(variable, span);
        let initializer = variable
            .initializer
            .as_ref()
            .map(|value| capture(value, bindings))
            .transpose()?;
        bindings.insert(variable.name.clone(), local.clone());
        if let Some(value) = initializer {
            values.push(self.assign(local, value, span));
        }
        Ok(())
    }
}

impl Visitor for Lowerer {
    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        if self.active.len() > 1 {
            if self.remaining == 0 {
                return Err(unsupported(
                    statement.span,
                    "ForceInline expansion size limit exceeded",
                ));
            }
            self.remaining -= 1;
        }
        crate::visit::walk_statement(self, statement)
    }

    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        if self.active.len() > 1 {
            if self.remaining == 0 {
                return Err(unsupported(
                    expression.span,
                    "ForceInline expansion size limit exceeded",
                ));
            }
            self.remaining -= 1;
        }
        walk_expression(self, expression)?;
        if let ExprKind::Call { name, arguments } = &expression.kind
            && self.forced.contains(name)
        {
            *expression = self.expand(name, arguments, expression.span)?;
        }
        Ok(())
    }
}

fn capture(value: &Expr, bindings: &BTreeMap<String, Expr>) -> Result<Expr, Diagnostic> {
    struct Capture<'a>(&'a BTreeMap<String, Expr>);
    impl Visitor for Capture<'_> {
        fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
            walk_expression(self, expression)?;
            if let ExprKind::Name(name) = &expression.kind
                && let Some(value) = self.0.get(name)
            {
                let span = expression.span;
                *expression = value.clone();
                expression.span = span;
            }
            Ok(())
        }
    }
    let mut result = value.clone();
    Capture(bindings).expression(&mut result)?;
    Ok(result)
}

fn storable(ty: &Type) -> bool {
    !matches!(
        ty,
        Type::Void | Type::Error | Type::Meta(_) | Type::Callable { .. } | Type::InitializerList
    )
}

/// Comma preserves left-to-right sequencing under either association. Balance
/// generated trees so a long linear body does not exhaust recursive visitors.
fn sequence(mut values: Vec<Expr>, span: Span) -> Expr {
    if values.len() == 1 {
        return values.pop().unwrap();
    }
    let right = values.split_off(values.len() / 2);
    let left = sequence(values, span);
    let right = sequence(right, span);
    Expr {
        ty: right.ty.clone(),
        span,
        kind: ExprKind::Binary {
            op: ",".into(),
            left: Box::new(left),
            right: Box::new(right),
        },
    }
}

fn unsupported(span: Span, message: &str) -> Diagnostic {
    Diagnostic::error("I005", span, message)
}
