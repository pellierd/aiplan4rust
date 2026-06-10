//! This module handles the syntax rendering of HDDL Task Networks.

use crate::aiplan4rust::compiler::lir::problem::TaskNetwork;
use crate::aiplan4rust::compiler::lir::renderers::context::RenderContext;
use crate::aiplan4rust::compiler::lir::renderers::syntax::expr;
use std::fmt;

/// Renders a [TaskNetwork] into HDDL format (Syntax version).
pub fn render(
    f: &mut fmt::Formatter<'_>,
    network: &TaskNetwork,
    ctx: &RenderContext,
) -> fmt::Result {
    // 1. Subtasks
    // HDDL utilise :ordered-subtasks si l'ordre est total, sinon :subtasks
    if network.is_declared_total_ordered() {
        write!(f, ":ordered-subtasks ")?;
    } else {
        write!(f, ":subtasks ")?;
    }

    // Rendu récursif via le module expr (va gérer les (and ...), les labels et les tasks)
    expr::render(f, network.tasks(), ctx)?;

    // 2. Ordering Constraints
    // Note: On vérifie si l'ID d'expression n'est pas vide (EMPTY_AND)
    if !network.ordering_constraints().is_some() {
        write!(f, "\n    :ordering ")?;
        expr::render(f, network.ordering_constraints(), ctx)?;
    }

    // 3. Logical Constraints
    if !network.logical_constraints().is_some() {
        write!(f, "\n    :constraints ")?;
        expr::render(f, network.logical_constraints(), ctx)?;
    }

    Ok(())
}
