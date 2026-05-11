//! This module handles the syntax rendering of PDDL derived predicates.

use crate::aiplan4rust::lir::store::problem::DerivedPredicateDef;
use crate::aiplan4rust::lir::store::renderers::syntax::{atom, expr};
use crate::aiplan4rust::lir::store::renderers::RenderContext;
use std::fmt;

/// Renders a [DerivedPredicateDef] into its PDDL representation (Syntax version).
///
/// Format: (:derived (head) (body))
pub fn render(
    f: &mut fmt::Formatter<'_>,
    derived: &DerivedPredicateDef,
    ctx: &RenderContext,
) -> fmt::Result {
    // 1. Début du bloc et "head"
    // Le head est un AtomicFormulaSkeleton (ex: (path ?x ?y))
    write!(f, "(:derived ")?;
    atom::render(f, derived.head(), ctx)?;

    // 2. Le "body" (la formule logique)
    // On indente le corps pour une meilleure lisibilité dans le fichier de domaine
    write!(f, "\n    ")?;
    expr::render(f, derived.body(), ctx)?;

    // 3. Fermeture du bloc (:derived ...)
    write!(f, "\n  )")
}
