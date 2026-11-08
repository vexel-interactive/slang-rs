use crate::{
    visit::{Visitor, walk_expression, walk_statement},
    *,
};
use slang_sema::Type;
use slang_source::{Diagnostic, Span};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Reconstruct values whose source storage cannot be copied directly by DXC:
/// packed structures and matrix arrays with a different default storage order.
pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    let default_major = module.matrix_major;
    let packed = packed_structures(&module.structures, default_major);
    if packed.is_empty()
        && !module
            .globals
            .iter()
            .any(|g| field_matrix_view(g, default_major))
    {
        return Ok(());
    }
    let direct: BTreeSet<_> = module
        .constant_buffers
        .iter()
        .map(|b| b.variable_name.clone())
        .collect();
    let names = crate::visit::names(module);
    let mut prefix = "_slang_uniform_view_".to_owned();
    while names.iter().any(|n| n.starts_with(&prefix)) {
        prefix.push('_');
    }
    let mut builder = Builder {
        structures: module
            .structures
            .iter()
            .map(|s| (s.name.clone(), s.clone()))
            .collect(),
        layouts: &module.uniform_types,
        packed: &packed,
        default_major,
        prefix,
        next: 0,
        remaining: 65536,
    };
    let mut resource_indices = vec![];
    let mut shared_views = vec![];
    for buffer in &mut module.constant_buffers {
        if !buffer
            .fields
            .iter()
            .any(|f| contains(&f.ty, &packed) || field_matrix_view(f, default_major))
        {
            continue;
        }
        let global = module
            .globals
            .iter()
            .find(|g| g.name == buffer.variable_name)
            .unwrap();
        let layout = global
            .uniform_layout
            .as_ref()
            .ok_or_else(|| error(global.span, "packed buffer has no data layout"))?;
        let offsets = &module.uniform_types[&layout.type_name].fields;
        let mut fields = vec![];
        for (field, offset) in buffer.fields.iter().zip(offsets) {
            if requires_view(&field.ty, &offset.type_name, &packed, default_major) {
                let initializer = builder.value(
                    &field.ty,
                    &offset.type_name,
                    offset.offset,
                    field.span,
                    &mut fields,
                )?;
                buffer.views.push(view(field, initializer));
            } else {
                let mut field = field.clone();
                field.pack_offset = Some(offset.offset);
                fields.push(field);
            }
        }
        buffer.fields = fields;
    }
    for (index, global) in module.globals.iter().enumerate() {
        if direct.contains(&global.name) {
            continue;
        }
        match &global.ty {
            Type::ConstantBuffer(element) if contains(element, &packed) => {
                let mut fields = vec![];
                let initializer =
                    builder.value(element, &element.to_string(), 0, global.span, &mut fields)?;
                let mut value = view(global, initializer);
                value.ty = *element.clone();
                module.constant_buffers.push(ConstantBuffer {
                    name: global.name.clone(),
                    variable_name: global.name.clone(),
                    fields,
                    views: vec![value],
                });
                resource_indices.push(index);
            }
            Type::Array(..) if packed_handle(&global.ty, &packed) => {
                return Err(error(
                    global.span,
                    "arrays of packed constant-buffer handles are not implemented",
                ));
            }
            _ => {}
        }
    }
    let uniforms: Vec<_> = module
        .globals
        .iter()
        .enumerate()
        .filter(|(_, g)| g.is_global_uniform())
        .map(|(i, _)| i)
        .collect();
    if uniforms.iter().any(|&i| {
        contains(&module.globals[i].ty, &packed)
            || field_matrix_view(&module.globals[i], default_major)
    }) {
        let mut fields = vec![];
        for &index in &uniforms {
            let global = &module.globals[index];
            let offset = global
                .uniform_layout
                .as_ref()
                .ok_or_else(|| error(global.span, "packed uniform has no data layout"))?
                .offset;
            if requires_view(
                &global.ty,
                &global.uniform_layout.as_ref().unwrap().type_name,
                &packed,
                default_major,
            ) {
                let initializer = builder.value(
                    &global.ty,
                    &global.uniform_layout.as_ref().unwrap().type_name,
                    offset,
                    global.span,
                    &mut fields,
                )?;
                module.globals[index] = view(global, initializer);
                shared_views.push(module.globals[index].clone());
            } else {
                let mut field = global.clone();
                field.pack_offset = Some(offset);
                fields.push(field);
            }
        }
        module.global_uniform_fields = Some(fields);
    }
    for index in resource_indices {
        module.globals[index].modifiers = vec!["static".into(), "const".into()];
    }
    let views = module
        .constant_buffers
        .iter()
        .flat_map(|b| b.views.clone())
        .chain(shared_views);
    let mut replacements = BTreeMap::new();
    let mut functions = vec![];
    let mut structures = vec![];
    for view in views {
        let (function, structure, replacement) = builder.getter(&view);
        module
            .uniform_value_functions
            .insert(view.name.clone(), function.name.clone());
        replacements.insert(view.name.clone(), replacement);
        functions.push(function);
        if let Some(structure) = structure {
            structures.push(structure);
        }
    }
    module.functions.extend(functions);
    module.structures.extend(structures);
    let mut converter = Converter {
        packed: &packed,
        replacements: &replacements,
    };
    for global in &mut module.globals {
        if !direct.contains(&global.name) {
            converter.ty(&mut global.ty);
        }
    }
    for structure in &mut module.structures {
        for field in &mut structure.fields {
            converter.ty(&mut field.ty);
        }
    }
    for function in &mut module.functions {
        converter.ty(&mut function.return_type);
        for parameter in &mut function.parameters {
            converter.ty(&mut parameter.ty);
        }
    }
    converter.module(module)
}

