use slang_lexer::lex;
use slang_source::SourceId;
use slang_syntax::{Declaration, ExprKind, StmtKind, parse};

#[test]
fn fixed_width_alias_casts_retain_targets_operands_and_spans() {
    for name in [
        "int16_t",
        "uint16_t",
        "float16_t",
        "int32_t",
        "uint32_t",
        "float32_t",
        "int64_t",
        "uint64_t",
        "float64_t",
    ] {
        for shape in ["", "1", "4", "1x1", "2x3", "4x4"] {
            let target_name = format!("{name}{shape}");
            let source = format!("{target_name} run() {{ return ({target_name}) next(); }}");
            let module = parse(&lex(SourceId(0), &source).tokens).unwrap();
            let Declaration::Function(function) = &module.declarations[0] else {
                panic!()
            };

            let StmtKind::Block(statements) = &function.body.as_ref().unwrap().kind else {
                panic!()
            };

            let StmtKind::Return(Some(expression)) = &statements[0].kind else {
                panic!()
            };

            let ExprKind::Cast {
                target,
                value,
            } = &expression.kind
            else {
                panic!("{source}")
            };

            assert_eq!(target.name, target_name);
            assert!(matches!(value.kind, ExprKind::Call { .. }));
            assert_eq!(&source[target.span.start..target.span.end], target_name);
            assert_eq!(
                &source[expression.span.start..expression.span.end],
                format!("({target_name}) next()")
            );
        }
    }
}

#[test]
fn numeric_lookalikes_remain_parenthesized_value_names() {
    for name in [
        "int16_t5",
        "float16_t0",
        "uint32_t2x5",
        "float64_t01",
        "intptr_t2",
        "uintptr_t2x2",
        "size_t3",
        "int8_t",
        "uint8_t4",
    ] {
        let source = format!("int run(int {name}) {{ return ({name}) + 1; }}");
        let module = parse(&lex(SourceId(0), &source).tokens).unwrap();
        let Declaration::Function(function) = &module.declarations[0] else {
            panic!()
        };

        let StmtKind::Block(statements) = &function.body.as_ref().unwrap().kind else {
            panic!()
        };

        let StmtKind::Return(Some(expression)) = &statements[0].kind else {
            panic!()
        };

        let ExprKind::Binary {
            left,
            ..
        } = &expression.kind
        else {
            panic!("{source}")
        };

        assert!(matches!(&left.kind, ExprKind::Name(value) if value == name));
        assert!(left.parenthesized);
    }
}
