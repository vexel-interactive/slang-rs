use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_syntax::Declaration;

#[test]
fn const_constructor_inputs_use_private_writable_copies_without_changing_requirements() {
    for mode in ["out", "inout"] {
        let source = format!(
            r#"
                interface IBuild
                {{
                    __init(const int input);
                }}

                struct Value : IBuild
                {{
                    int value;

                    __init({mode} int input)
                    {{
                        input = 4;
                        value = input + 1;
                    }}
                }};

                T make<T : IBuild>(int input) {{ return T(input); }}

                RWStructuredBuffer<int> output;

                [shader("compute")]
                [numthreads(1, 1, 1)]
                void main()
                {{
                    int input = 3;
                    output[0] = make<Value>(input).value;
                    output[1] = input;
                }}
            "#,
        );
        compile_source(
            "const-constructor-input.slang",
            &source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap();

        let mut sources = slang_source::SourceMap::default();
        let root = sources.add("const-constructor-input.slang", source);
        let program = slang_module::load(
            root,
            &mut sources,
            &MemoryIncludes::default(),
            &slang_preprocessor::Options::default(),
        )
        .unwrap();
        let checked = slang_sema::check_program(program).unwrap();
        let rechecked = slang_sema::check(checked.module.clone()).unwrap();
        for module in [&checked.module, &rechecked.module] {
            let interface = module.declarations.iter().find_map(|declaration| {
                if let Declaration::Interface(interface) = declaration {
                    Some(interface)
                } else {
                    None
                }
            });
            let interface = interface.unwrap();
            assert!(
                interface.requirements[0].parameters[0]
                    .modifiers
                    .iter()
                    .any(|modifier| modifier == "const")
            );
            let serialized = serde_json::to_value(interface).unwrap();
            assert!(
                serialized["requirements"][0]["parameters"][0]["modifiers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|modifier| modifier == "const")
            );
        }
        assert_eq!(checked.call_names, rechecked.call_names);
        assert_eq!(
            serde_json::to_string(&slang_ir::lower(&checked).unwrap()).unwrap(),
            serde_json::to_string(&slang_ir::lower(&rechecked).unwrap()).unwrap(),
        );
    }
}
