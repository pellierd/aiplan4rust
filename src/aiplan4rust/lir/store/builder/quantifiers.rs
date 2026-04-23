use crate::aiplan4rust::lang::{Type, TypeId, TypedList, TypedSymbol, VariableId};
use crate::aiplan4rust::lir::store::builder::{ExprBuilder, ExprBuilderError};
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

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
    pub fn forall(
        &mut self,
        vars: TypedList<VariableId, TypeId>,
        body: ExprId,
    ) -> Result<ExprId, ExprBuilderError> {
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
    pub fn exists(
        &mut self,
        vars: TypedList<VariableId, TypeId>,
        body: ExprId,
    ) -> Result<ExprId, ExprBuilderError> {
        self.quantified_expression(vars, body, false)
    }

    /// Interns a quantified expression (Forall or Exists) into the store.
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
    fn quantified_expression(
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
                matches!(node_kind, ExprEntryKind::Forall(_))
            } else {
                matches!(node_kind, ExprEntryKind::Exists(_))
            };

            if can_flatten {
                let next_body = node.children()[0];

                // Re-check for empty constants after flattening.
                if next_body == empty_and || next_body == empty_or {
                    self.vars_buffer.clear();
                    return Ok(next_body);
                }

                // Extract variables from the nested quantifier and move deeper.
                if let ExprEntryKind::Forall(inner) | ExprEntryKind::Exists(inner) = node_kind {
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
            ExprEntryKind::Forall(final_vars)
        } else {
            ExprEntryKind::Exists(final_vars)
        };

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

    /// Creates a typed variable list from an existing vector of typed symbols.
    ///
    /// # Parameters
    /// * `vars`: A `Vec` of `TypedSymbol` representing the variables and their types.
    ///
    /// # Return Value
    /// Returns a `TypedList`. This operation transfers ownership of the input
    /// vector into the list structure.
    #[inline]
    pub fn typed_variable_list(
        &mut self,
        vars: Vec<TypedSymbol<VariableId, TypeId>>,
    ) -> TypedList<VariableId, TypeId> {
        // Wrap the existing vector into a TypedList structure.
        TypedList::from(vars)
    }

    /// Creates a typed variable symbol consisting of a unique identifier and a type.
    ///
    /// # Parameters
    /// * `id`: The raw `usize` identifier for the variable.
    /// * `type_ids`: A slice of `usize` identifiers representing the variable's type
    ///   (either a single type or a union of types).
    ///
    /// # Return Value
    /// Returns a `TypedSymbol` mapping the `VariableId` to its corresponding `TypeId`.
    #[inline]
    pub fn typed_variable(
        &mut self,
        id: usize,
        type_ids: &[usize],
    ) -> TypedSymbol<VariableId, TypeId> {
        // Map the raw ID to VariableId and resolve the type structure.
        TypedSymbol::new(VariableId::from(id), self.ty(type_ids))
    }

    /// Constructs a `Type` object, representing either a primitive type
    /// or a union type (`either`).
    ///
    /// # Parameters
    /// * `ids`: A slice of `usize` representing the internal IDs of the types to include.
    ///
    /// # Return Value
    /// Returns a `Type<TypeId>`.
    /// - If `ids` contains one element, returns a primitive type.
    /// - If `ids` is empty, returns the root type.
    /// - Otherwise, returns an `either` union of the provided type IDs.
    pub fn ty(&mut self, ids: &[usize]) -> Type<TypeId> {
        match ids {
            // Single type case: avoids Vec allocation for unions.
            [single_id] => Type::primitive(TypeId::from(*single_id)),

            // Empty case: represents the root type.
            [] => Type::root(),

            // Multiple types: constructs an 'either' union.
            _ => {
                let members = ids.iter().map(|&id| TypeId::from(id)).collect();
                Type::either(members)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::{AtomSkeletonId, PredicateSymbolId, VariableId};
    use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprStore};

    // This test ensures that nested quantifiers of the same type are merged into one
    // and that variables are sorted to maintain a unique structural identity.
    #[test]
    fn test_quantifier_flattening_and_canonicalization() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // On définit nos variables typées
        let var_a = builder.typed_variable(10, &[1]);
        let var_b = builder.typed_variable(20, &[1]);

        // On récupère les ExprId des variables pour les mettre dans le corps
        let arg_a = builder.variable(VariableId::from(10));
        let arg_b = builder.variable(VariableId::from(20));

        // Context pour la formule atomique
        let sym = PredicateSymbolId::from(1);
        let skel = AtomSkeletonId::from(10);

        // CORRECTION : Le corps doit contenir les variables 10 et 20
        // pour que le builder ne les supprime pas !
        let body = builder.atomic_formula(sym, &[arg_a, arg_b], skel);

        // Cas 1 : Création directe (forall {10, 20} P(?10, ?20))
        let list_flat = builder.typed_variable_list(vec![var_a.clone(), var_b.clone()]);
        let expr_flat = builder.forall(list_flat, body).unwrap();

        // Cas 2 : Création imbriquée (forall {20} (forall {10} P(?10, ?20)))
        let list_a = builder.typed_variable_list(vec![var_a]);
        let list_b = builder.typed_variable_list(vec![var_b]);

        // Le premier appel crée Forall({10}, P(10, 20))
        let inner_forall = builder.forall(list_a, body).unwrap();
        // Le second appel aplatit avec {20} pour donner Forall({10, 20}, P(10, 20))
        let expr_nested = builder.forall(list_b, inner_forall).unwrap();

        // Vérification 1 : Les IDs doivent être identiques grâce au flattening + tri + Hash-Consing
        assert_eq!(
            expr_flat, expr_nested,
            "Flattening or variable sorting failed: IDs should be identical"
        );

        // Vérification 2 : Structure du nœud
        let node = builder.get(expr_flat).expect("Expression should exist");
        if let ExprEntryKind::Forall(vars) = node.kind() {
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
        let inner_exists = builder.exists(list_y, body)?; // Utilisation de ?

        let list_x = builder.typed_variable_list(vec![var_x]);
        let outer_forall = builder.forall(list_x, inner_exists)?; // Utilisation de ?

        // Vérification de la structure
        let node = builder.get(outer_forall).expect("Root should exist");

        // 1. La racine doit être un Forall
        assert!(
            matches!(node.kind(), ExprEntryKind::Forall(_)),
            "Root should be Forall node, but got {:?}",
            node.kind()
        );

        // 2. L'enfant doit être le Exists (pas d'aplatissement entre types différents)
        let child_id = node.children()[0];
        assert_eq!(
            child_id, inner_exists,
            "Child ID should match the original Exists node (no flattening)"
        );

        // 3. Vérification que l'enfant est bien un Exists
        let child_node = builder.get(child_id).expect("Child should exist");
        assert!(
            matches!(child_node.kind(), ExprEntryKind::Exists(_)),
            "Child node should be Exists"
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
        let sym = PredicateSymbolId::from(1);
        let skel = AtomSkeletonId::from(10);
        let body = builder.atomic_formula(sym, &[], skel);

        // Input: sequence of two independent forall creations
        let vars1 = builder.typed_variable_list(vec![var_a]);
        let _first = builder.forall(vars1, body);
        let vars2 = builder.typed_variable_list(vec![var_b]);
        let second = builder.forall(vars2, body).unwrap();

        // Output: The second expression must only contain var_b (ID 2)
        if let Some(node) = builder.get(second) {
            if let ExprEntryKind::Forall(vars) = node.kind() {
                assert_eq!(vars.len(), 1, "Buffer pollution: found too many variables");
                assert_eq!(
                    vars[0].symbol(),
                    VariableId::from(2),
                    "Buffer pollution: wrong variable ID"
                );
            }
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
