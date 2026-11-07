use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "threads.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

fn fails(source: &str, code: &str) {
    let error = compile_source(
        "bad.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(
        error.diagnostics.iter().any(|d| d.code == code),
        "{source}: {error}"
    );
}

#[test]
fn thread_dimensions_fold_typed_integer_expressions_and_literals() {
    let compiled = compile("[shader(\"compute\")][numthreads(1<<3,0x2u,(int)1.0)]void main(){}");
    assert_eq!(
        compiled.reflection.entry_points[0].thread_group_size,
        Some([8, 2, 1])
    );
    assert!(compiled.artifact.unwrap().contains("[numthreads(8, 2, 1)]"));
    let compiled = compile(
        "static const uint width=(0xffffffffu+9u);static const int height=uint(2);[shader(\"compute\")][numthreads(width,height,true)]void main(){}",
    );
    assert_eq!(
        compiled.reflection.entry_points[0].thread_group_size,
        Some([8, 2, 1])
    );
    let compiled = compile("[shader(\"compute\")][numthreads(4.5,1,1)]void main(){}");
    assert_eq!(
        compiled.reflection.entry_points[0].thread_group_size,
        Some([4, 1, 1])
    );
    let compiled =
        compile("[shader(\"compute\")][numthreads(4,(bool)1.5,bool(2.25))]void main(){}");
    assert_eq!(
        compiled.reflection.entry_points[0].thread_group_size,
        Some([4, 1, 1])
    );
}

#[test]
fn global_integer_constants_resolve_forward_dependencies_and_alias_constructors() {
    let compiled = compile(
        "typedef int Count;static const int width=height+2;static const Count height=depth+1;static const int depth={Count(1)};[shader(\"compute\")][numthreads(width,height,depth)]void main(){}",
    );
    assert_eq!(
        compiled.reflection.entry_points[0].thread_group_size,
        Some([4, 2, 1])
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("width = 4;"));
    assert!(code.contains("height = 2;"));
    assert!(code.contains("depth = 1;"));
    compile(
        "static const int label=next+1;static const int next=1;int run(int value){switch(value){case label:return 1;default:return 0;}}",
    );
    let compiled = compile(
        "[shader(\"compute\")][numthreads(width,1,1)]void main(){}static const int width=height*2;static const int height=2;",
    );
    assert_eq!(
        compiled.reflection.entry_points[0].thread_group_size,
        Some([4, 1, 1])
    );
}

#[test]
fn thread_attributes_resolve_before_parameters_shadow_globals() {
    let compiled = compile(
        "static const int width=4;[shader(\"compute\")][numthreads(width,1,1)]void main(uint width:SV_GroupIndex){}",
    );
    assert_eq!(
        compiled.reflection.entry_points[0].thread_group_size,
        Some([4, 1, 1])
    );
}

#[test]
fn imported_constants_preserve_their_declaration_visibility() {
    let files = MemoryIncludes {
        files: [(
            "sizes.slang".into(),
            "module sizes;public static const int width=hidden*2;static const int hidden=2;".into(),
        )]
        .into(),
    };
    let compiled = compile_source(
        "root.slang",
        "import sizes;[shader(\"compute\")][numthreads(width,1,1)]void main(){}",
        &CompileOptions::default(),
        &files,
    )
    .unwrap();
    assert_eq!(
        compiled.reflection.entry_points[0].thread_group_size,
        Some([4, 1, 1])
    );
}

#[test]
fn unresolved_or_invalid_dimensions_are_diagnostics() {
    for source in [
        "uint width;[shader(\"compute\")][numthreads(width,1,1)]void main(){}",
        "int read(){return 4;}[shader(\"compute\")][numthreads(read(),1,1)]void main(){}",
        "[shader(\"compute\")][numthreads(0,1,1)]void main(){}",
        "[shader(\"compute\")][numthreads(-1,1,1)]void main(){}",
        "[shader(\"compute\")][numthreads(uint2(4),1,1)]void main(){}",
        "[shader(\"compute\")][numthreads(32*64,1,1)]void main(){}",
        "[shader(\"compute\")][numthreads(32,64,1)]void main(){}",
        "[shader(\"compute\")][numthreads((bool)0.5,1,1)]void main(){}",
        "[shader(\"compute\")][numthreads((int)(1.5*2),1,1)]void main(){}",
    ] {
        fails(source, "T006");
    }
}

#[test]
fn constant_graphs_handle_long_chains_and_reject_cycles_without_recursion() {
    let mut source = "".to_owned();
    for index in 0..512 {
        source.push_str(&format!("static const int size{index}=size{};", index + 1));
    }
    source.push_str(
        "static const int size512=4;[shader(\"compute\")][numthreads(size0,1,1)]void main(){}",
    );
    assert_eq!(
        compile(&source).reflection.entry_points[0].thread_group_size,
        Some([4, 1, 1])
    );
    for source in [
        "static const int a=b;static const int b=a;[shader(\"compute\")][numthreads(a,1,1)]void main(){}",
        "static const int a=true?1:b;static const int b=a;void run(){}",
        "static const int a=a;void run(){}",
    ] {
        fails(source, "T026");
    }
}

#[test]
fn computed_thread_sizes_drive_derivative_quad_validation() {
    let compiled = compile(
        "static const int width=height*2;static const int height=2;[shader(\"compute\")][numthreads(width,1,1)]void main(){float value=ddx(1.0);}",
    );
    assert_eq!(
        compiled.reflection.entry_points[0]
            .target_profile
            .as_deref(),
        Some("cs_6_6")
    );
    fails(
        "static const int width=1+1;[shader(\"compute\")][numthreads(width,1,1)]void main(){float value=ddx(1.0);}",
        "T024",
    );
}

#[test]
fn integer_folding_keeps_fractional_global_values_and_arithmetic() {
    let compiled = compile(
        "static const float fractional=(float)1.5;static const int converted=(int)(fractional*2);int run(){return converted;}",
    );
    assert!(compiled.ir.globals.iter().all(|global| !matches!(
        global.initializer.as_ref().unwrap().kind,
        slang_ir::ExprKind::Integer(_)
    )));
    let code = compiled.artifact.unwrap();
    assert!(code.contains("((float)1.5)"));
    assert!(code.contains("fractional * 2"));
}
