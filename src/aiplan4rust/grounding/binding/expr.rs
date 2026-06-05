use crate::aiplan4rust::grounding::binding::error::BindingError;
use crate::aiplan4rust::grounding::binding::evaluator::{ExprConstant, ExprEvaluator};
use crate::aiplan4rust::grounding::binding::Bindings;
use crate::aiplan4rust::lir::expr::expr::Expr;
use crate::aiplan4rust::lir::expr::{ExprBuilder, ExprId, ExprKind, ExprStore};
use std::collections::hash_map::Entry;

use super::scratchpad::BindingScratchpad;

/// **Version Standard** : Applique uniquement les substitutions de variables (Grounding classique).
/// Alloue temporairement un scratchpad local.
#[inline]
pub fn bind(
    expr_id: ExprId,
    store: &mut ExprStore,
    sub: &Bindings,
) -> Result<ExprId, BindingError> {
    let mut scratchpad = BindingScratchpad::new();
    bind_with(expr_id, store, sub, None, &mut scratchpad)
}

/// **Version Avancée** : Moteur de parcours unique, ultra-optimisé et sans allocation.
/// Applique les substitutions et exécute l'élagage statique (Köhler) à la volée.
pub fn bind_with(
    expr_id: ExprId,
    store: &mut ExprStore,
    sub: &Bindings,
    evaluator: Option<&dyn ExprEvaluator>,
    scratchpad: &mut BindingScratchpad,
) -> Result<ExprId, BindingError> {
    scratchpad.clear();

    // On initialise notre builder unique pour l'interning
    let mut builder = ExprBuilder::new(store);

    // Amorçage du parcours itératif avec la racine
    scratchpad.stack.push((expr_id, false));
    scratchpad.substitution_map.insert(expr_id, expr_id);

    while let Some((old_id, children_pushed)) = scratchpad.stack.pop() {
        if !children_pushed {
            // --- PREMIER PASSAGE : Descente ---
            scratchpad.stack.push((old_id, true));

            let entry = &builder.store[old_id];
            for &child_id in entry.children().iter().rev() {
                if let Entry::Vacant(v) = scratchpad.substitution_map.entry(child_id) {
                    v.insert(child_id);
                    scratchpad.stack.push((child_id, false));
                }
            }
        } else {
            // --- SECOND PASSAGE : Remontée ---
            // Isolation de l'emprunt immuable sur `builder.store` pour extraire les infos du nœud
            let (entry_kind, has_children) = {
                let entry = &builder.store[old_id];
                (entry.kind().clone(), !entry.children().is_empty())
            };

            let mut has_changed = false;
            scratchpad.children_buffer.clear();

            // Si le nœud a des enfants, on collecte leurs correspondances mises à jour
            if has_children {
                let entry = &builder.store[old_id];
                for &child_id in entry.children() {
                    let new_child_id = *scratchpad
                        .substitution_map
                        .get(&child_id)
                        .unwrap_or(&child_id);
                    if new_child_id != child_id {
                        has_changed = true;
                    }
                    scratchpad.children_buffer.push(new_child_id);
                }
            }

            // Évaluation et reconstruction du nœud courant
            let mut current_id = match entry_kind {
                ExprKind::Variable(var_id) => {
                    if let Some(obj_id) = sub.get(&var_id) {
                        builder.object(obj_id)
                    } else {
                        old_id
                    }
                }
                _ => {
                    if has_changed {
                        builder.reconstruct(entry_kind, &scratchpad.children_buffer)?
                    } else {
                        old_id
                    }
                }
            };

            // --- Élagage métier Köhler ---
            if let Some(eval) = evaluator {
                let current_kind = builder.store[current_id].kind();

                if is_evaluable(current_kind) {
                    let expr_wrapper = Expr::new(current_id, builder.store);

                    if let Some(static_val) = eval.evaluate(expr_wrapper) {
                        current_id = match static_val {
                            ExprConstant::Boolean(true) => builder.store.empty_and(),
                            ExprConstant::Boolean(false) => builder.store.empty_or(),
                            ExprConstant::Number(n) => builder.number(*n),
                            ExprConstant::Object(o) => builder.object(o),
                        };
                    }
                }
            }

            // On écrase la valeur définitive calculée dans notre table
            scratchpad.substitution_map.insert(old_id, current_id);
        }
    }

    Ok(*scratchpad
        .substitution_map
        .get(&expr_id)
        .unwrap_or(&expr_id))
}

