use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::compiler::lir::renderers::debug::typed_list;
use crate::aiplan4rust::compiler::lir::renderers::LirRenderContext;
use std::fmt::{self, Formatter};

/// Rendu d'une signature nommée et typée (utilisée par les prédicats, fonctions, tâches).
/// Format : (nom ?x0 - type1 ?x1 - type2)
pub fn render(
    f: &mut Formatter<'_>,
    formula: &AtomicFormulaSkeleton,
    ctx: &LirRenderContext,
) -> fmt::Result {
    // 1. Nom du prédicat avec son ID [as#ID]
    let symbol_id = formula.symbol();
    write!(f, "({} [{}]", ctx.resolve_predicate(symbol_id), symbol_id)?;

    // 2. Paramètres : Récupération sécurisée via le store
    if let Some(params_list) = ctx.store().get_typed_list(formula.parameters()) {
        if !params_list.is_empty() {
            write!(f, " ")?;
            // On passe la slice des paramètres réels à la fonction de rendu
            typed_list::render_variable_typed_list(f, params_list.as_slice(), ctx)?;
        }
    } else {
        // Optionnel : un indicateur visuel discret si la liste est introuvable
        write!(f, " <error: params not found>")?;
    }

    // 3. Fermeture
    write!(f, ")")
}
