use std::fmt;
use std::fmt::Formatter;
use crate::aiplan4rust::lir::atomic_skeleton::AtomicFunctionSkeleton;
use crate::aiplan4rust::lir::renderers::RenderContext;
use crate::aiplan4rust::lir::renderers::syntax::{ty, typed_list};

/// Rendu d'un squelette de fonction atomique (ex: (distance ?c1 ?c2 - city) - number).
pub fn render(
    f: &mut Formatter<'_>,
    function: &AtomicFunctionSkeleton,
    ctx: &RenderContext
) -> fmt::Result {
    // 1. Début de la signature et nom de la fonction (functor)
    write!(f, "({}", ctx.resolve_symbol(function.functor()))?;

    // 2. Rendu des paramètres s'il y en a (ex: ?p1 - type1)
    if !function.parameters().is_empty() {
        write!(f, " ")?;
        typed_list::render_typed_variable_list(f, function.parameters(), ctx)?;
    }

    // Fermeture de la parenthèse de signature
    write!(f, ")")?;

    // 3. Rendu du type de retour (ex: " - number")
    if !function.ty().is_empty() {
        write!(f, " - ")?;
        ty::render(f, function.ty(), ctx)?;
    }

    Ok(())
}
