use crate::aiplan4rust::compiler::lir::expr::builder::{ExprBuilder, ExprBuilderError};
use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprKind};
use crate::aiplan4rust::support::lang::{
    Type, TypeId, TypedList, TypedListId, TypedSymbol, VariableId,
};

impl<'a> ExprBuilder<'a> {
    /// Creates a universal quantifier (`forall`) expression.
    ///
    /// This function attempts to flatten nested quantifiers of the same type and
    /// performs variable cleanup.
    ///
    /// # Parameters
    /// * `vars`: A `TypedList` of variables to be universally quantified.
    /// * `body`: The `ExprId` representing the scope of the quantifier.
    ///
    /// # Return Value
    /// Returns an `Ok(ExprId)` of the interned expression, or an `Err(ExprBuilderError)`
    /// if the variable list contains duplicates.
    pub fn forall(&mut self, vars: TypedListId, body: ExprId) -> Result<ExprId, ExprBuilderError> {
        self.quantified_expression(vars, body, true)
    }

    /// Creates an existential quantifier (`exists`) expression.
    ///
    /// This function attempts to flatten nested quantifiers of the same type and
    /// performs variable cleanup.
    ///
    /// # Parameters
    /// * `vars`: A `TypedList` of variables to be existentially quantified.
    /// * `body`: The `ExprId` representing the scope of the quantifier.
    ///
    /// # Return Value
    /// Returns an `Ok(ExprId)` of the interned expression, or an `Err(ExprBuilderError)`
    /// if the variable list contains duplicates.
    pub fn exists(&mut self, vars: TypedListId, body: ExprId) -> Result<ExprId, ExprBuilderError> {
        self.quantified_expression(vars, body, false)
    }

    /// Interns a quantified expression (Forall or Exists) into the old.
    ///
    /// This function performs automatic flattening of nested quantifiers of the same type,
    /// cleans up the variable list (sorting, deduplication, and removal of unused variables),
    /// and handles edge cases for empty logical constants.
    ///
    /// # Parameters
    /// * `vars`: The initial list of variables for this quantifier.
    /// * `body`: The `ExprId` of the expression inside the quantifier.
    /// * `is_forall`: A boolean flag; `true` for `Forall`, `false` for `Exists`.
    ///
    /// # Return Value
    /// Returns an `Ok(ExprId)` pointing to the interned expression, or an `Err(ExprBuilderError)`
    /// if a semantic error (like duplicate variables) is detected during the cleaning process.
    #[inline]
    /*fn quantified_expression(
        &mut self,
        vars: TypedList<VariableId, TypeId>,
        body: ExprId,
        is_forall: bool,
    ) -> Result<ExprId, ExprBuilderError> {
        let empty_and = self.empty_and();
        let empty_or = self.empty_or();

        // Constant folding: if the body is a neutral/identity element, return it directly.
        if body == empty_and || body == empty_or {
            return Ok(body);
        }

        // Initialize the shared buffer with the initial variables.
        self.vars_buffer.clear();
        self.vars_buffer.extend_from_slice(vars.as_slice());

        let mut current_body = body;

        // 1. Flattening: Merge nested quantifiers of the same kind (e.g., forall(?x, forall(?y, P)) -> forall(?x, ?y, P))
        while let Some(node) = self.store.get(current_body) {
            let node_kind = node.kind();

            // Check if the nested node matches the current quantifier kind.
            let can_flatten = if is_forall {
                matches!(node_kind, ExprKind::Forall(_))
            } else {
                matches!(node_kind, ExprKind::Exists(_))
            };

            if can_flatten {
                let next_body = node.children()[0];

                // Re-check for empty constants after flattening.
                if next_body == empty_and || next_body == empty_or {
                    self.vars_buffer.clear();
                    return Ok(next_body);
                }

                // Extract variables from the nested quantifier and move deeper.
                if let ExprKind::Forall(inner) | ExprKind::Exists(inner) = node_kind {
                    self.vars_buffer.extend_from_slice(inner.as_slice());
                }
                current_body = next_body;
            } else {
                break;
            }
        }

        // 2. Cleaning: Sort, check for duplicates, and remove non-free variables.
        // This uses our strict validation which may return a DuplicateVariable error.
        self.clean_vars_buffer(current_body)?;

        // If all variables were removed (e.g., none are free in the body), return the body directly.
        if self.vars_buffer.is_empty() {
            return Ok(current_body);
        }

        // 3. Finalization: Transfer buffer ownership to create the final entry.
        let final_vars = self.vars_buffer.take();
        let kind = if is_forall {
            ExprKind::Forall(final_vars)
        } else {
            ExprKind::Exists(final_vars)
        };

        Ok(self.store.intern(kind, &[current_body]))
    }*/

