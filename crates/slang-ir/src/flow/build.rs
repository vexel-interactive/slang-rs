use super::{constant, *};
use slang_sema::{Scalar, Type};

const MAX_ANALYSIS_VALUES: usize = 262_144;

#[derive(Clone, Copy)]
struct Evaluation {
    value: ValueId,
    pure: bool,
    propagated: bool,
}

#[derive(Clone, Copy)]
enum CallKind {
    User,
    Intrinsic,
    Method,
}

struct Builder {
    graph: Graph,
    local_types: Vec<Type>,
    next_local: usize,
    current: BlockId,
    state: Vec<ValueId>,
    scopes: Vec<BTreeMap<String, Option<usize>>>,
    controls: Vec<(BlockId, Option<BlockId>)>,
    signatures: BTreeMap<String, Vec<bool>>,
    globals: BTreeMap<String, (Type, i64)>,
    sites: BTreeMap<usize, Site>,
}

pub(super) fn build(module: &Module, function: &Function) -> Option<Analysis> {
    let body = function.body.as_ref()?;
    let mut local_types = vec![];
    for parameter in &function.parameters {
        collect_variable(parameter, &mut local_types);
    }
    let mut nodes = 1;
    collect(body, &mut local_types, &mut nodes);
    // Each structured node needs at most five blocks; each expression node is
    // included separately for short-circuit branches and temporary values.
    if nodes
        .saturating_mul(local_types.len().saturating_add(4))
        .saturating_mul(5)
        > MAX_ANALYSIS_VALUES
    {
        return None;
    }
    let mut builder = Builder {
        graph: Graph {
            blocks: vec![],
            values: vec![],
            return_type: function.return_type.clone(),
        },
        local_types,
        next_local: 0,
        current: BlockId(0),
        state: vec![],
        scopes: vec![BTreeMap::new()],
        controls: vec![],
        signatures: module
            .functions
            .iter()
            .map(|f| {
                (
                    f.name.clone(),
                    f.parameters
                        .iter()
                        .map(|p| {
                            p.modifiers
                                .iter()
                                .any(|m| matches!(m.as_str(), "out" | "inout"))
                        })
                        .collect(),
                )
            })
            .collect(),
        globals: module
            .globals
            .iter()
            .filter_map(|v| {
                let s = constant::scalar(&v.ty)?;
                if !v.modifiers.iter().any(|m| m == "const") {
                    return None;
                }
                let value = literal_expression(v.initializer.as_ref()?)?;
                Some((
                    v.name.clone(),
                    (v.ty.clone(), constant::normalize(value, s)),
                ))
            })
            .collect(),
        sites: BTreeMap::new(),
    };
    builder.graph.blocks.push(Block {
        parameters: vec![],
        instructions: vec![],
        terminator: Terminator::Unreachable,
    });
    for ty in builder.local_types.clone() {
        let value = builder.emit(ty, Operation::Unknown(vec![]));
        builder.state.push(value);
    }
    for parameter in &function.parameters {
        builder.bind(parameter);
    }
    builder.statement(body);
    debug_assert_eq!(builder.next_local, builder.local_types.len());
    Some(Analysis {
        graph: builder.graph,
        sites: builder.sites,
    })
}

fn literal_expression(expression: &Expr) -> Option<i64> {
    match &expression.kind {
        ExprKind::Integer(text) => constant::literal(text),
        ExprKind::Bool(value) => Some(i64::from(*value)),
        ExprKind::Zero => Some(0),
        _ => None,
    }
}

fn collect_variable(variable: &Variable, locals: &mut Vec<Type>) {
    if promotable(variable) {
        locals.push(variable.ty.clone());
    }
}

fn promotable(variable: &Variable) -> bool {
    constant::scalar(&variable.ty).is_some()
        && !variable
            .modifiers
            .iter()
            .any(|m| matches!(m.as_str(), "static" | "groupshared" | "volatile"))
}

