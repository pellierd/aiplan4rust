//! This module defines the tree data structures for representing **lifted** syntax problems
//! in hierarchical task network (HTN) and classical syntax domains.
//!
//! The primary struct [`Problem`] models a **lifted syntax problem**, meaning
//! that actions, methods, predicates, and tasks are represented with parameters (variables)
//! rather than fully instantiated ground elements.
//!
//! It captures all components necessary to specify a syntax problem instance at
//! the lifted level:
//!
//! - The **domain and problem identifiers** (`Ident`).
//! - The set of **requirements** (features used in the domain).
//! - The **types**, **constants**, and **objects** defining the domain vocabulary.
//! - The **lifted predicates** and **functions**, expressed as atomic formula skeletons.
//! - The **lifted actions** and **methods**, representing parametrized operators and HTN methods.
//! - The **lifted tasks**, forming the task skeletons for hierarchical syntax.
//! - The **initial state** and **goal conditions** expressed as symbolic logic (`Expr`).
//! - The **global domain constraints** and **problem-specific constraints**.
//! - The **metric and length specifications** for optimization and bounding.
//! - The **initial task network**, describing the starting point of hierarchical tasks.
//!
//! This lifted representation enables symbolic reasoning and efficient syntax
//! by deferring grounding (instantiation) to a later phase.
//!
//! The module supports serialization/deserialization with `serde` for persistence and interoperability.
//!
//! # Example
//! ```
//! use aiplan4rust::lir::problem::Problem;
//! use aiplan4rust::lang::Ident;
//!
//! let mut problem = Problem::default();
//! problem.set_domain_name(Ident::new("blocksworld"));
//! problem.set_problem_name(Ident::new("tower"));
//!
//! assert_eq!(problem.domain_name().as_str(), "blocksworld");
//! assert_eq!(problem.problem_name().as_str(), "tower");
//! ```
//!
//! This module is essential for representing lifted HTN and classical syntax problems
//! before grounding and solving.

use crate::aiplan4rust::interner::{InternerError, SymbolInterner};
use crate::aiplan4rust::lang::{
    ActionSymbolId, AtomSkeletonId, DerivedPredicateDefId, FunctionSkeletonId, FunctionSymbolId,
    MethodSymbolId, ObjectId, PredicateSymbolId, PreferenceSymbolId, Requirement, SymbolId,
    TaskSkeletonId, TaskSymbolId, Type, TypeId, TypedList, TypedSymbol,
};
use crate::aiplan4rust::lir::store::expr::{ExprId, ExprStore};
use crate::aiplan4rust::lir::store::problem::atomic_skeleton::{
    AtomicFormulaSkeleton, AtomicFunctionSkeleton, AtomicTaskSkeleton,
};
use crate::aiplan4rust::lir::store::problem::SymbolRegistry;
use crate::aiplan4rust::lir::store::problem::{
    ActionDef, DerivedPredicateDef, DomainDef, InitialTaskNetwork, MethodDef, ProblemDef,
};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::serialization::serde::SerdeSerializable;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Problem {
    /// Interner for efficient string storage and deduplication.
    interner: SymbolInterner,

    store: ExprStore,

    /// The symbolic name of the planning domain.
    domain_name: SymbolId,
    /// The symbolic name of the planning problem instance.
    problem_name: SymbolId,
    /// Set of PDDL/HDDL requirements (e.g., :strips, :typing, :htn).
    requirements: HashSet<Requirement>,

    // --- SYMBOL TABLES (Identity Management) ---
    /// Map between typing names and their internal IDs.
    type_symbols: SymbolRegistry<TypeId>,
    /// Map between object names and their internal IDs.
    object_symbols: SymbolRegistry<ObjectId>,
    /// Map between predicate names and their internal IDs.
    predicate_symbols: SymbolRegistry<PredicateSymbolId>,
    /// Map between function (functor) names and their internal IDs.
    function_symbols: SymbolRegistry<FunctionSymbolId>,
    /// Map between HTN task names and their internal IDs.
    task_symbols: SymbolRegistry<TaskSymbolId>,
    // Map between Action names and their internal IDs.
    action_symbols: SymbolRegistry<ActionSymbolId>,
    /// Map between Method names and their internal IDs.
    method_symbols: SymbolRegistry<MethodSymbolId>,
    /// Map between Preference names (labels) and their internal IDs.
    preference_symbols: SymbolRegistry<PreferenceSymbolId>,

    // --- DEFINITIONS (Lifted Structure / Skeletons) ---
    /// List of typing definitions, including hierarchy (parent-child relations).
    type_defs: TypedList<TypeId, TypeId>,
    /// List of objects defined in the domain or problem, associated with their types.
    object_defs: TypedList<ObjectId, TypeId>,
    /// Signatures of all predicates (name and typed parameters).
    predicate_defs: Vec<AtomicFormulaSkeleton>,
    /// Signatures of all functions (name, typed parameters, and return typing).
    function_defs: Vec<AtomicFunctionSkeleton>,
    /// Signatures of all abstract tasks for HTN planning.
    task_defs: Vec<AtomicTaskSkeleton>,
    /// List of preference definitions (the logical formula associated with a preference label).
    preference_defs: Vec<ExprId>,

    /// Index marking the boundary between domain constants and problem-specific objects.
    constant_offset: usize,

    // --- LOGIC & ACTIONS ---
    /// Predicates whose truth value is derived from other facts via axioms.
    derived_predicate_defs: Vec<DerivedPredicateDef>,
    predicate_derivations: Vec<Vec<DerivedPredicateDefId>>,

    /// Operators that can change the state of the world.
    action_defs: Vec<ActionDef>,

    /// HTN Methods describing how to decompose abstract tasks into subtasks.
    method_defs: Vec<MethodDef>,

    // --- PROBLEM INSTANCE SPECIFICS ---
    /// Initial state description (facts and initial functional values).
    init: ExprId,

    /// Target state or condition to be satisfied.
    goal: ExprId,

    /// Global constraints defined at the domain level.
    domain_constraints: ExprId,

    /// Constraints specific to this problem instance.
    problem_constraints: ExprId,

    /// Optimization objective (e.g., minimize plan-length or total-cost).
    metric_spec: ExprId,

    /// Specification for plan length (deprecated since PDDL 2.1).
    length_spec: ExprId,

    /// The top-level task hierarchy to decompose in HTN problems.
    initial_task_network: InitialTaskNetwork,
}

#[allow(dead_code)]
impl Problem {
    /// Creates a new empty `PlanningProblem` with default identifiers
    /// and no requirements, types, constants, predicates, functions, or actions.
    ///
    /// # Example
    ///
    /// ```
    /// use aiplan4rust::aiplan4rust::lir::problem::LiftedProblem;
    /// let problem = LiftedProblem::new();
    /// assert!(problem.action_defs().is_empty());
    /// assert!(problem.type_symbols().is_empty());
    /// ```
    pub(crate) fn new(requirements: HashSet<Requirement>) -> Self {
        Self {
            interner: SymbolInterner::new(),
            store: ExprStore::default(),
            domain_name: SymbolId::default(),
            problem_name: SymbolId::default(),
            requirements,
            type_symbols: SymbolRegistry::new(),
            type_defs: TypedList::new(),
            object_symbols: SymbolRegistry::new(),
            object_defs: TypedList::new(),
            constant_offset: 0,
            predicate_symbols: SymbolRegistry::new(),
            predicate_defs: Vec::new(),
            function_symbols: SymbolRegistry::new(),
            function_defs: Vec::new(),
            task_symbols: SymbolRegistry::new(),
            task_defs: Vec::new(),
            preference_symbols: SymbolRegistry::new(),
            preference_defs: Vec::new(),
            derived_predicate_defs: Vec::new(),
            predicate_derivations: Vec::new(),
            action_defs: Vec::new(),
            action_symbols: SymbolRegistry::new(),
            method_defs: Vec::new(),
            method_symbols: SymbolRegistry::new(),
            init: ExprId::EMPTY_AND,
            goal: ExprId::EMPTY_OR,
            domain_constraints: ExprId::EMPTY_AND,
            problem_constraints: ExprId::EMPTY_AND,
            metric_spec: ExprId::NONE,
            length_spec: ExprId::NONE,
            initial_task_network: InitialTaskNetwork::default(), // Add for HDDL
        }
    }

