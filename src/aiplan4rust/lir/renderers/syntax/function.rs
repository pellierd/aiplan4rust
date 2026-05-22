use crate::aiplan4rust::lir::problem::skeleton::AtomicFunctionSkeleton;
use crate::aiplan4rust::lir::renderers::syntax::{ty, typed_list};
use crate::aiplan4rust::lir::renderers::RenderContext;
use std::fmt;
use std::fmt::Formatter;

/// Rendu d'un squelette de fonction atomique (Syntaxe PDDL).
/// Format : (nom_fonction ?arg1 - type1) - type_retour
pub fn render(
    f: &mut Formatter<'_>,
    function: &AtomicFunctionSkeleton,
    ctx: &RenderContext,
) -> fmt::Result {
    // 1. Début de la signature et nom de la fonction (functor)
    // On utilise resolve_functor pour obtenir le nom correct (ex: "distance")
    write!(f, "({}", ctx.resolve_functor(function.functor()))?;

    // 2. Rendu des paramètres via la slice
    let parameters = function.parameters();
    if !parameters.is_empty() {
        write!(f, " ")?;
        typed_list::render_typed_variable_list(f, parameters.as_slice(), ctx)?;
    }

    // Fermeture de la parenthèse de signature : (name ?args)
    write!(f, ")")?;

    // 3. Rendu du type de retour (ex: " - number" ou " - city")
    // ty::render gère déjà le format " - (either ...)" si nécessaire
    if !function.ty().is_empty() {
        ty::render(f, function.ty(), ctx)?;
    }

    Ok(())
}
