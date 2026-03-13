//! Encoding Context Management
//!
//! This module defines the `EncodingContext`, the central structure used during
//! the logical encoding pass. It links syntactic declarations (AST) to their
//! resolved intermediate representations (LIR) and manages symbol visibility.

use std::collections::HashMap;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::{AtomSkeletonId, FunctionSkeletonId, FunctionSymbolId, ObjectId, PredicateSymbolId, TaskSymbolId, TaskSkeletonId, TypeId, VariableId, PreferenceSymbolId, SymbolId, TaskLabelSymbolId};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::lir::problem::SymbolRegistry;
use crate::aiplan4rust::semantic::symbol_table::SymbolTable;
use crate::aiplan4rust::tree::NodeId;

/// Context used during the encoding of actions, methods, and logic.
///
/// This structure acts as a bridge between the semantic analysis and the LIR.
/// It carries the necessary mappings to resolve names into indices.
pub struct EncodingRegistry {

    /// **The Symbol Table**: A reference to the semantic table containing
    /// identifiers for the current scope (e.g., action parameters, constants).
    symbol_table: SymbolTable,

    /// **Type Mapping**: Links a semantic `Type` structure (primitive or union)
    /// to its unique index in the LIR.
    type_node_to_id: HashMap<NodeId, TypeId>,
    type_symbol_to_id: HashMap<SymbolId, TypeId>,

    predicate_to_id: HashMap<NodeId, PredicateSymbolId>,

    /// **Predicate Mapping**: Links a predicate's logical `Symbol`
    /// to its unique positional index in the LIR.
    atom_skeleton_to_id: HashMap<NodeId, AtomSkeletonId>,

    functor_to_id: HashMap<NodeId, FunctionSymbolId>,

    /// **Function Mapping**: Links a function's logical `Symbol`
    /// to its unique index in the LIR.
    function_skeleton_to_id: HashMap<NodeId, FunctionSkeletonId>,

    /// **Object Mapping**: Links a logical `Symbol` (either a global Constant
    /// from the domain or an Object from the problem) to its unique index.
    object_to_id: HashMap<NodeId, ObjectId>,
    object_symbol_to_id: HashMap<SymbolId, ObjectId>,

    task_symbol_to_id: HashMap<NodeId, TaskSymbolId>,
    task_skeleton_to_id: HashMap<NodeId, TaskSkeletonId>,

    /// **Variable Mapping**: Links a variable's declaration `NodeId` (from AST)
    /// to its local `VariableID` index (0, 1, 2...).
    /// This handles local scope (actions, forall, exists) without naming conflicts.
    variable_to_id: HashMap<NodeId, VariableId>,
    variable_id_to_symbol: Vec<SymbolId>,

    preference_to_id: HashMap<NodeId, PreferenceSymbolId>,

    task_label_to_id: HashMap<SymbolId, TaskLabelSymbolId>,
    task_label_id_to_symbol: Vec<SymbolId>,

}

impl EncodingRegistry {
    pub fn set_symbol_table(&mut self, p0: SymbolTable) {
        self.symbol_table = p0;
    }
}

impl EncodingRegistry {
    /// Creates a new `EncodingContext`.
    ///
    /// # Arguments
    ///
    /// * `symbol_table` - The table used to resolve local variables and symbols.
    /// * `ast_pred_to_idx` - The global evaluator of predicate indices.
    /// * `ast_func_to_idx` - The global evaluator of function indices.
    pub fn new(
        symbol_table: SymbolTable,
    ) -> Self {
        Self {
            symbol_table,
            type_node_to_id : HashMap::new(),
            type_symbol_to_id: HashMap::new(),
            object_to_id : HashMap::new(),
            object_symbol_to_id : HashMap::new(),
            atom_skeleton_to_id: HashMap::new(),
            predicate_to_id: HashMap::new(),
            function_skeleton_to_id: HashMap::new(),
            functor_to_id: HashMap::new(),
            task_skeleton_to_id: HashMap::new(),
            task_symbol_to_id: HashMap::new(),
            variable_to_id : HashMap::new(),
            variable_id_to_symbol: Vec::new(),
            preference_to_id: HashMap::new(),
            task_label_to_id: HashMap::new(),
            task_label_id_to_symbol: Vec::new(),

        }
    }

