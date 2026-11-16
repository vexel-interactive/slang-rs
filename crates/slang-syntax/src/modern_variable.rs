use crate::{TypeRef, Variable, parser::Parser};
use slang_source::{Diagnostic, Span};

impl Parser {
    /// Modern declarations contain one name and an optional colon type.
    /// `let` retains immutability independently of a `const` modifier.
    pub(crate) fn modern_variable(
        &mut self,
        mut modifiers: Vec<String>,
    ) -> Result<Variable, Diagnostic> {
        let start = self.span();

        if self.eat("let") {
            modifiers.push("let".into());
        } else {
            self.expect("var")?;
        }

        self.modern_binding(modifiers, start)
    }

    /// A named modern binding shared by variables and `func` parameters.
    pub(crate) fn modern_binding(
        &mut self,
        modifiers: Vec<String>,
        start: Span,
    ) -> Result<Variable, Diagnostic> {
        let name = self.identifier()?;
        let ty = if self.eat(":") {
            self.ty()?
        } else {
            TypeRef::inferred(name.span)
        };
        let annotations = self.annotations()?;
        let initializer = if self.eat("=") {
            Some(self.expression(1)?)
        } else {
            None
        };
        let end = self.tokens[self.offset - 1].span;

        Ok(Variable {
            name: name.text,
            ty,
            modifiers,
            semantic: annotations.semantic,
            register: annotations.register,
            pack_offset: annotations.pack_offset,
            initializer,
            span: start.join(end),
        })
    }
}
