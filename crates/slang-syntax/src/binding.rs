use crate::{RegisterAnnotation, parser::Parser};
use slang_layout::{MAX_REGISTER_INDEX, RegisterBinding, RegisterClass};
use slang_source::Diagnostic;

impl Parser {
    pub(crate) fn annotations(
        &mut self,
    ) -> Result<(Option<String>, Option<RegisterAnnotation>), Diagnostic> {
        let mut semantic = None;
        let mut register: Option<RegisterAnnotation> = None;
        while self.eat(":") {
            let name = self.identifier()?;
            if name.is("register") && self.at("(") {
                let annotation = self.register_annotation(name.span)?;
                if register.is_some_and(|r| r.binding != annotation.binding) {
                    return Err(Diagnostic::error(
                        "S001",
                        annotation.span,
                        "conflicting register annotations",
                    ));
                }
                register = Some(annotation);
            } else if semantic.replace(name.text).is_some() {
                return Err(self.error("multiple declaration semantics are not implemented"));
            }
        }
        Ok((semantic, register))
    }

    fn register_annotation(
        &mut self,
        start: slang_source::Span,
    ) -> Result<RegisterAnnotation, Diagnostic> {
        self.expect("(")?;
        let name = self.identifier()?;
        let class = name
            .text
            .chars()
            .next()
            .and_then(RegisterClass::from_prefix)
            .ok_or_else(|| {
                Diagnostic::error("S001", name.span, "expected a b, t, u, or s register")
            })?;
        let index = register_index(&name.text[1..], name.span, "register index")?;
        // Upstream accepts a component mask, but resource registers use the
        // whole slot. Component packing belongs to uniform byte layout.
        if self.eat(".") {
            self.identifier()?;
        }
        let space = if self.eat(",") {
            let name = self.identifier()?;
            let digits = name.text.strip_prefix("space").ok_or_else(|| {
                Diagnostic::error("S001", name.span, "expected 'space' followed by an index")
            })?;
            let space = register_index(digits, name.span, "register space index")?;
            if self.eat(".") {
                self.identifier()?;
            }
            space
        } else {
            0
        };
        let end = self.expect(")")?;
        Ok(RegisterAnnotation {
            binding: RegisterBinding {
                class,
                index,
                space,
            },
            span: start.join(end),
        })
    }
}

fn register_index(digits: &str, span: slang_source::Span, kind: &str) -> Result<u32, Diagnostic> {
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Diagnostic::error(
            "S001",
            span,
            format!("expected a decimal {kind}"),
        ));
    }
    digits
        .parse::<u32>()
        .ok()
        .filter(|&index| index <= MAX_REGISTER_INDEX)
        .ok_or_else(|| {
            Diagnostic::error(
                "S001",
                span,
                format!("{kind} must be between 0 and {MAX_REGISTER_INDEX}"),
            )
        })
}
