//! Datalog Auxiliary Predicate Encoder and Atom Extraction Engine.
//!
//! This module provides high-performance utilities to transform complex logical formulas
//! (such as nested `AND` or `OR` expressions) into normalized Datalog Horn clauses. It operates
//! as a critical lowering phase between the high-level Abstract Syntax Tree (AST) representation
//! and the optimized flattened Datalog engine.
//!
//! # Architecture & Design Goals
//!
//! The design of this module is guided by three core principles:
//!
//! 1. **Zero Heap Allocation (Stack-First Pipeline):** By utilizing specialized continuous inline
//!    buffers ([`SmallVec`]) and localized bitsets, all core operations—such as variable tracking,
//!    deduplication, and safety guard synthesis—are executed entirely on the stack for typical
//!    logical scopes (e.g., arities $\le 4$).
//! 2. **Bitmask-Driven Variable Analysis:** Variable discovery, occurrence verification, and
//!    bound/free tracking are compressed into CPU-register-sized bitmasks (`settings::VariableMask`).
//!    This turns complex collection scans into fast, deterministic, instruction-level bitwise operations.
//! 3. **Rule Safety Enforcement (Datalog Stratification and Safety):** Implements a robust "Rule Repair"
//!    algorithm that automatically detects unsafe free variables (e.g., variables appearing exclusively
//!    in negative literals) and injects positive unary type guards to ensure declarative validity.
//!
//! # Main Core Components
//!
//! * **[`allocate_auxiliary_predicate`]:** The primary orchestrator. Generates a unique signature, registers
//!   the structural skeleton, and builds a safe, stack-allocated rule body.
//! * **[`extract_atom`]:** Lowers high-level expression tree nodes (`ExprNode`) into structural, groundable
//!   Datalog [`Atom`]s.
//! * **[`AuxPredicateBody`]:** A performance-tuned container designed to completely bypass heap allocation
//!   overhead for standard-sized auxiliary bodies.
//!
//! # Mathematical Safety Guarantee
//!
//! For every free variable $v$ belonging to a generated auxiliary clause body $B$:
//!
//! $$v \in \text{Variables}(B) \implies \exists A \in B \text{ s.t. } A \text{ is a positive literal and } v \in \text{Variables}(A)$$
//!
//! If this condition is violated, the module automatically lookups the type representation of $v$ from the parent
//! scope arguments and appends a positive unary type predicate $T(v)$ to enforce safety.

use crate::aiplan4rust::compiler::lir::expr::{ExprKind, ExprNode, ExprStore};
use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::support::lang::{
    AtomSkeletonId, PredicateSymbolId, TypedList, TypedListId, TypedSymbol, VariableId,
};
use crate::analysis::reachability::datalog::core::atom::AtomArgs;
use crate::analysis::reachability::datalog::core::{Atom, Term};
use crate::analysis::reachability::datalog::encoder::aliasing;
use crate::analysis::reachability::datalog::error::DatalogError;
use crate::analysis::reachability::datalog::settings;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

/// Represents the continuous sequence of atoms forming the body of an auxiliary rule.
///
/// This type uses a [`SmallVec`] inline buffer to optimize stack allocation and bypass
/// heap allocations during complex formula transformations (e.g., AND/OR to Horn clauses),
/// provided the combined atom count remains within `settings::INLINE_AUX_PREDICATE_CAPACITY`.
pub type AuxPredicateBody = SmallVec<[Atom; settings::INLINE_AUX_PREDICATE_CAPACITY]>;