/// Filtre pour savoir si on doit soumettre le nœud à l'évaluateur statique.
#[inline]
fn is_evaluable(kind: &ExprKind) -> bool {
    matches!(kind, ExprKind::AtomicFormula(_) | ExprKind::Function(_))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::{AtomSkeletonId, ObjectId, PredicateSymbolId, VariableId};

    #[test]
    fn test_bind_variable_substitution() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_id = VariableId::new(42);
        let obj_id = ObjectId::new(99);
        let expr_var = builder.variable(var_id);

        let mut bindings = Bindings::new();
        bindings.insert(var_id, obj_id);

        let result = bind(expr_var, &mut store, &bindings);

        assert!(result.is_ok());
        let final_id = result.unwrap();

        assert_ne!(final_id, expr_var);
        if let ExprKind::Object(o) = store[final_id].kind() {
            assert_eq!(*o, obj_id);
        } else {
            panic!("The final node should be an Object.");
        }
    }

    #[test]
    fn test_bind_no_change_returns_same_id() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let expr_num = builder.number(10);
        let bindings = Bindings::new();

        let result = bind(expr_num, &mut store, &bindings).unwrap();

        assert_eq!(result, expr_num);
    }

    #[test]
    fn test_bind_with_static_evaluation_pruning() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let predicate_id = PredicateSymbolId::new(1);
        let skeleton_id = AtomSkeletonId::new(1);
        let expr_atomic = builder.atomic_formula(predicate_id, &[], skeleton_id);

        let bindings = Bindings::new();
        let evaluator = MockEvaluator {
            should_be_true: expr_atomic,
        };

        let mut scratchpad = BindingScratchpad::new();
        let result = bind_with(
            expr_atomic,
            &mut store,
            &bindings,
            Some(&evaluator),
            &mut scratchpad,
        );

        assert!(result.is_ok());
        let final_id = result.unwrap();

        let expected_empty_and = store.empty_and();
        assert_eq!(final_id, expected_empty_and);
    }

    struct MockEvaluator {
        should_be_true: ExprId,
    }

    impl ExprEvaluator for MockEvaluator {
        fn evaluate(&self, expr: Expr) -> Option<ExprConstant> {
            if expr.root_id() == self.should_be_true {
                Some(ExprConstant::Boolean(true))
            } else {
                None
            }
        }
    }

    #[test]
    fn test_bind_atomic_formula_basic_substitution() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = VariableId::new(1);
        let obj_100 = ObjectId::new(100);
        let pred_id = PredicateSymbolId::new(10);
        let skel_id = AtomSkeletonId::new(1);

        let v_node = builder.variable(var_x);
        let root = builder.atomic_formula(pred_id, &[v_node], skel_id);

        let mut bindings = Bindings::new();
        bindings.insert(var_x, obj_100);

        let result = bind(root, &mut store, &bindings);

        assert!(result.is_ok());
        let new_root = result.unwrap();

        assert_ne!(new_root, root);

        let children = store[new_root].children();
        let arg_id = children[1];

        if let ExprKind::Object(id) = store[arg_id].kind() {
            assert_eq!(*id, obj_100);
        } else {
            panic!("The argument was not substituted into an Object.");
        }
    }

    #[test]
    fn test_bind_partial_substitution() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let x_id = VariableId::new(1);
        let z_id = VariableId::new(99);
        let obj_100 = ObjectId::new(100);

        let var_x = builder.variable(x_id);
        let var_z = builder.variable(z_id);
        let root = builder.atomic_formula(
            PredicateSymbolId::new(10),
            &[var_x, var_z],
            AtomSkeletonId::new(1),
        );

        let mut bindings = Bindings::new();
        bindings.insert(x_id, obj_100);

        let new_root = bind(root, &mut store, &bindings).unwrap();

        assert_ne!(new_root, root);
        let children = store[new_root].children();

        let arg_x = children[1];
        if let ExprKind::Object(id) = store[arg_x].kind() {
            assert_eq!(*id, obj_100);
        } else {
            panic!("First argument should be an Object");
        }

        let arg_z = children[2];
        if let ExprKind::Variable(id) = store[arg_z].kind() {
            assert_eq!(*id, z_id);
        } else {
            panic!("Second argument should remain a Variable");
        }
    }

    #[test]
    fn test_bind_zero_clone_on_empty_bindings() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = builder.variable(VariableId::new(1));
        let root =
            builder.atomic_formula(PredicateSymbolId::new(10), &[var_x], AtomSkeletonId::new(1));

        let bindings = Bindings::new();
        let new_root = bind(root, &mut store, &bindings).unwrap();

        assert_eq!(new_root, root);
    }

    #[test]
    fn test_bind_with_evaluator_pruning_propagation_false() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let target_predicate = PredicateSymbolId::new(10);
        let target_skeleton = AtomSkeletonId::new(1);
        let var_x = builder.variable(VariableId::new(1));
        let expr_atomic = builder.atomic_formula(target_predicate, &[var_x], target_skeleton);

        let mut bindings = Bindings::new();
        bindings.insert(VariableId::new(1), ObjectId::new(100));

        let evaluator = MockEvaluatorPruneFalse { target_skeleton };
        let mut scratchpad = BindingScratchpad::new();
        let new_root = bind_with(
            expr_atomic,
            &mut store,
            &bindings,
            Some(&evaluator),
            &mut scratchpad,
        )
        .unwrap();

        let expected_false = store.empty_or();
        assert_eq!(new_root, expected_false);
    }

    struct MockEvaluatorPruneFalse {
        target_skeleton: AtomSkeletonId,
    }

    impl ExprEvaluator for MockEvaluatorPruneFalse {
        fn evaluate(&self, expr: Expr) -> Option<ExprConstant> {
            let expr_id = expr.root_id();
            let entry_kind = expr.store()[expr_id].kind();

            match entry_kind {
                ExprKind::AtomicFormula(skel_id) => {
                    if *skel_id == self.target_skeleton {
                        Some(ExprConstant::Boolean(false))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
    }

    #[test]
    fn test_bind_duplicate_children_diamond_sharing() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = VariableId::new(1);
        let obj_100 = ObjectId::new(100);
        let v_node = builder.variable(var_x);

        let target_skeleton = AtomSkeletonId::new(5);
        let root = builder.atomic_formula(
            PredicateSymbolId::new(2),
            &[v_node, v_node],
            target_skeleton,
        );

        let mut bindings = Bindings::new();
        bindings.insert(var_x, obj_100);

        let result = bind(root, &mut store, &bindings);

        assert!(result.is_ok());
        let new_root = result.unwrap();

        let children = store[new_root].children();
        assert_eq!(children[1], children[2]);
        if let ExprKind::Object(id) = store[children[1]].kind() {
            assert_eq!(*id, obj_100);
        } else {
            panic!("Diamond substitution failed.");
        }
    }

    #[test]
    fn test_bind_with_static_evaluation_to_number() {
        let mut store = ExprStore::new();

        let (expr_function, function_skeleton) = {
            let mut builder = ExprBuilder::new(&mut store);
            let var_x = builder.variable(VariableId::new(1));
            let function_skeleton = AtomSkeletonId::new(42);
            let expr_function =
                builder.atomic_formula(PredicateSymbolId::new(9), &[var_x], function_skeleton);
            (expr_function, function_skeleton)
        };

        let bindings = Bindings::new();
        let evaluator = MockEvaluatorNumber {
            target_skeleton: function_skeleton,
        };

        let mut scratchpad = BindingScratchpad::new();
        let new_root = bind_with(
            expr_function,
            &mut store,
            &bindings,
            Some(&evaluator),
            &mut scratchpad,
        )
        .unwrap();

        let expected_number_node = ExprBuilder::new(&mut store).number(42.0);
        assert_eq!(new_root, expected_number_node);
    }

    struct MockEvaluatorNumber {
        target_skeleton: AtomSkeletonId,
    }

    impl ExprEvaluator for MockEvaluatorNumber {
        fn evaluate(&self, expr: Expr) -> Option<ExprConstant> {
            let expr_id = expr.root_id();
            if let ExprKind::AtomicFormula(skel_id) = expr.store()[expr_id].kind() {
                if *skel_id == self.target_skeleton {
                    return Some(ExprConstant::Number(42.0.into()));
                }
            }
            None
        }
    }

    #[test]
    fn test_bind_non_evaluable_nodes_bypass_evaluator() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = VariableId::new(7);
        let expr_var = builder.variable(var_x);

        let mut bindings = Bindings::new();
        bindings.insert(var_x, ObjectId::new(88));

        let evaluator = MockEvaluatorAggressive;

        let mut scratchpad = BindingScratchpad::new();
        let new_root = bind_with(
            expr_var,
            &mut store,
            &bindings,
            Some(&evaluator),
            &mut scratchpad,
        )
        .unwrap();

        if let ExprKind::Object(id) = store[new_root].kind() {
            assert_eq!(*id, ObjectId::new(88));
        } else {
            panic!("Variable should have bypassed the evaluator.");
        }
    }

    struct MockEvaluatorAggressive;

    impl ExprEvaluator for MockEvaluatorAggressive {
        fn evaluate(&self, _expr: Expr) -> Option<ExprConstant> {
            Some(ExprConstant::Boolean(true))
        }
    }
}
