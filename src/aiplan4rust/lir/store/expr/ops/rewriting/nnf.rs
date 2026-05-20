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

use crate::aiplan4rust::lir::store::expr::ops::error::ExprOpErrorHC;

use crate::aiplan4rust::lir::store::expr::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::expr::iter::scratchpad::Scratchpad;
use crate::aiplan4rust::lir::store::expr::{ExprEntryKind, ExprId};

/// Converts a logical expression to Negation Normal Form (NNF).
///
/// In NNF, all negations are pushed down to the literal level (atoms), and the only
/// allowed boolean operators are AND and OR. This implementation also handles quantified
/// expressions (Forall/Exists) by applying their respective dualities.
///
/// The conversion is performed iteratively using a Depth-First Search (DFS) strategy
/// facilitated by a [`Scratchpad`] to avoid deep recursion and minimize heap allocations.
///
/// # Polarity Cache Encoding
///
/// To optimize memoization within the scratchpad, structural IDs and their contextual
/// negations are bit-packed into a unified `usize` key:
/// * **Bits `1..61`**: Contain the raw structural index of the expression (`id.as_usize()`),
///   shifted left by 1 bit. This strips away any ephemeral system flags from the upper bits.
/// * **Bit `0` (LSB)**: Stores the negation polarity state (`0` for `negate = false`, `1` for `negate = true`).
///
/// If the input identifier matches `RAW_NONE`, the evaluation short-circuits immediately.
///
/// # Algorithm Phases
///
/// The transformation operates in three distinct logical phases within a single iterative loop:
/// 1. **Top-Down (Descent)**: Expressions are analyzed, and their children are written to
///    the arena buffer segment. The children are then pushed onto the DFS stack, inheriting
///    or flipping the accumulated negation polarity.
/// 2. **Memoization**: Each unique sub-expression combination is processed exactly once
///    per polarity context to efficiently handle Directed Acyclic Graph (DAG) structures.
/// 3. **Bottom-Up (Reconstruction)**: Once all dependent child structures are transformed,
///    the parent node is rebuilt using De Morgan's laws or quantifier dualities.
///
/// # Arguments
///
/// * `id` - The structural [`ExprId`] representing the root node to transform.
/// * `builder` - A mutable reference to the [`ExprBuilder`] used to intern newly generated nodes.
/// * `scratch` - A mutable reference to a reusable [`Scratchpad`] providing stable allocations
///   for stack operations, cached memoization, and localized child buffers.
///
/// # Returns
///
/// * `Ok(ExprId)` - The identifier pointing to the newly generated NNF expression.
/// * `Err(ExprOpErrorHC)` - An internal operational error if structural retrieval fails,
///   if a structural cache miss occurs ([`ExprOpErrorHC::CacheMiss`]), or if the root node
///   fails to reconstruct ([`ExprOpErrorHC::NnfLogicError`]).
pub fn to_nnf(
    id: ExprId,
    builder: &mut ExprBuilder,
    scratch: &mut Scratchpad,
) -> Result<ExprId, ExprOpErrorHC> {
    if id.is_none() {
        return Ok(id);
    }

    scratch.clear();
    let root_encoded = ExprId::new(encode(id.as_usize(), false));
    let mut final_id = None;

    scratch.push(root_encoded, false);

    while let Some((packed_id, processed)) = scratch.pop() {
        let (curr_id, negate) = decode(packed_id.value);

        if !curr_id.is_valid() {
            continue;
        }

        if !processed {
            if scratch.get(packed_id).is_some() {
                continue;
            }

            // --- PHASE 1: DESCENT (Top-Down) ---
            let (is_not, start, end) = {
                let entry = builder.fetch(curr_id)?;
                let (s, e) = scratch.prepare_children_segment(entry.children());
                (matches!(entry.kind(), ExprEntryKind::Not), s, e)
            };

            scratch.push(packed_id, true);

            if is_not {
                let child_id = scratch.children_buffer()[start];
                if child_id.is_valid() {
                    let child_packed = ExprId::new(encode(child_id.as_usize(), !negate));
                    scratch.push(child_packed, false);
                }
            } else {
                for i in (start..end).rev() {
                    let child_id = scratch.children_buffer()[i];
                    if child_id.is_valid() {
                        let child_packed = ExprId::new(encode(child_id.as_usize(), negate));
                        scratch.push(child_packed, false);
                    }
                }
            }
        } else {
            // --- PHASE 2: RECONSTRUCTION (Bottom-Up) ---
            let (kind, entry_child_count) = {
                let entry = builder.fetch(curr_id)?;
                (entry.kind().clone(), entry.children().len())
            };

            let (start, end) = scratch.last_segment_indices(entry_child_count);

            let new_id = match &kind {
                ExprEntryKind::Not => {
                    let child_id = scratch.children_buffer()[start];
                    let target_packed = ExprId::new(encode(child_id.as_usize(), !negate));

                    match scratch.get(target_packed) {
                        Some(res) => res,
                        None => {
                            let fallback_packed = ExprId::new(encode(child_id.as_usize(), negate));
                            scratch
                                .get(fallback_packed)
                                .ok_or_else(|| ExprOpErrorHC::cache_miss())?
                        }
                    }
                }

                ExprEntryKind::And | ExprEntryKind::Or => {
                    let is_and = matches!(kind, ExprEntryKind::And);

                    scratch.build_buffer_mut().clear();
                    for i in start..end {
                        let child_id = scratch.children_buffer()[i];
                        let target_packed = ExprId::new(encode(child_id.as_usize(), negate));

                        let transformed = match scratch.get(target_packed) {
                            Some(res) => res,
                            None => {
                                let fallback_packed =
                                    ExprId::new(encode(child_id.as_usize(), !negate));

                                scratch
                                    .get(fallback_packed)
                                    .ok_or_else(|| ExprOpErrorHC::cache_miss())?
                            }
                        };
                        scratch.build_buffer_mut().push(transformed);
                    }

                    if apply_de_morgan(is_and, negate) {
                        builder.and(scratch.build_buffer())
                    } else {
                        builder.or(scratch.build_buffer())
                    }
                }

                ExprEntryKind::Forall(vars) | ExprEntryKind::Exists(vars) => {
                    let is_forall = matches!(kind, ExprEntryKind::Forall(_));
                    let child_id = scratch.children_buffer()[start];
                    let target_packed = ExprId::new(encode(child_id.as_usize(), negate));

                    let body = match scratch.get(target_packed) {
                        Some(res) => res,
                        None => {
                            let fallback_packed = ExprId::new(encode(child_id.as_usize(), !negate));
                            scratch
                                .get(fallback_packed)
                                .ok_or_else(|| ExprOpErrorHC::cache_miss())?
                        }
                    };

                    if transform_quantifier(is_forall, negate) {
                        builder.forall(vars.clone(), body)?
                    } else {
                        builder.exists(vars.clone(), body)?
                    }
                }

                _ => {
                    let base = builder.intern(kind.clone(), &scratch.children_buffer()[start..end]);
                    if negate {
                        builder.not(base)
                    } else {
                        base
                    }
                }
            };

            scratch.children_buffer_mut().truncate(start);
            scratch.insert(packed_id, new_id);

            if packed_id == root_encoded {
                final_id = Some(new_id);
            }
        }
    }

    final_id.ok_or_else(|| ExprOpErrorHC::nnf_logic_error())
}

