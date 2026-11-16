//! Legalize typed raw-buffer accesses using Slang's natural byte layout.

use crate::{
    visit::{Visitor, walk_expression},
    *,
};
use slang_layout::MatrixMajor;
use slang_sema::{Scalar, Type};
use slang_source::{Diagnostic, Span};
use std::collections::BTreeMap;

pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    let names = crate::visit::names(module);
    let mut prefix = "_slang_byte_".to_owned();
    while names.iter().any(|name| name.starts_with(&prefix)) {
        prefix.push('_');
    }
    let mut lowerer = Lowerer {
        prefix,
        native_16bit: module.native_16bit_types,
        structures: module.structures.iter().map(|s| (s.name.clone(), s.fields.clone())).collect(),
        layouts: BTreeMap::new(),
        cache: BTreeMap::new(),
        functions: vec![],
        next_local: 0,
        remaining_accesses: 65536,
    };
    lowerer.module(module)?;
    module.functions.extend(lowerer.functions);
    Ok(())
}

struct Lowerer {
    prefix: String,
    native_16bit: bool,
    structures: BTreeMap<String, Vec<Variable>>,
    layouts: BTreeMap<String, Layout>,
    cache: BTreeMap<String, String>,
    functions: Vec<Function>,
    next_local: usize,
    remaining_accesses: usize,
}

#[derive(Clone, Copy)]
struct Layout {
    size: u64,
    alignment: u64,
}

fn rounded(value: u64, alignment: u64) -> u64 {
    value.div_ceil(alignment) * alignment
}

impl Lowerer {
    fn layout(&mut self, ty: &Type, span: Span, depth: usize) -> Result<Layout, Diagnostic> {
        if let Type::Struct(name) = ty
            && let Some(layout) = self.layouts.get(name)
        {
            return Ok(*layout);
        }
        if depth >= 128 {
            return Err(Diagnostic::error(
                "I002",
                span,
                "typed byte-buffer layout nesting limit exceeded",
            ));
        }
        let scalar_layout = |scalar: Scalar, count: u64| {
            let alignment = if scalar == Scalar::Bool {
                4
            } else {
                u64::from(scalar.bit_width() / 8)
            };
            Layout {
                size: alignment * count,
                alignment,
            }
        };
        let layout = match ty {
            Type::Void => Layout {
                size: 0,
                alignment: 1,
            },
            Type::Scalar(scalar) => scalar_layout(*scalar, 1),
            Type::Vector(scalar, count) => scalar_layout(*scalar, u64::from(*count)),
            Type::Matrix(scalar, rows, columns) => {
                scalar_layout(*scalar, u64::from(*rows) * u64::from(*columns))
            }
            Type::Array(element, count) => {
                let element = self.layout(element, span, depth + 1)?;
                Layout {
                    size: rounded(element.size, element.alignment)
                        .checked_mul(u64::from(*count))
                        .ok_or_else(|| {
                        Diagnostic::error("I002", span, "typed byte-buffer layout is too large")
                    })?,
                    alignment: element.alignment,
                }
            }
            Type::Struct(name) => {
                let fields = self.structures.get(name).cloned().ok_or_else(|| {
                    Diagnostic::error("I002", span, "missing typed byte-buffer structure")
                })?;
                let mut result = Layout {
                    size: 0,
                    alignment: 1,
                };
                for field in fields {
                    let field = self.layout(&field.ty, field.span, depth + 1)?;
                    result.size = rounded(result.size, field.alignment)
                        .checked_add(field.size)
                        .ok_or_else(|| {
                            Diagnostic::error("I002", span, "typed byte-buffer layout is too large")
                        })?;
                    result.alignment = result.alignment.max(field.alignment);
                }
                // Struct tails remain available to the following enclosing field.
                result
            }
            _ => {
                return Err(Diagnostic::error(
                    "I002",
                    span,
                    "unsupported typed byte-buffer data type",
                ));
            }
        };
        if layout.size > u64::from(u32::MAX) {
            return Err(Diagnostic::error(
                "I002",
                span,
                "typed byte-buffer layout exceeds the address range",
            ));
        }
        if let Type::Struct(name) = ty {
            self.layouts.insert(name.clone(), layout);
        }
        Ok(layout)
    }

