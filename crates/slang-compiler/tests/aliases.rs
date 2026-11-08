use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};
use slang_sema::{Scalar, Type};

fn compile(source: &str, files: &[(&str, &str)]) -> Compilation {
    compile_source(
        "alias.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes {
            files: files
                .iter()
                .map(|(p, s)| (p.to_string(), s.to_string()))
                .collect(),
        },
    )
    .unwrap()
}

#[test]
fn numeric_aliases_resolve_constructors_overloads_and_matrix_shapes() {
    let compiled = compile(
        "typedef float Number;typealias Vector=float2;typealias Matrix=float2x3;
        Number read(Number x){return x;}int read(int x){return x;}
        Number run(){Vector v=Vector(1);Matrix m=Matrix(2);return read(Number(1))+v.x+m[0][0];}",
        &[],
    );
    let run = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name == "run")
        .unwrap();
    assert_eq!(run.return_type, Type::Scalar(Scalar::Float));
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float2 v = ((float2)1)"));
    assert!(code.contains("float2x3 m = ((float2x3)2)"));
    assert!(!code.contains("typedef"));
    assert!(!code.contains("typealias"));
}

#[test]
fn core_fixed_width_aliases_cover_every_supported_numeric_shape() {
    for (alias, scalar) in [
        ("float16_t", Scalar::Half),
        ("float32_t", Scalar::Float),
        ("float64_t", Scalar::Double),
        ("int32_t", Scalar::Int),
        ("uint32_t", Scalar::Uint),
    ] {
        assert_eq!(Type::builtin(alias), Some(Type::Scalar(scalar)));
        for width in 1..=4 {
            let name = format!("{alias}{width}");
            assert_eq!(Type::builtin(&name), Some(Type::Vector(scalar, width)));
            let compiled = compile(
                &format!("{name} f({name} value){{return {name}(value);}}"),
                &[],
            );
            assert_eq!(
                compiled.ir.functions[0].return_type,
                Type::Vector(scalar, width)
            );
        }
        for rows in 1..=4 {
            for columns in 1..=4 {
                let name = format!("{alias}{rows}x{columns}");
                let ty = Type::Matrix(scalar, rows, columns);
                assert_eq!(Type::builtin(&name), Some(ty.clone()));
                let compiled = compile(
                    &format!("{name} f({name} value){{return {name}(value);}}"),
                    &[],
                );
                assert_eq!(compiled.ir.functions[0].return_type, ty);
            }
        }
    }
}

#[test]
fn float16_alias_uses_existing_half_identity_and_legacy_hlsl_storage() {
    let compiled = compile(
        "uint choose(half x){return 1;}uint choose(float x){return 2;}float16_t value;uint f(){return choose(float16_t(1));}",
        &[],
    );
    assert_eq!(compiled.reflection.globals[0].type_name, "half");
    assert_eq!(compiled.reflection.uniform_types["half"].size, 4);
    let code = compiled.artifact.unwrap();
    assert!(code.contains("half value;"), "{code}");
    assert!(code.contains("choose__overload_0"), "{code}");
    assert!(!code.contains("float16_t"), "{code}");
    let error = compile_source(
        "duplicate.slang",
        "float f(half x){return x;}float f(float16_t x){return x;}",
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "T003"));
}

#[test]
fn aliases_support_forward_references_structs_and_resources() {
    let compiled = compile(
        "typealias Input=StructuredBuffer<Element>;typealias Element=Pair;
        struct Pair{float value;};Input data;typealias Out=RWTexture2D<float>;
        Out output;float run(){Element pair=data[0];output[int2(0)]=pair.value;return pair.value;}",
        &[],
    );
    assert_eq!(compiled.ir.structures.len(), 1);
    assert_eq!(
        compiled.reflection.globals[0].type_name,
        "StructuredBuffer<Pair>"
    );
    assert_eq!(
        compiled.reflection.globals[1].type_name,
        "RWTexture2D<float>"
    );
}

#[test]
fn public_aliases_link_without_exposing_internal_structs() {
    let compiled = compile(
        "import lib;Number run(){return Number(2);}",
        &[(
            "lib.slang",
            "module lib;typealias Hidden=float;public typealias Number=Hidden;",
        )],
    );
    assert_eq!(
        compiled.ir.functions[0].return_type,
        Type::Scalar(Scalar::Float)
    );
    for library in [
        "module lib;struct Hidden{float value;};public typealias Exposed=Hidden;",
        "module lib;typealias Hidden=float;",
    ] {
        let error = compile_source(
            "main.slang",
            "import lib;Hidden run(){return 1;}",
            &CompileOptions::default(),
            &MemoryIncludes {
                files: [("lib.slang".into(), library.into())].into(),
            },
        )
        .unwrap_err();
        assert!(
            error
                .diagnostics
                .iter()
                .any(|d| matches!(d.code.as_str(), "T018" | "T002"))
        );
    }
}

#[test]
fn array_alias_dimensions_and_index_bounds_follow_declarator_order() {
    let compiled = compile(
        "typedef float Row[2];typealias Grid=Row[3];
        typedef int Leading[2][3];typedef int[2][3] Reversed;
        float run(){Grid grid;grid[2][1]=1;Leading a;Reversed b;
            a[1][2]=2;b[2][1]=3;return grid[2][1]+float(a[1][2]+b[2][1]);}",
        &[],
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float grid[3][2]"));
    assert!(code.contains("int a[2][3]"));
    assert!(code.contains("int b[3][2]"));
    let error = compile_source(
        "bad.slang",
        "typedef float Row[2];typealias Grid=Row[3];void run(){Grid g;g[3][0]=1;}",
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "T013"));
}

#[test]
fn alias_arrays_preserve_nested_reflection_and_parameter_declarations() {
    let compiled=compile("typealias Grid=float[2][3];Grid global;
        float first(Grid data){return data[0][0];}float run(){Grid data;data[0][0]=2;return first(data);}", &[]);
    assert_eq!(compiled.reflection.globals[0].type_name, "float[3][2]");
    assert!(
        compiled
            .artifact
            .unwrap()
            .contains("float first(float data[3][2])")
    );
}

#[test]
fn alias_cycles_duplicate_types_and_unsupported_uses_fail() {
    for (source, code) in [
        ("typealias A=B;typealias B=A;", "T002"),
        ("typedef float A;typealias A=float;", "T003"),
        ("typealias Missing=Unknown;", "T002"),
        ("typealias Number=float;float Number(){return 1;}", "T003"),
        ("typealias A=float[2];StructuredBuffer<A> data;", "T002"),
        ("typealias Generic<T>=T;", "S001"),
        ("typedef float *Pointer;", "S001"),
        ("typedef float Unsized[];", "S001"),
    ] {
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
}

#[test]
fn fixed_width_scalar_names_lower_to_native_scalar_types() {
    let compiled=compile("typedef int32_t Index;typealias Value=float32_t;
        RWStructuredBuffer<Value> output;
        void run(){Index i=(int32_t)1.0;uint32_t u=uint32_t(i);float64_t d=float64_t(u);output[i]=float32_t(d);}", &[]);
    assert_eq!(
        compiled.reflection.globals[0].type_name,
        "RWStructuredBuffer<float>"
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("double d = double(u)"));
    assert!(!code.contains("int32_t"));
    assert!(!code.contains("float32_t"));
}
