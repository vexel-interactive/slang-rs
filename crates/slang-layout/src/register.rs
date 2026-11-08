use serde::Serialize;
use std::fmt;

/// Largest nonnegative index preserved by the upstream signed register model.
pub const MAX_REGISTER_INDEX: u32 = i32::MAX as u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum RegisterClass {
    #[serde(rename = "b")]
    ConstantBuffer,
    #[serde(rename = "t")]
    ShaderResource,
    #[serde(rename = "u")]
    UnorderedAccess,
    #[serde(rename = "s")]
    Sampler,
    /// Legacy source annotation, never an allocated shader resource.
    #[serde(rename = "c")]
    LegacyUniform,
}

impl RegisterClass {
    pub fn from_prefix(prefix: char) -> Option<Self> {
        match prefix {
            'b' => Some(Self::ConstantBuffer),
            't' => Some(Self::ShaderResource),
            'u' => Some(Self::UnorderedAccess),
            's' => Some(Self::Sampler),
            'c' => Some(Self::LegacyUniform),
            _ => None,
        }
    }
}

impl fmt::Display for RegisterClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ConstantBuffer => "b",
            Self::ShaderResource => "t",
            Self::UnorderedAccess => "u",
            Self::Sampler => "s",
            Self::LegacyUniform => "c",
        })
    }
}

/// A source register annotation. Omitted register spaces normalize to zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct RegisterBinding {
    pub class: RegisterClass,
    pub index: u32,
    pub space: u32,
}

impl fmt::Display for RegisterBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "register({}{}", self.class, self.index)?;
        if self.space != 0 {
            write!(f, ", space{}", self.space)?;
        }
        f.write_str(")")
    }
}

/// A checked contiguous resource range in one register class and space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ResourceBinding {
    #[serde(flatten)]
    pub register: RegisterBinding,
    pub count: u32,
    /// Whether the source selected this range with a register annotation.
    pub explicit: bool,
}

impl ResourceBinding {
    /// Exclusive end, kept wide so the final supported register cannot wrap.
    pub fn end(self) -> u64 {
        u64::from(self.register.index) + u64::from(self.count)
    }
}
