use crate::{Type, checker::Checker, declaration::VariableScope};
use slang_syntax::{Function, Stmt, StmtKind, Variable};
use std::collections::{BTreeMap, BTreeSet};

/// Possible exits from a statement; branches can produce more than one kind.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Flow(u8);

impl Flow {
    const NONE: Self = Self(0);
    const FALLTHROUGH: Self = Self(1);
    const RETURN: Self = Self(2);
    const BREAK: Self = Self(4);
    const CONTINUE: Self = Self(8);

    fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    fn then(self, next: Self) -> Self {
        if self.0 & Self::FALLTHROUGH.0 != 0 {
            Self(self.0 & !Self::FALLTHROUGH.0).union(next)
        } else {
            self
        }
    }
    fn loop_exit(self, zero_iterations: bool) -> Self {
        let returns = Self(self.0 & Self::RETURN.0);
        if zero_iterations || self.0 & !Self::RETURN.0 != 0 {
            returns.union(Self::FALLTHROUGH)
        } else {
            returns
        }
    }
    fn switch_exit(self) -> Self {
        let exits = Self(self.0 & !Self::BREAK.0);
        if self.0 & Self::BREAK.0 != 0 {
            exits.union(Self::FALLTHROUGH)
        } else {
            exits
        }
    }
}

impl Checker {
    pub(crate) fn initialize_global(&mut self, variable: &Variable) {
        self.initializer(variable);
        if variable.modifiers.iter().any(|m| m == "const") {
            let value = variable
                .initializer
                .as_ref()
                .and_then(|expr| self.integer_constant(expr));
            if let Some(binding) = self.scopes[0].get_mut(&variable.name) {
                binding.constant =
                    value.map(|value| crate::constant::normalize(value, &binding.ty));
            }
        }
    }

    fn statements(&mut self, statements: &[Stmt]) -> Flow {
        let mut flow = Flow::FALLTHROUGH;
        for statement in statements {
            let next = self.statement(statement);
            flow = flow.then(next);
        }
        flow
    }

    pub(crate) fn initializer(&mut self, variable: &Variable) {
        if let Some(value) = &variable.initializer {
            let expected = self.resolve_type(&variable.ty);
            self.check_value(value, &expected);
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
            if !matches!(
                modifier.as_str(),
                "static" | "inline" | "public" | "internal"
            ) {
                self.error(
                    "T001",
                    function.span,
                    format!("function modifier '{modifier}' is not implemented"),
                );
            }
        }
        self.check_visibility_modifiers(&function.modifiers, function.span);
        self.return_type = self.resolve_type(&function.return_type);
        self.scopes.push(BTreeMap::new());
        for parameter in &function.parameters {
            self.bind(parameter, VariableScope::Parameter);
        }
        self.parameter_defaults(function);
        if let Some(body) = &function.body {
            let returns = self.statement(body);
            if self.return_type != Type::Void && returns != Flow::RETURN {
                self.error(
                    "T005",
                    function.span,
                    "not all control-flow paths return a value",
                );
            }
        }
        self.scopes.pop();
    }

    pub(crate) fn statement(&mut self, statement: &Stmt) -> Flow {
        match &statement.kind {
            StmtKind::Block(statements) => {
                self.scopes.push(BTreeMap::new());
                let returns = self.statements(statements);
                self.scopes.pop();
                returns
            }
            StmtKind::Variable(variable) => {
                self.initializer(variable);
                self.bind(variable, VariableScope::Local);
                Flow::FALLTHROUGH
            }
            StmtKind::Variables(variables) => {
                for variable in variables {
                    self.initializer(variable);
                    self.bind(variable, VariableScope::Local);
                }
                Flow::FALLTHROUGH
            }
            StmtKind::Expression(expression) => {
                if self.expression(expression) == Type::InitializerList {
                    self.error(
                        "T021",
                        expression.span,
                        "initializer list requires a concrete target type",
                    );
                }
                Flow::FALLTHROUGH
            }
            StmtKind::Return(value) => {
                if let Some(value) = value {
                    self.check_value(value, &self.return_type.clone());
                } else {
                    self.require_conversion(&Type::Void, &self.return_type.clone(), statement.span);
                }
                Flow::RETURN
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.condition(condition);
                let a = self.statement(then_branch);
                let b = else_branch
                    .as_ref()
                    .map_or(Flow::FALLTHROUGH, |b| self.statement(b));
                a.union(b)
            }
            StmtKind::While { condition, body } => {
                self.condition(condition);
                self.loops += 1;
                let flow = self.statement(body);
                self.loops -= 1;
                flow.loop_exit(true)
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
                    StmtKind::Variable(_)
                        | StmtKind::Variables(_)
                        | StmtKind::Expression(_)
                        | StmtKind::Empty
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
                let flow = self.statement(body);
                self.loops -= 1;
                self.scopes.pop();
                flow.loop_exit(true)
            }
            StmtKind::DoWhile { body, condition } => {
                self.loops += 1;
                let flow = self.statement(body);
                self.loops -= 1;
                self.condition(condition);
                flow.loop_exit(false)
            }
            StmtKind::Switch { selector, cases } => {
                let ty = self.expression(selector);
                if !matches!(
                    ty,
                    Type::Scalar(crate::Scalar::Int | crate::Scalar::Uint) | Type::Error
                ) {
                    self.error(
                        "T009",
                        selector.span,
                        "switch selector requires scalar int or uint",
                    );
                }
                self.switches += 1;
                self.scopes.push(BTreeMap::new());
                let mut labels = BTreeSet::new();
                let mut default = false;
                let mut flows = vec![];
                for case in cases {
                    if let Some(value) = &case.value {
                        let value_type = self.expression(value);
                        if !matches!(
                            value_type,
                            Type::Scalar(crate::Scalar::Int | crate::Scalar::Uint) | Type::Error
                        ) {
                            self.error(
                                "T017",
                                value.span,
                                "case label requires an integer constant",
                            );
                        }
                        if let Some(constant) = self.integer_constant(value) {
                            let constant = crate::constant::normalize(constant, &ty);
                            self.case_values.insert(value.id, constant);
                            if !labels.insert(constant) {
                                self.error("T017", case.span, "duplicate case value");
                            }
                        } else {
                            self.error(
                                "T017",
                                value.span,
                                "case value is not a supported integer constant",
                            );
                        }
                    } else if default {
                        self.error("T017", case.span, "duplicate default label");
                    } else {
                        default = true;
                    }
                    flows.push(self.statements(&case.statements));
                }
                self.scopes.pop();
                self.switches -= 1;
                let mut next_flow = Flow::FALLTHROUGH;
                let mut combined = if default {
                    Flow::NONE
                } else {
                    Flow::FALLTHROUGH
                };
                for flow in flows.into_iter().rev() {
                    next_flow = flow.then(next_flow);
                    combined = combined.union(next_flow);
                }
                combined.switch_exit()
            }
            StmtKind::Break => {
                if self.loops == 0 && self.switches == 0 {
                    self.error("T007", statement.span, "break outside a loop or switch");
                }
                Flow::BREAK
            }
            StmtKind::Continue => {
                if self.loops == 0 {
                    self.error("T007", statement.span, "continue outside a loop");
                }
                Flow::CONTINUE
            }
            StmtKind::Discard | StmtKind::Empty => Flow::FALLTHROUGH,
        }
    }
}
