use crate::{
    condition, expand,
    macros::{self, Builtin, Macro},
};
use slang_lexer::{Token, TokenKind, lex};
use slang_source::{Diagnostic, Severity, SourceId, SourceMap, Span};
use std::collections::{BTreeMap, BTreeSet};

/// Resolved name must be stable so pragma-once and cycle checks can use it.
pub trait IncludeResolver {
    fn resolve(
        &self,
        from: &str,
        requested: &str,
        system: bool,
    ) -> Result<(String, String), String>;
}

#[derive(Clone, Debug)]
pub struct Options {
    pub defines: BTreeMap<String, String>,
    pub max_include_depth: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            defines: BTreeMap::new(),
            max_include_depth: 64,
        }
    }
}

#[derive(Debug)]
pub struct Preprocessed {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Copy)]
struct Conditional {
    parent: bool,
    active: bool,
    taken: bool,
    seen_else: bool,
    span: Span,
}

pub fn preprocess(
    sources: &mut SourceMap,
    root: SourceId,
    resolver: &dyn IncludeResolver,
    options: &Options,
) -> Preprocessed {
    let mut processor = Processor {
        sources,
        resolver,
        options,
        macros: BTreeMap::new(),
        once: BTreeSet::new(),
        diagnostics: vec![],
    };
    for (name, builtin) in [("__LINE__", Builtin::Line), ("__FILE__", Builtin::File)] {
        processor.macros.insert(
            name.into(),
            Macro {
                params: None,
                variadic: false,
                body: vec![],
                builtin: Some(builtin),
            },
        );
    }
    for (name, value) in &options.defines {
        let body = lex(root, value)
            .tokens
            .into_iter()
            .filter(|t| !t.is_trivia() && t.kind != TokenKind::Eof)
            .collect();
        processor.builtin_redefinition(name, Span::new(root, 0, 0));
        processor.macros.insert(
            name.clone(),
            Macro {
                params: None,
                variadic: false,
                body,
                builtin: None,
            },
        );
    }
    let mut tokens = processor.file(root, 0);
    let end = processor.sources.get(root).map_or(0, |f| f.text.len());
    tokens.push(Token {
        kind: TokenKind::Eof,
        text: String::new(),
        span: Span::new(root, end, end),
    });
    Preprocessed {
        tokens,
        diagnostics: processor.diagnostics,
    }
}

struct Processor<'a> {
    sources: &'a mut SourceMap,
    resolver: &'a dyn IncludeResolver,
    options: &'a Options,
    macros: BTreeMap<String, Macro>,
    once: BTreeSet<String>,
    diagnostics: Vec<Diagnostic>,
}

