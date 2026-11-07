use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> slang_compiler::Compilation {
    compile_source(
        "one.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_or_else(|error| panic!("{source}: {error}"))
}

fn fails(source: &str, code: &str) {
    let error = compile_source(
        "bad.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == code), "{error}");
}

#[test]
fn one_component_aliases_and_applications_keep_their_distinct_types() {
    for scalar in [
        Scalar::Bool,
        Scalar::Int,
        Scalar::Uint,
        Scalar::Half,
        Scalar::Float,
        Scalar::Double,
    ] {
        let source = format!("{scalar}1 first;vector<{scalar},1> second;");
        let compiled = compile(&source);
        assert!(
            compiled
                .ir
                .globals
                .iter()
                .all(|global| global.ty == Type::Vector(scalar, 1))
        );
        assert!(
            compiled
                .reflection
                .globals
                .iter()
                .all(|global| global.type_name == format!("{scalar}1"))
        );
        for rows in 1..=4 {
            for columns in 1..=4 {
                if rows != 1 && columns != 1 {
                    continue;
                }
                let source = format!(
                    "{scalar}{rows}x{columns} first;matrix<{scalar},{rows},{columns}> second;"
                );
                assert!(
                    compile(&source)
                        .ir
                        .globals
                        .iter()
                        .all(|global| global.ty == Type::Matrix(scalar, rows, columns))
                );
            }
        }
    }
}

#[test]
fn one_component_arithmetic_preserves_vector_results_and_overloads() {
    let compiled = compile(
        "int choose(float value){return 1;}int choose(float1 value){return 2;}int run(){float1 a=1;float1 b=a+a;float1 c=a+1.0;float1 d=1.0+a;return choose(a)+choose(b)+choose(c)+choose(d)+choose(1.0)+int((a+a)[0]);}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("float1 b"));
    assert!(code.contains("choose__overload_1(a)"), "{code}");
    assert!(code.contains("choose__overload_0(1.0)"), "{code}");
}

#[test]
fn one_component_casts_constructors_conditions_and_scalar_conversion_work() {
    compile(
        "float run(){float1 a=(float1)2;int1 b=(int1)a;float c=(float)a;float d=float(a);float3 e=a;bool1 flag=bool1(true);bool enabled=flag;if(flag){c+=d+e.x;}return c+b[0];}",
    );
    compile("void run(){bool1 a=(bool1)1;bool b=(bool)a;float1 c=(float1)a;bool1 d=a==true;}");
    fails("void run(){float2 a=1;float b=(float)a;}", "T008");
    fails("void run(){float1x1 a=1;float b=(float)a;}", "T008");
    fails("void run(){float1 a=1;float1x1 b=a;}", "T008");
}

#[test]
fn matrix_rows_with_one_column_keep_vector_identity() {
    let compiled = compile(
        "int choose(float value){return 1;}int choose(float1 value){return 2;}int run(){float3x1 value=float3x1(1,2,3);return choose(value[0])+int(value[1][0]);}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("choose__overload_1((value)[0])"), "{code}");
    compile(
        "float run(){float1x3 row={1,2,3};float3x1 column={{1},{2},{3}};float1x1 product=mul(row,column);float1 vector=mul(row,float3(1));return product[0][0]+vector[0]+determinant(product);}",
    );
}

#[test]
fn one_component_members_preserve_writable_locations_and_bounds() {
    compile(
        "groupshared int1 vector;groupshared int1x1 matrix;[shader(\"compute\")][numthreads(1,1,1)]void main(){vector.x=1;matrix._11=2;InterlockedAdd(vector[0],1);InterlockedAdd(matrix._m00,1);}",
    );
    for source in [
        "void run(){float1 value=1;value.y=2;}",
        "void run(){float1 value=1;value[1]=2;}",
        "void run(){float1x1 value=1;value._12=2;}",
    ] {
        fails(source, "T013");
    }
    fails("void run(){float1 value=1;value.xx=float2(2);}", "T015");
    fails("void run(){const float1 value=1;value.x=2;}", "T015");
}

#[test]
fn one_component_output_intrinsics_and_derivatives_preserve_shapes() {
    for scalar in [Scalar::Half, Scalar::Float, Scalar::Double] {
        for shape in ["1", "1x1", "1x3", "3x1"] {
            let source = format!(
                "void run(){{{scalar}{shape} input=1,sine,cosine,whole;int{shape} exponent;sincos(input,sine,cosine);{scalar}{shape} fraction=modf(input,whole);{scalar}{shape} mantissa=frexp(input,exponent);}}"
            );
            compile(&source);
            let source = format!(
                "[shader(\"fragment\")]float4 main(float value:TEXCOORD0):SV_Target{{{scalar}{shape} input={scalar}{shape}(value);{scalar}{shape} result=fwidth_coarse(input)+ddx(input);return float4(1);}}"
            );
            compile(&source);
        }
    }
    fails(
        "void run(){float1 input=1;float sine,cosine;sincos(input,sine,cosine);}",
        "T014",
    );
}

#[test]
fn imported_one_component_aliases_and_nested_initializers_work() {
    let files = MemoryIncludes { files: [("types.slang".into(), "module types;public typedef vector<float,1> Value;public typedef matrix<float,1,1> Matrix;".into())].into() };
    compile_source("root.slang", "import types;struct Data{Value value={2};Matrix matrix={{3}};};float run(){Data values[2]={Data(),Data()};return values[0].value[0]+values[1].matrix[0][0];}", &CompileOptions::default(), &files).unwrap();
}

#[test]
fn overload_names_avoid_array_erasure_and_lexical_collisions() {
    let compiled = compile(
        "int choose(float value[2]){return 1;}int choose(float1 value[2]){return 2;}int choose__overload_1(){return 3;}int run(){int choose__overload_0=4;float1 values[2]={1,2};return choose(values)+choose__overload_0+choose__overload_1();}",
    );
    let code = compiled.artifact.unwrap();
    assert!(code.contains("choose__overload_1_(values)"), "{code}");
    assert!(
        code.contains("int choose__overload_0_(float value[2])"),
        "{code}"
    );
}
