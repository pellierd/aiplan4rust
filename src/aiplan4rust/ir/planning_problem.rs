use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use crate::aiplan4rust::ir::action::Action;
use crate::aiplan4rust::ir::atomic_skeleton::{Function, Predicate};
use crate::aiplan4rust::lang::{Ident, Requirement};

/// Represents a planning problem within a domain.
///
/// This struct contains the domain and problem identifiers,
/// along with the requirements, predicates, functions, and actions defined for the problem.
///
/// # Examples
///
/// ```
/// let mut problem = PlanningProblem::new();
/// problem.set_domain_name(Ident::new("my_domain"));
/// problem.set_problem_name(Ident::new("my_problem"));
/// assert_eq!(problem.domain_name().as_str(), "my_domain");
/// assert_eq!(problem.problem_name().as_str(), "my_problem");
/// assert!(problem.requirements().is_empty());
/// assert!(problem.predicates().is_empty());
/// assert!(problem.functions().is_empty());
/// assert!(problem.actions().is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PlanningProblem {
    /// The identifier of the domain.
    domain_name: Ident,

    /// The identifier of the problem.
    problem_name: Ident,

    /// The set of requirements for this planning problem.
    requirements: HashSet<Requirement>,

    /// The list of predicates in the planning problem.
    predicates: Vec<Predicate>,

    /// The list of functions in the planning problem.
    functions: Vec<Function>,

    /// The list of actions defined in this planning problem.
    actions: Vec<Action>,
}

impl PlanningProblem {
    /// Creates a new empty `PlanningProblem` with default identifiers and no requirements, predicates, functions or actions.
    ///
    /// By default, `domain_name` and `problem_name` are set to invalid (`Ident::default()`).
    pub fn new() -> Self {
        Self {
            domain_name: Ident::default(),
            problem_name: Ident::default(),
            requirements: HashSet::new(),
            predicates: Vec::new(),
            functions: Vec::new(),
            actions: Vec::new(),
        }
    }

    /// Returns the domain name.
    pub fn domain_name(&self) -> Ident {
        self.domain_name
    }

    /// Sets the domain name.
    pub fn set_domain_name(&mut self, name: Ident) {
        self.domain_name = name;
    }

    /// Returns the problem name.
    pub fn problem_name(&self) -> Ident {
        self.problem_name
    }

    /// Sets the problem name.
    pub fn set_problem_name(&mut self, name: Ident) {
        self.problem_name = name;
    }

    /// Returns a reference to the set of requirements.
    pub fn requirements(&self) -> &HashSet<Requirement> {
        &self.requirements
    }

    /// Returns a mutable reference to the set of requirements.
    pub fn requirements_mut(&mut self) -> &mut HashSet<Requirement> {
        &mut self.requirements
    }

    /// Adds a requirement to the planning problem.
    ///
    /// If the requirement already exists, this has no effect.
    pub fn add_requirement(&mut self, requirement: Requirement) {
        self.requirements.insert(requirement);
    }

    /// Removes a requirement from the planning problem.
    ///
    /// Returns `true` if the requirement was present and removed.
    pub fn remove_requirement(&mut self, requirement: &Requirement) -> bool {
        self.requirements.remove(requirement)
    }

    /// Adds a predicate to the planning problem.
    pub fn add_predicate(&mut self, predicate: Predicate) {
        self.predicates.push(predicate);
    }

    /// Returns a reference to the list of predicates.
    pub fn predicates(&self) -> &Vec<Predicate> {
        &self.predicates
    }

    /// Returns a mutable reference to the list of predicates.
    pub fn predicates_mut(&mut self) -> &mut Vec<Predicate> {
        &mut self.predicates
    }

    /// Adds a function to the planning problem.
    pub fn add_function(&mut self, function: Function) {
        self.functions.push(function);
    }

    /// Returns a reference to the list of functions.
    pub fn functions(&self) -> &Vec<Function> {
        &self.functions
    }

    /// Returns a mutable reference to the list of functions.
    pub fn functions_mut(&mut self) -> &mut Vec<Function> {
        &mut self.functions
    }

    /// Adds an action to the planning problem.
    pub fn add_action(&mut self, action: Action) {
        self.actions.push(action);
    }

    /// Returns a reference to the list of actions.
    pub fn actions(&self) -> &Vec<Action> {
        &self.actions
    }

    /// Returns a mutable reference to the list of actions.
    pub fn actions_mut(&mut self) -> &mut Vec<Action> {
        &mut self.actions
    }
}
