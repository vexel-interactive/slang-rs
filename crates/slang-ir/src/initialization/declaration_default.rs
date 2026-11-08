use super::*;

impl Lowerer {
    pub(super) fn declaration_default(&mut self, expression: &mut Expr) -> bool {
        let ExprKind::DeclarationDefault(value) = &expression.kind else {
            return false;
        };
        let name = format!("{}default_{}", self.prefix, self.functions.len());
        let array = matches!(value.ty, Type::Array(..));
        let mut statements = vec![];
        let (return_type, result) = if array {
            let wrapper = Struct {
                name: format!("{}array_{}", self.prefix, self.generated_structures.len()),
                fields: vec![Variable {
                    name: "_value".into(),
                    ty: value.ty.clone(),
                    modifiers: vec![],
                    semantic: None,
                    binding: None,
                    uniform_layout: None,
                    pack_offset: None,
                    initializer: None,
                    span: value.span,
                }],
            };
            self.generated_structures.push(wrapper.clone());
            let ty = Type::Struct(wrapper.name);
            let result = Expr {
                ty: ty.clone(),
                span: value.span,
                kind: ExprKind::Name(format!("{}default_result", self.prefix)),
            };
            let ExprKind::Name(local) = &result.kind else {
                unreachable!()
            };
            statements.push(Stmt {
                span: value.span,
                kind: StmtKind::Variable(Variable {
                    name: local.clone(),
                    ty: ty.clone(),
                    modifiers: vec![],
                    semantic: None,
                    binding: None,
                    uniform_layout: None,
                    pack_offset: None,
                    initializer: Some(Expr {
                        ty: ty.clone(),
                        span: value.span,
                        kind: ExprKind::Zero,
                    }),
                    span: value.span,
                }),
            });
            statements.push(Stmt {
                span: value.span,
                kind: StmtKind::Expression(Expr {
                    ty: value.ty.clone(),
                    span: value.span,
                    kind: ExprKind::Binary {
                        op: "=".into(),
                        left: Box::new(Expr {
                            ty: value.ty.clone(),
                            span: value.span,
                            kind: ExprKind::Member {
                                base: Box::new(result.clone()),
                                name: "_value".into(),
                            },
                        }),
                        right: value.clone(),
                    },
                }),
            });
            (ty, result)
        } else {
            (value.ty.clone(), *value.clone())
        };
        statements.push(Stmt {
            span: value.span,
            kind: StmtKind::Return(Some(result)),
        });
        self.functions.push(Function {
            name: name.clone(),
            return_type: return_type.clone(),
            parameters: vec![],
            modifiers: vec![],
            semantic: None,
            body: Some(Stmt {
                span: value.span,
                kind: StmtKind::Block(statements),
            }),
            span: value.span,
        });
        let call = Expr {
            ty: return_type,
            span: expression.span,
            kind: ExprKind::Call {
                name,
                arguments: vec![],
            },
        };
        expression.kind = if array {
            ExprKind::Member {
                base: Box::new(call),
                name: "_value".into(),
            }
        } else {
            call.kind
        };
        true
    }
}
