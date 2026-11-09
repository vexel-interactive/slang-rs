use crate::{
    CheckedModule, Initialization, InitializationKind, ShaderStage, Type, checker::SpanKey,
    span_key,
};
use slang_source::{Diagnostic, Span};
use slang_syntax::{Declaration, Expr, ExprKind, Stmt, StmtKind, Variable};
use std::collections::{BTreeMap, BTreeSet};

type Functions = BTreeMap<String, Vec<(Vec<Type>, usize)>>;

/// Original checked operation requiring an entry-point capability.
#[derive(Clone, Copy, Debug)]
pub struct RequiredOperation {
    pub name: &'static str,
    pub span: Span,
}

type Operation = RequiredOperation;

/// Capabilities that require target-specific profile selection after checking.
#[derive(Clone, Copy, Debug, Default)]
pub struct EntryPointRequirements {
    pub derivatives: Option<RequiredOperation>,
}

#[derive(Clone, Copy, Default)]
struct Requirements {
    fragment: Option<Operation>,
    compute: Option<Operation>,
    derivatives: Option<Operation>,
}

impl Requirements {
    fn any(self) -> bool {
        self.fragment.is_some() || self.compute.is_some() || self.derivatives.is_some()
    }

    /// Requirements only grow, including when the call graph contains cycles.
    fn merge(&mut self, other: Self) -> bool {
        let mut changed = false;
        if self.fragment.is_none() && other.fragment.is_some() {
            self.fragment = other.fragment;
            changed = true;
        }
        if self.compute.is_none() && other.compute.is_some() {
            self.compute = other.compute;
            changed = true;
        }
        if self.derivatives.is_none() && other.derivatives.is_some() {
            self.derivatives = other.derivatives;
            changed = true;
        }
        changed
    }
}

/// Propagate stage requirements through selected overloads without recursion.
pub(crate) fn validate(
    checked: &CheckedModule,
) -> Result<BTreeMap<String, EntryPointRequirements>, Vec<Diagnostic>> {
    if checked.entry_points.is_empty() {
        return Ok(BTreeMap::new());
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
                Some((span_key(v.span), v))
            } else {
                None
            }
        })
        .collect();
    let mut requirements = vec![Requirements::default(); functions.len()];
    let mut callers = vec![vec![]; functions.len()];
    for (index, function) in functions.iter().enumerate() {
        let mut scanner = Scanner {
            checked,
            indices: &indices,
            globals: &globals,
            work: vec![Work::Statement(function.body.as_ref().unwrap())],
            expressions: BTreeSet::new(),
            calls: BTreeSet::new(),
            requirements: Requirements::default(),
        };
        if let Some(values) = checked
            .constructor_initializations
            .get(&span_key(function.span))
        {
            scanner
                .work
                .extend(values.iter().map(|(_, value)| Work::Initialization(value)));
        }
        scanner.scan();
        requirements[index] = scanner.requirements;
        for callee in scanner.calls {
            callers[callee].push(index);
        }
    }
    let mut pending: Vec<_> = requirements
        .iter()
        .enumerate()
        .filter_map(|(i, r)| r.any().then_some(i))
        .collect();
    while let Some(callee) = pending.pop() {
        let inherited = requirements[callee];
        for &caller in &callers[callee] {
            if requirements[caller].merge(inherited) {
                pending.push(caller);
            }
        }
    }
    let mut diagnostics = vec![];
    let mut entry_requirements = BTreeMap::new();
    for entry in &checked.entry_points {
        let Some(index) = functions.iter().position(|f| f.name == entry.name) else {
            continue;
        };
        for (required, operation) in [
            (ShaderStage::Fragment, requirements[index].fragment),
            (ShaderStage::Compute, requirements[index].compute),
        ] {
            let Some(operation) = operation.filter(|_| entry.stage != required) else {
                continue;
            };
            diagnostics.push(Diagnostic::error(
                "T023",
                operation.span,
                format!(
                    "{} entry point '{}' reaches '{}', which requires {} stage",
                    stage_name(entry.stage),
                    entry.name,
                    operation.name,
                    stage_name(required)
                ),
            ));
        }
        let derivatives = requirements[index].derivatives;
        if let Some(operation) = derivatives {
            match entry.stage {
                ShaderStage::Vertex => diagnostics.push(Diagnostic::error("T023", operation.span,
                    format!("vertex entry point '{}' reaches '{}', which requires fragment or compute stage", entry.name, operation.name))),
                ShaderStage::Compute if !derivative_group(entry.thread_group_size.unwrap()) => diagnostics.push(Diagnostic::error("T024", operation.span,
                    format!("compute entry point '{}' uses derivatives with an invalid thread-group size; use a 1D group with x divisible by four or x and y both even", entry.name))),
                _ => {},
            }
        }
        entry_requirements.insert(entry.name.clone(), EntryPointRequirements { derivatives });
    }
    if diagnostics.is_empty() {
        Ok(entry_requirements)
    } else {
        Err(diagnostics)
    }
}

