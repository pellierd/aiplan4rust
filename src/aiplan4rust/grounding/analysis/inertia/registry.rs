use std::collections::{HashMap};
use crate::aiplan4rust::arena::{ArenaNode, NodeId};
use crate::aiplan4rust::lang::{AtomSkeletonId, FunctionSkeletonId, ObjectId, VariableId};
use crate::aiplan4rust::grounding::analysis::inertia::InertiaError;
use crate::aiplan4rust::grounding::analysis::inertia::table::InertiaTable;
use crate::aiplan4rust::lir::expr::{Expr, ExprKind, ExprNode};
use crate::aiplan4rust::lir::expr::ops::{StaticEvaluator, StaticValue};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::tree::Node;

/// Arité maximale supportée pour l'optimisation sur pile.
/// 16 est une marge de sécurité large pour du PDDL.
const MAX_ARITY: usize = 15;
const MAX_PROJ: usize = 3;

#[derive(Debug)]
pub struct InertiaRegistry {
    /// Mapping: PredicateID -> (Bitmask -> (Tuple de constantes -> Nombre d'occurrences))
    /// Le bitmask est un u8 (max 8 arguments, ce qui est énorme pour du PDDL).
    counting_predicates: HashMap<AtomSkeletonId, HashMap<u16, HashMap<[ObjectId; MAX_PROJ], usize>>>,
    static_functions: HashMap<FunctionSkeletonId, HashMap<u16, HashMap<[ObjectId; MAX_PROJ], StaticValue>>>,
    inertia: InertiaTable,
}


impl InertiaRegistry {
    pub fn new(inertia: InertiaTable) -> Self {
        Self {
            counting_predicates: HashMap::new(),
            static_functions: HashMap::new(),
            inertia,
        }
    }