    fn quantified_expression(
        &mut self,
        vars: TypedListId,
        body: ExprId,
        is_forall: bool,
    ) -> Result<ExprId, ExprBuilderError> {
        let empty_and = self.empty_and();
        let empty_or = self.empty_or();

        // Constant folding: if the body is a neutral/identity element, return it directly.
        if body == empty_and || body == empty_or {
            return Ok(body);
        }

        // --- CORRIGÉ : Initialisation linéaire et stricte avec fetch_typed_list ---
        self.vars_buffer.clear();
        let initial_list = self.store.fetch_typed_list(vars)?;

        self.vars_buffer.extend_from_slice(initial_list.as_slice());

        let mut current_body = body;

        // 1. Flattening: Merge nested quantifiers of the same kind
        while let Some(node) = self.store.get(current_body) {
            let node_kind = node.kind();

            // On inspecte les nouveaux variants légers de l'arène
            let can_flatten = if is_forall {
                matches!(node_kind, ExprKind::Forall(_))
            } else {
                matches!(node_kind, ExprKind::Exists(_))
            };

            if can_flatten {
                let next_body = node.children()[0];

                // Re-check for empty constants after flattening.
                if next_body == empty_and || next_body == empty_or {
                    self.vars_buffer.clear();
                    return Ok(next_body);
                }

                // Extraction par déréférencement sécurisé via l'API publique du Store
                if let ExprKind::Forall(list_id) | ExprKind::Exists(list_id) = node_kind {
                    if let Some(inner_list) = self.store.get_typed_list(*list_id) {
                        self.vars_buffer.extend_from_slice(inner_list.as_slice());
                    }
                }
                current_body = next_body;
            } else {
                break;
            }
        }

        // 2. Cleaning: Sort, check for duplicates, and remove non-free variables.
        self.clean_vars_buffer(current_body)?;

        // If all variables were removed (e.g., none are free in the body), return the body directly.
        if self.vars_buffer.is_empty() {
            return Ok(current_body);
        }

        // 3. Finalization: Transfer buffer ownership to the arena store, then intern the expression.
        let final_vars = self.vars_buffer.take();

        // Internement de la liste (Hash-Consing des paramètres)
        let list_id = self.store.intern_typed_list(final_vars);

        let kind = if is_forall {
            ExprKind::Forall(list_id)
        } else {
            ExprKind::Exists(list_id)
        };

        // Seul le corps reste dans le tableau des enfants
        Ok(self.store.intern(kind, &[current_body]))
    }

