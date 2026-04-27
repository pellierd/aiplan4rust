//! This module handles the syntax rendering of PDDL/HDDL lifted actions.

use crate::aiplan4rust::lir::store::problem::ActionDef;
use crate::aiplan4rust::lir::store::renderers::context::RenderContext;
use crate::aiplan4rust::lir::store::renderers::syntax::{expr, typed_list};
use std::fmt;

/// Renders a [ActionDef] into a PDDL-compliant action block (Syntax version).
pub fn render(
    f: &mut fmt::Formatter<'_>,
    action: &ActionDef,
    ctx: &RenderContext,
) -> std::fmt::Result {
    let is_durative = action.is_durative();

    // 1. En-tête
    let header_keyword = if is_durative {
        ":durative-action"
    } else {
        ":action"
    };
    // On résout le nom de l'action via le contexte
    writeln!(
        f,
        "  ({} {}",
        header_keyword,
        ctx.resolve_action_symbol(action.name())
    )?;

    // 2. Paramètres : (?x - type ?y - type)
    write!(f, "    :parameters (")?;
    typed_list::render_typed_variable_list(f, action.parameters().as_slice(), ctx)?;
    writeln!(f, ")")?;

    // 3. Durée (Spécifique aux actions duratives)
    if let Some(duration_expr) = action.duration() {
        write!(f, "    :duration ")?;
        expr::render(f, duration_expr, ctx)?;
        writeln!(f)?;
    }

    // 4. Préconditions / Conditions
    // On vérifie si l'expression n'est pas un (and) vide pour éviter les blocs inutiles
    if !action.precondition().is_some() {
        let cond_label = if is_durative {
            ":condition"
        } else {
            ":precondition"
        };
        write!(f, "    {} ", cond_label)?;
        expr::render(f, action.precondition(), ctx)?;
        writeln!(f)?;
    }

    // 5. Effets
    if !action.effect().is_some() {
        write!(f, "    :effect ")?;
        expr::render(f, action.effect(), ctx)?;
        writeln!(f)?;
    }

    // 6. Fermeture
    write!(f, "  )")
}
