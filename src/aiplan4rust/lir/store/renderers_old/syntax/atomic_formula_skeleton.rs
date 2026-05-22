use crate::aiplan4rust::lir::store::problem_old::atomic_skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::lir::store::renderers_old::context::RenderContext;
use crate::aiplan4rust::lir::store::renderers_old::syntax::typed_list;
use std::fmt::{self, Formatter};

/// Rendu d'une signature nommée et typée (utilisée par les prédicats, fonctions, tâches).
/// Cette fonction est le point d'entrée unique.
pub fn render(
    f: &mut Formatter<'_>,
    formula: &AtomicFormulaSkeleton,
    ctx: &RenderContext,
) -> fmt::Result {
    // 1. Début de la parenthèse et nom du symbole
    write!(f, "({}", ctx.resolve_predicate(formula.symbol()))?;

    // 2. Rendu des paramètres s'il y en a
    let parameters = formula.parameters();
    if !parameters.is_empty() {
        write!(f, " ")?;
        // On utilise la fonction de ton module typed_list qui gère le préfixe '?'
        typed_list::render_typed_variable_list(f, parameters.as_slice(), ctx)?;
    }

    // 3. Fermeture
    write!(f, ")")
}
