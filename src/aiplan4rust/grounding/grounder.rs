use crate::aiplan4rust::grounding::analysis::inertia::evaluator::InertiaEvaluator;
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::passes::{positive_form_normalization, quantifier_expansion};
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::grounding::problem::Problem;
use crate::aiplan4rust::grounding::{config, GroundingResult};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::renderers::LiftedSyntaxDisplay;
use crate::analysis::inertia::InertiaTable;
use crate::analysis::reachability::datalog::renderers::{action, fluent};
use crate::{DatalogEngine, DiagnosticManager};

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
        // 1. OBJECT FLUENT FLATTENING
        // TO DO

        // 2. ANALYSE D'INERTIE : On identifie ce qui ne change jamais.
        let table = InertiaTable::build(&lifted_problem)?;

        // 3. VALUE REGISTRY CONSTRUCTION
        // We pass typing/object definitions separately and inject the initial size config.
        let registry =
            ValueRegistry::build(lifted_problem.type_defs(), lifted_problem.object_defs())?;

        println!("{}", lifted_problem);
        println!("{}", registry);

        let evaluator = InertiaEvaluator::build(
            lifted_problem.predicate_defs(),
            lifted_problem.function_defs(),
            lifted_problem.init(),
            &table,
            &registry,
            config::DEFAULT_MAX_ARITY,
            config::DEFAULT_MAX_PROJ,
        )?;

        print!("{}", lifted_problem.domain_view().to_syntax_string());
        //print!("{}", lifted_problem.problem_view().to_syntax_string());
        //println!("{}", lifted_problem);
        // 5. QUANTIFIER EXPANSION : On déploie les forall/exists.
        // Il doit arriver APRES le flattening des types pour que le forall
        // sache exactement sur quels objets itérer.
        quantifier_expansion::problem::expand_with(
            &mut lifted_problem,
            &registry,
            Some(&evaluator),
        )?;

        print!("{}", lifted_problem.domain_view().to_syntax_string());
        //print!("{}", lifted_problem.problem_view().to_syntax_string());
        //println!("{}", lifted_problem);
        // 6. PNF
        let negated_predicates = positive_form_normalization::to_pnf(&mut lifted_problem)?;

        let mut datalog = DatalogEngine::new();
        datalog.load_problem(&lifted_problem, &negated_predicates)?;

        datalog.run();

        // Calcul de l'atteignabilité
        let actions = datalog.get_reachable_actions();
        let fluents = datalog.get_reachable_fluents();
        let types = datalog.get_type_extensions();

        let registry = lifted_problem.action_symbols();

        // APPEL DU DIAGNOSTIC ICI
        println!("--- DIAGNOSTIC DES ACTIONS ---");
        // 1. On récupère les définitions
        let action_defs = lifted_problem.action_defs();

        // 2. On FORCE la récupération d'un registre FRAIS
        // (Cela devrait normalement reconstruire la table de correspondance)
        let registry = lifted_problem.action_symbols();

        println!("Vérification Registry: taille = {}", registry.len());

        for action_tuple in datalog.get_reachable_actions() {
            // Appel de la fonction render avec les paramètres inversés (tuple, problème)
            // Elle renvoie directement une String "robuste"
            let action_label = action::render(&action_tuple, &lifted_problem);

            println!("{}", action_label);

            // Tu peux continuer ici ton traitement logique (BitSets, etc.)
            // action_tuple est toujours disponible pour extraire les IDs
        }

        println!("--- DIAGNOSTIC DES FLUENTS ACCESSIBLES ---");
        let reachable_fluents = datalog.get_reachable_fluents();
        println!("Nombre de fluents trouvés : {}", reachable_fluents.len());

        for fluent in reachable_fluents {
            // Appel à ton nouveau module : datalog::renderers::fluent
            let fluent_label = fluent::render(&fluent, &lifted_problem);

            println!("{}", fluent_label);
        }

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
