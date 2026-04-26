//! Negation Normal Form (NNF) Transformation Module.
//!
//! This module provides high-performance utilities to transform logical expressions
//! into Negation Normal Form.
//!
//! # Overview
//! An expression is in NNF if:
//! * Negations (`NOT`) are only applied to atomic formulas (literals).
//! * The only allowed connectors are `AND` and `OR`.
//! * Existential and Universal quantifiers are handled via duality.
//!
//! # Architecture
//! The transformation uses a **non-recursive DFS** approach. This is critical for
//! processing deeply nested expressions (common in planning problems) without
//! triggering stack overflow.
//!
//! ### Key Components
//! * **[`to_nnf`]**: The main entry point for the transformation.
//! * **[`Scratchpad`]**: An externalized state buffer that enables zero-allocation
//!   traversal and memoization of DAG (Directed Acyclic Graph) nodes.
//! * **Hash-Consing**: This implementation relies on the [`ExprBuilder`] to ensure
//!   that identical sub-expressions generated during De Morgan expansion are
//!   automatically de-duplicated.
//!
//! # Performance Features
//! * **Arena-based Slicing**: Children of nodes are managed in a contiguous buffer
//!   to maximize CPU cache hits.
//! * **Polarity Encoding**: Uses bit-packing to store both positive and negative
//!   variants of a node in the same memoization table.
//! * **Inlined Logic**: Boolean dualities (De Morgan) are computed via branchless
//!   bitwise operations where possible.

use crate::aiplan4rust::lir::store::iter::Scratchpad;
use crate::aiplan4rust::lir::store::ops::error::ExprOpErrorHC;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

use crate::aiplan4rust::lir::store::builder::ExprBuilder;

