use std::collections::{HashMap, HashSet};
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lang::{AtomSkeletonID, ObjectID};
use crate::aiplan4rust::lir::analysis::inertia::table::InertiaTable;
use crate::aiplan4rust::lir::expr::ExprKind;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::LiftedProblem;

#[derive(Debug, Default)]
pub struct StaticFactIndex {
    /// Mapping: PredicateID -> (Bitmask -> (Tuple de constantes -> Nombre d'occurrences))
    /// Le bitmask est un u8 (max 8 arguments, ce qui est énorme pour du PDDL).
    tables: HashMap<AtomSkeletonID, HashMap<u16, HashMap<Vec<ObjectID>, usize>>>,
}


impl StaticFactIndex {

    pub fn new() -> Self {
        Self {
            tables: HashMap::new()
        }
    }


    pub fn build(problem: &LiftedProblem, inertia: &InertiaTable) -> Result<Self, LirError> {
        let mut index = StaticFactIndex::new();
        let init = problem.init();
        let mut iter = init.preorder().values();

        while let Some(node) = iter.next() {
            match node.kind() {
                ExprKind::AtomicFormula => {
                    let pred_node_id = node.try_child(0)?;
                    let pred_node = init.try_node(pred_node_id)?;
                    let pred_id = pred_node.try_atom_skeleton()?;

                    if inertia.is_predicate_positive(pred_id).unwrap_or(false) {
                        let num_children = node.children().len();
                        let mut args = Vec::with_capacity(num_children.saturating_sub(1));

                        for i in 1..num_children {
                            let arg_node_id = node.try_child(i)?;
                            let arg_node = init.try_node(arg_node_id)?;
                            let obj_id = arg_node.try_constant()?;
                            args.push(obj_id);
                        }

                        let arity = args.len();

                        // Sécurité : Vérifier si l'arité rentre dans un u16 (max 15 arguments)
                        // 1u16 << 16 provoquerait un overflow
                        let max_mask = (1u16).checked_shl(arity as u32)
                            .ok_or_else(|| LirError::arity_too_high(pred_id, arity))?;

                        let predicate_entry = index.tables
                            .entry(pred_id)
                            .or_default();

                        for mask in 0..max_mask {
                            let table = predicate_entry.entry(mask).or_default();
                            let mut constants_key = Vec::with_capacity(arity);

                            for bit_pos in 0..arity {
                                // On utilise u16 pour le décalage du masque
                                if (mask & (1 << (arity - 1 - bit_pos))) != 0 {
                                    constants_key.push(args[bit_pos]);
                                }
                            }
                            *table.entry(constants_key).or_insert(0) += 1;
                        }
                    }
                    iter.skip_subtree();
                }
                _ => {}
            }
        }
        Ok(index)
    }
    pub fn is_fact_true(&self, pred_id: AtomSkeletonID, args: &[ObjectID]) -> bool {
        let arity = args.len();
        if arity >= 16 { return false; }

        let mask = (1u16 << arity) - 1;

        self.tables.get(&pred_id)
            .and_then(|masks| masks.get(&mask))
            .and_then(|table| table.get(args))
            .map(|&count| count > 0)
            .unwrap_or(false)
    }

    pub fn get_count(&self, pred_id: AtomSkeletonID, mask: u16, constants: &[ObjectID]) -> usize {
        self.tables.get(&pred_id)
            .and_then(|masks| masks.get(&mask))
            .and_then(|table| table.get(constants))
            .copied()
            .unwrap_or(0)
    }
}
