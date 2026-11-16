//! Emit global initializers after the globals they depend on.

use crate::{
    visit::{Visitor, walk_expression},
    *,
};
use slang_source::Diagnostic;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    let globals: BTreeMap<_, _> = module
        .globals
        .iter()
        .enumerate()
        .map(|(index, global)| (global.name.as_str(), index))
        .collect();
    let mut functions = BTreeMap::new();
    for function in &module.functions {
        if let Some(body) = &function.body {
            let references =
                functions.entry(function.name.as_str()).or_insert_with(References::default);
            references.statement(&mut body.clone())?;
        }
    }

    let mut dependencies = vec![BTreeSet::new(); module.globals.len()];
    for (index, global) in module.globals.iter().enumerate() {
        let Some(initializer) = &global.initializer else {
            continue;
        };
        let mut references = References::default();
        references.expression(&mut initializer.clone())?;
        let mut pending: Vec<_> = references.calls.into_iter().collect();
        let mut visited = BTreeSet::new();
        while let Some(name) = pending.pop() {
            if visited.insert(name.clone())
                && let Some(function) = functions.get(name.as_str())
            {
                references.globals.extend(function.globals.iter().cloned());
                pending.extend(function.calls.iter().cloned());
            }
        }
        dependencies[index].extend(
            references.globals.iter().filter_map(|name| globals.get(name.as_str()).copied()),
        );
    }

    // Visit source declarations in order, moving only a declaration's required
    // dependencies ahead of it. Use an explicit stack for long global chains.
    let mut states = vec![0; module.globals.len()];
    let mut order = Vec::with_capacity(module.globals.len());
    for root in 0..module.globals.len() {
        let mut pending = vec![(root, false)];
        while let Some((index, finish)) = pending.pop() {
            if finish {
                states[index] = 2;
                order.push(index);
                continue;
            }
            match states[index] {
                2 => continue,
                1 => {
                    return Err(Diagnostic::error(
                        "I002",
                        module.globals[index].span,
                        "cyclic global initializer dependencies",
                    ));
                }
                _ => {}
            }
            states[index] = 1;
            pending.push((index, true));
            pending.extend(dependencies[index].iter().rev().map(|&dependency| (dependency, false)));
        }
    }
    let mut original: Vec<_> = std::mem::take(&mut module.globals).into_iter().map(Some).collect();
    module.globals = order.into_iter().map(|index| original[index].take().unwrap()).collect();
    Ok(())
}

#[derive(Default)]
struct References {
    globals: BTreeSet<String>,
    calls: BTreeSet<String>,
}

impl Visitor for References {
    fn expression(&mut self, value: &mut Expr) -> Result<(), Diagnostic> {
        match &value.kind {
            ExprKind::GlobalName(name) => {
                self.globals.insert(name.clone());
            }
            ExprKind::Call {
                name,
                ..
            } => {
                self.calls.insert(name.clone());
            }
            _ => {}
        }
        walk_expression(self, value)
    }
}
