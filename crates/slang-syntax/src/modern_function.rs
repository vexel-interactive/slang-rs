use crate::{
    Attribute, Function, GenericParameter, StmtKind, StructMethod, TypeRef, Variable,
    parser::Parser,
};
use slang_lexer::TokenKind;
use slang_source::{Diagnostic, Span};

impl Parser {
    pub(crate) fn modern_function(
        &mut self,
        owner: Option<&str>,
        attributes: Vec<Attribute>,
        modifiers: Vec<String>,
        start: Span,
    ) -> Result<Function, Diagnostic> {
        self.expect("func")?;
        let name = self.identifier()?;
        let mut generic_parameters = self.generic_parameters()?;
        self.type_parameters
            .push(generic_parameters.iter().map(|parameter| parameter.name.clone()).collect());
        self.expect("(")?;
        let mut parameters = self.modern_parameters()?;
        let return_span = self.tokens[self.offset - 1].span;

        if self.at("const") {
            return Err(
                self.error("trailing receiver const requires the unimplemented HLSL dialect")
            );
        }

        if self.at("throws") {
            return Err(self.error("throwing functions are not implemented"));
        }

        let return_type = if self.eat("->") {
            self.ty()?
        } else {
            let mut ty = TypeRef::inferred(return_span);
            ty.name = "void".into();
            ty
        };
        self.modern_where_constraints(&mut generic_parameters)?;

        if self.at(":") {
            return Err(self.error("func results do not support colon semantics"));
        }

        let is_static = modifiers.iter().any(|modifier| modifier == "static");

        if let Some(owner) = owner
            && !is_static
        {
            let mut ty = TypeRef::inferred(name.span);
            ty.name = owner.into();
            ty.owner_ref = true;
            parameters.insert(
                0,
                Variable {
                    name: "this".into(),
                    ty,
                    modifiers: vec![if attributes
                        .iter()
                        .any(|attribute| attribute.name == "mutating")
                    {
                        "inout".into()
                    } else {
                        "const".into()
                    }],
                    semantic: None,
                    register: None,
                    pack_offset: None,
                    initializer: None,
                    span: name.span,
                },
            );
        }

        let body = if self.eat(";") {
            None
        } else {
            Some(self.statement()?)
        };

        if body.as_ref().is_some_and(|statement| !matches!(statement.kind, StmtKind::Block(_))) {
            return Err(self.error("function body must be a block"));
        }

        self.type_parameters.pop();
        let method = owner.map(|owner| StructMethod {
            owner: owner.into(),
            name: name.text.clone(),
            is_static,
        });
        let function_name = owner.map_or_else(
            || name.text.clone(),
            |owner| format!("{}{}_{}_{}", self.method_prefix, owner.len(), owner, name.text),
        );
        let end = self.tokens[self.offset - 1].span;

        Ok(Function {
            name: function_name,
            generic_parameters,
            constructor: None,
            method,
            return_type,
            parameters,
            attributes,
            modifiers,
            semantic: None,
            body,
            span: start.join(end),
        })
    }

    fn modern_parameters(&mut self) -> Result<Vec<Variable>, Diagnostic> {
        let mut parameters = vec![];

        while !self.at(")") {
            let start = self.span();

            if !self.attributes()?.is_empty() {
                return Err(self.error("parameter attributes are not implemented"));
            }

            let modifiers = self.modifiers();
            let named = self.current().is_some_and(|token| token.kind == TokenKind::Identifier)
                && self.tokens.get(self.offset + 1).is_some_and(|token| {
                    matches!(token.text.as_str(), ":" | "," | ")" | "}" | "]" | "{")
                });
            let parameter = if named {
                self.modern_binding(modifiers, start)?
            } else {
                let ty = self.ty()?;
                let name = self.identifier()?;
                self.variable(ty, name, modifiers)?
            };
            parameters.push(parameter);

            if !self.eat(",") {
                break;
            }
        }

        self.expect(")")?;
        Ok(parameters)
    }

    /// Express direct `where T : I, J` bounds in the existing parameter model.
    fn modern_where_constraints(
        &mut self,
        parameters: &mut [GenericParameter],
    ) -> Result<(), Diagnostic> {
        while self.eat("where") {
            let name = self.identifier()?;
            let Some(parameter) =
                parameters.iter_mut().find(|parameter| parameter.name == name.text)
            else {
                return Err(Diagnostic::error(
                    "S001",
                    name.span,
                    "where clauses require a type parameter declared by the function",
                ));
            };
            if !self.eat(":") {
                return Err(self.error(
                    "only direct parameter interface bounds are implemented in where clauses",
                ));
            }

            loop {
                parameter.constraints.push(self.ty()?);

                if !(self.eat(",") || self.eat("&")) {
                    break;
                }
            }
        }

        Ok(())
    }
}