    /// Returns a reference to the string interner used by the problem.
    pub fn interner(&self) -> &SymbolInterner {
        &self.interner
    }

    /// Returns a mutable reference to the string interner.
    pub fn interner_mut(&mut self) -> &mut SymbolInterner {
        &mut self.interner
    }

    /// Replaces the current interner with a new one.
    ///
    /// # Arguments
    /// * `interner` - The new `StringInterner` to use.
    pub fn set_interner(&mut self, interner: SymbolInterner) {
        self.interner = interner;
    }

    /// Takes ownership of the interner, leaving an empty one in its place.
    ///
    /// This is particularly useful for transferring resources from a lifted
    /// problem to a grounded one without cloning.
    pub fn take_interner(&mut self) -> SymbolInterner {
        std::mem::take(&mut self.interner)
    }

    /// Returns a reference to the expression store used by the problem.
    pub fn store(&self) -> &ExprStore {
        &self.store
    }

    /// Returns a mutable reference to the expression store.
    /// Useful for operations that need to add or normalize expressions.
    pub fn store_mut(&mut self) -> &mut ExprStore {
        &mut self.store
    }

    /// Replaces the current store with a new one.
    ///
    /// # Arguments
    /// * `store` - The new `ExprStore` to use.
    pub fn set_store(&mut self, store: ExprStore) {
        self.store = store;
    }

    /// Takes ownership of the store, leaving an empty one in its place.
    ///
    /// This is crucial for moving the optimized IR to the next stage (grounding)
    /// without performing expensive deep clones of the hash-consing tables.
    pub fn take_store(&mut self) -> ExprStore {
        std::mem::take(&mut self.store)
    }

    /// Returns the ID of the domain name.
    ///
    /// # Returns
    /// A [`SymbolId`] representing the unique identifier of the domain name.
    pub fn domain_name(&self) -> SymbolId {
        self.domain_name
    }

    /// Sets the domain name ID after verifying its existence.
    ///
    /// # Arguments
    /// * `id` - The [`SymbolId`] to be assigned as the new domain name.
    ///
    /// # Returns
    /// * `Ok(())` if the ID is valid and exists within the interner.
    /// * `Err(InternerError)` if the ID cannot be resolved.
    pub fn set_domain_name(&mut self, id: SymbolId) -> Result<(), InternerError> {
        self.interner.try_resolve_symbol(id)?;
        self.domain_name = id;
        Ok(())
    }

    /// Resolves the domain name ID into a string slice.
    ///
    /// # Returns
    /// * `Ok(&str)` containing the human-readable name of the domain.
    /// * `Err(InternerError)` if the stored ID is invalid or cannot be resolved.
    pub fn domain_name_str(&self) -> Result<&str, InternerError> {
        self.interner.try_resolve_symbol(self.domain_name)
    }

    /// Returns the ID of the problem name.
    ///
    /// # Returns
    /// A [`SymbolId`] representing the unique identifier of the problem instance.
    pub fn problem_name(&self) -> SymbolId {
        self.problem_name
    }

    /// Sets the problem name ID after verifying its existence.
    ///
    /// # Arguments
    /// * `id` - The [`SymbolId`] to be assigned as the new problem name.
    ///
    /// # Returns
    /// * `Ok(())` if the ID is valid.
    /// * `Err(InternerError)` if the ID is not recognized by the interner.
    pub fn set_problem_name(&mut self, id: SymbolId) -> Result<(), InternerError> {
        self.interner.try_resolve_symbol(id)?;
        self.problem_name = id;
        Ok(())
    }

    /// Resolves the problem name ID into a string slice.
    ///
    /// # Returns
    /// * `Ok(&str)` containing the human-readable name of the problem.
    /// * `Err(InternerError)` if the stored ID cannot be resolved.
    pub fn problem_name_str(&self) -> Result<&str, InternerError> {
        self.interner.try_resolve_symbol(self.problem_name)
    }

    /// Returns a reference to the set of requirements defined for this problem.
    ///
    /// # Returns
    /// A reference to a [`HashSet<Requirement>`].
    pub fn requirements(&self) -> &HashSet<Requirement> {
        &self.requirements
    }

    /// Returns a mutable reference to the set of requirements.
    ///
    /// # Returns
    /// A mutable reference to a [`HashSet<Requirement>`].
    pub fn requirements_mut(&mut self) -> &mut HashSet<Requirement> {
        &mut self.requirements
    }

    /// Adds a single requirement to the problem.
    ///
    /// # Arguments
    /// * `requirement` - The [`Requirement`] to enable.
    pub fn add_requirement(&mut self, requirement: Requirement) {
        self.requirements.insert(requirement);
    }

    /// Adds multiple requirements from an iter.
    ///
    /// # Arguments
    /// * `iter` - An iter yielding [`Requirement`] items.
    pub fn add_requirements<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Requirement>,
    {
        self.requirements.extend(iter);
    }

    /// Checks if a specific requirement is enabled.
    ///
    /// # Arguments
    /// * `requirement` - The [`Requirement`] to check.
    ///
    /// # Returns
    /// `true` if the requirement is present in the set, `false` otherwise.
    pub fn is_required(&self, requirement: Requirement) -> bool {
        self.requirements.contains(&requirement)
    }

    /// Replaces the current requirements with a new set.
    ///
    /// # Arguments
    /// * `new_requirements` - The new [`HashSet<Requirement>`] to use.
    pub fn set_requirements(&mut self, new_requirements: HashSet<Requirement>) {
        self.requirements = new_requirements;
    }

    /// Takes ownership of the requirements, leaving an empty set in its place.
    ///
    /// This is typically used during the grounding process to transfer ownership
    /// without cloning the data.
    ///
    /// # Returns
    /// The [`HashSet<Requirement>`] previously owned by the problem.
    pub fn take_requirements(&mut self) -> HashSet<Requirement> {
        std::mem::take(&mut self.requirements)
    }

    /// Returns a read-only reference to the typing symbol table.
    ///
    /// # Returns
    /// A reference to the [`SymbolRegistry<TypeId>`]. To add new symbols,
    /// use [`add_type_symbol`] to maintain internal consistency.
    pub fn type_symbols(&self) -> &SymbolRegistry<TypeId> {
        &self.type_symbols
    }

