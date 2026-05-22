use crate::aiplan4rust::grounding::passes::positive_form_normalization::expr;
use crate::aiplan4rust::lang::AtomSkeletonId;
use crate::aiplan4rust::lir::old::expr::ops::ExprOpError;
use crate::aiplan4rust::lir::DerivedPredicateDef;
use crate::aiplan4rust::tree::NodeId;

/// Applies the Positive Normal Form (PNF) transformation to a derived predicate.
///
/// This processes the axiom's body. The head of the predicate remains
/// unchanged as it represents the symbolic conclusion of the rule.
///
/// It uses a reusable `scratchpad` buffer to collect negated atoms without
/// triggering new memory allocations.
/// Applies the Positive Normal Form (PNF) transformation to a Derived Predicate (Axiom).
///
/// Derived predicates define a formula (the body) that implies the head.
/// We apply PNF to the entire body to eliminate structural negation.
pub fn to_pnf(
    predicate: &mut DerivedPredicateDef,
    negated_atoms: &mut Vec<AtomSkeletonId>,
    scratchpad: &mut Vec<AtomSkeletonId>,
    stack: &mut Vec<(NodeId, bool)>, // La pile DFS réutilisable injectée
) -> Result<(), ExprOpError> {
    // 0. On vide le buffer de travail pour cette unité (conserve la capacité)
    scratchpad.clear();

    // Derived predicates define a formula (the body) that implies the head.
    // We apply PNF to the entire body to eliminate structural negation.
    if let Some(root_id) = predicate.body_mut().root_id() {
        expr::to_pnf(root_id, predicate.body_mut(), scratchpad, stack, false)?;
    }

    // 1. Tri et dédoublonnement local (essentiel si le corps est complexe)
    // Cela évite de propager des doublons vers le vecteur global.
    scratchpad.sort_unstable();
    scratchpad.dedup();

    // 2. Merge into the global problem vector
    // extend_from_slice est privilégié pour sa rapidité de copie mémoire.
    negated_atoms.extend_from_slice(scratchpad);

    Ok(())
}
