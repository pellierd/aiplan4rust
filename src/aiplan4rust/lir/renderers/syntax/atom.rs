use crate::aiplan4rust::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::lir::renderers::context::RenderContext;
use crate::aiplan4rust::lir::renderers::syntax::typed_list;
use std::fmt::{self, Formatter};

/// Rendu syntaxique d'une signature nommée et typée.
/// Utilisé pour déclarer les prédicats, les fonctions ou les tâches.
///
/// Format produit : (nom_du_symbole ?x0 - type0 ?x1 - type1)
pub fn render(
    f: &mut Formatter<'_>,
    formula: &AtomicFormulaSkeleton,
    ctx: &RenderContext,
) -> fmt::Result {
    // 1. Début de la parenthèse et résolution du nom via l'interner
    // On utilise resolve_predicate pour le nom du symbole de la formule
    write!(f, "({}", ctx.resolve_predicate(formula.symbol()))?;

    // 2. Rendu des paramètres via la slice
    let parameters = formula.parameters();
    if !parameters.is_empty() {
        write!(f, " ")?;
        // Délégation au renderer de liste de variables (qui gère le ? et le " - type")
        typed_list::render_typed_variable_list(f, parameters.as_slice(), ctx)?;
    }

    // 3. Fermeture de la signature
    write!(f, ")")
}