fn collect_expression(expression: &Expr, nodes: &mut usize) {
    *nodes += 1;
    match &expression.kind {
        ExprKind::Construct(values)
        | ExprKind::Call {
            arguments: values, ..
        }
        | ExprKind::IntrinsicCall {
            arguments: values, ..
        } => {
            for value in values {
                collect_expression(value, nodes);
            }
        }
        ExprKind::MethodCall {
            receiver,
            arguments,
            ..
        } => {
            collect_expression(receiver, nodes);
            for value in arguments {
                collect_expression(value, nodes);
            }
        }
        ExprKind::ConstantBufferLoad(value)
        | ExprKind::DeclarationDefault(value)
        | ExprKind::Unary { value, .. }
        | ExprKind::Cast { value }
        | ExprKind::Postfix { value, .. } => collect_expression(value, nodes),
        ExprKind::Binary { left, right, .. } => {
            collect_expression(left, nodes);
            collect_expression(right, nodes);
        }
        ExprKind::Member { base, .. } => collect_expression(base, nodes),
        ExprKind::Index { base, index } => {
            collect_expression(base, nodes);
            collect_expression(index, nodes);
        }
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            collect_expression(condition, nodes);
            collect_expression(then_value, nodes);
            collect_expression(else_value, nodes);
        }
        _ => {}
    }
}

fn collect(statement: &Stmt, locals: &mut Vec<Type>, nodes: &mut usize) {
    *nodes += 1;
    match &statement.kind {
        StmtKind::Block(values) | StmtKind::Sequence(values) => {
            for value in values {
                collect(value, locals, nodes);
            }
        }
        StmtKind::Variable(variable) => {
            collect_variable(variable, locals);
            if let Some(value) = &variable.initializer {
                collect_expression(value, nodes);
            }
        }
        StmtKind::Variables(values) => {
            for variable in values {
                collect_variable(variable, locals);
                if let Some(value) = &variable.initializer {
                    collect_expression(value, nodes);
                }
            }
        }
        StmtKind::Expression(value) | StmtKind::Return(Some(value)) => {
            collect_expression(value, nodes)
        }
        StmtKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expression(condition, nodes);
            collect(then_branch, locals, nodes);
            if let Some(value) = else_branch {
                collect(value, locals, nodes);
            }
        }
        StmtKind::While { condition, body } | StmtKind::DoWhile { body, condition } => {
            collect_expression(condition, nodes);
            collect(body, locals, nodes);
        }
        StmtKind::For {
            initializer,
            condition,
            increment,
            body,
        } => {
            collect(initializer, locals, nodes);
            if let Some(value) = condition {
                collect_expression(value, nodes);
            }
            collect(body, locals, nodes);
            if let Some(value) = increment {
                collect_expression(value, nodes);
            }
        }
        StmtKind::Switch { selector, cases } => {
            collect_expression(selector, nodes);
            for case in cases {
                if let Some(value) = &case.value {
                    collect_expression(value, nodes);
                }
                for value in &case.statements {
                    collect(value, locals, nodes);
                }
            }
        }
        _ => {}
    }
}

impl Builder {
    fn emit(&mut self, ty: Type, operation: Operation) -> ValueId {
        let value = ValueId(self.graph.values.len());
        let block = &mut self.graph.blocks[self.current.0];
        self.graph.values.push(Value {
            ty,
            definition: Definition::Instruction {
                block: self.current,
                index: block.instructions.len(),
            },
        });
        block.instructions.push(Instruction {
            result: value,
            operation,
        });
        value
    }

    fn block(&mut self, result: Option<&Type>) -> BlockId {
        let id = BlockId(self.graph.blocks.len());
        let mut parameters = vec![];
        for ty in self.local_types.iter().chain(result) {
            let value = ValueId(self.graph.values.len());
            self.graph.values.push(Value {
                ty: ty.clone(),
                definition: Definition::Parameter {
                    block: id,
                    index: parameters.len(),
                },
            });
            parameters.push(value);
        }
        self.graph.blocks.push(Block {
            parameters,
            instructions: vec![],
            terminator: Terminator::Unreachable,
        });
        id
    }