impl Processor<'_> {
    fn file(&mut self, id: SourceId, depth: usize) -> Vec<Token> {
        let Some(file) = self.sources.get(id).cloned() else {
            return vec![];
        };
        self.sources
            .clear_line_directives(id)
            .expect("existing source ID");
        if self.once.contains(&file.name) {
            return vec![];
        }
        if depth >= self.options.max_include_depth {
            self.diagnostics.push(Diagnostic::error(
                "P001",
                Span::new(id, 0, 0),
                "maximum include depth exceeded",
            ));
            return vec![];
        }
        let lexed = lex(id, &file.text);
        self.diagnostics.extend(lexed.diagnostics);
        let mut tokens = vec![];
        let mut iter = lexed.tokens.into_iter().peekable();
        while let Some(token) = iter.next() {
            if token.is("\\") && iter.peek().is_some_and(|t| t.kind == TokenKind::Newline) {
                iter.next();
            } else {
                tokens.push(token);
            }
        }
        let mut conditions: Vec<Conditional> = vec![];
        let mut output = vec![];
        let mut pending = vec![];
        for line in tokens.split(|t| matches!(t.kind, TokenKind::Newline | TokenKind::Eof)) {
            let significant: Vec<Token> = line.iter().filter(|t| !t.is_trivia()).cloned().collect();
            let active = conditions.last().is_none_or(|c| c.active);
            if significant.first().is_some_and(|t| t.is("#")) {
                output.extend(self.expand(&pending));
                pending.clear();
                let Some(directive) = significant.get(1) else {
                    continue;
                };
                let args = &significant[2..];
                let span = directive.span;
                match directive.text.as_str() {
                    "if" | "ifdef" | "ifndef" => {
                        let value = if !active {
                            false
                        } else if directive.is("if") {
                            self.condition(args)
                        } else if args.len() != 1 || args[0].kind != TokenKind::Identifier {
                            self.error(span, "expected a macro name");
                            false
                        } else {
                            self.macros.contains_key(&args[0].text) != directive.is("ifndef")
                        };
                        conditions.push(Conditional {
                            parent: active,
                            active: active && value,
                            taken: value,
                            seen_else: false,
                            span,
                        });
                    }
                    "elif" => {
                        if let Some(frame) = conditions.last_mut() {
                            if frame.seen_else {
                                self.error(span, "#elif after #else");
                                continue;
                            }
                            let should_evaluate = frame.parent && !frame.taken;
                            let value = should_evaluate && self.condition(args);
                            let frame = conditions.last_mut().unwrap();
                            frame.active = value;
                            frame.taken |= value;
                        } else {
                            self.error(span, "#elif without #if");
                        }
                    }
                    "else" => {
                        if let Some(frame) = conditions.last_mut() {
                            if frame.seen_else {
                                self.error(span, "duplicate #else");
                                continue;
                            }
                            frame.active = frame.parent && !frame.taken;
                            frame.taken = true;
                            frame.seen_else = true;
                            if !args.is_empty() {
                                self.error(span, "unexpected tokens after #else");
                            }
                        } else {
                            self.error(span, "#else without #if");
                        }
                    }
                    "endif" => {
                        if conditions.pop().is_none() {
                            self.error(span, "#endif without #if");
                        }
                        if !args.is_empty() {
                            self.error(span, "unexpected tokens after #endif");
                        }
                    }
                    _ if !active => {}
                    "define" => self.define(args),
                    "line" => {
                        let args = self.expand(args);
                        match crate::line::parse(&args) {
                            Ok((directive, consumed)) => {
                                if let Err(message) =
                                    self.sources.add_line_directive(id, span.start, directive)
                                {
                                    self.error(span, message);
                                }
                                if let Some(extra) = args.get(consumed) {
                                    self.error(extra.span, "unexpected tokens after #line");
                                }
                            }
                            Err(diagnostic) => self.diagnostics.push(diagnostic),
                        }
                    }
                    "undef" => {
                        if args.len() == 1 && args[0].kind == TokenKind::Identifier {
                            self.macros.remove(&args[0].text);
                        } else {
                            self.error(span, "expected a macro name after #undef");
                        }
                    }
                    "include" => {
                        let args = self.expand(args);
                        let requested = if args.len() == 1 && args[0].kind == TokenKind::String {
                            Some((args[0].text[1..args[0].text.len() - 1].to_owned(), false))
                        } else if args.first().is_some_and(|t| t.is("<"))
                            && args.last().is_some_and(|t| t.is(">"))
                        {
                            Some((
                                args[1..args.len() - 1]
                                    .iter()
                                    .map(|t| t.text.as_str())
                                    .collect(),
                                true,
                            ))
                        } else {
                            None
                        };
                        if let Some((name, system)) = requested {
                            match self.resolver.resolve(&file.name, &name, system) {
                                Ok((name, text)) => {
                                    if !self.once.contains(&name) {
                                        let id = self.sources.add(name, text);
                                        output.extend(self.file(id, depth + 1));
                                    }
                                }
                                Err(message) => self
                                    .diagnostics
                                    .push(Diagnostic::error("P003", span, message)),
                            }
                        } else {
                            self.error(span, "expected a quoted or angle-bracket include path");
                        }
                    }
                    "pragma" if args.len() == 1 && args[0].is("once") => {
                        self.once.insert(file.name.clone());
                    }
                    "error" | "warning" => {
                        let message = args
                            .iter()
                            .map(|t| t.text.as_str())
                            .collect::<Vec<_>>()
                            .join(" ");
                        let mut diagnostic = Diagnostic::error("P004", span, message);
                        if directive.is("warning") {
                            diagnostic.severity = Severity::Warning;
                        }
                        self.diagnostics.push(diagnostic);
                    }
                    _ => self.error(
                        span,
                        format!("unsupported preprocessing directive #{}", directive.text),
                    ),
                }
            } else if active {
                pending.extend(significant);
            }
        }
        output.extend(self.expand(&pending));
        for condition in conditions {
            self.error(condition.span, "unterminated conditional directive");
        }
        output
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error("P002", span, message));
    }

    fn expand(&mut self, tokens: &[Token]) -> Vec<Token> {
        match expand::expand(tokens, &self.macros, self.sources) {
            Ok(tokens) => tokens,
            Err(diagnostic) => {
                self.diagnostics.push(diagnostic);
                vec![]
            }
        }
    }

    fn condition(&mut self, tokens: &[Token]) -> bool {
        let mut replaced = vec![];
        let mut i = 0;
        while i < tokens.len() {
            if tokens[i].is("defined") {
                let span = tokens[i].span;
                i += 1;
                let parens = tokens.get(i).is_some_and(|t| t.is("("));
                if parens {
                    i += 1;
                }
                let Some(name) = tokens.get(i).filter(|t| t.kind == TokenKind::Identifier) else {
                    self.error(span, "expected macro name after defined");
                    return false;
                };
                let value = self.macros.contains_key(&name.text);
                i += 1;
                if parens {
                    if !tokens.get(i).is_some_and(|t| t.is(")")) {
                        self.error(span, "expected ')' after defined");
                        return false;
                    }
                    i += 1;
                }
                replaced.push(Token {
                    kind: TokenKind::Integer,
                    text: if value { "1" } else { "0" }.into(),
                    span,
                });
            } else {
                replaced.push(tokens[i].clone());
                i += 1;
            }
        }
        let expanded = self.expand(&replaced);
        match condition::evaluate(&expanded) {
            Ok(value) => value != 0,
            Err(diagnostic) => {
                self.diagnostics.push(diagnostic);
                false
            }
        }
    }

    fn define(&mut self, args: &[Token]) {
        match macros::parse(args) {
            Ok((name, definition)) => {
                self.builtin_redefinition(&name, args[0].span);
                self.macros.insert(name, definition);
            }
            Err(diagnostic) => self.diagnostics.push(diagnostic),
        }
    }

    fn builtin_redefinition(&mut self, name: &str, span: Span) {
        if self
            .macros
            .get(name)
            .is_some_and(|definition| definition.builtin.is_some())
        {
            self.diagnostics.push(Diagnostic::warning(
                "P010",
                span,
                format!("redefinition of builtin macro '{name}'"),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct NoIncludes;
    impl IncludeResolver for NoIncludes {
        fn resolve(&self, _: &str, name: &str, _: bool) -> Result<(String, String), String> {
            Err(format!("missing {name}"))
        }
    }
    fn run(text: &str) -> Preprocessed {
        let mut sources = SourceMap::default();
        let id = sources.add("test", text);
        preprocess(&mut sources, id, &NoIncludes, &Options::default())
    }

    #[test]
    fn macro_aliases_nested_calls_and_self_recursion() {
        let result = run(
            "#define ADD(x,y) ((x)+(y))\n#define ALIAS ADD\n#define SELF SELF\nALIAS(ADD(1,2), 3) SELF",
        );
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|t| t.text.as_str())
                .collect::<String>(),
            "((((1)+(2)))+(3))SELF"
        );
    }

    #[test]
    fn conditionals_short_circuit_and_continuations() {
        let result = run(
            "#define X 1 + \\\n2\n#if defined(X) && X == 3 && (1 || 1/0)\nyes\n#else\n#error no\n#endif\n",
        );
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.tokens[0].text, "yes");
    }

    #[test]
    fn stringification_token_paste_and_variadic_parameters() {
        let result = run(
            "#define X 42\n#define STR(x) #x\n#define CAT(a,b) a##b\n#define SUM(a,...) a+__VA_ARGS__\nSTR(X) STR(a  + b) CAT(fl,oat) SUM(1,2) CAT(,ok) CAT(ok,)\n",
        );
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result
                .tokens
                .iter()
                .map(|t| t.text.as_str())
                .collect::<String>(),
            "\"X\"\"a + b\"float1+2okok"
        );
        for source in [
            "#define CAT(a,b) a##b\nCAT(+,*)",
            "#define F(x) ##x",
            "#define F(x,) x",
            "#define F(...) __VA_OPT__(x)",
        ] {
            assert!(!run(source).diagnostics.is_empty(), "{source}");
        }
    }

    #[test]
    fn excessive_conditional_nesting_is_a_diagnostic() {
        let source = format!("#if {}1{}\n#endif", "(".repeat(1000), ")".repeat(1000));
        assert!(!run(&source).diagnostics.is_empty());
    }

    #[test]
    fn raw_and_unused_arguments_are_not_expanded() {
        let result = run(
            "#define BAD(a,b) a+b\n#define STR(x) #x\n#define IGNORE(x) 1\nSTR(BAD(1)) IGNORE(BAD(1))",
        );
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.tokens[0].text, "\"BAD(1)\"");
        assert_eq!(result.tokens[1].text, "1");
    }

    #[test]
    fn builtin_lines_follow_initiating_calls_and_physical_crlf_lines() {
        let result = run(
            "#define HERE __LINE__\r\n#define ID(x) x\r\nHERE ID(\r\n__LINE__\r\n) __LINE__\r\n",
        );
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let values: Vec<_> = result
            .tokens
            .iter()
            .filter(|t| t.kind != TokenKind::Eof)
            .map(|t| t.text.as_str())
            .collect();
        assert_eq!(values, ["3", "3", "5"]);
        let result = run(
            "#define STR(x) #x\n#define EXPAND(x) STR(x)\n#define CAT(a,b) a##b\nSTR(__LINE__) EXPAND(__LINE__) CAT(__LI,NE__)",
        );
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.tokens[0].text, "\"__LINE__\"");
        assert_eq!(result.tokens[1].text, "\"4\"");
        assert_eq!(result.tokens[2].text, "4");
    }

    #[test]
    fn builtin_file_literals_escape_names_and_keep_invocation_spans() {
        let mut sources = SourceMap::default();
        let id = sources.add("C:\\shaders\\\"quoted\"\\shader.slang", "__FILE__");
        let result = preprocess(&mut sources, id, &NoIncludes, &Options::default());
        assert!(result.diagnostics.is_empty());
        assert_eq!(result.tokens[0].kind, TokenKind::String);
        assert_eq!(
            result.tokens[0].text,
            r#""C:\\shaders\\\"quoted\"\\shader.slang""#
        );
        assert_eq!(result.tokens[0].span, Span::new(id, 0, 8));
    }

    #[test]
    fn builtin_redefinition_and_undef_use_the_current_macro_environment() {
        let result = run(
            "#if !defined(__LINE__) || !defined(__FILE__)\n#error missing builtins\n#endif\n#define __LINE__ 17\n__LINE__\n#undef __LINE__\n#ifdef __LINE__\n#error undef failed\n#endif\n__LINE__",
        );
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code, "P010");
        assert_eq!(result.diagnostics[0].severity, Severity::Warning);
        assert_eq!(result.tokens[0].text, "17");
        assert_eq!(result.tokens[1].text, "__LINE__");
    }

    #[test]
    fn malformed_directives_fail() {
        for text in [
            "#else",
            "#if 1",
            "#define F(x,x) x",
            "#include \"missing\"",
            "#if 1/0\n#endif",
            "#define A(x) #unknown",
        ] {
            assert!(!run(text).diagnostics.is_empty(), "{text}");
        }
    }
}
