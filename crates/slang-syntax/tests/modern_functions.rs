use slang_lexer::lex;
use slang_source::SourceId;
use slang_syntax::{Declaration, ExprKind, StmtKind, parse};

#[test]
fn modern_functions_retain_named_traditional_parameters_and_void_returns() {
    let source = r#"
        [ForceInline]
        public func add(first : int, int second = 2, out target : int,)
        {
            target = first + second;
        }

        func declaration();
        func missing(value);
        func voidName(void);
    "#;
    let module = parse(&lex(SourceId(0), source).tokens).unwrap();
    let Declaration::Function(function) = &module.declarations[0] else {
        panic!()
    };
    assert_eq!(function.return_type.name, "void");
    assert_eq!(function.modifiers, ["public"]);
    assert_eq!(function.attributes[0].name, "ForceInline");
    assert_eq!(function.parameters.len(), 3);
    assert_eq!(function.parameters[0].name, "first");
    assert_eq!(function.parameters[1].ty.name, "int");
    assert!(function.parameters[1].initializer.is_some());
    assert_eq!(function.parameters[2].modifiers, ["out"]);

    for declaration in &module.declarations[1..] {
        let Declaration::Function(function) = declaration else {
            panic!()
        };
        assert!(function.body.is_none());
        assert_eq!(function.return_type.name, "void");
    }

    let Declaration::Function(missing) = &module.declarations[2] else {
        panic!()
    };
    assert!(missing.parameters[0].ty.is_inferred());
    let Declaration::Function(void_name) = &module.declarations[3] else {
        panic!()
    };
    assert_eq!(void_name.parameters.len(), 1);
    assert_eq!(void_name.parameters[0].name, "void");
    assert!(void_name.parameters[0].ty.is_inferred());
}

#[test]
fn modern_generics_retain_defaults_where_bounds_and_parameter_casts() {
    let source = r#"
        interface IValue {}
        interface IMarker {}

        func copy<T : IValue = Item, U = int>(value : T, other : U) -> T
            where T : IMarker
        {
            return (T) value;
        }
    "#;
    let module = parse(&lex(SourceId(0), source).tokens).unwrap();
    let Declaration::Function(function) = &module.declarations[2] else {
        panic!()
    };
    assert_eq!(function.generic_parameters.len(), 2);
    assert_eq!(function.generic_parameters[0].default.as_ref().unwrap().name, "Item");
    assert_eq!(function.generic_parameters[0].constraints[0].name, "IValue");
    assert_eq!(function.generic_parameters[0].constraints[1].name, "IMarker");
    let StmtKind::Block(statements) = &function.body.as_ref().unwrap().kind else {
        panic!()
    };
    assert!(matches!(&statements[0].kind, StmtKind::Return(Some(expression))
        if matches!(&expression.kind, ExprKind::Cast { target, .. } if target.parameter_ref)));
}

#[test]
fn modern_method_and_interface_receivers_retain_the_existing_modes() {
    let source = include_str!("../../../tests/conformance/positive/modern-function-methods.slang");
    let module = parse(&lex(SourceId(0), source).tokens).unwrap();
    let Declaration::Interface(interface) = &module.declarations[0] else {
        panic!()
    };
    assert_eq!(interface.requirements[0].parameters[0].name, "this");
    assert_eq!(interface.requirements[0].parameters[0].modifiers, ["const"]);
    assert_eq!(interface.requirements[1].parameters[0].modifiers, ["inout"]);
    let methods: Vec<_> = module
        .declarations
        .iter()
        .filter_map(|declaration| {
            if let Declaration::Function(function) = declaration
                && function.method.is_some()
            {
                Some(function)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(methods.len(), 4);
    assert!(methods.iter().all(|function| function.method.as_ref().unwrap().owner == "Box"));
    let offset =
        methods.iter().find(|function| function.method.as_ref().unwrap().name == "offset").unwrap();
    assert!(offset.method.as_ref().unwrap().is_static);
    assert_eq!(offset.parameters.len(), 1);
}

#[test]
fn unsupported_modern_function_families_do_not_discard_their_syntax() {
    for source in [
        r#"
            func invalid(value = 1) -> int
            {
                return value;
            }
        "#,
        r#"
            func invalid() throws int -> int
            {
                return 1;
            }
        "#,
        r#"
            struct Item
            {
                func invalid() const -> int
                {
                    return 1;
                }
            }
        "#,
        r#"
            func invalid<T>(value : T) -> T where T == int
            {
                return value;
            }
        "#,
        r#"
            func invalid<T>(value : T) -> T where optional T : IValue
            {
                return value;
            }
        "#,
        r#"
            func invalid([nointerpolation] value : int)
            {
            }
        "#,
        r#"
            void outer()
            {
                func inner()
                {
                }
            }
        "#,
        include_str!("../../../tests/conformance/negative/modern-function-result-semantic.slang"),
    ] {
        assert!(parse(&lex(SourceId(0), source).tokens).is_err(), "{source}");
    }
}
