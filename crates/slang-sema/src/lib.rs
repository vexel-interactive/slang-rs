//! Semantic checking is target-independent and never accepts error types as output.

mod alias;
mod binding;
mod call;
mod capability;
mod checker;
mod constant;
mod constant_buffer;
mod declaration;
mod default_argument;
mod entry;
mod expression;
mod initialization;
mod initializer_list;
mod interface;
mod intrinsic;
mod resource;
mod statement;
mod swizzle;
mod types;
mod value_type;
mod visibility;

pub use capability::{EntryPointRequirements, RequiredOperation};
pub use checker::{CheckedModule, EntryPoint, ShaderStage, check, check_program, span_key};
pub use initialization::{Initialization, InitializationKind};
pub use intrinsic::is_intrinsic_name;
pub use types::{Scalar, TextureDimension, Type};
