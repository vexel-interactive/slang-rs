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
            )
        }) {
            result.push(self.current().unwrap().text.clone());
            self.offset += 1;
        }
        result
    }

    pub(crate) fn declaration(&mut self) -> Result<Declaration, Diagnostic> {
        let start = self.span();
        let attributes = self.attributes()?;
        let modifiers = self.modifiers();
        if self.eat("struct") {
            if !attributes.is_empty() || !modifiers.is_empty() {
                return Err(self.error("struct attributes and modifiers are not implemented"));
            }
            let name = self.identifier()?.text;
            self.expect("{")?;
            let mut fields = vec![];
            while !self.at("}") && !self.at_end() {
                let modifiers = self.modifiers();
                let ty = self.ty()?;
                let name = self.identifier()?;
                let field = self.variable(ty, name, modifiers)?;
                self.expect(";")?;
                fields.push(field);
            }
            let end = self.expect("}")?;
            self.expect(";")?;
            return Ok(Declaration::Struct(Struct {
                name,
                fields,
                span: start.join(end),
            }));
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
            Ok(Declaration::Function(Box::new(Function {
                name: name.text,
                return_type: ty,
                parameters,
                attributes,
                modifiers,
                semantic,
                body,
                span,
            })))
        } else {
            if !attributes.is_empty() {
                return Err(self.error("global variable attributes are not implemented"));
            }
            let variable = self.variable(ty, name, modifiers)?;
            self.expect(";")?;
            Ok(Declaration::Global(variable))
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
            loop {
                arguments.push(self.ty()?);
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
        Ok(TypeRef {
            name: token.text,
            arguments,
            array: None,
            span: token.span,
        })
    }

    pub(crate) fn variable(
        &mut self,
        mut ty: TypeRef,
        name: Token,
        modifiers: Vec<String>,
    ) -> Result<Variable, Diagnostic> {
        if self.eat("[") {
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
            ty.array = Some(size);
            self.expect("]")?;
        }
        let semantic = self.semantic()?;
        let initializer = if self.eat("=") {
            Some(self.expression(0)?)
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
                arguments.push(self.expression(0)?);
                if !self.eat(",") {
                    break;
                }
            }
        }
        self.expect(close)?;
        Ok(arguments)
    }
}
