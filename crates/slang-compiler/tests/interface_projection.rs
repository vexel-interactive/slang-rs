use slang_compiler::{Compilation, CompileOptions, MemoryIncludes, compile_source};

const OVERLOADS: &str =
    include_str!("../../../tests/conformance/positive/interface-projection-overloads/main.slang");
const OVERLOAD_LIBRARY: &str =
    include_str!("../../../tests/conformance/positive/interface-projection-overloads/lib.slang");
const OPERATIONS: &str =
    include_str!("../../../tests/conformance/positive/interface-projection-operations/main.slang");
const OPERATION_LIBRARY: &str =
    include_str!("../../../tests/conformance/positive/interface-projection-operations/lib.slang");
const DEFAULTS: &str =
    include_str!("../../../tests/conformance/positive/interface-projection-defaults/main.slang");
const DEFAULT_LIBRARY: &str =
    include_str!("../../../tests/conformance/positive/interface-projection-defaults/lib.slang");

fn includes(library: &str) -> MemoryIncludes {
    MemoryIncludes {
        files: [("lib.slang".into(), library.into())].into(),
    }
}

fn compile(source: &str, library: &str) -> Compilation {
    compile_source("main.slang", source, &CompileOptions::default(), &includes(library)).unwrap()
}

#[test]
fn accessible_exact_overloads_hide_only_the_matching_projected_signature() {
    compile(OVERLOADS, OVERLOAD_LIBRARY);
}

#[test]
fn projection_preserves_generic_static_mutating_and_output_operations() {
    compile(OPERATIONS, OPERATION_LIBRARY);
}

#[test]
fn visible_concrete_prefixes_with_default_tails_hide_the_projected_signature() {
    compile(
        include_str!(
            "../../../tests/conformance/positive/interface-projection-default-tails/main.slang"
        ),
        include_str!(
            "../../../tests/conformance/positive/interface-projection-default-tails/lib.slang"
        ),
    );
}

#[test]
fn inaccessible_derived_requirement_preserves_the_public_base_projection() {
    compile(
        include_str!(
            "../../../tests/conformance/positive/interface-projection-derived-internal/main.slang"
        ),
        include_str!(
            "../../../tests/conformance/positive/interface-projection-derived-internal/lib.slang"
        ),
    );
}

#[test]
fn unrelated_public_requirements_remain_ambiguous() {
    assert_rejected(
        include_str!(
            "../../../tests/conformance/negative/interface-projection-unrelated/main.slang"
        ),
        include_str!(
            "../../../tests/conformance/negative/interface-projection-unrelated/lib.slang"
        ),
    );
}

#[test]
fn internal_interfaces_requirements_and_unrelated_members_remain_private() {
    for (source, library) in [
        (
            include_str!(
                "../../../tests/conformance/negative/interface-projection-internal-interface/main.slang"
            ),
            include_str!(
                "../../../tests/conformance/negative/interface-projection-internal-interface/lib.slang"
            ),
        ),
        (
            include_str!(
                "../../../tests/conformance/negative/interface-projection-internal-requirement/main.slang"
            ),
            include_str!(
                "../../../tests/conformance/negative/interface-projection-internal-requirement/lib.slang"
            ),
        ),
        (
            include_str!(
                "../../../tests/conformance/negative/interface-projection-private-unrelated/main.slang"
            ),
            include_str!(
                "../../../tests/conformance/negative/interface-projection-private-unrelated/lib.slang"
            ),
        ),
    ] {
        assert_rejected(source, library);
    }
}

#[test]
fn projected_defaults_retain_private_declaration_module_helpers() {
    let compiled = compile(DEFAULTS, DEFAULT_LIBRARY);
    assert!(compiled.artifact.unwrap().contains("seed"));
}

#[test]
fn unused_projected_owner_keeps_adapters_dormant() {
    let compiled = compile_source(
        "unused.slang",
        include_str!("../../../tests/conformance/positive/interface-projection-unused.slang"),
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )
    .unwrap();
    assert_eq!(compiled.ir.functions.len(), 1);
    assert_eq!(compiled.ir.functions[0].name, "main");
}

#[test]
fn projected_selections_survive_serialization_and_rechecking_with_module_origins() {
    for (source, library) in [
        (OVERLOADS, OVERLOAD_LIBRARY),
        (OPERATIONS, OPERATION_LIBRARY),
        (DEFAULTS, DEFAULT_LIBRARY),
    ] {
        let mut sources = slang_source::SourceMap::default();
        let root = sources.add("main.slang", source);
        let program = slang_module::load(
            root,
            &mut sources,
            &includes(library),
            &slang_preprocessor::Options::default(),
        )
        .unwrap();
        let checked = slang_sema::check_program(program).unwrap();
        assert!(!checked.module.constraint_calls.is_empty());
        serde_json::to_string(&checked.module).unwrap();
        let rechecked = slang_sema::check_program(slang_module::Program {
            syntax: checked.module.clone(),
            modules: checked.modules.clone(),
            symbols: checked.symbols.clone(),
            diagnostics: vec![],
        })
        .unwrap();
        assert_eq!(checked.call_names, rechecked.call_names);
        assert_eq!(
            serde_json::to_string(&slang_ir::lower(&checked).unwrap()).unwrap(),
            serde_json::to_string(&slang_ir::lower(&rechecked).unwrap()).unwrap(),
        );
    }
}

fn assert_rejected(source: &str, library: &str) {
    let error =
        compile_source("main.slang", source, &CompileOptions::default(), &includes(library))
            .unwrap_err();
    assert!(error.diagnostics.iter().any(|diagnostic| diagnostic.code == "T014"), "{error}");
}

#[test]
fn projected_requirement_defaults_preserve_private_caller_owned_types() {
    let includes = MemoryIncludes {
        files: [(
            "lib.slang".into(),
            include_str!(
                "../../../tests/conformance/positive/interface-projection-private-owner/lib.slang"
            )
            .into(),
        )]
        .into(),
    };
    let source = include_str!(
        "../../../tests/conformance/positive/interface-projection-private-owner/main.slang"
    );
    compile_source("main.slang", source, &CompileOptions::default(), &includes).unwrap();
}
