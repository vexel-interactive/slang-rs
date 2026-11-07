//! Backend dispatch. Unsupported targets fail instead of emitting placeholder artifacts.

mod hlsl;

use serde::Serialize;
use slang_source::{Diagnostic, Span};
use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    #[default]
    Hlsl,
    Spirv,
    Dxil,
    Glsl,
    Wgsl,
    Metal,
    Cuda,
    Cpp,
}

impl Target {
    pub fn is_implemented(self) -> bool {
        self == Self::Hlsl
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Hlsl => "hlsl",
                Self::Spirv => "spirv",
                Self::Dxil => "dxil",
                Self::Glsl => "glsl",
                Self::Wgsl => "wgsl",
                Self::Metal => "metal",
                Self::Cuda => "cuda",
                Self::Cpp => "cpp",
            }
        )
    }
}

impl FromStr for Target {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "hlsl" => Ok(Self::Hlsl),
            "spirv" => Ok(Self::Spirv),
            "dxil" => Ok(Self::Dxil),
            "glsl" => Ok(Self::Glsl),
            "wgsl" => Ok(Self::Wgsl),
            "metal" => Ok(Self::Metal),
            "cuda" => Ok(Self::Cuda),
            "cpp" => Ok(Self::Cpp),
            _ => Err(format!("unknown target '{text}'")),
        }
    }
}

pub fn emit(module: &slang_ir::Module, target: Target) -> Result<String, Diagnostic> {
    if !target.is_implemented() {
        return Err(Diagnostic::error(
            "G001",
            Span::default(),
            format!("target '{target}' has not been implemented"),
        ));
    }
    Ok(hlsl::emit(module))
}
