use slang_compiler::{Compilation, CompileError, CompileOptions, MemoryIncludes, compile_source};
use slang_ir::{ExprKind, StmtKind};
use slang_sema::{Scalar, Type};

fn compile(source: &str, files: &[(&str, &str)]) -> Result<Compilation, CompileError> {
    compile_source(
        "main.slang",
        source,
        &CompileOptions::default(),
        &MemoryIncludes {
            files: files
                .iter()
                .map(|(p, s)| (p.to_string(), s.to_string()))
                .collect(),
        },
    )
}

fn fails(source: &str, files: &[(&str, &str)], code: &str) -> CompileError {
    let error = compile(source, files).unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == code), "{error}");
    error
}

#[test]
fn public_imports_keep_helpers_types_resources_and_source_ownership() {
    let compiled = compile(
        "import math; float run(){Pair p;p.value=twice(1.0);return p.value+data[0];}",
        &[(
            "math.slang",
            "module math; public struct Pair {public float value;};
            float helper(float x){return x*2.0;}
            public float twice(float x){return helper(x);}
            public StructuredBuffer<float> data;",
        )],
    )
    .unwrap();
    assert_eq!(compiled.reflection.modules.len(), 2);
    assert_eq!(compiled.ir.functions.len(), 3);
    let global = &compiled.reflection.globals[0];
    assert_eq!(global.name, "data");
    assert_eq!(global.module.as_deref(), Some("math"));
    assert_ne!(global.target_name, global.name);
    assert_eq!(compiled.sources.files().len(), 2);
    let code = compiled.artifact.unwrap();
    assert!(!code.contains("public "));
    assert!(code.contains(&global.target_name));
}

#[test]
fn internal_declarations_and_fields_stay_in_their_module() {
    let library = [(
        "lib.slang",
        "module lib; public struct Pair {float hidden; public float value;};
        float hidden(){return 1;} static float secret=1;
        public float read(Pair p){return p.hidden+secret+hidden();}",
    )];
    fails(
        "import lib; float run(){return hidden();}",
        &library,
        "T014",
    );
    fails("import lib; float run(){return secret;}", &library, "T011");
    fails(
        "import lib; float run(){Pair p;return p.hidden;}",
        &library,
        "T018",
    );
    compile(
        "import lib; float run(){Pair p;p.value=1;return read(p);}",
        &library,
    )
    .unwrap();
}

#[test]
fn duplicate_and_diamond_imports_load_one_copy() {
    let compiled = compile(
        "import left; import right; import left; float run(){return l()+r();}",
        &[
            ("base.slang", "module base;public float value(){return 2;}"),
            (
                "left.slang",
                "module left;import base;public float l(){return value();}",
            ),
            (
                "right.slang",
                "module right;import base;public float r(){return value();}",
            ),
        ],
    )
    .unwrap();
    assert_eq!(compiled.reflection.modules.len(), 4);
    assert_eq!(compiled.sources.files().len(), 4);
    assert_eq!(compiled.ir.functions.len(), 4);
    assert_eq!(compiled.reflection.modules[0].imports.len(), 2);
}

#[test]
fn reexports_change_transitive_visibility() {
    let files = [
        ("base.slang", "module base;public float value(){return 2;}"),
        (
            "facade.slang",
            "module facade;import base;public float read(){return value();}",
        ),
        ("exported.slang", "module exported;__exported import base;"),
    ];
    compile("import exported;float run(){return value();}", &files).unwrap();
    fails("import facade;float run(){return value();}", &files, "T014");
}

#[test]
fn module_names_do_not_capture_parameters_locals_or_loop_bindings() {
    let compiled = compile(
        "import lib; float withParameter(float value){return value;}
        float run(){float total=value;{float value=total;total=value;}
            for(int value=0;value<2;value++){total+=float(value);}
            switch(1){case 0:float value=total;total=value;break;default:break;}
            return total+value;}",
        &[("lib.slang", "module lib;public static const float value=2;")],
    )
    .unwrap();
    let global = &compiled.reflection.globals[0].target_name;
    let code = compiled.artifact.unwrap();
    assert!(code.contains("return value;"));
    assert!(code.contains("for (int value = 0;"));
    assert!(code.contains(&format!("return (total + {global})")));
}