    /// Cleans the variable buffer by sorting, validating uniqueness, and removing non-free variables.
    ///
    /// This ensures a deterministic order for Hash-Consing and avoids redundant quantifiers.
    ///
    /// # Parameters
    /// * `body`: The `ExprId` of the expression where the variables are applied.
    ///
    /// # Errors
    /// Returns `ExprBuilderError::DuplicateVariable` if the same symbol is declared multiple times
    /// in the same scope.
    #[inline]
    fn clean_vars_buffer(&mut self, body: ExprId) -> Result<(), ExprBuilderError> {
        if self.vars_buffer.is_empty() {
            return Ok(());
        }

        // 1. Sort by symbol ID to ensure deterministic order for Hash-Consing.
        self.vars_buffer.sort_by_symbol();

        // 2. Strict validation: Check for duplicates in a single pass using windows.
        // Since the buffer is sorted, duplicates are adjacent.
        if let Some(duplicate) = self
            .vars_buffer
            .as_slice()
            .windows(2)
            .find(|w| w[0].symbol() == w[1].symbol())
        {
            return Err(ExprBuilderError::DuplicateVariable(duplicate[0].symbol()));
        }

        // 3. Remove duplicates (keeping the first one, though the error usually prevents reaching here).
        self.vars_buffer.dedup_by_symbol();

        // 4. Intelligent filtering: remove quantified variables that do not appear in the body.
        // This leverages the Store's free variable cache.
        self.vars_buffer
            .retain(|v| self.store.is_variable_free(body, v.symbol()));

        Ok(())
    }

    /// Creates and interns a typed variable list from any collection convertible into a vector of typed symbols.
    ///
    /// # Parameters
    /// * `vars`: Any collection or structure that implements `Into<Vec<TypedSymbol<VariableId, TypeId>>>`.
    ///   This includes `Vec`, boxed slices, or fixed-size arrays (e.g., `[var_x, var_y]`).
    ///
    /// # Return Value
    /// Returns a `TypedListId` representing the interned list inside the store.
    #[inline]
    pub fn typed_variable_list<I>(&mut self, vars: I) -> TypedListId
    where
        I: Into<Vec<TypedSymbol<VariableId, TypeId>>>,
    {
        // 1. Wrap the converted vector into a TypedList structure.
        let raw_list = TypedList::from(vars.into());

        // 2. Intern the list into the store to obtain its unique, structural TypedListId.
        self.store.intern_typed_list(raw_list)
    }

    /// Creates a typed variable symbol consisting of a unique identifier and a type.
    ///
    /// # Parameters
    /// * `id`: Anything convertible into a `VariableId` (e.g., `usize`, `VariableId`).
    /// * `type_ids`: Any collection that can be borrowed as a slice of `usize` identifiers
    ///   (e.g., fixed-size arrays like `[101]`, vectors, or slices).
    ///
    /// # Return Value
    /// Returns a `TypedSymbol` mapping the `VariableId` to its corresponding `TypeId`.
    #[inline]
    pub fn typed_variable<V, T>(&mut self, id: V, type_ids: T) -> TypedSymbol<VariableId, TypeId>
    where
        V: Into<VariableId>,
        T: AsRef<[usize]>,
    {
        // Convert the ID automatically and borrow the type identifiers as a slice
        TypedSymbol::new(id.into(), self.ty(type_ids.as_ref()))
    }

