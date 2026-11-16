use slang_lexer::lex;
use slang_source::SourceId;
use slang_syntax::{Declaration, parse};

#[test]
fn nested_interfaces_are_rejected_before_semantic_checking() {
    for source in [
        r#"
            struct Outer
            {
                interface IBuild
                {
                    __init(int input);
                }
            }
        "#,
        r#"
            struct Outer<T>
            {
                interface IBuild<U>
                {
                    __init(U input);
                }
            }
        "#,
    ] {
        let diagnostics = parse(&lex(SourceId(0), source).tokens).unwrap_err();
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "S001"
                && diagnostic
                    .message
                    .contains("interface declarations are not allowed inside structs")
        }));
    }
}

#[test]
fn struct_and_alias_members_keep_their_nested_type_owners() {
    let source = r#"
        interface IBuild
        {
            __init(int input);
        }

        struct Outer<T>
        {
            typealias Marker = T;

            struct Value : IBuild
            {
                Marker marker;
                int value;
            }
        }
    "#;
    let module = parse(&lex(SourceId(0), source).tokens).unwrap();
    assert!(matches!(&module.declarations[0], Declaration::Interface(_)));
    assert_eq!(module.nested_types.len(), 2);
    assert!(
        module.nested_types.iter().any(|nested| nested.owner == "Outer" && nested.name == "Marker")
    );
    assert!(
        module.nested_types.iter().any(|nested| nested.owner == "Outer" && nested.name == "Value")
    );
}
