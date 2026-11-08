use slang_sema::{Scalar, Type};

/// HLSL reserves words that remain valid identifiers in the Slang frontend.
/// Prefixing existing generated-name prefixes keeps this mapping injective.
pub(crate) fn identifier(name: &str) -> String {
    if name.starts_with("_slang_")
        || Type::builtin(name).is_some()
        || slang_sema::is_intrinsic_name(name)
        || reserved(name)
    {
        format!("_slang_{name}")
    } else {
        name.into()
    }
}

fn reserved(name: &str) -> bool {
    matches!(
        name,
        "asm"
            | "asm_fragment"
            | "auto"
            | "bool"
            | "break"
            | "case"
            | "catch"
            | "cbuffer"
            | "centroid"
            | "class"
            | "column_major"
            | "compile"
            | "compile_fragment"
            | "const"
            | "continue"
            | "default"
            | "delete"
            | "discard"
            | "do"
            | "double"
            | "dword"
            | "else"
            | "enum"
            | "explicit"
            | "extern"
            | "false"
            | "float"
            | "for"
            | "friend"
            | "fxgroup"
            | "groupshared"
            | "half"
            | "if"
            | "in"
            | "inline"
            | "inout"
            | "int"
            | "interface"
            | "line"
            | "lineadj"
            | "linear"
            | "matrix"
            | "namespace"
            | "new"
            | "nointerpolation"
            | "noperspective"
            | "NULL"
            | "operator"
            | "out"
            | "packoffset"
            | "pass"
            | "pixelfragment"
            | "point"
            | "precise"
            | "private"
            | "protected"
            | "public"
            | "register"
            | "return"
            | "row_major"
            | "sample"
            | "sampler"
            | "sampler1D"
            | "sampler2D"
            | "sampler3D"
            | "samplerCUBE"
            | "shared"
            | "signed"
            | "sizeof"
            | "snorm"
            | "stateblock"
            | "stateblock_state"
            | "static"
            | "string"
            | "struct"
            | "switch"
            | "tbuffer"
            | "technique"
            | "technique10"
            | "technique11"
            | "template"
            | "texture"
            | "texture1D"
            | "texture2D"
            | "texture3D"
            | "textureCUBE"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "triangle"
            | "triangleadj"
            | "typedef"
            | "typename"
            | "uint"
            | "uniform"
            | "union"
            | "unorm"
            | "unsigned"
            | "using"
            | "vector"
            | "vertexfragment"
            | "virtual"
            | "void"
            | "volatile"
            | "while"
    )
}

pub(crate) fn type_name(ty: &Type) -> String {
    match ty {
        Type::Vector(s @ (Scalar::Int64 | Scalar::Uint64), n) => format!("vector<{s}, {n}>"),
        Type::Matrix(s @ (Scalar::Int64 | Scalar::Uint64), rows, columns) => {
            format!("matrix<{s}, {rows}, {columns}>")
        }
        Type::Struct(name) => identifier(name),
        Type::ConstantBuffer { element, .. } => {
            format!("ConstantBuffer<{}>", template_element(element))
        }
        Type::Array(element, length) => format!("{}[{length}]", type_name(element)),
        Type::Buffer {
            element, writable, ..
        } => format!(
            "{}StructuredBuffer<{}>",
            if *writable { "RW" } else { "" },
            template_element(element)
        ),
        _ => ty.to_string(),
    }
}

// HLSL's parser treats adjacent template closers as a shift token.
fn template_element(ty: &Type) -> String {
    let mut text = type_name(ty);
    if text.ends_with('>') {
        text.push(' ');
    }
    text
}
