//! AST Type Finalization Pass
//!
//! This module provides the logic for synchronizing the physical Abstract Syntax Tree (AST)
//! with the semantically resolved types from the [`SymbolTable`].
//!
//! ## Overview
//!
//! During the early stages of parsing and linking, PDDL types are often represented
//! broadly as `(either ...)` blocks. Once the semantic analyzer (inference engine)
//! determines a specific narrowed type for a symbol, the AST becomes "stale"—it still
//! contains the original broad type list.
//!
//! This pass iterates through the tree and "prunes" (physically removes) type
//! identifiers that are no longer part of the resolved type definition.
//!
//! ## Architecture
//!
//! The finalization process follows a strictly ordered pattern to comply with
//! Rust's ownership model:
//!
//! 1. **Filtering**: Symbols are screened using `ty.is_root()` and `ty.is_primitive()`
//!    to avoid unnecessary tree traversals.
//! 2. **Inspection (Immutable)**: The module navigates to the `TypedItem` and identifies
//!    obsolete [`NodeId`]s, storing them in a stack-allocated [`SmallVec`].
//! 3. **Pruning (Mutable)**: The module re-borrows the tree mutably to detach the
//!    orphaned nodes and update the parent's children list.
//!
//! ## Constants
//!
//! * [`INLINE_CAPACITY`]: Optimized stack size for the pruning buffer to minimize
//!   heap allocations.
//!
//! ## Safety
//!
//! This module uses the `Arena` pattern. It ensures tree integrity by manually
//! orphaning nodes (setting their parent to `None`) before removing them from
//! child lists, preventing dangling parent-child references.

use crate::aiplan4rust::compiler::linking::finalization::error::FinalizationError;
use crate::aiplan4rust::compiler::linking::finalization::FinalizationContext;
use crate::aiplan4rust::compiler::semantic::symbol::{Declaration, SymbolKind};
use crate::aiplan4rust::compiler::syntax::ast::arena::ArenaNode;
use crate::aiplan4rust::compiler::syntax::ast::tree::{NodeId, Tree};
use crate::aiplan4rust::compiler::syntax::ast::{AstContent, AstKind, AstNode};
use crate::aiplan4rust::support::diagnostic::Diagnostic;
use crate::aiplan4rust::support::lang::SymbolId;
use crate::{DiagnosticManager, SymbolTable};
use smallvec::SmallVec;

/// The maximum number of elements to be stored on the stack before migrating to the heap.
///
/// This constant defines the inline capacity for [`SmallVec`] collections used during
/// AST pruning.
///
/// ### Performance Rationale
/// - **Stack Allocation**: In PDDL, the majority of `either` type blocks contain
///   fewer than 8 types (often 2 or 3). By setting this to `8`, we ensure that
///   most pruning operations involve zero heap allocations, significantly
///   speeding up the finalization pass.
/// - **Memory Footprint**: Keeping this value small prevents the stack frame
///   of the `finalize` function from becoming excessively large when processing
///   deeply nested declarations.
const INLINE_CAPACITY: usize = 8;

