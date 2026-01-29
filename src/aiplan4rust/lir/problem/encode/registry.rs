//! Encoding Context Management
//!
//! This module defines the `EncodingContext`, the central structure used during
//! the logical encoding pass. It links syntactic declarations (AST) to their
//! resolved intermediate representations (LIR) and manages symbol visibility.

use std::collections::HashMap;
use crate::aiplan4rust::lang::{FunctionID, ObjectID, PredicateID, StringID, Type, TypeID, VariableID};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::semantic::symbol::{Symbol, SymbolKind};
use crate::aiplan4rust::semantic::symbol_table::SymbolTable;
use crate::aiplan4rust::syntax::tree::NodeId;

/// Context used during the encoding of actions, methods, and expressions.
///
/// This structure acts as a bridge between the semantic analysis and the LIR.
/// It carries the necessary mappings to resolve names into indices.
pub struct EncodingContext {

    /// **The Symbol Table**: A reference to the semantic table containing
    /// identifiers for the current scope (e.g., action parameters, constants).
    symbol_table: SymbolTable,

    /// **Type Mapping**: Links a semantic `Type` structure (primitive or union)
    /// to its unique index in the LIR.
    type_to_id: HashMap<Symbol, TypeID>,

    /// **Predicate Mapping**: Links a predicate's logical `Symbol`
    /// to its unique positional index in the LIR.
    predicate_to_id: HashMap<Symbol, PredicateID>,

    /// **Function Mapping**: Links a function's logical `Symbol`
    /// to its unique index in the LIR.
    function_to_id: HashMap<Symbol, FunctionID>,

    /// **Object Mapping**: Links a logical `Symbol` (either a global Constant
    /// from the domain or an Object from the problem) to its unique index.
    object_to_id: HashMap<Symbol, ObjectID>,

    /// **Variable Mapping**: Links a variable's declaration `NodeId` (from AST)
    /// to its local `VariableID` index (0, 1, 2...).
    /// This handles local scope (actions, forall, exists) without naming conflicts.
    variable_to_id: HashMap<NodeId, VariableID>,
}

