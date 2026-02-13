use std::collections::{HashMap};
use crate::aiplan4rust::arena::{ArenaNode, NodeId};
use crate::aiplan4rust::lang::{AtomSkeletonID, FunctionSkeletonID, ObjectID, PredicateID};
use crate::aiplan4rust::lir::analysis::inertia::InertiaError;
use crate::aiplan4rust::lir::analysis::inertia::table::InertiaTable;
use crate::aiplan4rust::lir::expr::{Expr, ExprKind, ExprNode};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::LiftedProblem;

/// Arité maximale supportée pour l'optimisation sur pile.
/// 16 est une marge de sécurité large pour du PDDL.
const MAX_ARITY: usize = 15;
const MAX_PROJ: usize = 3;

#[derive(Debug)]
pub struct InertiaRegistry<'a> {
    /// Mapping: PredicateID -> (Bitmask -> (Tuple de constantes -> Nombre d'occurrences))
    /// Le bitmask est un u8 (max 8 arguments, ce qui est énorme pour du PDDL).
    counting_predicates: HashMap<AtomSkeletonID, HashMap<u16, HashMap<[ObjectID; MAX_PROJ], usize>>>,
    counting_functions: HashMap<FunctionSkeletonID, HashMap<u16, HashMap<[ObjectID; MAX_PROJ], usize>>>,
    inertia: &'a InertiaTable,
}


impl<'a> InertiaRegistry<'a> {

    pub fn new(inertia: &'a InertiaTable) -> Self {
        Self {
            counting_predicates: HashMap::new(),
            counting_functions: HashMap::new(),
            inertia,
        }
    }


    pub fn build(problem: &LiftedProblem, inertia: &'a InertiaTable) -> Result<Self, InertiaError> {
        // 1. Vérification des arités avant toute opération
        check_predicate_arity_limit(problem)?;
        check_function_arity_limit(problem)?;

        // 2. Construction de l’index
        let mut index = Self::new(inertia);
        let init = problem.init();

        let mut iter = init.preorder().values();
        while let Some(node) = iter.next() {
            if let ExprKind::AtomicFormula = node.kind() {
                // plus besoin de Vec, process_init gère le buffer fixe [ObjectID; MAX_PROJ]
                index.process_init(node, init, MAX_PROJ)?;
                iter.skip_subtree(); // chaque atome est une feuille
            }
        }

        Ok(index)
    }




    fn process_init(
        &mut self,
        node: &ExprNode,
        init: &Expr,
        num_proj: usize, // dynamique, ≤ MAX_PROJ
    ) -> Result<(), InertiaError> {
        let children = node.children();
        let arity = children.len().saturating_sub(1);

        if arity == 0 { return Ok(()); }

        // nombre maximum de variables projetées : jamais dépasser MAX_PROJ ni arity
        let num_proj = num_proj.min(MAX_PROJ).min(arity);

        // buffer fixe sur pile
        let mut args_buffer: [ObjectID; MAX_PROJ] = [ObjectID::default(); MAX_PROJ];
        for (i, &arg_id) in children[1..].iter().enumerate().take(num_proj) {
            args_buffer[i] = init.try_node(arg_id)?.try_constant()?;
        }

        let head_node = init.try_node(children[0])?;
        if let Ok(pred_id) = head_node.try_atom_skeleton() {
            if self.inertia.is_predicate_positive(pred_id)? {
                generate_masks_limited(&mut self.counting_predicates, pred_id, arity, &args_buffer, num_proj);
            }
        } else if let Ok(func_id) = head_node.try_function_skeleton() {
            if self.inertia.is_function_positive(func_id)? {
                generate_masks_limited(&mut self.counting_functions, func_id, arity, &args_buffer, num_proj);
            }
        }

        Ok(())
    }



