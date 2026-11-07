use crate::*;
use slang_lexer::{Token, TokenKind};
use slang_source::{Diagnostic, Span};

/// Parse a complete token stream. Errors never return a partially valid module.
pub fn parse(tokens: &[Token]) -> Result<Module, Vec<Diagnostic>> {
    let tokens: Vec<Token> = tokens.iter().filter(|t| !t.is_trivia()).cloned().collect();
    let mut parser = Parser {
        tokens,
        offset: 0,
        next_expr: 0,
        depth: 0,
    };
    let mut module = Module::default();
    let mut diagnostics = vec![];
    while !parser.at_end() {
        let start = parser.offset;
        let parsed = if parser.at("import")
            || parser.at("__exported")
            || parser.at("module")
            || parser.at("__include")
            || parser.at("implementing")
        {
            parser.module_directive(&mut module).map(|()| None)
        } else {
            parser.declaration().map(Some)
        };
        match parsed {
            Ok(None) => {}
            Ok(Some(declarations)) => module.declarations.extend(declarations),
            Err(error) => {
                diagnostics.push(error);
                if diagnostics.len() >= 32 {
                    break;
                }
                parser.offset = start;
                parser.synchronize();
            }
        }
    }
    if diagnostics.is_empty() {
        Ok(module)
    } else {
        Err(diagnostics)
    }
}

pub(crate) struct Parser {
    pub tokens: Vec<Token>,
    pub offset: usize,
    pub next_expr: usize,
    pub depth: usize,
}

impl Parser {
    pub fn current(&self) -> Option<&Token> {
        self.tokens.get(self.offset)
    }
    pub fn span(&self) -> Span {
        self.current()
            .or_else(|| self.tokens.last())
            .map_or(Span::default(), |t| t.span)
    }
    pub fn at_end(&self) -> bool {
        self.current().is_none_or(|t| t.kind == TokenKind::Eof)
    }
    pub fn at(&self, text: &str) -> bool {
        self.current().is_some_and(|t| t.is(text))
    }
    pub fn eat(&mut self, text: &str) -> bool {
        if self.at(text) {
            self.offset += 1;
            true
        } else {
            false
        }
    }
    pub fn error(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic::error("S001", self.span(), message)
    }
    pub fn expect(&mut self, text: &str) -> Result<Span, Diagnostic> {
        let span = self.span();
        if self.eat(text) {
            Ok(span)
        } else {
            Err(self.error(format!("expected '{text}'")))
        }
    }
    pub fn identifier(&mut self) -> Result<Token, Diagnostic> {
        let token = self
            .current()
            .filter(|t| t.kind == TokenKind::Identifier)
            .ok_or_else(|| self.error("expected identifier"))?
            .clone();
        if matches!(
            token.text.as_str(),
            "if" | "else"
                | "for"
                | "while"
                | "switch"
                | "case"
                | "default"
                | "return"
                | "break"
                | "continue"
                | "discard"
                | "do"
                | "true"
                | "false"
                | "struct"
                | "interface"
                | "import"
                | "module"
                | "namespace"
                | "var"
                | "let"
                | "typealias"
                | "typedef"
        ) {
            return Err(self.error("reserved keyword cannot be used as an identifier"));
        }
        self.offset += 1;
        Ok(token)
    }
    pub fn enter(&mut self) -> Result<(), Diagnostic> {
        if self.depth >= 32 {
            return Err(self.error("syntax nesting limit exceeded"));
        }
        self.depth += 1;
        Ok(())
    }

    fn synchronize(&mut self) {
        let mut braces = 0usize;
        while !self.at_end() {
            if self.eat("{") {
                braces += 1;
            } else if self.eat("}") {
                braces = braces.saturating_sub(1);
                if braces == 0 {
                    self.eat(";");
                    break;
                }
            } else if self.eat(";") && braces == 0 {
                break;
            } else {
                self.offset += 1;
            }
        }
        self.depth = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slang_lexer::lex;
    use slang_source::SourceId;

    fn module(text: &str) -> Result<Module, Vec<Diagnostic>> {
        parse(&lex(SourceId(0), text).tokens)
    }

    #[test]
    fn compute_shader_with_control_flow() {
        let text = "RWStructuredBuffer<float> output; [shader(\"compute\")] [numthreads(64,1,1)] void main(uint3 id : SV_DispatchThreadID) { for (uint i=0; i<4; i++) { if (i==2) continue; output[id.x] += float(i); } }";
        assert!(module(text).is_ok());
    }

    #[test]
    fn precedence_and_right_associative_assignment() {
        let module = module("int f() { int a; int b; a=b=1+2*3; return a; }").unwrap();
        let Declaration::Function(function) = &module.declarations[0] else {
            panic!()
        };
        let StmtKind::Block(statements) = &function.body.as_ref().unwrap().kind else {
            panic!()
        };
        let StmtKind::Expression(expr) = &statements[2].kind else {
            panic!()
        };
        let ExprKind::Binary { op, right, .. } = &expr.kind else {
            panic!()
        };
        assert_eq!(op, "=");
        assert!(matches!(&right.kind, ExprKind::Binary { op, .. } if op == "="));
    }

    #[test]
    fn long_flat_and_nested_expressions_are_bounded() {
        for expression in [
            "1+".repeat(1000) + "1",
            "(".repeat(1000) + "1" + &")".repeat(1000),
            "(int)".repeat(1000) + "1",
        ] {
            assert!(module(&format!("int f() {{return {expression};}}")).is_err());
        }
        for body in [
            "if(true)".repeat(1000) + ";",
            "{".repeat(1000) + &"}".repeat(1000),
        ] {
            assert!(module(&format!("void f() {{{body}}}")).is_err());
        }
    }

    #[test]
    fn malformed_and_unsupported_syntax_is_rejected() {
        for text in [
            "float f( {",
            "interface I {}",
            "float f(){return 1}",
            "float x[0];",
            "void f() { while(1) }",
        ] {
            assert!(module(text).is_err(), "{text}");
        }
    }
}
