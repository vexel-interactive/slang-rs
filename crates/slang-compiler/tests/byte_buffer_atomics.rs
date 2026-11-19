use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};
use slang_ir::{Expr, ExprKind, Stmt, StmtKind};
use slang_sema::{Scalar, Type};

fn compile(source: &str) -> Compilation {
    compile_source(
        "byte-buffer-atomics.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap()
}

fn reject(source: &str, code: &str) {
    let failure = compile_source(
        "invalid-byte-atomic.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(
        failure.diagnostics.iter().any(|diagnostic| diagnostic.code == code),
        "{source}: {failure}"
    );
}

fn expressions(statement: &Stmt) -> Vec<&Expr> {
    let mut values = vec![];
    visit_statement(statement, &mut |value| values.push(value));
    values
}

fn visit_statement<'a>(statement: &'a Stmt, visit: &mut impl FnMut(&'a Expr)) {
    match &statement.kind {
        StmtKind::Block(body) | StmtKind::Sequence(body) => {
            for statement in body {
                visit_statement(statement, visit);
            }
        }
        StmtKind::Variable(variable) => {
            if let Some(value) = &variable.initializer {
                visit_expression(value, visit);
            }
        }
        StmtKind::Variables(variables) => {
            for variable in variables {
                if let Some(value) = &variable.initializer {
                    visit_expression(value, visit);
                }
            }
        }
        StmtKind::Expression(value) | StmtKind::Return(Some(value)) => {
            visit_expression(value, visit)
        }
        StmtKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            visit_expression(condition, visit);
            visit_statement(then_branch, visit);
            if let Some(branch) = else_branch {
                visit_statement(branch, visit);
            }
        }
        StmtKind::While {
            condition,
            body,
        }
        | StmtKind::DoWhile {
            condition,
            body,
        } => {
            visit_expression(condition, visit);
            visit_statement(body, visit);
        }
        StmtKind::For {
            initializer,
            condition,
            increment,
            body,
        } => {
            visit_statement(initializer, visit);
            if let Some(value) = condition {
                visit_expression(value, visit);
            }
            if let Some(value) = increment {
                visit_expression(value, visit);
            }
            visit_statement(body, visit);
        }
        StmtKind::Switch {
            selector,
            cases,
        } => {
            visit_expression(selector, visit);
            for case in cases {
                if let Some(value) = &case.value {
                    visit_expression(value, visit);
                }
                for statement in &case.statements {
                    visit_statement(statement, visit);
                }
            }
        }
        _ => {}
    }
}

fn visit_expression<'a>(value: &'a Expr, visit: &mut impl FnMut(&'a Expr)) {
    visit(value);
    match &value.kind {
        ExprKind::BlockExpression {
            statements,
            value,
        } => {
            for statement in statements {
                visit_statement(statement, visit);
            }
            visit_expression(value, visit);
        }
        ExprKind::Construct(arguments)
        | ExprKind::Call {
            arguments,
            ..
        }
        | ExprKind::IntrinsicCall {
            arguments,
            ..
        } => {
            for argument in arguments {
                visit_expression(argument, visit);
            }
        }
        ExprKind::MethodCall {
            receiver,
            arguments,
            ..
        } => {
            visit_expression(receiver, visit);
            for argument in arguments {
                visit_expression(argument, visit);
            }
        }
        ExprKind::AddressView(value)
        | ExprKind::ConstantBufferLoad(value)
        | ExprKind::DeclarationDefault(value)
        | ExprKind::Unary {
            value,
            ..
        }
        | ExprKind::Cast {
            value,
        }
        | ExprKind::Postfix {
            value,
            ..
        }
        | ExprKind::Member {
            base: value,
            ..
        } => visit_expression(value, visit),
        ExprKind::Binary {
            left,
            right,
            ..
        } => {
            visit_expression(left, visit);
            visit_expression(right, visit);
        }
        ExprKind::Index {
            base,
            index,
        } => {
            visit_expression(base, visit);
            visit_expression(index, visit);
        }
        ExprKind::Conditional {
            condition,
            then_value,
            else_value,
        } => {
            visit_expression(condition, visit);
            visit_expression(then_value, visit);
            visit_expression(else_value, visit);
        }
        _ => {}
    }
}

