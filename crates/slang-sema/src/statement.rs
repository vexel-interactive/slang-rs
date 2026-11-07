use crate::{Type, checker::Checker, declaration::VariableScope};
use slang_syntax::{Function, Stmt, StmtKind, Variable};
use std::collections::BTreeMap;

impl Checker {
    pub(crate) fn initializer(&mut self, variable: &Variable) {
        if let Some(value) = &variable.initializer {
            let actual = self.expression(value);
            let expected = self.resolve_type(&variable.ty);
            self.require_conversion(&actual, &expected, value.span);
        } else if variable.modifiers.iter().any(|m| m == "const") {
            self.error(
                "T004",
                variable.span,
                "const variable requires an initializer",
            );
        }
    }

    pub(crate) fn function(&mut self, function: &Function) {
        for modifier in &function.modifiers {
            if !matches!(modifier.as_str(), "static" | "inline") {
                self.error(
                    "T001",
                    function.span,
                    format!("function modifier '{modifier}' is not implemented"),
                );
            }
        }
        self.return_type = self.resolve_type(&function.return_type);
        self.scopes.push(BTreeMap::new());
        for parameter in &function.parameters {
            self.bind(parameter, VariableScope::Parameter);
            if parameter.initializer.is_some() {
                self.error(
                    "T001",
                    parameter.span,
                    "default parameter values are not implemented",
                );
            }
        }
        if let Some(body) = &function.body {
            let returns = self.statement(body);
            if self.return_type != Type::Void && !returns {
                self.error(
                    "T005",
                    function.span,
                    "not all control-flow paths return a value",
                );
            }
        }
        self.scopes.pop();
    }

    fn scoped_statement(&mut self, statement: &Stmt) -> bool {
        self.scopes.push(BTreeMap::new());
        let returns = self.statement(statement);
        self.scopes.pop();
        returns
    }

    pub(crate) fn statement(&mut self, statement: &Stmt) -> bool {
        match &statement.kind {
            StmtKind::Block(statements) => {
                self.scopes.push(BTreeMap::new());
                let mut returns = false;
                for statement in statements {
                    returns |= self.statement(statement);
                }
                self.scopes.pop();
                returns
            }
            StmtKind::Variable(variable) => {
                self.initializer(variable);
                self.bind(variable, VariableScope::Local);
                false
            }
            StmtKind::Expression(expression) => {
                self.expression(expression);
                false
            }
            StmtKind::Return(value) => {
                let actual = value.as_ref().map_or(Type::Void, |v| self.expression(v));
                self.require_conversion(&actual, &self.return_type.clone(), statement.span);
                true
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.condition(condition);
                let a = self.scoped_statement(then_branch);
                let b = else_branch
                    .as_ref()
                    .is_some_and(|b| self.scoped_statement(b));
                a && b
            }
            StmtKind::While { condition, body } => {
                self.condition(condition);
                self.loops += 1;
                self.scoped_statement(body);
                self.loops -= 1;
                false
            }
            StmtKind::For {
                initializer,
                condition,
                increment,
                body,
            } => {
                self.scopes.push(BTreeMap::new());
                if !matches!(
                    initializer.kind,
                    StmtKind::Variable(_) | StmtKind::Expression(_) | StmtKind::Empty
                ) {
                    self.error("T001", initializer.span, "invalid for initializer");
                }
                self.statement(initializer);
                if let Some(condition) = condition {
                    self.condition(condition);
                }
                if let Some(increment) = increment {
                    self.expression(increment);
                }
                self.loops += 1;
                self.scoped_statement(body);
                self.loops -= 1;
                self.scopes.pop();
                false
            }
            StmtKind::Break | StmtKind::Continue => {
                if self.loops == 0 {
                    self.error(
                        "T007",
                        statement.span,
                        "loop control statement outside a loop",
                    );
                }
                false
            }
            StmtKind::Empty => false,
        }
    }
}
