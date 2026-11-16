use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_sema::CheckedModule;

const DORMANT_OWNER: &str =
    include_str!("../../../tests/conformance/positive/interface-constructor-adapter-dormant.slang");

const OVERLOADED_REQUIREMENTS: &str = include_str!(
    "../../../tests/conformance/positive/interface-constructor-adapter-overloads.slang"
);

const NESTED_OWNER: &str =
    include_str!("../../../tests/conformance/positive/interface-constructor-adapter-nested.slang");

const AMBIGUOUS_DEFAULT_TAILS: &str = include_str!(
    "../../../tests/conformance/negative/interface-constructor-adapter-ambiguous-tails.slang"
);

fn checked(source: &str) -> CheckedModule {
    let mut sources = slang_source::SourceMap::default();
    let root = sources.add("constructor-adapter-edges.slang", source);
    let program = slang_module::load(
        root,
        &mut sources,
        &MemoryIncludes::default(),
        &slang_preprocessor::Options::default(),
    )
    .unwrap();
    slang_sema::check_program(program).unwrap()
}

#[test]
fn dormant_generic_constructor_owners_do_not_emit_default_dependencies() {
    let compiled = compile_source(
        "dormant.slang",
        DORMANT_OWNER,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap();

    assert_eq!(compiled.ir.functions.len(), 1);
    assert_eq!(compiled.ir.functions[0].name, "main");
    assert!(compiled.ir.structures.is_empty());
}

#[test]
fn repeated_calls_reuse_each_overloaded_constructor_requirement_adapter() {
    let checked = checked(OVERLOADED_REQUIREMENTS);
    let adapters: Vec<_> = checked
        .module
        .generic_instances
        .iter()
        .filter(|instance| instance.witness_target.is_some())
        .collect();

    assert_eq!(adapters.len(), 2);
    assert_ne!(adapters[0].context, adapters[1].context);
    assert_eq!(adapters[0].witness_target, adapters[1].witness_target);
    let ir = slang_ir::lower(&checked).unwrap();
    assert_eq!(ir.functions.iter().filter(|function| function.name == "main").count(), 1);
}

#[test]
fn overloaded_constructor_adapters_do_not_duplicate_during_program_rechecking() {
    let checked = checked(OVERLOADED_REQUIREMENTS);
    let rechecked = slang_sema::check_program(slang_module::Program {
        syntax: checked.module.clone(),
        modules: checked.modules.clone(),
        symbols: checked.symbols.clone(),
        diagnostics: vec![],
    })
    .unwrap();

    assert_eq!(checked.module.generic_instances.len(), rechecked.module.generic_instances.len());
    assert_eq!(checked.call_names, rechecked.call_names);
    assert_eq!(
        serde_json::to_string(&slang_ir::lower(&checked).unwrap()).unwrap(),
        serde_json::to_string(&slang_ir::lower(&rechecked).unwrap()).unwrap(),
    );
}

#[test]
fn nested_generic_owner_constructor_adapters_keep_concrete_owner_names() {
    let checked = checked(NESTED_OWNER);
    let adapters: Vec<_> = checked
        .module
        .generic_instances
        .iter()
        .filter(|instance| instance.witness_target.is_some())
        .collect();
    assert_eq!(adapters.len(), 1);
    let ir = slang_ir::lower(&checked).unwrap();
    assert!(ir.structures.iter().all(|structure| !structure.name.contains("parameter_")));
    assert!(ir.functions.iter().all(|function| !function.name.contains("parameter_")));
}

#[test]
fn constructor_ranking_rejects_tied_default_tails() {
    let error = compile_source(
        "invalid-constructor-adapter.slang",
        AMBIGUOUS_DEFAULT_TAILS,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();

    assert!(error.diagnostics.iter().any(|diagnostic| diagnostic.code == "T032"), "{error}");
}

#[test]
fn exact_output_constructor_beats_a_worse_input_conversion() {
    let checked = checked(include_str!(
        "../../../tests/conformance/positive/interface-constructor-adapter-exact-output.slang"
    ));
    let targets: Vec<_> = checked
        .module
        .generic_instances
        .iter()
        .filter_map(|instance| instance.witness_target)
        .collect();
    assert_eq!(targets.len(), 1);
    let target = checked.module.declarations.iter().find_map(|declaration| {
        if let slang_syntax::Declaration::Function(function) = declaration
            && function.span == targets[0]
        {
            Some(function)
        } else {
            None
        }
    });
    assert!(target.unwrap().parameters[0].modifiers.iter().any(|modifier| modifier == "out"));
    slang_ir::lower(&checked).unwrap();
}

#[test]
fn constructor_requirement_modes_accept_input_copies_and_same_width_integer_views() {
    for required in ["in", "out", "inout"] {
        for actual in ["in", "out", "inout"] {
            for scalar in ["int", "uint"] {
                let source = format!(
                    r#"
                        interface IBuild
                        {{
                            __init({required} int input);
                        }}

                        struct Value : IBuild
                        {{
                            int value;

                            __init({actual} {scalar} input)
                            {{
                                input = {scalar}(5);
                                value = 7;
                            }}
                        }}

                        T make<T : IBuild>({required} int input)
                        {{
                            return T(input);
                        }}

                        [shader("compute")]
                        [numthreads(1, 1, 1)]
                        void main()
                        {{
                            int input = 3;
                            Value value = make<Value>(input);
                        }}
                    "#
                );
                compile_source(
                    "constructor-modes.slang",
                    &source,
                    &CompileOptions::default(),
                    &MemoryIncludes::default(),
                )
                .unwrap_or_else(|error| panic!("{required} -> {actual} {scalar}: {error}"));
            }
        }
    }
}

#[test]
fn interfaces_cannot_be_declared_inside_structs_or_generic_structs() {
    for source in [
        include_str!("../../../tests/conformance/negative/nested-interface.slang"),
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
        let error = compile_source(
            "nested-interface.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();

        assert!(error.diagnostics.iter().any(|diagnostic| diagnostic.code == "S001"), "{error}");
        assert!(
            error.diagnostics.iter().any(|diagnostic| diagnostic
                .message
                .contains("interface declarations are not allowed inside structs")),
            "{error}"
        );
    }
}
