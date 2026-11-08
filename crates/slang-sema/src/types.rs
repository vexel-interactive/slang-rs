use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Scalar {
    Bool,
    Int,
    Uint,
    Half,
    Float,
    Double,
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
    pub fn builtin(name: &str) -> Option<Self> {
        if matches!(name, "ByteAddressBuffer" | "RWByteAddressBuffer") {
            return Some(Self::ByteBuffer {
                writable: name.starts_with("RW"),
            });
        }
        if let Some(scalar) = match name {
            "int32_t" => Some(Scalar::Int),
            "uint32_t" => Some(Scalar::Uint),
            "float32_t" => Some(Scalar::Float),
            "float64_t" => Some(Scalar::Double),
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
        matches!(self.scalar(), Some(Scalar::Int | Scalar::Uint))
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
        } else {
            1
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
                Self::Scalar(_) => Some(a.with_scalar(sa.max(sb))),
                Self::Matrix(_, r, c) if (r, c) == (rows, columns) => {
                    Some(a.with_scalar(sa.max(sb)))
                }
                _ => None,
            };
        }
        if matches!(b, Self::Matrix(..)) {
            return Self::common(b, a);
        }
        let (wa, wb) = (a.width()?, b.width()?);
        if wa != wb && wa != 1 && wb != 1 {
            return None;
        }
        let scalar = sa.max(sb);
        let width = wa.max(wb);
        Some(
            if width == 1 && matches!((a, b), (Self::Scalar(_), Self::Scalar(_))) {
                Self::Scalar(scalar)
            } else {
                Self::Vector(scalar, width)
            },
        )
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
            Self::Vector(s, n) => write!(f, "{s}{n}"),
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