fn assert_atomic_initializer_captures(
    compiled: &Compilation,
    helper: &slang_ir::Function,
    output_coordinates: bool,
) {
    assert!(helper.parameters.is_empty());
    let values = expressions(helper.body.as_ref().unwrap());
    let calls: Vec<_> = values
        .iter()
        .filter_map(|value| match &value.kind {
            ExprKind::Call {
                name,
                ..
            } if matches!(name.as_str(), "receiver" | "address" | "inputValue" | "index") => {
                Some(name.as_str())
            }
            _ => None,
        })
        .collect();
    let mut expected = vec!["receiver", "address", "inputValue"];
    if output_coordinates {
        expected.push("index");
    }
    assert_eq!(calls, expected);
    let atomics: Vec<_> = values
        .iter()
        .filter(|value| {
            matches!(&value.kind, ExprKind::MethodCall { name, .. } if name == "InterlockedAdd")
        })
        .collect();
    assert_eq!(atomics.len(), 1);
    let ExprKind::MethodCall {
        receiver,
        arguments,
        ..
    } = &atomics[0].kind
    else {
        unreachable!();
    };
    assert!(matches!(receiver.kind, ExprKind::Name(_)));
    assert_eq!(arguments.len(), 2 + usize::from(output_coordinates));
    assert!(arguments[..2].iter().all(|argument| matches!(argument.kind, ExprKind::Name(_))));
    if output_coordinates {
        assert!(matches!(
            &arguments[2].kind,
            ExprKind::Index { base, index }
                if matches!(base.kind, ExprKind::Name(_)) && matches!(index.kind, ExprKind::Name(_))
        ));
    }
    for global in &compiled.ir.globals {
        if let Some(initializer) = &global.initializer {
            visit_expression(initializer, &mut |value| {
                assert!(!matches!(value.kind, ExprKind::MethodCall { .. }));
                assert!(!matches!(value.kind, ExprKind::BlockExpression { .. }));
            });
        }
    }
    let index =
        compiled.ir.functions.iter().position(|function| function.name == helper.name).unwrap();
    if let Some(graph) = slang_ir::flow::build_function(&compiled.ir, index) {
        graph.verify().unwrap();
    }
}

#[test]
fn raw_atomic_methods_accept_optional_outputs_and_writable_scalar_locations() {
    let compiled =
        compile(include_str!("../../../tests/conformance/positive/byte-buffer-atomics.slang"));
    let artifact = compiled.artifact.unwrap();
    for method in
        ["Add", "Min", "Max", "And", "Or", "Xor", "Exchange", "CompareExchange", "CompareStore"]
    {
        assert!(artifact.contains(&format!(".Interlocked{method}(")), "{artifact}");
    }
    assert_eq!(compiled.reflection.globals.len(), 4);
    for index in 0..compiled.ir.functions.len() {
        if let Some(graph) = slang_ir::flow::build_function(&compiled.ir, index) {
            graph.verify().unwrap();
        }
    }
}

#[test]
fn bounded_raw_atomic_program_compiles_with_its_shader_entry_and_bindings() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/byte-buffer-atomic-program.slang"
    ));
    assert_eq!(compiled.reflection.entry_points.len(), 1);
    assert_eq!(compiled.reflection.entry_points[0].thread_group_size, Some([1, 1, 1]));
    assert_eq!(
        compiled
            .reflection
            .globals
            .iter()
            .find(|global| global.name == "data")
            .unwrap()
            .binding
            .unwrap()
            .register
            .index,
        0
    );
    assert_eq!(
        compiled
            .reflection
            .globals
            .iter()
            .find(|global| global.name == "output")
            .unwrap()
            .binding
            .unwrap()
            .register
            .index,
        1
    );
    assert!(!compiled.artifact.unwrap().contains("@slang."));
}