    fn enter(&mut self, block: BlockId) {
        self.current = block;
        self.state = self.graph.blocks[block.0].parameters[..self.local_types.len()].to_vec();
    }

    fn edge(&self, target: BlockId, result: Option<ValueId>) -> Edge {
        let mut arguments = self.state.clone();
        arguments.extend(result);
        Edge { target, arguments }
    }

    fn jump(&mut self, target: BlockId, result: Option<ValueId>) {
        self.graph.blocks[self.current.0].terminator = Terminator::Jump(self.edge(target, result));
    }

    fn branch(&mut self, condition: ValueId, on_true: BlockId, on_false: BlockId) {
        let condition = self.convert(condition, &Type::Scalar(Scalar::Bool));
        self.graph.blocks[self.current.0].terminator = Terminator::Branch {
            condition,
            on_true: self.edge(on_true, None),
            on_false: self.edge(on_false, None),
        };
    }

    fn convert(&mut self, value: ValueId, ty: &Type) -> ValueId {
        if self.graph.values[value.0].ty == *ty {
            value
        } else {
            self.emit(ty.clone(), Operation::Convert(value))
        }
    }

    fn bind(&mut self, variable: &Variable) -> Option<usize> {
        let local = promotable(variable).then(|| {
            let local = self.next_local;
            self.next_local += 1;
            local
        });
        self.scopes
            .last_mut()
            .unwrap()
            .insert(variable.name.clone(), local);
        local
    }

