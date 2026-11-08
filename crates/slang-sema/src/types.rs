use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Scalar {
    Bool,
    Int,
    Uint,
    Int64,
    Uint64,
    IntPtr,
    UintPtr,
    Half,
    Float,
    Double,
}

impl Scalar {
    pub fn is_pointer_sized(self) -> bool {
        matches!(self, Self::IntPtr | Self::UintPtr)
    }

    /// HLSL uses the upstream target's 64-bit generic pointer size, independent
    /// of the compiler host. Pointer integers retain their own type identity.
    pub fn bit_width(self) -> u8 {
        match self {
            Self::Bool => 1,
            Self::Half => 16,
            Self::Int | Self::Uint | Self::Float => 32,
            Self::Int64 | Self::Uint64 | Self::IntPtr | Self::UintPtr | Self::Double => 64,
        }
    }

    /// Builtin arithmetic unification preserves operand order for distinct
    /// equal-width integers: signed selects the right, unsigned the left.
    pub fn common(self, other: Self) -> Self {
        if self == other || other == Self::Bool {
            return self;
        }
        if self == Self::Bool {
            return other;
        }
        let floating = |s| matches!(s, Self::Half | Self::Float | Self::Double);
        match (floating(self), floating(other)) {
            (true, false) => self,
            (false, true) => other,
            (true, true) => {
                if self.bit_width() >= other.bit_width() {
                    self
                } else {
                    other
                }
            }
            (false, false) => {
                if self.bit_width() > other.bit_width() {
                    self
                } else if self.bit_width() < other.bit_width()
                    || matches!(self, Self::Int | Self::Int64 | Self::IntPtr)
                {
                    other
                } else {
                    self
                }
            }
        }
    }

    /// Generic inference joins by conversion cost rather than arithmetic size.
    pub fn inferred_common(self, other: Self) -> Self {
        let a = Type::Scalar(self);
        let b = Type::Scalar(other);
        if b.conversion_cost(&a) > a.conversion_cost(&b) {
            other
        } else {
            self
        }
    }
}

/// Texture topology supported by the HLSL resource model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TextureDimension {
    D1,
    D2,
    D3,
    Cube,
}

impl TextureDimension {
    pub fn coordinates(self) -> u8 {
        match self {
            Self::D1 => 1,
            Self::D2 => 2,
            Self::D3 | Self::Cube => 3,
        }
    }

