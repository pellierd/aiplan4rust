use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::compiler::lir::renderers::context::LirRenderContext;
use crate::aiplan4rust::compiler::lir::renderers::syntax::typed_list;
use std::fmt::{self, Formatter};

/// Rendu syntaxique d'une signature nommée et typée.
/// Utilisé pour déclarer les prédicats, les fonctions ou les tâches.
///
/// Format produit : (nom_du_symbole ?x0 - type0 ?x1 - type1)
pub fn render(
    f: &mut Formatter<'_>,
    formula: &AtomicFormulaSkeleton,
    ctx: &LirRenderContext,
) -> fmt::Result {
    // 1. Début de la parenthèse et résolution du nom via l'interner
    // On utilise resolve_predicate pour le nom du symbole de la formule
    write!(f, "({}", ctx.resolve_predicate(formula.symbol()))?;

    // 2. Paramètres : Récupération via le store de l'arène
    if let Some(params_list) = ctx.store().get_typed_list(formula.parameters()) {
        if !params_list.is_empty() {
            write!(f, " ")?;
            // Délégation au renderer de liste de variables (qui gère le ? et le " - type")
            typed_list::render_typed_variable_list(f, params_list.as_slice(), ctx)?;
        }
    } else {
        write!(f, " <error: parameters not found>")?;
    }

    // 3. Fermeture de la signature
    write!(f, ")")
}
