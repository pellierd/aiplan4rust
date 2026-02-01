//! Encoding Context Management
//!
//! This module defines the `EncodingContext`, the central structure used during
//! the logical encoding pass. It links syntactic declarations (AST) to their
//! resolved intermediate representations (LIR) and manages symbol visibility.

use std::collections::HashMap;
use crate::aiplan4rust::lang::{AtomSkeletonID, FunctionSkeletonID, FunctorID, ObjectID, PredicateID, TaskSymbolID, TaskSkeletonID, TypeID, VariableID, PreferenceID};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::semantic::symbol::Symbol;
use crate::aiplan4rust::semantic::symbol_table::SymbolTable;
use crate::aiplan4rust::tree::NodeId;

/// Context used during the encoding of actions, methods, and expressions.
///
/// This structure acts as a bridge between the semantic analysis and the LIR.
/// It carries the necessary mappings to resolve names into indices.
pub struct EncodingRegistry {

    /// **The Symbol Table**: A reference to the semantic table containing
    /// identifiers for the current scope (e.g., action parameters, constants).
    symbol_table: SymbolTable,

    /// **Type Mapping**: Links a semantic `Type` structure (primitive or union)
    /// to its unique index in the LIR.
    type_to_id: HashMap<NodeId, TypeID>,

    predicate_to_id: HashMap<NodeId, PredicateID>,

    /// **Predicate Mapping**: Links a predicate's logical `Symbol`
    /// to its unique positional index in the LIR.
    atom_skeleton_to_id: HashMap<NodeId, AtomSkeletonID>,

    functor_to_id: HashMap<NodeId, FunctorID>,

    /// **Function Mapping**: Links a function's logical `Symbol`
    /// to its unique index in the LIR.
    function_skeleton_to_id: HashMap<NodeId, FunctionSkeletonID>,

    /// **Object Mapping**: Links a logical `Symbol` (either a global Constant
    /// from the domain or an Object from the problem) to its unique index.
    object_to_id: HashMap<NodeId, ObjectID>,

    task_symbol_to_id: HashMap<NodeId, TaskSymbolID>,

    task_skeleton_to_id: HashMap<NodeId, TaskSkeletonID>,

    /// **Variable Mapping**: Links a variable's declaration `NodeId` (from AST)
    /// to its local `VariableID` index (0, 1, 2...).
    /// This handles local scope (actions, forall, exists) without naming conflicts.
    variable_to_id: HashMap<NodeId, VariableID>,

    preference_to_id: HashMap<NodeId, PreferenceID>,

}

impl EncodingRegistry {
    /// Creates a new `EncodingContext`.
    ///
    /// # Arguments
    ///
    /// * `symbol_table` - The table used to resolve local variables and symbols.
    /// * `ast_pred_to_idx` - The global registry of predicate indices.
    /// * `ast_func_to_idx` - The global registry of function indices.
    pub fn new(
        symbol_table: SymbolTable,
    ) -> Self {
        Self {
            symbol_table,
            type_to_id : HashMap::new(),
            object_to_id : HashMap::new(),
            atom_skeleton_to_id: HashMap::new(),
            predicate_to_id: HashMap::new(),
            function_skeleton_to_id: HashMap::new(),
            functor_to_id: HashMap::new(),
            task_skeleton_to_id: HashMap::new(),
            task_symbol_to_id: HashMap::new(),
            variable_to_id : HashMap::new(),
            preference_to_id: HashMap::new(),

        }
    }

    /// Returns the symbol table for identifier resolution.
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /*pub fn resolve_type(&self, ty: &Type<NodeId>) -> Option<Vec<TypeID>> {
        let mut ids = Vec::new();
        for t in ty.members() {
            if let Some(id) = self.resolve_type_symbol(*t) {
                ids.push(id);
            } else {
                return None; // Un des membres est inconnu
            }
        }
        ids.sort();
        ids.dedup();
        Some(ids)
    }

    /// La version "Strict" que tu utiliseras lors de l'encodage.
    pub fn try_resolve_type(&self, ty: &Type<StringID>) -> Result<Vec<TypeID>, LirError> {
        self.resolve_type(ty)
            .ok_or_else(|| LirError::type_binding_failed(ty.clone()))
    }*/

    /// Récupère l'ID d'un type PRIMITIF uniquement (par son symbole).
    pub fn resolve_type_symbol(&self, symbol: NodeId) -> Option<TypeID> {
        self.type_to_id.get(&symbol).copied()
    }

