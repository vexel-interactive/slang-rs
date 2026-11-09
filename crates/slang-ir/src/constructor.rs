use crate::lower::Lowerer;
use crate::visit::{Visitor, walk_statement};
use crate::*;
use slang_sema::{Type, span_key};
use slang_source::Diagnostic;

impl Lowerer<'_> {
    pub(crate) fn function_body(
        &self,
        function: &slang_syntax::Function,
        body: &slang_syntax::Stmt,
    ) -> Result<Stmt, Diagnostic> {
        let mut body = self.statement(body)?;
        let Some(owner) = &function.constructor else {
            return Ok(body);
        };
        let ty = Type::Struct(owner.clone());
        let receiver = Expr {
            ty: ty.clone(),
            kind: ExprKind::Name("this".into()),
            span: function.span,
        };
        let mut returns = Returns {
            receiver: receiver.clone(),
        };
        returns.statement(&mut body)?;
        let mut statements = vec![Stmt {
            kind: StmtKind::Variable(Variable {
                name: "this".into(),
                ty,
                modifiers: vec![],
                semantic: None,
                binding: None,
                uniform_layout: None,
                pack_offset: None,
                initializer: None,
                span: function.span,
            }),
            span: function.span,
        }];
        if let Some(values) = self
            .checked
            .constructor_initializations
            .get(&span_key(function.span))
        {
            for (name, value) in values {
                let location = Expr {
                    ty: value.ty.clone(),
                    kind: ExprKind::Member {
                        base: Box::new(receiver.clone()),
                        name: name.clone(),
                    },
                    span: function.span,
                };
                statements.push(crate::effects::expression(crate::effects::assign(
                    location,
                    self.initialization(value, function.span)?,
                )));
            }
        }
        statements.push(body);
        statements.push(Stmt {
            kind: StmtKind::Return(Some(receiver)),
            span: function.span,
        });
        Ok(crate::effects::block(statements, function.span))
    }
}

struct Returns {
    receiver: Expr,
}
impl Visitor for Returns {
    fn expression(&mut self, _: &mut Expr) -> Result<(), Diagnostic> {
        Ok(())
    }
    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        if let StmtKind::Return(None) = statement.kind {
            statement.kind = StmtKind::Return(Some(self.receiver.clone()));
        }
        walk_statement(self, statement)
    }
}