    fn helper(
        &mut self,
        load: bool,
        major: MatrixMajor,
        arguments: &[Expr],
        ty: &Type,
        span: Span,
    ) -> Result<String, Diagnostic> {
        let key = format!("{load} {major:?} {} {ty} {}", arguments[0].ty, arguments.len());
        if let Some(name) = self.cache.get(&key) {
            return Ok(name.clone());
        }
        self.layout(ty, span, 0)?;
        let name = format!("{}{}", self.prefix, self.cache.len());
        self.cache.insert(key, name.clone());
        let mut parameters = vec![
            variable("_buffer", arguments[0].ty.clone(), None, span),
            variable("_address", uint(), None, span),
        ];
        if !load {
            parameters.push(variable("_value", ty.clone(), None, span));
        }
        if arguments.len()
            > if load {
                2
            } else {
                3
            }
        {
            parameters.push(variable("_alignment", uint(), None, span));
        }
        let receiver = named("_buffer", arguments[0].ty.clone(), span);
        let address = named("_address", uint(), span);
        let value = named("_value", ty.clone(), span);
        let mut body = vec![];
        if load && *ty != Type::Void {
            body.push(stmt(StmtKind::Variable(variable("_value", ty.clone(), None, span)), span));
        }
        self.access(load, major, &receiver, address, value.clone(), &mut body, 0)?;
        body.push(stmt(StmtKind::Return((load && *ty != Type::Void).then_some(value)), span));
        self.functions.push(Function {
            name: name.clone(),
            return_type: if load {
                ty.clone()
            } else {
                Type::Void
            },
            parameters,
            modifiers: vec![],
            semantic: None,
            body: Some(stmt(StmtKind::Block(body), span)),
            span,
        });
        Ok(name)
    }

    #[allow(clippy::too_many_arguments)]
    fn access(
        &mut self,
        load: bool,
        major: MatrixMajor,
        receiver: &Expr,
        address: Expr,
        value: Expr,
        body: &mut Vec<Stmt>,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        let span = value.span;
        if self.remaining_accesses == 0 {
            return Err(Diagnostic::error(
                "I002",
                span,
                "typed byte-buffer expansion limit exceeded",
            ));
        }
        self.remaining_accesses -= 1;
        match &value.ty {
            Type::Void => {}
            Type::Scalar(scalar) => self.scalar(load, *scalar, receiver, address, value, body),
            Type::Vector(scalar, count) => {
                let stride = self.layout(&Type::Scalar(*scalar), span, depth)?.size;
                for index in 0..*count {
                    self.access(
                        load,
                        major,
                        receiver,
                        offset(address.clone(), u64::from(index) * stride),
                        indexed(
                            value.clone(),
                            integer(u64::from(index), span),
                            Type::Scalar(*scalar),
                        ),
                        body,
                        depth + 1,
                    )?;
                }
            }
            Type::Matrix(scalar, rows, columns) => {
                let stride = self.layout(&Type::Scalar(*scalar), span, depth)?.size;
                for row in 0..*rows {
                    for column in 0..*columns {
                        let index = match major {
                            MatrixMajor::Row => {
                                u64::from(row) * u64::from(*columns) + u64::from(column)
                            }
                            MatrixMajor::Column => {
                                u64::from(column) * u64::from(*rows) + u64::from(row)
                            }
                        };
                        let row_value = indexed(
                            value.clone(),
                            integer(u64::from(row), span),
                            Type::Vector(*scalar, *columns),
                        );
                        self.access(
                            load,
                            major,
                            receiver,
                            offset(address.clone(), index * stride),
                            indexed(
                                row_value,
                                integer(u64::from(column), span),
                                Type::Scalar(*scalar),
                            ),
                            body,
                            depth + 1,
                        )?;
                    }
                }
            }
            Type::Array(element, count) => {
                if *count != 0 {
                    let layout = self.layout(element, span, depth + 1)?;
                    let stride = rounded(layout.size, layout.alignment);
                    let name = format!("_index_{}", self.next_local);
                    self.next_local += 1;
                    let index = named(&name, uint(), span);
                    let mut elements = vec![];
                    self.access(
                        load,
                        major,
                        receiver,
                        binary(
                            "+",
                            address,
                            binary("*", index.clone(), integer(stride, span), uint()),
                            uint(),
                        ),
                        indexed(value.clone(), index.clone(), *element.clone()),
                        &mut elements,
                        depth + 1,
                    )?;
                    body.push(stmt(
                        StmtKind::For {
                            initializer: Box::new(stmt(
                                StmtKind::Variable(variable(
                                    &name,
                                    uint(),
                                    Some(integer(0, span)),
                                    span,
                                )),
                                span,
                            )),
                            condition: Some(binary(
                                "<",
                                index.clone(),
                                integer(u64::from(*count), span),
                                Type::Scalar(Scalar::Bool),
                            )),
                            increment: Some(Expr {
                                ty: uint(),
                                span,
                                kind: ExprKind::Postfix {
                                    op: "++".into(),
                                    value: Box::new(index),
                                },
                            }),
                            body: Box::new(stmt(StmtKind::Block(elements), span)),
                        },
                        span,
                    ));
                }
            }
            Type::Struct(name) => {
                let fields = self.structures[name].clone();
                let mut position = 0;
                for field in fields {
                    let layout = self.layout(&field.ty, field.span, depth + 1)?;
                    position = rounded(position, layout.alignment);
                    let field_major = field
                        .modifiers
                        .iter()
                        .rev()
                        .find_map(|modifier| match modifier.as_str() {
                            "row_major" => Some(MatrixMajor::Row),
                            "column_major" => Some(MatrixMajor::Column),
                            _ => None,
                        })
                        .unwrap_or(major);
                    let member = Expr {
                        ty: field.ty,
                        span,
                        kind: ExprKind::Member {
                            base: Box::new(value.clone()),
                            name: field.name,
                        },
                    };
                    self.access(
                        load,
                        field_major,
                        receiver,
                        offset(address.clone(), position),
                        member,
                        body,
                        depth + 1,
                    )?;
                    position += layout.size;
                }
            }
            _ => {
                return Err(Diagnostic::error(
                    "I002",
                    span,
                    "unsupported typed byte-buffer data type",
                ));
            }
        }
        Ok(())
    }

