use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFunctionSkeleton;
use crate::aiplan4rust::compiler::lir::renderers::syntax::{ty, typed_list};
use crate::aiplan4rust::compiler::lir::renderers::RenderContext;
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

    // 2. Paramètres : Récupération via le store de l'arène
    if let Some(params_list) = ctx.store().get_typed_list(function.parameters()) {
        if !params_list.is_empty() {
            write!(f, " ")?;
            typed_list::render_typed_variable_list(f, params_list.as_slice(), ctx)?;
        }
    } else {
        write!(f, " <error: parameters not found>")?;
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
