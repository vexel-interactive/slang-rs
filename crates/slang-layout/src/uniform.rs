use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixMajor {
    #[default]
    Column,
    Row,
}

/// Legacy HLSL constant-buffer layout, with unrounded occupied byte sizes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UniformTypeLayout {
    pub size: u64,
    pub alignment: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub array_stride: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matrix_stride: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matrix_major: Option<MatrixMajor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_type: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<UniformFieldLayout>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UniformFieldLayout {
    pub name: String,
    /// Byte offset relative to the containing structure.
    pub offset: u64,
    pub type_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UniformVariableLayout {
    /// Byte offset in the shared buffer, or zero for a ConstantBuffer element.
    pub offset: u64,
    /// Key of the value's layout in the uniform type table.
    pub type_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UniformLayoutError {
    InvalidShape,
    Overflow,
}

impl fmt::Display for UniformLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidShape => "unsupported uniform component size or shape",
            Self::Overflow => "uniform byte layout exceeds the 64-bit size range",
        })
    }
}

impl std::error::Error for UniformLayoutError {}

impl UniformTypeLayout {
    /// Scalars and vectors align to their 32-bit or 64-bit components.
    pub fn numeric(component_bytes: u32, width: u8) -> Result<Self, UniformLayoutError> {
        if !matches!(component_bytes, 4 | 8) || !(1..=4).contains(&width) {
            return Err(UniformLayoutError::InvalidShape);
        }
        Ok(Self::value(
            u64::from(component_bytes) * u64::from(width),
            component_bytes,
        ))
    }

    /// Column-major matrices; a single column packs as a vector in legacy DXC.
    pub fn matrix(component_bytes: u32, rows: u8, columns: u8) -> Result<Self, UniformLayoutError> {
        Self::matrix_with_major(component_bytes, rows, columns, MatrixMajor::Column)
    }

    pub fn matrix_with_major(
        component_bytes: u32,
        rows: u8,
        columns: u8,
        major: MatrixMajor,
    ) -> Result<Self, UniformLayoutError> {
        if !(1..=4).contains(&columns) || !(1..=4).contains(&rows) {
            return Err(UniformLayoutError::InvalidShape);
        }
        let (width, count) = if major == MatrixMajor::Row {
            (columns, rows)
        } else {
            (rows, columns)
        };
        let vector = Self::numeric(component_bytes, width)?;
        let stride = align(vector.size, 16)?;
        let size = stride * u64::from(count - 1) + vector.size;
        // DXC collapses single-column column-major matrices to vectors, but
        // keeps single-row row-major matrices aligned to a whole register.
        let alignment = if major == MatrixMajor::Column && count == 1 {
            component_bytes
        } else {
            16
        };
        let mut layout = Self::value(size, alignment);
        layout.matrix_stride = Some(stride);
        layout.matrix_major = Some(major);
        Ok(layout)
    }

    /// Fixed arrays preserve the last element's tail space without expansion.
    pub fn array(&self, element_type: String, count: u32) -> Result<Self, UniformLayoutError> {
        let alignment = self.alignment.max(16);
        let stride = align(self.size, alignment)?;
        let size = if count == 0 {
            0
        } else {
            stride
                .checked_mul(u64::from(count - 1))
                .and_then(|size| size.checked_add(self.size))
                .ok_or(UniformLayoutError::Overflow)?
        };
        let mut layout = Self::value(size, alignment);
        layout.array_stride = Some(stride);
        layout.element_type = Some(element_type);
        Ok(layout)
    }

    fn value(size: u64, alignment: u32) -> Self {
        Self {
            size,
            alignment,
            array_stride: None,
            matrix_stride: None,
            matrix_major: None,
            element_type: None,
            fields: vec![],
        }
    }
}

/// Packs fields or global values with legacy HLSL register-boundary rules.
#[derive(Default)]
pub struct UniformPacker {
    size: u64,
    alignment: u32,
}

impl UniformPacker {
    /// Explicit offsets bypass automatic alignment and may overlap other fields.
    pub fn place(
        &mut self,
        offset: u64,
        layout: &UniformTypeLayout,
    ) -> Result<(), UniformLayoutError> {
        if layout.size == 0 {
            return Ok(());
        }
        self.size = self.size.max(
            offset
                .checked_add(layout.size)
                .ok_or(UniformLayoutError::Overflow)?,
        );
        self.alignment = self.alignment.max(layout.alignment);
        Ok(())
    }

    pub fn push(&mut self, layout: &UniformTypeLayout) -> Result<u64, UniformLayoutError> {
        if layout.size == 0 {
            return Ok(self.size);
        }
        let mut offset = align(self.size, layout.alignment)?;
        self.alignment = self.alignment.max(layout.alignment);
        let last = offset
            .checked_add(layout.size - 1)
            .ok_or(UniformLayoutError::Overflow)?;
        if offset / 16 != last / 16 {
            offset = align(offset, 16)?;
        }
        self.size = offset
            .checked_add(layout.size)
            .ok_or(UniformLayoutError::Overflow)?;
        Ok(offset)
    }

    pub fn finish(self, fields: Vec<UniformFieldLayout>) -> UniformTypeLayout {
        let mut layout = UniformTypeLayout::value(self.size, self.alignment.max(16));
        layout.fields = fields;
        layout
    }
}

fn align(size: u64, alignment: u32) -> Result<u64, UniformLayoutError> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return Err(UniformLayoutError::InvalidShape);
    }
    let mask = u64::from(alignment - 1);
    size.checked_add(mask)
        .map(|size| size & !mask)
        .ok_or(UniformLayoutError::Overflow)
}