#[test]
fn linked_expression_ids_and_overload_calls_remain_distinct() {
    let compiled = compile(
        "import lib; float run(){return select(1);}",
        &[(
            "lib.slang",
            "module lib;
            public float select(float x){return x+1.0;}
            int select(int x){return x+2;}
            public float check(){return float(select(1));}
            public static float select__overload_0=0;",
        )],
    )
    .unwrap();
    let run = compiled
        .ir
        .functions
        .iter()
        .find(|f| f.name == "run")
        .unwrap();
    let StmtKind::Block(body) = &run.body.as_ref().unwrap().kind else {
        panic!()
    };
    let StmtKind::Return(Some(value)) = &body[0].kind else {
        panic!()
    };
    let ExprKind::Call { name, .. } = &value.kind else {
        panic!()
    };
    assert_eq!(value.ty, Type::Scalar(Scalar::Float));
    let target = compiled
        .ir
        .functions
        .iter()
        .find(|f| &f.name == name)
        .unwrap();
    assert_eq!(target.parameters[0].ty, Type::Scalar(Scalar::Float));
    assert!(compiled.ir.functions.iter().any(|f| {
        f.parameters
            .first()
            .is_some_and(|p| p.ty == Type::Scalar(Scalar::Int))
            && f.name != *name
    }));
    assert!(compiled.ir.globals.iter().all(|g| &g.name != name));
    fails(
        "import lib;int2 run(){return select(int2(1));}",
        &[(
            "lib.slang",
            "module lib;public float select(float x){return x;}int2 select(int2 x){return x;}",
        )],
        "T014",
    );
}

#[test]
fn generated_module_names_cannot_collide_with_source_identifiers() {
    let compiled = compile("import lib;float run(){float _slang_module_1_value=3;return value()+_slang_module_1_value;}", &[
        ("lib.slang", "module lib;public float value(){return 2;}"),
    ]).unwrap();
    let name = &compiled.ir.functions[0].name;
    assert_ne!(name, "_slang_module_1_value");
    assert!(name.starts_with("_slang_module__"));
}

#[test]
fn module_members_allow_cycles_and_forward_references_once() {
    let compiled = compile("import lib;float run(){return publicValue();}", &[
        ("lib.slang", "module lib;__include lib;__include a;__include a;public float publicValue(){return aValue()+bValue();}"),
        ("a.slang", "implementing lib;__include b;float aValue(){return bValue();}"),
        ("b.slang", "implementing \"lib.slang\";__include a;float bValue(){return 2;}"),
    ]).unwrap();
    assert_eq!(compiled.ir.functions.len(), 4);
    assert_eq!(compiled.reflection.modules.len(), 2);
    assert_eq!(compiled.reflection.modules[1].files.len(), 3);
    assert_eq!(compiled.sources.files().len(), 4);
}

#[test]
fn macros_are_isolated_between_modules_and_members() {
    let mut options = CompileOptions::default();
    options.defines.insert("COMMAND_VALUE".into(), "7".into());
    let files = MemoryIncludes {files: [
        ("lib.slang".into(), "module lib;\n#define LOCAL_VALUE 10\n__include helper;\npublic float read(){return member()+LOCAL_VALUE;}".into()),
        ("helper.slang".into(), "implementing lib;\n#ifdef LOCAL_VALUE\n#error module macro leaked\n#endif\n#define MEMBER_VALUE 1\nfloat member(){return COMMAND_VALUE;}".into()),
    ].into()};
    let compiled = compile_source("main.slang",
        "#define LOCAL_VALUE 3\nimport lib;\n#ifdef MEMBER_VALUE\n#error member macro leaked\n#endif\nfloat run(){return read()+LOCAL_VALUE;}",
        &options, &files).unwrap();
    let code = compiled.artifact.unwrap();
    assert!(code.contains("return 7;"));
    assert!(code.contains(" + 10)"));
    assert!(code.contains(" + 3)"));
}

