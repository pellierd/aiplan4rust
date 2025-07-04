use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::lir::lifted::{LiftedAction, LiftedMethod};
use crate::aiplan4rust::lir::def::{FunctionDef, PredicateDef, TaskDef};
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lang::{Ident, Requirement, TypedSymbol};

/// Represents a planning problem within a domain.
///
/// This struct contains the domain and problem identifiers,
/// along with the requirements, types, constants, predicates, functions,
/// and actions defined for the problem.
///
/// # Examples
///
/// ```
/// let mut problem = PlanningProblem::new();
/// problem.set_domain_name(Ident::new("my_domain"));
/// problem.set_problem_name(Ident::new("my_problem"));
/// assert_eq!(problem.domain_name().as_str(), "my_domain");
/// assert_eq!(problem.problem_name().as_str(), "my_problem");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Problem {
    /// The identifier of the domain.
    domain_name: Ident,

    /// The identifier of the problem.
    problem_name: Ident,

    /// The set of requirements for this planning problem.
    requirements: HashSet<Requirement>,

    /// The set of types defined in this planning problem.
    types: HashSet<TypedSymbol>,

    /// The set of constants defined in this planning problem.
    constants: HashSet<TypedSymbol>,

    /// The list of predicates in the planning problem.
    predicates: Vec<PredicateDef>,

    /// The list of functions in the planning problem.
    functions: Vec<FunctionDef>,

    /// The constraints for this planning problem,
    /// No constraints are represented by an empty and `Expr'.
    constraints: Expr,

    /// The list of tasks defined in this planning problem.
    tasks: Vec<TaskDef>,

    /// The list of actions defined in this planning problem.
    actions: Vec<LiftedAction>,

    /// The list of methods defined in this planning problem.
    methods: Vec<LiftedMethod>,
}

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
            domain_name: Ident::default(),
            problem_name: Ident::default(),
            requirements: HashSet::new(),
            types: HashSet::new(),
            constants: HashSet::new(),
            predicates: Vec::new(),
            functions: Vec::new(),
            constraints: Expr::empty_and(),
            tasks: Vec::new(),
            actions: Vec::new(),
            methods: Vec::new(),
        }
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

    /// Adds a single type.
    ///
    /// If the type already exists, it is not added again.
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
    pub fn predicates(&self) -> &Vec<PredicateDef> {
        &self.predicates
    }

    /// Returns a mutable reference to the list of predicates.
    pub fn predicates_mut(&mut self) -> &mut Vec<PredicateDef> {
        &mut self.predicates
    }

    /// Adds a single predicate.
    pub fn add_predicate(&mut self, predicate: PredicateDef) {
        self.predicates.push(predicate);
    }

    /// Adds multiple predicates.
    pub fn add_predicates<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item =PredicateDef>,
    {
        self.predicates.extend(iter);
    }

    // === Functions ===

    /// Returns a reference to the list of functions.
    pub fn functions(&self) -> &Vec<FunctionDef> {
        &self.functions
    }

    /// Returns a mutable reference to the list of functions.
    pub fn functions_mut(&mut self) -> &mut Vec<FunctionDef> {
        &mut self.functions
    }

    /// Adds a single function.
    pub fn add_function(&mut self, function: FunctionDef) {
        self.functions.push(function);
    }

    /// Adds multiple functions.
    pub fn add_functions<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item =FunctionDef>,
    {
        self.functions.extend(iter);
    }

    // === Constraints ===

    /// Returns a reference to the constraints expression.
    pub fn constraints(&self) -> &Expr {
        &self.constraints
    }

    /// Returns a mutable reference to the constraints expression.
    pub fn constraints_mut(&mut self) -> &mut Expr {
        &mut self.constraints
    }

    /// Sets the constraints expression.
    pub fn set_constraints(&mut self, constraints: Expr) {
        self.constraints = constraints;
    }

    // === Tasks ===

    /// Returns a reference to the list of tasks.
    pub fn tasks(&self) -> &Vec<TaskDef> {
        &self.tasks
    }

    /// Returns a mutable reference to the list of tasks.
    pub fn tasks_mut(&mut self) -> &mut Vec<TaskDef> {
        &mut self.tasks
    }

    /// Adds a single task.
    pub fn add_task(&mut self, task: TaskDef) {
        self.tasks.push(task);
    }

    /// Adds multiple tasks.
    pub fn add_tasks<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item =TaskDef>,
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

}