#[test]
fn raw_atomic_input_conversions_preserve_unsigned_operations_and_signed_output_bits() {
    let source = r#"
RWByteAddressBuffer bytes;
void run() {
    int previous;
    bytes.InterlockedAdd(4.5, -1, previous);
    bytes.InterlockedMin(true, 7.5);
    bytes.InterlockedCompareExchange(8, -1, false, previous);
}
"#;
    let compiled = compile(source);
    let run = compiled.ir.functions.iter().find(|function| function.name == "run").unwrap();
    let values = expressions(run.body.as_ref().unwrap());
    let arguments = values
        .iter()
        .find_map(|value| match &value.kind {
            ExprKind::MethodCall {
                name,
                arguments,
                ..
            } if name == "InterlockedAdd" => Some(arguments),
            _ => None,
        })
        .unwrap();
    assert_eq!(arguments[0].ty, Type::Scalar(Scalar::Uint));
    assert_eq!(arguments[1].ty, Type::Scalar(Scalar::Uint));
    assert_eq!(arguments[2].ty, Type::Scalar(Scalar::Int));
    assert!(matches!(&arguments[2].kind, ExprKind::Name(name) if name == "previous"));
    let min = values
        .iter()
        .find_map(|value| match &value.kind {
            ExprKind::MethodCall {
                name,
                arguments,
                ..
            } if name == "InterlockedMin" => Some(arguments),
            _ => None,
        })
        .unwrap();
    assert!(min.iter().all(|argument| argument.ty == Type::Scalar(Scalar::Uint)));
    let artifact = compiled.artifact.unwrap();
    assert!(!artifact.contains("((uint)previous)"), "{artifact}");
}

#[test]
fn raw_atomic_input_conversions_accept_numeric_scalars_and_only_uint_one_component_vectors() {
    for ty in [
        "bool",
        "half",
        "float",
        "double",
        "int16_t",
        "uint16_t",
        "int",
        "uint",
        "int64_t",
        "uint64_t",
        "intptr_t",
        "uintptr_t",
        "uint1",
    ] {
        compile(&format!(
            r#"
RWByteAddressBuffer bytes;
void run({ty} address, {ty} value) {{
    uint previous;
    bytes.InterlockedAdd(address, value, previous);
}}
"#
        ));
    }
    for ty in ["int1", "float1", "bool1", "uint2"] {
        for (address, value) in [("input", "1u"), ("0u", "input")] {
            reject(
                &format!(
                    r#"
RWByteAddressBuffer bytes;
void run({ty} input) {{
    uint previous;
    bytes.InterlockedAdd({address}, {value}, previous);
}}
"#
                ),
                "T008",
            );
        }
    }
}

#[test]
fn raw_atomic_inputs_load_constant_buffer_bool_and_one_component_values() {
    compile(
        r#"
RWByteAddressBuffer bytes;
ConstantBuffer<uint> address;
ConstantBuffer<bool> flag;
ConstantBuffer<uint1> one;
void run() {
    uint previous;
    bytes.InterlockedAdd(address, flag, previous);
    bytes.InterlockedAdd(one, one, previous);
}
"#,
    );
}

#[test]
fn raw_atomic_inputs_accept_contextual_nested_braces_and_reject_excess_components() {
    compile(
        r#"
RWByteAddressBuffer bytes;
ConstantBuffer<bool> flag;
ConstantBuffer<uint> address;
void run() {
    uint previous;
    bytes.InterlockedAdd({0}, {{true}}, previous);
    bytes.InterlockedAdd({{0}}, {{1.5}}, previous);
    bytes.InterlockedAdd({}, {{}}, previous);
    bytes.InterlockedAdd({uint1(0)}, {int1(1)}, previous);
    bytes.InterlockedAdd({address}, {{flag}}, previous);
    bytes.InterlockedCompareExchange({0}, {7}, {{8}}, previous);
}
"#,
    );
    for expression in ["{0, 1}", "{{0, 1}}"] {
        reject(
            &format!(
                r#"
RWByteAddressBuffer bytes;
void run() {{
    bytes.InterlockedAdd(0, {expression});
}}
"#
            ),
            "T021",
        );
    }
}

