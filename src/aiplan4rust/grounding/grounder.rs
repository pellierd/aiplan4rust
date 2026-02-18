use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::{analysis, GroundingResult};
use crate::aiplan4rust::grounding::problem::Problem;
use crate::aiplan4rust::grounding::passes::type_flattening;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::DiagnosticManager;

/// The `Grounder` is responsible for converting a lifted planning problem
/// into a fully grounded problem, instantiating all types, objects, predicates,
/// functions, and actions. It maintains a diagnostic manager to collect
/// warnings, errors, or other messages encountered during the grounding process.
#[derive(Debug, Default)]
pub struct Grounder {
    diagnostic_manager: DiagnosticManager,
}

impl Grounder {
    /// Creates a new `Grounder` with an empty diagnostic manager.
    pub fn new() -> Self {
        Grounder {
            diagnostic_manager: DiagnosticManager::new(),
        }
    }

    /// Returns a reference to the internal diagnostic manager.
    ///
    /// # Example
    /// ```
    /// let grounder = Grounder::new();
    /// let diag = grounder.diagnostic_manager();
    /// ```
    pub fn diagnostic_manager(&self) -> &DiagnosticManager {
        &self.diagnostic_manager
    }

    /// Performs grounding on a lifted problem, returning a `GroundingResult`
    /// that contains the grounded problem and all collected diagnostics.
    ///
    /// # Arguments
    /// * `lifted_problem` - The lifted planning problem to ground.
    ///
    /// # Returns
    /// * `Ok(GroundingResult)` on success.
    /// * `Err(GroundingError)` if grounding fails.
    ///
    /// # Example
    /// ```
    /// let mut grounder = Grounder::new();
    /// let grounded = grounder.ground(lifted_problem)?;
    /// ```
    pub fn ground(
        &mut self,
        mut lifted_problem: LiftedProblem,
    ) -> Result<GroundingResult, GroundingError> {

        // Perform type inference and flatten the lifted problem.
        type_flattening::problem::flatten(&mut lifted_problem)?;


        let table = analysis::inertia::analyzer::analyze(&lifted_problem)?;



        println!("{}", table);
        let problem = Problem::from(lifted_problem);



        Ok(GroundingResult::success(
            problem,
            std::mem::take(&mut self.diagnostic_manager),
        ))
    }

    /// Sets a custom diagnostic manager and performs grounding on a lifted problem.
    ///
    /// # Arguments
    /// * `lifted_problem` - The lifted problem to ground.
    /// * `diagnostic_manager` - A diagnostic manager to replace the internal one.
    ///
    /// # Returns
    /// * `Ok(GroundingResult)` on success.
    /// * `Err(GroundingError)` if grounding fails.
    ///
    /// # Example
    /// ```
    /// let mut grounder = Grounder::new();
    /// let diag_manager = DiagnosticManager::new();
    /// let grounded = grounder.build_with_diagnostic_manager(lifted_problem, diag_manager)?;
    /// ```
    pub fn build_with_diagnostic_manager(
        &mut self,
        lifted_problem: LiftedProblem,
        diagnostic_manager: DiagnosticManager,
    ) -> Result<GroundingResult, GroundingError> {
        self.diagnostic_manager = diagnostic_manager;
        self.ground(lifted_problem)
    }
}