    pub fn dimensions(self) -> usize {
        match self {
            Self::D1 => 1,
            Self::D2 | Self::Cube => 2,
            Self::D3 => 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Type {
    /// A contextual source expression; never a concrete IR or declaration type.
    InitializerList,
    Void,
    Scalar(Scalar),
    Vector(Scalar, u8),
    /// Component type, row count, and column count; indexing selects a row.
    Matrix(Scalar, u8, u8),
    Array(Box<Type>, u32),
    Struct(String),
    ConstantBuffer {
        element: Box<Type>,
        #[serde(skip_serializing_if = "Option::is_none")]
        matrix_major: Option<slang_layout::MatrixMajor>,
    },
    ByteBuffer {
        writable: bool,
    },
    Buffer {
        element: Box<Type>,
        writable: bool,
        /// An explicit element layout differing from the compilation default.
        #[serde(skip_serializing_if = "Option::is_none")]
        matrix_major: Option<slang_layout::MatrixMajor>,
    },
    Texture {
        element: Box<Type>,
        dimension: TextureDimension,
        writable: bool,
    },
    Sampler {
        comparison: bool,
    },
    /// Used only for named callees; function values are not implemented.
    Callable {
        parameters: Vec<Type>,
        result: Box<Type>,
    },
    Meta(Box<Type>),
    Error,
}

impl Type {
    /// A fixed-array chain containing a zero dimension has no elements to store.
    pub fn is_empty_array(&self) -> bool {
        let mut ty = self;
        while let Self::Array(element, size) = ty {
            if *size == 0 {
                return true;
            }
            ty = element;
        }
        false
    }

    pub fn builtin(name: &str) -> Option<Self> {
        if matches!(name, "ByteAddressBuffer" | "RWByteAddressBuffer") {
            return Some(Self::ByteBuffer {
                writable: name.starts_with("RW"),
            });
        }
        if let Some(scalar) = match name {
            "intptr_t" | "ssize_t" => Some(Scalar::IntPtr),
            "uintptr_t" | "size_t" | "usize_t" => Some(Scalar::UintPtr),
            _ => None,
        } {
            return Some(Self::Scalar(scalar));
        }
        if let Some(texture) = Self::texture(name, Self::Vector(Scalar::Float, 4)) {
            return Some(texture);
        }
        if matches!(name, "SamplerState" | "SamplerComparisonState") {
            return Some(Self::Sampler {
                comparison: name == "SamplerComparisonState",
            });
        }
        if name == "void" {
            return Some(Self::Void);
        }
        for (prefix, scalar) in [
            ("float16_t", Scalar::Half),
            ("float32_t", Scalar::Float),
            ("float64_t", Scalar::Double),
            ("int32_t", Scalar::Int),
            ("uint32_t", Scalar::Uint),
            ("int64_t", Scalar::Int64),
            ("uint64_t", Scalar::Uint64),
            ("bool", Scalar::Bool),
            ("int", Scalar::Int),
            ("uint", Scalar::Uint),
            ("half", Scalar::Half),
            ("float", Scalar::Float),
            ("double", Scalar::Double),
        ] {
            if name == prefix {
                return Some(Self::Scalar(scalar));
            }
            if let Some(suffix) = name.strip_prefix(prefix)
                && let Ok(size @ 1..=4) = suffix.parse::<u8>()
            {
                return Some(Self::Vector(scalar, size));
            }
            if let Some(suffix) = name.strip_prefix(prefix)
                && let Some((rows, columns)) = suffix.split_once('x')
                && let (Ok(rows @ 1..=4), Ok(columns @ 1..=4)) =
                    (rows.parse::<u8>(), columns.parse::<u8>())
            {
                return Some(Self::Matrix(scalar, rows, columns));
            }
        }
        None
    }

    /// Recognize a texture alias without resolving or validating its element type.
    pub fn texture(name: &str, element: Self) -> Option<Self> {
        let (name, writable) = name
            .strip_prefix("RW")
            .map_or((name, false), |name| (name, true));
        let dimension = match name {
            "Texture1D" => TextureDimension::D1,
            "Texture2D" => TextureDimension::D2,
            "Texture3D" => TextureDimension::D3,
            "TextureCube" if !writable => TextureDimension::Cube,
            _ => return None,
        };
        Some(Self::Texture {
            element: Box::new(element),
            dimension,
            writable,
        })
    }

    pub fn is_resource(&self) -> bool {
        if let Self::Array(element, _) = self {
            return element.is_resource();
        }
        matches!(
            self,
            Self::ConstantBuffer { .. }
                | Self::ByteBuffer { .. }
                | Self::Buffer { .. }
                | Self::Texture { .. }
                | Self::Sampler { .. }
        )
    }

    pub fn numeric_shape(scalar: Scalar, width: u8) -> Self {
        if width == 1 {
            Self::Scalar(scalar)
        } else {
            Self::Vector(scalar, width)
        }
    }

    pub fn scalar(&self) -> Option<Scalar> {
        match self {
            Self::Scalar(s) | Self::Vector(s, _) | Self::Matrix(s, _, _) => Some(*s),
            _ => None,
        }
    }

    pub fn width(&self) -> Option<u8> {
        match self {
            Self::Scalar(_) => Some(1),
            Self::Vector(_, n) => Some(*n),
            _ => None,
        }
    }

    pub fn numeric(&self) -> bool {
        self.scalar().is_some_and(|s| s != Scalar::Bool)
    }

    /// Number of scalar components; unlike `width`, this includes matrices.
    pub fn components(&self) -> Option<u16> {
        match self {
            Self::Matrix(_, rows, columns) => Some(u16::from(*rows) * u16::from(*columns)),
            _ => self.width().map(u16::from),
        }
    }

    /// Preserve this value's shape while replacing its component type.
    pub fn with_scalar(&self, scalar: Scalar) -> Self {
        match self {
            Self::Scalar(_) => Self::Scalar(scalar),
            Self::Vector(_, width) => Self::Vector(scalar, *width),
            Self::Matrix(_, rows, columns) => Self::Matrix(scalar, *rows, *columns),
            _ => Self::Error,
        }
    }
    pub fn integral(&self) -> bool {
        matches!(
            self.scalar(),
            Some(
                Scalar::Int
                    | Scalar::Uint
                    | Scalar::Int64
                    | Scalar::Uint64
                    | Scalar::IntPtr
                    | Scalar::UintPtr
            )
        )
    }

    pub fn value_type(&self) -> &Self {
        match self {
            Self::ConstantBuffer { element, .. } => element,
            _ => self,
        }
    }

    pub(crate) fn conversion_cost(&self, target: &Self) -> usize {
        if self == target || *self == Self::InitializerList {
            0
        } else if matches!(self, Self::ConstantBuffer { .. }) && !target.is_resource() {
            1 + usize::from(self.value_type() != target)
        } else if let (Some(from), Some(to)) = (self.scalar(), target.scalar()) {
            let info = |scalar| match scalar {
                Scalar::Bool => (0, 1),
                Scalar::Int => (1, 32),
                Scalar::Int64 => (1, 64),
                Scalar::IntPtr => (1, 48),
                Scalar::Uint => (2, 32),
                Scalar::Uint64 => (2, 64),
                Scalar::UintPtr => (2, 48),
                Scalar::Half => (3, 16),
                Scalar::Float => (3, 32),
                Scalar::Double => (3, 64),
            };
            let (from_kind, from_rank) = info(from);
            let (to_kind, to_rank) = info(to);
            // Logical conversion ranks place pointer integers between 32 and 64 bits.
            // Core scalar conversion costs precede scalar/shape conversion.
            let cost = if from == to {
                0
            } else if from_kind == to_kind && to_rank > from_rank {
                150
            } else if from_kind == 2
                && to_kind == 1
                && to_rank > from_rank
                && !from.is_pointer_sized()
                && !to.is_pointer_sized()
            {
                200
            } else if from_kind == 1 && to_kind == 2 && to_rank >= from_rank {
                250
            } else if from_kind == 2
                && to_kind == 1
                && to_rank == from_rank
                && !from.is_pointer_sized()
                && !to.is_pointer_sized()
            {
                300
            } else if matches!(from_kind, 1 | 2) && to_kind == 3 {
                if to_rank == 16 { 500 } else { 400 }
            } else {
                900
            };
            cost + if matches!(self, Self::Scalar(_))
                && matches!(target, Self::Vector(..) | Self::Matrix(..))
            {
                2
            } else {
                usize::from(self != target && from == to)
            }
        } else {
            1
        }
    }

    pub(crate) fn literal_conversion_cost(
        &self,
        target: &Self,
        literal: Option<slang_lexer::IntegerLiteral>,
    ) -> usize {
        let cost = self.conversion_cost(target);
        if cost == 0 || !matches!(target, Self::Scalar(_)) || !target.integral() {
            return cost;
        }
        let Some(literal) = literal else { return cost };
        // Upstream stores every literal as signed bits when counting magnitude.
        let magnitude = (literal.bits as i64).unsigned_abs();
        let bits = (64 - magnitude.leading_zeros()).max(1);
        if bits > u32::from(target.scalar().unwrap().bit_width()) {
            return cost;
        }
        let unsigned = |s| matches!(s, Scalar::Uint | Scalar::Uint64 | Scalar::UintPtr);
        match (self.scalar().map(unsigned), target.scalar().map(unsigned)) {
            (Some(a), Some(b)) if a == b => 23,
            (_, Some(false)) => 81,
            _ => 32,
        }
    }

    pub fn convertible_to(&self, target: &Self) -> bool {
        if let Self::ConstantBuffer { element, .. } = self
            && !target.is_resource()
        {
            return element.convertible_to(target);
        }
        if self == target || *self == Self::Error || *target == Self::Error {
            return true;
        }
        if matches!(
            (self, target),
            (Self::Scalar(Scalar::Bool), Self::Vector(Scalar::Bool, 1))
                | (Self::Vector(Scalar::Bool, 1), Self::Scalar(Scalar::Bool))
        ) {
            return true;
        }
        if let Self::Matrix(b, rows, columns) = target {
            return *b != Scalar::Bool
                && match self {
                    Self::Scalar(a) => *a != Scalar::Bool,
                    Self::Matrix(a, r, c) => *a != Scalar::Bool && (r, c) == (rows, columns),
                    _ => false,
                };
        }
        if let (Some(a), Some(b), Some(aw), Some(bw)) =
            (self.scalar(), target.scalar(), self.width(), target.width())
        {
            return (aw == bw || aw == 1) && a != Scalar::Bool && b != Scalar::Bool;
        }
        false
    }

    pub fn common(a: &Self, b: &Self) -> Option<Self> {
        if *a == Self::Error || *b == Self::Error {
            return Some(Self::Error);
        }
        let (sa, sb) = (a.scalar()?, b.scalar()?);
        if sa == Scalar::Bool || sb == Scalar::Bool {
            if matches!(
                (a, b),
                (Self::Scalar(Scalar::Bool), Self::Vector(Scalar::Bool, 1))
                    | (Self::Vector(Scalar::Bool, 1), Self::Scalar(Scalar::Bool))
            ) {
                return Some(Self::Vector(Scalar::Bool, 1));
            }
            return (a == b).then(|| a.clone());
        }
        if let Self::Matrix(_, rows, columns) = a {
            return match b {
                Self::Scalar(_) => Some(a.with_scalar(sa.common(sb))),
                Self::Matrix(_, r, c) if (r, c) == (rows, columns) => {
                    Some(a.with_scalar(sa.common(sb)))
                }
                _ => None,
            };
        }
        if matches!((a, b), (Self::Scalar(_), Self::Matrix(..))) {
            return Some(b.with_scalar(sa.common(sb)));
        }
        let (wa, wb) = (a.width()?, b.width()?);
        if wa != wb && wa != 1 && wb != 1 {
            return None;
        }
        let scalar = sa.common(sb);
        let width = wa.max(wb);
        Some(
            if width == 1 && matches!((a, b), (Self::Scalar(_), Self::Scalar(_))) {
                Self::Scalar(scalar)
            } else {
                Self::Vector(scalar, width)
            },
        )
    }

    pub fn inferred_common(a: &Self, b: &Self) -> Option<Self> {
        let shape = Self::common(a, b)?;
        if shape == Self::Error {
            return Some(shape);
        }
        let (sa, sb) = (a.scalar()?, b.scalar()?);
        // Generic scalar/vector joins consider the vector element first.
        let scalar = if matches!((a, b), (Self::Scalar(_), Self::Vector(..))) {
            sb.inferred_common(sa)
        } else {
            sa.inferred_common(sb)
        };
        Some(shape.with_scalar(scalar))
    }

    pub fn boolean_shape(&self) -> Self {
        self.with_scalar(Scalar::Bool)
    }
}

impl fmt::Display for Scalar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Uint => "uint",
            Self::Int64 => "int64_t",
            Self::Uint64 => "uint64_t",
            Self::IntPtr => "intptr_t",
            Self::UintPtr => "uintptr_t",
            Self::Half => "half",
            Self::Float => "float",
            Self::Double => "double",
        })
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Void => f.write_str("void"),
            Self::InitializerList => f.write_str("<initializer list>"),
            Self::Error => f.write_str("<error>"),
            Self::Scalar(s) => write!(f, "{s}"),
            Self::Vector(s, n) if s.is_pointer_sized() => write!(f, "vector<{s}, {n}>"),
            Self::Vector(s, n) => write!(f, "{s}{n}"),
            Self::Matrix(s, rows, columns) if s.is_pointer_sized() => {
                write!(f, "matrix<{s}, {rows}, {columns}>")
            }
            Self::Matrix(s, rows, columns) => write!(f, "{s}{rows}x{columns}"),
            Self::Array(..) => {
                let mut ty = self;
                let mut suffix = String::new();
                while let Self::Array(element, n) = ty {
                    suffix.push_str(&format!("[{n}]"));
                    ty = element;
                }
                write!(f, "{ty}{suffix}")
            }
            Self::Struct(name) => f.write_str(name),
            Self::ConstantBuffer {
                element,
                matrix_major,
            } => write!(
                f,
                "ConstantBuffer<{}{element}>",
                match matrix_major {
                    Some(slang_layout::MatrixMajor::Row) => "row_major ",
                    Some(slang_layout::MatrixMajor::Column) => "column_major ",
                    None => "",
                }
            ),
            Self::ByteBuffer { writable } => {
                write!(f, "{}ByteAddressBuffer", if *writable { "RW" } else { "" })
            }
            Self::Buffer {
                element,
                writable,
                matrix_major,
            } => write!(
                f,
                "{}StructuredBuffer<{}{element}>",
                if *writable { "RW" } else { "" },
                match matrix_major {
                    Some(slang_layout::MatrixMajor::Row) => "row_major ",
                    Some(slang_layout::MatrixMajor::Column) => "column_major ",
                    None => "",
                }
            ),
            Self::Texture {
                element,
                dimension,
                writable,
            } => write!(
                f,
                "{}Texture{}<{element}>",
                if *writable { "RW" } else { "" },
                match dimension {
                    TextureDimension::D1 => "1D",
                    TextureDimension::D2 => "2D",
                    TextureDimension::D3 => "3D",
                    TextureDimension::Cube => "Cube",
                }
            ),
            Self::Sampler { comparison } => f.write_str(if *comparison {
                "SamplerComparisonState"
            } else {
                "SamplerState"
            }),
            Self::Callable { .. } => f.write_str("<function>"),
            Self::Meta(ty) => write!(f, "<type {ty}>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Scalar::*, Type};

    #[test]
    fn pointer_arithmetic_and_inference_use_distinct_common_type_rules() {
        for (a, b, arithmetic, inferred) in [
            (IntPtr, Int64, Int64, Int64),
            (Int64, IntPtr, IntPtr, Int64),
            (UintPtr, Uint64, UintPtr, Uint64),
            (Uint64, UintPtr, Uint64, Uint64),
            (IntPtr, Uint64, Uint64, Uint64),
            (UintPtr, Int64, UintPtr, UintPtr),
            (IntPtr, Uint, IntPtr, IntPtr),
            (Uint, IntPtr, IntPtr, Uint),
        ] {
            assert_eq!(a.common(b), arithmetic);
            assert_eq!(a.inferred_common(b), inferred);
            let scalar = Type::Scalar(a);
            let matrix = Type::Matrix(b, 2, 3);
            assert_eq!(
                Type::common(&scalar, &matrix),
                Some(matrix.with_scalar(arithmetic))
            );
            assert_eq!(
                Type::common(&matrix, &scalar),
                Some(matrix.with_scalar(b.common(a)))
            );
        }
    }

    #[test]
    fn pointer_conversion_rank_does_not_assume_lossless_unsigned_to_signed() {
        for (from, to, cost) in [
            (Int, IntPtr, 150),
            (IntPtr, Int64, 150),
            (Int64, IntPtr, 900),
            (Uint, IntPtr, 900),
            (UintPtr, Int64, 900),
            (UintPtr, IntPtr, 900),
            (IntPtr, UintPtr, 250),
            (IntPtr, Uint64, 250),
            (UintPtr, Float, 400),
        ] {
            assert_eq!(Type::Scalar(from).conversion_cost(&Type::Scalar(to)), cost);
        }
        let literal = slang_lexer::parse_integer_literal("1UZ").unwrap();
        assert_eq!(
            Type::Scalar(UintPtr).literal_conversion_cost(&Type::Scalar(Int), Some(literal)),
            81
        );
        assert_eq!(
            Type::Scalar(UintPtr).literal_conversion_cost(&Type::Scalar(Uint), Some(literal)),
            23
        );
        assert_eq!(
            Type::Scalar(UintPtr).literal_conversion_cost(&Type::Scalar(UintPtr), Some(literal)),
            0
        );
        let literal = slang_lexer::parse_integer_literal("4294967296UZ").unwrap();
        assert_eq!(
            Type::Scalar(UintPtr).literal_conversion_cost(&Type::Scalar(Uint), Some(literal)),
            900
        );
    }
}