    fn lookup(&self, name: &str) -> Option<usize> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
            .flatten()
    }

    fn variable(&mut self, variable: &Variable) {
        let local = self.bind(variable);
        if let Some(expression) = &variable.initializer {
            let value = self.expression(expression).value;
            if let Some(local) = local {
                self.state[local] = self.convert(value, &variable.ty);
            }
        } else if let Some(local) = local {
            // A declaration inside a loop starts a new lifetime on each visit.
            self.invalidate(local, vec![]);
        }
    }

    fn invalidate(&mut self, local: usize, dependencies: Vec<ValueId>) {
        self.state[local] = self.emit(
            self.local_types[local].clone(),
            Operation::Unknown(dependencies),
        );
    }

    fn statement(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Block(values) => {
                self.scopes.push(BTreeMap::new());
                for value in values {
                    self.statement(value);
                }
                self.scopes.pop();
            }
            StmtKind::Sequence(values) => {
                for value in values {
                    self.statement(value);
                }
            }
            StmtKind::Variable(variable) => self.variable(variable),
            StmtKind::Variables(values) => {
                for variable in values {
                    self.variable(variable);
                }
            }
            StmtKind::Expression(value) => {
                self.expression(value);
            }
            StmtKind::Return(value) => {
                let value = value.as_ref().map(|e| {
                    let value = self.expression(e).value;
                    self.convert(value, &self.graph.return_type.clone())
                });
                self.graph.blocks[self.current.0].terminator = Terminator::Return(value);
                let dead = self.block(None);
                self.enter(dead);
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = self.expression(condition).value;
                let yes = self.block(None);
                let no = self.block(None);
                let after = self.block(None);
                self.branch(condition, yes, no);
                self.enter(yes);
                self.statement(then_branch);
                self.jump(after, None);
                self.enter(no);
                if let Some(branch) = else_branch {
                    self.statement(branch);
                }
                self.jump(after, None);
                self.enter(after);
            }
            StmtKind::While { condition, body } => {
                let head = self.block(None);
                let inside = self.block(None);
                let after = self.block(None);
                self.jump(head, None);
                self.enter(head);
                let value = self.expression(condition).value;
                self.branch(value, inside, after);
                self.controls.push((after, Some(head)));
                self.enter(inside);
                self.statement(body);
                self.jump(head, None);
                self.controls.pop();
                self.enter(after);
            }
            StmtKind::DoWhile { body, condition } => {
                let inside = self.block(None);
                let test = self.block(None);
                let after = self.block(None);
                self.jump(inside, None);
                self.controls.push((after, Some(test)));
                self.enter(inside);
                self.statement(body);
                self.jump(test, None);
                self.enter(test);
                let value = self.expression(condition).value;
                self.branch(value, inside, after);
                self.controls.pop();
                self.enter(after);
            }
            StmtKind::For {
                initializer,
                condition,
                increment,
                body,
            } => {
                self.scopes.push(BTreeMap::new());
                self.statement(initializer);
                let head = self.block(None);
                let inside = self.block(None);
                let step = self.block(None);
                let after = self.block(None);
                self.jump(head, None);
                self.enter(head);
                if let Some(condition) = condition {
                    let value = self.expression(condition).value;
                    self.branch(value, inside, after);
                } else {
                    self.jump(inside, None);
                }
                self.controls.push((after, Some(step)));
                self.enter(inside);
                self.statement(body);
                self.jump(step, None);
                self.enter(step);
                if let Some(value) = increment {
                    self.expression(value);
                }
                self.jump(head, None);
                self.controls.pop();
                self.scopes.pop();
                self.enter(after);
            }
            StmtKind::Switch { selector, cases } => {
                let value = self.expression(selector).value;
                let after = self.block(None);
                let blocks: Vec<_> = cases.iter().map(|_| self.block(None)).collect();
                let mut default = after;
                let mut choices = vec![];
                for (case, &block) in cases.iter().zip(&blocks) {
                    if let Some(label) = &case.value {
                        if let Some(bits) = literal_expression(label) {
                            choices.push((bits, self.edge(block, None)));
                        }
                    } else {
                        default = block;
                    }
                }
                self.graph.blocks[self.current.0].terminator = Terminator::Switch {
                    selector: value,
                    cases: choices,
                    default: self.edge(default, None),
                };
                self.controls.push((after, None));
                self.scopes.push(BTreeMap::new());
                for (index, case) in cases.iter().enumerate() {
                    self.enter(blocks[index]);
                    for value in &case.statements {
                        self.statement(value);
                    }
                    self.jump(blocks.get(index + 1).copied().unwrap_or(after), None);
                }
                self.scopes.pop();
                self.controls.pop();
                self.enter(after);
            }
            StmtKind::Break | StmtKind::Continue => {
                let target = if matches!(statement.kind, StmtKind::Break) {
                    self.controls.last().map(|c| c.0)
                } else {
                    self.controls.iter().rev().find_map(|c| c.1)
                };
                if let Some(target) = target {
                    self.jump(target, None);
                }
                let dead = self.block(None);
                self.enter(dead);
            }
            StmtKind::Discard => {
                self.graph.blocks[self.current.0].terminator = Terminator::Unreachable;
                let dead = self.block(None);
                self.enter(dead);
            }
            StmtKind::Empty => {}
        }
    }

    /// Address expressions are evaluated once, without substituting writable
    /// names. Aggregate locations deliberately remain outside scalar promotion.
    fn location(&mut self, expression: &Expr) -> Option<usize> {
        match &expression.kind {
            ExprKind::Name(name) => self.lookup(name),
            ExprKind::GlobalName(_) => None,
            ExprKind::Member { base, .. } => {
                if let Some(local) = self.location(base) {
                    self.invalidate(local, vec![self.state[local]]);
                }
                None
            }
            ExprKind::Index { base, index } => {
                let root = self.location(base);
                self.expression(index);
                if let Some(local) = root {
                    self.invalidate(local, vec![self.state[local]]);
                }
                None
            }
            _ => {
                self.expression(expression);
                None
            }
        }
    }

    fn binary(
        &mut self,
        op: &str,
        left: ValueId,
        right: ValueId,
        ty: &Type,
        compound: bool,
    ) -> ValueId {
        let (mut left, mut right) = (left, right);
        let left_ty = self.graph.values[left.0].ty.clone();
        let right_ty = self.graph.values[right.0].ty.clone();
        let common = if compound || matches!(op, "<<" | ">>") {
            Some(left_ty)
        } else {
            Type::common(&left_ty, &right_ty)
        };
        if let Some(common) = common {
            left = self.convert(left, &common);
            if !matches!(op, "<<" | ">>") {
                right = self.convert(right, &common);
            }
        }
        self.emit(
            ty.clone(),
            Operation::Binary {
                op: op.into(),
                left,
                right,
            },
        )
    }

    fn call(&mut self, name: &str, arguments: &[Expr], ty: &Type, kind: CallKind) -> Evaluation {
        if !matches!(kind, CallKind::Method)
            && Type::builtin(name).as_ref() == Some(ty)
            && arguments.len() == 1
        {
            let source = self.expression(&arguments[0]);
            return Evaluation {
                value: self.convert(source.value, ty),
                ..source
            };
        }
        let outputs = match kind {
            CallKind::Intrinsic if name == "asuint" && arguments.len() == 3 => {
                Some(vec![false, true, true])
            }
            CallKind::Intrinsic if readonly_intrinsic(name) => Some(vec![false; arguments.len()]),
            CallKind::User => self.signatures.get(name).cloned(),
            _ => None,
        };
        let mut operands = vec![];
        let mut writes = vec![];
        for (index, argument) in arguments.iter().enumerate() {
            if outputs
                .as_ref()
                .and_then(|v| v.get(index))
                .copied()
                .unwrap_or(true)
            {
                if let Some(local) = self.location(argument) {
                    writes.push(local);
                    operands.push(self.state[local]);
                }
            } else {
                operands.push(self.expression(argument).value);
            }
        }
        let result = self.emit(ty.clone(), Operation::Unknown(operands));
        for local in writes {
            self.invalidate(local, vec![result]);
        }
        Evaluation {
            value: result,
            pure: false,
            propagated: false,
        }
    }

    fn expression(&mut self, expression: &Expr) -> Evaluation {
        let mut compound = false;
        let mut preserve_right = false;
        let result = match &expression.kind {
            ExprKind::Integer(_) | ExprKind::Bool(_) | ExprKind::Zero => {
                let operation = constant::scalar(&expression.ty)
                    .and_then(|_| literal_expression(expression))
                    .map_or_else(|| Operation::Unknown(vec![]), Operation::Constant);
                Evaluation {
                    value: self.emit(expression.ty.clone(), operation),
                    pure: true,
                    propagated: false,
                }
            }
            ExprKind::Name(name) if self.lookup(name).is_some() => {
                let local = self.lookup(name).unwrap();
                Evaluation {
                    value: self.emit(expression.ty.clone(), Operation::Copy(self.state[local])),
                    pure: true,
                    propagated: true,
                }
            }
            ExprKind::GlobalName(name) if self.globals.contains_key(name) => {
                let bits = self.globals[name].1;
                Evaluation {
                    value: self.emit(expression.ty.clone(), Operation::Constant(bits)),
                    pure: true,
                    propagated: false,
                }
            }
            ExprKind::Cast { value } | ExprKind::DeclarationDefault(value) => {
                let source = self.expression(value);
                Evaluation {
                    value: self.convert(source.value, &expression.ty),
                    ..source
                }
            }
            ExprKind::Unary { op, value } | ExprKind::Postfix { op, value }
                if matches!(op.as_str(), "++" | "--") =>
            {
                let local = self.location(value);
                let old = match local {
                    Some(local) => self.state[local],
                    None => self.emit(value.ty.clone(), Operation::Unknown(vec![])),
                };
                let one = self.emit(value.ty.clone(), Operation::Constant(1));
                let new = self.binary(
                    if op == "++" { "+" } else { "-" },
                    old,
                    one,
                    &value.ty,
                    true,
                );
                if let Some(local) = local {
                    self.state[local] = new;
                }
                Evaluation {
                    value: if matches!(expression.kind, ExprKind::Postfix { .. }) {
                        old
                    } else {
                        new
                    },
                    pure: false,
                    propagated: true,
                }
            }
            ExprKind::Unary { op, value } => {
                let source = self.expression(value);
                Evaluation {
                    value: self.emit(
                        expression.ty.clone(),
                        Operation::Unary {
                            op: op.clone(),
                            value: source.value,
                        },
                    ),
                    ..source
                }
            }
            ExprKind::Binary { op, left, right } if assignment(op) => {
                let local = self.location(left);
                let right = self.expression(right);
                preserve_right = !right.pure;
                let right = right.value;
                let result = if op == "=" {
                    self.convert(right, &left.ty)
                } else {
                    // The inout operand is loaded after argument evaluation,
                    // so a right-hand side writing it changes the left input.
                    let old = match local {
                        Some(local) => self.state[local],
                        None => self.emit(left.ty.clone(), Operation::Unknown(vec![])),
                    };
                    self.binary(op.strip_suffix('=').unwrap(), old, right, &left.ty, true)
                };
                if let Some(local) = local {
                    self.state[local] = result;
                    compound = op != "=";
                }
                Evaluation {
                    value: result,
                    pure: false,
                    propagated: true,
                }
            }
            ExprKind::Binary { op, left, right }
                if matches!(op.as_str(), "&&" | "||") && matches!(left.ty, Type::Scalar(_)) =>
            {
                self.logical(op, left, right, &expression.ty)
            }
            ExprKind::Binary { op, left, right } if op == "," => {
                let left = self.expression(left);
                let right = self.expression(right);
                Evaluation {
                    pure: left.pure && right.pure,
                    propagated: left.propagated || right.propagated,
                    ..right
                }
            }
            ExprKind::Binary { op, left, right } => {
                let a = self.expression(left);
                let b = self.expression(right);
                Evaluation {
                    value: self.binary(op, a.value, b.value, &expression.ty, false),
                    pure: a.pure && b.pure,
                    propagated: a.propagated || b.propagated,
                }
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } if matches!(condition.ty, Type::Scalar(_)) => {
                self.conditional(condition, then_value, else_value, &expression.ty)
            }
            ExprKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let values = [condition.as_ref(), then_value.as_ref(), else_value.as_ref()]
                    .map(|v| self.expression(v).value)
                    .to_vec();
                Evaluation {
                    value: self.emit(expression.ty.clone(), Operation::Unknown(values)),
                    pure: false,
                    propagated: false,
                }
            }
            ExprKind::Call { name, arguments } => {
                self.call(name, arguments, &expression.ty, CallKind::User)
            }
            ExprKind::IntrinsicCall { name, arguments } => {
                self.call(name, arguments, &expression.ty, CallKind::Intrinsic)
            }
            ExprKind::MethodCall {
                receiver,
                name,
                arguments,
            } => {
                self.expression(receiver);
                self.call(name, arguments, &expression.ty, CallKind::Method)
            }
            ExprKind::Construct(values)
                if values.len() == 1 && constant::scalar(&expression.ty).is_some() =>
            {
                let source = self.expression(&values[0]);
                Evaluation {
                    value: self.convert(source.value, &expression.ty),
                    ..source
                }
            }
            ExprKind::Construct(values) => {
                let values = values.iter().map(|v| self.expression(v).value).collect();
                Evaluation {
                    value: self.emit(expression.ty.clone(), Operation::Unknown(values)),
                    pure: false,
                    propagated: false,
                }
            }
            ExprKind::ConstantBufferLoad(value) | ExprKind::Member { base: value, .. } => {
                let source = self.expression(value).value;
                Evaluation {
                    value: self.emit(expression.ty.clone(), Operation::Unknown(vec![source])),
                    pure: true,
                    propagated: false,
                }
            }
            ExprKind::Index { base, index } => {
                let base = self.expression(base);
                let index = self.expression(index);
                Evaluation {
                    value: self.emit(
                        expression.ty.clone(),
                        Operation::Unknown(vec![base.value, index.value]),
                    ),
                    pure: base.pure && index.pure,
                    propagated: false,
                }
            }
            _ => Evaluation {
                value: self.emit(expression.ty.clone(), Operation::Unknown(vec![])),
                pure: true,
                propagated: false,
            },
        };
        let shift = matches!(&expression.kind, ExprKind::Binary { op, .. } if matches!(op.as_str(), "<<" | ">>"));
        if constant::scalar(&expression.ty).is_some()
            && (compound || result.pure && (result.propagated || shift))
        {
            self.sites.insert(
                expression_key(expression),
                Site {
                    value: result.value,
                    rewrite: if compound {
                        Rewrite::Compound { preserve_right }
                    } else {
                        Rewrite::Value
                    },
                },
            );
        }
        result
    }

    fn conditional(&mut self, condition: &Expr, yes: &Expr, no: &Expr, ty: &Type) -> Evaluation {
        let condition = self.expression(condition);
        let yes_block = self.block(None);
        let no_block = self.block(None);
        let after = self.block(Some(ty));
        self.branch(condition.value, yes_block, no_block);
        self.enter(yes_block);
        let a = self.expression(yes);
        let a_value = self.convert(a.value, ty);
        self.jump(after, Some(a_value));
        self.enter(no_block);
        let b = self.expression(no);
        let b_value = self.convert(b.value, ty);
        self.jump(after, Some(b_value));
        self.enter(after);
        Evaluation {
            value: *self.graph.blocks[after.0].parameters.last().unwrap(),
            pure: condition.pure && a.pure && b.pure,
            propagated: condition.propagated || a.propagated || b.propagated,
        }
    }

    fn logical(&mut self, op: &str, left: &Expr, right: &Expr, ty: &Type) -> Evaluation {
        let left = self.expression(left);
        let rhs = self.block(None);
        let short = self.block(None);
        let after = self.block(Some(ty));
        if op == "&&" {
            self.branch(left.value, rhs, short);
        } else {
            self.branch(left.value, short, rhs);
        }
        self.enter(rhs);
        let right = self.expression(right);
        let value = self.convert(right.value, ty);
        self.jump(after, Some(value));
        self.enter(short);
        let value = self.emit(ty.clone(), Operation::Constant(i64::from(op == "||")));
        self.jump(after, Some(value));
        self.enter(after);
        Evaluation {
            value: *self.graph.blocks[after.0].parameters.last().unwrap(),
            pure: left.pure && right.pure,
            propagated: left.propagated || right.propagated,
        }
    }
}

fn assignment(op: &str) -> bool {
    matches!(
        op,
        "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>="
    )
}

fn readonly_intrinsic(name: &str) -> bool {
    matches!(
        name,
        "min"
            | "max"
            | "clamp"
            | "abs"
            | "sign"
            | "countbits"
            | "reversebits"
            | "firstbithigh"
            | "firstbitlow"
            | "asint"
            | "asuint"
            | "asfloat"
            | "sin"
            | "cos"
            | "tan"
            | "sqrt"
            | "rsqrt"
            | "exp"
            | "exp2"
            | "log"
            | "log2"
            | "pow"
            | "floor"
            | "ceil"
            | "round"
            | "trunc"
            | "frac"
            | "saturate"
            | "lerp"
            | "step"
            | "smoothstep"
            | "isnan"
            | "isinf"
            | "isfinite"
            | "mul"
            | "all"
            | "any"
            | "ddx"
            | "ddy"
            | "fwidth"
            | "ddx_coarse"
            | "ddy_coarse"
            | "ddx_fine"
            | "ddy_fine"
            | "fwidth_coarse"
            | "fwidth_fine"
    )
}