    /// Constructs a `Type` object, representing either a primitive type
    /// or a union type (`either`).
    ///
    /// # Parameters
    /// * `ids`: Any collection that can be borrowed as a slice of `usize` identifiers
    ///   (e.g., fixed-size arrays like `[101]`, vectors, or slices).
    ///
    /// # Return Value
    /// Returns a `Type<TypeId>`.
    /// - If `ids` contains one element, returns a primitive type.
    /// - If `ids` is empty, returns the root type.
    /// - Otherwise, returns an `either` union of the provided type IDs.
    #[inline]
    pub fn ty<T>(&mut self, ids: T) -> Type<TypeId>
    where
        T: AsRef<[usize]>,
    {
        match ids.as_ref() {
            // Single type case: Zero allocation, directly on the stack.
            [single_id] => Type::primitive(TypeId::from(*single_id)),

            // Empty case: Root type.
            [] => Type::root(),

            // Multiple types case: Collect directly into Type.
            // Thanks to FromIterator, SmallVec automatically handles stack-to-heap
            // transition only if ids.len() exceeds its inline capacity.
            slice => slice.iter().map(|&id| TypeId::from(id)).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::lir::expr::ExprStore;
    use crate::aiplan4rust::support::lang::{AtomSkeletonId, PredicateSymbolId, VariableId};

    // This test ensures that nested quantifiers of the same type are merged into one
    // and that variables are sorted to maintain a unique structural identity.
    #[test]
    fn test_quantifier_flattening_and_canonicalization() {
        let mut store = ExprStore::new();

        // Context pour la formule atomique
        let sym = PredicateSymbolId::from(1);
        let skel = AtomSkeletonId::from(10);

        // On isole la construction dans un scope pour détruire le builder et libérer store
        let (expr_flat, expr_nested, body) = {
            let mut builder = ExprBuilder::new(&mut store);

            // On définit nos variables typées
            let var_a = builder.typed_variable(10, &[1]);
            let var_b = builder.typed_variable(20, &[1]);

            // On récupère les ExprId des variables pour les mettre dans le corps
            let arg_a = builder.variable(VariableId::from(10));
            let arg_b = builder.variable(VariableId::from(20));

            // CORRECTION : Le corps doit contenir les variables 10 et 20
            // pour que le builder ne les supprime pas !
            let body_id = builder.atomic_formula(sym, &[arg_a, arg_b], skel);

            // Cas 1 : Création directe (forall {10, 20} P(?10, ?20))
            let list_flat = builder.typed_variable_list(vec![var_a.clone(), var_b.clone()]);
            let flat_id = builder.forall(list_flat, body_id).unwrap();

            // Cas 2 : Création imbriquée (forall {20} (forall {10} P(?10, ?20)))
            let list_a = builder.typed_variable_list(vec![var_a]);
            let list_b = builder.typed_variable_list(vec![var_b]);

            // Le premier appel crée Forall({10}, P(10, 20))
            let inner_forall = builder.forall(list_a, body_id).unwrap();
            // Le second appel aplatit avec {20} pour donner Forall({10, 20}, P(10, 20))
            let nested_id = builder.forall(list_b, inner_forall).unwrap();

            (flat_id, nested_id, body_id)
        };

        // Vérification 1 : Les IDs doivent être identiques grâce au flattening + tri + Hash-Consing
        assert_eq!(
            expr_flat, expr_nested,
            "Flattening or variable sorting failed: IDs should be identical"
        );

        // Vérification 2 : Structure du nœud
        let node = store.get(expr_flat).expect("Expression should exist");

        // CORRECTION : Extraction via ForallNew et récupération de la liste typée dans le store
        if let ExprKind::Forall(vars_id) = node.kind() {
            let vars = store
                .get_typed_list(*vars_id)
                .expect("Typed list must exist");

            assert_eq!(vars.len(), 2, "Should have exactly 2 variables");
            // Vérification du tri (10 < 20)
            assert_eq!(vars[0].symbol(), VariableId::from(10));
            assert_eq!(vars[1].symbol(), VariableId::from(20));

            // Vérification que le corps est bien l'atome original
            assert_eq!(node.children()[0], body);
        } else {
            panic!("Resulting expression is not a Forall node. Check if variables were pruned!");
        }
    }

    // This test verifies that quantifiers with constant bodies (true/false) are
    // immediately simplified to the constant itself without creating a node.
    #[test]
    fn test_quantifier_constant_folding() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let var = builder.typed_variable(1, &[0]);
        let list = builder.typed_variable_list(vec![var]);

        let true_expr = builder.empty_and();
        let false_expr = builder.empty_or();

        // Input: (forall x (true))
        // Output: true_expr
        assert_eq!(
            builder.forall(list.clone(), true_expr).unwrap(),
            true_expr,
            "Forall with true body must return true directly"
        );

        // Input: (exists x (false))
        // Output: false_expr
        assert_eq!(
            builder.exists(list, false_expr).unwrap(),
            false_expr,
            "Exists with false body must return false directly"
        );
    }

    // This test checks that duplicate variables in a list are removed and that
    // a quantifier with no variables is bypassed entirely.
    // This test ensures that the builder correctly detects and rejects duplicate
    // variable declarations within the same quantifier scope.
    #[test]
    fn test_variable_deduplication_and_vacuity() -> Result<(), ExprBuilderError> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // We create two variables with the same ID (1)
        let var_x1 = builder.typed_variable(1, &[0]);
        let var_x2 = builder.typed_variable(1, &[0]);

        let arg_x = builder.variable(VariableId::from(1));
        let sym = PredicateSymbolId::from(1);
        let skel = AtomSkeletonId::from(10);
        let body = builder.atomic_formula(sym, &[arg_x], skel);

        // We create a list containing the same ID twice
        let list_with_dupes = builder.typed_variable_list(vec![var_x1, var_x2]);

        // HERE: We do NOT use .unwrap(), we capture the Result to check for the error
        let result = builder.forall(list_with_dupes, body);

        // Verification: The builder must return an error because of the duplicate
        assert!(
            result.is_err(),
            "The builder should have rejected duplicate variables"
        );

        if let Err(e) = result {
            assert!(
                matches!(e, ExprBuilderError::DuplicateVariable(_)),
                "The error should be DuplicateVariable, but got: {:?}",
                e
            );
        }

        Ok(())
    }

