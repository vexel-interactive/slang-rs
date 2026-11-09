use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> Compilation {
    compile_source(
        "constructors.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn selected_constructors_preserve_fields_shadows_outputs_and_early_returns() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/user-constructors.slang"
    ));
    let json = serde_json::to_string(&compiled.ir).unwrap();
    assert!(!json.contains("BlockExpression") && !json.contains("AddressView"));
    for index in 0..compiled.ir.functions.len() {
        if let Some(graph) = slang_ir::flow::build_function(&compiled.ir, index) {
            graph.verify().unwrap();
        }
    }
    let artifact = compiled.artifact.unwrap();
    let main = artifact.split("void main()").last().unwrap();
    assert!(!main.contains("constructor_Value("), "{main}");
    assert!(artifact.contains("__overload_"), "{artifact}");
}

#[test]
fn defaults_initialize_declarations_and_nested_fields_with_explicit_array_elements() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/user-constructors-defaults.slang"
    ));
    assert!(
        !serde_json::to_string(&compiled.ir)
            .unwrap()
            .contains("BlockExpression")
    );
    let artifact = compiled.artifact.unwrap();
    let main = artifact.split("void main()").last().unwrap();
    assert!(!main.contains("constructor_Item("), "{main}");
}

#[test]
fn global_constructor_expansion_stays_in_initializer_functions() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/user-constructors-global.slang"
    ));
    assert!(compiled.ir.globals.iter().any(|g| {
        g.initializer
            .as_ref()
            .is_some_and(|v| matches!(v.kind, slang_ir::ExprKind::Call { .. }))
    }));
    let json = serde_json::to_string(&compiled.ir).unwrap();
    assert!(!json.contains("BlockExpression"));
    assert!(
        compiled
            .ir
            .functions
            .iter()
            .any(|f| f.name.contains("global_init_"))
    );
}

#[test]
fn global_numeric_control_and_fixed_array_calls_have_legal_function_storage() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/global-force-inline.slang"
    ));
    assert!(
        compiled
            .ir
            .functions
            .iter()
            .all(|f| !matches!(f.return_type, slang_sema::Type::Array(..)))
    );
    assert!(
        !serde_json::to_string(&compiled.ir)
            .unwrap()
            .contains("BlockExpression")
    );
}

#[test]
fn imported_constructors_keep_type_ownership_and_private_field_scope() {
    let includes = MemoryIncludes { files: [("lib.slang".into(), "module lib;public struct Item { internal int value; [ForceInline] public __init(int x) { value=x+2; } }; public int read(Item x) {return x.value;}".into())].into() };
    let compiled = compile_source(
        "main.slang",
        "import lib;static int value=99;int caller(){Item x=Item(3);return read(x);}",
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    let artifact = compiled.artifact.unwrap();
    let caller = artifact.split("int caller()").last().unwrap();
    assert!(!caller.contains("constructor_Item("), "{caller}");
    assert!(caller.contains("_slang_module_"), "{caller}");
}

#[test]
fn constructor_return_and_argument_errors_are_source_diagnostics() {
    for (source, code) in [
        (
            include_str!("../../../tests/conformance/negative/user-constructor-arity.slang"),
            "T014",
        ),
        (
            include_str!(
                "../../../tests/conformance/negative/user-constructor-default-missing.slang"
            ),
            "T014",
        ),
        (
            include_str!("../../../tests/conformance/negative/user-constructor-return.slang"),
            "T008",
        ),
        (
            include_str!("../../../tests/conformance/negative/global-const-runtime-call.slang"),
            "T031",
        ),
        (
            include_str!("../../../tests/conformance/negative/global-const-runtime-value.slang"),
            "T031",
        ),
    ] {
        let error = compile_source(
            "invalid.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(error.diagnostics.iter().any(|d| d.code == code), "{error}");
    }
}

#[test]
fn unwritten_constructor_fields_stay_uninitialized() {
    let compiled = compile("struct S{int a,b;__init(int x){a=x;}};S caller(){return S(2);}");
    let constructor = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name.contains("constructor_S"))
        .unwrap();
    let body = serde_json::to_string(&constructor.body).unwrap();
    assert!(body.contains("\"initializer\":null"), "{body}");
    assert!(!body.contains("\"name\":\"b\""), "{body}");
}
