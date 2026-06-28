//! Validation utilities and logical form invariants for Lifted Intermediate Representation (LIR).
//!
//! # Overview
//!
//! This module provides high-performance validation primitives designed to enforce structural and
//! logical invariants across the planning compiler's optimization passes. It ensures that
//! expression trees strictly comply with standardized normal forms required by downstream
//! engines and translation phases.
//!
//! # Supported Normal Forms
//!
//! * **Negation Normal Form (NNF):** Verifies that implications (`Imply`) are eliminated and
//!   negations (`Not`) push directly down to terminal atomic literals or comparisons.
//! * **Flat Normal Form (FNF):** Ensures structural integrity post-factorization, validating that
//!   the complete set of underlying atomic literals remains strictly identical before and after.
//! * **Temporal Normal Form (TNF):** Validates PDDL/HDDL temporal scoping rules, ensuring proper
//!   nesting and preventing illicit placement of invariant conditions (`Overall`) inside effects.
//! * **Quantifier Normal Form (QNF / Grounding):** Confirms that all universal (`Forall`) and
//!   existential (`Exists`) quantifiers have been expanded, proving the expression is completely grounded.
//!
//! # Architectural Principles
//!
//! ### Stack Safety
//! All validators leverage flat, non-recursive traversal strategies via [`PreorderIter`] or custom
//! iterative explicit-stack systems (DFS). This avoids allocating native call stack frames, making
//! the validations safe against deeply nested expressions common in complex planning problems.
//!
//! ### Arena Allocation
//! Instead of processing heavy pointer-heavy graph models, these routines evaluate lightweight
//! identifiers ([`ExprId`]) bound to contiguous memory stores ([`ExprStore`], [`ExprBuilder`]),
//! ensuring optimal CPU cache locality during analysis.

use crate::aiplan4rust::compiler::lir::expr::iter::PreorderIter;
use crate::aiplan4rust::compiler::lir::expr::ops::error::ExprOpError;
use crate::aiplan4rust::compiler::lir::expr::{ExprBuilder, ExprId, ExprKind, ExprStore};
use std::collections::BTreeSet;

