use crate::aiplan4rust::lir::store::problem::DerivedPredicateDef;
use crate::aiplan4rust::lir::store::renderers::default::atomic_skeleton;
use crate::aiplan4rust::lir::store::renderers::default::common::writeln_centered;
use crate::aiplan4rust::lir::store::renderers::default::expr::render;
use crate::aiplan4rust::lir::store::renderers::RenderContext;
use std::fmt;

pub fn render_derived_predicate(
    f: &mut fmt::Formatter<'_>,
    dp: &DerivedPredicateDef,
    ctx: &RenderContext,
) -> fmt::Result {
    writeln_centered(f, " [ DERIVED PREDICATE ] ", 60, '=')?;

    // 1. HEAD : Utilisation de la signature complète (Nom, Variables, Types)
    write!(f, "  HEAD         : ")?;
    // On appelle la fonction de signature que l'on vient de porter
    atomic_skeleton::render(f, dp.head(), ctx)?;

    // On garde l'ID technique à côté pour le debug
    writeln!(f, " ({})", dp.header_id())?;

    // 2. BODY : Rendu en arbre de l'expression
    writeln!(f, "  BODY         :")?;
    render(f, dp.body(), ctx)?;

    writeln!(f, "{}", "=".repeat(60))
}
