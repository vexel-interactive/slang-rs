use crate::{
    Program, SymbolOrigin,
    graph::{ModuleId, Node},
    resolve::Resolver,
};
use slang_source::{Diagnostic, Span};
use slang_syntax::{Declaration, Module};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default)]
pub(crate) struct Names {
    pub types: BTreeMap<String, BTreeSet<String>>,
    pub values: BTreeMap<String, BTreeSet<String>>,
    pub generic_functions: BTreeSet<String>,
}

#[derive(Default)]
pub(crate) struct Members {
    pub all: BTreeMap<String, BTreeSet<String>>,
    pub static_names: BTreeMap<String, BTreeSet<String>>,
    pub fields: BTreeMap<(String, String), String>,
    pub owners: BTreeMap<String, String>,
    pub types: BTreeMap<(String, String), String>,
    pub type_owners: BTreeMap<String, String>,
    pub generic_parameters: BTreeMap<String, Vec<String>>,
}

impl Names {
    pub fn add(&mut self, source: String, target: String, is_type: bool) {
        let map = if is_type {
            &mut self.types
        } else {
            &mut self.values
        };
        map.entry(source).or_default().insert(target);
    }

    pub fn extend(&mut self, names: &Self) {
        self.generic_functions
            .extend(names.generic_functions.iter().cloned());
        for (source, targets) in &names.types {
            self.types
                .entry(source.clone())
                .or_default()
                .extend(targets.iter().cloned());
        }
        for (source, targets) in &names.values {
            self.values
                .entry(source.clone())
                .or_default()
                .extend(targets.iter().cloned());
        }
    }

    pub fn lookup(
        &self,
        name: &str,
        is_type: bool,
        span: Span,
    ) -> Result<Option<String>, Diagnostic> {
        let map = if is_type { &self.types } else { &self.values };
        let Some(targets) = map.get(name) else {
            return Ok(None);
        };
        if targets.len() > 1 {
            return Err(Diagnostic::error(
                "M004",
                span,
                format!("ambiguous imported name '{name}'"),
            ));
        }
        Ok(targets.first().cloned())
    }
}

