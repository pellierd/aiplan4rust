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
//! - The **initial state** and **goal conditions** expressed as symbolic expressions (`Expr`).
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

use crate::aiplan4rust::interner::{InternerError, SelfInternerDisplay, StringInterner};
use crate::aiplan4rust::lang::{Ident, Requirement, TypedSymbol};
use crate::aiplan4rust::lir::atomic_skeleton::{
    AtomicFormulaSkeleton, AtomicFunctionSkeleton, AtomicTaskSkeleton,
};
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::problem::{extract, normalize, renderers, InitialTaskNetwork, LiftedAction, LiftedDerivedPredicate, LiftedDurativeAction, LiftedMethod, LiftedProblem};
use crate::aiplan4rust::lir::problem::{DomainDef, ProblemDef};
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::serialization::serde::SerdeSerializable;
use crate::aiplan4rust::syntax::SyntaxDisplay;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fmt::{Display, Formatter};
use crate::aiplan4rust::linking::LinkedSemanticContext;

/// Represents a lifted planning problem defined in PDDL syntax.
///
/// A `Problem` aggregates all syntactic constructs relevant to a specific problem instance,
/// including domain and problem identifiers, type definitions, constants, predicates, actions,
/// and constraints, along with the initial state and goal specifications.
///
/// # Fields
///
/// - `interner`: String interner used to deduplicate identifiers and symbols.
/// - `domain_name`: Identifier of the associated domain.
/// - `problem_name`: Identifier of the problem instance.
/// - `requirements`: Declared requirements (features) used in the problem.
/// - `types`: Types declared in the problem (if any; may be inherited from the domain).
/// - `constants`: Constants declared in the problem.
/// - `predicates`: Predicate skeletons (signatures) available in the problem.
/// - `functions`: Function skeletons (signatures) available in the problem.
/// - `domain_constraints`: Global domain-level constraints (can be empty).
/// - `tasks`: Decomposable task declarations used in HTN planning.
/// - `derived_predicates`: Derived predicates available in the problem.
/// - `actions`: Primitive actions available in the problem.
/// - `durative_actions`: Durative actions available in the problem.
/// - `methods`: HTN decomposition methods.
/// - `objects`: Concrete objects defined in the problem instance.
/// - `init`: The initial state, expressed as a logical expression.
/// - `goal`: The goal condition to be achieved, as a logical expression.
/// - `problem_constraints`: Problem-specific constraints (distinct from domain-level).
/// - `metric_spec`: The optimization metric, such as `minimize` or `maximize` some expression.
/// - `length_spec`: A deprecated field from PDDL 2.1 specifying plan length bounds.
/// - `initial_task_network`: The initial task network for HTN planning (if applicable).
///
/// # Example
///
/// ```
/// use aiplan4rust::aiplan4rust::interner::Ident;
/// use aiplan4rust::aiplan4rust::lir::problem::LiftedProblem;
/// let mut problem = LiftedProblem::new();
/// problem.set_domain_id(Ident::new("my_domain"));
/// problem.set_problem_id(Ident::new("my_problem"));
///
/// assert_eq!(problem.domain_id().as_str(), "my_domain");
/// assert_eq!(problem.problem_id().as_str(), "my_problem");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Problem {
    /// The interner used for string deduplication.
    interner: StringInterner,

    /// The identifier of the domain.
    domain_id: Ident,

    /// The identifier of the problem.
    problem_id: Ident,

    /// The set of requirements for this syntax problem.
    requirements: HashSet<Requirement>,

    /// The set of types defined in this syntax problem.
    types: HashMap<Ident, TypedSymbol>,

    /// The set of constants defined in this syntax problem.
    constants: HashMap<Ident, TypedSymbol>,

    /// The list of predicates in the syntax problem.
    predicates: Vec<AtomicFormulaSkeleton>,

    /// The list of functions in the syntax problem.
    functions: Vec<AtomicFunctionSkeleton>,

    /// The constraints defined in the domain, i.e., the global constraints
    /// No constraints are represented by an empty and `Expr'.
    domain_constraints: Expr,

    /// The list of tasks defined in this syntax problem.
    tasks: Vec<AtomicTaskSkeleton>,

    /// The list of derived predicates defined in this syntax problem.
    derived_predicates: Vec<LiftedDerivedPredicate>,

    /// The list of actions defined in this syntax problem.
    actions: Vec<LiftedAction>,

    /// The list of durative actions defined in this syntax problem.
    durative_actions: Vec<LiftedDurativeAction>,

    /// The list of methods defined in this syntax problem.
    methods: Vec<LiftedMethod>,

    /// The set of objects defined in this syntax problem.
    objects: HashMap<Ident, TypedSymbol>,

    /// The initial state of the problem.
    init: Expr,

    /// The goal of the problem.
    goal: Expr,

    /// The constraints defined in the problem, i.e., the constraints specific to an instance.
    /// No constraints are represented by an empty and `Expr'.
    problem_constraints: Expr,

    /// The metric specification of the problem.
    metric_spec: Expr,

    /// The length specification of the problem.
    /// The length-spec is deprecated since PDDL 2.1.
    length_spec: Expr,

    /// The initial task network of the problem.
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
    /// assert!(problem.actions().is_empty());
    /// assert!(problem.types().is_empty());
    /// ```
    pub(crate) fn new(interner : StringInterner, requirements: HashSet<Requirement>) -> Self {
        Self {
            interner,
            domain_id: Ident::default(),
            problem_id: Ident::default(),
            requirements,
            types: HashMap::new(),
            constants: HashMap::new(),
            predicates: Vec::new(),
            functions: Vec::new(),
            domain_constraints: Expr::empty_or(),
            tasks: Vec::new(), // Add for HDDL
            derived_predicates: Vec::new(),
            actions: Vec::new(),
            durative_actions: Vec::new(),
            methods: Vec::new(), // Add for HDDL
            objects: HashMap::new(),
            init: Expr::empty_and(),
            goal: Expr::empty_or(),
            problem_constraints: Expr::empty_or(),
            metric_spec: Expr::metric_none(),
            length_spec: Expr::empty_length_spec(),
            initial_task_network: InitialTaskNetwork::default(), // Add for HDDL
        }
    }

    /// Returns an immutable reference to the unified string interner.
    ///
    /// # Returns
    ///
    /// A reference to the `StringInterner` used for identifier management.
    pub fn interner(&self) -> &StringInterner {
        &self.interner
    }

    /// Returns a mutable reference to the unified string interner.
    ///
    /// # Returns
    ///
    /// A mutable reference to the `StringInterner`.
    pub fn interner_mut(&mut self) -> &mut StringInterner {
        &mut self.interner
    }

    /// Sets the internal [`StringInterner`] used by this problem.
    ///
    /// This replaces the existing interner with the one provided.
    ///
    /// # Arguments
    ///
    /// * `interner` - The new [`StringInterner`] to assign to this problem.
    pub fn set_interner(&mut self, interner: StringInterner) {
        self.interner = interner;
    }

    /// Consumes and returns the internal `StringInterner`, leaving a new empty one in its place.
    ///
    /// # Returns
    /// The previously held `StringInterner`.
    pub fn take_interner(&mut self) -> StringInterner {
        std::mem::take(&mut self.interner)
    }


    /// Returns the identifier of the domain.
    pub fn domain_id(&self) -> Ident {
        self.domain_id
    }

    /// Sets the identifier of the domain.
    ///
    /// Checks that the identifier exists in the interner.
    ///
    /// # Errors
    /// Returns `InternerError` if the identifier is not present in the interner.
    pub fn set_domain_id(&mut self, id: Ident) -> Result<(), InternerError> {
        self.interner.try_resolve_ident(id)?;
        self.domain_id = id;
        Ok(())
    }

    /// Returns the name of the domain as a string slice.
    ///
    /// # Returns
    /// `Ok(&str)` if the identifier exists in the interner, otherwise `Err(InternerError)`.
    pub fn domain_name(&self) -> Result<&str, InternerError> {
        self.interner.try_resolve_ident(self.domain_id)
    }

    /// Returns the identifier of the problem.
    pub fn problem_id(&self) -> Ident {
        self.problem_id.clone()
    }

    /// Sets the identifier of the problem.
    ///
    /// Checks that the identifier exists in the interner.
    ///
    /// # Errors
    /// Returns `InternerError` if the identifier is not present in the interner.
    pub fn set_problem_id(&mut self, id: Ident) -> Result<(), InternerError> {
        self.interner.try_resolve_ident(id)?;
        self.problem_id = id;
        Ok(())
    }

    /// Returns the name of the problem as a string slice.
    ///
    /// # Returns
    /// `Ok(&str)` if the identifier exists in the interner, otherwise `Err(InternerError)`.
    pub fn problem_name(&self) -> Result<&str, InternerError> {
        self.interner.try_resolve_ident(self.problem_id)
    }

    // === Requirements ===

    /// Returns a reference to the set of requirements.
    pub fn requirements(&self) -> &HashSet<Requirement> {
        &self.requirements
    }

    /// Returns a mutable reference to the set of requirements.
    pub fn requirements_mut(&mut self) -> &mut HashSet<Requirement> {
        &mut self.requirements
    }

    /// Adds a single requirement.
    ///
    /// If the requirement already exists, it is not added again.
    pub fn add_requirement(&mut self, requirement: Requirement) {
        self.requirements.insert(requirement);
    }

    /// Adds multiple requirements.
    pub fn add_requirements<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Requirement>,
    {
        self.requirements.extend(iter);
    }

    /// Checks if a given semantic requirement is actually required by the AST content.
    ///
    /// Required requirements represent the minimal set of features that are actively used
    /// in the domain or problem definitions. A requirement may be declared in the context
    /// but not actually required if it is never referenced in the ASTs.
    ///
    /// # Arguments
    /// * `requirement` - The semantic requirement to check.
    ///
    /// # Returns
    /// `true` if the requirement is required by the AST (i.e., used in the domain/problem),
    /// `false` otherwise.
    ///
    /// # Example
    /// ```rust
    /// if context.is_required(Requirement::DurativeActions) {
    ///     println!("DurativeActions are actually used in this context.");
    /// }
    /// ```
    pub fn is_required(&self, requirement: Requirement) -> bool {
        self.requirements.contains(&requirement)
    }

    /// Replaces the current set of requirements with a new set.
    ///
    /// # Arguments
    /// * `new_requirements` - The new set of requirements to use.
    ///
    /// # Example
    /// ```rust
    /// problem.set_requirements(HashSet::from([Requirement::DurativeActions]));
    /// ```
    pub fn set_requirements(&mut self, new_requirements: HashSet<Requirement>) {
        self.requirements = new_requirements;
    }

    /// Consumes and returns all declared requirements, leaving an empty set in its place.
    ///
    /// # Returns
    /// The previously held `HashSet<Requirement>`.
    pub fn take_requirements(&mut self) -> HashSet<Requirement> {
        std::mem::take(&mut self.requirements)
    }

    // === Types ===

    /// Returns a reference to the set of types.
    ///
    /// # Example
    ///
    /// ```
    /// use aiplan4rust::aiplan4rust::lir::problem::LiftedProblem;
    /// let problem = LiftedProblem::new();
    /// assert!(problem.types().is_empty());
    /// ```
    pub fn types(&self) -> impl Iterator<Item = &TypedSymbol> {
        self.types.values()
    }

    /// Returns true if the problem contains any types.
    pub fn has_types(&self) -> bool {
        !self.types.is_empty()
    }

    /// Returns a mutable reference to the types map.
    ///
    /// This allows modifying existing `TypedSymbol`s directly,
    /// while still keeping the internal storage as a HashMap.
    pub fn types_mut(&mut self) -> &mut HashMap<Ident, TypedSymbol> {
        &mut self.types
    }

    /// Get a type by its Ident (immutable)
    pub fn get_type(&self, id: Ident) -> Option<&TypedSymbol> {
        self.types.get(&id)
    }

    /// Get a type by its Ident (mutable)
    pub fn get_type_mut(&mut self, id: Ident) -> Option<&mut TypedSymbol> {
        self.types.get_mut(&id)
    }

    /// Get a type by its `Ident` (immutable).
    pub fn try_get_type(&self, id: Ident) -> Result<&TypedSymbol, LirError> {
        self.types.get(&id).ok_or_else(|| LirError::type_not_found(id))
    }

    /// Get a type by its `Ident` (mutable).
    pub fn try_get_type_mut(&mut self, id: Ident) -> Result<&mut TypedSymbol, LirError> {
        self.types.get_mut(&id).ok_or_else(|| LirError::type_not_found(id))
    }

    /// Adds a single type_checker.
    ///
    /// If the type_checker already exists, it is not added again.
    ///
    /// # Example
    ///
    /// ```
    /// problem.add_type(TypedSymbol::new("vehicle", "object"));
    /// ```
    pub fn add_type(&mut self, ty: TypedSymbol) {
        self.types.insert(ty.symbol(), ty);
    }

    /// Adds multiple types at once.
    ///
    /// Existing types with the same `Ident` will be overwritten.
    ///
    /// # Example
    ///
    /// ```
    /// let mut problem = LiftedProblem::new();
    /// problem.add_types(vec![
    ///     TypedSymbol::new(a, type_a),
    ///     TypedSymbol::new(b, type_b),
    /// ]);
    /// ```
    pub fn add_types<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = TypedSymbol>,
    {
        for ty in iter {
            self.add_type(ty);
        }
    }

    // === Constants ===

    /// Returns an iterator over the constants of the problem.
    ///
    /// # Example
    ///
    /// ```
    /// let problem = LiftedProblem::new();
    /// for c in problem.constants() {
    ///     println!("{:?}", c);
    /// }
    /// ```
    pub fn constants(&self) -> impl Iterator<Item = &TypedSymbol> {
        self.constants.values()
    }

    /// Returns a mutable iterator over the constants of the problem.
    ///
    /// # Example
    ///
    /// ```
    /// let mut problem = LiftedProblem::new();
    /// for c in problem.constants_mut_iter() {
    ///     c.set_name(Ident::new("new_name")); // Exemple de modification
    /// }
    /// ```
    pub fn constants_mut(&mut self) -> impl Iterator<Item = &mut TypedSymbol> {
        self.constants.values_mut()
    }

    /// Returns true if the problem contains any constants.
    ///
    /// # Example
    ///
    /// ```
    /// let problem = LiftedProblem::new();
    /// assert!(!problem.has_constants());
    /// ```
    pub fn has_constants(&self) -> bool {
        !self.constants.is_empty()
    }


    /// Get a constant by its `Ident` (immutable).
    ///
    /// # Returns
    /// - `Some(&TypedSymbol)` if the constant exists.
    /// - `None` otherwise.
    pub fn get_constant(&self, id: Ident) -> Option<&TypedSymbol> {
        self.constants.get(&id)
    }

    /// Get a constant by its `Ident` (mutable).
    ///
    /// # Returns
    /// - `Some(&mut TypedSymbol)` if the constant exists.
    /// - `None` otherwise.
    pub fn get_constant_mut(&mut self, id: Ident) -> Option<&mut TypedSymbol> {
        self.constants.get_mut(&id)
    }

    /// Get a constant by its `Ident` (immutable).
    ///
    /// # Errors
    /// - [`LirError::constant_not_found`] if the constant does not exist.
    pub fn try_get_constant(&self, id: Ident) -> Result<&TypedSymbol, LirError> {
        self.constants
            .get(&id)
            .ok_or_else(|| LirError::constant_not_found(id))
    }

    /// Get a constant by its `Ident` (mutable).
    ///
    /// # Errors
    /// - [`LirError::constant_not_found`] if the constant does not exist.
    pub fn try_get_constant_mut(&mut self, id: Ident) -> Result<&mut TypedSymbol, LirError> {
        self.constants
            .get_mut(&id)
            .ok_or_else(|| LirError::constant_not_found(id))
    }

    /// Adds a single constant to the problem.
    ///
    /// If a constant with the same `Ident` already exists, it is overwritten.
    ///
    /// # Example
    ///
    /// ```
    /// problem.add_constant(TypedSymbol::new(id, ty));
    /// ```
    pub fn add_constant(&mut self, constant: TypedSymbol) {
        self.constants.insert(constant.symbol(), constant);
    }

    /// Adds multiple constants at once.
    ///
    /// Existing constants with the same `Ident` will be overwritten.
    ///
    /// # Example
    ///
    /// ```
    /// problem.add_constants(vec![c1, c2, c3]);
    /// ```
    pub fn add_constants<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = TypedSymbol>,
    {
        for constant in iter {
            self.add_constant(constant);
        }
    }

    // === Predicates ===

    /// Returns a reference to the list of predicates.
    pub fn predicates(&self) -> &Vec<AtomicFormulaSkeleton> {
        &self.predicates
    }

    /// Returns a mutable reference to the list of predicates.
    pub fn predicates_mut(&mut self) -> &mut Vec<AtomicFormulaSkeleton> {
        &mut self.predicates
    }

    /// Adds a single predicate.
    pub fn add_predicate(&mut self, predicate: AtomicFormulaSkeleton) {
        self.predicates.push(predicate);
    }

    /// Adds multiple predicates.
    pub fn add_predicates<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = AtomicFormulaSkeleton>,
    {
        self.predicates.extend(iter);
    }

    // === Functions ===

    /// Returns a reference to the list of functions.
    pub fn functions(&self) -> &Vec<AtomicFunctionSkeleton> {
        &self.functions
    }

    /// Returns a mutable reference to the list of functions.
    pub fn functions_mut(&mut self) -> &mut Vec<AtomicFunctionSkeleton> {
        &mut self.functions
    }

    /// Adds a single function.
    pub fn add_function(&mut self, function: AtomicFunctionSkeleton) {
        self.functions.push(function);
    }

    /// Adds multiple functions.
    pub fn add_functions<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = AtomicFunctionSkeleton>,
    {
        self.functions.extend(iter);
    }

    // === Domain Constraints ===

    /// Returns a reference to the domain constraints' expression.
    pub fn domain_constraints(&self) -> &Expr {
        &self.domain_constraints
    }

    /// Returns a mutable reference to the domain constraints' expression.
    pub fn domain_constraints_mut(&mut self) -> &mut Expr {
        &mut self.domain_constraints
    }

    /// Sets the domain constraints expression.
    pub fn set_domain_constraints(&mut self, constraints: Expr) {
        self.domain_constraints = constraints;
    }

    // === Tasks ===

    /// Returns a reference to the list of tasks.
    pub fn tasks(&self) -> &Vec<AtomicTaskSkeleton> {
        &self.tasks
    }

    /// Returns a mutable reference to the list of tasks.
    pub fn tasks_mut(&mut self) -> &mut Vec<AtomicTaskSkeleton> {
        &mut self.tasks
    }

    /// Adds a single task.
    pub fn add_task(&mut self, task: AtomicTaskSkeleton) {
        self.tasks.push(task);
    }

    /// Adds multiple tasks.
    pub fn add_tasks<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = AtomicTaskSkeleton>,
    {
        self.tasks.extend(iter);
    }

    // === Derived Predicates ===

    /// Returns a reference to the list of derived predicates.
    pub fn derived_predicates(&self) -> &Vec<LiftedDerivedPredicate> {
        &self.derived_predicates
    }

    /// Returns a mutable reference to the list of derived predicates.
    pub fn derived_predicates_mut(&mut self) -> &mut Vec<LiftedDerivedPredicate> {
        &mut self.derived_predicates
    }

    /// Adds a single derived predicate to the problem.
    pub fn add_derived_predicate(&mut self, predicate: LiftedDerivedPredicate) {
        self.derived_predicates.push(predicate);
    }

    // === Actions ===

    /// Returns a reference to the list of actions.
    pub fn actions(&self) -> &Vec<LiftedAction> {
        &self.actions
    }

    /// Returns a mutable reference to the list of actions.
    pub fn actions_mut(&mut self) -> &mut Vec<LiftedAction> {
        &mut self.actions
    }

    /// Adds a single action.
    pub fn add_action(&mut self, action: LiftedAction) {
        self.actions.push(action);
    }

    // === Durative Actions ===

    /// Returns a reference to the list of durative actions.
    pub fn durative_actions(&self) -> &Vec<LiftedDurativeAction> {
        &self.durative_actions
    }

    /// Returns a mutable reference to the list of durative actions.
    pub fn durative_actions_mut(&mut self) -> &mut Vec<LiftedDurativeAction> {
        &mut self.durative_actions
    }

    /// Adds a single durative action.
    pub fn add_durative_action(&mut self, action: LiftedDurativeAction) {
        self.durative_actions.push(action);
    }

    /// Adds multiple duratives actions.
    pub fn add_actions<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = LiftedDurativeAction>,
    {
        self.durative_actions.extend(iter);
    }

    // === Methods ===

    /// Returns a reference to the list of methods.
    pub fn methods(&self) -> &Vec<LiftedMethod> {
        &self.methods
    }

    /// Returns a mutable reference to the list of methods.
    pub fn methods_mut(&mut self) -> &mut Vec<LiftedMethod> {
        &mut self.methods
    }

    /// Adds a single method.
    pub fn add_method(&mut self, method: LiftedMethod) {
        self.methods.push(method);
    }

    /// Adds multiple methods.
    pub fn add_methods<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = LiftedMethod>,
    {
        self.methods.extend(iter);
    }

    // === Objects ===

    /// Returns an iterator over the objects of the problem.
    ///
    /// # Example
    ///
    /// ```
    /// let problem = LiftedProblem::new();
    /// for obj in problem.objects_iter() {
    ///     println!("{:?}", obj);
    /// }
    /// ```
    pub fn objects(&self) -> impl Iterator<Item = &TypedSymbol> {
        self.objects.values()
    }

    /// Returns a mutable iterator over the objects of the problem.
    ///
    /// # Example
    ///
    /// ```
    /// let mut problem = LiftedProblem::new();
    /// for obj in problem.objects_iter_mut() {
    ///     obj.set_name(Ident::new("new_object"));
    /// }
    /// ```
    pub fn objects_mut(&mut self) -> impl Iterator<Item = &mut TypedSymbol> {
        self.objects.values_mut()
    }

    /// Returns true if the problem contains any objects.
    pub fn has_objects(&self) -> bool {
        !self.objects.is_empty()
    }

    /// Get an object by its `Ident` (immutable).
    pub fn get_object(&self, id: Ident) -> Option<&TypedSymbol> {
        self.objects.get(&id)
    }

    /// Get an object by its `Ident` (mutable).
    pub fn get_object_mut(&mut self, id: Ident) -> Option<&mut TypedSymbol> {
        self.objects.get_mut(&id)
    }

    /// Get an object by its `Ident` (immutable), or return an error if not found.
    pub fn try_get_object(&self, id: Ident) -> Result<&TypedSymbol, LirError> {
        self.objects.get(&id).ok_or_else(|| LirError::object_not_found(id))
    }

    /// Get an object by its `Ident` (mutable), or return an error if not found.
    pub fn try_get_object_mut(&mut self, id: Ident) -> Result<&mut TypedSymbol, LirError> {
        self.objects.get_mut(&id).ok_or_else(|| LirError::object_not_found(id))
    }

    /// Adds a single object.
    ///
    /// If the object already exists, it is overwritten.
    ///
    /// # Example
    ///
    /// ```
    /// problem.add_object(TypedSymbol::new("robot1", "vehicle"));
    /// ```
    pub fn add_object(&mut self, object: TypedSymbol) {
        self.objects.insert(object.symbol(), object);
    }

    /// Adds multiple objects at once.
    ///
    /// Existing objects with the same `Ident` will be overwritten.
    ///
    /// # Example
    ///
    /// ```
    /// problem.add_objects(vec![
    ///     TypedSymbol::new("robot1", "vehicle"),
    ///     TypedSymbol::new("robot2", "vehicle"),
    /// ]);
    /// ```
    pub fn add_objects<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = TypedSymbol>,
    {
        for object in iter {
            self.add_object(object);
        }
    }

    // === Init ===

    /// Returns the initial state expression.
    ///
    /// Typically, an `Expr::And(...)` representing a conjunction of initial facts.
    pub fn init(&self) -> &Expr {
        &self.init
    }

    /// Returns a mutable reference to the initial state expression.
    pub fn init_mut(&mut self) -> &mut Expr {
        &mut self.init
    }

    /// Sets the initial state expression.
    pub fn set_init(&mut self, init_expr: Expr) {
        self.init = init_expr;
    }

    // === Goal ===

    /// Returns the goal state expression.
    ///
    /// Typically, an `Expr::And(...)` representing a conjunction of goal conditions.
    pub fn goal(&self) -> &Expr {
        &self.goal
    }

    /// Returns a mutable reference to the goal state expression.
    pub fn goal_mut(&mut self) -> &mut Expr {
        &mut self.goal
    }

    /// Sets the goal state expression.
    pub fn set_goal(&mut self, goal_expr: Expr) {
        self.goal = goal_expr;
    }

    // === Problem Constraints ===

    /// Returns a reference to the problem constraints' expression.
    pub fn problem_constraints(&self) -> &Expr {
        &self.problem_constraints
    }

    /// Returns a mutable reference to the problem constraints' expression.
    pub fn problem_constraints_mut(&mut self) -> &mut Expr {
        &mut self.problem_constraints
    }

    /// Sets the problem constraints expression.
    pub fn set_problem_constraints(&mut self, constraints: Expr) {
        self.problem_constraints = constraints;
    }

    // === Metric Specification ===

    /// Returns a reference to the metric specification expression.
    pub fn metric_spec(&self) -> &Expr {
        &self.metric_spec
    }

    /// Returns a mutable reference to the metric specification expression.
    pub fn metric_spec_mut(&mut self) -> &mut Expr {
        &mut self.metric_spec
    }

    /// Sets the metric specification expression.
    pub fn set_metric_spec(&mut self, metric: Expr) {
        self.metric_spec = metric;
    }
    // === Length Specification ===

    /// Returns a reference to the length specification.
    pub fn length_spec(&self) -> &Expr {
        &self.length_spec
    }

    /// Returns a mutable reference to the length specification.
    pub fn length_spec_mut(&mut self) -> &mut Expr {
        &mut self.length_spec
    }

    /// Sets the length specification.
    pub fn set_length_spec(&mut self, length_spec: Expr) {
        self.length_spec = length_spec;
    }

    // === Initial Task Network ===

    /// Returns a reference to the initial task network.
    pub fn initial_task_network(&self) -> &InitialTaskNetwork {
        &self.initial_task_network
    }

    /// Returns a mutable reference to the initial task network.
    pub fn initial_task_network_mut(&mut self) -> &mut InitialTaskNetwork {
        &mut self.initial_task_network
    }

    /// Sets the initial task network.
    pub fn set_initial_task_network(&mut self, initial_task_network: InitialTaskNetwork) {
        self.initial_task_network = initial_task_network;
    }

    /// Normalizes all expressions and normalizable components of this `Problem`.
    ///
    /// This includes:
    /// - Problem-level expressions: `domain_constraints`, `init`, `goal`,
    ///   `problem_constraints`, `metric_spec`, `length_spec`.
    /// - All actions (`precondition` and `effect`).
    /// - All methods (`precondition` and task network logical constraints).
    /// - The initial task network (`logical_constraints` only).
    ///
    /// # Errors
    ///
    /// Returns a `LirError` if normalization of any expression fails.
    pub fn normalize(&mut self) -> Result<(), LirError> {
        Ok(normalize::normalize_problem(self)?)
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
    pub fn domain_def(&self) -> DomainDef<'_> {
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
    pub fn problem_def(&self) -> ProblemDef<'_> {
        ProblemDef::new(self)
    }
}