fn derivative_group([x, y, z]: [u32; 3]) -> bool {
    (y == 1 && z == 1 && x.is_multiple_of(4)) || (x.is_multiple_of(2) && y.is_multiple_of(2))
}

fn stage_name(stage: ShaderStage) -> &'static str {
    match stage {
        ShaderStage::Compute => "compute",
        ShaderStage::Vertex => "vertex",
        ShaderStage::Fragment => "fragment",
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
    globals: &'a BTreeMap<SpanKey, &'a Variable>,
    work: Vec<Work<'a>>,
    expressions: BTreeSet<slang_syntax::ExprId>,
    calls: BTreeSet<usize>,
    requirements: Requirements,
}

impl<'a> Scanner<'a> {
    fn scan(&mut self) {
        while let Some(work) = self.work.pop() {
            match work {
                Work::Statement(s) => self.statement(s),
                Work::Expression(e) => self.expression(e),
                Work::Initialization(i) => match &i.kind {
                    InitializationKind::ConstructorCall {
                        name,
                        parameters,
                        arguments,
                    } => {
                        if let Some(overloads) = self.indices.get(name)
                            && let Some((_, index)) =
                                overloads.iter().find(|(types, _)| types == parameters)
                        {
                            self.calls.insert(*index);
                        }
                        self.work.extend(arguments.iter().map(Work::Initialization));
                    }
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
                if let Some(value) = self.checked.variable_initializations.get(&span_key(v.span)) {
                    self.work.push(Work::Initialization(value));
                }
            }
            StmtKind::Variables(values) => {
                self.work.extend(
                    values
                        .iter()
                        .filter_map(|v| {
                            self.checked.variable_initializations.get(&span_key(v.span))
                        })
                        .map(Work::Initialization),
                );
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
                self.requirements.fragment.get_or_insert(Operation {
                    name: "discard",
                    span: statement.span,
                });
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
        if let Some(declaration) = self.checked.global_references.get(&expression.id)
            && let Some(global) = self.globals.get(declaration)
        {
            if global.modifiers.iter().any(|m| m == "groupshared") {
                self.requirements.compute.get_or_insert(Operation {
                    name: "groupshared",
                    span: expression.span,
                });
            }
            if let Some(value) = &global.initializer {
                self.work.push(Work::Expression(value));
            }
            if let Some(value) = self.checked.variable_initializations.get(declaration) {
                self.work.push(Work::Initialization(value));
            }
        }
        if self.checked.static_references.contains_key(&expression.id) {
            return;
        }
        match &expression.kind {
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
                // Static member lookup checks the base's type but does not
                // evaluate its expression, including constructors and defaults.
                if !matches!(
                    self.checked.method_receivers.get(&callee.id),
                    Some(crate::MethodReceiver::Static)
                ) {
                    self.work.push(Work::Expression(callee));
                }
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
                } else if let ExprKind::Name(name) = &callee.kind
                    && matches!(
                        self.checked.expression_types.get(&callee.id),
                        Some(Type::Callable { .. })
                    )
                {
                    let operation = match name.as_str() {
                        "clip" => Some((&mut self.requirements.fragment, "clip")),
                        "GroupMemoryBarrier" => {
                            Some((&mut self.requirements.compute, "GroupMemoryBarrier"))
                        }
                        "AllMemoryBarrier" => {
                            Some((&mut self.requirements.compute, "AllMemoryBarrier"))
                        }
                        "GroupMemoryBarrierWithGroupSync" => Some((
                            &mut self.requirements.compute,
                            "GroupMemoryBarrierWithGroupSync",
                        )),
                        "DeviceMemoryBarrierWithGroupSync" => Some((
                            &mut self.requirements.compute,
                            "DeviceMemoryBarrierWithGroupSync",
                        )),
                        "AllMemoryBarrierWithGroupSync" => Some((
                            &mut self.requirements.compute,
                            "AllMemoryBarrierWithGroupSync",
                        )),
                        name => crate::intrinsic::derivative_name(name)
                            .map(|name| (&mut self.requirements.derivatives, name)),
                    };
                    if let Some((requirement, name)) = operation {
                        requirement.get_or_insert(Operation {
                            name,
                            span: expression.span,
                        });
                    }
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