impl EncodingContext {
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
            predicate_to_id : HashMap::new(),
            function_to_id : HashMap::new(),
            variable_to_id : HashMap::new(),
        }
    }

    /// Returns the symbol table for identifier resolution.
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }



    pub fn get_type_id(&self, ty: &Type<StringID>) -> Option<Vec<TypeID>> {
        let mut ids = Vec::new();
        for t in ty.members() {
            let type_symbol = Symbol::new(*t, SymbolKind::PrimitiveType);
            if let Some(id) = self._get_type_id(&type_symbol) {
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
    pub fn try_get_type_id(&self, ty: &Type<StringID>) -> Result<Vec<TypeID>, LirError> {
        self.get_type_id(ty)
            .ok_or_else(|| LirError::type_binding_failed(ty.clone()))
    }

    /// Récupère l'ID d'un type PRIMITIF uniquement (par son symbole).
    fn _get_type_id(&self, symbol: &Symbol) -> Option<TypeID> {
        self.type_to_id.get(symbol).copied()
    }

    /// Version avec erreur fatale
    fn _try_get_type_id(&self, symbol: &Symbol) -> Result<TypeID, LirError> {
        self._get_type_id(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol.clone()))
    }

    /// Returns the PredicateID associated with a given AST declaration ID, if it exists.
    ///
    /// # Parameters
    ///
    /// - `decl_id`: The unique [`NodeId`] of the predicate declaration in the AST.
    ///
    /// # Returns
    ///
    /// An `Option<PredicateID>` containing the mapped LIR ID, or `None` if not found.
    pub fn get_predicate_id(&self, symbol: &Symbol) -> Option<PredicateID> {
        self.predicate_to_id.get(symbol).copied()
    }

    /// Attempts to retrieve the PredicateID for a declaration, returning a fatal error if missing.
    ///
    /// This is used during the encoding phase when a predicate is expected to be
    /// already registered in the LIR mapping.
    ///
    /// # Parameters
    ///
    /// - `decl_id`: The unique [`NodeId`] of the predicate declaration.
    ///
    /// # Returns
    ///
    /// A `Result<PredicateID, LirError>` containing the ID or a binding error.
    pub fn try_get_predicate_id(&self, symbol: &Symbol) -> Result<PredicateID, LirError> {
        self.get_predicate_id(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol.clone()))
    }

    /// Returns the FunctionID associated with a given AST declaration ID, if it exists.
    ///
    /// # Parameters
    ///
    /// - `decl_id`: The unique [`NodeId`] of the function declaration in the AST.
    ///
    /// # Returns
    ///
    /// An `Option<FunctionID>` containing the mapped LIR ID, or `None` if not found.
    pub fn get_function_id(&self, symbol: &Symbol) -> Option<FunctionID> {
        self.function_to_id.get(symbol).copied()
    }

    /// Attempts to retrieve the FunctionID for a declaration, returning a fatal error if missing.
    ///
    /// # Parameters
    ///
    /// - `symbol`: The unique [`NodeId`] of the function declaration.
    ///
    /// # Returns
    ///
    /// A `Result<FunctionID, LirError>` containing the ID or a binding error.
    pub fn try_get_function_id(&self, symbol: &Symbol) -> Result<FunctionID, LirError> {
        self.get_function_id(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol.clone()))
    }

    /// This covers both global Constants (from the domain) and Objects (from the problem).
    ///
    /// # Parameters
    ///
    /// - `symbol`: The unique [`Symbol`] representing the constant or object.
    ///
    /// # Returns
    ///
    /// An `Option<ObjectID>` containing the mapped LIR ID, or `None` if not found.
    pub fn get_object_id(&self, symbol: &Symbol) -> Option<ObjectID> {
        self.object_to_id.get(symbol).copied()
    }

    /// Attempts to retrieve the ObjectID for a symbol, returning a fatal error if missing.
    ///
    /// This is used during expression encoding when a constant or object reference
    /// is expected to be already registered in the LIR.
    ///
    /// # Parameters
    ///
    /// - `symbol`: The [`Symbol`] to resolve.
    ///
    /// # Returns
    ///
    /// A `Result<ObjectID, LirError>` containing the ID or an `ObjectNotFound` error.
    pub fn try_get_object_id(&self, symbol: &Symbol) -> Result<ObjectID, LirError> {
        self.get_object_id(symbol)
            .ok_or_else(|| LirError::symbol_binding_failed(symbol.clone()))
    }

    /// Enregistre une variable avec un ID fourni de l'extérieur.
    /// (L'ID vient par exemple de l'indexation des enfants dans l'AST)
    pub fn register_variable(&mut self, decl_id: NodeId, var_id: VariableID) {
        self.variable_to_id.insert(decl_id, var_id);
    }

    /// Récupère l'ID associé au NodeId de déclaration.
    pub fn get_variable_id(&self, decl_id: NodeId) -> Option<VariableID> {
        self.variable_to_id.get(&decl_id).copied()
    }

    /// Tries to retrieve the VariableID associated with a declaration NodeId.
    ///
    /// # Errors
    /// Returns `LirError::VariableNotFound` if the NodeId is not registered,
    /// which usually indicates a logic error in the encoder or a missing registration
    /// during the action header or quantifier processing.
    pub fn try_get_variable_id(&self, decl_id: NodeId) -> Result<VariableID, LirError> {
        self.variable_to_id
            .get(&decl_id)
            .copied()
            .ok_or_else(|| LirError::variable_not_found(decl_id))
    }

    /// Vide la map pour la prochaine action.
    pub fn clear_variables(&mut self) {
        self.variable_to_id.clear();
    }

    /// Registers a new Type with a pre-assigned TypeID.
    pub fn register_type(&mut self, symbol: Symbol, id: TypeID) {
        self.type_to_id.insert(symbol, id);
    }

    /// Registers a new object symbol with a pre-assigned ObjectID.
    pub fn register_object(&mut self, symbol: Symbol, id: ObjectID) {
        self.object_to_id.insert(symbol, id);
    }

    /// Registers a new Predicate symbol with a pre-assigned PredicateID.
    pub fn register_predicate(&mut self, symbol: Symbol, id: PredicateID) {
        self.predicate_to_id.insert(symbol, id);
    }

    /// Registers a new Function symbol with a pre-assigned FunctionID.
    pub fn register_function(&mut self, symbol: Symbol, id: FunctionID) {
        self.function_to_id.insert(symbol, id);
    }
}
