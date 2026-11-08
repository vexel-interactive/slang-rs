use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, Target, compile_source};
use slang_sema::{Scalar, Type};

fn compile(source: &str, options: CompileOptions) -> Compilation {
    compile_source(
        "native16.slang",
        source,
        &options,
        &MemoryIncludes::default(),
    )
    .unwrap()
}

#[test]
fn native_storage_is_selected_by_flag_profile_and_integer_requirements() {
    let shader = "half value;[shader(\"compute\")][numthreads(1,1,1)]void main(){}";
    for (options, native, profile) in [
        (CompileOptions::default(), false, "cs_6_0"),
        (
            CompileOptions {
                native_16bit_types: true,
                ..CompileOptions::default()
            },
            true,
            "cs_6_2",
        ),
        (
            CompileOptions {
                profile: Some("cs_6_2".parse().unwrap()),
                ..CompileOptions::default()
            },
            true,
            "cs_6_2",
        ),
        (
            CompileOptions {
                native_16bit_types: true,
                profile: Some("cs_6_0".parse().unwrap()),
                check_only: true,
                ..CompileOptions::default()
            },
            true,
            "cs_6_2",
        ),
    ] {
        let checked = options.check_only;
        let compiled = compile(shader, options);
        assert_eq!(compiled.reflection.native_16bit_types, native);
        assert_eq!(compiled.ir.native_16bit_types, native);
        assert_eq!(
            compiled.reflection.uniform_types["half"].size,
            if native { 2 } else { 4 }
        );
        assert_eq!(
            compiled.reflection.entry_points[0]
                .target_profile
                .as_deref(),
            Some(profile)
        );
        assert_eq!(
            compiled.reflection.target_compiler_args,
            if native {
                vec!["-enable-16bit-types"]
            } else {
                vec![]
            }
        );
        assert_eq!(compiled.artifact.is_none(), checked);
    }
    let compiled = compile(
        "uint16_t value;[shader(\"compute\")][numthreads(1,1,1)]void main(){}",
        CompileOptions::default(),
    );
    assert!(compiled.reflection.native_16bit_types);
    assert_eq!(
        compiled.reflection.entry_points[0]
            .target_profile
            .as_deref(),
        Some("cs_6_2")
    );
}

#[test]
fn all_narrow_alias_shapes_are_distinct_and_emit_hlsl_templates() {
    for (name, scalar) in [("int16_t", Scalar::Int16), ("uint16_t", Scalar::Uint16)] {
        assert_eq!(Type::builtin(name), Some(Type::Scalar(scalar)));
        let mut source = format!("{name} scalar({name} x){{return x;}}");
        for width in 1..=4 {
            let ty = format!("{name}{width}");
            assert_eq!(Type::builtin(&ty), Some(Type::Vector(scalar, width)));
            source.push_str(&format!("{ty} v{width}({ty} x){{return x;}}"));
        }
        for rows in 1..=4 {
            for columns in 1..=4 {
                let ty = format!("{name}{rows}x{columns}");
                assert_eq!(
                    Type::builtin(&ty),
                    Some(Type::Matrix(scalar, rows, columns))
                );
                source.push_str(&format!("{ty} m{rows}{columns}({ty} x){{return x;}}"));
            }
        }
        let compiled = compile(&source, CompileOptions::default());
        let code = compiled.artifact.unwrap();
        assert!(code.contains(&format!("vector<{name}, 4>")), "{code}");
        assert!(code.contains(&format!("matrix<{name}, 4, 4>")), "{code}");
    }
}

#[test]
fn narrow_globals_truncate_and_preserve_signed_constants_and_dimensions() {
    let compiled = compile(
        "static const int16_t negative=int16_t(65535);static const uint16_t zero=uint16_t(65536);static const int16_t minimum=int16_t(32767)+int16_t(1);static const uint16_t count=uint16_t(2);uint f(){int16_t values[count]={negative,minimum};return uint(values[0])+zero;}",
        CompileOptions::default(),
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("int16_t negative = int16_t(-1);"), "{code}");
    assert!(code.contains("uint16_t zero = uint16_t(0u);"), "{code}");
    assert!(
        code.contains("int16_t minimum = int16_t(-32768);"),
        "{code}"
    );
    assert!(code.contains("int16_t values[2]"), "{code}");
}

