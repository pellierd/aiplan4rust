use serde::{Deserialize, Serialize};
use crate::aiplan4rust::ir::action::Action;
use crate::aiplan4rust::lang::Ident;

/// Represents a planning problem in the domain.
///
/// This struct holds the names of the domain and problem,
/// as well as the list of actions defined in the problem.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PlanningProblem {
    /// The identifier of the domain.
    domain_name: Ident,

    /// The identifier of the problem.
    problem_name: Ident,

    /// The list of actions defined in this planning problem.
    actions: Vec<Action>,
}

impl PlanningProblem {
    /// Creates a new empty `PlanningProblem` with default identifiers and no actions.
    ///
    /// By default, `domain_name` and `problem_name` are set to invalid (`Ident::default()`).
    ///
    /// # Example
    /// ```
    /// let problem = PlanningProblem::new();
    /// assert!(problem.domain_name.is_invalid());
    /// assert!(problem.actions.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            domain_name: Ident::default(),
            problem_name: Ident::default(),
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