/// Determines the effective boolean operator (AND or OR) after applying a negation polarity.
///
/// This helper handles De Morgan's laws ($\neg(A \land B) \equiv \neg A \lor \neg B$ and
/// $\neg(A \lor B) \equiv \neg A \land \neg B$) using a branchless XOR-like equivalence.
///
/// # Truth Table
///
/// | `is_and` | `negate` | Output (`true` = AND, `false` = OR) | Semic-equivalent |
/// | :---:    | :---:    | :---:                               | :---             |
/// | `true`   | `false`  | `true`                              | $\land$ stays $\land$ |
/// | `true`   | `true`   | `false`                             | $\neg\land$ becomes $\lor$ |
/// | `false`  | `false`  | `false`                             | $\lor$ stays $\lor$ |
/// | `false`  | `true`   | `true`                              | $\neg\lor$ becomes $\land$ |
#[inline(always)]
fn apply_de_morgan(is_and: bool, negate: bool) -> bool {
    is_and == !negate
}

/// Determines the effective quantifier (Forall or Exists) after applying a negation polarity.
///
/// This helper handles first-order logic quantifier duality ($\neg\forall x. P(x) \equiv \exists x. \neg P(x)$
/// and $\neg\exists x. P(x) \equiv \forall x. \neg P(x)$) using a branchless XOR-like equivalence.
///
/// # Truth Table
///
/// | `is_forall` | `negate` | Output (`true` = Forall, `false` = Exists) | Semic-equivalent |
/// | :---:       | :---:    | :---:                                      | :---             |
/// | `true`      | `false`  | `true`                                     | $\forall$ stays $\forall$ |
/// | `true`      | `true`   | `false`                                    | $\neg\forall$ becomes $\exists$ |
/// | `false`     | `false`  | `false`                                    | $\exists$ stays $\exists$ |
/// | `false`     | `true`   | `true`                                     | $\neg\exists$ becomes $\forall$ |
#[inline(always)]
fn transform_quantifier(is_forall: bool, negate: bool) -> bool {
    is_forall == !negate
}

/// Encodes a structural `usize` index and its contextual negation polarity into a single `usize` key.
///
/// This encoding strips away any high-precision system flags from the original `ExprId` by
/// operating on a cleaned `raw_idx`, ensuring the key is dense and safe for memoization.
///
/// # Bit Representation
///
/// ```text
/// Bits 1..63: [ raw_idx (shifted left by 1) ]
/// Bit     0: [ Polarity Flag (0 = Positive, 1 = Negated) ]
/// ```
///
/// # Arguments
///
/// * `raw_idx` - The raw structural index extracted via `id.as_usize()`.
/// * `negate` - The accumulated negation polarity state.
#[inline(always)]
fn encode(raw_idx: usize, negate: bool) -> usize {
    let val = raw_idx << 1;
    if negate {
        val | 1
    } else {
        val
    }
}

/// Decodes a packed `usize` memoization key back into a clean [`ExprId`] and its logical polarity.
///
/// # Safety and Validation
///
/// The reconstructed identifier is instantiated using `ExprId::from`, which triggers the internal
/// boundary assertions of your structural store. This guarantees that corrupted or shifted
/// sentinel values (like an invalid `RAW_NONE` spillover) are caught immediately.
///
/// # Returns
///
/// A tuple containing:
/// 1. The structural [`ExprId`] cleaned of its packed polarity bit.
/// 2. A `bool` indicating the active negation polarity state (`true` if negated).
#[inline(always)]
fn decode(val: usize) -> (ExprId, bool) {
    (ExprId::from(val >> 1), (val & 1) == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::lang::{AtomSkeletonId, VariableId};
    use crate::aiplan4rust::lir::store::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::store::expr::ExprStore;

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
