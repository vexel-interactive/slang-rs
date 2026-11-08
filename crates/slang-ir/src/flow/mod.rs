//! Scalar SSA and sparse conditional constant propagation over structured IR.
//!
//! Block arguments merge local values at branches and loop backedges. Resource
//! accesses, aggregates, floating values, and calls remain opaque. The graph is
//! an analysis representation: source writes and their evaluation order stay in
//! structured IR when proven scalar values are substituted.

mod build;
mod constant;
mod model;
mod sccp;
mod verify;

pub use model::*;
pub use sccp::{Lattice, Propagation, propagate};

use crate::{visit::Visitor, *};
use slang_source::Diagnostic;
use std::collections::BTreeMap;

/// Build a scalar SSA graph for a function, including module call signatures.
/// Returns `None` for a prototype, missing function, or an analysis exceeding
/// the optional optimizer's size budget. Compilation can proceed without it.
pub fn build_function(module: &Module, index: usize) -> Option<Graph> {
    build::build(module, module.functions.get(index)?).map(|analysis| analysis.graph)
}

pub(crate) fn lower(module: &mut Module) -> Result<(), Diagnostic> {
    for index in 0..module.functions.len() {
        let Some(analysis) = build::build(module, &module.functions[index]) else {
            continue;
        };
        analysis
            .graph
            .verify()
            .map_err(|message| Diagnostic::error("I004", module.functions[index].span, message))?;
        let constants = propagate(&analysis.graph);
        Rewriter {
            sites: analysis.sites,
            constants,
        }
        .statement(module.functions[index].body.as_mut().unwrap())?;
    }
    Ok(())
}

/// Expressions do not yet carry IR IDs. These addresses are temporary keys:
/// analysis borrows the tree without moving it, and rewriting visits children
/// before replacing a kind. No address escapes this pass or is dereferenced.
fn expression_key(expression: &Expr) -> usize {
    std::ptr::from_ref(expression) as usize
}

#[derive(Clone, Copy)]
enum Rewrite {
    Value,
    Compound { preserve_right: bool },
}

struct Site {
    value: ValueId,
    rewrite: Rewrite,
}

struct Analysis {
    graph: Graph,
    sites: BTreeMap<usize, Site>,
}

struct Rewriter {
    sites: BTreeMap<usize, Site>,
    constants: Propagation,
}

impl Visitor for Rewriter {
    fn expression(&mut self, expression: &mut Expr) -> Result<(), Diagnostic> {
        let site = self.sites.remove(&expression_key(expression));
        crate::visit::walk_expression(self, expression)?;
        let Some(site) = site else { return Ok(()) };
        let Lattice::Constant(bits) = self.constants.values[site.value.0] else {
            return Ok(());
        };
        let kind = if expression.ty == slang_sema::Type::Scalar(slang_sema::Scalar::Bool) {
            ExprKind::Bool(bits != 0)
        } else {
            ExprKind::Integer(crate::integer::literal_text(bits, &expression.ty))
        };
        match site.rewrite {
            Rewrite::Value => expression.kind = kind,
            Rewrite::Compound { preserve_right } => {
                // Only a direct scalar local is eligible. Retain the write and
                // assignment result; indexed and member destinations stay intact.
                let ExprKind::Binary { op, right, .. } = &mut expression.kind else {
                    unreachable!()
                };
                *op = "=".into();
                let constant = Expr {
                    ty: expression.ty.clone(),
                    span: expression.span,
                    kind,
                };
                **right = if preserve_right {
                    Expr {
                        ty: constant.ty.clone(),
                        span: expression.span,
                        kind: ExprKind::Binary {
                            op: ",".into(),
                            left: right.clone(),
                            right: Box::new(constant),
                        },
                    }
                } else {
                    constant
                };
            }
        }
        Ok(())
    }
}