#[test]
fn native_numeric_storage_packs_two_byte_components_and_register_boundaries() {
    let compiled = compile(
        "struct Params{float lead;int16_t signedValue;uint16_t2 pair;half halfValue;};ConstantBuffer<Params> params;struct Arrays{half scalar;half values[2];half tail;row_major half2x3 rows;half after;};ConstantBuffer<Arrays> arrays;",
        CompileOptions::default(),
    );
    let p = &compiled.reflection.uniform_types["Params"];
    assert_eq!(p.size, 12);
    assert_eq!(
        p.fields.iter().map(|f| f.offset).collect::<Vec<_>>(),
        [0, 4, 6, 10]
    );
    let arrays = &compiled.reflection.uniform_types["Arrays"];
    assert_eq!(
        arrays.fields.iter().map(|f| f.offset).collect::<Vec<_>>(),
        [0, 16, 34, 48, 70]
    );
    assert_eq!(arrays.size, 72);
    let rows = &compiled.reflection.uniform_types["row_major half2x3"];
    assert_eq!((rows.size, rows.matrix_stride), (22, Some(16)));
    assert_eq!(
        compiled.reflection.uniform_types["half[2]"].array_stride,
        Some(16)
    );
}

#[test]
fn packed_uniforms_keep_subcomponent_offsets_through_value_reconstruction() {
    let compiled = compile(
        "struct P{half anchor:packoffset(c1);half value;half2 pair;};ConstantBuffer<P> params;half f(){return params.value+params.pair.x;}",
        CompileOptions {
            native_16bit_types: true,
            ..CompileOptions::default()
        },
    );
    let p = &compiled.reflection.uniform_types["P"];
    assert_eq!(
        p.fields.iter().map(|f| f.offset).collect::<Vec<_>>(),
        [16, 18, 20]
    );
    assert_eq!(p.size, 24);
    let code = compiled.artifact.unwrap();
    assert!(code.contains("half _padding_0_0;"), "{code}");
    assert!(code.contains("half _value_0;"), "{code}");
    assert!(code.contains("._value_0"), "{code}");
}

#[test]
fn rejected_narrow_cases_keep_target_and_location_diagnostics() {
    for source in [
        "void f(){uint16_t x;InterlockedAdd(x,uint16_t(1));}",
        "uint f(int16_t2x2 x){return countbits(x);}",
        "void f(){int16_t x;switch(x){case -1:break;case 65535:break;}}",
    ] {
        assert!(
            compile_source(
                "bad.slang",
                source,
                &CompileOptions::default(),
                &MemoryIncludes::default()
            )
            .is_err(),
            "{source}"
        );
    }
    let error = compile_source(
        "bad.slang",
        "half value;",
        &CompileOptions {
            native_16bit_types: true,
            target: Target::Spirv,
            check_only: true,
            ..CompileOptions::default()
        },
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "C004"));
}

#[test]
fn native_column_matrices_follow_source_register_alignment() {
    let compiled = compile(
        "struct P{half lead;half3x1 matrix;half tail;};ConstantBuffer<P> params;",
        CompileOptions {
            native_16bit_types: true,
            ..CompileOptions::default()
        },
    );
    let p = &compiled.reflection.uniform_types["P"];
    assert_eq!(
        p.fields.iter().map(|f| f.offset).collect::<Vec<_>>(),
        [0, 16, 22]
    );
    assert_eq!(p.size, 24);
    assert_eq!(compiled.reflection.uniform_types["half3x1"].alignment, 16);
}

#[test]
fn narrow_scalar_shift_saturation_preserves_wide_counts_and_input_evaluation() {
    let compiled = compile(
        "static const int16_t Left=int16_t(1)<<40;static const int16_t Right=int16_t(-1)>>40;uint64_t next(){return 40ULL;}int16_t f(){return int16_t(1)<<next();}",
        CompileOptions::default(),
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("Left = int16_t(0)"), "{code}");
    assert!(code.contains("Right = int16_t(-1)"), "{code}");
    assert!(code.contains("min(next(), 16u)"), "{code}");
    assert_eq!(code.matches("min(next()").count(), 1);
}
