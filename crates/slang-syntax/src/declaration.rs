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
        if self.at("typedef") || self.at("typealias") {
            if !attributes.is_empty()
                || modifiers
                    .iter()
                    .any(|m| !matches!(m.as_str(), "public" | "internal"))
            {
                return Err(self.error("unsupported type alias attributes or modifiers"));
            }
            let (name, mut target) = if self.eat("typealias") {
                let name = self.identifier()?.text;
                self.expect("=")?;
                (name, self.ty()?)
            } else {
                self.expect("typedef")?;
                let target = self.ty()?;
                let name = self.identifier()?.text;
                (name, target)
            };
            let trailing = self.array_sizes()?;
            target.arrays.splice(0..0, trailing);
            let end = self.expect(";")?;
            return Ok(vec![Declaration::TypeAlias(TypeAlias {
                name,
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
            let name = self.identifier()?.text;
            self.expect("{")?;
            let mut fields = vec![];
            while !self.at("}") && !self.at_end() {
                let modifiers = self.modifiers();
                let ty = self.ty()?;
                let name = self.identifier()?;
                fields.extend(self.variables(ty, name, modifiers)?);
                self.expect(";")?;
            }
            let end = self.expect("}")?;
            self.expect(";")?;
            return Ok(vec![Declaration::Struct(Struct {
                name,
                modifiers,
                fields,
                span: start.join(end),
            })]);
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
                    | "cbuffer"
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
            let semantic = self.semantic()?;
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
                return_type: ty,
                parameters,
                attributes,
                modifiers,
                semantic,
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

    pub(crate) fn ty(&mut self) -> Result<TypeRef, Diagnostic> {
        self.enter()?;
        let result = self.ty_inner();
        self.depth -= 1;
        result
    }

    fn ty_inner(&mut self) -> Result<TypeRef, Diagnostic> {
        let token = self.identifier()?;
        let mut arguments = vec![];
        if self.eat("<") {
            while !self.at(">") && !self.at(">>") {
                if self.current().is_some_and(|t| t.kind == TokenKind::Integer) {
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
        }
        let mut arrays = self.array_sizes()?;
        arrays.reverse();
        Ok(TypeRef {
            name: token.text,
            arguments,
            arrays,
            span: token.span.join(self.tokens[self.offset - 1].span),
        })
    }

    pub(crate) fn variable(
        &mut self,
        mut ty: TypeRef,
        name: Token,
        modifiers: Vec<String>,
    ) -> Result<Variable, Diagnostic> {
        let arrays = self.array_sizes()?;
        ty.arrays.splice(0..0, arrays);
        let semantic = self.semantic()?;
        let initializer = if self.eat("=") {
            Some(self.expression(1)?)
        } else {
            None
        };
        let span = ty
            .span
            .join(initializer.as_ref().map_or(name.span, |e| e.span));
        Ok(Variable {
            name: name.text,
            ty,
            modifiers,
            semantic,
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

    pub(crate) fn array_sizes(&mut self) -> Result<Vec<u32>, Diagnostic> {
        let mut arrays = vec![];
        while self.eat("[") {
            if arrays.len() >= 32 {
                return Err(self.error("array nesting limit exceeded"));
            }
            let token = self
                .current()
                .filter(|t| t.kind == TokenKind::Integer)
                .ok_or_else(|| self.error("array size must be a positive integer literal"))?
                .clone();
            self.offset += 1;
            let size: u32 = token
                .text
                .trim_end_matches(['u', 'U'])
                .parse()
                .map_err(|_| self.error("invalid array size"))?;
            if size == 0 {
                return Err(self.error("array size must be positive"));
            }
            arrays.push(size);
            self.expect("]")?;
        }
        Ok(arrays)
    }

    fn semantic(&mut self) -> Result<Option<String>, Diagnostic> {
        if self.eat(":") {
            Ok(Some(self.identifier()?.text))
        } else {
            Ok(None)
        }
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
