use crate::{FileInclude, Import, Module, parser::Parser};
use slang_lexer::TokenKind;
use slang_source::Diagnostic;

impl Parser {
    pub(crate) fn module_directive(&mut self, module: &mut Module) -> Result<(), Diagnostic> {
        let start = self.span();
        if self.at("module") || self.at("implementing") {
            let primary = self.eat("module");
            if !primary {
                self.expect("implementing")?;
            }
            if module.module_declared || module.name.is_some() || module.implementation.is_some() {
                return Err(self.error("duplicate module declaration"));
            }
            if !module.declarations.is_empty()
                || !module.imports.is_empty()
                || !module.file_includes.is_empty()
            {
                return Err(self.error("module declaration must precede imports and declarations"));
            }
            if primary && self.eat(";") {
                module.module_declared = true;
                return Ok(());
            }
            let path = self.module_reference(!primary)?;
            self.expect(";")?;
            let name = Some(path.trim_end_matches(".slang").to_owned());
            if primary {
                module.module_declared = true;
                module.name = name;
            } else {
                module.implementation = name;
            }
        } else if self.eat("__include") {
            let path = self.module_reference(true)?;
            let end = self.expect(";")?;
            module.file_includes.push(FileInclude {
                path,
                span: start.join(end),
            });
        } else {
            let exported = self.eat("__exported");
            self.expect("import")?;
            let path = self.module_reference(true)?;
            let end = self.expect(";")?;
            module.imports.push(Import {
                path,
                exported,
                span: start.join(end),
            });
        }
        Ok(())
    }

    fn module_reference(&mut self, file_reference: bool) -> Result<String, Diagnostic> {
        let mut path = if self.current().is_some_and(|t| t.kind == TokenKind::String) {
            let token = self.current().unwrap().clone();
            self.offset += 1;
            serde_json::from_str::<String>(&token.text)
                .map_err(|_| self.error("invalid module path string"))?
        } else {
            let identifier = self.identifier()?.text;
            let mut path = if file_reference {
                identifier.replace('_', "-")
            } else {
                identifier
            };
            while file_reference && self.eat(".") {
                path.push('/');
                path.push_str(&self.identifier()?.text.replace('_', "-"));
            }
            path
        };
        if path.is_empty() {
            return Err(self.error("empty module path"));
        }
        if !path.ends_with(".slang") {
            path.push_str(".slang");
        }
        Ok(path)
    }
}
