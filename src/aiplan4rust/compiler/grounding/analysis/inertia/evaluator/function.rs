use crate::aiplan4rust::compiler::grounding::analysis::inertia::evaluator::InertiaEvaluatorError;
use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprConstant;
use crate::aiplan4rust::compiler::lir::expr::{ExprKind, ExprNode, ExprStore};
use crate::analysis::inertia::evaluator::evaluator::ArgumentBuffer;
use crate::analysis::inertia::evaluator::InertiaEvaluator;
use ordered_float::OrderedFloat;

impl<'a> InertiaEvaluator<'a> {
    /// Évalue un terme de fonction numériquement ou par objet selon l'état initial.
    ///
    /// Cette méthode applique les simplifications d'inertie positive sur les fonctions numériques
    /// et s'assure de renvoyer le fallback PDDL par défaut (0.0) si aucun fait n'a été fourni
    /// dans l'état initial pour une fonction pleinement instanciée.
    pub(super) fn evaluate_function_internal(
        &self,
        node: ExprNode<'_>,
        store: &ExprStore,
        buffer: &mut ArgumentBuffer,
    ) -> Result<Option<ExprConstant>, InertiaEvaluatorError> {
        // Dans le LIR, l'ID de la fonction est encapsulé dans le Kind
        let func_id = match node.kind() {
            ExprKind::Function(id) => *id,
            _ => return Ok(None),
        };

        // 1. Check d'inertie : Si la fonction peut changer (fluent), on ne simplifie rien ici.
        if !self.inertia.is_function_positive_inertia(func_id)? {
            return Ok(None);
        }

        // 2. Extraction du masque et des arguments constants (saute l'index 0 du symbole)
        let mask = self.extract_mask_dynamic(node, store, buffer);

        // Protection contre les projections trop larges (IPP Section 3.4)
        let n_limit = buffer.len().min(self.max_proj);
        let lookup_slice = &buffer[..n_limit];

        // 3. Recherche de la valeur dans le registre statique construit au build()
        let mut value = self
            .static_functions
            .get(&func_id)
            .and_then(|masks| masks.get(&mask))
            .and_then(|entries| entries.get(lookup_slice))
            .copied();

        // 4. Logique de décision et Fallback standard PDDL
        if self.all_args_grounded(node, store) {
            if value.is_none() {
                // Si aucune valeur n'est trouvée dans l'init pour une fonction totalement instanciée
                let def = &self.function_defs[func_id.as_usize()];

                // Standard PDDL : une fonction numérique non initialisée vaut 0.0 par défaut
                if def.ty().is_number() {
                    value = Some(ExprConstant::Number(OrderedFloat(0.0)));
                } else {
                    // Pour les fonctions d'objets (Object-Fluents), on renvoie None.
                    value = None;
                }
            }
            return Ok(value);
        } else {
            // Cas avec Variables (instanciation partielle) :
            // Pas de simplification sans analyse d'unanimité (non implémentée).
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::grounding::analysis::inertia::inertia::Inertia;
    use crate::aiplan4rust::compiler::grounding::analysis::inertia::table::InertiaTable;
    use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
    use crate::aiplan4rust::compiler::lir::expr::{ExprBuilder, ExprKind, ExprStore};
    use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFunctionSkeleton;
    use crate::aiplan4rust::support::lang::{
        FunctionSkeletonId, FunctionSymbolId, ObjectId, Type, TypeId, TypedList, TypedSymbol,
        VariableId,
    };

    /// # Purpose
    /// Verifies the PDDL fallback rule for uninitialized grounded numeric functions.
    /// When a function is fully grounded (`f(99)`) but has no entry in the initial state,
    /// the evaluator must yield `0.0` as a default value if the return type is numeric.
    ///
    /// # Input
    /// - Function `f` marked as positive inert.
    /// - Grounded call expression `f(99)`.
    /// - The function skeleton's return type is explicitly set to a **Numeric** type.
    /// - No registration in the initial state masks.
    ///
    /// # Expected Output
    /// - `evaluate_function_internal` must return `Ok(Some(ExprConstant::Number(0.0)))`.
    #[test]
    fn test_evaluate_function_grounded_missing_returns_pddl_default_zero() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let skel_id_val = 1;
        let skel_id = FunctionSkeletonId::from(skel_id_val);
        let type_id = TypeId::from(0);
        let obj_99 = ObjectId::from(99);

        let mut i_table = InertiaTable::empty();
        i_table.insert_function(skel_id, Inertia::positive());

        // --- ALIGNEMENT SUR L'OPTION A ---
        // On s'assure que le type de retour est bien numérique.
        let numeric_type = Type::<TypeId>::number();

        let f_defs = vec![
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(0),
                TypedList::new(),
                Type::from(type_id),
            ),
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(skel_id_val),
                TypedList::from_iter(vec![TypedSymbol::new(
                    VariableId::from(0),
                    Type::from(type_id),
                )]),
                numeric_type, // Type de retour numérique pour activer le fallback PDDL à 0.0
            ),
        ];
        let p_defs = vec![];
        let v_reg = ValueRegistry::empty();

        // Utilisation du mock d'InertiaEvaluator
        let registry = InertiaEvaluator::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // Construction du nœud f(99) avec l'index 0 réservé au symbole dans ton LIR
        let const_node = builder.intern(ExprKind::Object(obj_99), &[]);
        let func_node_id = builder.intern(ExprKind::Function(skel_id), &[const_node]);
        let func_node = store.get(func_node_id).unwrap();

        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_function_internal(func_node, &store, &mut buffer);

        assert_eq!(
            res.unwrap(),
            Some(ExprConstant::Number(ordered_float::OrderedFloat(0.0))),
            "Une fonction numérique grounded manquante à l'init doit retourner 0.0 par défaut"
        );
    }
}