fn packed_structures(
    structures: &[Struct],
    default_major: slang_layout::MatrixMajor,
) -> BTreeSet<String> {
    let mut reverse: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut packed = BTreeSet::new();
    let mut queue = VecDeque::new();
    for structure in structures {
        if structure
            .fields
            .iter()
            .any(|f| f.pack_offset.is_some() || field_matrix_view(f, default_major))
        {
            packed.insert(structure.name.clone());
            queue.push_back(structure.name.clone());
        }
        for field in &structure.fields {
            let mut ty = &field.ty;
            while let Type::Array(element, _) = ty {
                ty = element;
            }
            if let Type::Struct(name) = ty {
                reverse
                    .entry(name.clone())
                    .or_default()
                    .push(structure.name.clone());
            }
        }
    }
    while let Some(name) = queue.pop_front() {
        if let Some(parents) = reverse.get(&name) {
            for parent in parents {
                if packed.insert(parent.clone()) {
                    queue.push_back(parent.clone());
                }
            }
        }
    }
    packed
}

fn contains(mut ty: &Type, packed: &BTreeSet<String>) -> bool {
    while let Type::Array(element, _) = ty {
        ty = element;
    }
    matches!(ty, Type::Struct(name) if packed.contains(name))
}

fn matrix_array(mut ty: &Type) -> bool {
    if !matches!(ty, Type::Array(..)) {
        return false;
    }
    while let Type::Array(element, _) = ty {
        ty = element;
    }
    matches!(ty, Type::Matrix(..))
}

fn field_matrix_view(field: &Variable, default_major: slang_layout::MatrixMajor) -> bool {
    let major = field
        .modifiers
        .iter()
        .rev()
        .find_map(|m| match m.as_str() {
            "row_major" => Some(slang_layout::MatrixMajor::Row),
            "column_major" => Some(slang_layout::MatrixMajor::Column),
            _ => None,
        })
        .unwrap_or(default_major);
    matrix_array(&field.ty) && major != default_major
}

fn requires_view(
    ty: &Type,
    layout_name: &str,
    packed: &BTreeSet<String>,
    default_major: slang_layout::MatrixMajor,
) -> bool {
    let major = if layout_name.starts_with("row_major ") {
        slang_layout::MatrixMajor::Row
    } else {
        slang_layout::MatrixMajor::Column
    };
    contains(ty, packed) || (matrix_array(ty) && major != default_major)
}

fn packed_handle(mut ty: &Type, packed: &BTreeSet<String>) -> bool {
    while let Type::Array(element, _) = ty {
        ty = element;
    }
    matches!(ty, Type::ConstantBuffer(element) if contains(element, packed))
}

fn view(source: &Variable, initializer: Expr) -> Variable {
    let mut value = source.clone();
    value.modifiers = vec!["static".into(), "const".into()];
    value.semantic = None;
    value.binding = None;
    value.pack_offset = None;
    value.initializer = Some(initializer);
    value
}

struct Builder<'a> {
    structures: BTreeMap<String, Struct>,
    layouts: &'a BTreeMap<String, slang_layout::UniformTypeLayout>,
    packed: &'a BTreeSet<String>,
    default_major: slang_layout::MatrixMajor,
    prefix: String,
    next: usize,
    remaining: usize,
}

