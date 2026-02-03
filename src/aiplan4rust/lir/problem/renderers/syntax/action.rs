use std::fmt;
use crate::aiplan4rust::lang::{TypeID, TypedSymbol};
use crate::aiplan4rust::lir::problem::LiftedAction;
use crate::aiplan4rust::lir::problem::renderers::render_context::RenderContext;
use crate::aiplan4rust::lir::problem::renderers::syntax::{expr, typed_list};

pub fn render(
    f: &mut fmt::Formatter<'_>,
    action: &LiftedAction,
    ctx: &RenderContext,
) -> std::fmt::Result  {
    // 1. Début de l'action et nom
    writeln!(f, "(:action {}", action.name())?;

    // 2. Paramètres : utilisation du helper du contexte pour le formatage PDDL
    write!(f, "  :parameters (")?;
    typed_list::render(f, action.parameters(), ctx)?;
    writeln!(f, ")")?;

    // 3. Préconditions : appel de ton itérateur non-récursif
    write!(f, "  :precondition ")?;
    expr::render(f, action.precondition(), ctx)?;
    writeln!(f)?;

    // 4. Effets : appel de ton itérateur non-récursif
    write!(f, "  :effect ")?;
    expr::render(f, action.effect(), ctx)?;
    writeln!(f)?;

    // 5. Fermeture du bloc action
    writeln!(f, ")")?;

    Ok(())
}
