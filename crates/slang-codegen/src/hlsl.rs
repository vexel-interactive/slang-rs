use crate::names::{identifier, type_name};
use slang_ir::*;
use slang_sema::Type;
use std::collections::BTreeSet;

pub(crate) fn emit(module: &Module) -> String {
    let mut emitter = Emitter {
        text: String::new(),
        indent: 0,
    };
    let mut emitted = BTreeSet::new();
    emitter.line("#pragma pack_matrix(column_major)");
    emitter.line("");
    for structure in &module.structures {
        emitter.structure(module, structure, &mut emitted);
    }
    let mut signatures = BTreeSet::new();
    for function in &module.functions {
        let signature = signature(function);
        if signatures.insert(signature.clone()) {
            emitter.line(&format!("{signature};"));
        }
    }
    emitter.line("");
    if let Some(binding) = module.global_uniform_binding {
        // This target name cannot be produced by identifier(): source names
        // beginning with _slang_ receive another prefix, and globals is unreserved.
        emitter.line(&format!("cbuffer _slang_globals : {}", binding.register));
        emitter.line("{");
        emitter.indent += 1;
        let mut uniforms: Vec<_> = module
            .globals
            .iter()
            .filter(|g| g.is_global_uniform())
            .collect();
        uniforms.sort_by_key(|g| g.uniform_layout.as_ref().map(|layout| layout.offset));
        for global in uniforms {
            emitter.line(&format!("{};", variable(global)));
        }
        emitter.indent -= 1;
        emitter.line("};");
        emitter.line("");
    }
    for global in &module.globals {
        if module.global_uniform_binding.is_none() || !global.is_global_uniform() {
            emitter.line(&format!("{};", variable(global)));
        }
    }
    if !module.globals.is_empty() {
        emitter.line("");
    }
    for function in &module.functions {
        if let Some(body) = &function.body {
            if let Some(entry) = module.entry_points.iter().find(|e| e.name == function.name)
                && let Some([x, y, z]) = entry.thread_group_size
            {
                emitter.line(&format!("[numthreads({x}, {y}, {z})]"));
            }
            emitter.line(&signature(function));
            emitter.statement(body);
            emitter.line("");
        }
    }
    emitter.text
}

struct Emitter {
    text: String,
    indent: usize,
}

impl Emitter {
    fn line(&mut self, text: &str) {
        if !text.is_empty() {
            self.text.push_str(&"    ".repeat(self.indent));
            self.text.push_str(text);
        }
        self.text.push('\n');
    }

    fn structure(&mut self, module: &Module, structure: &Struct, emitted: &mut BTreeSet<String>) {
        if !emitted.insert(structure.name.clone()) {
            return;
        }
        for field in &structure.fields {
            let mut ty = &field.ty;
            while let Type::Array(element, _) = ty {
                ty = element;
            }
            if let Type::Struct(name) = ty
                && let Some(dependency) = module.structures.iter().find(|s| &s.name == name)
            {
                self.structure(module, dependency, emitted);
            }
        }
        self.line(&format!("struct {}", identifier(&structure.name)));
        self.line("{");
        self.indent += 1;
        for field in &structure.fields {
            self.line(&format!("{};", variable(field)));
        }
        self.indent -= 1;
        self.line("};");
        self.line("");
    }

    fn controlled_body(&mut self, body: &Stmt) {
        if matches!(body.kind, StmtKind::Block(_)) {
            self.statement(body);
        } else {
            self.line("{");
            self.indent += 1;
            self.statement(body);
            self.indent -= 1;
            self.line("}");
        }
    }

    fn statement(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Sequence(statements) => {
                for statement in statements {
                    self.statement(statement);
                }
            }
            StmtKind::Block(statements) => {
                self.line("{");
                self.indent += 1;
                for statement in statements {
                    self.statement(statement);
                }
                self.indent -= 1;
                self.line("}");
            }
            StmtKind::Variable(value) => self.line(&format!("{};", variable(value))),
            StmtKind::Variables(variables) => {
                for value in variables {
                    self.line(&format!("{};", variable(value)));
                }
            }
            StmtKind::Expression(value) => self.line(&format!("{};", expression(value))),
            StmtKind::Return(value) => self.line(&value.as_ref().map_or_else(
                || "return;".into(),
                |v| format!("return {};", expression(v)),
            )),
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.line(&format!("if ({})", expression(condition)));
                self.controlled_body(then_branch);
                if let Some(branch) = else_branch {
                    self.line("else");
                    self.controlled_body(branch);
                }
            }
            StmtKind::While { condition, body } => {
                self.line(&format!("while ({})", expression(condition)));
                self.controlled_body(body);
            }
            StmtKind::DoWhile { body, condition } => {
                self.line("do");
                self.controlled_body(body);
                self.line(&format!("while ({});", expression(condition)));
            }
            StmtKind::Switch { selector, cases } => {
                self.line(&format!("switch ({})", expression(selector)));
                self.line("{");
                self.indent += 1;
                for case in cases {
                    self.line(&case.value.as_ref().map_or_else(
                        || "default:".into(),
                        |value| format!("case {}:", expression(value)),
                    ));
                    self.indent += 1;
                    for statement in &case.statements {
                        self.statement(statement);
                    }
                    self.indent -= 1;
                }
                self.indent -= 1;
                self.line("}");
            }
            StmtKind::For {
                initializer,
                condition,
                increment,
                body,
            } => {
                // Independent declarations preserve mixed array suffixes. Keep
                // their enclosing scope identical to the source for loop.
                let group = matches!(
                    initializer.kind,
                    StmtKind::Variables(_) | StmtKind::Sequence(_)
                );
                if group {
                    self.line("{");
                    self.indent += 1;
                    self.statement(initializer);
                }
                let init = match &initializer.kind {
                    StmtKind::Variable(v) => variable(v),
                    StmtKind::Expression(e) => expression(e),
                    _ => String::new(),
                };
                let condition = condition.as_ref().map_or_else(String::new, expression);
                let increment = increment.as_ref().map_or_else(String::new, expression);
                self.line(&format!("for ({init}; {condition}; {increment})"));
                self.controlled_body(body);
                if group {
                    self.indent -= 1;
                    self.line("}");
                }
            }
            StmtKind::Break => self.line("break;"),
            StmtKind::Continue => self.line("continue;"),
            StmtKind::Discard => self.line("discard;"),
            StmtKind::Empty => self.line(";"),
        }
    }
}

