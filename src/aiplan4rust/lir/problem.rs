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

use std::collections::HashSet;
use std::fmt;
use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};
use crate::aiplan4rust::lir::{InitialTaskNetwork, LiftedAction, LiftedMethod};
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lang::{Ident, Requirement, TypedSymbol};
use crate::aiplan4rust::lir::atomic_skeleton::{AtomicFormulaSkeleton, AtomicFunctionSkeleton, AtomicTaskSkeleton};
use crate::aiplan4rust::serialization::serde::SerdeSerializable;

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
/// - `actions`: Primitive actions available in the problem.
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
/// let mut problem = PlanningProblem::new();
/// problem.set_domain_name(Ident::new("my_domain"));
/// problem.set_problem_name(Ident::new("my_problem"));
///
/// assert_eq!(problem.domain_name().as_str(), "my_domain");
/// assert_eq!(problem.problem_name().as_str(), "my_problem");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Problem {

    /// The interner used for string deduplication.
    interner: StringInterner,

    /// The identifier of the domain.
    domain_name: Ident,

    /// The identifier of the problem.
    problem_name: Ident,

    /// The set of requirements for this syntax problem.
    requirements: HashSet<Requirement>,

    /// The set of types defined in this syntax problem.
    types: HashSet<TypedSymbol>,

    /// The set of constants defined in this syntax problem.
    constants: HashSet<TypedSymbol>,

    /// The list of predicates in the syntax problem.
    predicates: Vec<AtomicFormulaSkeleton>,

    /// The list of functions in the syntax problem.
    functions: Vec<AtomicFunctionSkeleton>,

    /// The constraints defined in the domain, i.e., the global constraints
    /// No constraints are represented by an empty and `Expr'.
    domain_constraints: Expr,

    /// The list of tasks defined in this syntax problem.
    tasks: Vec<AtomicTaskSkeleton>,

    /// The list of actions defined in this syntax problem.
    actions: Vec<LiftedAction>,

    /// The list of methods defined in this syntax problem.
    methods: Vec<LiftedMethod>,

    /// The set of objects defined in this syntax problem.
    objects: HashSet<TypedSymbol>,

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
    /// let problem = PlanningProblem::new();
    /// assert!(problem.actions().is_empty());
    /// assert!(problem.types().is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            interner: StringInterner::new(),
            domain_name: Ident::default(),
            problem_name: Ident::default(),
            requirements: HashSet::new(),
            types: HashSet::new(),
            constants: HashSet::new(),
            predicates: Vec::new(),
            functions: Vec::new(),
            domain_constraints: Expr::empty_or(),
            tasks: Vec::new(), // Add for HDDL
            actions: Vec::new(),
            methods: Vec::new(), // Add for HDDL
            objects: HashSet::new(),
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

    /// Returns the identifier of the domain.
    pub fn domain_name(&self) -> Ident {
        self.domain_name.clone()
    }

    /// Sets the identifier of the domain.
    ///
    /// # Example
    ///
    /// ```
    /// let mut problem = PlanningProblem::new();
    /// problem.set_domain_name(Ident::new("transport"));
    /// ```
    pub fn set_domain_name(&mut self, name: Ident) {
        self.domain_name = name;
    }

    /// Returns the identifier of the problem.
    pub fn problem_name(&self) -> Ident {
        self.problem_name.clone()
    }

    /// Sets the identifier of the problem.
    ///
    /// # Example
    ///
    /// ```
    /// let mut problem = PlanningProblem::new();
    /// problem.set_problem_name(Ident::new("logistics"));
    /// ```
    pub fn set_problem_name(&mut self, name: Ident) {
        self.problem_name = name;
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

    // === Types ===

    /// Returns a reference to the set of types.
    ///
    /// # Example
    ///
    /// ```
    /// let problem = PlanningProblem::new();
    /// assert!(problem.types().is_empty());
    /// ```
    pub fn types(&self) -> &HashSet<TypedSymbol> {
        &self.types
    }

    /// Returns a mutable reference to the set of types.
    pub fn types_mut(&mut self) -> &mut HashSet<TypedSymbol> {
        &mut self.types
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
        self.types.insert(ty);
    }

    /// Adds multiple types.
    ///
    /// # Example
    ///
    /// ```
    /// problem.add_types(vec![
    ///     TypedSymbol::new("truck", "vehicle"),
    ///     TypedSymbol::new("package", "object"),
    /// ]);
    /// ```
    pub fn add_types<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = TypedSymbol>,
    {
        self.types.extend(iter);
    }

    // === Constants ===

    /// Returns a reference to the set of constants.
    pub fn constants(&self) -> &HashSet<TypedSymbol> {
        &self.constants
    }

    /// Returns a mutable reference to the set of constants.
    pub fn constants_mut(&mut self) -> &mut HashSet<TypedSymbol> {
        &mut self.constants
    }

    /// Adds a single constant.
    pub fn add_constant(&mut self, constant: TypedSymbol) {
        self.constants.insert(constant);
    }

    /// Adds multiple constants.
    pub fn add_constants<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = TypedSymbol>,
    {
        self.constants.extend(iter);
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
        I: IntoIterator<Item =AtomicFormulaSkeleton>,
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
        I: IntoIterator<Item =AtomicFunctionSkeleton>,
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
        I: IntoIterator<Item =AtomicTaskSkeleton>,
    {
        self.tasks.extend(iter);
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

    /// Adds multiple actions.
    pub fn add_actions<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = LiftedAction>,
    {
        self.actions.extend(iter);
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

    /// Returns a reference to the set of objects.
    pub fn objects(&self) -> &HashSet<TypedSymbol> {
        &self.objects
    }

    /// Returns a mutable reference to the set of objects.
    pub fn objects_mut(&mut self) -> &mut HashSet<TypedSymbol> {
        &mut self.objects
    }

    /// Adds a single object.
    pub fn add_object(&mut self, object: TypedSymbol) {
        self.objects.insert(object);
    }

    /// Adds multiple objects.
    pub fn add_objects<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = TypedSymbol>,
    {
        self.objects.extend(iter);
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

}

impl Display for Problem {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Problem {{")?;
        writeln!(f, "  domain_name: {}", self.domain_name)?;
        writeln!(f, "  problem_name: {}", self.problem_name)?;
        writeln!(f, "  requirements: {:?}", self.requirements)?;
        writeln!(f, "  types: {:?}", self.types)?;
        writeln!(f, "  constants: {:?}", self.constants)?;
        writeln!(f, "  predicates: {:?}", self.predicates)?;
        writeln!(f, "  functions: {:?}", self.functions)?;
        writeln!(f, "  domain_constraints: {}", self.domain_constraints)?;
        writeln!(f, "  tasks: {:?}", self.tasks)?;
        writeln!(f, "  actions: {:?}", self.actions)?;
        writeln!(f, "  methods: {:?}", self.methods)?;
        writeln!(f, "  objects: {:?}", self.objects)?;
        writeln!(f, "  init: {}", self.init)?;
        writeln!(f, "  goal: {}", self.goal)?;
        writeln!(f, "  problem_constraints: {}", self.problem_constraints)?;
        writeln!(f, "  metric_spec: {}", self.metric_spec)?;
        writeln!(f, "  length_spec: {}", self.length_spec)?;
        writeln!(f, "  initial_task_network: {:?}", self.initial_task_network)?;
        writeln!(f, "}}")
    }
}

impl InternerDisplay for Problem {
    fn fmt_with_interner(&self, f: &mut Formatter<'_>, _interner: &StringInterner) -> fmt::Result {
        writeln!(f, "Problem {{")?;
        writeln!(f, "  domain_name: {}", self.domain_name)?;
        writeln!(f, "  problem_name: {}", self.problem_name)?;
        writeln!(f, "  requirements: {:?}", self.requirements)?;
        writeln!(f, "  types: {:?}", self.types)?;
        writeln!(f, "  constants: {:?}", self.constants)?;
        writeln!(f, "  predicates: {:?}", self.predicates)?;
        writeln!(f, "  functions: {:?}", self.functions)?;
        writeln!(f, "  domain_constraints: {}", self.domain_constraints)?;
        writeln!(f, "  tasks: {:?}", self.tasks)?;
        writeln!(f, "  actions: {:?}", self.actions)?;
        writeln!(f, "  methods: {:?}", self.methods)?;
        writeln!(f, "  objects: {:?}", self.objects)?;
        writeln!(f, "  init: {}", self.init)?;
        writeln!(f, "  goal: {}", self.goal)?;
        writeln!(f, "  problem_constraints: {}", self.problem_constraints)?;
        writeln!(f, "  metric_spec: {}", self.metric_spec)?;
        writeln!(f, "  length_spec: {}", self.length_spec)?;
        writeln!(f, "  initial_task_network: {:?}", self.initial_task_network)?;
        writeln!(f, "}}")
    }
}


impl SerdeSerializable for Problem {}
