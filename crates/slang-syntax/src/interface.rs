use crate::{parser::Parser, *};
use slang_source::{Diagnostic, Span};

impl Parser {
    pub(crate) fn interface_declaration(
        &mut self,
        start: Span,
        attributes: Vec<Attribute>,
        modifiers: Vec<String>,
    ) -> Result<Vec<Declaration>, Diagnostic> {
        if !attributes.is_empty()
            || modifiers
                .iter()
                .any(|m| !matches!(m.as_str(), "public" | "internal"))
        {
            return Err(self.error("unsupported interface attributes or modifiers"));
        }
        self.expect("interface")?;
        let source_name = self.identifier()?.text;
        let name = self.nested_type_name(source_name, start);
        let generic_parameters = self.generic_parameters()?;
        if self.type_owners.len() >= 32 {
            return Err(self.error("nested type depth exceeds 32"));
        }
        self.type_owners.push(name.clone());
        self.type_parameters
            .push(generic_parameters.iter().map(|p| p.name.clone()).collect());
        let bases = self.inheritance_types()?;
        self.expect("{")?;
        let mut requirements = vec![];
        while !self.at("}") && !self.at_end() {
            let start = self.span();
            let attributes = self.attributes()?;
            let mut modifiers = self.modifiers();
            if !modifiers
                .iter()
                .any(|m| matches!(m.as_str(), "public" | "internal"))
            {
                modifiers.push("public".into());
            }
            if self.eat("__init") {
                let Declaration::Function(function) =
                    self.constructor_member(&name, attributes, modifiers, start)?
                else {
                    unreachable!()
                };
                requirements.push(*function);
                continue;
            }
            let return_type = self.ty()?;
            let member = self.identifier()?;
            let parameters = self.generic_parameters()?;
            self.type_parameters
                .push(parameters.iter().map(|p| p.name.clone()).collect());
            self.expect("(")?;
            let mut arguments = self.parameters()?;
            let is_static = modifiers.iter().any(|m| m == "static");
            if !is_static {
                let mut receiver_modifiers = vec![];
                if attributes.iter().any(|a| a.name == "mutating") {
                    receiver_modifiers.push("inout".into());
                } else {
                    receiver_modifiers.push("const".into());
                }
                arguments.insert(
                    0,
                    Variable {
                        name: "this".into(),
                        ty: TypeRef {
                            name: name.clone(),
                            base: None,
                            application: false,
                            owner_ref: true,
                            parameter_ref: false,
                            arguments: vec![],
                            matrix_major: None,
                            arrays: vec![],
                            span: start,
                        },
                        modifiers: receiver_modifiers,
                        initializer: None,
                        semantic: None,
                        register: None,
                        pack_offset: None,
                        span: start,
                    },
                );
            }
            let semantic = if self.eat(":") {
                Some(self.identifier()?.text)
            } else {
                None
            };
            let body = if self.eat(";") {
                None
            } else {
                Some(self.statement()?)
            };
            if body
                .as_ref()
                .is_some_and(|b| !matches!(b.kind, StmtKind::Block(_)))
            {
                return Err(self.error("interface method body must be a block"));
            }
            self.type_parameters.pop();
            requirements.push(Function {
                name: format!(
                    "{}{}_{}_{}",
                    self.method_prefix,
                    name.len(),
                    name,
                    member.text
                ),
                generic_parameters: parameters,
                constructor: None,
                method: Some(StructMethod {
                    owner: name.clone(),
                    name: member.text,
                    is_static,
                }),
                return_type,
                parameters: arguments,
                attributes,
                modifiers,
                semantic,
                span: start.join(body.as_ref().map_or(start, |b| b.span)),
                body,
            });
        }
        let end = self.expect("}")?;
        self.type_owners.pop();
        self.type_parameters.pop();
        self.eat(";");
        Ok(vec![Declaration::Interface(Interface {
            name,
            generic_parameters,
            bases,
            requirements,
            modifiers,
            span: start.join(end),
        })])
    }
}
