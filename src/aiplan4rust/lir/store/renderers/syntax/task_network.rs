//! This module handles the syntax rendering of HDDL Task Networks.

use crate::aiplan4rust::lir::store::problem::{InitialTaskNetwork, TaskNetwork};
use crate::aiplan4rust::lir::store::renderers::context::RenderContext;
use crate::aiplan4rust::lir::store::renderers::syntax::{expr, typed_list};
use std::fmt;
use std::fmt::Formatter;

/// Renders a [TaskNetwork] into HDDL format (Syntax version).
pub fn render_task_network(
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

/// Renders the Initial Task Network (HTN) for a problem file.
pub fn render_initial_task_network(
    f: &mut Formatter<'_>,
    itn: &InitialTaskNetwork,
    ctx: &RenderContext,
) -> fmt::Result {
    write!(f, "(:htn")?;

    // 1. Parameters (Optional in HTN)
    if !itn.parameters().is_empty() {
        write!(f, "\n    :parameters (")?;
        typed_list::render_typed_variable_list(f, itn.parameters().as_slice(), ctx)?;
        write!(f, ")")?;
    }

    // 2. The core network
    write!(f, "\n    ")?;
    render_task_network(f, itn.task_network(), ctx)?;

    write!(f, "\n  )")
}