    /// Adds a new typing symbol and ensures its definition exists.
    ///
    /// This method preserves the invariant that the index of the typing in `type_defs`
    /// matches its [`TypeId`].
    ///
    /// # Arguments
    /// * `symbol` - The [`SymbolId`] representing the typing name.
    ///
    /// # Returns
    /// The unique [`TypeId`] assigned to this typing.
    pub fn add_type_symbol(&mut self, symbol: SymbolId) -> TypeId {
        let id = self.type_symbols.insert(symbol);
        let idx = id.as_usize();

        // Synchronisation : on crée une définition par défaut si elle n'existe pas encore.
        if idx >= self.type_defs.len() {
            let typed = TypedSymbol::new(id, Type::default());
            self.type_defs.push(typed);
        }
        id
    }

    /// Takes ownership of the typing symbols table, leaving an empty one in its place.
    ///
    /// This is used to transfer the symbol data to another structure (like a Grounder)
    /// without copying the underlying strings.
    ///
    /// # Returns
    /// The [`SymbolRegistry<TypeId>`] previously owned by the problem.
    pub fn take_type_symbols(&mut self) -> SymbolRegistry<TypeId> {
        std::mem::take(&mut self.type_symbols)
    }

    /// Returns a slice of all typing definitions.
    ///
    /// # Returns
    /// A slice of [`TypedSymbol<TypeId, TypeId>`].
    pub fn type_defs(&self) -> &TypedList<TypeId, TypeId> {
        &self.type_defs
    }

    /// Returns a mutable slice of all typing definitions.
    ///
    /// This is particularly useful for transformation logic, such as
    /// flattening typing hierarchies.
    ///
    /// # Returns
    /// A mutable slice of [`TypedSymbol<TypeId, TypeId>`].
    pub fn type_defs_mut(&mut self) -> &mut TypedList<TypeId, TypeId> {
        &mut self.type_defs
    }

    /// Checks if any typing definitions have been registered.
    ///
    /// # Returns
    /// `true` if the internal list of typing definitions is not empty.
    pub fn has_type_defs(&self) -> bool {
        !self.type_defs.is_empty()
    }

    /// Updates a pre-allocated typing definition with its complete specification.
    ///
    /// This method replaces the placeholder definition at the index corresponding
    /// to the symbol's ID. It is typically used during the second pass of LIR
    /// construction or during a typing hierarchy flattening phase.
    ///
    /// # Arguments
    /// * `ty` - The complete [`TypedSymbol`] definition to be stored.
    ///
    /// # Returns
    /// * `Ok(TypeID)` - The ID of the successfully updated typing.
    /// * `Err(LirError::TypeDefinitionOrphan)` - If the ID's index exceeds the
    ///   allocated definitions, indicating the symbol was never registered via `add_type_symbol`.
    pub fn add_type_defs(&mut self, ty: TypedSymbol<TypeId, TypeId>) -> Result<TypeId, LirError> {
        let id = ty.symbol();
        let idx = id.as_usize();

        // Safety check: The ID must have been pre-allocated in the definitions vector.
        // If the index is out of bounds, it means the definition is an "orphan"
        // without a corresponding registered symbol.
        if idx >= self.type_defs.len() {
            return Err(LirError::type_definition_orphan(id));
        }

        self.type_defs[idx] = ty;
        Ok(id)
    }

    /// Takes ownership of the typing definitions, leaving an empty vector in its place.
    ///
    /// # Returns
    /// The [`Vec<TypedSymbol<TypeId, TypeId>>`] previously owned by the problem.
    pub fn take_type_defs(&mut self) -> TypedList<TypeId, TypeId> {
        std::mem::take(&mut self.type_defs)
    }

    /// Returns a reference to a typing definition if it exists.
    ///
    /// # Arguments
    /// * `id` - The [`TypeId`] of the definition to retrieve.
    ///
    /// # Returns
    /// An `Option<&TypedSymbol>` which is `None` if the ID is out of bounds.
    pub fn get_type_def(&self, id: TypeId) -> Option<&TypedSymbol<TypeId, TypeId>> {
        self.type_defs.get(id.as_usize())
    }

    /// Returns a mutable reference to a typing definition if it exists.
    ///
    /// # Arguments
    /// * `id` - The [`TypeId`] of the definition to retrieve.
    ///
    /// # Returns
    /// An `Option<&mut TypedSymbol>` which is `None` if the ID is out of bounds.
    pub fn get_type_def_mut(&mut self, id: TypeId) -> Option<&mut TypedSymbol<TypeId, TypeId>> {
        self.type_defs.get_mut(id.as_usize())
    }

    /// Attempts to retrieve a typing definition or returns an error.
    ///
    /// # Arguments
    /// * `id` - The [`TypeId`] of the definition to retrieve.
    ///
    /// # Returns
    /// * `Ok(&TypedSymbol)` on success.
    /// * `Err(LirError::TypeDefinitionOrphan)` if the definition does not exist.
    pub fn try_get_type(&self, id: TypeId) -> Result<&TypedSymbol<TypeId, TypeId>, LirError> {
        self.get_type_def(id)
            .ok_or_else(|| LirError::type_definition_orphan(id))
    }

    /// Attempts to retrieve a mutable typing definition or returns an error.
    ///
    /// # Arguments
    /// * `id` - The [`TypeId`] of the definition to retrieve.
    ///
    /// # Returns
    /// * `Ok(&mut TypedSymbol)` on success.
    /// * `Err(LirError::TypeDefinitionOrphan)` if the definition does not exist.
    pub fn try_get_type_mut(
        &mut self,
        id: TypeId,
    ) -> Result<&mut TypedSymbol<TypeId, TypeId>, LirError> {
        self.get_type_def_mut(id)
            .ok_or_else(|| LirError::type_definition_orphan(id))
    }

    /// Returns a read-only reference to the object symbol table.
    pub fn object_symbols(&self) -> &SymbolRegistry<ObjectId> {
        &self.object_symbols
    }

    /// Adds a new object symbol and ensures a corresponding definition placeholder exists.
    ///
    /// If the object is new, a [`TypedSymbol`] with `Type::default()` is added to
    /// `object_defs` to keep the table and definitions synchronized.
    pub fn add_object_symbol(&mut self, symbol: SymbolId) -> ObjectId {
        let id = self.object_symbols.insert(symbol);
        let idx = id.as_usize();
        if idx >= self.object_defs.len() {
            // Placeholder definition using the default typing (usually 'object')
            self.object_defs.push(TypedSymbol::new(id, Type::default()));
        }
        id
    }

    /// Takes ownership of the object symbol table, leaving an empty one in its place.
    ///
    /// # Returns
    /// The [`SymbolRegistry<ObjectId>`] previously owned by the problem.
    pub fn take_object_symbols(&mut self) -> SymbolRegistry<ObjectId> {
        std::mem::take(&mut self.object_symbols)
    }

    /// Returns a slice of all object definitions.
    ///
    /// # Returns
    /// A slice of [`TypedSymbol<ObjectId, TypeId>`].
    pub fn object_defs(&self) -> &TypedList<ObjectId, TypeId> {
        &self.object_defs
    }

    /// Returns a mutable slice of all object definitions.
    ///
    /// # Returns
    /// A mutable slice of [`TypedSymbol<ObjectId, TypeId>`].
    pub fn object_defs_mut(&mut self) -> &mut TypedList<ObjectId, TypeId> {
        &mut self.object_defs
    }

    /// Checks if any object definitions have been registered.
    ///
    /// # Returns
    /// `true` if the internal list of object definitions is not empty.
    pub fn has_object_defs(&self) -> bool {
        !self.object_defs.is_empty()
    }