/// Converts a logical expression to Negation Normal Form (NNF).
///
/// In NNF, all negations are pushed down to the literal level (atoms), and the only
/// allowed boolean operators are AND and OR. This implementation also handles
/// quantified expressions (Forall/Exists) by applying their respective dualities.
///
/// The conversion is performed iteratively using a Depth-First Search (DFS) strategy
/// facilitated by a [`Scratchpad`] to avoid recursion and minimize heap allocations.
///
/// # Logic
///
/// The algorithm operates in three distinct phases within a single loop:
/// 1. **Top-Down (Descent)**: Expressions are decomposed, and their children are pushed
///    onto the stack with the current negation state (polarity).
/// 2. **Memoization**: Each sub-expression is processed only once per polarity to
///    handle Directed Acyclic Graph (DAG) structures efficiently.
/// 3. **Bottom-Up (Reconstruction)**: Once children are transformed, the parent node
///    is reconstructed using De Morgan's laws or quantifier duality.
///
/// # Arguments
///
/// * `root_id` - The [`ExprId`] of the root node to transform.
/// * `builder` - A mutable reference to the [`ExprBuilder`] used to intern new nodes.
/// * `scratch` - A mutable reference to a [`Scratchpad`] providing reusable buffers
///   for stack operations, memoization, and child management.
///
/// # Returns
///
/// * `Ok(ExprId)` - The ID of the resulting expression in NNF.
/// * `Err(ExprOpErrorHC)` - An error if node fetching or construction fails.
///
/// # Memory Management
///
/// This function uses the `scratch.children_buffer()` as an arena-style stack. Each
/// non-processed node stores its children's IDs in a segment of the buffer. Upon
/// reconstruction (the `processed` phase), the segment is retrieved to look up
/// transformed children and then truncated to free space for other branches.
pub fn to_nnf(
    root_id: ExprId,
    builder: &mut ExprBuilder,
    scratch: &mut Scratchpad,
) -> Result<ExprId, ExprOpErrorHC> {
    scratch.clear();
    let root_encoded = ExprId::from(encode(root_id, false));
    scratch.push(root_encoded, false);

    while let Some((encoded_id, processed)) = scratch.pop() {
        let (curr_id, negate) = decode(encoded_id.as_usize());

        if !processed {
            // Check memoization cache to avoid redundant work in DAGs
            if scratch.get(encoded_id).is_some() {
                continue;
            }

            // --- PHASE 3 : DESCENT (Top-Down) ---
            // Isolate builder access to retrieve kind and children
            let (kind, start, end) = {
                let entry = builder.fetch(curr_id)?;
                let (s, e) = scratch.prepare_children_segment(entry.children());
                (entry.kind().clone(), s, e)
            };

            // Mark this node as "to be reconstructed" after its children
            scratch.push(encoded_id, true);

            if matches!(kind, ExprEntryKind::Not) {
                // For NOT nodes, we simply flip the negation state for the single child
                let child_id = scratch.children_buffer()[start];
                scratch.push(ExprId::from(encode(child_id, !negate)), false);
            } else {
                // Push children onto the stack to process them first
                // Reversed to maintain the original logical order
                for i in (start..end).rev() {
                    let child_id = scratch.children_buffer()[i];
                    scratch.push(ExprId::from(encode(child_id, negate)), false);
                }
            }
        } else {
            // --- PHASE 2 : RECONSTRUCTION (Bottom-Up) ---
            let (kind, entry_child_count) = {
                let entry = builder.fetch(curr_id)?;
                (entry.kind().clone(), entry.children().len())
            };

            // Identify the segment in the children arena corresponding to this node
            let (start, end) = scratch.last_segment_indices(entry_child_count);

            let new_id = match &kind {
                ExprEntryKind::Not => {
                    // Logic for double negation or pushing negation further down
                    let child_id = scratch.children_buffer()[start];
                    scratch.fetch(ExprId::from(encode(child_id, !negate)))
                }

                ExprEntryKind::And | ExprEntryKind::Or => {
                    let is_and = matches!(kind, ExprEntryKind::And);

                    // Collect already transformed children from the scratchpad cache
                    scratch.build_buffer_mut().clear();
                    for i in start..end {
                        let c = scratch.children_buffer()[i];
                        let transformed = scratch.fetch(ExprId::from(encode(c, negate)));
                        scratch.build_buffer_mut().push(transformed);
                    }

                    // Apply De Morgan's laws to decide the new operator
                    if apply_de_morgan(is_and, negate) {
                        builder.and(scratch.build_buffer())
                    } else {
                        builder.or(scratch.build_buffer())
                    }
                }

                ExprEntryKind::Forall(vars) | ExprEntryKind::Exists(vars) => {
                    let is_forall = matches!(kind, ExprEntryKind::Forall(_));
                    let child_id = scratch.children_buffer()[start];
                    let body = scratch.fetch(ExprId::from(encode(child_id, negate)));

                    // Apply Quantifier Duality (¬∀ -> ∃, ¬∃ -> ∀)
                    if transform_quantifier(is_forall, negate) {
                        builder.forall(vars.clone(), body)?
                    } else {
                        builder.exists(vars.clone(), body)?
                    }
                }

                _ => {
                    // Terminal nodes (Atoms, Fluents, etc.)
                    // Re-intern the kind, applying a NOT if the cumulative negation is odd
                    let base = builder.intern(kind.clone(), &scratch.children_buffer()[start..end]);
                    if negate {
                        builder.not(base)
                    } else {
                        base
                    }
                }
            };

            // Clean up the children arena for this depth and memoize result
            scratch.children_buffer_mut().truncate(start);
            scratch.insert(encoded_id, new_id);
        }
    }

    // The root result is now stored in the scratchpad cache
    Ok(scratch.fetch(root_encoded))
}

