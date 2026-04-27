use crate::aiplan4rust::lir::store::problem::atomic_skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::lir::store::renderers::default::typed_list;
use crate::aiplan4rust::lir::store::renderers::RenderContext;
use std::fmt::{self, Formatter};

/// Rendu d'une signature nommée et typée (utilisée par les prédicats, fonctions, tâches).
/// Format : (nom ?x0 - type1 ?x1 - type2)
pub fn render(
    f: &mut Formatter<'_>,
    formula: &AtomicFormulaSkeleton,
    ctx: &RenderContext,
) -> fmt::Result {
    // 1. Nom du prédicat avec son ID [as#ID]
    let symbol_id = formula.symbol();
    write!(f, "({} [{}]", ctx.resolve_predicate(symbol_id), symbol_id)?;

    // 2. Paramètres : on délègue tout à la fonction spécialisée
    if !formula.parameters().is_empty() {
        write!(f, " ")?;
        // Utilise la TypedList directement
        typed_list::render_variable_typed_list(f, formula.parameters().as_slice(), ctx)?;
    }

    // 3. Fermeture
    write!(f, ")")
}