    /// Updates a pre-allocated object definition with its complete specification.
    ///
    /// This method replaces the placeholder definition at the index corresponding
    /// to the object's ID. It ensures that the object was previously registered
    /// via `add_object_symbol` during the initial parsing or declaration phase.
    ///
    /// # Arguments
    /// * `obj` - The complete [`TypedSymbol`] definition for the object.
    ///
    /// # Returns
    /// * `Ok(ObjectID)` - The ID of the successfully updated object.
    /// * `Err(LirError::ObjectDefinitionOrphan)` - If the ID's index exceeds the
    ///   allocated definitions, indicating the symbol was never registered.
    pub fn add_object_def(
        &mut self,
        obj: TypedSymbol<ObjectId, TypeId>,
    ) -> Result<ObjectId, LirError> {
        let id = obj.symbol();
        let idx = id.as_usize();

        // Safety check: The ID must have been pre-allocated in the definitions vector.
        // If the index is out of bounds, this definition has no corresponding
        // symbol entry, making it an "orphan".
        if idx >= self.object_defs.len() {
            return Err(LirError::object_definition_orphan(id));
        }

        self.object_defs[idx] = obj;
        Ok(id)
    }

    /// Takes ownership of the object definitions, leaving an empty vector in its place.
    ///
    /// # Returns
    /// The [`Vec<TypedSymbol<ObjectId, TypeId>>`] previously owned by the problem.
    pub fn take_object_defs(&mut self) -> TypedList<ObjectId, TypeId> {
        std::mem::take(&mut self.object_defs)
    }

    /// Returns a reference to an object definition if it exists.
    pub fn get_object_def(&self, id: ObjectId) -> Option<&TypedSymbol<ObjectId, TypeId>> {
        self.object_defs.get(id.as_usize())
    }

    /// Returns a mutable reference to an object definition if it exists.
    pub fn get_object_def_mut(
        &mut self,
        id: ObjectId,
    ) -> Option<&mut TypedSymbol<ObjectId, TypeId>> {
        self.object_defs.get_mut(id.as_usize())
    }

    /// Attempts to retrieve an object definition or returns an error.
    ///
    /// # Errors
    /// Returns `LirError::ObjectDefinitionOrphan` if the ID is not registered.
    pub fn try_get_object(&self, id: ObjectId) -> Result<&TypedSymbol<ObjectId, TypeId>, LirError> {
        self.get_object_def(id)
            .ok_or_else(|| LirError::object_definition_orphan(id))
    }

    /// Attempts to retrieve a mutable object definition or returns an error.
    ///
    /// # Errors
    /// Returns `LirError::ObjectDefinitionOrphan` if the ID is not registered.
    pub fn try_get_object_mut(
        &mut self,
        id: ObjectId,
    ) -> Result<&mut TypedSymbol<ObjectId, TypeId>, LirError> {
        self.get_object_def_mut(id)
            .ok_or_else(|| LirError::object_definition_orphan(id))
    }

    /// Checks if there are any objects defined specifically in the problem
    /// (excluding domain constants).
    pub fn has_problem_object_defs(&self) -> bool {
        self.object_defs.len() > self.constant_offset
    }

    /// Returns a slice of the definitions that are considered Domain Constants.
    pub fn domain_constant_def(&self) -> &[TypedSymbol<ObjectId, TypeId>] {
        // On utilise as_slice() de TypedList pour obtenir le slice complet,
        // puis on applique le range.
        &self.object_defs.as_slice()[..self.constant_offset]
    }

    /// Checks if the domain has any constant definitions.
    pub fn has_domain_constant_defs(&self) -> bool {
        // Cette logique reste identique : l'offset indique la frontière.
        self.constant_offset > 0
    }

    /// Returns a slice of the definitions that are considered Problem Objects.
    pub fn problem_object_def(&self) -> &[TypedSymbol<ObjectId, TypeId>] {
        // Slice à partir de l'offset jusqu'à la fin.
        &self.object_defs.as_slice()[self.constant_offset..]
    }

    /// Sets the constant offset based on the current number of symbols.
    ///
    /// This should be called exactly once, after parsing the Domain constants
    /// but before parsing the Problem objects.
    pub fn set_constant_offset(&mut self) {
        self.constant_offset = self.object_symbols.len();
    }

    /// Returns the number of constants defined in the domain.
    ///
    /// Objects with an ID smaller than this offset are considered constants
    /// (global), while IDs equal to or greater are problem-specific objects.
    pub fn constant_offset(&self) -> usize {
        self.constant_offset
    }

    /// Returns true if the given ID refers to a Domain Constant.
    pub fn is_constant(&self, id: ObjectId) -> bool {
        id.as_usize() < self.constant_offset
    }

    /// Returns true if the given ID refers to a Problem Object.
    pub fn is_object(&self, id: ObjectId) -> bool {
        let idx = id.as_usize();
        idx >= self.constant_offset && idx < self.object_defs.len()
    }

    /// Returns a read-only reference to the predicate symbol table.
    ///
    /// This table maps [`PredicateSymbolId`]s to their string identifiers.
    pub fn predicate_symbols(&self) -> &SymbolRegistry<PredicateSymbolId> {
        &self.predicate_symbols
    }

    /// Insère un nom de prédicat dans la table des symboles et retourne son ID.
    /// Utile pour obtenir l'identité du prédicat avant de construire sa structure (squelette).
    pub fn add_predicate_symbol(&mut self, symbol: SymbolId) -> PredicateSymbolId {
        self.predicate_symbols.insert(symbol)
    }

    /// Takes ownership of the predicate symbol table, leaving an empty one in its place.
    ///
    /// Use this when moving the problem representation to a grounder or exporter
    /// that needs to own the symbol mapping.
    ///
    /// # Returns
    /// The [`SymbolRegistry<PredicateSymbolId>`] previously owned by the problem.
    pub fn take_predicate_symbols(&mut self) -> SymbolRegistry<PredicateSymbolId> {
        std::mem::take(&mut self.predicate_symbols)
    }

    /// Returns a slice of all atomic formula skeletons in the problem.
    pub fn predicate_defs(&self) -> &[AtomicFormulaSkeleton] {
        &self.predicate_defs
    }

    /// Returns a mutable slice of all atomic formula skeletons in the problem.
    pub fn predicate_defs_mut(&mut self) -> &mut [AtomicFormulaSkeleton] {
        &mut self.predicate_defs
    }

    /// Checks if any predicate definitions have been registered.
    pub fn has_predicate_defs(&self) -> bool {
        !self.predicate_defs.is_empty()
    }

    /// Adds a new predicate definition and registers its symbol.
    ///
    /// This method performs a single-pass registration: it inserts the symbol
    /// and pushes the skeleton simultaneously.
    pub fn add_predicate_def(&mut self, atom_skeleton: AtomicFormulaSkeleton) -> AtomSkeletonId {
        let symbol_id = atom_skeleton.symbol();

        // SAFETY: Check that the predicate name was registered before adding the structure
        debug_assert!(
            symbol_id.as_usize() < self.predicate_symbols.len(),
            "Predicate symbol {:?} is not registered. Register the symbol before adding the skeleton.",
            symbol_id
        );

        let skeleton_id = AtomSkeletonId::from(self.predicate_defs.len());
        self.predicate_defs.push(atom_skeleton);
        skeleton_id
    }

    /// Takes ownership of the predicate definitions, leaving an empty vector in its place.
    pub fn take_predicate_defs(&mut self) -> Vec<AtomicFormulaSkeleton> {
        std::mem::take(&mut self.predicate_defs)
    }

