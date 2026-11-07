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

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Type {
    Void,
    Scalar(Scalar),
    Vector(Scalar, u8),
    Array(Box<Type>, u32),
    Struct(String),
    Buffer {
        element: Box<Type>,
        writable: bool,
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
                && let Ok(size @ 2..=4) = suffix.parse::<u8>()
            {
                return Some(Self::Vector(scalar, size));
            }
        }
        None
    }

    pub fn scalar(&self) -> Option<Scalar> {
        match self {
            Self::Scalar(s) | Self::Vector(s, _) => Some(*s),
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
    pub fn integral(&self) -> bool {
        matches!(self.scalar(), Some(Scalar::Int | Scalar::Uint))
    }

    pub fn convertible_to(&self, target: &Self) -> bool {
        if self == target || *self == Self::Error || *target == Self::Error {
            return true;
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
        let (sa, sb, wa, wb) = (a.scalar()?, b.scalar()?, a.width()?, b.width()?);
        if sa == Scalar::Bool || sb == Scalar::Bool {
            return (a == b).then(|| a.clone());
        }
        if wa != wb && wa != 1 && wb != 1 {
            return None;
        }
        let scalar = sa.max(sb);
        let width = wa.max(wb);
        Some(if width == 1 {
            Self::Scalar(scalar)
        } else {
            Self::Vector(scalar, width)
        })
    }

    pub fn boolean_shape(&self) -> Self {
        match self.width() {
            Some(1) => Self::Scalar(Scalar::Bool),
            Some(n) => Self::Vector(Scalar::Bool, n),
            None => Self::Error,
        }
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
            Self::Error => f.write_str("<error>"),
            Self::Scalar(s) => write!(f, "{s}"),
            Self::Vector(s, n) => write!(f, "{s}{n}"),
            Self::Array(element, n) => write!(f, "{element}[{n}]"),
            Self::Struct(name) => f.write_str(name),
            Self::Buffer { element, writable } => write!(
                f,
                "{}StructuredBuffer<{element}>",
                if *writable { "RW" } else { "" }
            ),
            Self::Callable { .. } => f.write_str("<function>"),
            Self::Meta(ty) => write!(f, "<type {ty}>"),
        }
    }
}
