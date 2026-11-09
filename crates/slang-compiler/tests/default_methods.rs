use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "defaults.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn default_methods_call_the_selected_requirement_and_preserve_static_receivers() {
    compile(
        "interface I{int read(int x);int run(){return read(1.0);}static int value(){return 100;}static int twice(){return value()*2;}}struct Item:I{int read(int x){return 3;}int read(float x){return 7;}}int read<T:I>(T x){return x.run();}int run(){Item item;return read(item)+Item::twice();}",
    );
}

#[test]
fn inherited_defaults_preserve_mutating_self_and_constructor_calls() {
    compile(
        "interface Base{This combine(This other);[mutating]void update(This other){this=combine(other);}}interface Derived:Base{}struct Value:Derived{int value;__init(int input){value=input;}This combine(This other){return This(value+other.value);}}T combine<T:Base>(T left,T right){left.update(right);return left;}int run(){return combine(Value(20),Value(22)).value;}",
    );
}

#[test]
fn generic_interface_defaults_keep_owner_and_helper_arguments() {
    compile(
        "T zero<T>(){return T();}interface I<T>{T get(T input=zero<T>());T read(){return get();}static T fallback(){return zero<T>();}}struct Item:I<int>{int get(int input){return input+4;}}int run(){Item item;return item.read()+Item::fallback();}",
    );
}

#[test]
fn unused_bodies_are_checked_without_exposing_concrete_members() {
    for source in [
        "interface I{int read(){return unknown;}}int run(){return 0;}",
        "interface I{int read(){return this.value;}}struct Item:I{int value;}int run(){return 0;}",
        "interface I{int read(){return hidden();}}struct Item:I{int hidden(){return 5;}}int run(){return 0;}",
        "interface I{[mutating]void change();void run(){change();}}int run(){return 0;}",
    ] {
        assert!(
            compile_source(
                "defaults.slang",
                source,
                &CompileOptions::default(),
                &MemoryIncludes::default()
            )
            .is_err(),
            "{source}"
        );
    }
}

#[test]
fn dormant_defaults_do_not_activate_concrete_helpers() {
    let compiled = compile(
        "T zero<T>(){return T();}interface I<T>{T read(){return zero<T>();}}T read<T,U:I<T>>(U input){return input.read();}int run(){return 0;}",
    );
    assert_eq!(compiled.ir.functions.len(), 1);
    assert_eq!(compiled.ir.functions[0].name, "run");
}

#[test]
fn imported_default_bodies_keep_globals_and_private_witness_access() {
    let includes=MemoryIncludes{files:[("lib.slang".into(),"module lib;int offset(){return 3;}public interface I{int get();int read(){return get()+offset();}}public int invoke<T:I>(T value){return value.read();}".into())].into()};
    compile_source("main.slang","module main;import lib;struct Item:I{int get(){return 4;}}int run(){Item item;int offset=100;return invoke(item);}",&CompileOptions::default(),&includes).unwrap();
}

#[test]
fn explicit_overrides_replace_defaults_and_require_a_matching_signature() {
    compile(
        "interface I{int read(){return 3;}}struct Item:I{override int read(){return 7;}}int read<T:I>(T item){return item.read();}int run(){Item item;return read(item);}",
    );
    for source in [
        "interface I{int read(){return 3;}}struct Item:I{int read(){return 7;}}int run(){return 0;}",
        "interface I{int read(){return 3;}}struct Item:I{override int read(int x){return x;}}int run(){return 0;}",
        "struct Item{override int read(){return 7;}}int run(){return 0;}",
        "override int run(){return 0;}",
    ] {
        let error = compile_source(
            "defaults.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == "T032"),
            "{error}"
        );
    }
}

#[test]
fn multiple_concrete_owners_keep_the_same_opaque_body_selection() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/interface-default-methods.slang"
    ));
    assert!(!compiled.artifact.unwrap().contains("override"));
    compile(include_str!(
        "../../../tests/conformance/positive/interface-default-inherited.slang"
    ));
}

#[test]
fn default_body_bindings_survive_serialization_and_rechecking() {
    let source = "interface I{int get(int input=3);int read(){return get();}}struct Item:I{int get(int input){return input;}}int run(){Item item;return item.read();}";
    let mut sources = slang_source::SourceMap::default();
    let root = sources.add("defaults.slang", source);
    let tokens = slang_preprocessor::preprocess(
        &mut sources,
        root,
        &MemoryIncludes::default(),
        &slang_preprocessor::Options::default(),
    )
    .tokens;
    let checked = slang_sema::check(slang_syntax::parse(&tokens).unwrap()).unwrap();
    assert!(!checked.module.constraint_calls.is_empty());
    serde_json::to_string(&checked.module).unwrap();
    let rechecked = slang_sema::check(checked.module.clone()).unwrap();
    assert_eq!(checked.call_names, rechecked.call_names);
    assert_eq!(
        serde_json::to_string(&slang_ir::lower(&checked).unwrap()).unwrap(),
        serde_json::to_string(&slang_ir::lower(&rechecked).unwrap()).unwrap()
    );
}