    /// Returns a reference to a predicate definition if it exists.
    ///
    /// # Arguments
    /// * `id` - The [`AtomSkeletonId`] of the predicate to retrieve.
    pub fn get_predicate_def(&self, id: AtomSkeletonId) -> Option<&AtomicFormulaSkeleton> {
        self.predicate_defs.get(id.as_usize())
    }

    /// Returns a mutable reference to a predicate definition if it exists.
    ///
    /// # Arguments
    /// * `id` - The [`AtomSkeletonId`] of the predicate to retrieve.
    pub fn get_predicate_def_mut(
        &mut self,
        id: AtomSkeletonId,
    ) -> Option<&mut AtomicFormulaSkeleton> {
        self.predicate_defs.get_mut(id.as_usize())
    }

    /// Attempts to retrieve a predicate definition or returns an error.
    ///
    /// # Errors
    /// Returns `LirError::PredicateDefinitionOrphan` if the skeleton ID is invalid.
    pub fn try_get_predicate(
        &self,
        id: AtomSkeletonId,
    ) -> Result<&AtomicFormulaSkeleton, LirError> {
        self.get_predicate_def(id)
            .ok_or_else(|| LirError::predicate_definition_orphan(id))
    }

    /// Attempts to retrieve a mutable predicate definition or returns an error.
    ///
    /// # Errors
    /// Returns `LirError::PredicateDefinitionOrphan` if the skeleton ID is invalid.
    pub fn try_get_predicate_mut(
        &mut self,
        id: AtomSkeletonId,
    ) -> Result<&mut AtomicFormulaSkeleton, LirError> {
        self.get_predicate_def_mut(id)
            .ok_or_else(|| LirError::predicate_definition_orphan(id))
    }

    /// Returns a read-only reference to the function symbol table.
    ///
    /// This table maps [`FunctionSymbolId`]s to their string identifiers.
    pub fn function_symbols(&self) -> &SymbolRegistry<FunctionSymbolId> {
        &self.function_symbols
    }

    pub fn add_function_symbol(&mut self, symbol: SymbolId) -> FunctionSymbolId {
        self.function_symbols.insert(symbol)
    }

    /// Takes ownership of the function symbols table, leaving an empty one in its place.
    ///
    /// This is used to move the symbols to the next stage of the pipeline
    /// (e.g., the Grounder) without cloning.
    ///
    /// # Returns
    /// The [`SymbolRegistry<FunctionSymbolId>`] previously owned by the problem.
    pub fn take_function_symbols(&mut self) -> SymbolRegistry<FunctionSymbolId> {
        std::mem::take(&mut self.function_symbols)
    }

    /// Returns a slice containing all registered atomic function skeletons.
    ///
    /// This provides read-only access to the structural definitions of all
    /// functions in the problem.
    ///
    /// # Returns
    /// A slice of [`AtomicFunctionSkeleton`].
    pub fn function_defs(&self) -> &[AtomicFunctionSkeleton] {
        &self.function_defs
    }

    /// Returns a mutable slice of all atomic function skeletons.
    ///
    /// Use this to perform in-place modifications or optimizations on the
    /// structural definitions of functions.
    ///
    /// # Returns
    /// A mutable slice of [`AtomicFunctionSkeleton`].
    pub fn function_defs_mut(&mut self) -> &mut [AtomicFunctionSkeleton] {
        &mut self.function_defs
    }

    /// Checks whether any function definitions have been registered in the problem.
    ///
    /// # Returns
    /// `true` if the internal function definition collection is not empty, `false` otherwise.
    pub fn has_function_defs(&self) -> bool {
        !self.function_defs.is_empty()
    }

    /// Adds a new function definition and registers its functor symbol.
    ///
    /// This method performs a single-pass registration: it inserts the function
    /// name into the symbol table and stores its structural definition (skeleton)
    /// simultaneously.
    ///
    /// # Parameters
    /// * `function`: The [`AtomicFunctionSkeleton`] containing the symbol and argument types.
    ///
    /// # Returns
    /// A tuple consisting of:
    /// 1. [`FunctionSymbolId`]: The unique identifier for the function's symbol (name).
    /// 2. [`FunctionSkeletonId`]: The identifier for the structural definition in the function vector.
    pub fn add_function_def(&mut self, function: AtomicFunctionSkeleton) -> FunctionSkeletonId {
        let symbol_id = function.symbol();

        debug_assert!(
            symbol_id.as_usize() < self.function_symbols.len(),
            "Function symbol {:?} is not registered. Ensure the functor is in the registry.",
            symbol_id
        );

        let skeleton_id = FunctionSkeletonId::from(self.function_defs.len());
        self.function_defs.push(function);
        skeleton_id
    }

    /// Takes ownership of the function definitions, leaving an empty vector in its place.
    ///
    /// This operation is typically used to move data to another stage of the pipeline
    /// (such as a Grounder) without performing expensive data clones.
    ///
    /// # Returns
    /// A [`Vec<AtomicFunctionSkeleton>`] containing all previously registered definitions.
    pub fn take_function_defs(&mut self) -> Vec<AtomicFunctionSkeleton> {
        std::mem::take(&mut self.function_defs)
    }

    /// Returns a reference to a function definition if it exists.
    ///
    /// # Parameters
    /// * `id`: The [`FunctionSkeletonId`] uniquely identifying the function skeleton.
    ///
    /// # Returns
    /// * `Some(&AtomicFunctionSkeleton)`: An immutable reference to the definition if the ID is valid.
    /// * `None`: If the ID does not correspond to any registered definition.
    pub fn get_function_def(&self, id: FunctionSkeletonId) -> Option<&AtomicFunctionSkeleton> {
        self.function_defs.get(id.as_usize())
    }

    /// Returns a mutable reference to a function definition if it exists.
    ///
    /// # Parameters
    /// * `id`: The [`FunctionSkeletonId`] of the function to retrieve.
    ///
    /// # Returns
    /// * `Some(&mut AtomicFunctionSkeleton)`: A mutable reference allowing in-place modification.
    /// * `None`: If the ID is out of bounds.
    pub fn get_function_def_mut(
        &mut self,
        id: FunctionSkeletonId,
    ) -> Option<&mut AtomicFunctionSkeleton> {
        self.function_defs.get_mut(id.as_usize())
    }

    /// Attempts to retrieve a function definition or returns a specialized error.
    ///
    /// # Parameters
    /// * `id`: The target [`FunctionSkeletonId`].
    ///
    /// # Returns
    /// * `Ok(&AtomicFunctionSkeleton)`: The reference to the requested definition.
    /// * `Err(LirError::FunctionDefinitionOrphan)`: If the ID has no associated definition.
    pub fn try_get_function(
        &self,
        id: FunctionSkeletonId,
    ) -> Result<&AtomicFunctionSkeleton, LirError> {
        self.get_function_def(id)
            .ok_or_else(|| LirError::function_definition_orphan(id))
    }

    /// Attempts to retrieve a mutable definition or returns a specialized error.
    ///
    /// # Parameters
    /// * `id`: The target [`FunctionSkeletonId`].
    ///
    /// # Returns
    /// * `Ok(&mut AtomicFunctionSkeleton)`: The requested mutable reference.
    /// * `Err(LirError::FunctionDefinitionOrphan)`: If the ID is invalid, allowing for clean error handling.
    pub fn try_get_function_mut(
        &mut self,
        id: FunctionSkeletonId,
    ) -> Result<&mut AtomicFunctionSkeleton, LirError> {
        self.get_function_def_mut(id)
            .ok_or_else(|| LirError::function_definition_orphan(id))
    }