#[test]
fn raw_atomic_receivers_inputs_and_output_coordinates_execute_once_in_source_order() {
    let source = r#"
RWByteAddressBuffer bytes;
RWStructuredBuffer<int> output;
static uint trace;
RWByteAddressBuffer receiver() { trace = trace * 10 + 1; return bytes; }
uint address() { trace = trace * 10 + 2; return 0; }
uint value() { trace = trace * 10 + 3; return 1; }
uint index() { trace = trace * 10 + 4; return 0; }
void run() {
    receiver().InterlockedAdd(address(), value(), output[index()]);
}
"#;
    let compiled = compile(source);
    let function = compiled.ir.functions.iter().find(|function| function.name == "run").unwrap();
    let values = expressions(function.body.as_ref().unwrap());
    let calls: Vec<_> = values
        .iter()
        .filter_map(|value| match &value.kind {
            ExprKind::Call {
                name,
                ..
            } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(calls, ["receiver", "address", "value", "index"]);
    let atomic = values
        .iter()
        .find(|value| {
            matches!(&value.kind, ExprKind::MethodCall { name, .. } if name == "InterlockedAdd")
        })
        .unwrap();
    let mut atomic_calls = vec![];
    visit_expression(atomic, &mut |value| {
        if let ExprKind::Call {
            name,
            ..
        } = &value.kind
        {
            atomic_calls.push(name.as_str());
        }
    });
    assert!(atomic_calls.is_empty(), "atomic operands must use the preceding captures");
    if let ExprKind::MethodCall {
        receiver,
        arguments,
        ..
    } = &atomic.kind
    {
        assert!(matches!(receiver.kind, ExprKind::Name(_)));
        assert!(arguments[..2].iter().all(|argument| matches!(argument.kind, ExprKind::Name(_))));
        assert!(matches!(
            &arguments[2].kind,
            ExprKind::Index { index, .. } if matches!(index.kind, ExprKind::Name(_))
        ));
    }
    let artifact = compiled.artifact.unwrap();
    let run = artifact.split("void run()\n{").last().unwrap().split("\n}\n").next().unwrap();
    let mut positions = vec![];
    for call in ["receiver()", "address()", "value()", "index()"] {
        assert_eq!(run.matches(call).count(), 1, "{run}");
        positions.push(run.find(call).unwrap());
    }
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "{run}");
}

#[test]
fn direct_global_atomic_initializers_capture_calls_in_a_private_function() {
    let compiled = compile(
        r#"
RWByteAddressBuffer bytes;
RWStructuredBuffer<uint> output;
static uint trace = 0;
static uint _slang_effect_global_init_0 = 9;
RWByteAddressBuffer receiver() { trace = trace * 10 + 1; return bytes; }
uint address() { trace = trace * 10 + 2; return 0; }
uint inputValue() { trace = trace * 10 + 3; return 5; }
uint index() { trace = trace * 10 + 4; return 0; }
static uint startup = (receiver().InterlockedAdd(address(), inputValue(), output[index()]), 0u);
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    output[0] = startup;
    output[1] = trace;
    output[2] = _slang_effect_global_init_0;
}
"#,
    );
    let startup = compiled.ir.globals.iter().find(|global| global.name == "startup").unwrap();
    let ExprKind::Call {
        name,
        arguments,
    } = &startup.initializer.as_ref().unwrap().kind
    else {
        panic!("the global initializer must call its generated function");
    };
    assert!(
        name.strip_prefix("_slang_effect_")
            .unwrap()
            .trim_start_matches('_')
            .starts_with("global_init_")
    );
    assert_ne!(name, "_slang_effect_global_init_0");
    assert!(arguments.is_empty());
    let helper = compiled.ir.functions.iter().find(|function| &function.name == name).unwrap();
    assert_eq!(helper.return_type, Type::Scalar(Scalar::Uint));
    assert_atomic_initializer_captures(&compiled, helper, true);
}

