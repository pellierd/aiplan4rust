//! This module handles the syntax rendering of PDDL/HDDL lifted actions.

use crate::aiplan4rust::compiler::lir::problem::ActionDef;
use crate::aiplan4rust::compiler::lir::renderers::context::LirRenderContext;
use crate::aiplan4rust::compiler::lir::renderers::syntax::{expr, typed_list};
use std::fmt;

/// Renders a [ActionDef] into a PDDL-compliant action block (Syntax version).
pub fn render(
    f: &mut fmt::Formatter<'_>,
    action: &ActionDef,
    ctx: &LirRenderContext,
) -> std::fmt::Result {
    let is_durative = action.is_durative();

    // 1. En-tête
    let header_keyword = if is_durative {
        ":durative-action"
    } else {
        ":action"
    };
    writeln!(
        f,
        "  ({} {}",
        header_keyword,
        ctx.resolve_action_symbol(action.name())
    )?;

    // 2. Paramètres : Récupération via le store de l'arène
    write!(f, "    :parameters (")?;
    if let Some(params_list) = ctx.store().get_typed_list(action.parameters()) {
        typed_list::render_typed_variable_list(f, params_list.as_slice(), ctx)?;
    } else {
        write!(f, "<error: parameters not found>")?;
    }
    writeln!(f, ")")?;

    // 3. Durée (Spécifique aux actions duratives)
    if let Some(duration_expr) = action.duration() {
        writeln!(f, "    :duration")?;
        expr::render_with_indent(f, duration_expr, ctx, 1)?;
        writeln!(f)?;
    }

    // 4. Préconditions / Conditions
    if action.precondition().is_some() {
        let cond_label = if is_durative {
            ":condition"
        } else {
            ":precondition"
        };
        writeln!(f, "    {}", cond_label)?;
        expr::render_with_indent(f, action.precondition(), ctx, 1)?;
        writeln!(f)?;
    }

    // 5. Effets
    if action.effect().is_some() {
        writeln!(f, "    :effect")?;
        expr::render_with_indent(f, action.effect(), ctx, 1)?;
        writeln!(f)?;
    }

    // 6. Fermeture
    write!(f, "  )")
}