    /// Returns a read-only reference to the task symbol table.
    ///
    /// This table maps [`TaskSymbolId`]s to their string identifiers,
    /// typically used for HTN (Hierarchical Task Network) tasks.
    pub fn task_symbols(&self) -> &SymbolRegistry<TaskSymbolId> {
        &self.task_symbols
    }

    pub fn add_task_symbol(&mut self, symbol: SymbolId) -> TaskSymbolId {
        self.task_symbols.insert(symbol)
    }

    /// Takes ownership of the task symbol table, leaving an empty one in its place.
    ///
    /// This is used to move task identifiers to the next stage of the compilation
    /// or grounding process without cloning the underlying data.
    ///
    /// # Returns
    /// The [`SymbolRegistry<TaskSymbolId>`] previously owned by the problem.
    pub fn take_task_symbols(&mut self) -> SymbolRegistry<TaskSymbolId> {
        std::mem::take(&mut self.task_symbols)
    }

    /// Returns a slice of all atomic task skeletons in the problem.
    ///
    /// This provides a view into all abstract or primitive task structures
    /// registered in the current problem instance.
    ///
    /// # Returns
    /// A slice of [`AtomicTaskSkeleton`].
    pub fn task_defs(&self) -> &[AtomicTaskSkeleton] {
        &self.task_defs
    }

    /// Returns a mutable slice of all atomic task skeletons in the problem.
    ///
    /// This allows for batch modification of task structures, which is useful
    /// for logic or lifting logic.
    ///
    /// # Returns
    /// A mutable slice of [`AtomicTaskSkeleton`].
    pub fn task_defs_mut(&mut self) -> &mut [AtomicTaskSkeleton] {
        &mut self.task_defs
    }

    /// Checks if any task definitions have been registered.
    ///
    /// # Returns
    /// `true` if there is at least one task definition; `false` otherwise.
    pub fn has_task_defs(&self) -> bool {
        !self.task_defs.is_empty()
    }

    /// Adds a new task definition and registers its task symbol.
    ///
    /// This follows the single-pass registration pattern: the task's name is
    /// added to the symbol table, and its structure is appended to the definitions.
    ///
    /// # Parameters
    /// * `task_skeleton`: The [`AtomicTaskSkeleton`] to be registered.
    ///
    /// # Returns
    /// A tuple containing:
    /// 1. The [`TaskSymbolId`] associated with the task's name.
    /// 2. The [`TaskSkeletonId`] indexing the specific structural definition.
    pub fn add_task_def(&mut self, task_skeleton: AtomicTaskSkeleton) -> TaskSkeletonId {
        let symbol_id = task_skeleton.symbol();

        debug_assert!(
            symbol_id.as_usize() < self.task_symbols.len(),
            "Task symbol {:?} is not registered. Symbols must exist before skeletons.",
            symbol_id
        );

        let task_skeleton_id = TaskSkeletonId::from(self.task_defs.len());
        self.task_defs.push(task_skeleton);
        task_skeleton_id
    }

    /// Takes ownership of the task definitions, leaving an empty vector in its place.
    ///
    /// This is a high-performance, zero-cost operation used to move task data
    /// to the next compilation stage.
    ///
    /// # Returns
    /// The [`Vec<AtomicTaskSkeleton>`] previously owned by the problem.
    pub fn take_task_defs(&mut self) -> Vec<AtomicTaskSkeleton> {
        std::mem::take(&mut self.task_defs)
    }

    /// Returns a reference to a task definition if it exists.
    ///
    /// # Parameters
    /// * `id`: The [`TaskSkeletonId`] to look up.
    ///
    /// # Returns
    /// * `Some(&AtomicTaskSkeleton)`: If the ID is valid.
    /// * `None`: If the ID is out of bounds.
    pub fn get_task_def(&self, id: TaskSkeletonId) -> Option<&AtomicTaskSkeleton> {
        self.task_defs.get(id.as_usize())
    }

    /// Returns a mutable reference to a task definition if it exists.
    ///
    /// # Parameters
    /// * `id`: The [`TaskSkeletonId`] to look up.
    ///
    /// # Returns
    /// * `Some(&mut AtomicTaskSkeleton)`: A mutable reference if the ID is valid.
    /// * `None`: If the ID is out of bounds.
    pub fn get_task_def_mut(&mut self, id: TaskSkeletonId) -> Option<&mut AtomicTaskSkeleton> {
        self.task_defs.get_mut(id.as_usize())
    }

    /// Attempts to retrieve a task definition or returns a specialized error.
    ///
    /// # Parameters
    /// * `id`: The [`TaskSkeletonId`] to retrieve.
    ///
    /// # Returns
    /// * `Ok(&AtomicTaskSkeleton)`: The requested task definition.
    ///
    /// # Errors
    /// Returns [`LirError::TaskDefinitionOrphan`] if the skeleton ID is invalid.
    pub fn try_get_task(&self, id: TaskSkeletonId) -> Result<&AtomicTaskSkeleton, LirError> {
        self.get_task_def(id)
            .ok_or_else(|| LirError::task_definition_orphan(id))
    }

    /// Attempts to retrieve a mutable task definition or returns a specialized error.
    ///
    /// # Parameters
    /// * `id`: The [`TaskSkeletonId`] to retrieve.
    ///
    /// # Returns
    /// * `Ok(&mut AtomicTaskSkeleton)`: The requested mutable task definition.
    ///
    /// # Errors
    /// Returns [`LirError::TaskDefinitionOrphan`] if the skeleton ID is invalid.
    pub fn try_get_task_mut(
        &mut self,
        id: TaskSkeletonId,
    ) -> Result<&mut AtomicTaskSkeleton, LirError> {
        self.get_task_def_mut(id)
            .ok_or_else(|| LirError::task_definition_orphan(id))
    }

    /// Returns a read-only reference to the preference symbol table.
    pub fn preference_symbols(&self) -> &SymbolRegistry<PreferenceSymbolId> {
        &self.preference_symbols
    }

    /// Registers a new preference label and returns its internal ID.
    pub fn add_preference_symbol(&mut self, symbol: SymbolId) -> PreferenceSymbolId {
        self.preference_symbols.insert(symbol)
    }

    /// Takes ownership of the preference symbol table, leaving an empty one in its place.
    pub fn take_preference_symbols(&mut self) -> SymbolRegistry<PreferenceSymbolId> {
        std::mem::take(&mut self.preference_symbols)
    }

    /// Returns a slice of all preference definitions (logical expressions) in the problem.
    pub fn preference_defs(&self) -> &[ExprId] {
        &self.preference_defs
    }

    /// Checks if any preference definitions have been registered.
    pub fn has_preference_defs(&self) -> bool {
        !self.preference_defs.is_empty()
    }

    /// Adds a new preference definition associated with a specific label.
    ///
    /// Note: The `PreferenceSymbolId` should be obtained first via `add_preference_symbol`.
    /// The definition is appended to the vector, maintaining the same index as the symbol.
    pub fn add_preference_def(
        &mut self,
        symbol_id: PreferenceSymbolId,
        condition: ExprId,
    ) -> PreferenceSymbolId {
        // On s'assure que le vecteur de defs reste synchronisé avec le registre de symboles
        // Idéalement, preference_defs[id] correspond au symbole d'ID 'id'.
        let expected_id = self.preference_defs.len();

        debug_assert_eq!(
            symbol_id.as_usize(),
            expected_id,
            "Preference definitions must be added in the same order as their symbols to maintain alignment."
        );

        self.preference_defs.push(condition);
        symbol_id
    }

