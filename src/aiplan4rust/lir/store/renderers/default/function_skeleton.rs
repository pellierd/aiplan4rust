use crate::aiplan4rust::lir::store::problem::atomic_skeleton::AtomicFunctionSkeleton;
use crate::aiplan4rust::lir::store::renderers::default::{ty, typed_list};
use crate::aiplan4rust::lir::store::renderers::RenderContext;
use core::fmt::Formatter;
use std::fmt;

/// Rendu d'un squelette de fonction atomique en mode hybride.
/// Exemple : (distance [f#1] ?x0 [v#0] - city (1) ?x1 [v#1] - city (1)) - number (2)
pub fn render(
    f: &mut Formatter<'_>,
    function: &AtomicFunctionSkeleton,
    ctx: &RenderContext,
) -> fmt::Result {
    let functor_id = function.functor();
    let name = ctx.resolve_functor(functor_id);

    // 1. Nom de la fonction avec son ID [f#ID]
    write!(f, "({} [{}]", name, functor_id)?;

    // 2. Paramètres d'entrée (utilise render_typed_variable_list pour le mode hybride)
    if !function.parameters().is_empty() {
        write!(f, " ")?;
        typed_list::render_variable_typed_list(f, function.parameters().as_slice(), ctx)?;
    }

    // Fermeture de la parenthèse du foncteur
    write!(f, ")")?;

    // 3. Type de retour (utilise render_type_expression pour l'ID et le either)
    if !function.ty().is_empty() {
        ty::render(f, function.ty(), ctx)?;
    }

    Ok(())
}
