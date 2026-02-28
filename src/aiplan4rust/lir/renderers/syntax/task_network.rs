//! This module handles the rendering of HDDL Task Networks.

use std::fmt;
use std::fmt::Formatter;
use crate::aiplan4rust::lir::{InitialTaskNetwork, TaskNetwork};
use crate::aiplan4rust::lir::renderers::context::RenderContext;
use crate::aiplan4rust::lir::renderers::syntax::{expr, typed_list};

/// Renders a [TaskNetwork] into HDDL format.
///
/// # Parameters
/// - `f`: The formatter.
/// - `network`: The task network containing subtasks and constraints.
/// - `ctx`: The rendering context.
pub fn render_task_network(
    f: &mut fmt::Formatter<'_>,
    network: &TaskNetwork,
    ctx: &RenderContext,
) -> fmt::Result {
    // 1. Subtasks
    if network.is_declared_total_ordered() {
        write!(f, "  :ordered-subtasks ")?;
    } else {
        write!(f, "  :subtasks ")?;
    }
    expr::render(f, &network.tasks(), ctx)?;

    // 2. Ordering
    if !network.ordering_constraints().is_empty() {
        // On ajoute un saut de ligne AVANT le nouveau mot-clé
        write!(f, "\n  :ordering ")?;
        expr::render(f, &network.ordering_constraints(), ctx)?;
    }

    // 3. Constraints
    if !network.logical_constraints().is_empty() {
        // On ajoute un saut de ligne AVANT le nouveau mot-clé
        write!(f, "\n  :constraints ")?;
        expr::render(f, &network.logical_constraints(), ctx)?;
    }

    Ok(())
}


/// Rendu du réseau de tâches initial (HDDL)
pub fn render_initial_task_network(
    f: &mut Formatter<'_>,
    itn: &InitialTaskNetwork,
    ctx: &RenderContext
) -> fmt::Result {
    write!(f, "(:htn")?;

    // 1. Rendu des paramètres s'il y en a
    if !itn.parameters().is_empty() {
        write!(f, "\n    :parameters (")?;
        typed_list::render_typed_variable_list(f, itn.parameters().as_slice(), ctx)?;
        write!(f, ")")?;
    }

    // 2. Rendu du contenu du réseau (tasks, orderings, constraints)
    // On suppose que LiftedTaskNetwork a sa propre fonction render dans ce module ou un autre
    // Ici, on indente le contenu du réseau de tâches
    write!(f, "\n    ")?;
    render_task_network(f, itn.task_network(), ctx)?;

    write!(f, "\n  )")
}
