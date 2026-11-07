use slang_compiler::{CompileOptions, MemoryIncludes, ShaderProfile, Target, compile_source};
use slang_sema::ShaderStage;

const SOURCE: &str = "RWStructuredBuffer<float> data;[shader(\"compute\")][numthreads(4,1,1)]void main(uint index:SV_GroupIndex){data[index]=ddx(float(index));}";

#[test]
fn explicit_compute_profiles_upgrade_to_derivative_requirements() {
    for profile in ["cs_6_0", "cs_6_5", "cs_6_6", "cs_6_8"] {
        let options = CompileOptions {
            profile: Some(profile.parse().unwrap()),
            ..CompileOptions::default()
        };
        let compiled = compile_source(
            "profile.slang",
            SOURCE,
            &options,
            &MemoryIncludes::default(),
        )
        .unwrap();
        let expected = if matches!(profile, "cs_6_0" | "cs_6_5") {
            "cs_6_6"
        } else {
            profile
        };
        assert_eq!(
            compiled.reflection.entry_points[0]
                .target_profile
                .as_deref(),
            Some(expected)
        );
        assert_eq!(
            compiled
                .diagnostics
                .iter()
                .filter(|d| d.code == "C003")
                .count(),
            usize::from(expected != profile)
        );
        let reflection = serde_json::to_value(&compiled.reflection).unwrap();
        assert_eq!(reflection["entry_points"][0]["target_profile"], expected);
    }
}

#[test]
fn profiles_reject_stage_mismatches_and_apply_to_the_selected_entry() {
    let source = "[shader(\"compute\")][numthreads(1,1,1)]void computeMain(){}[shader(\"fragment\")]float4 fragmentMain():SV_Target{return float4(1);}";
    let options = CompileOptions {
        entry_point: Some("fragmentMain".into()),
        profile: Some("ps_6_4".parse().unwrap()),
        ..CompileOptions::default()
    };
    let compiled = compile_source(
        "profile.slang",
        source,
        &options,
        &MemoryIncludes::default(),
    )
    .unwrap();
    assert_eq!(
        compiled.reflection.entry_points[0]
            .target_profile
            .as_deref(),
        Some("cs_6_0")
    );
    assert_eq!(
        compiled.reflection.entry_points[1]
            .target_profile
            .as_deref(),
        Some("ps_6_4")
    );
    let options = CompileOptions {
        profile: Some("ps_6_4".parse().unwrap()),
        ..CompileOptions::default()
    };
    let error = compile_source(
        "profile.slang",
        source,
        &options,
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "C004"));
}

#[test]
fn shader_profiles_validate_spelling_stage_and_model() {
    let profile: ShaderProfile = "cs_6_6".parse().unwrap();
    assert_eq!(profile.stage(), ShaderStage::Compute);
    assert_eq!(profile.shader_model(), [6, 6]);
    for value in [
        "cs_5_0",
        "ps_6_10",
        "gs_6_0",
        "sm_6_0",
        "vs_6_01",
        "cs_6",
        "cs_6_6_extra",
    ] {
        assert!(value.parse::<ShaderProfile>().is_err(), "{value}");
    }
}

#[test]
fn check_only_keeps_profile_requirements_and_other_targets_are_explicit() {
    let options = CompileOptions {
        check_only: true,
        ..CompileOptions::default()
    };
    let compiled = compile_source(
        "profile.slang",
        SOURCE,
        &options,
        &MemoryIncludes::default(),
    )
    .unwrap();
    assert!(compiled.artifact.is_none());
    assert_eq!(
        compiled.reflection.entry_points[0]
            .target_profile
            .as_deref(),
        Some("cs_6_6")
    );
    let options = CompileOptions {
        target: Target::Spirv,
        profile: Some("cs_6_6".parse().unwrap()),
        check_only: true,
        ..CompileOptions::default()
    };
    let error = compile_source(
        "profile.slang",
        SOURCE,
        &options,
        &MemoryIncludes::default(),
    )
    .unwrap_err();
    assert!(error.diagnostics.iter().any(|d| d.code == "C004"));
}
