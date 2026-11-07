use crate::{CheckedModule, Initialization, InitializationKind, ShaderStage, Type, span_key};
use slang_source::{Diagnostic, Span};
use slang_syntax::{Declaration, Expr, ExprKind, Stmt, StmtKind};
use std::collections::{BTreeMap, BTreeSet};

type Functions = BTreeMap<String, Vec<(Vec<Type>, usize)>>;

/// Propagate fragment-only operations through selected overloads without recursion.
pub(crate) fn validate(checked: &CheckedModule) -> Result<(), Vec<Diagnostic>> {
    if checked
        .entry_points
        .iter()
        .all(|e| e.stage == ShaderStage::Fragment)
    {
        return Ok(());
    }
    let functions: Vec<_> = checked
        .module
        .declarations
        .iter()
        .filter_map(|d| {
            if let Declaration::Function(f) = d
                && f.body.is_some()
            {
                Some(f)
            } else {
                None
            }
        })
        .collect();
    let mut indices = Functions::new();
    for (index, function) in functions.iter().enumerate() {
        let name = checked
            .function_names
            .get(&span_key(function.span))
            .unwrap_or(&function.name);
        let parameters = function
            .parameters
            .iter()
            .map(|p| checked.declaration_types[&span_key(p.span)].clone())
            .collect();
        indices
            .entry(name.clone())
            .or_default()
            .push((parameters, index));
    }
    let globals = checked
        .module
        .declarations
        .iter()
        .filter_map(|d| {
            if let Declaration::Global(v) = d {
                v.initializer.as_ref().map(|e| (v.name.clone(), e))
            } else {
                None
            }
        })
        .collect();
    let mut requirements = vec![None; functions.len()];
    let mut callers = vec![vec![]; functions.len()];
    for (index, function) in functions.iter().enumerate() {
        let mut scanner = Scanner {
            checked,
            indices: &indices,
            globals: &globals,
            work: vec![Work::Statement(function.body.as_ref().unwrap())],
            expressions: BTreeSet::new(),
            calls: BTreeSet::new(),
            fragment: None,
        };
        scanner.scan();
        requirements[index] = scanner.fragment;
        for callee in scanner.calls {
            callers[callee].push(index);
        }
    }
    let mut pending: Vec<_> = requirements
        .iter()
        .enumerate()
        .filter_map(|(i, r)| r.map(|_| i))
        .collect();
    while let Some(callee) = pending.pop() {
        for &caller in &callers[callee] {
            if requirements[caller].is_none() {
                requirements[caller] = requirements[callee];
                pending.push(caller);
            }
        }
    }
    let mut diagnostics = vec![];
    for entry in &checked.entry_points {
        if entry.stage == ShaderStage::Fragment {
            continue;
        }
        let Some(index) = functions.iter().position(|f| f.name == entry.name) else {
            continue;
        };
        if let Some((operation, span)) = requirements[index] {
            let stage = match entry.stage {
                ShaderStage::Compute => "compute",
                ShaderStage::Vertex => "vertex",
                ShaderStage::Fragment => unreachable!(),
            };
            diagnostics.push(Diagnostic::error(
                "T023",
                span,
                format!(
                    "{stage} entry point '{}' reaches '{operation}', which requires fragment stage",
                    entry.name
                ),
            ));
        }
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

enum Work<'a> {
    Statement(&'a Stmt),
    Expression(&'a Expr),
    Initialization(&'a Initialization),
}

struct Scanner<'a> {
    checked: &'a CheckedModule,
    indices: &'a Functions,
    globals: &'a BTreeMap<String, &'a Expr>,
    work: Vec<Work<'a>>,
    expressions: BTreeSet<slang_syntax::ExprId>,
    calls: BTreeSet<usize>,
    fragment: Option<(&'static str, Span)>,
}

impl<'a> Scanner<'a> {
    fn scan(&mut self) {
        while let Some(work) = self.work.pop() {
            match work {
                Work::Statement(s) => self.statement(s),
                Work::Expression(e) => self.expression(e),
                Work::Initialization(i) => match &i.kind {
                    InitializationKind::Value(e) | InitializationKind::DeclarationDefault(e) => {
                        self.work.push(Work::Expression(e))
                    }
                    InitializationKind::Aggregate(values) => {
                        self.work.extend(values.iter().map(Work::Initialization))
                    }
                    InitializationKind::Zero => {}
                },
            }
        }
    }

    fn statement(&mut self, statement: &'a Stmt) {
        match &statement.kind {
            StmtKind::Block(body) => self.work.extend(body.iter().map(Work::Statement)),
            StmtKind::Variable(v) => {
                if let Some(e) = &v.initializer {
                    self.work.push(Work::Expression(e));
                }
            }
            StmtKind::Variables(values) => {
                self.work.extend(
                    values
                        .iter()
                        .filter_map(|v| v.initializer.as_ref())
                        .map(Work::Expression),
                );
            }
            StmtKind::Expression(e) | StmtKind::Return(Some(e)) => {
                self.work.push(Work::Expression(e))
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.work.push(Work::Expression(condition));
                self.work.push(Work::Statement(then_branch));
                if let Some(branch) = else_branch {
                    self.work.push(Work::Statement(branch));
                }
            }
            StmtKind::While { condition, body } | StmtKind::DoWhile { condition, body } => {
                self.work.push(Work::Expression(condition));
                self.work.push(Work::Statement(body));
            }
            StmtKind::For {
                initializer,
                condition,
                increment,
                body,
            } => {
                self.work.push(Work::Statement(initializer));
                self.work.push(Work::Statement(body));
                self.work.extend(
                    [condition, increment]
                        .into_iter()
                        .filter_map(|e| e.as_ref())
                        .map(Work::Expression),
                );
            }
            StmtKind::Switch { selector, cases } => {
                self.work.push(Work::Expression(selector));
                for case in cases {
                    if let Some(e) = &case.value {
                        self.work.push(Work::Expression(e));
                    }
                    self.work
                        .extend(case.statements.iter().map(Work::Statement));
                }
            }
            StmtKind::Discard => {
                self.fragment.get_or_insert(("discard", statement.span));
            }
            _ => {}
        }
    }

    fn expression(&mut self, expression: &'a Expr) {
        if !self.expressions.insert(expression.id) {
            return;
        }
        if let Some(plan) = self.checked.initializations.get(&expression.id) {
            self.work.push(Work::Initialization(plan));
        }
        match &expression.kind {
            ExprKind::Name(name) => {
                if let Some(e) = self.globals.get(name) {
                    self.work.push(Work::Expression(e));
                }
            }
            ExprKind::InitializerList(values) => {
                self.work.extend(values.iter().map(Work::Expression))
            }
            ExprKind::Unary { value, .. }
            | ExprKind::Cast { value, .. }
            | ExprKind::Postfix { value, .. } => self.work.push(Work::Expression(value)),
            ExprKind::Binary { left, right, .. } => {
                self.work.push(Work::Expression(left));
                self.work.push(Work::Expression(right));
            }
            ExprKind::Call { callee, arguments } => {
                self.work.push(Work::Expression(callee));
                self.work.extend(arguments.iter().map(Work::Expression));
                if let Some(defaults) = self.checked.default_arguments.get(&callee.id) {
                    self.work.extend(defaults.iter().map(Work::Initialization));
                }
                if let Some(name) = self.checked.call_names.get(&callee.id) {
                    if let Some(Type::Callable { parameters, .. }) =
                        self.checked.expression_types.get(&callee.id)
                        && let Some(overloads) = self.indices.get(name)
                        && let Some((_, index)) =
                            overloads.iter().find(|(types, _)| types == parameters)
                    {
                        self.calls.insert(*index);
                    }
                } else if matches!(&callee.kind, ExprKind::Name(n) if n == "clip")
                    && matches!(
                        self.checked.expression_types.get(&callee.id),
                        Some(Type::Callable { .. })
                    )
                {
                    self.fragment.get_or_insert(("clip", expression.span));
                }
            }
            ExprKind::Member { base, .. } => self.work.push(Work::Expression(base)),
            ExprKind::Index { base, index } => {
                self.work.push(Work::Expression(base));
                self.work.push(Work::Expression(index));
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.work.push(Work::Expression(condition));
                self.work.push(Work::Expression(then_value));
                self.work.push(Work::Expression(else_value));
            }
            _ => {}
        }
    }
}
