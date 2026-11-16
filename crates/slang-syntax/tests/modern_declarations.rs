use slang_lexer::lex;
use slang_source::{SourceId, Span};
use slang_syntax::{Declaration, ExprKind, Module, StmtKind, TypeRef, parse};

fn module(source: &str) -> Module {
    parse(&lex(SourceId(0), source).tokens).unwrap()
}

#[test]
fn inferred_types_have_an_unambiguous_omitted_type_invariant() {
    let span = Span::default();
    let mut inferred = TypeRef::inferred(span);
    assert!(inferred.is_inferred());
    assert_eq!(inferred.span, span);
    inferred.application = true;
    assert!(!inferred.is_inferred());
}

#[test]
fn global_var_and_let_preserve_explicit_types_modifiers_and_annotations() {
    let source = r#"
        static const var count = 3;
        public let color : float4 : COLOR = float4(1);
        var input : StructuredBuffer<int> : register(t2, space3);
        var values : int[2][3];
    "#;
    let module = module(source);
    let globals: Vec<_> = module
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            Declaration::Global(variable) => Some(variable),
            _ => None,
        })
        .collect();
    assert_eq!(globals.len(), 4);
    assert!(globals[0].ty.is_inferred());
    assert_eq!(globals[0].modifiers, ["static", "const"]);
    assert_eq!(&source[globals[0].span.start..globals[0].span.end], "var count = 3");
    assert_eq!(globals[1].ty.name, "float4");
    assert_eq!(globals[1].modifiers, ["public", "let"]);
    assert_eq!(globals[1].semantic.as_deref(), Some("COLOR"));
    assert_eq!(globals[2].register.unwrap().binding.index, 2);
    assert_eq!(globals[2].register.unwrap().binding.space, 3);
    assert!(globals[2].initializer.is_none());
    assert_eq!(globals[3].ty.arrays.len(), 2);
    assert!(matches!(&globals[3].ty.arrays[0].kind, ExprKind::Integer(value) if value == "3"));
    assert!(matches!(&globals[3].ty.arrays[1].kind, ExprKind::Integer(value) if value == "2"));
}

#[test]
fn struct_var_let_and_static_members_use_the_existing_owner_model() {
    let module = module(
        r#"
            struct Box<T>
            {
                var value : T = T();
                let readonlyValue = 2;
                static var count = 3;
                static const let limit = 4;
            }
        "#,
    );
    let Declaration::Struct(structure) = &module.declarations[0] else {
        panic!()
    };
    assert_eq!(structure.fields.len(), 2);
    assert_eq!(structure.fields[0].ty.name, "T");
    assert!(structure.fields[1].ty.is_inferred());
    assert_eq!(structure.fields[1].modifiers, ["let"]);
    assert_eq!(module.static_members.len(), 2);

    for member in &module.static_members {
        assert_eq!(member.owner, "Box");
        let variable = module.declarations.iter().find_map(|declaration| {
            if let Declaration::Global(variable) = declaration
                && variable.name == member.variable_name
            {
                Some(variable)
            } else {
                None
            }
        });
        assert!(variable.unwrap().ty.is_inferred());
    }
}

#[test]
fn local_modern_declarations_preserve_runtime_immutability_without_const() {
    let module = module(
        r#"
            void run(int input)
            {
                var mutableValue = input;
                let readonlyValue = mutableValue + 1;
                static const let fixedValue = 2;
                var explicitValue : int;
                let uninitialized : int;
                var missingType;
                let commaValue = (input, mutableValue);
            }
        "#,
    );
    let Declaration::Function(function) = &module.declarations[0] else {
        panic!()
    };
    let StmtKind::Block(statements) = &function.body.as_ref().unwrap().kind else {
        panic!()
    };
    let variables: Vec<_> = statements
        .iter()
        .map(|statement| {
            let StmtKind::Variable(variable) = &statement.kind else {
                panic!()
            };
            variable
        })
        .collect();
    assert_eq!(variables[0].modifiers.len(), 0);
    assert_eq!(variables[1].modifiers, ["let"]);
    assert_eq!(variables[2].modifiers, ["static", "const", "let"]);
    assert_eq!(variables[3].ty.name, "int");
    assert!(variables[3].initializer.is_none());
    assert_eq!(variables[4].ty.name, "int");
    assert!(variables[4].initializer.is_none());
    assert!(variables[5].ty.is_inferred());
    assert!(variables[5].initializer.is_none());
    assert!(
        matches!(&variables[6].initializer.as_ref().unwrap().kind, ExprKind::Binary { op, .. } if op == ",")
    );
}