/// Encodes a new auxiliary predicate based on a collection of atoms.
///
/// This is a helper function used during the transformation of complex formulas
/// (such as AND/OR) into Horn clauses. It performs three main steps:
/// 1. Collects and resolves unique variables from the provided `atoms` to determine
///    the signature (arity and types) of the new predicate.
/// 2. Performs a bitmask-driven safety repair by appending unary type atoms to the body
///    for any free variables that are not covered by the body atoms. This step utilizes
///    `SmallVec` to optimize stack allocation and completely avoid heap allocations
///    when the capacity is within bounds.
/// 3. Allocates a new unique predicate ID and returns the corresponding head [`Atom`]
///    along with the secured body.
///
/// # Arguments
/// * `atoms` - The list of atoms that will form the body (for AND) or the options (for OR)
///             of the rules associated with this auxiliary predicate.
/// * `arguments` - The identifier of the typed list of arguments from the current scope.
/// * `next_aux_id` - A mutable reference to the generator for unique auxiliary predicate IDs.
/// * `aux_defs` - A mutable reference to the vector storing the definitions of auxiliary formulas.
/// * `type_to_skeleton` - A slice mapping type IDs to their corresponding atom skeleton IDs.
/// * `current_aliases` - A map containing variable aliases and substitutions active in the current scope.
/// * `store` - A mutable reference to the expression store for data lookups and interning.
///
/// # Errors
/// Returns a [`DatalogError`] if variable collection fails, if an alias resolution error occurs,
/// or if there is an inconsistency within the retrieved typed arguments list.
pub(crate) fn allocate_auxiliary_predicate(
    atoms: &[Atom],
    arguments: TypedListId,
    next_aux_id: &mut usize,
    aux_defs: &mut Vec<AtomicFormulaSkeleton>,
    type_to_skeleton: &[AtomSkeletonId],
    current_aliases: &FxHashMap<VariableId, Term>,
    store: &mut ExprStore,
) -> Result<(Atom, AuxPredicateBody), DatalogError> {
    // 1. Collect and directly resolve variables using a u64 bitmask
    let mask = compute_variable_bitmask(atoms, current_aliases)?;
    let mut resolved_terms = decode_bitmask_to_resolved_terms(mask, current_aliases);

    // Sorting and deduplication are only required if aliases introduced constants
    resolved_terms.sort();
    resolved_terms.dedup();

    // --- 2. BITMASK-BASED SAFETY REPAIR (Optimized SmallVec + Unary Atom Insertion) ---
    let mut covered_mask = compute_covered_variable_bitmask(atoms)?;

    // Initialize a SmallVec directly instead of a standard Vec.
    // If (atoms.len() + resolved_terms.len()) <= settings::INLINE_BODY_CAPACITY,
    // heap allocation is completely avoided.
    let mut secured_body = AuxPredicateBody::with_capacity(atoms.len() + resolved_terms.len());
    secured_body.extend(atoms.iter().cloned());

    let args_list = store.fetch_typed_list(arguments)?;

    for term in &resolved_terms {
        if let Term::Variable(v_id) = term {
            let bit_projected = 1 << v_id.as_usize();
            if (covered_mask & bit_projected) == 0 {
                let type_id = args_list[v_id.as_usize()].ty().members()[0].as_usize();
                let type_sk = type_to_skeleton[type_id];

                // OPTIMIZATION: The unary atom stays on the stack and is pushed directly
                // into the contiguous memory space of the SmallVec (remaining on the stack if within capacity).
                secured_body.push(Atom::unary(type_sk, Term::Variable(*v_id)));

                covered_mask |= bit_projected;
            }
        }
    }

    // 3. Generate the head atom signature
    let head =
        intern_auxiliary_signature(&resolved_terms, arguments, next_aux_id, aux_defs, store)?;

    Ok((head, secured_body))
}

/// Extracts a logical [`Atom`] from a specific expression node.
///
/// This function serves as the bridge between the high-level expression tree and the flattened Datalog
/// representation. It resolves the predicate identity and maps each child argument to its concrete [`Term`].
///
/// # Arguments
/// * `node` - A reference to the current [`ExprNode`], which must represent an `AtomicFormula` or a `Comparison`.
/// * `store` - A reference to the expression store used to resolve the nature and kinds of child nodes.
///
/// # Errors
/// Returns a [`DatalogError`] if the node kind is incompatible, if any child argument resolution fails,
/// or if an invalid argument type is encountered.
pub(crate) fn extract_atom(node: ExprNode<'_>, store: &ExprStore) -> Result<Atom, DatalogError> {
    let kind = node.kind();
    let children = node.children();

    // 1. Fast determination of the Skeleton ID and the child skip offset.
    let (skeleton_id, skip_count) = match kind {
        ExprKind::Comparison(_) => (AtomSkeletonId::from(Atom::EQUALITY_ID), 0),
        ExprKind::AtomicFormula(sk_id) => (*sk_id, 1),
        _ => {
            return Err(DatalogError::incompatible_node(kind.clone(), node.id()));
        }
    };

    // 2. Exact allocation using SmallVec to prevent heap allocations for arity <= 4.
    let capacity = children.len().saturating_sub(skip_count);
    let mut terms = AtomArgs::with_capacity(capacity);

    // 3. Optimized term collection.
    for &arg_id in children.iter().skip(skip_count) {
        let arg_node = store.fetch(arg_id)?;

        let term = match arg_node.kind() {
            ExprKind::Variable(v_id) => Term::Variable(*v_id),
            ExprKind::Object(obj_id) => Term::Constant(*obj_id),
            _ => {
                return Err(DatalogError::invalid_atom_argument(arg_id));
            }
        };
        terms.push(term);
    }

    // OPTIMIZATION: Utilizes the n-ary constructor directly on the stack to bypass heap overhead.
    Ok(Atom::nary(skeleton_id, terms))
}

