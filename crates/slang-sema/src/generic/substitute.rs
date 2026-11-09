use super::*;

pub(super) fn replace_type(ty: &mut TypeRef, types: &BTreeMap<String, TypeRef>) {
    let (first, member) = ty
        .name
        .split_once("::")
        .map_or((ty.name.as_str(), None), |(a, b)| (a, Some(b)));
    if ty.base.is_none()
        && let Some(argument) = types.get(first)
    {
        let mut replacement = argument.clone();
        if let Some(member) = member {
            replacement = TypeRef {
                name: member.into(),
                base: Some(Box::new(replacement)),
                application: ty.application,
                owner_ref: false,
                arguments: ty.arguments.clone(),
                matrix_major: ty.matrix_major,
                arrays: vec![],
                span: ty.span,
            };
        }
        replacement.arrays.splice(0..0, ty.arrays.clone());
        replacement.matrix_major = ty.matrix_major.or(replacement.matrix_major);
        replacement.span = ty.span;
        *ty = replacement;
    }
}

pub(super) struct Substitute<'a> {
    pub root: &'a str,
    pub names: &'a BTreeMap<String, String>,
    pub types: &'a BTreeMap<String, TypeRef>,
    pub context: usize,
    pub next_expression: &'a mut usize,
    pub remaining_nodes: &'a mut usize,
}

impl Substitute<'_> {
    fn node(&mut self, span: Span) -> Result<(), Diagnostic> {
        *self.remaining_nodes = self
            .remaining_nodes
            .checked_sub(1)
            .ok_or_else(|| error(span, "generic syntax expansion budget exceeded"))?;
        Ok(())
    }
    fn name(&self, name: &mut String) {
        if let Some(target) = self.names.get(name) {
            *name = target.clone();
        }
    }
}

impl VisitorMut for Substitute<'_> {
    fn span(&mut self, span: &mut Span) {
        *span = span.with_context(self.context);
    }
    fn expression_id(&mut self, id: &mut ExprId) {
        *id = ExprId(*self.next_expression);
        *self.next_expression += 1;
    }
    fn ty(&mut self, ty: &mut TypeRef) -> Result<(), Diagnostic> {
        self.node(ty.span)?;
        replace_type(ty, self.types);
        if ty.owner_ref || ty.name != self.root {
            self.name(&mut ty.name);
        }
        ty.owner_ref = false;
        visit::walk_type(self, ty)
    }
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        self.node(expression.span)?;
        if let ExprKind::Name(name) = &mut expression.kind {
            if let Some(argument) = self.types.get(name) {
                expression.kind = ExprKind::TypeApplication(Box::new(argument.clone()));
            } else if name != self.root {
                self.name(name);
            }
        }
        visit::walk_expression(self, expression)
    }
    fn declaration(&mut self, declaration: &mut Declaration) -> Result<(), Diagnostic> {
        self.node(declaration_span(declaration))?;
        match declaration {
            Declaration::Struct(s) => self.name(&mut s.name),
            Declaration::TypeAlias(a) => self.name(&mut a.name),
            Declaration::Global(v) => self.name(&mut v.name),
            Declaration::Function(f) => {
                self.name(&mut f.name);
                if let Some(owner) = &mut f.constructor {
                    self.name(owner);
                }
                if let Some(method) = &mut f.method {
                    self.name(&mut method.owner);
                }
            }
        }
        visit::walk_declaration(self, declaration)
    }
}
