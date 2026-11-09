use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> Compilation {
    compile_source(
        "interfaces.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn constrained_functions_use_instance_static_and_mutating_requirements() {
    let compiled = compile(
        "interface I { int read(); static int base(); [mutating] void set(int value); }struct Item : I { int value; int read(){return value;}static int base(){return 2;}[mutating]void set(int input){value=input;} };int read<T:I>(T input){return input.read()+T::base();}void set<T:I>(inout T input,int value){input.set(value);}int run(){Item item={3};set(item,7);return read(item)+read<Item>(item);}",
    );
    assert!(!compiled.artifact.unwrap().contains("parameter_"));
    assert!(compiled.ir.structures.iter().all(|s| s.name != "I"));
}

#[test]
fn inherited_diamond_and_multiple_constraints_preserve_self_types() {
    compile(
        "interface Root{This copy();}interface Left:Root{int left();}interface Right:Root{int right();}interface Both:Left,Right{}struct Item:Both{int value;This copy(){return this;}int left(){return value;}int right(){return 2;}};int read<T:Left & Right>(T input){return input.copy().left()+input.right();}int run(){Item item={3};return read(item);}",
    );
}

#[test]
fn generic_owners_methods_and_constructors_enforce_constraints() {
    compile(
        "interface I{int read();}struct Item:I{int value;int read(){return value;}};struct Box<T:I>{T item;int read(){return item.read();}};struct Runner{int run<T:I>(T item){return item.read();}};struct Result{int value;__init<T:I>(T item){value=item.read();}};int run(){Item item={4};Box<Item> box={item};Runner runner;Result result=Result(item);return box.read()+runner.run(item)+result.value;}",
    );
}

#[test]
fn generic_interface_arguments_and_dependent_constraints_are_canonical() {
    compile(
        "interface Read<Value>{Value get();}struct Item:Read<int>{int value;int get(){return value;}};U read<U,T:Read<U>>(T item,U ignored){return item.get();}struct Box<T:Read<int>>{T item;int get(){return item.get();}};int run(){Item item={5};Box<Item> box={item};return read(item,0)+box.get();}",
    );
    assert_rejected(
        "interface Read<Value>{Value get();}struct Item:Read<float>{int get(){return 3;}};",
        Some("T032"),
    );
    assert_rejected(
        "interface Read<Value>{Value get();}U read<T:Read<U>,U>(T item,U ignored){return item.get();}",
        Some("T032"),
    );
}

#[test]
fn constructor_and_output_requirements_are_checked_on_opaque_owners() {
    compile(
        "interface Make{__init(int value);void read(out int value);}struct Item:Make{int value;__init(int input){value=input;}void read(out int output){output=value;}};int read<T:Make>(int input){T item=T(input);int output;item.read(output);return output;}int run(){return read<Item>(7);}",
    );
    assert_rejected(
        "interface I{}int read<T:I>(T item){return item.read();}",
        Some("T014"),
    );
}

#[test]
fn nominal_constraints_filter_nonconforming_overloads() {
    compile(
        "interface I{}interface J{}struct A:I{};struct B:J{};int choose<T:I>(T item){return 3;}int choose<T:J>(T item){return 5;}int run(){A a;B b;return choose(a)+choose(b)+choose<A>(a);}",
    );
    compile(
        "interface I{int read();}struct Item:I{int read(){return 4;}};int choose<T:I>(T item){return item.read();}int choose(int item){return item;}int run(){Item item;return choose(item)+choose(3);}",
    );
    for source in [
        "interface I{int read();}struct Item{int read(){return 4;}};int read<T:I>(T item){return item.read();}int run(){Item item;return read(item);}",
        "interface I{int read();}int read<T:I>(T item){return item.read();}int run(){return read<int>(1);}",
        "interface I{}struct Box<T:I>{T value;}Box<int> item;",
    ] {
        assert_rejected(source, None);
    }
}

#[test]
fn invalid_conformance_and_opaque_bodies_are_rejected() {
    for source in [
        "interface I{static int read();}struct Item:I{int read(){return 4;}};",
        "interface I{int read();}struct Item:I{static int read(){return 4;}};",
        "interface I{int read();}struct Item:I{float read(){return 4;}};",
        "interface I{void get(out int item);}struct Item:I{void get(inout int item){item=4;}};",
        "interface I{int read();}struct Item:I{[mutating]int read(){return 4;}};",
        "interface I{int read();}struct Item:I{};",
        "interface I:J{int read();}interface J:I{}struct Item:I{int read(){return 5;}};int read<T:J>(T item){return item.read();}",
        "struct Base{}struct Item:Base{};",
        "int read<T:int>(T input){return input;}",
    ] {
        assert_rejected(source, Some("T032"));
    }
    assert_rejected(
        "interface I{int read();}int read<T:I>(T item){return item.missing();}",
        Some("T014"),
    );
    assert_rejected(
        "interface I{[mutating]void change();}void change<T:I>(const T item){item.change();}",
        Some("T015"),
    );
}

#[test]
fn imported_constraints_resolve_requirements_and_private_caller_types() {
    let includes = MemoryIncludes { files: [("lib.slang".into(), "module lib;public interface I{int read();}public int read<T:I>(T item){return item.read();}".into())].into() };
    compile_source("main.slang", "module main;import lib;struct Item:I{int value;int read(){return value;}};int run(){Item item={7};return read(item);}", &CompileOptions::default(), &includes).unwrap();
    let includes = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            include_str!("../../../tests/conformance/positive/interface-module/lib.slang").into(),
        )]
        .into(),
    };
    compile_source(
        "main.slang",
        include_str!("../../../tests/conformance/positive/interface-module/main.slang"),
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    let includes = MemoryIncludes { files: [("lib.slang".into(), "module lib;public interface I{int read();}public struct Item:I{int read(){return 7;}}public int read<T:I>(T item){return item.read();}".into())].into() };
    compile_source(
        "main.slang",
        "module main;import lib;int run(){Item item;return read(item);}",
        &CompileOptions::default(),
        &includes,
    )
    .unwrap();
    let error = compile_source(
        "main.slang",
        "module main;import lib;int run(){Item item;return item.read();}",
        &CompileOptions::default(),
        &includes,
    )
    .unwrap_err();
    assert!(
        error.diagnostics.iter().any(|d| d.code == "T014"),
        "{error}"
    );
}

