use crate::aiplan4rust::grounding::passes::positive_normal_form::expr;
use crate::aiplan4rust::lang::AtomSkeletonId;
use crate::aiplan4rust::lir::ActionDef;
use crate::aiplan4rust::lir::expr::ops::ExprOpError;
use crate::aiplan4rust::tree::NodeId;

/// Applies the Positive Normal Form (PNF) transformation to an action.
///
/// This function traverses both preconditions and effects to absorb 'Not'
/// operators and tag the atom IDs with the MSB (Most Significant Bit).
///
/// It uses a reusable `scratchpad` buffer to perform local deduplication
/// without new allocations.
pub fn encode_to_pnf(
    action: &mut ActionDef,
    negated_atoms: &mut Vec<AtomSkeletonId>,
    scratchpad: &mut Vec<AtomSkeletonId>,
    stack: &mut Vec<NodeId>,
) -> Result<(), ExprOpError> {
    // 0. On vide le buffer de travail local (garde la capacité)
    scratchpad.clear();

    // 1. Collecte dans les préconditions
    if let Some(root_id) = action.precondition_mut().root_id() {
        expr::encode_to_pnf(
            root_id,
            action.precondition_mut(),
            scratchpad,
            stack, // On passe la pile au moteur de traversée
        )?;
    }

    // 2. Collecte dans les effets (conditions des 'When')
    // Note: On ne vide pas le scratchpad ici car on veut accumuler
    // les atomes des préconditions ET des effets avant le tri unique.
    if let Some(root_id) = action.effect_mut().root_id() {
        expr::encode_to_pnf(
            root_id,
            action.effect_mut(),
            scratchpad,
            stack,
        )?;
    }

    // 3. Tri et dédoublonnement local sur le buffer.
    // Cela garantit que chaque action ne fournit que des IDs uniques au global.
    scratchpad.sort_unstable();
    scratchpad.dedup();

    // 4. Fusion dans le vecteur global.
    // extend_from_slice est plus performant (memcpy) qu'un itérateur.
    negated_atoms.extend_from_slice(scratchpad);

    Ok(())
}