#[test]
fn for_headers_and_unbraced_bodies_parse_modern_declarations() {
    let module = module(
        r#"
            void run()
            {
                for (var index = 0; index < 2; index++)
                    let current = index;

                if (true)
                    var selected : int = 1;
                else
                    let other = 2;

                while (false)
                    static var value = 3;
            }
        "#,
    );
    let Declaration::Function(function) = &module.declarations[0] else {
        panic!()
    };
    let StmtKind::Block(statements) = &function.body.as_ref().unwrap().kind else {
        panic!()
    };
    let StmtKind::For {
        initializer,
        body,
        ..
    } = &statements[0].kind
    else {
        panic!()
    };
    assert!(matches!(&initializer.kind, StmtKind::Variable(variable) if variable.name == "index"));
    assert!(matches!(&body.kind, StmtKind::Variable(variable) if variable.modifiers == ["let"]));
    assert!(
        matches!(&statements[1].kind, StmtKind::If { then_branch, else_branch: Some(other), .. }
        if matches!(&then_branch.kind, StmtKind::Variable(_))
            && matches!(&other.kind, StmtKind::Variable(_)))
    );
    assert!(matches!(&statements[2].kind, StmtKind::While { body, .. }
        if matches!(&body.kind, StmtKind::Variable(variable) if variable.modifiers == ["static"])));
}

#[test]
fn cbuffer_modern_fields_retain_inference_initializers_in_the_hidden_structure() {
    let module = module(
        r#"
            cbuffer Params
            {
                var first = 1;
                let second : int = 2;
                static let extra = 3;
            }
        "#,
    );
    let Declaration::Struct(structure) = &module.declarations[0] else {
        panic!()
    };
    assert_eq!(structure.fields.len(), 2);
    assert!(structure.fields[0].ty.is_inferred());
    assert!(structure.fields[0].initializer.is_some());
    assert_eq!(structure.fields[1].modifiers, ["let"]);
    let globals: Vec<_> = module
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            Declaration::Global(variable) => Some(variable),
            _ => None,
        })
        .collect();
    let first = globals.iter().find(|variable| variable.name == "first").unwrap();
    assert!(first.ty.is_inferred());
    assert_eq!(first.span, structure.fields[0].span);
    assert!(first.initializer.is_none());
    let extra = globals.iter().find(|variable| variable.name == "extra").unwrap();
    assert_eq!(extra.modifiers, ["static", "let"]);
    assert!(extra.initializer.is_some());
}

#[test]
fn modern_declaration_groups_and_name_array_suffixes_are_rejected() {
    for source in [
        r#"
            void run()
            {
                var first = 1, second = 2;
            }
        "#,
        r#"
            void run()
            {
                let first : int = 1, second = 2;
            }
        "#,
        r#"
            void run()
            {
                var values[2] : int = { 1, 2 };
            }
        "#,
        r#"
            void run()
            {
                var value : const int = 1;
            }
        "#,
        r#"
            void run(var value : int)
            {
            }
        "#,
        r#"
            void run(let value : int)
            {
            }
        "#,
        r#"
            void run()
            {
                var = 1;
            }
        "#,
        r#"
            void run()
            {
                let value : = 1;
            }
        "#,
    ] {
        assert!(parse(&lex(SourceId(0), source).tokens).is_err(), "{source}");
    }
}
