use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> Compilation {
    compile_source(
        "inline.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn forced_calls_expand_before_ssa_while_ordinary_calls_remain_opaque() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/force-inline-values.slang"
    ));
    let code = compiled.artifact.unwrap();
    let main = code.split("void main()").last().unwrap();
    for name in [
        "narrowIdentity",
        "narrowAdd",
        "narrowChoose",
        "pair",
        "ignored",
        "modified",
        "locals",
        "nested",
        "positive",
        "splat",
        "matrixValue",
    ] {
        assert!(!main.contains(&format!("{name}(")), "{name}: {main}");
    }
    assert!(main.contains("ordinaryIdentity("), "{main}");
    assert!(main.contains("(x = uint16_t(0u))"), "{main}");
    assert!(main.contains("ordinary <<= 16"), "{main}");
}

#[test]
fn captures_are_assigned_inside_conditional_arms_and_all_arguments_evaluate_once() {
    let compiled = compile(
        "[ForceInline]int use(int a,int b){return a+a;}int f(bool flag){int i=1;int result=flag?use(i++,i++):use(i++,i++);return result+i;}",
    );
    let code = compiled.artifact.unwrap();
    let caller = code.split("int f(bool flag)").last().unwrap();
    assert_eq!(caller.matches("i++").count(), 4, "{caller}");
    assert!(caller.contains("flag ?"), "{caller}");
    assert!(!caller.contains("use("), "{caller}");
    assert!(caller.contains("_slang_inline_"), "{caller}");
}

#[test]
fn prototypes_overloads_aliases_and_nested_calls_keep_selected_targets() {
    let code = compile("typedef uint16_t Small;[ForceInline]Small select(Small x);Small select(Small x){return x;}[ForceInline]int select(int x){return x+1;}[ForceInline]Small nested(Small x){return select(x);}uint f(){Small x=nested(Small(1));x<<=16;return uint(x)+uint(select(2));}").artifact.unwrap();
    let caller = code.split("uint f()").last().unwrap();
    assert!(caller.contains("return (0u +"), "{caller}");
    assert!(!caller.contains("select("), "{caller}");
    assert!(!caller.contains("nested("), "{caller}");
}

#[test]
fn imported_functions_preserve_global_references_under_caller_shadowing() {
    let includes=MemoryIncludes{files:[("lib.slang".into(),"module lib;public static const int bias=4;[ForceInline]public int add(int x){return x+bias;}".into())].into()};
    let compiled = compile_source(
        "main.slang",
        "import lib;int f(){int bias=99;return add(2);}",
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    let code = compiled.artifact.unwrap();
    let caller = code.split("int f()").last().unwrap();
    assert!(!caller.contains("add("), "{caller}");
    assert!(caller.contains("6"), "{caller}");
}

#[test]
fn generated_capture_names_avoid_source_prefixes() {
    let code=compile("static int _slang_inline_0=9;[ForceInline]int id(int x){return x;}int f(int x){return id(x)+_slang_inline_0;}").artifact.unwrap();
    assert!(code.contains("_slang_inline__0"), "{code}");
}

#[test]
fn malformed_attributes_and_unsupported_body_or_storage_forms_report_limits() {
    for (source, code) in [
        ("[ForceInline(1)]int f(int x){return x;}", "T006"),
        (
            "[ForceInline]int f(inout int x){return x;}int g(){int x=1;return f(x);}",
            "I005",
        ),
        (
            "[ForceInline]int f(int x){return x;}static int value=f(1);",
            "I005",
        ),
    ] {
        let error = compile_source(
            "limits.slang",
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
}

#[test]
fn branches_loops_early_returns_and_lexical_scopes_expand_before_ssa() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/force-inline-control.slang"
    ));
    let json = serde_json::to_string(&compiled.ir).unwrap();
    assert!(
        !json.contains("BlockExpression"),
        "statement expressions escaped normalization"
    );
    for index in 0..compiled.ir.functions.len() {
        if let Some(graph) = slang_ir::flow::build_function(&compiled.ir, index) {
            graph.verify().unwrap();
        }
    }
    let code = compiled.artifact.unwrap();
    let main = code.split("void main()").last().unwrap();
    for name in [
        "choose",
        "scoped",
        "forReturn",
        "whileReturn",
        "doReturn",
        "switched",
        "change",
        "next",
        "isLess",
        "countStatic",
        "nestedFlow",
    ] {
        assert!(!main.contains(&format!("{name}(")), "{name}: {main}");
    }
}

#[test]
fn dynamic_lifted_conditions_preserve_continue_and_output_locations() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/force-inline-effects.slang"
    ));
    let json = serde_json::to_string(&compiled.ir).unwrap();
    assert!(!json.contains("BlockExpression"));
    for index in 0..compiled.ir.functions.len() {
        if let Some(graph) = slang_ir::flow::build_function(&compiled.ir, index) {
            graph.verify().unwrap();
        }
    }
    let code = compiled.artifact.unwrap();
    let main = code.split("void main(").last().unwrap();
    assert!(main.contains("continue;"), "{main}");
    assert!(main.contains("GetDimensions("), "{main}");
    assert!(main.contains("InterlockedAdd("), "{main}");
    assert!(!main.contains("step("), "{main}");
}

#[test]
fn recursive_inline_expansion_is_bounded() {
    for source in [
        "[ForceInline]int f(int x){return f(x);}",
        "[ForceInline]int f(int x){return g(x);}[ForceInline]int g(int x){return f(x);}",
    ] {
        let error = compile_source(
            "recursive.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error
                .diagnostics
                .iter()
                .any(|d| d.code == "I005" && d.message.contains("recursive")),
            "{error}"
        );
    }
}

#[test]
fn long_linear_bodies_do_not_create_deep_generated_expression_chains() {
    let mut source = "[ForceInline]int linear(int input){".to_owned();
    for index in 0..1000 {
        source.push_str(&format!("int value{index}=input+{index};"));
    }
    source.push_str("return value999;}int caller(int input){return linear(input);}");
    let code = compile(&source).artifact.unwrap();
    let caller = code.split("int caller(int input)").last().unwrap();
    assert!(!caller.contains("linear("), "{caller}");
    assert!(caller.contains("999"));
}

#[test]
fn exponential_inline_growth_reports_a_bounded_diagnostic() {
    let mut source = "[ForceInline]int f0(int x){return x;}".to_owned();
    for index in 1..18 {
        let previous = index - 1;
        source.push_str(&format!(
            "[ForceInline]int f{index}(int x){{return f{previous}(x)+f{previous}(x);}}"
        ));
    }
    let error = compile_source(
        "growth.slang",
        &source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(
        error
            .diagnostics
            .iter()
            .any(|d| d.code == "I005" && d.message.contains("size limit")),
        "{error}"
    );
}
