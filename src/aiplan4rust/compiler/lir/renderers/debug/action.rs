use crate::aiplan4rust::compiler::lir::problem::ActionDef;
use crate::aiplan4rust::compiler::lir::renderers::debug::common::{
    render_labeled_variable_typed_list, writeln_centered,
};
use crate::aiplan4rust::compiler::lir::renderers::debug::expr;
use crate::aiplan4rust::compiler::lir::renderers::LirRenderContext;
use std::fmt;

pub fn render(
    f: &mut fmt::Formatter<'_>,
    action: &ActionDef,
    ctx: &LirRenderContext,
) -> fmt::Result {
    // 1. Détermination du titre et du style
    // On suppose que ton Action a une méthode is_durative() ou qu'on le déduit du Store
    let is_durative = action.is_durative();
    let symbol_id = action.name();
    let action_name = ctx.resolve_action_symbol(symbol_id);

    let title = if is_durative {
        format!(" [ DURATIVE ACTION: {} ({}) ] ", action_name, symbol_id)
    } else {
        format!(" [ ACTION: {} ({}) ] ", action_name, symbol_id)
    };

    let border_char = if is_durative { '=' } else { '-' };
    writeln_centered(f, &title, 80, border_char)?;

    // 2. Paramètres (Utilise notre version hybride labellisée)
    if let Some(params_list) = ctx.store().get_typed_list(action.parameters()) {
        render_labeled_variable_typed_list(f, "PARAMETERS", params_list, ctx)?;
    } else {
        writeln!(f, "  PARAMETERS   : <error: list not found>")?;
    }

    // 3. Durée (Si applicable)
    if is_durative {
        if let Some(duration_id) = action.duration() {
            writeln!(f, "  DURATION     :")?;
            expr::render(f, duration_id, ctx)?;
        }
    }

    // 4. Condition / Précondition
    let cond_label = if is_durative {
        "CONDITION"
    } else {
        "PRECONDITION"
    };
    writeln!(f, "  {:<12} :", cond_label)?;
    expr::render(f, action.precondition(), ctx)?;

    // 5. Effets
    writeln!(f, "  EFFECTS      :")?;
    expr::render(f, action.effect(), ctx)?;

    writeln!(f, "{}", border_char.to_string().repeat(80))
}