/// Computes a compressed bitmask representing all unique variables present in a collection of atoms.
///
/// This function iterates through the arguments of each provided atom, resolves any variable aliases
/// active within the current scope, and encodes the resulting variable indices into a dense bitmask.
///
/// # Arguments
/// * `atoms` - A slice of atoms whose arguments will be inspected.
/// * `current_aliases` - A map containing variable-to-term substitutions used to resolve active aliases.
///
/// # Errors
/// Returns a [`DatalogError::VariableLimitExceeded`] if a resolved variable index equals or exceeds
/// the compile-time limit specified by `settings::MAX_VARIABLES_PER_SCOPE`.
fn compute_variable_bitmask(
    atoms: &[Atom],
    current_aliases: &FxHashMap<VariableId, Term>,
) -> Result<settings::VariableMask, DatalogError> {
    let mut mask: settings::VariableMask = 0;

    for atom in atoms {
        for term in atom.arguments() {
            let resolved_term = match term {
                Term::Variable(v) => aliasing::resolve_var(*v, current_aliases),
                Term::Constant(_) => term.clone(),
            };

            if let Term::Variable(v) = resolved_term {
                let v_idx = v.as_usize();

                if v_idx >= settings::MAX_VARIABLES_PER_SCOPE {
                    return Err(DatalogError::variable_limit_exceeded(
                        v,
                        settings::MAX_VARIABLES_PER_SCOPE,
                    ));
                }
                mask |= 1 << v_idx;
            }
        }
    }
    Ok(mask)
}

/// Decodes a variable bitmask back into a collection of resolved terms.
///
/// This function iteratively extracts the active variable indices from the dense bitmask,
/// maps them back to their corresponding `VariableId`, resolves any active aliases
/// within the current scope, and aggregates them into an `AtomArgs` collection.
///
/// # Arguments
/// * `mask` - The compressed bitmask containing the variable indices to decode.
/// * `current_aliases` - A map containing variable-to-term substitutions used to resolve active aliases.
fn decode_bitmask_to_resolved_terms(
    mut mask: settings::VariableMask,
    current_aliases: &FxHashMap<VariableId, Term>,
) -> AtomArgs {
    let mut terms = AtomArgs::with_capacity(mask.count_ones() as usize);
    while mask != 0 {
        let bit = mask.trailing_zeros();
        let v_id = VariableId::from(bit as usize);
        terms.push(aliasing::resolve_var(v_id, current_aliases));
        mask &= mask - 1; // Clear the lowest set bit
    }
    terms
}

/// Computes a compressed bitmask representing all variables covered by at least one non-negated atom.
///
/// This is a pure, deterministic function executing entirely on the stack. It isolates positive
/// atoms to identify bound variables, which is critical for determining safety requirements
/// in Horn clause conversions.
///
/// # Arguments
/// * `atoms` - A slice of atoms whose arguments will be inspected for variable coverage.
///
/// # Errors
/// Returns a [`DatalogError::VariableLimitExceeded`] if a discovered variable index equals or exceeds
/// the compile-time limit specified by `settings::MAX_VARIABLES_PER_SCOPE`.
fn compute_covered_variable_bitmask(
    atoms: &[Atom],
) -> Result<settings::VariableMask, DatalogError> {
    let mut mask: settings::VariableMask = 0;
    for atom in atoms {
        if !atom.is_negated() {
            for term in atom.arguments() {
                if let Term::Variable(v) = term {
                    let v_idx = v.as_usize();

                    if v_idx >= settings::MAX_VARIABLES_PER_SCOPE {
                        return Err(DatalogError::variable_limit_exceeded(
                            *v,
                            settings::MAX_VARIABLES_PER_SCOPE,
                        ));
                    }
                    mask |= 1 << v_idx;
                }
            }
        }
    }
    Ok(mask)
}

