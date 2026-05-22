use crate::aiplan4rust::grounding::passes::positive_form_normalization::expr;
use crate::aiplan4rust::lang::AtomSkeletonId;
use crate::aiplan4rust::lir::old::expr::ops::ExprOpError;
use crate::aiplan4rust::lir::MethodDef;
use crate::aiplan4rust::tree::NodeId;

/// Applies the Positive Normal Form (PNF) transformation to an HTN Method.
///
/// This processes both the method's preconditions and the logical constraints
/// within its task network.
///
/// It uses a reusable `scratchpad` buffer to collect negated atoms locally,
/// ensuring that the global registry receives only unique, sorted IDs per method.
/// Applies the Positive Normal Form (PNF) transformation to an HTN Method.
///
/// This processes both the method's preconditions and the logical constraints
/// within its task network.
pub fn to_pnf(
    method: &mut MethodDef,
    negated_atoms: &mut Vec<AtomSkeletonId>,
    scratchpad: &mut Vec<AtomSkeletonId>,
    stack: &mut Vec<(NodeId, bool)>, // La pile DFS réutilisable injectée
) -> Result<(), ExprOpError> {
    // 0. On vide le buffer de travail local (conserve la capacité allouée)
    scratchpad.clear();

    // 1. Transform Preconditions
    // Logic required for the method to be decomposing the task.
    if let Some(root_id) = method.precondition_mut().root_id() {
        expr::to_pnf(root_id, method.precondition_mut(), scratchpad, stack, false)?;
    }

    // 2. Transform Task Network Logical Constraints
    // HTN Task Networks can contain constraints (e.g., in HDDL) that
    // must be evaluated during decomposition.
    if let Some(root_id) = method
        .task_network_mut()
        .logical_constraints_mut()
        .root_id()
    {
        expr::to_pnf(
            root_id,
            method.task_network_mut().logical_constraints_mut(),
            scratchpad,
            stack,
            false,
        )?;
    }

    // 3. Local sorting and deduplication to keep the global vector clean.
    // Important car un prédicat peut être nié à la fois dans les préconditions et les contraintes.
    scratchpad.sort_unstable();
    scratchpad.dedup();

    // 4. Merge into the global problem vector.
    // Utilisation de extend_from_slice pour une copie mémoire directe et rapide.
    negated_atoms.extend_from_slice(scratchpad);

    Ok(())
}