#[test]
fn direct_array_global_atomic_initializers_wrap_the_generated_array_result() {
    let compiled = compile(
        r#"
typedef uint Pair[2];
RWByteAddressBuffer bytes;
RWStructuredBuffer<uint> output;
static uint trace = 0;
RWByteAddressBuffer receiver() { trace = trace * 10 + 1; return bytes; }
uint address() { trace = trace * 10 + 2; return 0; }
uint inputValue() { trace = trace * 10 + 3; return 5; }
static Pair startup = (receiver().InterlockedAdd(address(), inputValue()), Pair(7u, 11u));
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    output[0] = startup[0];
    output[1] = startup[1];
    output[2] = trace;
}
"#,
    );
    let startup = compiled.ir.globals.iter().find(|global| global.name == "startup").unwrap();
    assert!(matches!(startup.ty, Type::Array(_, 2)));
    let ExprKind::Member {
        base,
        name,
    } = &startup.initializer.as_ref().unwrap().kind
    else {
        panic!("the array initializer must unwrap its generated result structure");
    };
    assert_eq!(name, "_value");
    let ExprKind::Call {
        name,
        arguments,
    } = &base.kind
    else {
        panic!("the wrapped array initializer must call its generated function");
    };
    assert!(name.starts_with("_slang_effect_global_init_"));
    assert!(arguments.is_empty());
    let helper = compiled.ir.functions.iter().find(|function| &function.name == name).unwrap();
    let Type::Struct(result_name) = &helper.return_type else {
        panic!("HLSL array returns must use a result structure");
    };
    let result = compiled.ir.structures.iter().find(|result| &result.name == result_name).unwrap();
    assert_eq!(result.fields.len(), 1);
    assert_eq!(result.fields[0].name, "_value");
    assert_eq!(result.fields[0].ty, startup.ty);
    assert_atomic_initializer_captures(&compiled, helper, false);
}

#[test]
fn global_atomic_matrix_array_initializers_preserve_constructor_parameter_storage_order() {
    for major in ["row_major", "column_major"] {
        let source = format!(
            r#"
typedef {major} float2x3 Matrix;
typedef Matrix Matrices[2];
RWByteAddressBuffer bytes;
RWStructuredBuffer<uint> output;
static uint trace = 0;
RWByteAddressBuffer receiver() {{ trace = trace * 10 + 1; return bytes; }}
uint address() {{ trace = trace * 10 + 2; return 0; }}
uint inputValue() {{ trace = trace * 10 + 3; return 5; }}
static Matrices startup = (
    receiver().InterlockedAdd(address(), inputValue()),
    Matrices(Matrix(1, 2, 3, 4, 5, 6), Matrix(7, 8, 9, 10, 11, 12))
);
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {{
    output[0] = uint(startup[0][1][0]);
    output[1] = uint(startup[1][0][2]);
    output[2] = trace;
}}
"#
        );
        let compiled = compile(&source);
        let startup = compiled.ir.globals.iter().find(|global| global.name == "startup").unwrap();
        assert!(startup.modifiers.iter().any(|modifier| modifier == major), "{major}");
        let helper = compiled
            .ir
            .functions
            .iter()
            .find(|function| function.name.starts_with("_slang_effect_global_init_"))
            .unwrap();
        let Type::Struct(result_name) = &helper.return_type else {
            panic!("matrix array initializer must return its result structure");
        };
        let result =
            compiled.ir.structures.iter().find(|result| &result.name == result_name).unwrap();
        let field = result.fields.iter().find(|field| field.name == "_value").unwrap();
        assert_eq!(field.ty, startup.ty);
        assert!(field.modifiers.iter().any(|modifier| modifier == major), "{major}");
        let parameter = compiled
            .ir
            .functions
            .iter()
            .filter(|function| function.return_type == helper.return_type)
            .flat_map(|function| &function.parameters)
            .find(|parameter| parameter.ty == startup.ty)
            .unwrap();
        assert!(parameter.modifiers.iter().any(|modifier| modifier == major), "{major}");
        assert_atomic_initializer_captures(&compiled, helper, false);
    }
}

