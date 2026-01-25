//! Encoding Context Management
//!
//! This module defines the `EncodingContext`, the central structure used during
//! the logical encoding pass. It links syntactic declarations (AST) to their
//! resolved intermediate representations (LIR) and manages symbol visibility.

use std::collections::HashMap;
use crate::aiplan4rust::semantic::symbol_table::SymbolTable;
use crate::aiplan4rust::syntax::tree::NodeId;

/// Context used during the encoding of actions, methods, and expressions.
///
/// This structure acts as a bridge between the semantic analysis and the LIR.
/// It carries the necessary mappings to resolve names into indices.
pub struct EncodingContext<'a> {
    /// **The Symbol Table**: A reference to the semantic table containing
    /// identifiers for the current scope (e.g., action parameters, constants).
    symbol_table: &'a SymbolTable,

    /// **Predicate Mapping**: A hash map linking the `NodeId` of a predicate
    /// declaration in the AST to its unique positional index in the LIR.
    ast_pred_to_idx: &'a HashMap<NodeId, usize>,

    /// **Function Mapping**: A hash map linking the `NodeId` of a numeric
    /// function (fluent) declaration in the AST to its unique index in the LIR.
    ast_func_to_idx: &'a HashMap<NodeId, usize>,
}

impl<'a> EncodingContext<'a> {
    /// Creates a new `EncodingContext`.
    ///
    /// # Arguments
    ///
    /// * `symbol_table` - The table used to resolve local variables and symbols.
    /// * `ast_pred_to_idx` - The global registry of predicate indices.
    /// * `ast_func_to_idx` - The global registry of function indices.
    pub fn new(
        symbol_table: &'a SymbolTable,
        ast_pred_to_idx: &'a HashMap<NodeId, usize>,
        ast_func_to_idx: &'a HashMap<NodeId, usize>,
    ) -> Self {
        Self {
            symbol_table,
            ast_pred_to_idx,
            ast_func_to_idx,
        }
    }

    /// Returns the symbol table for identifier resolution.
    pub fn symbol_table(&self) -> &SymbolTable {
        self.symbol_table
    }

    /// Returns the mapping of AST predicate declarations to LIR indices.
    pub fn ast_pred_to_idx(&self) -> &HashMap<NodeId, usize> {
        self.ast_pred_to_idx
    }

    /// Returns the mapping of AST function declarations to LIR indices.
    pub fn ast_func_to_idx(&self) -> &HashMap<NodeId, usize> {
        self.ast_func_to_idx
    }

    /// Resolves a predicate index from its AST declaration ID.
    ///
    /// # Parameters
    /// * `decl_id` - The unique identifier of the predicate in the AST.
    pub fn get_predicate_index(&self, decl_id: &NodeId) -> Option<usize> {
        self.ast_pred_to_idx.get(decl_id).copied()
    }

    /// Resolves a function index from its AST declaration ID.
    ///
    /// # Parameters
    /// * `decl_id` - The unique identifier of the function in the AST.
    pub fn get_function_index(&self, decl_id: &NodeId) -> Option<usize> {
        self.ast_func_to_idx.get(decl_id).copied()
    }
}
