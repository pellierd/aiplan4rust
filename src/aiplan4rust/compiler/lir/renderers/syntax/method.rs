//! This module handles the HDDL representation of methods for hierarchical planning.

use crate::aiplan4rust::compiler::lir::problem::MethodDef;
use crate::aiplan4rust::compiler::lir::renderers::context::RenderContext;
use crate::aiplan4rust::compiler::lir::renderers::syntax::{expr, task_network, typed_list};
use std::fmt;

/// Renders a [MethodDef] into an HDDL-compliant method block (Syntax version).
pub fn render(f: &mut fmt::Formatter<'_>, method: &MethodDef, ctx: &RenderContext) -> fmt::Result {
    // 1. En-tête et nom de la méthode
    writeln!(f, "(:method {}", ctx.resolve_method_symbol(method.name()))?;

    // 2. Paramètres : (?x - type)
    write!(f, "  :parameters (")?;
    typed_list::render_typed_variable_list(f, method.parameters().as_slice(), ctx)?;
    writeln!(f, ")")?;

    // 3. La tâche abstraite décomposée par cette méthode
    write!(f, "  :task ")?;
    expr::render(f, method.task(), ctx)?;
    writeln!(f)?;

    // 4. Précondition (Souvent un bloc 'and')
    if !method.precondition().is_some() {
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
