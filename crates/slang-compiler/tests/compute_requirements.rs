use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

const COMPUTE: &str = "[shader(\"compute\")][numthreads(1,1,1)]void main()";
const FRAGMENT: &str = "[shader(\"fragment\")]float4 main():SV_Target";

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "stages.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

fn fails(source: &str) -> slang_compiler::CompileError {
    let error = compile_source(
        "bad.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(
        error.diagnostics.iter().any(|d| d.code == "T023"),
        "{source}: {error}"
    );
    error
}

#[test]
fn compute_accepts_all_memory_barriers_with_direct_hlsl_calls() {
    let names = [
        "GroupMemoryBarrier",
        "DeviceMemoryBarrier",
        "AllMemoryBarrier",
        "GroupMemoryBarrierWithGroupSync",
        "DeviceMemoryBarrierWithGroupSync",
        "AllMemoryBarrierWithGroupSync",
    ];
    let calls = names.map(|name| format!("{name}();")).join("");
    let code = compile(&format!("{COMPUTE}{{{calls}}}")).artifact.unwrap();
    for name in names {
        assert!(code.contains(&format!("{name}();")));
    }
    compile(&format!(
        "{FRAGMENT}{{DeviceMemoryBarrier();return float4(1);}}"
    ));
    compile(
        "[shader(\"vertex\")]float4 main():SV_Position{DeviceMemoryBarrier();return float4(1);}",
    );
}

#[test]
fn group_barriers_require_compute_in_fragment_and_vertex_entries() {
    for name in [
        "GroupMemoryBarrier",
        "AllMemoryBarrier",
        "GroupMemoryBarrierWithGroupSync",
        "DeviceMemoryBarrierWithGroupSync",
        "AllMemoryBarrierWithGroupSync",
    ] {
        for entry in [FRAGMENT, "[shader(\"vertex\")]float4 main():SV_Position"] {
            let error = fails(&format!("{entry}{{{name}();return float4(1);}}"));
            assert!(
                error
                    .diagnostics
                    .iter()
                    .any(|d| d.message.contains(name)
                        && d.message.contains("requires compute stage"))
            );
        }
    }
}

#[test]
fn compute_requirements_follow_calls_used_globals_and_omitted_defaults() {
    for source in [
        format!(
            "void barrier(){{GroupMemoryBarrierWithGroupSync();}}void relay(){{barrier();}}{FRAGMENT}{{relay();return float4(1);}}"
        ),
        format!(
            "float read(){{AllMemoryBarrier();return 1;}}float sample(float x=read()){{return x;}}{FRAGMENT}{{return float4(sample());}}"
        ),
        format!(
            "float read(){{GroupMemoryBarrier();return 1;}}struct P{{float x=read();}};{FRAGMENT}{{P p={{}};return float4(p.x);}}"
        ),
        format!(
            "float read(){{DeviceMemoryBarrierWithGroupSync();return 1;}}static float value=read();{FRAGMENT}{{return float4(value);}}"
        ),
        format!("groupshared int shared;{FRAGMENT}{{return float4(shared);}}"),
        format!("groupshared int shared[2];{FRAGMENT}{{return float4(shared[0]);}}"),
    ] {
        fails(&source);
    }
}

#[test]
fn unused_overloads_defaults_and_shadowed_globals_do_not_add_requirements() {
    for source in [
        format!(
            "void select(float x){{GroupMemoryBarrier();}}void select(int x){{}}{FRAGMENT}{{select(1);return float4(1);}}"
        ),
        format!(
            "void GroupMemoryBarrierWithGroupSync(){{}}{FRAGMENT}{{GroupMemoryBarrierWithGroupSync();return float4(1);}}"
        ),
        format!(
            "struct GroupMemoryBarrier{{float x;}};{FRAGMENT}{{GroupMemoryBarrier p=GroupMemoryBarrier(1);return float4(p.x);}}"
        ),
        format!(
            "float read(){{GroupMemoryBarrier();return 1;}}float sample(float x=read()){{return x;}}struct P{{float x=read();}};{FRAGMENT}{{P p=P(2);return float4(sample(3)+p.x);}}"
        ),
        format!("groupshared int value;{FRAGMENT}{{int value=1;return float4(value);}}"),
        format!("groupshared int value;{FRAGMENT}{{return float4(1);}}"),
    ] {
        compile(&source);
    }
}

#[test]
fn cyclic_graphs_merge_independent_fragment_and_compute_requirements() {
    let error = fails(
        "void a(){b();discard;}void b(){a();GroupMemoryBarrier();}[shader(\"vertex\")]float4 main():SV_Position{a();return float4(1);}",
    );
    let messages: Vec<_> = error
        .diagnostics
        .iter()
        .filter(|d| d.code == "T023")
        .map(|d| &d.message)
        .collect();
    assert_eq!(messages.len(), 2);
    assert!(
        messages
            .iter()
            .any(|m| m.contains("requires compute stage"))
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("requires fragment stage"))
    );
    fails(&format!(
        "void a(){{b();discard;}}void b(){{a();GroupMemoryBarrier();}}{COMPUTE}{{a();}}"
    ));
    fails(&format!(
        "void a(){{b();discard;}}void b(){{a();GroupMemoryBarrier();}}{FRAGMENT}{{a();return float4(1);}}"
    ));
}

#[test]
fn imported_requirements_keep_original_source_locations() {
    let files = MemoryIncludes {
        files: [(
            "shared.slang".into(),
            "module shared;groupshared uint value;public uint read(){return value;}".into(),
        )]
        .into(),
    };
    let error = compile_source(
        "main.slang",
        &format!("import shared;{FRAGMENT}{{return float4(read());}}"),
        &CompileOptions::default(),
        &files,
    )
    .unwrap_err();
    let diagnostic = error.diagnostics.iter().find(|d| d.code == "T023").unwrap();
    assert!(
        error
            .sources
            .get(diagnostic.span.source)
            .unwrap()
            .name
            .ends_with("shared.slang")
    );
    assert!(diagnostic.message.contains("groupshared"));
}
