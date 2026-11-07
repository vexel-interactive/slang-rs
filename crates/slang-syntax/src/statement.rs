use crate::parser::Parser;
use crate::*;
use slang_lexer::TokenKind;
use slang_source::Diagnostic;

impl Parser {
    pub(crate) fn statement(&mut self) -> Result<Stmt, Diagnostic> {
        self.enter()?;
        let result = self.statement_inner();
        self.depth -= 1;
        result
    }

    fn statement_inner(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.span();
        let kind = if self.eat("{") {
            let mut statements = vec![];
            while !self.at("}") && !self.at_end() {
                statements.push(self.statement()?);
            }
            let end = self.expect("}")?;
            return Ok(Stmt {
                kind: StmtKind::Block(statements),
                span: start.join(end),
            });
        } else if self.eat(";") {
            StmtKind::Empty
        } else if self.eat("return") {
            let value = if self.at(";") {
                None
            } else {
                Some(self.expression(0)?)
            };
            self.expect(";")?;
            StmtKind::Return(value)
        } else if self.eat("if") {
            self.expect("(")?;
            let condition = self.expression(0)?;
            self.expect(")")?;
            let then_branch = Box::new(self.statement()?);
            let else_branch = if self.eat("else") {
                Some(Box::new(self.statement()?))
            } else {
                None
            };
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            }
        } else if self.eat("while") {
            self.expect("(")?;
            let condition = self.expression(0)?;
            self.expect(")")?;
            StmtKind::While {
                condition,
                body: Box::new(self.statement()?),
            }
        } else if self.eat("for") {
            self.expect("(")?;
            let initializer = Box::new(self.simple_statement()?);
            let condition = if self.at(";") {
                None
            } else {
                Some(self.expression(0)?)
            };
            self.expect(";")?;
            let increment = if self.at(")") {
                None
            } else {
                Some(self.expression(0)?)
            };
            self.expect(")")?;
            StmtKind::For {
                initializer,
                condition,
                increment,
                body: Box::new(self.statement()?),
            }
        } else if self.eat("do") {
            let body = Box::new(self.statement()?);
            self.expect("while")?;
            self.expect("(")?;
            let condition = self.expression(0)?;
            self.expect(")")?;
            self.expect(";")?;
            StmtKind::DoWhile { body, condition }
        } else if self.eat("switch") {
            self.expect("(")?;
            let selector = self.expression(0)?;
            self.expect(")")?;
            self.expect("{")?;
            let mut cases = vec![];
            while !self.at("}") && !self.at_end() {
                let label_span = self.span();
                let value = if self.eat("case") {
                    Some(self.expression(0)?)
                } else if self.eat("default") {
                    None
                } else {
                    return Err(self.error("expected case or default label"));
                };
                self.expect(":")?;
                let mut statements = vec![];
                while !self.at("case") && !self.at("default") && !self.at("}") && !self.at_end() {
                    statements.push(self.statement()?);
                }
                cases.push(SwitchCase {
                    value,
                    statements,
                    span: label_span,
                });
            }
            self.expect("}")?;
            StmtKind::Switch { selector, cases }
        } else if self.eat("break") {
            self.expect(";")?;
            StmtKind::Break
        } else if self.eat("continue") {
            self.expect(";")?;
            StmtKind::Continue
        } else if self.eat("discard") {
            self.expect(";")?;
            StmtKind::Discard
        } else {
            return self.simple_statement();
        };
        let end = self
            .tokens
            .get(self.offset.saturating_sub(1))
            .map_or(start, |t| t.span);
        Ok(Stmt {
            kind,
            span: start.join(end),
        })
    }

    fn simple_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.span();
        if self.eat(";") {
            return Ok(Stmt {
                kind: StmtKind::Empty,
                span: start,
            });
        }
        if self.looks_like_variable() {
            let modifiers = self.modifiers();
            let ty = self.ty()?;
            let name = self.identifier()?;
            let mut variables = self.variables(ty, name, modifiers)?;
            let end = self.expect(";")?;
            return Ok(Stmt {
                kind: if variables.len() == 1 {
                    StmtKind::Variable(variables.pop().unwrap())
                } else {
                    StmtKind::Variables(variables)
                },
                span: start.join(end),
            });
        }
        let expr = self.expression(0)?;
        let end = self.expect(";")?;
        Ok(Stmt {
            kind: StmtKind::Expression(expr),
            span: start.join(end),
        })
    }

    fn looks_like_variable(&self) -> bool {
        let mut i = self.offset;
        while self.tokens.get(i).is_some_and(|t| {
            matches!(
                t.text.as_str(),
                "const" | "static" | "uniform" | "groupshared" | "precise" | "nointerpolation"
            )
        }) {
            i += 1;
        }
        if !self
            .tokens
            .get(i)
            .is_some_and(|t| t.kind == TokenKind::Identifier)
        {
            return false;
        }
        i += 1;
        if self.tokens.get(i).is_some_and(|t| t.is("<")) {
            let mut nesting = 0i32;
            loop {
                let Some(token) = self.tokens.get(i) else {
                    return false;
                };
                match token.text.as_str() {
                    "<" => nesting += 1,
                    ">" => nesting -= 1,
                    ">>" => nesting -= 2,
                    ";" | "{" | "}" | "=" => return false,
                    _ => {}
                }
                i += 1;
                if nesting <= 0 {
                    break;
                }
            }
        }
        while self.tokens.get(i).is_some_and(|t| t.is("[")) {
            if !self
                .tokens
                .get(i + 1)
                .is_some_and(|t| t.kind == TokenKind::Integer)
                || !self.tokens.get(i + 2).is_some_and(|t| t.is("]"))
            {
                return false;
            }
            i += 3;
        }
        self.tokens
            .get(i)
            .is_some_and(|t| t.kind == TokenKind::Identifier)
    }
}
