use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::{config, GroundingResult};
use crate::aiplan4rust::grounding::analysis::inertia::evaluator::InertiaEvaluator;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::DatalogEngine;
use crate::aiplan4rust::grounding::problem::Problem;
use crate::aiplan4rust::grounding::passes::{quantifier_expansion, type_flattening};
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::analysis::inertia::InertiaTable;
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

        // 1. TYPE FLATTENING : On résout la hiérarchie (A est un B).
        // Obligatoire avant tout car tout le reste en dépend.
        type_flattening::problem::flatten(&mut lifted_problem)?;

        // 2. OBJECT FLUENT FLATTENING
        // TO DO

        // 3. ANALYSE D'INERTIE : On identifie ce qui ne change jamais.
        let table = InertiaTable::build(&lifted_problem)?;
        // 3. VALUE REGISTRY CONSTRUCTION
        // We pass type/object definitions separately and inject the initial size config.
        let registry = ValueRegistry::build(
            lifted_problem.type_defs(),
            lifted_problem.object_defs(),
            config::DEFAULT_VALUE_REGISTRY_SIZE,
        )?;


        let evaluator = InertiaEvaluator::build(
            lifted_problem.predicate_defs(),
            lifted_problem.function_defs(),
            lifted_problem.init(),
            &table,
            &registry,
            config::DEFAULT_MAX_ARITY,
            config::DEFAULT_MAX_PROJ,
        )?;

        // 5. QUANTIFIER EXPANSION : On déploie les forall/exists.
        // Il doit arriver APRES le flattening des types pour que le forall
        // sache exactement sur quels objets itérer.
        quantifier_expansion::problem::expand_with(&mut lifted_problem, &registry, Some(&evaluator))?;

        let mut datalog = DatalogEngine::new();
        datalog.load_problem(&lifted_problem)?;


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
