//! This module handles the HDDL representation of methods for hierarchical planning.

use crate::aiplan4rust::compiler::lir::problem::MethodDef;
use crate::aiplan4rust::compiler::lir::renderers::context::LirRenderContext;
use crate::aiplan4rust::compiler::lir::renderers::syntax::{expr, task_network, typed_list};
use std::fmt;

/// Renders a [MethodDef] into an HDDL-compliant method block (Syntax version).
pub fn render(
    f: &mut fmt::Formatter<'_>,
    method: &MethodDef,
    ctx: &LirRenderContext,
) -> fmt::Result {
    // 1. En-tête et nom de la méthode
    writeln!(f, "(:method {}", ctx.resolve_method_symbol(method.name()))?;

    // 2. Paramètres : Récupération sécurisée via le store
    write!(f, "  :parameters (")?;
    if let Some(params_list) = ctx.store().get_typed_list(method.parameters()) {
        typed_list::render_typed_variable_list(f, params_list.as_slice(), ctx)?;
    } else {
        write!(f, "<error: parameters not found>")?;
    }
    writeln!(f, ")")?;

    // 3. La tâche abstraite décomposée par cette méthode
    write!(f, "  :task ")?;
    expr::render(f, method.task(), ctx)?;
    writeln!(f)?;

    // 4. Précondition (Souvent un bloc 'and')
    if method.precondition().is_some() {
        write!(f, "  :precondition ")?;
        expr::render(f, method.precondition(), ctx)?;
        writeln!(f)?;
    }

    // 5. Réseau de tâches (Subtasks, Ordering, Constraints)
    // On délègue au module task_network qui gère l'indentation interne
    task_network::render(f, method.task_network(), ctx)?;
    writeln!(f)?;

    // 6. Fermeture du bloc (:method ...)
    write!(f, ")")
}
