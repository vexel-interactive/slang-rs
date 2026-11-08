use slang_sema::{Scalar, Type};

/// Emit typed integer bit patterns. Splitting signed minima avoids HLSL's
/// unsigned type selection for the magnitude of a negative minimum literal.
pub(crate) fn literal_text(bits: i64, ty: &Type) -> String {
    match ty.scalar() {
        Some(Scalar::Uint) => format!("{}u", bits as u32),
        Some(Scalar::Uint64) => format!("{}ull", bits as u64),
        Some(Scalar::Int64) if bits == i64::MIN => "(-9223372036854775807ll - 1ll)".into(),
        Some(Scalar::Int64) => format!("{bits}ll"),
        Some(Scalar::Int) if bits as i32 == i32::MIN => "(-2147483647 - 1)".into(),
        Some(Scalar::Int) => (bits as i32).to_string(),
        _ => bits.to_string(),
    }
}