    /// Tente de réduire statiquement un parent (AND/OR) pour un prédicat
    pub fn can_reduce_predicate(
        &self,
        child_id: NodeId,
        expr: &Expr,
        parent_kind: ExprKind,
    ) -> Result<Option<bool>, InertiaError> {
        can_reduce_generic(child_id, expr, parent_kind, |id, e| {
            self.evaluate_predicate(id, e)
        })
    }

    /// Tente de réduire statiquement un parent (AND/OR) pour une fonction
    pub fn can_reduce_function(
        &self,
        child_id: NodeId,
        expr: &Expr,
        parent_kind: ExprKind,
    ) -> Result<Option<bool>, InertiaError> {
        can_reduce_generic(child_id, expr, parent_kind, |id, e| {
            self.evaluate_function(id, e)
        })
    }

    /// Évalue un prédicat statiquement sans allocation dynamique
    pub fn evaluate_predicate(
        &self,
        node_id: NodeId,
        expr: &Expr,
    ) -> Result<Option<bool>, InertiaError> {
        let node = expr.try_node(node_id)?;
        let pred_id = node.try_atom_skeleton()?; // récupère l'ID du prédicat

        if self.inertia.is_predicate_negative(pred_id)? { return Ok(Some(false)); }
        if !self.inertia.is_predicate_positive(pred_id)? { return Ok(None); }

        // Buffer sur pile pour éviter allocations
        let mut constants_buffer = [ObjectID::default(); MAX_ARITY];
        let (mask, num_constants) = extract_mask(node, expr, &mut constants_buffer);

        let count = count_occurrences(
            &self.counting_predicates,
            pred_id,
            mask,
            &constants_buffer[..num_constants]
        );

        Ok(if count == 0 { Some(false) } else { None })
    }

    /// Évalue une fonction statiquement sans allocation dynamique
    pub fn evaluate_function(
        &self,
        node_id: NodeId,
        expr: &Expr,
    ) -> Result<Option<bool>, InertiaError> {
        let node = expr.try_node(node_id)?;
        let func_id = node.try_function_skeleton()?; // récupère l'ID de la fonction

        if self.inertia.is_function_negative(func_id)? { return Ok(Some(false)); }
        if !self.inertia.is_function_positive(func_id)? { return Ok(None); }

        // Buffer sur pile pour éviter allocations
        let mut constants_buffer = [ObjectID::default(); MAX_ARITY];
        let (mask, num_constants) = extract_mask(node, expr, &mut constants_buffer);

        let count = count_occurrences(
            &self.counting_functions,
            func_id,
            mask,
            &constants_buffer[..num_constants]
        );

        Ok(if count == 0 { Some(false) } else { None })
    }



}
/// Vérifie l'arité de tous les prédicats du problème.
fn check_predicate_arity_limit(problem: &LiftedProblem) -> Result<(), InertiaError> {
    for (index, pred_def) in problem.predicate_defs().iter().enumerate() {
        let arity = pred_def.arity();
        if arity > MAX_ARITY {
            return Err(InertiaError::predicate_arity_too_high(
                AtomSkeletonID::from(index),
                arity,
            ));
        }
    }
    Ok(())
}

/// Vérifie l'arité de toutes les fonctions du problème.
fn check_function_arity_limit(problem: &LiftedProblem) -> Result<(), InertiaError> {
    for (index, func_def) in problem.function_defs().iter().enumerate() {
        let arity = func_def.arity();
        if arity > MAX_ARITY {
            return Err(InertiaError::function_arity_too_high(
                FunctionSkeletonID::from(index),
                arity,
            ));
        }
    }
    Ok(())
}