impl Builder<'_> {
    fn getter(&mut self, view: &Variable) -> (Function, Option<Struct>, Expr) {
        let name = format!("{}load_{}", self.prefix, self.next);
        self.next += 1;
        let mut result_type = view.ty.clone();
        let mut initializer = view.initializer.clone().unwrap();
        let structure = if matches!(view.ty, Type::Array(..)) {
            let structure = Struct {
                name: format!("{}array_{}", self.prefix, self.next),
                fields: vec![Variable {
                    name: "_value".into(),
                    ty: view.ty.clone(),
                    modifiers: vec![],
                    semantic: None,
                    binding: None,
                    uniform_layout: None,
                    pack_offset: None,
                    initializer: None,
                    span: view.span,
                }],
            };
            result_type = Type::Struct(structure.name.clone());
            initializer = Expr {
                ty: result_type.clone(),
                kind: ExprKind::Construct(vec![initializer]),
                span: view.span,
            };
            Some(structure)
        } else {
            None
        };
        let result = Variable {
            name: "_result".into(),
            ty: result_type.clone(),
            modifiers: vec![],
            semantic: None,
            binding: None,
            uniform_layout: None,
            pack_offset: None,
            initializer: Some(initializer),
            span: view.span,
        };
        let function = Function {
            name: name.clone(),
            return_type: result_type.clone(),
            parameters: vec![],
            modifiers: vec![],
            semantic: None,
            body: Some(Stmt {
                span: view.span,
                kind: StmtKind::Block(vec![
                    Stmt {
                        span: view.span,
                        kind: StmtKind::Variable(result),
                    },
                    Stmt {
                        span: view.span,
                        kind: StmtKind::Return(Some(Expr {
                            ty: result_type.clone(),
                            kind: ExprKind::Name("_result".into()),
                            span: view.span,
                        })),
                    },
                ]),
            }),
            span: view.span,
        };
        let mut replacement = Expr {
            ty: result_type,
            kind: ExprKind::Call {
                name,
                arguments: vec![],
            },
            span: view.span,
        };
        if structure.is_some() {
            replacement = Expr {
                ty: view.ty.clone(),
                kind: ExprKind::Member {
                    base: Box::new(replacement),
                    name: "_value".into(),
                },
                span: view.span,
            };
        }
        (function, structure, replacement)
    }

    fn value(
        &mut self,
        ty: &Type,
        layout_name: &str,
        offset: u64,
        span: Span,
        fields: &mut Vec<Variable>,
    ) -> Result<Expr, Diagnostic> {
        self.remaining = self
            .remaining
            .checked_sub(1)
            .ok_or_else(|| error(span, "packed uniform expansion limit exceeded"))?;
        let layout = self
            .layouts
            .get(layout_name)
            .ok_or_else(|| error(span, "packed uniform has no value layout"))?
            .clone();
        let kind = if layout.size == 0 {
            ExprKind::Zero
        } else if requires_view(ty, layout_name, self.packed, self.default_major) {
            match ty {
                Type::Struct(name) => {
                    let structure = self.structures[name].clone();
                    let mut values = vec![];
                    for (field, location) in structure.fields.iter().zip(&layout.fields) {
                        let at = offset
                            .checked_add(location.offset)
                            .ok_or_else(|| error(span, "packed uniform offset overflow"))?;
                        values.push(self.value(
                            &field.ty,
                            &location.type_name,
                            at,
                            span,
                            fields,
                        )?);
                    }
                    ExprKind::Construct(values)
                }
                Type::Array(element, count) => {
                    if u64::from(*count) > self.remaining as u64 {
                        return Err(error(span, "packed uniform expansion limit exceeded"));
                    }
                    let stride = layout.array_stride.unwrap();
                    let mut values = vec![];
                    for i in 0..*count {
                        let at = stride
                            .checked_mul(u64::from(i))
                            .and_then(|at| offset.checked_add(at))
                            .ok_or_else(|| error(span, "packed uniform offset overflow"))?;
                        values.push(self.value(
                            element,
                            layout.element_type.as_ref().unwrap(),
                            at,
                            span,
                            fields,
                        )?);
                    }
                    ExprKind::Construct(values)
                }
                _ => unreachable!(),
            }
        } else {
            let name = format!("{}{}", self.prefix, self.next);
            self.next += 1;
            fields.push(Variable {
                name: name.clone(),
                ty: ty.clone(),
                modifiers: match layout.matrix_major {
                    Some(slang_layout::MatrixMajor::Row) => vec!["row_major".into()],
                    Some(slang_layout::MatrixMajor::Column) => vec!["column_major".into()],
                    None => vec![],
                },
                semantic: None,
                binding: None,
                uniform_layout: None,
                pack_offset: Some(offset),
                initializer: None,
                span,
            });
            ExprKind::Name(name)
        };
        Ok(Expr {
            ty: ty.clone(),
            kind,
            span,
        })
    }
}

struct Converter<'a> {
    packed: &'a BTreeSet<String>,
    replacements: &'a BTreeMap<String, Expr>,
}
impl Converter<'_> {
    fn ty(&self, ty: &mut Type) {
        match ty {
            Type::ConstantBuffer(element) if contains(element, self.packed) => {
                *ty = *element.clone();
            }
            Type::Array(element, _) => self.ty(element),
            _ => {}
        }
    }
}
impl Visitor for Converter<'_> {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        if let ExprKind::GlobalName(name) = &expression.kind
            && let Some(replacement) = self.replacements.get(name)
        {
            let span = expression.span;
            *expression = replacement.clone();
            expression.span = span;
        }
        self.ty(&mut expression.ty);
        walk_expression(self, expression)
    }
    fn statement(&mut self, statement: &mut Stmt) -> Result<(), Diagnostic> {
        match &mut statement.kind {
            StmtKind::Variable(v) => self.ty(&mut v.ty),
            StmtKind::Variables(values) => {
                for v in values {
                    self.ty(&mut v.ty);
                }
            }
            _ => {}
        }
        walk_statement(self, statement)
    }
}

fn error(span: Span, message: &str) -> Diagnostic {
    Diagnostic::error("I004", span, message)
}
