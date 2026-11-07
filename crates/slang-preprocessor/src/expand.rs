use crate::{
    macros::{Builtin, Macro},
    substitute::substitute,
};
use slang_lexer::{Token, TokenKind};
use slang_source::{Diagnostic, SourceMap, Span};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone)]
struct Item {
    token: Token,
    hidden: BTreeSet<String>,
    initiating: Option<Span>,
}

pub(crate) fn expand(
    tokens: &[Token],
    macros: &BTreeMap<String, Macro>,
    sources: &SourceMap,
) -> Result<Vec<Token>, Diagnostic> {
    expand_at(tokens, macros, sources, 0, None)
}

fn expand_at(
    tokens: &[Token],
    macros: &BTreeMap<String, Macro>,
    sources: &SourceMap,
    depth: usize,
    initiating: Option<Span>,
) -> Result<Vec<Token>, Diagnostic> {
    if depth > 128 {
        return Err(Diagnostic::error(
            "P005",
            tokens.first().map_or(Span::default(), |t| t.span),
            "macro nesting limit exceeded",
        ));
    }
    let mut queue: VecDeque<Item> = tokens
        .iter()
        .map(|t| Item {
            token: t.clone(),
            hidden: BTreeSet::new(),
            initiating,
        })
        .collect();
    let mut output = vec![];
    let mut steps = 0;
    while let Some(item) = queue.pop_front() {
        steps += 1;
        if steps > 100_000 {
            return Err(Diagnostic::error(
                "P005",
                item.token.span,
                "macro expansion token limit exceeded",
            ));
        }
        let name = &item.token.text;
        let Some(definition) = macros
            .get(name)
            .filter(|_| item.token.kind == TokenKind::Identifier && !item.hidden.contains(name))
        else {
            output.push(item.token);
            continue;
        };
        let initiating = item.initiating.unwrap_or(item.token.span);
        if let Some(builtin) = definition.builtin {
            if let Some(file) = sources.get(initiating.source) {
                let location = file.presumed_location(initiating.start);
                output.push(Token {
                    kind: match builtin {
                        Builtin::Line => TokenKind::Integer,
                        Builtin::File => TokenKind::String,
                    },
                    text: match builtin {
                        Builtin::Line => location.line.to_string(),
                        Builtin::File => crate::line::quote_name(location.name),
                    },
                    span: item.token.span,
                });
            } else {
                output.push(item.token);
            }
            continue;
        }
        let mut arguments = BTreeMap::new();
        let mut raw_arguments = BTreeMap::new();
        if let Some(params) = &definition.params {
            if !queue.front().is_some_and(|t| t.token.is("(")) {
                output.push(item.token);
                continue;
            }
            queue.pop_front();
            let mut args: Vec<Vec<Token>> = vec![vec![]];
            let mut nesting = 0;
            loop {
                let Some(next) = queue.pop_front() else {
                    return Err(Diagnostic::error(
                        "P006",
                        item.token.span,
                        "unterminated macro invocation",
                    ));
                };
                if next.token.is(")") && nesting == 0 {
                    break;
                }
                if next.token.is(",") && nesting == 0 {
                    args.push(vec![]);
                    continue;
                }
                if next.token.is("(") {
                    nesting += 1;
                }
                if next.token.is(")") {
                    nesting -= 1;
                }
                args.last_mut().unwrap().push(next.token);
            }
            if params.is_empty() && args.len() == 1 && args[0].is_empty() {
                args.clear();
            }
            let fixed = params.len() - usize::from(definition.variadic);
            if args.len() < fixed || !definition.variadic && args.len() != fixed {
                return Err(Diagnostic::error(
                    "P006",
                    item.token.span,
                    format!(
                        "macro {name} expects {} arguments, got {}",
                        params.len(),
                        args.len()
                    ),
                ));
            }
            if definition.variadic {
                let tail = args.split_off(fixed);
                let mut variadic = vec![];
                for (index, argument) in tail.into_iter().enumerate() {
                    if index > 0 {
                        variadic.push(Token {
                            kind: TokenKind::Punctuation,
                            text: ",".into(),
                            span: item.token.span,
                        });
                    }
                    variadic.extend(argument);
                }
                args.push(variadic);
            }
            for (param, argument) in params.iter().zip(args) {
                let expanded_use = definition.body.iter().enumerate().any(|(i, token)| {
                    token.is(param)
                        && !i
                            .checked_sub(1)
                            .and_then(|j| definition.body.get(j))
                            .is_some_and(|t| t.is("#") || t.is("##"))
                        && !definition.body.get(i + 1).is_some_and(|t| t.is("##"))
                });
                if expanded_use {
                    arguments.insert(
                        param.clone(),
                        expand_at(&argument, macros, sources, depth + 1, Some(initiating))?,
                    );
                }
                raw_arguments.insert(param.clone(), argument);
            }
        }
        let mut hidden = item.hidden.clone();
        hidden.insert(name.clone());
        let replacement = substitute(definition, &raw_arguments, &arguments, item.token.span)?;
        for token in replacement.into_iter().rev() {
            queue.push_front(Item {
                token,
                hidden: hidden.clone(),
                initiating: Some(initiating),
            });
        }
    }
    Ok(output)
}