    /// Ensures that the PDDL 'number' typing is registered in the evaluator.
    ///
    /// If the typing is not yet registered, it maps the `NUMBER_SYMBOL_ID`
    /// to the reserved `TypeId::NUMBER_TYPE_ID` (1).
    /// Returns the resolved `TypeId`.
    pub fn ensure_numeric_type(&mut self) -> TypeId {
        // Check if the symbol is already mapped to a TypeId
        if let Some(&existing_id) = self.type_symbol_to_id.get(&SymbolInterner::NUMBER_SYMBOL_ID) {
            existing_id
        } else {
            // Force the use of the constant TypeId(1)
            let id = TypeId::NUMBER_TYPE_ID;
            self.type_symbol_to_id.insert(SymbolInterner::NUMBER_SYMBOL_ID, id);

            // Note: We don't necessarily have a NodeId here because it's
            // a built-in typing, so we only update the symbol-to-id map.
            id
        }
    }

    pub fn types_count(&self) -> usize {
        self.type_node_to_id.len()
    }

    pub fn type_symbols_count(&self) -> usize {
        self.type_symbol_to_id.len()
    }
    /// Returns the symbol table for identifier resolution.
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Récupère l'ID d'un typing PRIMITIF uniquement (par son symbole).
    pub fn resolve_type_symbol(&self, symbol: NodeId) -> Option<TypeId> {
        self.type_node_to_id.get(&symbol).copied()
    }