#[test]
fn invalid_module_graphs_report_the_requesting_source() {
    let error = fails("import missing;", &[], "M001");
    assert_eq!(error.diagnostics[0].span.source.0, 0);
    fails(
        "import a;",
        &[
            ("a.slang", "module a;import b;"),
            ("b.slang", "module b;import a;"),
        ],
        "M002",
    );
    fails(
        "import helper;",
        &[("helper.slang", "implementing lib;float h(){return 1;}")],
        "M003",
    );
    for fragment in [
        "float h(){return 1;}",
        "implementing other;float h(){return 1;}",
    ] {
        fails(
            "import lib;",
            &[
                ("lib.slang", "module lib;__include helper;"),
                ("helper.slang", fragment),
            ],
            "M003",
        );
    }
    fails(
        "import lib;import other;",
        &[
            ("lib.slang", "module lib;__include helper;"),
            ("other.slang", "module other;__include helper;"),
            ("helper.slang", "implementing lib;"),
        ],
        "M003",
    );
    fails(
        "import lib;import helper;",
        &[
            ("lib.slang", "module lib;__include helper;"),
            ("helper.slang", "implementing lib;"),
        ],
        "M003",
    );
}

#[test]
fn visibility_signatures_and_ambiguous_imports_are_validated() {
    for library in [
        "module lib;struct Hidden{float x;};public Hidden read(){Hidden h;return h;}",
        "module lib;struct Hidden{float x;};public struct Visible{public Hidden h;};",
        "module lib;struct Hidden{float x;};public StructuredBuffer<Hidden> data;",
        "module lib;internal struct Hidden{public float x;};",
        "module lib;public internal float read(){return 1;}",
    ] {
        fails("import lib;", &[("lib.slang", library)], "T018");
    }
    fails(
        "import a;import b;float run(){return value;}",
        &[
            ("a.slang", "module a;public static const float value=1;"),
            ("b.slang", "module b;public static const float value=2;"),
        ],
        "M004",
    );
    compile(
        "import a;float value(){return 3;}float run(){return value();}",
        &[("a.slang", "module a;public float value(){return 1;}")],
    )
    .unwrap();
}

#[test]
fn legacy_visibility_and_module_path_spellings_follow_slang() {
    let compiled = compile("import folder.math_utils;import \"folder/math-utils\";float run(){Pair p;p.value=2;return read(p);}", &[
        ("folder/math-utils.slang", "struct Pair{float value;};float read(Pair p){return p.value;}"),
    ]).unwrap();
    assert_eq!(compiled.reflection.modules.len(), 2);
    assert!(compiled.reflection.modules[1].legacy_visibility);
    fails(
        "import lib;float run(){return read();}",
        &[(
            "lib.slang",
            "public float visible(){return 1;}float read(){return 2;}",
        )],
        "T014",
    );
}

#[test]
fn imported_modules_cannot_read_unimported_root_declarations() {
    for (source, library, code) in [
        (
            "static float value=3;",
            "module lib;public float read(){return value;}",
            "T011",
        ),
        (
            "float hidden(){return 3;}",
            "module lib;public float read(){return hidden();}",
            "T014",
        ),
        (
            "struct Root{float x;};",
            "module lib;public float read(){Root r;return r.x;}",
            "T002",
        ),
        (
            "typealias Root=float;",
            "module lib;public float read(){Root r=2;return r;}",
            "T002",
        ),
        (
            "struct Root{float x;};",
            "module lib;public struct Foreign{public Root value;};public float read(){return 1;}",
            "T002",
        ),
        (
            "typealias Root=float;",
            "module lib;typealias Foreign=Root;public float read(){Foreign r=2;return r;}",
            "T002",
        ),
    ] {
        fails(
            &format!("import lib;{source}float run(){{return read();}}"),
            &[("lib.slang", library)],
            code,
        );
    }
    compile(
        "import lib;static float value=3;float run(){return read(2);}",
        &[(
            "lib.slang",
            "module lib;public float read(float value){return value;}",
        )],
    )
    .unwrap();
}