impl SyntaxDisplay for Problem {
    /// Formats the entire problem, including both domain and problem definitions,
    /// as a syntax string.
    ///
    /// This implementation delegates rendering to the syntax renderers for
    /// the domain (`render_domain_def`) and the problem (`render_problem_def`),
    /// inserting a newline between them.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the syntax string into.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure.
    fn fmt_syntax(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderers::syntax::render_domain_def(f, &self.domain_def(), &self.interner())?;
        writeln!(f)?;
        renderers::syntax::render_problem_def(f, &self.problem_def(), &self.interner())
    }
}

impl SelfInternerDisplay for Problem {
    /// Formats the problem using its internal `StringInterner`, including both
    /// domain and problem definitions.
    ///
    /// This implementation delegates rendering to the interner-aware renderers
    /// for the domain (`render_domain_def`) and the problem (`render_problem_def`),
    /// inserting a newline between them.
    ///
    /// # Arguments
    ///
    /// * `f` - The formatter to write the string into.
    ///
    /// # Returns
    ///
    /// A [`fmt::Result`] indicating success or failure.
    fn fmt_interner(&self, f: &mut Formatter<'_>) -> fmt::Result {
        renderers::interner::render_domain_def(f, &self.domain_def(), &self.interner())?;
        writeln!(f)?;
        renderers::interner::render_problem_def(f, &self.problem_def(), &self.interner())
    }
}

