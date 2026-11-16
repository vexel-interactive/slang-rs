use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "cast-aliases.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_or_else(|error| panic!("{source}: {error}"))
}

#[test]
fn numeric_alias_casts_compile_all_scalar_vector_and_matrix_shapes() {
    for (name, scalar) in [
        ("bool", Scalar::Bool),
        ("int", Scalar::Int),
        ("uint", Scalar::Uint),
        ("half", Scalar::Half),
        ("float", Scalar::Float),
        ("double", Scalar::Double),
        ("int16_t", Scalar::Int16),
        ("uint16_t", Scalar::Uint16),
        ("int32_t", Scalar::Int),
        ("uint32_t", Scalar::Uint),
        ("int64_t", Scalar::Int64),
        ("uint64_t", Scalar::Uint64),
        ("float16_t", Scalar::Half),
        ("float32_t", Scalar::Float),
        ("float64_t", Scalar::Double),
    ] {
        let mut source = format!("{name} scalar(float x)\n{{\n    return ({name}) x;\n}}\n");
        let mut expected = vec![Type::Scalar(scalar)];

        for width in 1..=4 {
            source.push_str(&format!(
                "{name}{width} v{width}(float{width} x)\n{{\n    return ({name}{width}) x;\n}}\n"
            ));
            expected.push(Type::Vector(scalar, width));
        }

        for rows in 1..=4 {
            for columns in 1..=4 {
                source.push_str(&format!(
                    "{name}{rows}x{columns} m{rows}{columns}(float{rows}x{columns} x)\n{{\n    return ({name}{rows}x{columns}) x;\n}}\n"
                ));
                expected.push(Type::Matrix(scalar, rows, columns));
            }
        }

        let compiled = compile(&source);
        assert_eq!(compiled.ir.functions.len(), expected.len(), "{name}");

        for (function, ty) in compiled.ir.functions.iter().zip(expected) {
            assert_eq!(function.return_type, ty, "{name}: {}", function.name);
        }

        assert!(compiled.artifact.is_some());
    }
}

#[test]
fn pointer_alias_casts_preserve_integer_types_and_single_operand_evaluation() {
    for (name, scalar) in [
        ("intptr_t", Scalar::IntPtr),
        ("ssize_t", Scalar::IntPtr),
        ("uintptr_t", Scalar::UintPtr),
        ("size_t", Scalar::UintPtr),
        ("usize_t", Scalar::UintPtr),
    ] {
        let compiled = compile(&format!(
            r#"
                uint next(uint x)
                {{
                    return x + 1;
                }}

                {name} run(uint x)
                {{
                    return ({name}) next(x);
                }}
            "#
        ));
        let run = compiled.ir.functions.iter().find(|function| function.name == "run").unwrap();
        assert_eq!(run.return_type, Type::Scalar(scalar));
        let code = compiled.artifact.unwrap();
        assert_eq!(code.matches("next(x)").count(), 1, "{name}: {code}");
        assert!(
            code.contains(if scalar == Scalar::IntPtr {
                "((int64_t)next(x))"
            } else {
                "((uint64_t)next(x))"
            }),
            "{name}: {code}"
        );
    }
}

#[test]
fn alias_casts_keep_shape_validation_in_the_type_checker() {
    for source in [
        r#"
            int16_t2 run(float3 x)
            {
                return (int16_t2) x;
            }
        "#,
        r#"
            uint32_t run(float2 x)
            {
                return (uint32_t) x;
            }
        "#,
        r#"
            float64_t3x3 run(float2x2 x)
            {
                return (float64_t3x3) x;
            }
        "#,
    ] {
        let error = compile_source(
            "bad-cast-aliases.slang",
            source,
            &CompileOptions::default(),
            &MemoryIncludes::default(),
        )
        .unwrap_err();

        assert!(
            error.diagnostics.iter().any(|diagnostic| diagnostic.code == "T008"),
            "{source}: {error}"
        );
        assert!(
            !error.diagnostics.iter().any(|diagnostic| diagnostic.code == "S001"),
            "{source}: {error}"
        );
    }
}