    /// Version avec erreur fatale
    pub fn try_resolve_type_symbol(&self, symbol: NodeId) -> Result<TypeID, LirError> {
        self.resolve_type_symbol(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol))
    }

    /// Récupère l'ID d'un prédicat par le NodeId de son symbole de déclaration.
    pub fn resolve_predicate(&self, symbol: NodeId) -> Option<PredicateID> {
        self.predicate_to_id.get(&symbol).copied()
    }

    /// Version avec erreur fatale si le prédicat n'est pas lié dans le registre.
    pub fn try_resolve_predicate(&self, symbol: NodeId) -> Result<PredicateID, LirError> {
        self.resolve_predicate(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol))
    }

    pub fn resolve_atom_skeleton(&self, symbol: NodeId) -> Option<AtomSkeletonID> {
        self.atom_skeleton_to_id.get(&symbol).copied()
    }

    pub fn try_resolve_atom_skeleton(&self, symbol: NodeId) -> Result<AtomSkeletonID, LirError> {
        self.resolve_atom_skeleton(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol.clone()))
    }

    pub fn resolve_functor_symbol(&self, symbol: NodeId) -> Option<FunctorID> {
        self.functor_to_id.get(&symbol).copied()
    }

    pub fn try_resolve_functor(&self, symbol: NodeId) -> Result<FunctorID, LirError> {
        self.resolve_functor_symbol(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol))
    }

    pub fn resolve_function_skeleton(&self, symbol: NodeId) -> Option<FunctionSkeletonID> {
        self.function_skeleton_to_id.get(&symbol).copied()
    }

    pub fn try_resolve_function_skeleton(&self, symbol: NodeId) -> Result<FunctionSkeletonID, LirError> {
        self.resolve_function_skeleton(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol.clone()))
    }

    pub fn resolve_object(&self, symbol: NodeId) -> Option<ObjectID> {
        self.object_to_id.get(&symbol).copied()
    }

    pub fn try_resolve_object(&self, symbol: NodeId) -> Result<ObjectID, LirError> {
        self.resolve_object(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol.clone()))
    }

    pub fn register_variable(&mut self, variable: NodeId) -> VariableID{
        let id = VariableID::new(self.variable_to_id.len());
        self.variable_to_id.insert(variable, id);
        id
    }

    pub fn resolve_variable(&self, decl_id: NodeId) -> Option<VariableID> {
        self.variable_to_id.get(&decl_id).copied()
    }

    pub fn try_resolve_variable(&self, decl_id: NodeId) -> Result<VariableID, LirError> {
        self.variable_to_id
            .get(&decl_id)
            .copied()
            .ok_or_else(|| LirError::variable_not_found(decl_id))
    }

    pub fn clear_variables(&mut self) {
        self.variable_to_id.clear();
    }

    pub fn resolve_task_symbol(&self, symbol: NodeId) -> Option<TaskSymbolID> {
        self.task_symbol_to_id.get(&symbol).copied()
    }

    pub fn try_resolve_task_symbol(&self, symbol: NodeId) -> Result<TaskSymbolID, LirError> {
        self.resolve_task_symbol(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol))
    }

    pub fn resolve_task_skeleton(&self, symbol: NodeId) -> Option<TaskSkeletonID> {
        self.task_skeleton_to_id.get(&symbol).copied()
    }

    pub fn try_resolve_task_skeleton(&self, symbol: NodeId) -> Result<TaskSkeletonID, LirError> {
        self.resolve_task_skeleton(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol.clone()))
    }

    pub fn resolve_preference(&self, symbol: NodeId) -> Option<PreferenceID> {
        self.preference_to_id.get(&symbol).copied()
    }

    pub fn try_resolve_preference(&self, symbol: NodeId) -> Result<PreferenceID, LirError> {
        self.resolve_preference(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol.clone()))
    }

    pub fn register_type_symbol(&mut self, symbol: NodeId) -> TypeID {
        let id = TypeID::new(self.type_to_id.len());
        self.type_to_id.insert(symbol, id);
        id
    }

    pub fn register_object(&mut self, symbol: NodeId, id: ObjectID) {
        self.object_to_id.insert(symbol, id);
    }

    pub fn register_predicate(&mut self, symbol: NodeId, id: PredicateID) {
        self.predicate_to_id.insert(symbol, id);
    }

    pub fn register_atom_skeleton(&mut self, symbol: NodeId, id: AtomSkeletonID) {
        self.atom_skeleton_to_id.insert(symbol, id);
    }

    pub fn register_functor(&mut self, symbol: NodeId, id: FunctorID) {
        self.functor_to_id.insert(symbol, id);
    }

    pub fn register_function_skeleton(&mut self, symbol: NodeId, id: FunctionSkeletonID) {
        self.function_skeleton_to_id.insert(symbol, id);
    }

    pub fn register_task_skeleton(&mut self, symbol: NodeId, id: TaskSkeletonID) {
        self.task_skeleton_to_id.insert(symbol, id);
    }

    pub fn register_task_symbol(&mut self, symbol: NodeId, id: TaskSymbolID) {
        self.task_symbol_to_id.insert(symbol, id);
    }

    pub fn register_preference(&mut self, symbol: NodeId, id: PreferenceID) {
        self.preference_to_id.insert(symbol, id);
    }
}