impl TryFrom<LinkedSemanticContext> for Problem {
    type Error = LirError;

    /// Attempts to create a `LiftedProblem` from a fully linked and semantically verified
    /// `LinkedSemanticContext`.
    ///
    /// This conversion performs the following steps:
    /// 1. Consumes the `StringInterner` from the context to manage identifiers.
    /// 2. Consumes the required `Requirement`s from the context to reflect the actual
    ///    semantic requirements used in the domain and problem.
    /// 3. Creates a new `LiftedProblem` initialized with the interner and requirements.
    /// 4. Extracts all domain-level elements (types, constants, predicates, functions,
    ///    actions, methods) from the context and populates the problem.
    /// 5. Extracts all problem-level elements (objects, initial state, goals,
    ///    constraints, metrics, initial task network) and populates the problem.
    /// 6. Normalizes all expressions in the problem to canonical form.
    ///
    /// After this conversion, the `LiftedProblem` contains a fully constructed,
    /// semantically consistent, and normalized representation of the lifted problem.
    ///
    /// # Errors
    ///
    /// Returns a `LirError` if any extraction or normalization step fails.
    fn try_from(mut context: LinkedSemanticContext) -> Result<Self, Self::Error> {

        // 1. Consume interner and required requirements from the context
        let interner = context.take_interner();
        let requirements = context.take_required_requirements();

        // 2. Create a new lifted problem with interner and requirements
        let mut problem = LiftedProblem::new(interner, requirements);

        // 3. Extract domain-level elements
        extract::extract_domain(&context, &mut problem)?;

        // 4. Extract problem-level elements
        extract::extract_problem(&context, &mut problem)?;

        // 5. Normalize all expressions in the problem
        normalize::normalize_problem(&mut problem)?;

        // 6. Return the fully constructed and normalized problem
        Ok(problem)
    }
}


impl Display for Problem {
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
        renderers::interner::render_problem(f, &self, &self.interner())
    }
}

impl SerdeSerializable for Problem { }
