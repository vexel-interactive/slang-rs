use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "witnesses.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn opaque_selection_retains_the_instance_witness_among_concrete_overloads() {
    let compiled = compile(
        "interface I{int read(int input);}struct Item:I{int read(int input){return 1;}int read(float input){return 2;}static int read(int input,int extra=0){return 3;}}int read<T:I>(T item){return item.read(1.0);}int run(){Item item;return read(item);}",
    );
    let function = compiled
        .ir
        .functions
        .iter()
        .find(|f| {
            f.name.contains("generic") && f.parameters.first().is_some_and(|p| p.name == "item")
        })
        .unwrap();
    let slang_ir::StmtKind::Block(statements) = &function.body.as_ref().unwrap().kind else {
        panic!("expected a function block");
    };
    let slang_ir::StmtKind::Return(Some(value)) = &statements[0].kind else {
        panic!("expected the constrained return");
    };
    let slang_ir::ExprKind::Call { name, arguments } = &value.kind else {
        panic!("expected a selected call");
    };
    let witness = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name == *name)
        .unwrap();
    assert_eq!(witness.parameters.len(), 2);
    assert_eq!(witness.parameters[0].name, "this");
    assert_eq!(
        witness.parameters[1].ty,
        slang_sema::Type::Scalar(slang_sema::Scalar::Int)
    );
    assert_eq!(arguments.len(), 2);
}

#[test]
fn owner_and_method_substitutions_retain_generic_interface_identity() {
    compile(include_str!(
        "../../../tests/conformance/positive/interface-witness-owners.slang"
    ));
    compile(include_str!(
        "../../../tests/conformance/positive/interface-witness-nested-arguments.slang"
    ));
}

#[test]
fn constructors_retain_required_overloads_and_default_construction() {
    compile(include_str!(
        "../../../tests/conformance/positive/interface-witness-constructors.slang"
    ));
}

#[test]
fn declaration_defaults_keep_generic_dependencies_outside_caller_scope() {
    compile(include_str!(
        "../../../tests/conformance/positive/interface-witness-defaults.slang"
    ));
    compile(include_str!(
        "../../../tests/conformance/positive/interface-witness-generic-defaults.slang"
    ));
}

#[test]
fn imported_defaults_and_private_witnesses_keep_their_declaration_module() {
    let includes = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            include_str!("../../../tests/conformance/positive/interface-witness-module/lib.slang")
                .into(),
        )]
        .into(),
    };
    compile_source(
        "main.slang",
        include_str!("../../../tests/conformance/positive/interface-witness-module/main.slang"),
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
}

#[test]
fn dormant_constraint_calls_do_not_activate_their_default_dependencies() {
    let compiled = compile(
        "T zero<T>(){return T();}interface I<Value>{int read(Value input=zero<Value>());}int read<T:I<int>>(T item){return item.read();}int run(){return 0;}",
    );
    assert_eq!(compiled.ir.functions.len(), 1);
    assert_eq!(compiled.ir.functions[0].name, "run");
}

#[test]
fn unused_interface_defaults_are_checked_without_panics() {
    for (source, code) in [
        (
            include_str!("../../../tests/conformance/negative/interface-default-unknown.slang"),
            "T011",
        ),
        (
            include_str!("../../../tests/conformance/negative/interface-default-output.slang"),
            "T022",
        ),
        (
            include_str!("../../../tests/conformance/negative/interface-default-trailing.slang"),
            "T022",
        ),
    ] {
        let error = compile_source(
            "witnesses.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(error.diagnostics.iter().any(|d| d.code == code), "{error}");
    }
}
