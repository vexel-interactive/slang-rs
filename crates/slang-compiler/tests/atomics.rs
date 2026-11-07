use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "atomics.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn groupshared_integer_atomics_keep_direct_destinations() {
    let compiled = compile(include_str!("../../../examples/group-atomics.slang"));
    let code = compiled.artifact.unwrap();
    assert!(code.contains("InterlockedAdd(counter, 1u, previous);"));
    assert!(code.contains("groupshared uint counter;"));
    assert_eq!(
        compiled.reflection.entry_points[0].thread_group_size,
        Some([64, 1, 1])
    );
}

#[test]
fn rw_fields_textures_and_groupshared_arrays_are_atomic_locations() {
    let compiled = compile(
        "struct Counter{int value;};RWStructuredBuffer<Counter> data;RWTexture2D<uint> image;groupshared Counter shared;groupshared int slots[2];void run(){int previous;uint old;InterlockedAdd(data[0].value,2.5,previous);InterlockedAnd(data[0].value,3);InterlockedOr(image[int2(0)],1u,old);InterlockedXor(data[0].value,4,previous);InterlockedMin(shared.value,-1,previous);InterlockedMax(slots[0],7);InterlockedExchange(data[0].value,2);InterlockedCompareExchange(data[0].value,2,3,previous);InterlockedCompareStore(data[0].value,3,4);}",
    );
    assert_eq!(compiled.reflection.globals.len(), 4);
    let code = compiled.artifact.unwrap();
    assert!(code.contains("InterlockedAdd(((data)[0]).value, ((int)2.5), previous);"));
    assert!(code.contains("InterlockedMax((slots)[0], 7);"));
    assert!(code.contains("InterlockedExchange(((data)[0]).value, 2, _slang_"));
}

#[test]
fn atomic_locations_and_output_types_are_checked() {
    for (source, expected) in [
        ("void run(){int value=0;InterlockedAdd(value,1);}", "T015"),
        (
            "static int value;void run(){InterlockedAdd(value,1);}",
            "T015",
        ),
        (
            "groupshared int value;void run(){int value=0;InterlockedAdd(value,1);}",
            "T015",
        ),
        (
            "void run(inout int value){InterlockedAdd(value,1);}",
            "T015",
        ),
        (
            "StructuredBuffer<int> data;void run(){InterlockedAdd(data[0],1);}",
            "T015",
        ),
        (
            "groupshared int value;void run(){uint old;InterlockedAdd(value,1,old);}",
            "T014",
        ),
        (
            "groupshared int value;void run(){const int old=0;InterlockedAdd(value,1,old);}",
            "T015",
        ),
        (
            "groupshared float value;void run(){InterlockedAdd(value,1);}",
            "T014",
        ),
        (
            "groupshared int2 value;void run(){InterlockedAdd(value,1);}",
            "T014",
        ),
        (
            "groupshared int value;void run(){InterlockedCompareExchange(value,1,2);}",
            "T014",
        ),
    ] {
        let error = compile_source(
            "bad.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();
        assert!(
            error.diagnostics.iter().any(|d| d.code == expected),
            "{source}: {error}"
        );
    }
}

#[test]
fn imported_atomic_calls_cannot_be_captured_by_user_functions() {
    let includes = MemoryIncludes { files: [("lib.slang".into(), "module lib;groupshared uint value;public uint read(){InterlockedAdd(value,1u);return value;}".into())].into() };
    let compiled = compile_source("main.slang", "import lib;float InterlockedAdd(float x){return x+1;}float run(){return InterlockedAdd(2)+float(read());}", &CompileOptions::default(), &includes).unwrap();
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float _slang_InterlockedAdd(float x)"));
    assert!(code.contains("InterlockedAdd(_slang__slang_module_1_value, 1u);"));
}

#[test]
fn ignored_exchange_outputs_preserve_global_defaults_and_source_names() {
    let compiled = compile(
        "RWStructuredBuffer<uint> data;static uint _slang_atomic_ignored_0=7;static uint initial=(InterlockedExchange(data[0],1u),1u);struct Packet{uint value=(InterlockedExchange(data[1],2u),2u);};uint run(){Packet p=Packet();return initial+p.value+_slang_atomic_ignored_0;}",
    );
    assert_eq!(compiled.reflection.globals.len(), 3);
    assert_eq!(
        compiled
            .ir
            .globals
            .iter()
            .filter(|g| g.name.starts_with("_slang_atomic_ignored__"))
            .count(),
        2
    );
    let code = compiled.artifact.unwrap();
    assert!(
        code.find("static uint _slang__slang_atomic_ignored__0;")
            .unwrap()
            < code.find("static uint initial").unwrap()
    );
}
