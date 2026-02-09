//! This module handles the PDDL/HDDL representation of lifted actions.

use std::fmt;
use crate::aiplan4rust::lir::LiftedAction;
use crate::aiplan4rust::lir::renderers::context::RenderContext;
use crate::aiplan4rust::lir::renderers::syntax::{expr, ty, typed_list};

/// Renders a [LiftedAction] into a PDDL-compliant action block.
///
/// # Parameters
/// - `f`: The formatter to write the output to.
/// - `action`: The lifted action data (preconditions, effects, etc.).
/// - `ctx`: The rendering context for ID-to-string resolution.
///
/// # Errors
/// Returns [fmt::Error] if the underlying write operations fail.
/// Note: Invalid IDs are handled gracefully by rendering fallback strings
/// like `"<unknown_id>"` instead of returning an error.
pub fn render(
    f: &mut fmt::Formatter<'_>,
    action: &LiftedAction,
    ctx: &RenderContext,
) -> std::fmt::Result  {
    // 1. Début de l'action et nom
    writeln!(f, "  (:action {}", ctx.resolve_symbol(action.name()))?;

    // 2. Paramètres : utilisation du helper du contexte pour le formatage PDDL
    write!(f, "    :parameters (")?;
    typed_list::render_typed_variable_list(f, action.parameters(), ctx)?;
    writeln!(f, "  )")?;

    // 3. Préconditions : appel de ton itérateur non-récursif
    write!(f, "    :precondition ")?;
    expr::render(f, action.precondition(), ctx)?;
    writeln!(f)?;

    // 4. Effets : appel de ton itérateur non-récursif
    write!(f, "    :effect ")?;
    expr::render(f, action.effect(), ctx)?;
    writeln!(f)?;

    // 5. Fermeture du bloc action
    writeln!(f, "  )")?;

    Ok(())
}