    fn scalar(
        &mut self,
        load: bool,
        scalar: Scalar,
        receiver: &Expr,
        address: Expr,
        value: Expr,
        body: &mut Vec<Stmt>,
    ) {
        let span = value.span;
        if scalar == Scalar::Half && !self.native_16bit {
            if load {
                let word = method(
                    receiver.clone(),
                    "Load",
                    vec![binary("&", address.clone(), integer(0xffff_fffc, span), uint())],
                    uint(),
                );
                let shift = binary(
                    "<<",
                    binary("&", address, integer(2, span), uint()),
                    integer(3, span),
                    uint(),
                );
                let bits =
                    binary("&", binary(">>", word, shift, uint()), integer(0xffff, span), uint());
                let loaded = intrinsic("f16tof32", vec![bits], Type::Scalar(Scalar::Float), span);
                body.push(expression(assign(value, cast(loaded, Type::Scalar(Scalar::Half)))));
            } else {
                self.half_store(receiver, address, value, body);
            }
            return;
        }
        if load {
            let loaded = if scalar == Scalar::Bool {
                binary(
                    "!=",
                    method(receiver.clone(), "Load", vec![address], uint()),
                    integer(0, span),
                    Type::Scalar(Scalar::Bool),
                )
            } else {
                method(receiver.clone(), "@slang.byte.scalar.load", vec![address], value.ty.clone())
            };
            body.push(expression(assign(value, loaded)));
        } else {
            let stored = if scalar == Scalar::Bool {
                cast(value, uint())
            } else {
                value
            };
            let name = if scalar == Scalar::Bool {
                "Store"
            } else {
                "@slang.byte.scalar.store"
            };
            body.push(expression(method(
                receiver.clone(),
                name,
                vec![address, stored],
                Type::Void,
            )));
        }
    }