/// Synchronizes the Abstract Syntax Tree (AST) with narrowed semantic type definitions.
///
/// This function performs a "Pruning Pass" over the AST. In PDDL, symbols can be declared
/// with multiple possible types using the `either` syntax: `?x - (either typeA typeB)`.
/// During semantic analysis (linking and inference), it is common to discover that a symbol
/// is actually restricted to a subset of those types (e.g., only `typeA`).
///
/// While the `SymbolTable` is updated to reflect this narrowed reality, the physical AST
/// remains unchanged. This function identifies such discrepancies and physically removes
/// the obsolete type identifiers from the AST nodes.
///
/// # Arguments
///
/// * `context` - The finalization context containing shared state and settings for the pass.
/// * `symbol_table` - A reference to the resolved symbol table containing the "semantic truth."
/// * `ast` - A mutable reference to the AST tree to be pruned.
/// * `diagnostic_manager` - A mutable reference to the manager responsible for collecting
///   and reporting warnings (e.g., when a type list is narrowed).
///
/// # Logic and Optimizations
///
/// The function follows a "Look-ahead, then Mutate" pattern to respect Rust's ownership rules:
/// 1. **High-Speed Filtering**: Symbols that are root types or primitive types are skipped
///    immediately as they cannot contain `either` blocks.
/// 2. **Structural Validation**: It uses a helper to locate the physical `TypedItem` node.
///    If the symbol is a usage rather than a declaration, it is ignored.
/// 3. **Step A (Immutable)**: Identifies specific child nodes in the AST that represent
///    types no longer present in the `SymbolTable`.
/// 4. **Reporting**: If pruning occurs, a warning is emitted via the `diagnostic_manager`
///    to inform the user of the automated type narrowing.
/// 5. **Step B (Mutable)**: Detaches the identified nodes from the tree and updates the
///    parent's children list.
///
/// # Errors
///
/// Returns a [`FinalizationError`] if:
/// * The AST structure is corrupted (e.g., a parent link is missing).
/// * A required node ID cannot be found in the tree arena.
/// * The symbol kind is unsupported for finalization.
///
/// # Examples
///
/// Input AST:  `?v - (either ship object)`
/// SymbolTable: `v` is resolved to only `ship`.
/// Output AST: `?v - ship`
/// *Diagnostic: A warning is issued notifying that `object` was removed from `v`'s type list.*
pub fn finalize(
    context: &FinalizationContext,
    symbol_table: &SymbolTable,
    ast: &mut Tree<AstNode>,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<(), FinalizationError> {
    // We pre-allocate buffers here. They stay on the stack and
    // we just clear them at each iteration.
    let mut to_remove: SmallVec<[NodeId; INLINE_CAPACITY]> = SmallVec::new();
    let mut removed_type_ids: SmallVec<[SymbolId; INLINE_CAPACITY]> = SmallVec::new();

    for symbol in symbol_table {
        for declaration in symbol.declarations() {
            // Check if the declaration has an associated type.
            // If not, there's nothing to synchronize.
            let Some(ty) = declaration.ty() else {
                continue;
            };

            // Optimization: Skip root types (object, number) and simple types.
            // If a type has 0 or 1 member, it cannot be an 'either' type that
            // requires physical narrowing in the AST.
            // We skip this BEFORE the expensive AST navigation helper.
            if ty.is_root() || ty.is_primitive() {
                continue;
            }

            // 1. Locate the "Type" container node.
            // If None, this declaration doesn't use a TypedItem syntax (e.g., it's
            // a function usage/initialization), so we skip it.
            let Some(type_node_id) = get_type_node_id(declaration, ast)? else {
                continue;
            };

            let type_node = ast.try_node(type_node_id)?;
            let members_to_keep = ty.members();

            // Secondary Optimization: If the AST child count already matches the
            // SymbolTable member count, the nodes are already synchronized.
            if members_to_keep.len() == type_node.children().len() {
                continue;
            }

            // --- STEP A: Collect IDs (No heap allocation if types <= INLINE_CAPACITY) ---
            to_remove.clear();
            removed_type_ids.clear();

            for &child_id in type_node.children() {
                if let Ok(child_node) = ast.try_node(child_id) {
                    if let AstContent::Ident(id) = child_node.content() {
                        if !members_to_keep.contains(id) {
                            to_remove.push(child_id);
                            removed_type_ids.push(*id);
                        }
                    }
                }
            }

            // --- STEP B: Report Diagnostic ---
            if !removed_type_ids.is_empty() {
                // We only convert to a Vec at the moment of reporting if your Diagnostic requires it,
                // or if we need to adapt the constructor to accept an IntoIterator.
                diagnostic_manager.report(Diagnostic::warning_type_narrowing(
                    *declaration.symbol(),
                    removed_type_ids.to_vec(), // Conversion happens here, only when a warning is actually triggered.
                    members_to_keep.to_vec(),
                    context.provider(),
                    context.source(),
                    declaration.span(),
                ));
            }

            // --- STEP B: Cleanup and Orphaning (Mutable Borrow) ---
            // Detach nodes from parent first to maintain AST integrity.
            for &id in &to_remove {
                if let Ok(node) = ast.try_node_mut(id) {
                    node.set_parent(None);
                }
            }

            // Physically update the children list of the Type node.
            let type_node_mut = ast.try_node_mut(type_node_id)?;
            let children = type_node_mut.children_mut();

            // Retain only the valid narrowed types.
            children.retain(|id| !to_remove.contains(id));
        }
    }
    Ok(())
}

/// Navigates the AST to find the specific "Type" node associated with a declaration.
///
/// This helper abstracts the structural differences between various symbol types
/// (variables, constants, and functions) to find the common `TypedItem` container.
///
/// # Returns
///
/// - `Ok(Some(NodeId))` if a valid type definition block is found.
/// - `Ok(None)` if the symbol is not part of a type-narrowing context (e.g., built-ins or
///   malformed nodes).
///
/// # Errors
///
/// Returns [`FinalizationError`] if:
/// - The expected tree structure is missing or a parent/child link is broken.
/// - The symbol kind is not supported for type finalization (e.g., trying to finalize
///   a symbol that doesn't belong to a typed structure).
fn get_type_node_id(
    declaration: &Declaration,
    ast: &Tree<AstNode>,
) -> Result<Option<NodeId>, FinalizationError> {
    let declaration_node_id = declaration.source();

    // 1. Traverse up to the TypedItem node.
    // The path differs based on the symbol kind:
    // - Variables/Constants: Ident -> TypedItem
    // - Functions: FunctionSymbol -> FunctionSkeleton -> TypedItem
    let symbol = declaration.symbol();
    let typed_item_node_id = match symbol.kind() {
        SymbolKind::PrimitiveType | SymbolKind::Constant | SymbolKind::Variable => {
            ast.try_node(declaration_node_id)?.try_parent()?
        }
        SymbolKind::Function => {
            let skeleton_id = ast.try_node(declaration_node_id)?.try_parent()?;
            ast.try_node(skeleton_id)?.try_parent()?
        }
        _ => return Err(FinalizationError::unsupported_symbol_kind(symbol)),
    };

    let typed_item = ast.try_node(typed_item_node_id)?;

    // 2. Validate the container.
    // We ensure the node is indeed a TypedItem and that it actually contains
    // a type definition (index 1). If not, we skip it.
    if typed_item.kind() != AstKind::TypedItem || typed_item.children().len() <= 1 {
        return Ok(None);
    }

    // 3. Extract the Type node ID.
    // In a TypedItem structure, child 0 is the TypedList (names)
    // and child 1 is the Type (the definition to be pruned).
    let type_node_id = typed_item.try_child(1)?;

    // 4. Built-in protection.
    // Built-in types (indices 0-5) are virtual and do not exist as
    // physical "either" blocks in the AST that require pruning.
    if AstNode::is_builtin(type_node_id) {
        return Ok(None);
    }

    Ok(Some(type_node_id))
}
