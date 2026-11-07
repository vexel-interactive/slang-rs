use slang_lexer::lex;
use slang_source::SourceId;
use slang_syntax::{Declaration, parse};

#[test]
fn register_tokens_stay_out_of_expression_and_semantic_names() {
    let source = "Texture2D<float> first:COLOR:register(t01.x,space02),second:register(t3);";
    let module = parse(&lex(SourceId(0), source).tokens).unwrap();
    let Declaration::Global(first) = &module.declarations[0] else {
        panic!()
    };
    let Declaration::Global(second) = &module.declarations[1] else {
        panic!()
    };
    assert_eq!(first.semantic.as_deref(), Some("COLOR"));
    assert!(second.semantic.is_none());
    let annotation = first.register.unwrap();
    assert_eq!(
        &source[annotation.span.start..annotation.span.end],
        "register(t01.x,space02)"
    );
    assert_eq!(
        &source[first.span.start..first.span.end],
        "Texture2D<float> first:COLOR:register(t01.x,space02)"
    );
    assert_eq!(annotation.binding.index, 1);
    assert_eq!(annotation.binding.space, 2);
    assert_eq!(second.register.unwrap().binding.index, 3);
}

#[test]
fn ordinary_parameter_and_result_semantics_survive_register_parsing() {
    let source = "float4 main(float4 value:SV_Position):SV_Target{return value;}";
    let module = parse(&lex(SourceId(0), source).tokens).unwrap();
    let Declaration::Function(function) = &module.declarations[0] else {
        panic!()
    };
    assert_eq!(
        function.parameters[0].semantic.as_deref(),
        Some("SV_Position")
    );
    assert!(function.parameters[0].register.is_none());
    assert_eq!(function.semantic.as_deref(), Some("SV_Target"));
}