    /// Version avec erreur fatale
    pub fn try_resolve_type_symbol(&self, symbol: NodeId) -> Result<TypeId, LirError> {
        self.resolve_type_symbol(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol))
    }

    pub fn resolve_type_symbol_by_name(&self, name_id: SymbolId) -> Option<TypeId> {
        self.type_symbol_to_id.get(&name_id).copied()
    }

    pub fn try_resolve_type_symbol_by_name(&self, name_id: SymbolId) -> Result<TypeId, LirError> {
        self.resolve_type_symbol_by_name(name_id)
            .ok_or_else(|| LirError::type_not_found(name_id))
    }

    /// Récupère l'ID d'un prédicat par le NodeId de son symbole de déclaration.
    pub fn resolve_predicate(&self, symbol: NodeId) -> Option<PredicateSymbolId> {
        self.predicate_to_id.get(&symbol).copied()
    }

    /// Version avec erreur fatale si le prédicat n'est pas lié dans le registre.
    pub fn try_resolve_predicate(&self, symbol: NodeId) -> Result<PredicateSymbolId, LirError> {
        self.resolve_predicate(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol))
    }


    pub fn resolve_atom_skeleton(&self, symbol: NodeId) -> Option<AtomSkeletonId> {
        self.atom_skeleton_to_id.get(&symbol).copied()
    }

    pub fn try_resolve_atom_skeleton(&self, symbol: NodeId) -> Result<AtomSkeletonId, LirError> {
        self.resolve_atom_skeleton(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol.clone()))
    }

    pub fn resolve_functor_symbol(&self, symbol: NodeId) -> Option<FunctionSymbolId> {
        self.functor_to_id.get(&symbol).copied()
    }

    pub fn try_resolve_functor(&self, symbol: NodeId) -> Result<FunctionSymbolId, LirError> {
        self.resolve_functor_symbol(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol))
    }

    pub fn resolve_function_skeleton(&self, symbol: NodeId) -> Option<FunctionSkeletonId> {
        self.function_skeleton_to_id.get(&symbol).copied()
    }

    pub fn try_resolve_function_skeleton(&self, symbol: NodeId) -> Result<FunctionSkeletonId, LirError> {
        self.resolve_function_skeleton(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol.clone()))
    }

    pub fn resolve_object(&self, symbol: NodeId) -> Option<ObjectId> {
        self.object_to_id.get(&symbol).copied()
    }

    pub fn try_resolve_object(&self, symbol: NodeId) -> Result<ObjectId, LirError> {
        self.resolve_object(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol.clone()))
    }


    /// Résout un ObjectID à partir de son nom (StringID).
    /// Retourne None si l'objet n'a pas été enregistré en Phase 1.
    pub fn resolve_object_symbol_by_name(&self, name_id: SymbolId) -> Option<ObjectId> {
        self.object_symbol_to_id.get(&name_id).copied()
    }

    /// Tente de résoudre un ObjectID à partir de son nom.
    ///
    /// # Errors
    /// Retourne une erreur `LirError::ObjectNotFound` si le symbole est inconnu.
    pub fn try_resolve_object_symbol_by_name(&self, name_id: SymbolId) -> Result<ObjectId, LirError> {
        self.resolve_object_symbol_by_name(name_id)
            .ok_or_else(|| LirError::object_not_found(name_id))
    }

    /// Enregistre une variable liée à un nœud AST.
    pub fn register_variable(&mut self, node_id: NodeId, symbol: SymbolId) -> VariableId {
        if let Some(&id) = self.variable_to_id.get(&node_id) {
            return id;
        }
        let id = VariableId::new(self.variable_id_to_symbol.len());
        self.variable_to_id.insert(node_id, id);
        self.variable_id_to_symbol.push(symbol);
        id
    }

    /// Extrait les symboles dans un `SymbolRegistry<VariableID>` tout propre.
    pub fn get_variable_symbols(&mut self) -> SymbolRegistry<VariableId> {
        let mut registry = SymbolRegistry::new();
        for &symbol in self.variable_id_to_symbol.iter() {
            registry.insert(symbol);
        }
        registry
    }

    pub fn resolve_variable(&self, decl_id: NodeId) -> Option<VariableId> {
        self.variable_to_id.get(&decl_id).copied()
    }

    pub fn try_resolve_variable(&self, decl_id: NodeId) -> Result<VariableId, LirError> {
        self.variable_to_id
            .get(&decl_id)
            .copied()
            .ok_or_else(|| LirError::variable_not_found(decl_id))
    }

    pub fn clear_variables(&mut self) {
        self.variable_to_id.clear();
        self.variable_id_to_symbol.clear();
    }

    pub fn resolve_task_symbol(&self, symbol: NodeId) -> Option<TaskSymbolId> {
        self.task_symbol_to_id.get(&symbol).copied()
    }

    pub fn try_resolve_task_symbol(&self, symbol: NodeId) -> Result<TaskSymbolId, LirError> {
        self.resolve_task_symbol(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol))
    }

    pub fn resolve_task_skeleton(&self, symbol: NodeId) -> Option<TaskSkeletonId> {
        self.task_skeleton_to_id.get(&symbol).copied()
    }

    pub fn try_resolve_task_skeleton(&self, symbol: NodeId) -> Result<TaskSkeletonId, LirError> {
        self.resolve_task_skeleton(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol.clone()))
    }

    pub fn resolve_preference(&self, symbol: NodeId) -> Option<PreferenceSymbolId> {
        self.preference_to_id.get(&symbol).copied()
    }

    pub fn try_resolve_preference(&self, symbol: NodeId) -> Result<PreferenceSymbolId, LirError> {
        self.resolve_preference(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol.clone()))
    }


    pub fn register_type_symbol(&mut self, symbol: SymbolId, node_id: NodeId) -> TypeId {
        // 1. Check if the typing symbol is already registered
        if let Some(&existing_id) = self.type_symbol_to_id.get(&symbol) {
            // Map this specific node to the existing typing ID
            self.type_node_to_id.insert(node_id, existing_id);
            return existing_id;
        }

        // 2. Otherwise, generate a new unique TypeID
        let new_id = TypeId::new(self.type_symbol_to_id.len());

        // 3. Register the new typing in both mappings
        self.type_symbol_to_id.insert(symbol, new_id);
        self.type_node_to_id.insert(node_id, new_id);

        new_id
    }

    pub fn register_object(&mut self, symbol: NodeId, id: ObjectId) {
        self.object_to_id.insert(symbol, id);
    }

    pub fn register_object_symbol(&mut self, symbol: SymbolId, node_id: NodeId) -> ObjectId {
        // 1. On vérifie si l'objet existe déjà (ex: c'est une constante du domaine)
        let id = if let Some(&existing_id) = self.object_symbol_to_id.get(&symbol) {
            existing_id
        } else {
            // 2. Sinon, on crée un nouvel ID basé sur le nombre total d'objets enregistrés
            let new_id = ObjectId::new(self.object_symbol_to_id.len());
            self.object_symbol_to_id.insert(symbol, new_id);
            new_id
        };

        // 3. On lie le NodeId actuel à cet ID pour que try_resolve_object(node_id) fonctionne
        self.object_to_id.insert(node_id, id);
        id
    }

    pub fn register_predicate(&mut self, symbol: NodeId, id: PredicateSymbolId) {
        self.predicate_to_id.insert(symbol, id);
    }

    pub fn register_atom_skeleton(&mut self, symbol: NodeId, id: AtomSkeletonId) {
        self.atom_skeleton_to_id.insert(symbol, id);
    }

    pub fn register_functor(&mut self, symbol: NodeId, id: FunctionSymbolId) {
        self.functor_to_id.insert(symbol, id);
    }

    pub fn register_function_skeleton(&mut self, symbol: NodeId, id: FunctionSkeletonId) {
        self.function_skeleton_to_id.insert(symbol, id);
    }

    pub fn register_task_skeleton(&mut self, symbol: NodeId, id: TaskSkeletonId) {
        self.task_skeleton_to_id.insert(symbol, id);
    }

    pub fn register_task_symbol(&mut self, symbol: NodeId, id: TaskSymbolId) {
        self.task_symbol_to_id.insert(symbol, id);
    }

    pub fn register_preference(&mut self, symbol: NodeId, id: PreferenceSymbolId) {
        self.preference_to_id.insert(symbol, id);
    }

    pub fn register_task_label(&mut self, symbol: SymbolId) -> TaskLabelSymbolId {
        if let Some(&id) = self.task_label_to_id.get(&symbol) {
            return id;
        }

        let id = TaskLabelSymbolId::new(self.task_label_to_id.len());
        self.task_label_to_id.insert(symbol, id);
        self.task_label_id_to_symbol.push(symbol);

        id

    }

    pub fn resolve_task_label(&self, symbol: SymbolId) -> Option<TaskLabelSymbolId> {
        self.task_label_to_id.get(&symbol).copied()
    }

    pub fn try_resolve_task_label(&self, symbol: SymbolId) -> Result<TaskLabelSymbolId, LirError> {
        self.resolve_task_label(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(NodeId::default()))
    }
    /// La méthode dont tu as besoin dans finalize_task_network
    pub fn resolve_task_label_symbol(&self, id: TaskLabelSymbolId) -> SymbolId {
        self.task_label_id_to_symbol[id.as_usize()]
    }

    pub fn get_task_label_symbols(&self) -> SymbolRegistry<TaskLabelSymbolId> {
        let mut registry = SymbolRegistry::new();
        for &symbol in self.task_label_id_to_symbol.iter() {
            registry.insert(symbol);
        }
        registry
    }

    pub fn clear_task_labels(&mut self) {
        self.task_label_to_id.clear();
        self.task_label_id_to_symbol.clear();
    }

    pub fn task_label_symbols_count(&self) -> usize {
        self.task_label_to_id.len()
    }
}