/// Generates and registers a new unique auxiliary predicate signature based on resolved terms.
///
/// This function allocates a new unique predicate identifier, extracts the required type information
/// from the parent scope's argument list to construct a new `TypedList`, and registers the auxiliary
/// definition into `aux_defs`.
///
/// # Arguments
/// * `resolved_terms` - A slice of terms that dictate the arguments and arity of the auxiliary predicate.
/// * `arguments` - The identifier of the typed list containing scope arguments for type resolution.
/// * `next_aux_id` - A mutable reference to the counter generating unique auxiliary IDs.
/// * `aux_defs` - A mutable reference to the vector storing active predicate definitions.
/// * `store` - A mutable reference to the global expression store for list interning.
///
/// # Errors
/// Returns a [`DatalogError`] if the provided `arguments` list identifier cannot be
/// successfully fetched from the `ExprStore`.
fn intern_auxiliary_signature(
    resolved_terms: &[Term],
    arguments: TypedListId,
    next_aux_id: &mut usize,
    aux_defs: &mut Vec<AtomicFormulaSkeleton>,
    store: &mut ExprStore,
) -> Result<Atom, DatalogError> {
    let id = *next_aux_id;
    *next_aux_id += 1;

    let parameters = store.fetch_typed_list(arguments)?;

    let mut aux_params = TypedList::new();
    for term in resolved_terms {
        if let Term::Variable(v_id) = term {
            let ty = parameters[v_id.as_usize()].ty();
            aux_params.push(TypedSymbol::new(*v_id, ty.clone()));
        }
    }

    let list_id = store.intern_typed_list(aux_params);

    aux_defs.push(AtomicFormulaSkeleton::new(
        PredicateSymbolId::from(id),
        list_id,
    ));

    // OPTIMIZATION: Convert the slice directly into a SmallVec backed container.
    // Prevents standard heap allocations if the predicate arity falls within limits.
    let terms = AtomArgs::from_slice(resolved_terms);

    Ok(Atom::nary(AtomSkeletonId::from(id), terms))
}

/// Collects all unique variables from a slice of atoms and returns them as a sorted vector.
///
/// This function identifies every `VariableId` present in the terms of the provided atoms,
/// resolves any active variable aliases, and produces a compact, deduplicated list.
///
/// # Logic and Implementation
/// This function uses a **Bitset** (a bitmask of type `settings::VariableMask` via `compute_variable_bitmask`)
/// to perform a "Union" operation of all variables in a single pass.
/// 1. It iterates through all terms of all atoms and resolves active aliases.
/// 2. For each variable, it sets the corresponding bit in the mask.
/// 3. It then extracts the set bits (`decode_bitmask_to_variables`) to reconstruct the sorted `VariableId` list.
///
/// # Performance
/// - **Time Complexity**: $O(T + V)$, where $T$ is the total number of terms across all atoms
///   and $V$ is the number of unique variables. The bitwise extraction is extremely fast
///   thanks to CPU-level instructions.
/// - **Space Complexity**: $O(V)$ for the resulting `Vec`. The internal bitset resides
///   entirely on the stack.
/// - **Instruction-Level Optimization**:
///     - Uses `count_ones()` (POPCNT) to pre-allocate the exact capacity of the `Vec`,
///       preventing reallocations.
///     - Uses `trailing_zeros()` (TZCNT) to jump directly to the next set bit, avoiding
///       a full bit-by-bit linear scan.
///
/// # Constraints & Safety
/// - **Variable Limit**: This implementation is strictly bound by the capacity of `settings::VariableMask`
///   and enforced up to `settings::MAX_VARIABLES_PER_SCOPE`. This ensures the bitset fits entirely
///   within a local CPU register context for optimal performance.
///
/// # Errors
/// Returns a [`DatalogError::VariableLimitExceeded`] if a resolved variable index equals or exceeds
/// the compile-time limit specified by `settings::MAX_VARIABLES_PER_SCOPE`.
///
/// # Returns
/// A `Vec<VariableId>` sorted by ID in ascending order (due to the nature of bit-scanning).
fn extract_unique_variables(
    atoms: &[Atom],
    current_aliases: &FxHashMap<VariableId, Term>,
) -> Result<Vec<VariableId>, DatalogError> {
    let mask = compute_variable_bitmask(atoms, current_aliases)?;
    Ok(decode_bitmask_to_variables(mask))
}