/// Determines the effective boolean operator (AND or OR) after applying a negation.
///
/// This helper implements De Morgan's laws by deciding whether the resulting node
/// should be a conjunction or a disjunction based on the current negation state.
///
/// # Arguments
///
/// * `is_and` - A boolean indicating if the original operator is a conjunction (AND).
/// * `negate` - A boolean indicating if a negation is being pushed down from a parent.
///
/// # Returns
///
/// * `true` if the resulting operator is an **AND**.
/// * `false` if the resulting operator is an **OR**.
#[inline(always)]
fn apply_de_morgan(is_and: bool, negate: bool) -> bool {
    // If negate is false, keep the original operator.
    // If negate is true, flip it (AND becomes OR, OR becomes AND).
    // Logic: is_and NXOR negate
    is_and == !negate
}

/// Determines the effective quantifier (Forall or Exists) after applying a negation.
///
/// Follows the equivalence: ¬∀x.P ≡ ∃x.¬P and ¬∃x.P ≡ ∀x.¬P.
///
/// # Arguments
///
/// * `is_forall` - `true` if the original node is a `Forall` quantifier.
/// * `negate` - `true` if a negation is being pushed down.
///
/// # Returns
///
/// * `true` if the resulting quantifier should be a **Forall**.
/// * `false` if the resulting quantifier should be an **Exists**.
#[inline(always)]
fn transform_quantifier(is_forall: bool, negate: bool) -> bool {
    is_forall == !negate
}

/// Encodes an [`ExprId`] and its negation polarity into a single `usize`.
///
/// This is used for memoization in the scratchpad, allowing it to store
/// both the positive and negative versions of a transformed sub-expression.
///
/// # Arguments
///
/// * `id` - The unique identifier of the expression.
/// * `negate` - The current negation state (polarity).
///
/// # Returns
///
/// An encoded `usize` where:
/// * Even values represent positive polarity (`negate = false`).
/// * Odd values represent negative polarity (`negate = true`).
#[inline(always)]
fn encode(id: ExprId, negate: bool) -> usize {
    let val = id.as_usize() << 1;
    if negate {
        val | 1
    } else {
        val
    }
}