/// Tente de réduire statiquement un parent (AND/OR) à une valeur constante
/// en se basant sur l'évaluation d'un de ses enfants.
///
/// `eval_fn` est une fonction qui évalue un nœud et renvoie Option<bool>.
#[inline(always)]
pub fn can_reduce_generic<F>(
    child_id: NodeId,
    expr: &Expr,
    parent_kind: ExprKind,
    eval_fn: F,
) -> Result<Option<bool>, InertiaError>
where
    F: Fn(NodeId, &Expr) -> Result<Option<bool>, InertiaError>,
{
    // On tente d’évaluer l’enfant via la fonction passée
    let Some(static_val) = eval_fn(child_id, expr)? else {
        return Ok(None);
    };

    match (parent_kind, static_val) {
        (ExprKind::And, false) => Ok(Some(false)),
        (ExprKind::Or, true) => Ok(Some(true)),
        _ => Ok(None),
    }
}


/// Génère les combinaisons de projections pour l’inertie, sur pile
fn generate_masks_limited<K: std::hash::Hash + Eq + Copy>(
    table: &mut HashMap<K, HashMap<u16, HashMap<[ObjectID; MAX_PROJ], usize>>>,
    key: K,
    arity: usize,
    args: &[ObjectID; MAX_PROJ],
    num_proj: usize,
) {
    if num_proj == 0 || arity == 0 { return; }

    let num_proj = num_proj.min(arity); // sécurité supplémentaire
    let mut buffer: [ObjectID; MAX_PROJ] = [ObjectID::default(); MAX_PROJ];

    // indices initiaux : 0..num_proj-1
    let mut indices: [usize; MAX_PROJ] = [0; MAX_PROJ];
    for i in 0..num_proj {
        indices[i] = i;
    }

    loop {
        for i in 0..num_proj {
            buffer[i] = args[indices[i]];
        }

        // calcul du mask Koehler
        let mask: u16 = indices[..num_proj].iter().fold(0, |acc, &i| acc | (1 << (arity - 1 - i)));

        let entry = table.entry(key).or_default();
        let mask_table = entry.entry(mask).or_default();
        *mask_table.entry(buffer).or_insert(0) += 1;

        // génération lexicographique
        let mut done = true;
        for pos in (0..num_proj).rev() {
            let max_val = arity - num_proj + pos;
            if indices[pos] < max_val {
                indices[pos] += 1;
                for j in pos + 1..num_proj {
                    indices[j] = indices[j - 1] + 1;
                }
                done = false;
                break;
            }
        }
        if done { break; }
    }
}


/// Extrait les arguments d'un nœud en masque Koehler et les écrit dans un buffer sur pile.
/// Renvoie `(mask, nombre d'arguments)` pour le nœud donné.
#[inline(always)]
pub fn extract_mask(
    node: &ExprNode,
    expr: &Expr,
    out_constants: &mut [ObjectID; MAX_ARITY],
) -> (u16, usize) {
    let children = node.children();
    if children.len() <= 1 {
        return (0, 0);
    }

    let pred_args = &children[1..];
    let arity = pred_args.len();
    let mut mask = 0u16;
    let mut count = 0;

    for (i, &arg_id) in pred_args.iter().enumerate() {
        if let Some(arg_node) = expr.try_node(arg_id).ok() {
            if let Ok(obj_id) = arg_node.try_constant() {
                // Calcul du bitmask selon Koehler
                mask |= 1 << (arity - 1 - i);

                // Remplissage sécurisé du buffer fixe
                if count < MAX_ARITY {
                    out_constants[count] = obj_id;
                    count += 1;
                }
            }
        }
    }

    (mask, count)
}

/// Compte le nombre d'occurrences d'un symbole (prédicat ou fonction) dans une table donnée.
#[inline(always)]
fn count_occurrences<K: Copy + std::hash::Hash + Eq>(
    table: &HashMap<K, HashMap<u16, HashMap<[ObjectID; MAX_PROJ], usize>>>,
    key: K,
    mask: u16,
    constants: &[ObjectID],
) -> usize {
    table.get(&key)
        .and_then(|masks| masks.get(&mask))
        .and_then(|entries| entries.get(constants))
        .copied()
        .unwrap_or(0)
}
