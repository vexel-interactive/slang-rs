use crate::macros::Macro;
use slang_lexer::{Token, TokenKind, lex};
use slang_source::{Diagnostic, Span};
use std::collections::BTreeMap;

pub(crate) fn substitute(
    definition: &Macro,
    raw: &BTreeMap<String, Vec<Token>>,
    expanded: &BTreeMap<String, Vec<Token>>,
    span: Span,
) -> Result<Vec<Token>, Diagnostic> {
    let mut output: Vec<Token> = vec![];
    let mut paste = false;
    let mut i = 0;
    while i < definition.body.len() {
        let token = &definition.body[i];
        if token.is("##") {
            paste = true;
            i += 1;
            continue;
        }
        let mut piece = if token.is("#") {
            i += 1;
            let name = &definition.body[i].text;
            vec![Token {
                kind: TokenKind::String,
                text: stringify(raw.get(name).map_or(&[], |t| t.as_slice())),
                span,
            }]
        } else if let Some(argument) = raw.get(&token.text) {
            let adjacent = paste || definition.body.get(i + 1).is_some_and(|t| t.is("##"));
            if adjacent {
                argument.clone()
            } else {
                expanded.get(&token.text).cloned().unwrap_or_default()
            }
        } else {
            vec![Token {
                span,
                ..token.clone()
            }]
        };
        if paste && !piece.is_empty() && !output.is_empty() {
            let left = output.pop().unwrap();
            let right = piece.remove(0);
            let text = format!("{}{}", left.text, right.text);
            let result = lex(span.source, &text);
            let tokens: Vec<_> = result
                .tokens
                .into_iter()
                .filter(|t| t.kind != TokenKind::Eof)
                .collect();
            if !result.diagnostics.is_empty() || tokens.len() != 1 || tokens[0].is_trivia() {
                return Err(Diagnostic::error(
                    "P009",
                    span,
                    format!("token paste '{text}' does not form one token"),
                ));
            }
            output.push(Token {
                span,
                ..tokens[0].clone()
            });
        }
        output.extend(piece);
        paste = false;
        i += 1;
    }
    Ok(output)
}

fn stringify(tokens: &[Token]) -> String {
    let mut text = String::new();
    let mut previous: Option<&Token> = None;
    for token in tokens {
        if previous
            .is_some_and(|p| p.span.source != token.span.source || p.span.end < token.span.start)
        {
            text.push(' ');
        }
        text.push_str(&token.text);
        previous = Some(token);
    }
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}