    /// Takes ownership of the preference definitions, leaving an empty vector.
    pub fn take_preference_defs(&mut self) -> Vec<ExprId> {
        std::mem::take(&mut self.preference_defs)
    }

    /// Returns a reference to a preference expression if it exists.
    pub fn get_preference_def(&self, id: PreferenceSymbolId) -> Option<ExprId> {
        self.preference_defs.get(id.as_usize()).copied()
    }

    /// Attempts to retrieve a preference definition or returns a specialized error.
    ///
    /// # Errors
    /// Returns [`LirError::PreferenceDefinitionOrphan`] if the ID is invalid.
    pub fn try_get_preference(&self, id: PreferenceSymbolId) -> Result<ExprId, LirError> {
        self.get_preference_def(id)
            .ok_or_else(|| LirError::preference_definition_orphan(id))
    }

    /// Returns a reference to the global domain constraints.
    ///
    /// Domain constraints (often defined via `:constraints` in PDDL) represent
    /// logical conditions that must be satisfied by every state or trajectory
    /// within the domain.
    ///
    /// # Returns
    /// A reference to the [`Expr`] representing the constraints.
    pub fn domain_constraints(&self) -> ExprId {
        self.domain_constraints
    }

    /// Sets the global domain constraints.
    ///
    /// This replaces any existing constraints with the provided expression.
    ///
    /// # Parameters
    /// * `constraints`: The new [`Expr`] to be applied as the domain's global constraints.
    pub fn set_domain_constraints(&mut self, constraints: ExprId) {
        self.domain_constraints = constraints;
    }

    /// Returns a slice of all lifted derived predicates in the problem.
    ///
    /// Derived predicates (axioms) define rules used to infer new facts
    /// from existing ones in a state. This provides read-only access to
    /// the collection of these rules.
    ///
    /// # Returns
    /// A slice of [`LiftedDerivedPredicate`].
    pub fn derived_predicate_defs(&self) -> &[DerivedPredicateDef] {
        &self.derived_predicate_defs
    }

    /// Returns a mutable slice of all derived predicates in the problem.
    ///
    /// This allows for in-place modifications, such as simplifying the
    /// preconditions of the derived rules or applying optimizations.
    ///
    /// # Returns
    /// A mutable slice of [`LiftedDerivedPredicate`].
    pub fn derived_predicate_defs_mut(&mut self) -> &mut [DerivedPredicateDef] {
        &mut self.derived_predicate_defs
    }

    /// Adds a new derived predicate to the problem.
    ///
    /// This appends the provided [`LiftedDerivedPredicate`] to the
    /// internal collection of derivation rules.
    ///
    /// # Parameters
    /// * `predicate`: The [`LiftedDerivedPredicate`] definition to be added.
    pub fn add_derived_predicate_def(&mut self, def: DerivedPredicateDef) {
        let skeleton_id = def.header_id();
        let next_def_id = DerivedPredicateDefId::new(self.derived_predicate_defs.len());

        // 1. Ajouter la définition à la liste globale
        self.derived_predicate_defs.push(def);

        // 2. Mettre à jour la table de dérivation
        let idx = skeleton_id.as_usize();
        if idx >= self.predicate_derivations.len() {
            // On remplit avec des Vec vides pour les prédicats non-dérivés
            self.predicate_derivations.resize(idx + 1, Vec::new());
        }

        // 3. "Visser" cette dérivation à sa signature
        self.predicate_derivations[idx].push(next_def_id);
    }

    /// Retourne vrai si le prédicat associé à ce squelette est un prédicat dérivé.
    /// (S'il possède au moins une dérivation/axiome).
    pub fn is_derived(&self, skeleton_id: AtomSkeletonId) -> bool {
        let idx = skeleton_id.as_usize();
        // On vérifie si l'index existe et si le vecteur de dérivations n'est pas vide
        self.predicate_derivations
            .get(idx)
            .map(|derivations| !derivations.is_empty())
            .unwrap_or(false)
    }

