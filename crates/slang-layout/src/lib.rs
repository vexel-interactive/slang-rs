//! Resource binding models shared by compiler phases without AST dependencies.

mod register;

pub use register::{MAX_REGISTER_INDEX, RegisterBinding, RegisterClass, ResourceBinding};
