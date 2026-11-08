//! Resource binding models shared by compiler phases without AST dependencies.

mod allocator;
mod register;
mod uniform;

pub use allocator::{AllocationError, RegisterAllocator};
pub use register::{MAX_REGISTER_INDEX, RegisterBinding, RegisterClass, ResourceBinding};
pub use uniform::{
    UniformFieldLayout, UniformLayoutError, UniformPacker, UniformTypeLayout, UniformVariableLayout,
};
