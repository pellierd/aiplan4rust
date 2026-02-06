use std::fmt::{self, Formatter};
use crate::aiplan4rust::lir::atomic_skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::lir::renderers::context::RenderContext;
use crate::aiplan4rust::lir::renderers::syntax::typed_list;

/// Rendu d'une signature nommée et typée (utilisée par les prédicats, fonctions, tâches).
/// Cette fonction est le point d'entrée unique.
pub fn render(
    f: &mut Formatter<'_>,
    formula: &AtomicFormulaSkeleton,
    ctx: &RenderContext
) -> fmt::Result {
    // 1. Début de la parenthèse et nom du symbole
    write!(f, "({}", ctx.resolve_symbol(formula.symbol()))?;

    // 2. Rendu des paramètres s'il y en a
    let params = formula.parameters();
    if !params.is_empty() {
        write!(f, " ")?;
        // On utilise la fonction de ton module typed_list qui gère le préfixe '?'
        typed_list::render_typed_variable_list(f, params, ctx)?;
    }

    // 3. Fermeture
    write!(f, ")")
}