    /// Retourne la liste des IDs de définitions (axiomes) pour un squelette donné.
    /// Retourne une slice vide si le prédicat n'est pas dérivé.
    pub fn get_derivations(&self, skeleton_id: AtomSkeletonId) -> &[DerivedPredicateDefId] {
        let idx = skeleton_id.as_usize();
        self.predicate_derivations
            .get(idx)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn action_symbols(&self) -> &SymbolRegistry<ActionSymbolId> {
        &self.action_symbols
    }

    /// Insère un nom d'action dans la table des symboles et retourne son ID.
    /// Utile pour obtenir l'identité de l'action avant de construire sa structure (paramètres, préconditions, effets).
    pub fn add_action_symbol(&mut self, symbol: SymbolId) -> ActionSymbolId {
        self.action_symbols.insert(symbol)
    }

    /// Returns a slice of all lifted actions in the problem.
    ///
    /// Lifted actions represent the operators defined in the domain.
    /// This provides read-only access to their schemas, including
    /// parameters, preconditions, and effects.
    ///
    /// # Returns
    /// A slice of [`LiftedAction`].
    pub fn action_defs(&self) -> &[ActionDef] {
        &self.action_defs
    }

    /// Returns a mutable slice of all actions in the problem.
    ///
    /// This is used during the flattening or logic phases to modify
    /// action signatures, preconditions, and effects in place without
    /// reallocating the underlying collection.
    ///
    /// # Returns
    /// A mutable slice of [`LiftedAction`].
    pub fn action_defs_mut(&mut self) -> &mut [ActionDef] {
        &mut self.action_defs
    }

    /// Adds a new lifted action to the problem.
    ///
    /// This appends a [`LiftedAction`] to the internal action collection.
    /// Actions added here are typically processed later by the grounder.
    ///
    /// # Parameters
    /// * `action`: The [`LiftedAction`] schema to be registered.
    pub fn add_action_def(&mut self, action: ActionDef) {
        let symbol_id = action.name();

        // SAFETY: Ensure the action symbol is already registered.
        // This check is only active in debug/test builds and has zero cost in release.
        debug_assert!(
            symbol_id.as_usize() < self.action_symbols.len(),
            "Action symbol {:?} is missing from the registry. Symbols must be registered before adding definitions.",
            symbol_id
        );

        self.action_defs.push(action);
    }

    pub fn method_symbols(&self) -> &SymbolRegistry<MethodSymbolId> {
        &self.method_symbols
    }

    /// Insère un nom de méthode dans la table des symboles et retourne son ID.
    /// Utile pour obtenir l'identité de la méthode avant de construire sa structure (décomposition, contraintes).
    pub fn add_method_symbol(&mut self, symbol: SymbolId) -> MethodSymbolId {
        self.method_symbols.insert(symbol)
    }

    /// Returns a slice of all lifted methods in the problem.
    ///
    /// Methods define the decomposition rules in HTN planning, mapping
    /// abstract tasks to a network of sub-tasks (either primitive or compound).
    ///
    /// # Returns
    /// A slice of [`LiftedMethod`].
    pub fn method_defs(&self) -> &[MethodDef] {
        &self.method_defs
    }

    /// Returns a mutable slice of all methods in the problem.
    ///
    /// Methods are key in HTN planning as they define how tasks are decomposed.
    /// This accessor allows updating the method's parameters, preconditions,
    /// and its sub-task network during optimization or grounding phases.
    ///
    /// # Returns
    /// A mutable slice of [`LiftedMethod`].
    pub fn method_defs_mut(&mut self) -> &mut [MethodDef] {
        &mut self.method_defs
    }

    /// Adds a new lifted method to the problem.
    ///
    /// This registers a decomposition rule that the planner can use to
    /// expand abstract tasks into more specific task networks.
    ///
    /// # Parameters
    /// * `method`: The [`LiftedMethod`] definition to be added.
    pub fn add_method_def(&mut self, method: MethodDef) {
        let symbol_id = method.name();

        // SAFETY: Ensure the method symbol is already registered.
        // This invariant prevents indexing issues during task decomposition.
        debug_assert!(
            symbol_id.as_usize() < self.method_symbols.len(),
            "Method symbol {:?} is not registered. Ensure the symbol is added to the registry before its definition.",
            symbol_id
        );

        self.method_defs.push(method);
    }
    /// Returns a reference to the initial state expression.
    ///
    /// The initial state (typically the `:init` block in PDDL) describes
    /// the truth values of predicates and the values of functions at the
    /// start of the planning process.
    ///
    /// # Returns
    /// A reference to the [`Expr`] representing the initial state.
    pub fn init(&self) -> ExprId {
        self.init
    }

    /// Sets the initial state expression.
    ///
    /// This replaces the current initial state description with a new
    /// provided expression.
    ///
    /// # Parameters
    /// * `init_expr`: The new [`Expr`] representing the starting state.
    pub fn set_init(&mut self, init_expr: ExprId) {
        self.init = init_expr;
    }

    /// Returns a reference to the goal expression.
    ///
    /// The goal (typically the `:goal` block in PDDL) defines the logical
    /// conditions that must be true in the final state of a valid plan.
    ///
    /// # Returns
    /// A reference to the [`Expr`] representing the goal conditions.
    pub fn goal(&self) -> ExprId {
        self.goal
    }

    /// Sets the goal expression.
    ///
    /// This replaces the current goal definition with a new provided
    /// expression.
    ///
    /// # Parameters
    /// * `goal_expr`: The new [`Expr`] representing the target state conditions.
    pub fn set_goal(&mut self, goal_expr: ExprId) {
        self.goal = goal_expr;
    }

    /// Returns a reference to the global problem constraints.
    ///
    /// Unlike the goal, these constraints (often defined via the `:constraints`
    /// block in the PDDL problem file) typically represent temporal or
    /// trajectory-wide conditions that the plan must satisfy.
    ///
    /// # Returns
    /// A reference to the [`Expr`] representing the problem constraints.
    pub fn problem_constraints(&self) -> ExprId {
        self.problem_constraints
    }

    /// Sets the global problem constraints.
    ///
    /// This replaces the current trajectory constraints with a new provided
    /// expression.
    ///
    /// # Parameters
    /// * `constraints`: The new [`Expr`] to be applied as the problem's constraints.
    pub fn set_problem_constraints(&mut self, constraints: ExprId) {
        self.problem_constraints = constraints;
    }

    /// Returns a reference to the metric specification.
    ///
    /// The metric (defined via `:metric` in PDDL) specifies the objective
    /// function used to evaluate the quality of a plan, typically involving
    /// terms like `total-cost` or specific numeric fluents.
    ///
    /// # Returns
    /// A reference to the [`Expr`] representing the optimization metric.
    pub fn metric_spec(&self) -> ExprId {
        self.metric_spec
    }

    /// Sets the metric specification.
    ///
    /// This replaces the current optimization criteria with a new provided
    /// expression (e.g., `minimize (total-cost)`).
    ///
    /// # Parameters
    /// * `metric`: The new [`Expr`] representing the plan's optimization goal.
    pub fn set_metric_spec(&mut self, metric: ExprId) {
        self.metric_spec = metric;
    }

    /// Returns a reference to the length specification.
    ///
    /// The length specification can be used to define constraints on the
    /// total number of steps in a valid plan, or as a guideline for
    /// search depth.
    ///
    /// # Returns
    /// A reference to the [`Expr`] representing the length constraints.
    pub fn length_spec(&self) -> ExprId {
        self.length_spec
    }

    /// Sets the length specification.
    ///
    /// This replaces the current length constraints with a new provided
    /// expression.
    ///
    /// # Parameters
    /// * `length_spec`: The new [`Expr`] defining the plan length bounds.
    pub fn set_length_spec(&mut self, length_spec: ExprId) {
        self.length_spec = length_spec;
    }

    /// Returns a reference to the initial task network.
    ///
    /// In HTN planning, the initial task network defines the starting sequence
    /// or set of tasks (abstract or primitive) that the planner must decompose
    /// and execute to solve the problem.
    ///
    /// # Returns
    /// A reference to the [`InitialTaskNetwork`].
    pub fn initial_task_network(&self) -> &InitialTaskNetwork {
        &self.initial_task_network
    }

    /// Returns a mutable reference to the initial task network.
    ///
    /// This is used during the flattening or logic process to remap
    /// parameter types, update task identifiers, or modify ordering constraints
    /// in the HTN problem's entry point.
    ///
    /// # Returns
    /// A mutable reference to the [`InitialTaskNetwork`].
    pub fn initial_task_network_mut(&mut self) -> &mut InitialTaskNetwork {
        &mut self.initial_task_network
    }

    /// Sets the initial task network.
    ///
    /// This replaces the current task network with a new definition,
    /// effectively redefining the top-level objectives of the HTN problem.
    ///
    /// # Parameters
    /// * `initial_task_network`: The new [`InitialTaskNetwork`] to be
    ///   used as the planning starting point.
    pub fn set_initial_task_network(&mut self, initial_task_network: InitialTaskNetwork) {
        self.initial_task_network = initial_task_network;
    }

    /// Returns a wrapper around the domain view of this problem.
    ///
    /// The returned [`DomainDef`] provides read-only access to all domain-level
    /// information, such as types, constants, predicates, functions, actions,
    /// methods, requirements, and domain-level constraints.
    ///
    /// # Returns
    /// A [`DomainDef`] instance borrowing the current `LiftedProblem`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let problem: LiftedProblem = /* obtain LiftedProblem */;
    /// let domain = problem.domain_def();
    /// println!("Domain name: {}", domain.domain_name());
    /// ```
    pub fn domain_view(&self) -> DomainDef<'_> {
        DomainDef::new(self)
    }

    /// Returns a wrapper around the problem view of this problem.
    ///
    /// The returned [`ProblemDef`] provides read-only access to all problem-specific
    /// information, such as objects, initial state, goal state, problem constraints,
    /// metric and length specifications, and the initial task network.
    ///
    /// # Returns
    /// A [`ProblemDef`] instance borrowing the current `LiftedProblem`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let problem: LiftedProblem = /* obtain LiftedProblem */;
    /// let problem_def = problem.problem_def();
    /// println!("Problem name: {}", problem_def.problem_name());
    /// ```
    pub fn problem_view(&self) -> ProblemDef<'_> {
        ProblemDef::new(self)
    }
}

/*impl Display for Problem {
    /// Implements standard Rust [`Display`] for the problem.
    ///
    /// Delegates to the default interner-aware renderer for the entire problem.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the output into.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderers::default::render_problem(f, self)
    }
}*/

impl SerdeSerializable for Problem {}