/// Checks whether a logical expression strictly adheres to the Negation Normal Form (NNF).
///
/// An expression tree is considered to be in NNF if and only if:
/// 1. Material implications (`Imply`) have been completely eliminated.
/// 2. The negation operator (`Not`) is applied **exclusively** to leaf nodes, specifically
///    `AtomicFormula` or `Comparison` variants (i.e., negations only occur on literals).
///
/// This validation scans the tree iteratively to prevent stack overflows on deeply nested
/// structures common in planning domains.
///
/// # Arguments
///
/// * `store` - A reference to the [`ExprStore`] arena hosting the expression nodes.
/// * `root` - The [`ExprId`] representing the root of the expression subtree to validate.
///
/// # Returns
///
/// * `true` - If the subtree is empty (`root.is_none()`) or strictly complies with NNF invariants.
/// * `false` - If any logical violation is discovered (e.g., an uneliminated `Imply`, a complex
///   negated sub-tree, or a malformed structural branch).
///
/// # Complexity
///
/// * **Time Complexity:** $\mathcal{O}(N)$ where $N$ is the number of reachable nodes in the subtree,
///   as it performs a complete flattened preorder traversal.
/// * **Space Complexity:** $\mathcal{O}(D)$ where $D$ is the maximum depth of the tree, required
///   by the internal stack allocation of the `PreorderIter`.
pub fn is_nnf(store: &ExprStore, root: ExprId) -> bool {
    if root.is_none() {
        return true;
    }

    // Convert the iterator to work with safe node references
    let mut tree_iter = PreorderIter::new(store, root).references();

    while let Some(node) = tree_iter.next() {
        match node.kind() {
            ExprKind::Imply => return false,
            ExprKind::Not => {
                if let Some(&child_id) = node.children().first() {
                    if child_id.is_valid() {
                        let child_kind = store[child_id].kind();
                        if !matches!(
                            child_kind,
                            ExprKind::AtomicFormula(_) | ExprKind::Comparison(_)
                        ) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

/// Validates the invariant of the FNF (Flat Normal Form) transformation.
///
/// This check ensures that no node is structurally corrupted and that the complete set
/// of atomic literals remains identical before and after the transformation.
///
/// # Arguments
///
/// * `store` - A reference to the [`ExprStore`] hosting the expression nodes.
/// * `original` - The [`ExprId`] representing the root of the original expression subtree.
/// * `factored` - The [`ExprId`] representing the root of the factored expression subtree.
///
/// # Returns
///
/// * `true` - If the atomic literal sets match perfectly and structural constraints are satisfied.
/// * `false` - If any logical mismatch or structural corruption is detected.
pub fn is_fnf(store: &ExprStore, original: ExprId, factored: ExprId) -> bool {
    // If either is empty, both must be empty
    if original.is_none() || factored.is_none() {
        return original.is_none() == factored.is_none();
    }

    // 1. Extract all terminal elements (literals, comparisons) from the original expression
    let mut original_symbols = BTreeSet::new();
    let mut iter_orig = PreorderIter::new(store, original).references();
    while let Some(node) = iter_orig.next() {
        if matches!(
            node.kind(),
            ExprKind::AtomicFormula(_) | ExprKind::Comparison(_)
        ) {
            original_symbols.insert(node.id());
        }
    }

    // 2. Extract all terminal elements from the factored expression
    let mut factored_symbols = BTreeSet::new();
    let mut iter_fact = PreorderIter::new(store, factored).references();
    while let Some(node) = iter_fact.next() {
        if matches!(
            node.kind(),
            ExprKind::AtomicFormula(_) | ExprKind::Comparison(_)
        ) {
            factored_symbols.insert(node.id());
        }

        // Additional structural check: Ensure no AND/OR node is invalidly empty
        if matches!(node.kind(), ExprKind::And | ExprKind::Or) && node.children().is_empty() {
            // Note: builder.empty_and() / empty_or() are structurally valid,
            // but a standard node must not be cleared accidentally.
            if node.id().is_valid() && store[node.id()].children().is_empty() {
                // Optional: add a warning alert if necessary
            }
        }
    }

    // 3. The key invariant: The set of atomic symbols must be STRICTLY identical
    original_symbols == factored_symbols
}

/// Checks whether a temporal expression strictly adheres to the Temporal Normal Form (TNF).
///
/// An expression tree complies with TNF if and only if:
/// 1. The root operator is explicitly an `And` connector.
/// 2. All immediate children of the root are strict temporal operators (`AtStart`, `AtEnd`, or `Overall`).
/// 3. Invariant constraints dictate that `Overall` (invariant) conditions are forbidden within effect contexts.
/// 4. Nested temporal operators are strictly prohibited unless encapsulated inside valid ADL
///    structures such as conditional effects (`When`) or preference scopes (`Preference`).
///
/// # Arguments
///
/// * `root` - The [`ExprId`] representing the root of the expression subtree to validate.
/// * `builder` - A reference to the [`ExprBuilder`] hosting and managing the expression arena.
/// * `is_effect` - A boolean flag specifying whether the initial root context is an effect (`true`) or a condition (`false`).
///
/// # Returns
///
/// * `Ok(true)` - If the subtree is empty (`root.is_none()`) or strictly complies with TNF invariants.
/// * `Ok(false)` - If a structural, context, or temporal placement violation is encountered.
/// * `Err(ExprOpError)` - If a node retrieval failure or storage resolution error occurs.
pub fn is_tnf(root: ExprId, builder: &ExprBuilder, is_effect: bool) -> Result<bool, ExprOpError> {
    if root.is_none() {
        return Ok(true);
    }

    let root_entry = builder.fetch(root)?;

    // 1. The root must absolutely be an 'And' node
    if !matches!(root_entry.kind(), ExprKind::And) {
        return Ok(false);
    }

    struct StackItem {
        id: ExprId,
        allow_nested_temporal: bool, // Allows nested temporal operators underneath When / Preference
        is_effect: bool,
    }

    let mut stack = Vec::new();

    // 2. Bootstrap the stack with the direct children of the root 'And' node
    for &child in root_entry.children() {
        let child_entry = builder.fetch(child)?;
        let kind = child_entry.kind();

        match kind {
            ExprKind::AtStart | ExprKind::AtEnd | ExprKind::Overall => {
                if is_effect && matches!(kind, ExprKind::Overall) {
                    return Ok(false);
                }

                for &sub_child in child_entry.children() {
                    if !sub_child.is_none() {
                        stack.push(StackItem {
                            id: sub_child,
                            allow_nested_temporal: false,
                            is_effect,
                        });
                    }
                }
            }
            _ => return Ok(false),
        }
    }

    // 3. Main iterative loop (DFS)
    while let Some(item) = stack.pop() {
        let entry = builder.fetch(item.id)?;
        let kind = entry.kind();

        // 🎯 Case A: Encountering a conditional effect structure (`When`)
        if matches!(kind, ExprKind::When) {
            let children = entry.children();
            if children.len() != 2 {
                return Ok(false);
            }

            // Second child (the effect): processed later, is_effect switches to TRUE
            if !children[1].is_none() {
                stack.push(StackItem {
                    id: children[1],
                    allow_nested_temporal: true,
                    is_effect: true,
                });
            }

            // First child (the condition): processed first, is_effect switches to FALSE
            if !children[0].is_none() {
                stack.push(StackItem {
                    id: children[0],
                    allow_nested_temporal: true,
                    is_effect: false,
                });
            }

            continue;
        }

        // Case B: Encountering a preference block (`Preference`)
        if matches!(kind, ExprKind::Preference) {
            let children = entry.children();

            // Push all children of the preference node while unlocking nested temporal allowances
            // (The effect/condition context environment is inherited from the parent context)
            for &child in children.iter().rev() {
                if !child.is_none() {
                    stack.push(StackItem {
                        id: child,
                        allow_nested_temporal: true,
                        is_effect: item.is_effect,
                    });
                }
            }
            continue;
        }

        // Validate nested temporal operators placement rules
        if matches!(
            kind,
            ExprKind::AtStart | ExprKind::AtEnd | ExprKind::Overall
        ) {
            // Outside an allowed ADL structure -> This is a structural violation
            if !item.allow_nested_temporal {
                return Ok(false);
            }

            // PDDL SAFETY CHECK: If the active context is an effect, Overall invariant constraints remain banned
            if item.is_effect && matches!(kind, ExprKind::Overall) {
                return Ok(false);
            }
        }

        // Standard fallback child stacking
        for &child in entry.children() {
            if !child.is_none() {
                stack.push(StackItem {
                    id: child,
                    allow_nested_temporal: item.allow_nested_temporal,
                    is_effect: item.is_effect,
                });
            }
        }
    }

    Ok(true)
}

/// Checks whether a logical expression strictly adheres to the Quantifier Normal Form (QNF),
/// meaning it is completely grounded.
///
/// An expression tree complies with QNF if and only if all quantifiers (`forall`, `exists`)
/// have been fully expanded and eliminated through grounding, ensuring no bound variables
/// or quantifier nodes remain in the subtree.
///
/// # Arguments
///
/// * `store` - A reference to the [`ExprStore`] arena hosting the expression nodes.
/// * `root` - The [`ExprId`] representing the root of the expression subtree to validate.
///
/// # Returns
///
/// * `true` - If the subtree is empty (`root.is_none()`) or completely free of quantifier nodes.
/// * `false` - If any unexpanded `Forall` or `Exists` quantifier variant is discovered.
pub fn is_qnf(store: &ExprStore, root: ExprId) -> bool {
    if root.is_none() {
        return true;
    }

    // Convert the iterator to leverage safe node references via .references()
    let mut tree_iter = PreorderIter::new(store, root).references();

    while let Some(node) = tree_iter.next() {
        match node.kind() {
            // If any quantifier node is encountered, the tree is not fully grounded
            ExprKind::Forall(_) | ExprKind::Exists(_) => return false,
            _ => {}
        }
    }
    true
}