    // This test ensures that the builder maintains type integrity by preventing
    // multiple declarations of the same variable ID with different types.
    #[test]
    fn test_variable_type_safety() -> Result<(), ExprBuilderError> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Variable ID 1 with Type 100
        let var_x_type_a = builder.typed_variable(1, &[100]);
        // Variable ID 1 with Type 200 -> ID conflict!
        let var_x_type_b = builder.typed_variable(1, &[200]);

        let arg_x = builder.variable(VariableId::from(1));
        let sym = PredicateSymbolId::from(1);
        let skel = AtomSkeletonId::from(10);
        let body = builder.atomic_formula(sym, &[arg_x], skel);

        let list = builder.typed_variable_list(vec![var_x_type_a, var_x_type_b]);

        // HERE: We expect this to fail
        let result = builder.forall(list, body);

        assert!(
            result.is_err(),
            "The builder must prevent two variables with the same ID in the same scope"
        );

        Ok(())
    }

    // This test ensures that the builder does not accidentally flatten different
    // quantifier types (e.g., nesting an Exists inside a Forall).
    // Logic: (forall x (exists y body)) must remain two separate nodes.
    #[test]
    fn test_quantifier_alternation_safety() -> Result<(), ExprBuilderError> {
        let mut store = ExprStore::new();

        // On isole la construction dans un scope pour détruire le builder et libérer store
        let (outer_forall, inner_exists) = {
            let mut builder = ExprBuilder::new(&mut store);

            // Définition des variables : IDs uniques 1 et 2
            let var_x = builder.typed_variable(1, &[0]);
            let var_y = builder.typed_variable(2, &[0]);

            // Récupération des ExprId pour les utiliser dans le corps
            let arg_x = builder.variable(VariableId::from(1));
            let arg_y = builder.variable(VariableId::from(2));

            let sym = PredicateSymbolId::from(1);
            let skel = AtomSkeletonId::from(10);

            // CORRECTION : Le corps DOIT utiliser les variables x et y
            // pour éviter qu'elles ne soient nettoyées par le builder.
            let body = builder.atomic_formula(sym, &[arg_x, arg_y], skel);

            // Input: (forall {x} (exists {y} body))
            let list_y = builder.typed_variable_list(vec![var_y]);
            let inner_exists_id = builder.exists(list_y, body)?; // Utilisation de ?

            let list_x = builder.typed_variable_list(vec![var_x]);
            let outer_forall_id = builder.forall(list_x, inner_exists_id)?; // Utilisation de ?

            (outer_forall_id, inner_exists_id)
        };

        // Vérification de la structure via le store directement
        let node = store.get(outer_forall).expect("Root should exist");

        // 1. La racine doit être un ForallNew (CORRECTION du variant)
        assert!(
            matches!(node.kind(), ExprKind::Forall(_)),
            "Root should be ForallNew node, but got {:?}",
            node.kind()
        );

        // 2. L'enfant doit être le ExistsNew (pas d'aplatissement entre types différents)
        let child_id = node.children()[0];
        assert_eq!(
            child_id, inner_exists,
            "Child ID should match the original Exists node (no flattening)"
        );

        // 3. Vérification que l'enfant est bien un ExistsNew (CORRECTION du variant)
        let child_node = store.get(child_id).expect("Child should exist");
        assert!(
            matches!(child_node.kind(), ExprKind::Exists(_)),
            "Child node should be ExistsNew"
        );

        Ok(())
    }

    // This test ensures the internal vars_buffer is properly isolated between calls.
    // Logic: Creating one expression should not leave "leftover" variables for the next one.
    #[test]
    fn test_builder_buffer_isolation() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_a = builder.typed_variable(1, &[0]);
        let var_b = builder.typed_variable(2, &[0]);

        // Create valid variable leaf expressions to pass as arguments
        let expr_a = builder.variable(VariableId::from(1));
        let expr_b = builder.variable(VariableId::from(2));

        let sym = PredicateSymbolId::from(1);
        let skel = AtomSkeletonId::from(10);

        // --- FIX: Formulate distinct bodies where variables are actually USED ---
        let body_a = builder.atomic_formula(sym, &[expr_a], skel);
        let body_b = builder.atomic_formula(sym, &[expr_b], skel);

        // Input: A sequence of two completely independent universal quantifier (forall) creations.
        let vars1 = builder.typed_variable_list(vec![var_a]);
        let _first = builder.forall(vars1, body_a);

        let vars2 = builder.typed_variable_list(vec![var_b]);
        let second = builder.forall(vars2, body_b).unwrap(); // Will now successfully create a ForallNew

        // --- BORROW CHECKER DECONFLICTION ---
        let target_vars_id = if let Some(node) = builder.get(second) {
            if let ExprKind::Forall(vars_id) = node.kind() {
                Some(*vars_id)
            } else {
                None
            }
        } else {
            None
        };

        // --- ASSERTION & ISOLATION CHECK ---
        if let Some(vars_id) = target_vars_id {
            let actual_vars = builder.store_mut().fetch_typed_list(vars_id).unwrap();

            assert_eq!(
                actual_vars.len(),
                1,
                "Buffer pollution detected: found too many variables inside the quantifier"
            );
            assert_eq!(
                actual_vars[0].symbol(),
                VariableId::from(2),
                "Buffer pollution detected: wrong variable ID retrieved from the quantifier"
            );
        } else {
            panic!("Failed to validate buffer isolation: Expression node or kind not found");
        }
    }

    // This test ensures the builder is "smart" enough to prune variables that
    // are declared but never used in the body, ensuring canonical Hash-Consing.
    // Logic: (forall {x, y} P(x)) -> Output should be identical to (forall {x} P(x))
    #[test]
    fn test_unused_variable_pruning_and_hash_consing() -> Result<(), ExprBuilderError> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        let var_x = builder.typed_variable(1, &[0]);
        let var_y = builder.typed_variable(2, &[0]);

        let sym = PredicateSymbolId::from(1);
        let skel = AtomSkeletonId::from(10);
        let arg_x = builder.variable(VariableId::from(1));
        let body = builder.atomic_formula(sym, &[arg_x], skel);

        // Input A: forall {x} P(x)
        let list_x = builder.typed_variable_list(vec![var_x.clone()]);
        let expr_clean = builder.forall(list_x, body)?;

        // Input B: forall {x, y} P(x) -- y is unused
        let list_xy = builder.typed_variable_list(vec![var_x, var_y]);
        let expr_with_unused = builder.forall(list_xy, body)?;

        // Output: Both must point to the same ID because the builder pruned 'y'
        assert_eq!(
            expr_clean, expr_with_unused,
            "The builder should prune unused variables to maintain Hash-Consing integrity"
        );

        Ok(())
    }
}
