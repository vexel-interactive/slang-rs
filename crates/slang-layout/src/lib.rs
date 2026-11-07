//! Resource binding models shared by compiler phases without AST dependencies.

mod allocator;
mod register;

pub use allocator::{AllocationError, RegisterAllocator};
pub use register::{MAX_REGISTER_INDEX, RegisterBinding, RegisterClass, ResourceBinding};
