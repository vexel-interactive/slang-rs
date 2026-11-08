use crate::{parser::Parser, *};
use slang_source::{Diagnostic, Span};

impl Parser {
    /// Normalize anonymous buffer fields into lexical global bindings and a hidden handle.
    pub(crate) fn constant_buffer(
        &mut self,
        start: Span,
        modifiers: Vec<String>,
    ) -> Result<Vec<Declaration>, Diagnostic> {
        let name = self.identifier()?;
        let annotations = self.annotations()?;
        if annotations.semantic.is_some() || annotations.pack_offset.is_some() {
            return Err(self
                .error("cbuffer declarations require register annotations rather than semantics"));
        }
        self.expect("{")?;
        let mut members = vec![];
        let mut fields = vec![];
        let mut globals = vec![];
        while !self.at("}") && !self.at_end() {
            let field_modifiers = self.modifiers();
            let ty = self.ty()?;
            let field_name = self.identifier()?;
            for mut field in self.variables(ty, field_name, field_modifiers)? {
                let static_storage = field.modifiers.iter().any(|m| m == "static");
                members.push(ConstantBufferMember {
                    name: field.name.clone(),
                    variable_name: field.name.clone(),
                    static_storage,
                    pack_offset: field.pack_offset,
                    span: field.span,
                });
                if !static_storage {
                    let mut data_field = field.clone();
                    data_field
                        .modifiers
                        .retain(|m| !matches!(m.as_str(), "const" | "uniform"));
                    data_field.register = None;
                    fields.push(data_field);
                    field.initializer = None;
                    field.modifiers.retain(|m| m != "const");
                }
                field.pack_offset = None;
                field.register = None;
                globals.push(Declaration::Global(field));
            }
            self.expect(";")?;
        }
        let end = self.expect("}")?;
        self.eat(";");
        let span = start.join(end);
        let stem = format!(
            "{}{}_{}",
            self.buffer_prefix,
            span.source.0,
            self.constant_buffers.len()
        );
        let element_type = format!("{stem}_type");
        let variable_name = format!("{stem}_handle");
        self.constant_buffers.push(ConstantBufferInfo {
            name: name.text,
            variable_name: variable_name.clone(),
            element_type: element_type.clone(),
            members,
            span,
        });
        let mut declarations = vec![
            Declaration::Struct(Struct {
                name: element_type.clone(),
                modifiers: modifiers.clone(),
                fields,
                span,
            }),
            Declaration::Global(Variable {
                name: variable_name,
                ty: TypeRef {
                    name: "ConstantBuffer".into(),
                    matrix_major: None,
                    arguments: vec![TypeArgument::Type(TypeRef {
                        name: element_type,
                        matrix_major: None,
                        arguments: vec![],
                        arrays: vec![],
                        span,
                    })],
                    arrays: vec![],
                    span,
                },
                modifiers,
                semantic: None,
                register: annotations.register,
                pack_offset: None,
                initializer: None,
                span,
            }),
        ];
        declarations.append(&mut globals);
        Ok(declarations)
    }
}