/// Decodes a dense variable bitmask into a sorted vector of unique variable identifiers.
///
/// This function extracts active variable indices from the bitmask using low-level bit
/// manipulation primitives. The resulting sequence is guaranteed to be sorted in
/// ascending order due to the nature of the bit-scanning process.
///
/// # Arguments
/// * `mask` - The compressed bitmask containing the variable indices to decode.
///
/// # Performance
/// - **Pre-allocation**: Utilizes `count_ones()` (POPCNT) to compute the exact number of
///   set bits, allowing the target `Vec` to be allocated with optimal capacity up front,
///   completely avoiding dynamic reallocations.
/// - **Bit Scanning**: Employs `trailing_zeros()` (TZCNT) paired with bitwise clearing
///   (`mask & (mask - 1)`) to jump directly from one set bit to the next, skipping
///   unnecessary zero-padding sequences entirely on the stack.
fn decode_bitmask_to_variables(mut mask: settings::VariableMask) -> Vec<VariableId> {
    let mut vars = Vec::with_capacity(mask.count_ones() as usize);
    while mask != 0 {
        let bit = mask.trailing_zeros();
        vars.push(VariableId::from(bit as usize));
        mask &= mask - 1; // Clear the lowest set bit
    }
    vars
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::lir::expr::ExprBuilder;
    use crate::aiplan4rust::support::lang::{Type, TypeId};

    // =========================================================================
    // 1. BITMASK & BINARY OPERATIONS TESTS
    // =========================================================================

    /// Objective: Verify nominal bitmask computation from a set of atoms and ensure variable aliases are resolved correctly.
    /// Input:
    ///   - `atoms`: A unary atom with variable index 0, and an n-ary atom with variable indexes 2 and 5.
    ///   - `current_aliases`: Empty map (no aliasing redirection).
    /// Expected Output: Success returning a bitmask with bits 0, 2, and 5 set to 1.
    #[test]
    fn test_variable_bitmask_nominal() {
        // 1. Arrange
        let current_aliases = FxHashMap::default();

        let p_unary = AtomSkeletonId::from(101);
        let p_nary = AtomSkeletonId::from(102);

        let v0 = Term::Variable(VariableId::from(0));
        let v2 = Term::Variable(VariableId::from(2));
        let v5 = Term::Variable(VariableId::from(5));

        // Construction des arguments de l'atome n-aire (sur la pile 🚀)
        let mut nary_args = AtomArgs::new();
        nary_args.push(v2);
        nary_args.push(v5);

        // Utilisation stricte de tes deux constructeurs réels
        let atoms = vec![Atom::unary(p_unary, v0), Atom::nary(p_nary, nary_args)];

        // 2. Act
        let mask = compute_variable_bitmask(&atoms, &current_aliases)
            .expect("Bitmask computation failed unexpectedly");

        // 3. Assert
        let expected_mask: settings::VariableMask = (1 << 0) | (1 << 2) | (1 << 5);

        assert_eq!(
            mask, expected_mask,
            "The computed bitmask {:#b} did not match the expected mask {:#b}",
            mask, expected_mask
        );

        assert_eq!(mask.count_ones(), 3, "Exactly 3 bits should be active");
    }

    /// Objective: Ensure that the variable limit guard-rail triggers an error when a variable index matches or exceeds the capacity.
    /// Input: An atom containing a `VariableId` index equal to `settings::MAX_VARIABLES_PER_SCOPE`.
    /// Expected Output: `Err(DatalogError::VariableLimitExceeded)` containing the offending variable ID.
    #[test]
    fn test_variable_bitmask_overflow_safety() {
        // 1. Arrange
        let current_aliases = FxHashMap::default();
        let invalid_idx = settings::MAX_VARIABLES_PER_SCOPE;

        let p_unary = AtomSkeletonId::from(999);
        let overflow_var = Term::Variable(VariableId::from(invalid_idx));

        // Create an atom containing the out-of-bounds variable
        let atoms = vec![Atom::unary(p_unary, overflow_var)];

        // 2. Act
        let result = compute_variable_bitmask(&atoms, &current_aliases);

        // 3. Assert
        assert!(
            result.is_err(),
            "Expected compute_variable_bitmask to fail with VariableLimitExceeded, but it succeeded."
        );

        match result.unwrap_err() {
            DatalogError::VariableLimitExceeded { var_id, limit } => {
                assert_eq!(
                    var_id.as_usize(),
                    invalid_idx,
                    "The error did not report the correct offending VariableId."
                );
                assert_eq!(
                    limit,
                    settings::MAX_VARIABLES_PER_SCOPE,
                    "The error did not report the correct system limit boundary."
                );
            }
            other_error => panic!(
                "Expected DatalogError::VariableLimitExceeded, but got a different error: {:?}",
                other_error
            ),
        }
    }

    /// Objective: Verify that negated atoms are completely ignored when computing the covered variable bitmask.
    /// Input: A slice of atoms containing one positive atom (var 1) and one negated atom (var 3).
    /// Expected Output: Success returning a bitmask where ONLY bit 1 is set. Bit 3 must be 0.
    #[test]
    fn test_covered_variable_bitmask_negation_handling() {
        // 1. Arrange
        let p_sk = AtomSkeletonId::from(50);

        let v1 = Term::Variable(VariableId::from(1));
        let v3 = Term::Variable(VariableId::from(3));

        // A positive atom containing variable 1
        let pos_atom = Atom::unary(p_sk, v1);

        // An atom containing variable 3, which is set to negated via the `.negated()` method
        let mut neg_atom = Atom::unary(p_sk, v3);
        neg_atom.negated();

        let atoms = vec![pos_atom, neg_atom];

        // 2. Act
        let mask = compute_covered_variable_bitmask(&atoms)
            .expect("Covered bitmask computation failed unexpectedly");

        // 3. Assert
        // Only bit 1 should be present (1 << 1 = 2). Bit 3 (1 << 3 = 8) must be ignored.
        let expected_mask: settings::VariableMask = 1 << 1;

        assert_eq!(
            mask,
            expected_mask,
            "The covered mask {:#b} should only contain bit 1, but it differs from {:#b}. Negated atoms were likely not ignored.",
            mask,
            expected_mask
        );

        assert_eq!(
            mask.count_ones(),
            1,
            "Exactly 1 covered variable should be detected"
        );
    }

    /// Objective: Validate that decoding a bitmask extracts unique variables in strict ascending order using CPU bit-scanning.
    /// Input: A `VariableMask` with bits 1, 4, and 12 explicitly set to 1.
    /// Expected Output: A `Vec<VariableId>` containing `[VariableId(1), VariableId(4), VariableId(12)]`.
    #[test]
    fn test_bitmask_decoding_order() {
        let mask: settings::VariableMask = (1 << 1) | (1 << 12) | (1 << 4);

        let vars = decode_bitmask_to_variables(mask);

        assert_eq!(vars.len(), 3);
        assert_eq!(vars[0], VariableId::from(1));
        assert_eq!(vars[1], VariableId::from(4));
        assert_eq!(vars[2], VariableId::from(12));
    }

    // =========================================================================
    // 2. HIGH-LEVEL PIPELINE & STORE MUTATION TESTS
    // =========================================================================

    /// Objective: Validate that `extract_atom` builds a standard Datalog atom from an AST node, mapping variables and objects properly.
    /// Input: An `ExprStore` populated with an `AtomicFormula` node via the `ExprBuilder` zero-allocation mechanics.
    /// Expected Output: A structural `Atom` with `skeleton_id` matching the predicate and terms containing the mapped variables/constants.
    #[test]
    fn test_extract_atom_from_formula_and_comparison() {
        // 1. Arrange
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Declare identifiers and terminals compliant with the logical model
        let target_sk_id = AtomSkeletonId::from(42);
        let pred_sym_id = PredicateSymbolId::from(10);
        let var_id = VariableId::from(0);
        let obj_id = crate::aiplan4rust::support::lang::ObjectId::from(7);

        // Extract expression identifiers generated by the store/builder
        let expr_var = builder.variable(var_id);

        // Using an Object/Constant to align with the matching logic inside extract_atom
        let expr_obj = builder.object(obj_id);

        // Construct the target atomic formula via the builder API
        let root_expr_id = builder.atomic_formula(pred_sym_id, &[expr_var, expr_obj], target_sk_id);

        // Retrieve the concrete reference of the root node (ExprNode)
        let root_node = builder
            .fetch(root_expr_id)
            .expect("Failed to fetch generated root atomic formula from the store");

        // 2. Act
        let result = extract_atom(root_node, builder.store)
            .expect("extract_atom failed to process the built AST nodes");

        // 3. Assert
        assert_eq!(
            result.symbol().as_usize(),
            42,
            "The extracted Datalog atom skeleton ID does not match the AST definition."
        );

        let args = result.arguments();
        assert_eq!(
            args.len(),
            2,
            "The extracted Datalog atom should have exactly 2 resolved arguments."
        );

        // Verify successful structural mapping to concrete Datalog Term types
        assert_eq!(
            args[0],
            Term::Variable(var_id),
            "First argument should have mapped to Term::Variable."
        );
        assert_eq!(
            args[1],
            Term::Constant(obj_id),
            "Second argument should have mapped to Term::Constant (Object)."
        );
    }

    /// Objective: Test the Rule Repair algorithm within `allocate_auxiliary_predicate` to inject safety unary type atoms for unbound variables.
    /// Input:
    ///   - `atoms`: A negated atom containing a variable that is not bound by any positive atom in the scope.
    ///   - `parameters_id`: The ID of the scoped parameters list to extract types from.
    /// Expected Output:
    ///   - A generated `head` Atom matching the newly declared auxiliary predicate signature.
    ///   - A `secured_body` containing the original atoms PLUS the newly injected unary type atom locking the free variable.
    #[test]
    fn test_allocate_auxiliary_predicate_rule_repair() {
        // 1. Arrange
        let mut store = ExprStore::new();
        let mut aux_defs = Vec::new();
        let mut next_id = 0;
        let current_aliases = rustc_hash::FxHashMap::default();

        // Dummy type identifiers and skeletons for testing
        let type_id = TypeId::from(3);
        let type_skel_id = AtomSkeletonId::from(500); // The unary predicate associated with this type

        // Indexed mapping: type_to_skeleton[type_id] -> type_skel_id
        let type_to_skeleton = vec![
            AtomSkeletonId::from(0),
            AtomSkeletonId::from(0),
            AtomSkeletonId::from(0),
            type_skel_id,
        ];

        // Scope variable declarations (our free variable has VariableId(0) and TypeId(3))
        let free_var_id = VariableId::from(0);
        let mut scope_parameters: TypedList<VariableId, TypeId> = TypedList::new();

        // Wrap inside a TypedSymbol to match your parameter list API
        let typed_var = TypedSymbol::new(free_var_id, Type::primitive(type_id));
        scope_parameters.push(typed_var);

        // Intern the list properly into your ExprStore to get a valid ID
        let parameters_id = store.intern_typed_list(scope_parameters);

        // Create the initial body: a single NEGATED atom containing the free variable
        let mut neg_atom = Atom::unary(AtomSkeletonId::from(100), Term::Variable(free_var_id));
        neg_atom.negated();
        let initial_atoms = vec![neg_atom];

        // 2. Act
        let (head, secured_body) = allocate_auxiliary_predicate(
            &initial_atoms,
            parameters_id,
            &mut next_id,
            &mut aux_defs,
            &type_to_skeleton,
            &current_aliases,
            &mut store,
        )
        .expect("allocate_auxiliary_predicate failed during rule repair fabrication");

        // 3. Assert
        // Verify the creation of the auxiliary predicate signature (Head)
        assert_eq!(
            next_id, 1,
            "The auxiliary predicate ID counter should have incremented."
        );
        assert_eq!(
            aux_defs.len(),
            1,
            "One new auxiliary definition skeleton should have been generated."
        );
        assert!(
            !head.is_negated(),
            "The auxiliary head must always be a positive literal."
        );

        // Verify Rule Repair (Secured body)
        assert_eq!(
            secured_body.len(),
            2,
            "The secured body must contain exactly 2 atoms (the original negated atom + 1 injected type-guard)."
        );

        // Find the injected type guard inside the secured_body
        let type_guard_atom = secured_body
            .iter()
            .find(|atom| atom.symbol() == type_skel_id)
            .expect(
                "Rule Repair failure: The unary type-guard atom was not injected into the body.",
            );

        assert!(
            !type_guard_atom.is_negated(),
            "The injected type-guard atom must be positive to successfully bind the variable."
        );
        assert_eq!(
            type_guard_atom.arguments()[0],
            Term::Variable(free_var_id),
            "The injected type-guard must target the free variable."
        );
    }

    /// Objective: Verify that when a variable is resolved as a Constant via `current_aliases`,
    ///            it is not treated as a free variable and no type-guard is injected.
    /// Input:
    ///   - `initial_atoms`: A single negated atom targeting `VariableId(0)`.
    ///   - `current_aliases`: A mapping resolving `VariableId(0)` directly to `Constant(7)`.
    /// Expected Output:
    ///   - A `secured_body` containing exactly 1 atom (the original negated atom). No safety unary type-guard is added since constants are inherently bound.
    #[test]
    fn test_allocate_auxiliary_predicate_with_constant_alias() {
        let mut store = ExprStore::new();
        let mut aux_defs = Vec::new();
        let mut next_id = 0;

        // 1. Arrange an alias: Variable(0) -> Constant(7)
        let mut current_aliases = FxHashMap::default();
        let free_var_id = VariableId::from(0);
        let obj_id = crate::aiplan4rust::support::lang::ObjectId::from(7);
        current_aliases.insert(free_var_id, Term::Constant(obj_id));

        let type_id = TypeId::from(3);
        let type_skel_id = AtomSkeletonId::from(500);
        let type_to_skeleton = vec![
            AtomSkeletonId::from(0),
            AtomSkeletonId::from(0),
            AtomSkeletonId::from(0),
            type_skel_id,
        ];

        let mut scope_parameters: TypedList<VariableId, TypeId> = TypedList::new();
        scope_parameters.push(TypedSymbol::new(free_var_id, Type::primitive(type_id)));
        let parameters_id = store.intern_typed_list(scope_parameters);

        // A negated atom targeting the variable (which resolves to a constant)
        let mut neg_atom = Atom::unary(AtomSkeletonId::from(100), Term::Variable(free_var_id));
        neg_atom.negated();
        let initial_atoms = vec![neg_atom];

        // 2. Act
        let (_head, secured_body) = allocate_auxiliary_predicate(
            &initial_atoms,
            parameters_id,
            &mut next_id,
            &mut aux_defs,
            &type_to_skeleton,
            &current_aliases,
            &mut store,
        )
        .unwrap();

        // 3. Assert: No type-guard should be injected because the term resolves to a Constant!
        assert_eq!(
            secured_body.len(),
            1,
            "The secured body should NOT contain a type-guard since the variable resolves to a constant."
        );
    }

    /// Objective: Ensure `extract_atom` returns `Err(DatalogError::IncompatibleNode)`
    ///            when called with an invalid node kind (e.g., ExprKind::And).
    /// Input:
    ///   - `root_node`: An invalid `ExprNode` context representing an `ExprKind::And` logical junction.
    /// Expected Output:
    ///   - An explicit `Err(DatalogError::IncompatibleNode)` signaling structural misalignment.
    #[test]
    fn test_extract_atom_incompatible_node_error() {
        let mut store = ExprStore::new();
        let mut builder = ExprBuilder::new(&mut store);

        // Create an invalid node context for an atom (an AND junction)
        let root_expr_id = builder.and(&[]);
        let root_node = builder.fetch(root_expr_id).unwrap();

        // 2. Act
        let result = extract_atom(root_node, builder.store);

        // 3. Assert
        assert!(
            result.is_err(),
            "Expected extract_atom to fail for an And node."
        );
        match result.unwrap_err() {
            DatalogError::IncompatibleNode { .. } => {} // Success
            other => panic!("Expected DatalogError::IncompatibleNode, got {:?}", other),
        }
    }

    /// Objective: Verify that if a variable appears multiple times in the body,
    ///            the auxiliary head predicate signature only contains it once (deduplication).
    /// Input:
    ///   - `initial_atoms`: Two separate positive atoms sharing the exact same `VariableId(0)`.
    /// Expected Output:
    ///   - An auxiliary `head` atom featuring a deduplicated arguments footprint with an exact arity of 1.
    #[test]
    fn test_allocate_auxiliary_predicate_variable_deduplication() {
        let mut store = ExprStore::new();
        let mut aux_defs = Vec::new();
        let mut next_id = 0;
        let current_aliases = FxHashMap::default();

        let type_id = TypeId::from(0);
        let type_to_skeleton = vec![AtomSkeletonId::from(99)];

        let var_id = VariableId::from(0);
        let mut scope_parameters: TypedList<VariableId, TypeId> = TypedList::new();
        scope_parameters.push(TypedSymbol::new(var_id, Type::primitive(type_id)));
        let parameters_id = store.intern_typed_list(scope_parameters);

        // Two positive atoms sharing the EXACT same variable
        let atom1 = Atom::unary(AtomSkeletonId::from(100), Term::Variable(var_id));
        let atom2 = Atom::unary(AtomSkeletonId::from(200), Term::Variable(var_id));
        let initial_atoms = vec![atom1, atom2];

        // 2. Act
        let (head, _secured_body) = allocate_auxiliary_predicate(
            &initial_atoms,
            parameters_id,
            &mut next_id,
            &mut aux_defs,
            &type_to_skeleton,
            &current_aliases,
            &mut store,
        )
        .unwrap();

        // 3. Assert: Head arity must be exactly 1, not 2!
        assert_eq!(
            head.arguments().len(),
            1,
            "The auxiliary predicate head must deduplicate variables and have an arity of 1."
        );
    }

    /// Objective: Verify that a variable exactly at the upper boundary limit (MAX_VARIABLES_PER_SCOPE - 1)
    ///            is handled correctly without panicking or triggering an overflow error.
    /// Input:
    ///   - `atoms`: An atom containing a `VariableId` set at the maximum valid bitmask boundary index.
    /// Expected Output:
    ///   - A successful execution resulting in a bitmask with only the uppermost registered bit activated (`1 << boundary_idx`).
    #[test]
    fn test_variable_bitmask_exact_boundary() {
        // 1. Arrange
        let current_aliases = FxHashMap::default();
        // Maximum allowed index (e.g., 63 if the limit is 64)
        let boundary_idx = settings::MAX_VARIABLES_PER_SCOPE - 1;

        let p_unary = AtomSkeletonId::from(100);
        let boundary_var = Term::Variable(VariableId::from(boundary_idx));
        let atoms = vec![Atom::unary(p_unary, boundary_var)];

        // 2. Act
        let mask = compute_variable_bitmask(&atoms, &current_aliases);

        // 3. Assert
        assert!(
            mask.is_ok(),
            "The exact boundary index {} should be valid but returned an error.",
            boundary_idx
        );

        let expected_mask: settings::VariableMask = 1 << boundary_idx;
        assert_eq!(
            mask.unwrap(),
            expected_mask,
            "The boundary bit was not set correctly in the mask."
        );
    }
}
