use crate::CompileOptions;
use slang_codegen::Target;
use slang_sema::{CheckedModule, ShaderStage};
use slang_source::{Diagnostic, Span};
use std::{collections::BTreeMap, fmt, str::FromStr};

/// HLSL shader model 6 profile for an implemented execution stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShaderProfile {
    stage: ShaderStage,
    minor: u8,
}

impl ShaderProfile {
    pub fn stage(self) -> ShaderStage {
        self.stage
    }

    pub fn shader_model(self) -> [u8; 2] {
        [6, self.minor]
    }
}

impl FromStr for ShaderProfile {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = value.split('_').collect();
        let [stage, "6", minor] = parts.as_slice() else {
            return Err(format!(
                "unsupported shader profile '{value}'; use cs_6_N, vs_6_N, or ps_6_N"
            ));
        };
        let stage = match *stage {
            "cs" => ShaderStage::Compute,
            "vs" => ShaderStage::Vertex,
            "ps" => ShaderStage::Fragment,
            _ => return Err(format!("unsupported shader profile stage '{stage}'")),
        };
        let minor = minor
            .parse::<u8>()
            .ok()
            .filter(|n| *n <= 9 && minor.len() == 1)
            .ok_or_else(|| {
                format!(
                    "unsupported shader model in '{value}'; supported profiles are 6.0 through 6.9"
                )
            })?;
        Ok(Self { stage, minor })
    }
}

impl fmt::Display for ShaderProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let stage = match self.stage {
            ShaderStage::Compute => "cs",
            ShaderStage::Vertex => "vs",
            ShaderStage::Fragment => "ps",
        };
        write!(f, "{stage}_6_{}", self.minor)
    }
}

pub(crate) struct Profiles {
    pub entries: BTreeMap<String, ShaderProfile>,
    pub diagnostics: Vec<Diagnostic>,
    pub native_16bit_types: bool,
}

/// Select minimum HLSL profiles without imposing target rules on semantic types.
pub(crate) fn resolve(
    checked: &CheckedModule,
    options: &CompileOptions,
) -> Result<Profiles, Vec<Diagnostic>> {
    let mut profiles = Profiles {
        entries: BTreeMap::new(),
        diagnostics: vec![],
        native_16bit_types: options.native_16bit_types
            || options.profile.is_some_and(|p| p.minor >= 2)
            || checked
                .declaration_types
                .values()
                .chain(checked.expression_types.values())
                .any(slang_sema::Type::requires_native_16bit),
    };
    if options.target != Target::Hlsl {
        return if options.profile.is_some() || options.native_16bit_types {
            Err(vec![Diagnostic::error(
                "C004",
                Span::default(),
                "shader profile and native 16-bit selection currently support only the HLSL target",
            )])
        } else {
            Ok(profiles)
        };
    }
    let mut errors = vec![];
    for entry in &checked.entry_points {
        let entry_span = checked
            .module
            .declarations
            .iter()
            .find_map(|declaration| {
                if let slang_syntax::Declaration::Function(function) = declaration
                    && function.name == entry.name
                {
                    Some(function.span)
                } else {
                    None
                }
            })
            .unwrap_or_default();
        let requested = options.profile.filter(|_| {
            options
                .entry_point
                .as_ref()
                .is_none_or(|name| name == &entry.name)
        });
        if let Some(requested) = requested
            && requested.stage != entry.stage
        {
            errors.push(Diagnostic::error(
                "C004",
                entry_span,
                format!(
                    "profile '{requested}' does not match the stage of entry point '{}'",
                    entry.name
                ),
            ));
            continue;
        }
        let mut selected = requested.unwrap_or(ShaderProfile {
            stage: entry.stage,
            minor: 0,
        });
        if profiles.native_16bit_types && selected.minor < 2 {
            selected.minor = 2;
            if let Some(requested) = requested {
                profiles.diagnostics.push(Diagnostic::warning("C003", entry_span,
                    format!("profile '{requested}' upgraded to '{selected}' because native 16-bit types are enabled")));
            }
        }
        if let Some(operation) = checked
            .entry_point_requirements
            .get(&entry.name)
            .and_then(|r| r.derivatives)
            && entry.stage == ShaderStage::Compute
            && selected.minor < 6
        {
            selected.minor = 6;
            if let Some(requested) = requested {
                profiles.diagnostics.push(Diagnostic::warning("C003", operation.span,
                    format!("profile '{requested}' upgraded to '{selected}' because entry point '{}' uses compute derivatives", entry.name)));
            }
        }
        profiles.entries.insert(entry.name.clone(), selected);
    }
    // One emitted module has one physical type mode, including helpers and
    // globals shared by entries with different feature requirements.
    profiles.native_16bit_types |= profiles.entries.values().any(|profile| profile.minor >= 2);
    if profiles.native_16bit_types {
        for profile in profiles.entries.values_mut() {
            profile.minor = profile.minor.max(2);
        }
    }
    if errors.is_empty() {
        Ok(profiles)
    } else {
        Err(errors)
    }
}