fn variable(variable: &Variable) -> String {
    let mut text = String::new();
    for modifier in &variable.modifiers {
        text.push_str(modifier);
        text.push(' ');
    }
    let mut ty = &variable.ty;
    let mut suffix = String::new();
    while let Type::Array(element, length) = ty {
        suffix.push_str(&format!("[{length}]"));
        ty = element;
    }
    text.push_str(&format!(
        "{} {}{suffix}",
        type_name(ty),
        identifier(&variable.name)
    ));
    if let Some(semantic) = &variable.semantic {
        text.push_str(&format!(" : {semantic}"));
    }
    if let Some(binding) = &variable.binding {
        text.push_str(&format!(" : {}", binding.register));
    }
    if let Some(initializer) = &variable.initializer {
        text.push_str(&format!(" = {}", expression(initializer)));
    }
    text
}

fn signature(function: &Function) -> String {
    let modifiers = if function.modifiers.is_empty() {
        String::new()
    } else {
        format!("{} ", function.modifiers.join(" "))
    };
    let params = function
        .parameters
        .iter()
        .map(variable)
        .collect::<Vec<_>>()
        .join(", ");
    let semantic = function
        .semantic
        .as_ref()
        .map_or_else(String::new, |s| format!(" : {s}"));
    format!(
        "{modifiers}{} {}({params}){semantic}",
        type_name(&function.return_type),
        identifier(&function.name)
    )
}

fn expression(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::DeclarationDefault(value) => expression(value),
        ExprKind::Zero => format!("(({})0)", type_name(&expr.ty)),
        ExprKind::Construct(values) => {
            let values = values.iter().map(expression).collect::<Vec<_>>().join(", ");
            if matches!(expr.ty, Type::Vector(..) | Type::Matrix(..)) {
                format!("{}({values})", type_name(&expr.ty))
            } else {
                format!("{{{values}}}")
            }
        }
        ExprKind::Integer(text) | ExprKind::Float(text) => text.clone(),
        ExprKind::Name(name) => identifier(name),
        ExprKind::Bool(value) => value.to_string(),
        ExprKind::Unary { op, value } => format!("({op}{})", expression(value)),
        ExprKind::Cast { value } => format!("(({}){})", type_name(&expr.ty), expression(value)),
        ExprKind::Binary { op, left, right } => {
            format!("({} {op} {})", expression(left), expression(right))
        }
        ExprKind::Call { name, arguments }
            if Type::builtin(name).as_ref() == Some(&expr.ty)
                && arguments.len() == 1
                && matches!(expr.ty, Type::Vector(..) | Type::Matrix(..))
                && matches!(arguments[0].ty, Type::Scalar(_)) =>
        {
            format!("(({}){})", expr.ty, expression(&arguments[0]))
        }
        ExprKind::Call { name, arguments } => format!(
            "{}({})",
            if Type::builtin(name).is_some() {
                name.clone()
            } else {
                identifier(name)
            },
            arguments
                .iter()
                .map(expression)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ExprKind::IntrinsicCall { name, arguments } => format!(
            "{name}({})",
            arguments
                .iter()
                .map(expression)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ExprKind::Member { base, name } => format!("({}).{}", expression(base), identifier(name)),
        ExprKind::MethodCall {
            receiver,
            name,
            arguments,
        } => format!(
            "({}).{name}({})",
            expression(receiver),
            arguments
                .iter()
                .map(expression)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ExprKind::Index { base, index } => format!("({})[{}]", expression(base), expression(index)),
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => format!(
            "({} ? {} : {})",
            expression(condition),
            expression(then_value),
            expression(else_value)
        ),
        ExprKind::Postfix { op, value } => format!("({}{op})", expression(value)),
    }
}