#[test]
fn generated_half_store_compare_exchange_updates_the_retry_observation() {
    let compiled =
        compile(include_str!("../../../tests/conformance/positive/typed-byte-half-legacy.slang"));
    assert!(!compiled.ir.native_16bit_types);
    let mut exchanges = 0;
    for function in &compiled.ir.functions {
        let Some(body) = &function.body else {
            continue;
        };
        let values = expressions(body);
        for arguments in values.iter().filter_map(|value| match &value.kind {
            ExprKind::MethodCall {
                name,
                arguments,
                ..
            } if name == "InterlockedCompareExchange" => Some(arguments),
            _ => None,
        }) {
            exchanges += 1;
            assert_eq!(arguments.len(), 4);
            assert_eq!(arguments[3].ty, Type::Scalar(Scalar::Uint));
            let ExprKind::Name(observed) = &arguments[3].kind else {
                panic!("CAS output must remain the writable retry observation");
            };
            assert!(observed.starts_with("_observed_"), "{observed}");
            assert!(values.iter().any(|value| matches!(
                &value.kind,
                ExprKind::Binary { op, left, .. }
                    if op == "!=" && matches!(&left.kind, ExprKind::Name(name) if name == observed)
            )), "CAS output must be read by the retry condition");
        }
    }
    assert!(exchanges > 0);
}

#[test]
fn ignored_raw_exchange_outputs_use_private_uint_storage_without_reflection_leaks() {
    let compiled = compile(include_str!(
        "../../../tests/conformance/positive/byte-buffer-atomic-exchange-discard.slang"
    ));
    let scratch: Vec<_> = compiled
        .ir
        .globals
        .iter()
        .filter(|global| global.name.starts_with("_slang_atomic_ignored__"))
        .collect();
    assert_eq!(scratch.len(), 3);
    assert!(scratch.iter().all(|global| global.ty == Type::Scalar(Scalar::Uint)
        && global.modifiers.iter().any(|modifier| modifier == "static")
        && global.binding.is_none()));
    assert_eq!(compiled.reflection.globals.len(), 4);
    assert!(
        compiled.reflection.globals.iter().any(|global| global.name == "_slang_atomic_ignored_0")
    );
    assert!(
        !compiled
            .reflection
            .globals
            .iter()
            .any(|global| global.name.starts_with("_slang_atomic_ignored__"))
    );
    let artifact = compiled.artifact.unwrap();
    assert!(!artifact.contains(".InterlockedExchange(8, ((uint)5u));"), "{artifact}");
}

#[test]
fn raw_atomics_do_not_capture_generic_user_methods_with_the_same_names() {
    let source = r#"
struct User {
    T InterlockedAdd<T>(T value) { return value; }
    T InterlockedExchange<T>(T value) { return value; }
    T InterlockedMin<T>(T value) { return value; }
}
RWByteAddressBuffer bytes;
void run() {
    User user;
    uint value = user.InterlockedAdd(2u);
    uint previous;
    bytes.InterlockedAdd(0, value, previous);
    value = user.InterlockedExchange<float>(3.5);
    bytes.InterlockedExchange(4, value);
    value = user.InterlockedMin(1u);
    bytes.InterlockedMin(8, value);
}
"#;
    let artifact = compile(source).artifact.unwrap();
    assert!(artifact.contains(".InterlockedAdd("), "{artifact}");
    assert!(artifact.contains(".InterlockedExchange("), "{artifact}");
    assert!(artifact.contains(".InterlockedMin("), "{artifact}");
    assert!(!artifact.contains("@slang."), "{artifact}");
}