    pub fn build(problem: &LiftedProblem, inertia: InertiaTable) -> Result<Self, InertiaError> {
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
        num_proj: usize,
    ) -> Result<(), InertiaError> {
        let children = node.children();

        match node.kind() {
            // CAS DES PRÉDICATS
            ExprKind::AtomicFormula => {
                let head_node = init.try_node(children[0])?;
                if let Ok(pred_id) = head_node.try_atom_skeleton() {
                    if self.inertia.is_predicate_positive(pred_id)? {
                        let arity = children.len().saturating_sub(1);
                        if arity == 0 { return Ok(()); }

                        let n_proj = num_proj.min(MAX_PROJ).min(arity);
                        let mut args_buffer = [ObjectId::default(); MAX_PROJ];
                        for (i, &arg_id) in children[1..].iter().enumerate().take(n_proj) {
                            args_buffer[i] = init.try_node(arg_id)?.try_constant()?;
                        }

                        // On appelle la fonction libre définie plus bas
                        generate_predicate_masks(
                            &mut self.counting_predicates,
                            pred_id,
                            arity,
                            &args_buffer,
                            n_proj,
                        );
                    }
                }
            }

            // CAS DES FONCTIONS
            ExprKind::FComp => {
                let func_term_node = init.try_node(children[0])?;
                if let Ok(func_id) = func_term_node.try_function_skeleton() {
                    if self.inertia.is_function_positive(func_id)? {
                        let func_children = func_term_node.children();
                        let arity = func_children.len().saturating_sub(1);

                        let n_proj = num_proj.min(MAX_PROJ).min(arity);
                        let mut args_buffer = [ObjectId::default(); MAX_PROJ];
                        for (i, &arg_id) in func_children[1..].iter().enumerate().take(n_proj) {
                            args_buffer[i] = init.try_node(arg_id)?.try_constant()?;
                        }

                        let val_node = init.try_node(children[1])?;
                        let value = if let Ok(num) = val_node.try_float() {
                            StaticValue::Number(num)
                        } else {
                            StaticValue::Object(val_node.try_constant()?)
                        };

                        // On appelle la fonction libre définie plus bas
                        generate_function_masks(
                            &mut self.static_functions,
                            func_id,
                            arity,
                            &args_buffer,
                            n_proj,
                            value,
                        );
                    }
                }
            }
            _ => {}
        }
        Ok(())
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
        let mut constants_buffer = [ObjectId::default(); MAX_ARITY];
        let (mask, num_constants) = extract_mask(node, expr, &mut constants_buffer);

        let count = count_predicate_occurrences(
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
    ) -> Result<Option<StaticValue>, InertiaError> {
        let node = expr.try_node(node_id)?;
        let func_id = node.try_function_skeleton()?;

        // Si la fonction n'est pas statique, on ne peut rien prédire
        if !self.inertia.is_function_positive(func_id)? { return Ok(None); }

        let mut constants_buffer = [ObjectId::default(); MAX_ARITY];
        let (mask, num_constants) = extract_mask(node, expr, &mut constants_buffer);

        // On cherche la valeur stockée
        let value = self.static_functions.get(&func_id)
            .and_then(|masks| masks.get(&mask))
            .and_then(|entries| {
                let mut key = [ObjectId::default(); MAX_PROJ];
                for (i, &obj) in constants_buffer[..num_constants.min(MAX_PROJ)].iter().enumerate() {
                    key[i] = obj;
                }
                entries.get(&key)
            })
            .copied();

        Ok(value)
    }


    pub fn evaluate_predicate_with_env(
        &self,
        node_id: NodeId,
        expr: &Expr,
        env: &HashMap<VariableId, ObjectId>, // Changé ici aussi
    ) -> Result<Option<bool>, InertiaError> {
        let node = expr.try_node(node_id)?;
        let pred_id = node.try_atom_skeleton()?;

        // --- FILTRAGE NIVEAU 1 : INERTIE PURE ---
        // Si le prédicat est déclaré "toujours faux" globalement,
        // on prune même s'il y a des fluents !
        if self.inertia.is_predicate_negative(pred_id)? {
            return Ok(Some(false));
        }

        // Si le prédicat n'est pas statique, on ne peut rien dire
        if !self.inertia.is_predicate_positive(pred_id)? {
            return Ok(None);
        }

        // --- FILTRAGE NIVEAU 2 : MASQUE DE KOEHLER ---
        let mut constants_buffer = [ObjectId::default(); MAX_ARITY];
        let (mask, num_constants) = extract_mask_with_env(node, expr, env, &mut constants_buffer);

        // Si le masque est 0 (que des fluents ou variables inconnues),
        // on ne peut pas interroger la counting_table
        if mask == 0 {
            return Ok(None);
        }

        let count = count_predicate_occurrences(
            &self.counting_predicates,
            pred_id,
            mask,
            &constants_buffer[..num_constants]
        );

        // Si count == 0, même avec des fluents aux autres positions,
        // aucune instance n'existe dans l'init avec ces objets constants précis.
        // Donc c'est FALSE.
        Ok(if count == 0 { Some(false) } else { None })
    }

    /// Retourne une vue d'évaluation du registre.
    /// On utilise souvent une référence vers self qui implémente le trait.
    pub fn evaluator(&self) -> &dyn StaticEvaluator {
        // Comme InertiaRegistry implémente déjà StaticEvaluator,
        // on se cast simplement en trait object.
        self as &dyn StaticEvaluator
    }
}

fn extract_mask_with_env(
    node: &ExprNode,
    expr: &Expr,
    env: &HashMap<VariableId, ObjectId>, // Changé de ObjectID à ArgumentID
    out_constants: &mut [ObjectId; MAX_ARITY],
) -> (u16, usize) {
    let children = node.children();
    if children.len() <= 1 { return (0, 0); }

    let pred_args = &children[1..];
    let arity = pred_args.len();
    let mut mask = 0u16;
    let mut count = 0;

    for (i, &arg_id) in pred_args.iter().enumerate() {
        let Ok(arg_node) = expr.try_node(arg_id) else { continue; };

        // On cherche à obtenir un ObjectID pour le filtrage Koehler
        let final_obj = match arg_node.content().try_constant() {
            Ok(obj_id) => Some(obj_id),
            Err(_) => {
                if let Ok(var_id) = arg_node.content().try_variable() {
                    // Si la variable est dans l'env, on ne prend l'ID que si c'est un Object
                    match env.get(&var_id) {
                        Some(obj_id) => Some(*obj_id),
                        _ => None, // C'est un ObjectFluentID ou Variable inconnue
                    }
                } else {
                    None
                }
            }
        };

        if let Some(obj_id) = final_obj {
            mask |= 1 << (arity - 1 - i);
            if count < MAX_ARITY {
                out_constants[count] = obj_id;
                count += 1;
            }
        }
    }
    (mask, count)
}

/// Vérifie l'arité de tous les prédicats du problème.
fn check_predicate_arity_limit(problem: &LiftedProblem) -> Result<(), InertiaError> {
    for (index, pred_def) in problem.predicate_defs().iter().enumerate() {
        let arity = pred_def.arity();
        if arity > MAX_ARITY {
            return Err(InertiaError::predicate_arity_too_high(
                AtomSkeletonId::from(index),
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
                FunctionSkeletonId::from(index),
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
    table: &mut HashMap<K, HashMap<u16, HashMap<[ObjectId; MAX_PROJ], usize>>>,
    key: K,
    arity: usize,
    args: &[ObjectId; MAX_PROJ],
    num_proj: usize,
) {
    if num_proj == 0 || arity == 0 { return; }

    let num_proj = num_proj.min(arity); // sécurité supplémentaire
    let mut buffer: [ObjectId; MAX_PROJ] = [ObjectId::default(); MAX_PROJ];

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
    out_constants: &mut [ObjectId; MAX_ARITY],
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
/// Compte le nombre d'occurrences d'un prédicat dans la table d'inertie.
#[inline(always)]
fn count_predicate_occurrences(
    table: &HashMap<AtomSkeletonId, HashMap<u16, HashMap<[ObjectId; MAX_PROJ], usize>>>,
    key: AtomSkeletonId,
    mask: u16,
    constants: &[ObjectId],
) -> usize {
    // 1. On prépare la clé fixe pour le lookup
    let mut lookup_key = [ObjectId::default(); MAX_PROJ];
    let to_copy = constants.len().min(MAX_PROJ);
    lookup_key[..to_copy].copy_from_slice(&constants[..to_copy]);

    // 2. On fouille dans la table
    table.get(&key)
        .and_then(|masks| masks.get(&mask))
        .and_then(|entries| entries.get(&lookup_key)) // Utilisation de la clé fixe
        .copied()
        .unwrap_or(0)
}

// Génère les masques pour les PRÉDICATS (Incrémente un compteur usize)
fn generate_predicate_masks(
    table: &mut HashMap<AtomSkeletonId, HashMap<u16, HashMap<[ObjectId; MAX_PROJ], usize>>>,
    key: AtomSkeletonId,
    arity: usize,
    args: &[ObjectId; MAX_PROJ],
    num_proj: usize,
) {
    if num_proj == 0 || arity == 0 { return; }
    let mut buffer = [ObjectId::default(); MAX_PROJ];
    let mut indices = [0; MAX_PROJ];
    for i in 0..num_proj { indices[i] = i; }

    loop {
        for i in 0..num_proj { buffer[i] = args[indices[i]]; }
        let mask: u16 = indices[..num_proj].iter().fold(0, |acc, &i| acc | (1 << (arity - 1 - i)));

        let entry = table.entry(key).or_default();
        let mask_table = entry.entry(mask).or_default();
        *mask_table.entry(buffer).or_insert(0) += 1;

        let mut done = true;
        for pos in (0..num_proj).rev() {
            let max_val = arity - num_proj + pos;
            if indices[pos] < max_val {
                indices[pos] += 1;
                for j in pos + 1..num_proj { indices[j] = indices[j - 1] + 1; }
                done = false;
                break;
            }
        }
        if done { break; }
    }
}

/// Génère les masques pour les FONCTIONS (Insère une InertiaValue)
fn generate_function_masks(
    table: &mut HashMap<FunctionSkeletonId, HashMap<u16, HashMap<[ObjectId; MAX_PROJ], StaticValue>>>,
    key: FunctionSkeletonId,
    arity: usize,
    args: &[ObjectId; MAX_PROJ],
    num_proj: usize,
    value: StaticValue,
) {
    if num_proj == 0 || arity == 0 { return; }
    let mut buffer = [ObjectId::default(); MAX_PROJ];
    let mut indices = [0; MAX_PROJ];
    for i in 0..num_proj { indices[i] = i; }

    loop {
        for i in 0..num_proj { buffer[i] = args[indices[i]]; }
        let mask: u16 = indices[..num_proj].iter().fold(0, |acc, &i| acc | (1 << (arity - 1 - i)));

        let entry = table.entry(key).or_default();
        let mask_table = entry.entry(mask).or_default();
        mask_table.insert(buffer, value);

        let mut done = true;
        for pos in (0..num_proj).rev() {
            let max_val = arity - num_proj + pos;
            if indices[pos] < max_val {
                indices[pos] += 1;
                for j in pos + 1..num_proj { indices[j] = indices[j - 1] + 1; }
                done = false;
                break;
            }
        }
        if done { break; }
    }



}

impl StaticEvaluator for InertiaRegistry {
    fn evaluate(&self, node_id: NodeId, expr: &Expr) -> Option<StaticValue> {
        let node = expr.try_node(node_id).ok()?;

        match node.kind() {
            // Évaluation des Prédicats
            ExprKind::AtomicFormula => {
                self.evaluate_predicate(node_id, expr)
                    .ok()
                    .flatten()
                    .map(StaticValue::Boolean)
            }

            // Évaluation des Fonctions
            ExprKind::FunctionTerm => {
                // Si tu as remplacé InertiaValue par StaticValue dans le registre,
                // evaluate_function renvoie déjà Option<StaticValue>
                self.evaluate_function(node_id, expr)
                    .ok()
                    .flatten()
            }

            _ => None,
        }
    }



}
