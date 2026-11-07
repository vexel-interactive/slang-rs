use slang_lexer::{Token, TokenKind};
use slang_source::{Diagnostic, Span};

#[derive(Clone, Debug)]
pub(crate) struct Macro {
    pub params: Option<Vec<String>>,
    pub variadic: bool,
    pub body: Vec<Token>,
}

pub(crate) fn parse(tokens: &[Token]) -> Result<(String, Macro), Diagnostic> {
    let span = tokens.first().map_or(Span::default(), |t| t.span);
    let error = |message: &str| Diagnostic::error("P008", span, message);
    let name = tokens
        .first()
        .filter(|t| t.kind == TokenKind::Identifier)
        .ok_or_else(|| error("expected macro name"))?;
    let mut i = 1;
    let mut variadic = false;
    let params = if tokens
        .get(i)
        .is_some_and(|t| t.is("(") && t.span.start == name.span.end)
    {
        i += 1;
        let mut params = vec![];
        while !tokens.get(i).is_some_and(|t| t.is(")")) {
            if tokens.get(i).is_some_and(|t| t.is("...")) {
                variadic = true;
                params.push("__VA_ARGS__".into());
                i += 1;
                if !tokens.get(i).is_some_and(|t| t.is(")")) {
                    return Err(error("variadic parameter must be last"));
                }
                break;
            }
            let param = tokens
                .get(i)
                .filter(|t| t.kind == TokenKind::Identifier)
                .ok_or_else(|| error("expected macro parameter"))?;
            if params.contains(&param.text) || param.is("__VA_ARGS__") {
                return Err(error("duplicate or reserved macro parameter"));
            }
            params.push(param.text.clone());
            i += 1;
            if tokens.get(i).is_some_and(|t| t.is(",")) {
                i += 1;
                if tokens.get(i).is_some_and(|t| t.is(")")) {
                    return Err(error("trailing comma in macro parameters"));
                }
            } else if !tokens.get(i).is_some_and(|t| t.is(")")) {
                return Err(error("expected ',' or ')' in macro parameters"));
            }
        }
        i += 1;
        Some(params)
    } else {
        None
    };
    let body = tokens[i..].to_vec();
    for (i, token) in body.iter().enumerate() {
        if token.is("__VA_OPT__") {
            return Err(error("__VA_OPT__ is not implemented"));
        }
        if token.is("__VA_ARGS__") && !variadic {
            return Err(error("__VA_ARGS__ requires a variadic macro"));
        }
        if token.is("#")
            && !body
                .get(i + 1)
                .is_some_and(|t| params.as_ref().is_some_and(|p| p.contains(&t.text)))
        {
            return Err(error("'#' must precede a macro parameter"));
        }
        if token.is("##")
            && (i == 0 || i + 1 == body.len() || body[i - 1].is("##") || body[i + 1].is("##"))
        {
            return Err(error("'##' requires tokens on both sides"));
        }
    }
    Ok((
        name.text.clone(),
        Macro {
            params,
            variadic,
            body,
        },
    ))
}