    fn half_store(&mut self, receiver: &Expr, address: Expr, value: Expr, body: &mut Vec<Stmt>) {
        let span = value.span;
        let suffix = self.next_local;
        self.next_local += 1;
        let local = |name: &str| format!("_{name}_{suffix}");
        let word_address = named(&local("word_address"), uint(), span);
        let shift = named(&local("shift"), uint(), span);
        let mask = named(&local("mask"), uint(), span);
        let bits = named(&local("bits"), uint(), span);
        let expected = named(&local("expected"), uint(), span);
        let observed = named(&local("observed"), uint(), span);
        for (name, initializer) in [
            (
                local("word_address"),
                binary("&", address.clone(), integer(0xffff_fffc, span), uint()),
            ),
            (
                local("shift"),
                binary(
                    "<<",
                    binary("&", address, integer(2, span), uint()),
                    integer(3, span),
                    uint(),
                ),
            ),
            (local("mask"), binary("<<", integer(0xffff, span), shift.clone(), uint())),
            (
                local("bits"),
                binary(
                    "<<",
                    intrinsic(
                        "f32tof16",
                        vec![cast(value, Type::Scalar(Scalar::Float))],
                        uint(),
                        span,
                    ),
                    shift,
                    uint(),
                ),
            ),
            (
                local("observed"),
                method(receiver.clone(), "Load", vec![word_address.clone()], uint()),
            ),
        ] {
            body.push(stmt(
                StmtKind::Variable(variable(&name, uint(), Some(initializer), span)),
                span,
            ));
        }
        body.push(stmt(StmtKind::Variable(variable(&local("expected"), uint(), None, span)), span));
        let desired = binary(
            "|",
            binary(
                "&",
                expected.clone(),
                Expr {
                    ty: uint(),
                    span,
                    kind: ExprKind::Unary {
                        op: "~".into(),
                        value: Box::new(mask),
                    },
                },
                uint(),
            ),
            bits,
            uint(),
        );
        // Retry on a changed containing word so adjacent half stores cannot lose
        // each other's bits. The successful exchange is this store's commit point.
        let retry = vec![
            expression(assign(expected.clone(), observed.clone())),
            expression(method(
                receiver.clone(),
                "InterlockedCompareExchange",
                vec![word_address, expected.clone(), desired, observed.clone()],
                Type::Void,
            )),
        ];
        body.push(stmt(
            StmtKind::DoWhile {
                condition: binary("!=", observed, expected, Type::Scalar(Scalar::Bool)),
                body: Box::new(stmt(StmtKind::Block(retry), span)),
            },
            span,
        ));
    }
}

impl Visitor for Lowerer {
    fn expression(&mut self, value: &mut Expr) -> Result<(), Diagnostic> {
        walk_expression(self, value)?;
        if let ExprKind::Call {
            name,
            arguments,
        } = &mut value.kind
            && name.starts_with("@slang.byte.")
        {
            let load = name.starts_with("@slang.byte.load.");
            let major = if name.ends_with(".row") {
                MatrixMajor::Row
            } else {
                MatrixMajor::Column
            };
            let ty = if load {
                value.ty.clone()
            } else {
                arguments[2].ty.clone()
            };
            *name = self.helper(load, major, arguments, &ty, value.span)?;
        }
        Ok(())
    }
}

fn uint() -> Type {
    Type::Scalar(Scalar::Uint)
}
fn named(name: &str, ty: Type, span: Span) -> Expr {
    Expr {
        ty,
        span,
        kind: ExprKind::Name(name.into()),
    }
}
fn integer(value: u64, span: Span) -> Expr {
    Expr {
        ty: uint(),
        span,
        kind: ExprKind::Integer(format!("{value}u")),
    }
}
fn variable(name: &str, ty: Type, initializer: Option<Expr>, span: Span) -> Variable {
    Variable {
        name: name.into(),
        ty,
        modifiers: vec![],
        semantic: None,
        binding: None,
        uniform_layout: None,
        pack_offset: None,
        initializer,
        span,
    }
}
fn stmt(kind: StmtKind, span: Span) -> Stmt {
    Stmt {
        kind,
        span,
    }
}
fn expression(value: Expr) -> Stmt {
    stmt(StmtKind::Expression(value.clone()), value.span)
}
fn binary(op: &str, left: Expr, right: Expr, ty: Type) -> Expr {
    Expr {
        ty,
        span: left.span,
        kind: ExprKind::Binary {
            op: op.into(),
            left: Box::new(left),
            right: Box::new(right),
        },
    }
}
fn assign(left: Expr, right: Expr) -> Expr {
    let ty = left.ty.clone();
    binary("=", left, right, ty)
}
fn cast(value: Expr, ty: Type) -> Expr {
    Expr {
        ty,
        span: value.span,
        kind: ExprKind::Cast {
            value: Box::new(value),
        },
    }
}
fn offset(address: Expr, offset: u64) -> Expr {
    if offset == 0 {
        address
    } else {
        let span = address.span;
        binary("+", address, integer(offset, span), uint())
    }
}
fn indexed(base: Expr, index: Expr, ty: Type) -> Expr {
    Expr {
        ty,
        span: base.span,
        kind: ExprKind::Index {
            base: Box::new(base),
            index: Box::new(index),
        },
    }
}
fn method(receiver: Expr, name: &str, arguments: Vec<Expr>, ty: Type) -> Expr {
    Expr {
        ty,
        span: receiver.span,
        kind: ExprKind::MethodCall {
            receiver: Box::new(receiver),
            name: name.into(),
            arguments,
        },
    }
}
fn intrinsic(name: &str, arguments: Vec<Expr>, ty: Type, span: Span) -> Expr {
    Expr {
        ty,
        span,
        kind: ExprKind::IntrinsicCall {
            name: name.into(),
            arguments,
        },
    }
}