pub(crate) fn link(
    mut nodes: Vec<Node>,
    order: &[ModuleId],
    identifiers: &BTreeSet<String>,
) -> Result<Program, Vec<Diagnostic>> {
    let mut prefix = "_slang_module_".to_owned();
    while identifiers.iter().any(|name| name.starts_with(&prefix)) {
        prefix.push('_');
    }
    let mut own = vec![Names::default(); nodes.len()];
    let mut exports = vec![Names::default(); nodes.len()];
    let mut origins = BTreeMap::new();
    for node in &nodes {
        let nested_names: BTreeSet<_> = node
            .syntax
            .nested_types
            .iter()
            .map(|m| &m.type_name)
            .collect();
        for declaration in &node.syntax.declarations {
            let (name, modifiers, is_type) = match declaration {
                Declaration::Struct(s) => (&s.name, &s.modifiers, true),
                Declaration::Function(f) => (&f.name, &f.modifiers, false),
                Declaration::Global(v) => (&v.name, &v.modifiers, false),
                Declaration::TypeAlias(a) => (&a.name, &a.modifiers, true),
            };
            let target = if node.info.id.0 == 0 {
                name.clone()
            } else {
                format!("{prefix}{}_{}", node.info.id.0, name)
            };
            own[node.info.id.0].add(name.clone(), target.clone(), is_type);
            if matches!(declaration, Declaration::Function(f) if !f.generic_parameters.is_empty()) {
                own[node.info.id.0].generic_functions.insert(name.clone());
            }
            if modifiers.iter().any(|m| m == "public") && !nested_names.contains(name) {
                exports[node.info.id.0].add(name.clone(), target.clone(), is_type);
                if matches!(declaration, Declaration::Function(f) if !f.generic_parameters.is_empty())
                {
                    exports[node.info.id.0]
                        .generic_functions
                        .insert(name.clone());
                }
            }
            origins.insert(
                target,
                SymbolOrigin {
                    module: node.info.id,
                    name: name.clone(),
                },
            );
        }
    }
    let mut syntax = Module {
        name: nodes[0].syntax.name.clone(),
        ..Module::default()
    };
    let mut next_expression = 0;
    for id in order {
        let mut imported = Names::default();
        for (dependency, reexport) in &nodes[id.0].dependencies {
            let names = exports[dependency.0].clone();
            imported.extend(&names);
            if *reexport {
                exports[id.0].extend(&names);
            }
        }
        let mut members = Members {
            all: nodes[id.0]
                .syntax
                .declarations
                .iter()
                .filter_map(|d| {
                    if let Declaration::Struct(s) = d {
                        Some((
                            s.name.clone(),
                            s.fields.iter().map(|f| f.name.clone()).collect(),
                        ))
                    } else {
                        None
                    }
                })
                .collect(),
            ..Members::default()
        };
        for declaration in &nodes[id.0].syntax.declarations {
            let generic = match declaration {
                Declaration::Struct(s) => Some((&s.name, &s.generic_parameters)),
                Declaration::TypeAlias(a) => Some((&a.name, &a.generic_parameters)),
                _ => None,
            };
            if let Some((name, parameters)) = generic {
                members.generic_parameters.insert(
                    name.clone(),
                    parameters.iter().map(|p| p.name.clone()).collect(),
                );
            }
            if let Declaration::Function(function) = declaration
                && let Some(method) = &function.method
            {
                members
                    .all
                    .entry(method.owner.clone())
                    .or_default()
                    .insert(method.name.clone());
                if method.is_static {
                    members
                        .static_names
                        .entry(method.owner.clone())
                        .or_default()
                        .insert(method.name.clone());
                }
            }
        }
        for member in &nodes[id.0].syntax.static_members {
            members
                .all
                .entry(member.owner.clone())
                .or_default()
                .insert(member.name.clone());
            members
                .static_names
                .entry(member.owner.clone())
                .or_default()
                .insert(member.name.clone());
            members.fields.insert(
                (member.owner.clone(), member.name.clone()),
                member.variable_name.clone(),
            );
            members
                .owners
                .insert(member.variable_name.clone(), member.owner.clone());
        }
        for member in &nodes[id.0].syntax.nested_types {
            members
                .all
                .entry(member.owner.clone())
                .or_default()
                .insert(member.name.clone());
            members
                .static_names
                .entry(member.owner.clone())
                .or_default()
                .insert(member.name.clone());
            members.types.insert(
                (member.owner.clone(), member.name.clone()),
                member.type_name.clone(),
            );
            members
                .type_owners
                .insert(member.type_name.clone(), member.owner.clone());
        }
        let mut resolver = Resolver::new(
            &own[id.0],
            &imported,
            &mut next_expression,
            format!("{prefix}local_{}_", id.0),
            *id,
            &mut origins,
            &members,
        );
        for declaration in &mut nodes[id.0].syntax.declarations {
            if id.0 != 0
                && matches!(declaration, Declaration::Function(f) if f.attributes.iter().any(|a| a.name == "shader" || a.name == "numthreads"))
            {
                let Declaration::Function(f) = declaration else {
                    unreachable!()
                };
                return Err(vec![Diagnostic::error(
                    "M003",
                    f.span,
                    "imported shader entry points are not implemented",
                )]);
            }
            resolver.declaration(declaration).map_err(|d| vec![d])?;
        }
        for buffer in &mut nodes[id.0].syntax.constant_buffers {
            resolver.constant_buffer(buffer).map_err(|d| vec![d])?;
        }
        for member in &mut nodes[id.0].syntax.nested_types {
            resolver.nested_type(member).map_err(|d| vec![d])?;
        }
        for member in &mut nodes[id.0].syntax.static_members {
            resolver.static_member(member).map_err(|d| vec![d])?;
        }
        syntax
            .declarations
            .append(&mut nodes[id.0].syntax.declarations);
        syntax
            .constant_buffers
            .append(&mut nodes[id.0].syntax.constant_buffers);
        syntax
            .static_members
            .append(&mut nodes[id.0].syntax.static_members);
        syntax
            .nested_types
            .append(&mut nodes[id.0].syntax.nested_types);
    }
    Ok(Program {
        syntax,
        modules: nodes.into_iter().map(|node| node.info).collect(),
        symbols: origins,
        diagnostics: vec![],
    })
}