#[test]
fn raw_atomic_outputs_keep_fields_swizzles_array_and_resource_locations() {
    compile(
        r#"
struct Result { uint value; row_major int2x2 matrix; }
RWByteAddressBuffer bytes;
RWStructuredBuffer<Result> output;
RWTexture2D<int> image;
void run() {
    Result result;
    uint2 lanes;
    int2x2 matrix;
    uint slots[2];
    bytes.InterlockedMax(0, 4u, result.value);
    bytes.InterlockedMax(0, 5u, lanes.y);
    bytes.InterlockedMax(0, 6u, matrix[1][0]);
    bytes.InterlockedMax(0, 7u, slots[1]);
    bytes.InterlockedMax(0, 8u, output[0].value);
    bytes.InterlockedMax(0, 9u, image[int2(0)]);
    bytes.InterlockedMax(0, 10u, output[0].matrix[1][0]);
}
"#,
    );
}

#[test]
fn raw_atomic_arities_and_readonly_receivers_report_errors() {
    for method in ["Add", "Min", "Max", "And", "Or", "Xor", "Exchange"] {
        for arguments in ["", "0", "0, 1u, previous, previous"] {
            reject(
                &format!(
                    r#"
RWByteAddressBuffer bytes;
void run() {{
    uint previous;
    bytes.Interlocked{method}({arguments});
}}
"#
                ),
                "T014",
            );
        }
    }
    for (method, arguments) in [
        ("CompareStore", "0, 1u"),
        ("CompareStore", "0, 1u, 2u, previous"),
        ("CompareExchange", "0, 1u, 2u"),
        ("CompareExchange", "0, 1u, 2u, previous, previous"),
    ] {
        reject(
            &format!(
                r#"
RWByteAddressBuffer bytes;
void run() {{
    uint previous;
    bytes.Interlocked{method}({arguments});
}}
"#
            ),
            "T014",
        );
    }
    reject(
        include_str!("../../../tests/conformance/negative/byte-buffer-atomic-readonly.slang"),
        "T014",
    );
}

#[test]
fn raw_atomic_outputs_reject_noninteger_shapes_and_immutable_locations() {
    for ty in
        ["bool", "float", "double", "int16_t", "uint16_t", "int64_t", "uint64_t", "uint1", "int2"]
    {
        reject(
            &format!(
                r#"
RWByteAddressBuffer bytes;
void run() {{
    {ty} previous;
    bytes.InterlockedAdd(0, 1u, previous);
}}
"#
            ),
            "T014",
        );
    }
    for declaration in ["const uint previous = 0u", "let previous = 0u"] {
        reject(
            &format!(
                r#"
RWByteAddressBuffer bytes;
void run() {{
    {declaration};
    bytes.InterlockedAdd(0, 1u, previous);
}}
"#
            ),
            "T015",
        );
    }
    reject(
        r#"
RWByteAddressBuffer bytes;
StructuredBuffer<uint> previous;
void run() { bytes.InterlockedAdd(0, 1u, previous[0]); }
"#,
        "T015",
    );
    reject(
        r#"
struct Immutable { let value: int; }
RWByteAddressBuffer bytes;
RWStructuredBuffer<Immutable> previous;
void run() { bytes.InterlockedAdd(0, 1u, previous[0].value); }
"#,
        "T015",
    );
    reject(
        r#"
RWByteAddressBuffer bytes;
ConstantBuffer<uint> previous;
void run() { bytes.InterlockedAdd(0, 1u, previous); }
"#,
        "T015",
    );
}

#[test]
fn raw_atomic_calls_produce_unit_values_in_typed_ir() {
    let compiled = compile(
        r#"
RWByteAddressBuffer bytes;
void run() {
    var unit = bytes.InterlockedAdd(0, 1u);
}
"#,
    );
    let run = compiled.ir.functions.iter().find(|function| function.name == "run").unwrap();
    let values = expressions(run.body.as_ref().unwrap());
    assert!(values.iter().any(|value| value.ty == Type::Void
        && matches!(&value.kind, ExprKind::MethodCall { name, .. } if name == "InterlockedAdd")));
    assert!(!compiled.artifact.unwrap().contains("void unit"));
}
