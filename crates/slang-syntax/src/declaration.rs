use crate::parser::Parser;
use crate::*;
use slang_lexer::{Token, TokenKind};
use slang_source::Diagnostic;

impl Parser {
    pub(crate) fn modifiers(&mut self) -> Vec<String> {
        let mut result = vec![];
        while self.current().is_some_and(|t| {
            matches!(
                t.text.as_str(),
                "const"
                    | "static"
                    | "uniform"
                    | "groupshared"
                    | "in"
                    | "out"
                    | "inout"
                    | "public"
                    | "internal"
                    | "export"
                    | "extern"
                    | "inline"
                    | "precise"
                    | "nointerpolation"
                    | "row_major"
                    | "column_major"
            )
        }) {
            result.push(self.current().unwrap().text.clone());
            self.offset += 1;
        }
        result
    }

    pub(crate) fn declaration(&mut self) -> Result<Vec<Declaration>, Diagnostic> {
        let start = self.span();
        let attributes = self.attributes()?;
        let modifiers = self.modifiers();
        if self.eat("cbuffer") {
            if !attributes.is_empty()
                || modifiers
                    .iter()
                    .any(|m| !matches!(m.as_str(), "public" | "internal"))
            {
                return Err(self.error("unsupported cbuffer attributes or modifiers"));
            }
            return self.constant_buffer(start, modifiers);
        }
        if self.at("typedef") || self.at("typealias") {
            if !attributes.is_empty()
                || modifiers
                    .iter()
                    .any(|m| !matches!(m.as_str(), "public" | "internal"))
            {
                return Err(self.error("unsupported type alias attributes or modifiers"));
            }
            let (name, generic_parameters, mut target) = if self.eat("typealias") {
                let name = self.identifier()?.text;
                let parameters = self.generic_parameters()?;
                self.expect("=")?;
                (name, parameters, self.ty()?)
            } else {
                self.expect("typedef")?;
                let target = self.ty()?;
                let name = self.identifier()?.text;
                (name, vec![], target)
            };
            let name = self.nested_type_name(name, start);
            let trailing = self.array_sizes()?;
            target.arrays.splice(0..0, trailing);
            let end = self.expect(";")?;
            return Ok(vec![Declaration::TypeAlias(TypeAlias {
                name,
                generic_parameters,
                target,
                modifiers,
                span: start.join(end),
            })]);
        }
        if self.eat("struct") {
            if !attributes.is_empty()
                || modifiers
                    .iter()
                    .any(|m| !matches!(m.as_str(), "public" | "internal"))
            {
                return Err(self.error("unsupported struct attributes or modifiers"));
            }
            let source_name = self.identifier()?.text;
            let name = self.nested_type_name(source_name, start);
            let generic_parameters = self.generic_parameters()?;
            if self.type_owners.len() >= 32 {
                return Err(self.error("nested type depth exceeds 32"));
            }
            self.type_owners.push(name.clone());
            self.type_parameters
                .push(generic_parameters.iter().map(|p| p.name.clone()).collect());
            self.expect("{")?;
            let mut fields = vec![];
            let mut members = vec![];
            while !self.at("}") && !self.at_end() {
                let member_offset = self.offset;
                let member_start = self.span();
                let attributes = self.attributes()?;
                let member_modifiers = self.modifiers();
                if self.at("struct") || self.at("typedef") || self.at("typealias") {
                    self.offset = member_offset;
                    members.extend(self.declaration()?);
                    continue;
                }
                if self.eat("__init") {
                    self.expect("(")?;
                    let parameters = self.parameters()?;
                    let body = if self.eat(";") {
                        None
                    } else {
                        Some(self.statement()?)
                    };
                    if body
                        .as_ref()
                        .is_some_and(|s| !matches!(s.kind, StmtKind::Block(_)))
                    {
                        return Err(self.error("constructor body must be a block"));
                    }
                    members.push(Declaration::Function(Box::new(Function {
                        name: format!("{}{}", self.constructor_prefix, name),
                        constructor: Some(name.clone()),
                        method: None,
                        return_type: TypeRef {
                            name: name.clone(),
                            base: None,
                            application: false,
                            owner_ref: true,
                            parameter_ref: false,
                            arguments: vec![],
                            matrix_major: None,
                            arrays: vec![],
                            span: member_start,
                        },
                        parameters,
                        attributes,
                        modifiers: member_modifiers,
                        semantic: None,
                        span: member_start.join(body.as_ref().map_or(member_start, |s| s.span)),
                        body,
                    })));
                    continue;
                }
                let ty = self.ty()?;
                let member = self.identifier()?;
                if self.eat("(") {
                    let mut parameters = self.parameters()?;
                    let is_static = member_modifiers.iter().any(|m| m == "static");
                    if !is_static {
                        parameters.insert(
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
                                    span: member.span,
                                },
                                modifiers: vec![
                                    if attributes.iter().any(|a| a.name == "mutating") {
                                        "inout".into()
                                    } else {
                                        "const".into()
                                    },
                                ],
                                semantic: None,
                                register: None,
                                pack_offset: None,
                                initializer: None,
                                span: member.span,
                            },
                        );
                    }
                    let annotations = self.annotations()?;
                    if annotations.register.is_some() || annotations.pack_offset.is_some() {
                        return Err(self.error("methods cannot have storage annotations"));
                    }
                    let body = if self.eat(";") {
                        None
                    } else {
                        Some(self.statement()?)
                    };
                    if body
                        .as_ref()
                        .is_some_and(|s| !matches!(s.kind, StmtKind::Block(_)))
                    {
                        return Err(self.error("method body must be a block"));
                    }
                    members.push(Declaration::Function(Box::new(Function {
                        name: format!(
                            "{}{}_{}_{}",
                            self.method_prefix,
                            name.len(),
                            name,
                            member.text
                        ),
                        constructor: None,
                        method: Some(StructMethod {
                            owner: name.clone(),
                            name: member.text,
                            is_static,
                        }),
                        return_type: ty,
                        parameters,
                        attributes,
                        modifiers: member_modifiers,
                        semantic: annotations.semantic,
                        span: member_start.join(body.as_ref().map_or(member.span, |s| s.span)),
                        body,
                    })));
                    continue;
                }
                if !attributes.is_empty() {
                    return Err(self.error("struct field attributes are not implemented"));
                }
                let is_static = member_modifiers.iter().any(|m| m == "static");
                let variables = self.variables(ty, member, member_modifiers)?;
                if is_static {
                    for mut variable in variables {
                        let member_name = variable.name.clone();
                        variable.name = format!(
                            "{}{}_{}_{}",
                            self.member_prefix,
                            name.len(),
                            name,
                            member_name
                        );
                        self.static_members.push(StaticMemberInfo {
                            owner: name.clone(),
                            name: member_name,
                            variable_name: variable.name.clone(),
                            span: variable.span,
                        });
                        members.push(Declaration::Global(variable));
                    }
                } else {
                    fields.extend(variables);
                }
                self.expect(";")?;
            }
            let end = self.expect("}")?;
            self.type_owners.pop();
            self.type_parameters.pop();
            self.eat(";");
            let mut declarations = vec![Declaration::Struct(Struct {
                name,
                generic_parameters,
                modifiers,
                fields,
                span: start.join(end),
            })];
            declarations.extend(members);
            return Ok(declarations);
        }
        if self.current().is_some_and(|t| {
            matches!(
                t.text.as_str(),
                "import"
                    | "module"
                    | "interface"
                    | "extension"
                    | "typedef"
                    | "typealias"
                    | "namespace"
                    | "__generic"
                    | "let"
                    | "var"
            )
        }) {
            return Err(self.error(format!(
                "declaration '{}' is not implemented",
                self.current().unwrap().text
            )));
        }
        let ty = self.ty()?;
        let name = self.identifier()?;
        if self.eat("(") {
            let parameters = self.parameters()?;
            let annotations = self.annotations()?;
            if let Some(pack) = annotations.pack_offset {
                return Err(Diagnostic::error(
                    "S001",
                    pack.span,
                    "packoffset is not allowed on function results",
                ));
            }
            let body = if self.eat(";") {
                None
            } else {
                Some(self.statement()?)
            };
            if body
                .as_ref()
                .is_some_and(|s| !matches!(s.kind, StmtKind::Block(_)))
            {
                return Err(self.error("function body must be a block"));
            }
            let span = start.join(body.as_ref().map_or(name.span, |s| s.span));
            Ok(vec![Declaration::Function(Box::new(Function {
                name: name.text,
                constructor: None,
                method: None,
                return_type: ty,
                parameters,
                attributes,
                modifiers,
                semantic: annotations.semantic,
                body,
                span,
            }))])
        } else {
            if !attributes.is_empty() {
                return Err(self.error("global variable attributes are not implemented"));
            }
            let variables = self.variables(ty, name, modifiers)?;
            self.expect(";")?;
            Ok(variables.into_iter().map(Declaration::Global).collect())
        }
    }

    fn parameters(&mut self) -> Result<Vec<Variable>, Diagnostic> {
        let mut parameters = vec![];
        if !(self.at(")")
            || self.at("void") && self.tokens.get(self.offset + 1).is_some_and(|t| t.is(")")))
        {
            loop {
                let modifiers = self.modifiers();
                let ty = self.ty()?;
                let name = self.identifier()?;
                parameters.push(self.variable(ty, name, modifiers)?);
                if !self.eat(",") {
                    break;
                }
            }
        } else {
            self.eat("void");
        }
        self.expect(")")?;
        Ok(parameters)
    }

    fn nested_type_name(&mut self, name: String, span: slang_source::Span) -> String {
        let Some(owner) = self.type_owners.last() else {
            return name;
        };
        let target = format!("{}{}_{}_{}", self.type_prefix, owner.len(), owner, name);
        self.nested_types.push(NestedTypeInfo {
            owner: owner.clone(),
            name,
            type_name: target.clone(),
            span,
        });
        target
    }

    fn generic_parameters(&mut self) -> Result<Vec<GenericParameter>, Diagnostic> {
        if !self.eat("<") {
            return Ok(vec![]);
        }
        let mut parameters = vec![];
        loop {
            if self.at("let") {
                return Err(self.error("generic value parameters are not implemented"));
            }
            let token = self.identifier()?;
            if self.at(":") {
                return Err(self.error("generic interface constraints are not implemented"));
            }
            let default = if self.eat("=") {
                Some(self.ty()?)
            } else {
                None
            };
            parameters.push(GenericParameter {
                name: token.text,
                default,
                span: token.span,
            });
            if !self.eat(",") {
                break;
            }
        }
        self.close_type_arguments()?;
        Ok(parameters)
    }

    pub(crate) fn ty(&mut self) -> Result<TypeRef, Diagnostic> {
        self.enter()?;
        let result = self.ty_inner(true);
        self.depth -= 1;
        result
    }

    pub(crate) fn expression_type(&mut self) -> Result<TypeRef, Diagnostic> {
        self.enter()?;
        let result = self.ty_inner(false);
        self.depth -= 1;
        result
    }

    fn ty_inner(&mut self, qualified: bool) -> Result<TypeRef, Diagnostic> {
        let start = self.span();
        let mut matrix_major = None;
        while self.at("row_major") || self.at("column_major") {
            matrix_major = Some(if self.eat("row_major") {
                slang_layout::MatrixMajor::Row
            } else {
                self.expect("column_major")?;
                slang_layout::MatrixMajor::Column
            });
        }
        let mut token = self.identifier()?;
        while qualified && (self.at(".") || self.at("::")) {
            self.offset += 1;
            token.text.push_str("::");
            token.text.push_str(&self.identifier()?.text);
        }
        let application = self.at("<");
        let arguments = self.type_arguments(&token.text)?;
        let mut reference = TypeRef {
            name: token.text,
            base: None,
            application,
            owner_ref: false,
            parameter_ref: false,
            arguments,
            matrix_major,
            arrays: vec![],
            span: start.join(self.tokens[self.offset - 1].span),
        };
        while qualified && (self.at(".") || self.at("::")) {
            self.offset += 1;
            let token = self.identifier()?;
            let application = self.at("<");
            let arguments = self.type_arguments(&token.text)?;
            reference = TypeRef {
                name: token.text,
                base: Some(Box::new(reference)),
                application,
                owner_ref: false,
                parameter_ref: false,
                arguments,
                matrix_major: None,
                arrays: vec![],
                span: start.join(self.tokens[self.offset - 1].span),
            };
        }
        let mut arrays = self.array_sizes()?;
        arrays.reverse();
        reference.arrays = arrays;
        reference.span = start.join(self.tokens[self.offset - 1].span);
        Ok(reference)
    }

    pub(crate) fn type_arguments(&mut self, name: &str) -> Result<Vec<TypeArgument>, Diagnostic> {
        let mut arguments = vec![];
        if self.eat("<") {
            while !self.at(">") && !self.at(">>") {
                if matches!(name, "vector" | "matrix") && !arguments.is_empty() {
                    let previous = self.expression_angle_stop;
                    self.expression_angle_stop = true;
                    let expression = self.expression(1);
                    self.expression_angle_stop = previous;
                    arguments.push(TypeArgument::Expression(Box::new(expression?)));
                } else if self.current().is_some_and(|t| t.kind == TokenKind::Integer) {
                    let token = self.current().unwrap().clone();
                    self.offset += 1;
                    let text = token.text.trim_end_matches(['u', 'U']);
                    let value = if let Some(hex) =
                        text.strip_prefix("0x").or_else(|| text.strip_prefix("0X"))
                    {
                        u32::from_str_radix(hex, 16)
                    } else if let Some(binary) =
                        text.strip_prefix("0b").or_else(|| text.strip_prefix("0B"))
                    {
                        u32::from_str_radix(binary, 2)
                    } else {
                        text.parse()
                    }
                    .map_err(|_| self.error("invalid integer type argument"))?;
                    arguments.push(TypeArgument::Integer {
                        value,
                        span: token.span,
                    });
                } else {
                    arguments.push(TypeArgument::Type(self.ty()?));
                }
                if !self.eat(",") {
                    break;
                }
            }
            self.close_type_arguments()?;
        }
        Ok(arguments)
    }

    fn close_type_arguments(&mut self) -> Result<(), Diagnostic> {
        if self.at(">>") {
            let token = self.current().unwrap().clone();
            self.tokens[self.offset].text = ">".into();
            self.tokens.insert(
                self.offset + 1,
                Token {
                    text: ">".into(),
                    ..token
                },
            );
        }
        self.expect(">")?;
        Ok(())
    }

    pub(crate) fn variable(
        &mut self,
        mut ty: TypeRef,
        name: Token,
        modifiers: Vec<String>,
    ) -> Result<Variable, Diagnostic> {
        let arrays = self.array_sizes()?;
        ty.arrays.splice(0..0, arrays);
        let annotations = self.annotations()?;
        let initializer = if self.eat("=") {
            Some(self.expression(1)?)
        } else {
            None
        };
        let span = ty.span.join(self.tokens[self.offset - 1].span);
        Ok(Variable {
            name: name.text,
            ty,
            modifiers,
            semantic: annotations.semantic,
            register: annotations.register,
            pack_offset: annotations.pack_offset,
            initializer,
            span,
        })
    }

    /// Every declarator shares the base type, but owns its suffix and initializer.
    pub(crate) fn variables(
        &mut self,
        ty: TypeRef,
        mut name: Token,
        modifiers: Vec<String>,
    ) -> Result<Vec<Variable>, Diagnostic> {
        let mut variables = vec![];
        loop {
            variables.push(self.variable(ty.clone(), name, modifiers.clone())?);
            if !self.eat(",") {
                return Ok(variables);
            }
            name = self.identifier()?;
        }
    }

    pub(crate) fn array_sizes(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        let mut arrays = vec![];
        while self.eat("[") {
            if arrays.len() >= 32 {
                return Err(self.error("array nesting limit exceeded"));
            }
            arrays.push(self.expression(1)?);
            self.expect("]")?;
        }
        Ok(arrays)
    }

    fn attributes(&mut self) -> Result<Vec<Attribute>, Diagnostic> {
        let mut attributes = vec![];
        while self.eat("[") {
            let name = self.identifier()?;
            let arguments = if self.eat("(") {
                self.arguments(")")?
            } else {
                vec![]
            };
            let end = self.expect("]")?;
            attributes.push(Attribute {
                name: name.text,
                arguments,
                span: name.span.join(end),
            });
        }
        Ok(attributes)
    }

    pub(crate) fn arguments(&mut self, close: &str) -> Result<Vec<Expr>, Diagnostic> {
        let mut arguments = vec![];
        if !self.at(close) {
            loop {
                arguments.push(self.expression(1)?);
                if !self.eat(",") {
                    break;
                }
            }
        }
        self.expect(close)?;
        Ok(arguments)
    }
}
