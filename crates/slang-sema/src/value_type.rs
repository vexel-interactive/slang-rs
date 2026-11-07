use crate::{Type, checker::Checker};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
enum State {
    Active,
    Complete(Option<u16>),
}

impl Checker {
    /// Compute each struct's value depth once, without using the thread stack.
    pub(crate) fn validate_value_types(&mut self) {
        let mut states = BTreeMap::new();
        for root in self.structures.keys() {
            let mut stack = vec![(root.as_str(), false)];
            while let Some((name, exiting)) = stack.pop() {
                if !exiting {
                    if states.contains_key(name) {
                        continue;
                    }
                    states.insert(name, State::Active);
                    stack.push((name, true));
                    if let Some(fields) = self.structures.get(name) {
                        for ty in fields.values() {
                            if let Some(child) = value_struct(ty) {
                                stack.push((child, false));
                            }
                        }
                    }
                } else {
                    let mut depth = Some(1);
                    if let Some(fields) = self.structures.get(name) {
                        for ty in fields.values() {
                            if let Some(child) = value_struct(ty) {
                                // An active child is a back edge. An invalid
                                // child also invalidates every embedding parent.
                                let child_depth = match states.get(child) {
                                    Some(State::Complete(Some(depth))) => Some(*depth + 1),
                                    _ => None,
                                };
                                depth = depth
                                    .zip(child_depth)
                                    .map(|(a, b)| a.max(b))
                                    .filter(|d| *d <= 128);
                            }
                        }
                    }
                    states.insert(name, State::Complete(depth));
                }
            }
        }
        let invalid: Vec<_> = states
            .into_iter()
            .filter(|(_, state)| matches!(state, State::Complete(None)))
            .map(|(name, _)| (name.to_owned(), self.struct_definitions[name].span))
            .collect();
        for (name, span) in invalid {
            self.error(
                "T002",
                span,
                format!(
                    "recursive or excessively nested value type '{}'",
                    self.source_name(&name)
                ),
            );
        }
    }
}

fn value_struct(mut ty: &Type) -> Option<&str> {
    while let Type::Array(element, _) = ty {
        ty = element;
    }
    if let Type::Struct(name) = ty {
        Some(name)
    } else {
        None
    }
}