#[test]
fn unsupported_interface_storage_is_diagnosed_before_lowering() {
    assert_rejected("interface I{Missing read();}", Some("T002"));
    assert_rejected("interface I{void read(void input);}", Some("T002"));
    assert_rejected("interface I{void read(out inout int input);}", Some("T001"));
    for source in [
        "interface I{}I input;",
        "interface I{}struct Item{I input;};",
        "interface I{}I read(){return I();}",
        "interface I{}void read(I input){}",
    ] {
        assert_rejected(source, Some("T001"));
    }
}

#[test]
fn compute_fixtures_emit_concrete_hlsl() {
    for source in [
        include_str!("../../../tests/conformance/positive/interface-methods.slang"),
        include_str!("../../../tests/conformance/positive/interface-inheritance.slang"),
        include_str!("../../../tests/conformance/positive/interface-generic-owners.slang"),
        include_str!("../../../tests/conformance/positive/interface-generic-arguments.slang"),
        include_str!("../../../tests/conformance/positive/interface-constructor.slang"),
        include_str!("../../../tests/conformance/positive/interface-overloads.slang"),
    ] {
        assert!(!compile(source).artifact.unwrap().contains("parameter_"));
    }
}

fn assert_rejected(source: &str, code: Option<&str>) {
    let error = compile_source(
        "interfaces.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    if let Some(code) = code {
        assert!(
            error.diagnostics.iter().any(|d| d.code == code),
            "{source}: {error}"
        );
    }
}
