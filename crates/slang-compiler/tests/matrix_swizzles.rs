use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "swizzles.slang",
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
fn both_matrix_swizzle_conventions_cover_every_supported_component_type_and_shape() {
    for scalar in [
        Scalar::Bool,
        Scalar::Int,
        Scalar::Uint,
        Scalar::Half,
        Scalar::Float,
        Scalar::Double,
    ] {
        for rows in 2..=4 {
            for columns in 2..=4 {
                let matrix = Type::Matrix(scalar, rows, columns);
                let component = Type::Scalar(scalar);
                let mut source = format!("void run({matrix} m){{");
                for row in 0..rows {
                    for column in 0..columns {
                        source.push_str(&format!("{component} a{row}{column}=m._m{row}{column};{component} b{row}{column}=m._{}{};", row+1, column+1));
                    }
                }
                source.push('}');
                let compiled = compile(&source);
                let slang_ir::StmtKind::Block(body) =
                    &compiled.ir.functions[0].body.as_ref().unwrap().kind
                else {
                    panic!()
                };
                for statement in body {
                    let slang_ir::StmtKind::Variable(variable) = &statement.kind else {
                        panic!()
                    };
                    assert_eq!(variable.initializer.as_ref().unwrap().ty, component);
                }
            }
        }
    }
}

#[test]
fn matrix_swizzle_assignments_and_out_arguments_preserve_target_locations() {
    let compiled = compile(
        "void write(out float2 value){value=float2(3,4);}float4 run(){float2x3 m=float2x3(1);m._m00_m12=float2(2,3);m._12_21+=float2(1);write(m._m02_m11);return m._m00_m01_m10_m12;}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("(m)._m00_m12 = float2(2, 3)"));
    assert!(code.contains("(m)._12_21 += ((float2)1)"));
    assert!(code.contains("write((m)._m02_m11)"));
}

#[test]
fn repeated_matrix_components_are_readable_but_never_writable() {
    compile("float4 run(float2x3 m){return m._m00_m00_m12_m12;}");
    for source in [
        "void run(){float2x3 m=float2x3(1);m._m00_m00=float2(2);}",
        "void run(){float2x3 m=float2x3(1);m._11_11+=float2(2);}",
        "void write(out float2 v){}void run(){float2x3 m=float2x3(1);write(m._m00_m00);}",
        "void run(){float2x3 m=float2x3(1);m._11_11++;}",
        "void run(){float2x3 m=float2x3(1);float2 integral;modf(1.0,m._m00_m00);}",
    ] {
        fails(source, "T015");
    }
}

#[test]
fn matrix_selectors_check_bounds_length_digits_and_conventions() {
    for selector in [
        "_m22",
        "_31",
        "_m00_12",
        "_11_m00",
        "_00",
        "_m000",
        "m00",
        "_m00_",
        "_m00_m01_m02_m10_m11",
        "_m0x",
        "_",
        "_m44",
    ] {
        fails(
            &format!("void run(float2x3 m){{float4 value=float4(m.{selector});}}"),
            "T013",
        );
    }
}

#[test]
fn matrix_location_rules_follow_buffers_const_storage_and_atomic_components() {
    let compiled = compile(
        "RWStructuredBuffer<float2x3> data;groupshared int2x3 shared;void run(){float2 integral;modf(float2(1.25,2.5),data[0]._m00_m12);int old;InterlockedAdd(shared._m00,1,old);}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("modf(float2(1.25, 2.5), ((data)[0])._m00_m12)"));
    assert!(code.contains("InterlockedAdd((_slang_shared)._m00, 1, old)"));
    for source in [
        "void run(){const float2x3 m=float2x3(1);m._m00=2;}",
        "StructuredBuffer<float2x3> data;void run(){data[0]._m00=1;}",
        "float2x3 make(){return float2x3(1);}void run(){make()._11=2;}",
    ] {
        fails(source, "T015");
    }
}

#[test]
fn struct_fields_and_imported_matrix_access_keep_their_own_member_rules() {
    compile("struct P{float _m00;};float run(P p){return p._m00;}");
    let files = MemoryIncludes {
        files: [(
            "math.slang".into(),
            "module math;public float2 pair(float2x3 m){return m._11_23;}".into(),
        )]
        .into(),
    };
    let compiled = compile_source(
        "root.slang",
        "import math;float2 run(){return pair(float2x3(1));}",
        &CompileOptions::default(),
        &files,
    )
    .unwrap();
    assert!(compiled.artifact.unwrap().contains("(m)._11_23"));
}
