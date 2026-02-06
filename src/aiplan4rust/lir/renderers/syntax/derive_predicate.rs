//! This module handles the rendering of PDDL derived predicates.

use std::fmt;
use crate::aiplan4rust::lir::problem::LiftedDerivedPredicate;
use crate::aiplan4rust::lir::renderers::RenderContext;
use crate::aiplan4rust::lir::renderers::syntax::{atomic_formula_skeleton, expr};

/// Renders a [DerivedPredicate] into its PDDL representation.
///
/// Format: (:derived (head) (body))
pub fn render(
    f: &mut fmt::Formatter<'_>,
    derived: &LiftedDerivedPredicate,
    ctx: &RenderContext,
) -> fmt::Result {
    // 1. Début du bloc et "head" (le nom du prédicat et ses arguments)
    // On utilise expr::render pour le head car c'est une AtomicFormulaSkeleton
    write!(f, "(:derived ")?;
    atomic_formula_skeleton::render(f, &derived.head(), ctx)?;

    // 2. Le "body" (la condition logique qui définit le prédicat)
    // On ajoute un espace ou un saut de ligne selon ta préférence.
    // Pour rester compact comme tes exemples précédents :
    write!(f, "\n  ")?;
    expr::render(f, &derived.body(), ctx)?;

    // 3. Fermeture du bloc
    write!(f, ")")?;

    Ok(())
}