/// Decodes a raw `usize` value retrieved from the scratchpad into an [`ExprId`] and its polarity.
///
/// # Arguments
///
/// * `val` - The encoded value previously generated by [`encode`].
///
/// # Returns
///
/// A tuple containing:
/// * The original [`ExprId`].
/// * A boolean indicating if the expression is negated (`true` if odd).
#[inline(always)]
fn decode(val: usize) -> (ExprId, bool) {
    (ExprId::from(val >> 1), (val & 1) == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::{AtomSkeletonId, VariableId};
    use crate::aiplan4rust::lir::store::builder::ExprBuilder;
    use crate::aiplan4rust::lir::store::iter::Scratchpad;
    use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprStore};

    /// Test pushing negation through AND using De Morgan's law.
    /// Input: (not (and (A) (B))) -> (or (not (A)) (not (B)))
    #[test]
    fn test_push_negation_and() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Setup: ¬(A ∧ B)
        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);
        let and_node = builder.and(&[a, b]);
        let root = builder.not(and_node);

        // 2. Transformation: De Morgan's Law ¬(A ∧ B) -> (¬A ∨ ¬B)
        let result_id = to_nnf(root, &mut builder, &mut scratch)?;
        let root_node = builder.fetch(result_id)?;

        // 3. Validation
        // On attend un OR à la racine
        assert!(matches!(root_node.kind(), ExprEntryKind::Or));
        let children = root_node.children();
        assert_eq!(children.len(), 2);

        // Vérification robuste : chaque enfant doit être un NOT
        // et les feuilles doivent être nos atomes d'origine
        let mut found_not_a = false;
        let mut found_not_b = false;

        for &child_id in children {
            let child_node = builder.fetch(child_id)?;
            assert!(matches!(child_node.kind(), ExprEntryKind::Not));

            let leaf_id = child_node.children()[0];
            if leaf_id == a {
                found_not_a = true;
            } else if leaf_id == b {
                found_not_b = true;
            }
        }

        assert!(found_not_a, "Not(A) est manquant dans le résultat");
        assert!(found_not_b, "Not(B) est manquant dans le résultat");

        Ok(())
    }

    /// Test pushing negation through OR using De Morgan's law.
    /// Input: (not (or (A) (B))) -> (and (not (A)) (not (B)))
    #[test]
    fn test_push_negation_or() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Setup: ¬(A ∨ B)
        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);
        let or_node = builder.or(&[a, b]);
        let root = builder.not(or_node);

        // 2. Transformation: De Morgan's Law ¬(A ∨ B) -> (¬A ∧ ¬B)
        let result_id = to_nnf(root, &mut builder, &mut scratch)?;
        let root_node = builder.fetch(result_id)?;

        // 3. Validation
        // La racine doit être un AND
        assert!(matches!(root_node.kind(), ExprEntryKind::And));
        let children = root_node.children();
        assert_eq!(children.len(), 2);

        // Vérification robuste (le Store peut trier les IDs)
        let mut found_not_a = false;
        let mut found_not_b = false;

        for &child_id in children {
            let child_node = builder.fetch(child_id)?;
            assert!(matches!(child_node.kind(), ExprEntryKind::Not));

            let leaf_id = child_node.children()[0];
            if leaf_id == a {
                found_not_a = true;
            } else if leaf_id == b {
                found_not_b = true;
            }
        }

        assert!(found_not_a, "Not(A) est manquant dans la conjonction");
        assert!(found_not_b, "Not(B) est manquant dans la conjonction");

        Ok(())
    }

    /// Test pushing negation through a Forall quantifier.
    /// Input: (not (forall x (A(x)))) -> (exists x (not (A(x))))
    #[test]
    fn test_push_negation_forall() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Setup: ¬(forall (?X) (A(?X)))
        let var_id = VariableId::from(10);
        let var_x = builder.typed_variable(10, &[100]); // ID 10, Type 100
        let forall_vars = builder.typed_variable_list(vec![var_x]);

        // FIX: Create a variable argument and use it in the atomic formula
        // to prevent the builder from pruning the "unused" quantifier.
        let arg_x = builder.variable(var_id);
        let a = builder.atomic_formula(1, &[arg_x], skel);

        let forall_node = builder.forall(forall_vars, a)?;
        let root = builder.not(forall_node);

        // 2. Transformation: ¬∀x.A -> ∃x.¬A
        let result_id = to_nnf(root, &mut builder, &mut scratch)?;
        let root_node = builder.fetch(result_id)?;

        // 3. Validation
        // The Forall under negation must have become an Exists
        assert!(
            matches!(root_node.kind(), ExprEntryKind::Exists(_)),
            "Expected Exists node, but the quantifier was likely pruned. Got: {:?}",
            root_node.kind()
        );

        // Verify that variables are preserved
        if let ExprEntryKind::Exists(ref vars) = root_node.kind() {
            assert_eq!(vars.len(), 1);
            assert_eq!(vars[0].symbol(), var_id);
        }

        // The body of the Exists must be ¬A
        let body_id = root_node.children()[0];
        let body_node = builder.fetch(body_id)?;
        assert!(matches!(body_node.kind(), ExprEntryKind::Not));

        let inner_atom_id = body_node.children()[0];
        assert_eq!(inner_atom_id, a, "The atom inside the negation was altered");

        Ok(())
    }

    /// Test pushing negation through an Exists quantifier.
    /// Input: (not (exists x (A(x)))) -> (forall x (not (A(x))))
    #[test]
    fn test_push_negation_exists() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Setup: ¬(exists (?X) (A(?X)))
        let var_id = VariableId::from(10);
        let var_x = builder.typed_variable(10, &[100]);
        let exists_vars = builder.typed_variable_list(vec![var_x]);

        // FIX: Link the variable to the atomic formula to prevent pruning
        let arg_x = builder.variable(var_id);
        let a = builder.atomic_formula(1, &[arg_x], skel);

        let exists_node = builder.exists(exists_vars, a)?;
        let root = builder.not(exists_node);

        // 2. Transformation: ¬∃x.A -> ∀x.¬A
        let result_id = to_nnf(root, &mut builder, &mut scratch)?;
        let root_node = builder.fetch(result_id)?;

        // 3. Validation
        // The Exists under negation must have become a Forall
        assert!(
            matches!(root_node.kind(), ExprEntryKind::Forall(_)),
            "Expected Forall node, but it was likely pruned because the variable was unused. Got: {:?}",
            root_node.kind()
        );

        // Verify the body: ¬A
        let body_id = root_node.children()[0];
        let body_node = builder.fetch(body_id)?;
        assert!(matches!(body_node.kind(), ExprEntryKind::Not));

        let inner_atom_id = body_node.children()[0];
        assert_eq!(inner_atom_id, a, "The inner atom was lost or modified");

        Ok(())
    }

    /// Test that no transformation occurs for a NOT whose child is an atomic formula.
    /// Input: (not (A)) -> (not (A)) (unchanged)
    #[test]
    fn test_push_negation_no_change() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Setup: ¬A (Atome déjà négatif)
        let a = builder.atomic_formula(1, &[], skel);
        let root = builder.not(a);

        // 2. Transformation: Aucun changement structurel attendu
        let result_id = to_nnf(root, &mut builder, &mut scratch)?;
        let root_node = builder.fetch(result_id)?;

        // 3. Validation
        // Le Kind doit rester Not
        assert!(matches!(root_node.kind(), ExprEntryKind::Not));

        // L'enfant doit toujours être l'atome d'origine
        let child_id = root_node.children()[0];
        assert_eq!(child_id, a, "L'atome interne ne devrait pas être modifié");

        Ok(())
    }

    /// Test pushing negation through a nested expression.
    /// Input: ¬(A ∧ ¬B ∧ ∃x.C)
    /// Expected: (¬A ∨ B ∨ ∀x.¬C)  <-- Note que ¬¬B est devenu B !
    /// Test pushing negation through a nested expression.
    /// Input: ¬(A ∧ ¬B ∧ ∃x.C(x))
    /// Expected: (¬A ∨ B ∨ ∀x.¬C(x))  <-- Note that ¬¬B becomes B!
    #[test]
    fn test_push_negation_nested() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Setup: ¬(A ∧ ¬B ∧ ∃x.C(x))
        let var_id = VariableId::from(10);
        let arg_x = builder.variable(var_id);

        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);
        // FIX: Include arg_x in C to prevent the builder from pruning the quantifier
        let c = builder.atomic_formula(3, &[arg_x], skel);

        let not_b = builder.not(b);
        let var_x = builder.typed_variable(10, &[100]);
        let exists_vars = builder.typed_variable_list(vec![var_x]);
        let exists_c = builder.exists(exists_vars, c)?;

        let and_node = builder.and(&[a, not_b, exists_c]);
        let root = builder.not(and_node);

        // 2. Transformation
        // Internal push_negation logic transforms:
        // ¬(A ∧ ¬B ∧ ∃x.C)  =>  (¬A ∨ B ∨ ∀x.¬C)
        let result_id = to_nnf(root, &mut builder, &mut scratch)?;
        let root_node = builder.fetch(result_id)?;

        // 3. Validation
        assert!(
            matches!(root_node.kind(), ExprEntryKind::Or),
            "Root must be an OR node"
        );
        let children = root_node.children();
        assert_eq!(
            children.len(),
            3,
            "Expected 3 children after De Morgan expansion"
        );

        let mut found_not_a = false;
        let mut found_b_simplified = false;
        let mut found_forall_not_c = false;

        for &child_id in children {
            // Direct comparison by ID (highly efficient due to Hash-Consing)
            if child_id == b {
                found_b_simplified = true;
                continue;
            }

            let node = builder.fetch(child_id)?;
            match node.kind() {
                // Case ¬A
                ExprEntryKind::Not if node.children()[0] == a => {
                    found_not_a = true;
                }

                // Case ∀x.¬C
                ExprEntryKind::Forall(_) => {
                    let body_id = node.children()[0];
                    let body_node = builder.fetch(body_id)?;
                    // Verify that the Forall body is Not(C)
                    if matches!(body_node.kind(), ExprEntryKind::Not)
                        && body_node.children()[0] == c
                    {
                        found_forall_not_c = true;
                    }
                }
                _ => {}
            }
        }

        assert!(found_not_a, "¬A is missing or malformed");
        assert!(
            found_b_simplified,
            "B should have been simplified (¬¬B -> B) and identified by its ID"
        );
        assert!(found_forall_not_c, "∀x.¬C is missing or malformed");

        Ok(())
    }

    /// Test pushing negation through deep nested structures.
    ///
    /// Input: ¬(A ∧ ¬(B ∨ C) ∧ ∀x.∃y.D(x, y))
    /// Expected: (¬A ∨ (B ∨ C) ∨ ∃x.∀y.¬D(x, y))
    ///
    /// Note: The atomic formula D must explicitly use both variables x and y
    /// as arguments. Otherwise, the ExprBuilder's optimization would detect
    /// vacuous quantification and prune the Forall/Exists nodes.
    #[test]
    fn test_push_negation_deep_nested() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        // 1. Define Variable IDs and TypedVariables
        let var_x_id = VariableId::from(10);
        let var_y_id = VariableId::from(11);

        let var_x = builder.typed_variable(10, &[1]);
        let var_y = builder.typed_variable(11, &[1]);

        let vars_x = builder.typed_variable_list(vec![var_x]);
        let vars_y = builder.typed_variable_list(vec![var_y]);

        // 2. Define Atomic Formulas using the variables
        let arg_x = builder.variable(var_x_id);
        let arg_y = builder.variable(var_y_id);

        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);
        let c = builder.atomic_formula(3, &[], skel);

        // FIX: Include BOTH arg_x and arg_y here.
        // This prevents the builder from pruning 'forall x' later.
        let d = builder.atomic_formula(4, &[arg_x, arg_y], skel);

        // 3. Construct the nested structure: ¬(A ∧ ¬(B ∨ C) ∧ ∀x.∃y.D(x,y))
        let or_bc = builder.or(&[b, c]);
        let not_or_bc = builder.not(or_bc);

        // (exists y. D(x, y))
        let exists_d = builder.exists(vars_y, d)?;
        // (forall x. (exists y. D(x, y))) -> x is now "free" in the body, so it stays!
        let forall_exists_d = builder.forall(vars_x, exists_d)?;

        let and_node = builder.and(&[a, not_or_bc, forall_exists_d]);
        let root_id = builder.not(and_node);

        // 4. Perform NNF Transformation
        let result_id = to_nnf(root_id, &mut builder, &mut scratch)?;

        // 5. Validation phase
        let children: Vec<ExprId> = {
            let node = builder.fetch(result_id)?;
            assert!(
                matches!(node.kind(), ExprEntryKind::Or),
                "Root must be an OR node"
            );
            node.children().to_vec()
        };

        // Expected: ¬A ∨ (B ∨ C) ∨ ∃x.∀y.¬D(x,y)
        assert_eq!(
            children.len(),
            4,
            "Should have 4 children (¬A, B, C, ∃x.∀y.¬D)"
        );

        // 6. Verify simple members
        let not_a = builder.not(a);
        assert!(children.contains(&not_a), "¬A is missing");
        assert!(children.contains(&b), "B is missing");
        assert!(children.contains(&c), "C is missing");

        // 7. Verify the inverted quantifier exists in the children
        // Now this will pass because 'x' was not pruned during construction.
        let has_exists = children.iter().any(|&id| {
            let node = builder.get(id).unwrap();
            matches!(node.kind(), ExprEntryKind::Exists(_))
        });

        assert!(
            has_exists,
            "The inverted quantified branch (Exists x) must be present"
        );

        Ok(())
    }

    /// Vérifie le partage de structure (DAG) et l'efficacité du Hash-Consing.
    ///
    /// Ce test s'assure que :
    /// 1. L'algorithme `push_negation` ne duplique pas le travail : si une sous-expression
    ///    identique apparaît plusieurs fois, elle doit être représentée par le même `ExprId`.
    /// 2. Le Store réutilise les nœuds déjà existants : transformer une expression puis
    ///    la transformer à nouveau séparément doit retourner le même identifiant unique.
    ///
    /// Logique du test :
    /// - On construit ¬((A ∧ B) ∨ (A ∧ C)).
    /// - Après application de De Morgan, on attend (¬(A ∧ B) ∧ ¬(A ∧ C)).
    /// - On vérifie que l'ID du composant ¬(A ∧ B) au sein du résultat global est
    ///   strictement identique à l'ID obtenu en transformant ¬(A ∧ B) de manière isolée.
    #[test]
    fn test_push_negation_dag_sharing() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);
        let c = builder.atomic_formula(3, &[], skel);

        // (A ∧ B)
        let and_ab = builder.and(&[a, b]);
        // (A ∧ B) ∨ (A ∧ C) -- On utilise deux branches différentes pour éviter la simplification X v X
        let and_ac = builder.and(&[a, c]);
        let or_node = builder.or(&[and_ab, and_ac]);

        // ROOT: ¬((A ∧ B) ∨ (A ∧ C))
        let root = builder.not(or_node);

        let result_id = to_nnf(root, &mut builder, &mut scratch)?;

        // On calcule manuellement ¬(A ∧ B) pour vérifier le partage
        let not_and_ab = builder.not(and_ab);
        let expected_part_id = to_nnf(not_and_ab, &mut builder, &mut scratch)?;

        let root_node = builder.fetch(result_id)?;

        // Validation
        // 1. La racine est bien un AND (De Morgan sur le OR)
        assert!(
            matches!(root_node.kind(), ExprEntryKind::And),
            "Doit être un AND"
        );

        // 2. L'un des enfants du résultat doit être EXACTEMENT l'ID de la transformation de la branche AB
        let children = root_node.children();
        assert!(
            children.contains(&expected_part_id),
            "Le store doit partager l'ID de la sous-expression transformée"
        );

        Ok(())
    }
    /// Test d'une triple négation.
    /// ¬¬¬A -> ¬A
    #[test]
    fn test_push_negation_triple() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let a = builder.atomic_formula(1, &[], skel);

        // Décomposition de ¬¬¬A
        let n1 = builder.not(a);
        let n2 = builder.not(n1);
        let not_3_a = builder.not(n2);

        let result_id = to_nnf(not_3_a, &mut builder, &mut scratch)?;
        let root_node = builder.fetch(result_id)?;

        // Le résultat doit être simplement ¬A (ID de n1)
        assert!(matches!(root_node.kind(), ExprEntryKind::Not));
        assert_eq!(root_node.children()[0], a);
        assert_eq!(
            result_id, n1,
            "La triple négation doit être réduite à une seule"
        );
        Ok(())
    }

    /// Test de stabilité (Idempotence de la fonction).
    /// push_negation(push_negation(X)) == push_negation(X)
    #[test]
    fn test_push_negation_idempotency() -> Result<(), Box<dyn std::error::Error>> {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);
        let mut scratch = Scratchpad::new();
        let skel = AtomSkeletonId::from(0);

        let a = builder.atomic_formula(1, &[], skel);
        let b = builder.atomic_formula(2, &[], skel);

        // ¬(A ∧ B)
        let and_node = builder.and(&[a, b]);
        let root = builder.not(and_node);

        let first_pass = to_nnf(root, &mut builder, &mut scratch)?;
        let second_pass = to_nnf(first_pass, &mut builder, &mut scratch)?;

        assert_eq!(
            first_pass, second_pass,
            "Appliquer push_negation deux fois ne doit rien changer"
        );
        Ok(())
    }
}
