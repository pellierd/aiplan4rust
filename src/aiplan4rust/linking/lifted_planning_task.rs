use crate::aiplan4rust::semantics::annotated_syntax_tree::{LiftedDomain, LiftedProblem};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a lifted planning task consisting of a domain and a problem.
///
/// This structure holds both the lifted domain and problem, which are used in the planning process.
/// The domain defines the possible actions, predicates, and other relevant information for planning,
/// while the problem specifies the initial state, goal, and other problem-specific details.
///
/// # Fields
/// - `domain`: The lifted domain that defines the planning problem's actions and predicates.
/// - `problem`: The lifted problem that defines the initial state and goal for the planning task.
///
/// # Methods
/// - `new(domain: LiftedDomain, problem: LiftedProblem)`: Creates a new `LiftedPlanningTask`
///   from a domain and problem.
/// - `domain()`: Returns an immutable reference to the `LiftedDomain`.
/// - `problem()`: Returns an immutable reference to the `LiftedProblem`.
/// - `domain_mut()`: Returns a mutable reference to the `LiftedDomain` to allow modification.
/// - `problem_mut()`: Returns a mutable reference to the `LiftedProblem` to allow modification.
///
/// # Display Implementation
/// The `Display` trait is implemented to provide a human-readable string representation of the
/// `LiftedPlanningTask`. It includes general information about the domain and problem, along with
/// their details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiftedPlanningTask {
    domain: LiftedDomain,
    problem: LiftedProblem,
}

impl LiftedPlanningTask {
    /// Creates a new `LiftedPlanningTask` from the provided domain and problem.
    ///
    /// # Arguments
    /// - `domain`: A `LiftedDomain` that defines the actions and predicates for the task.
    /// - `problem`: A `LiftedProblem` that specifies the initial state, goal, and other details.
    ///
    /// # Returns
    /// A new instance of `LiftedPlanningTask`.
    pub fn new(domain: LiftedDomain, problem: LiftedProblem) -> Self {
        LiftedPlanningTask { domain, problem }
    }

    /// Returns an immutable reference to the `LiftedDomain` of the planning task.
    ///
    /// # Returns
    /// An immutable reference to the `LiftedDomain` struct.
    pub fn domain(&self) -> &LiftedDomain {
        &self.domain
    }

    /// Returns an immutable reference to the `LiftedProblem` of the planning task.
    ///
    /// # Returns
    /// An immutable reference to the `LiftedProblem` struct.
    pub fn problem(&self) -> &LiftedProblem {
        &self.problem
    }

    /// Returns a mutable reference to the `LiftedDomain` of the planning task.
    ///
    /// # Returns
    /// A mutable reference to the `LiftedDomain` struct, allowing modifications to the domain.
    pub fn domain_mut(&mut self) -> &mut LiftedDomain {
        &mut self.domain
    }

    /// Returns a mutable reference to the `LiftedProblem` of the planning task.
    ///
    /// # Returns
    /// A mutable reference to the `LiftedProblem` struct, allowing modifications to the problem.
    pub fn problem_mut(&mut self) -> &mut LiftedProblem {
        &mut self.problem
    }
}

impl fmt::Display for LiftedPlanningTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display the general information about the LiftedPlanningTask
        write!(f, "Lifted Planning Task Information:\n")?;

        // Display domain details
        write!(f, "Domain: \n{}", self.domain())?;

        // Display problem details
        write!(f, "Problem: \n{}", self.problem())?;

        Ok(())
    }
}
