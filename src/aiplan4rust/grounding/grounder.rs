use crate::aiplan4rust::grounding::analysis::inertia::evaluator::InertiaEvaluator;
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::passes::{positive_form_normalization, quantifier_expansion};
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::grounding::problem::Problem;
use crate::aiplan4rust::grounding::{config, GroundingResult};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::renderers::LiftedSyntaxDisplay;
use crate::analysis::inertia::InertiaTable;
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

        let mut datalog =
            DatalogEngine::new(&lifted_problem, &registry, &table, &negated_predicates);
        datalog.load_problem()?;

        datalog.run();

        // Calcul de l'atteignabilité
        /*let actions = datalog.get_reachable_actions();
        let fluents = datalog.get_reachable_fluents();
        let types = datalog.get_type_extensions();

        let registry = lifted_problem.action_symbols();

        // APPEL DU DIAGNOSTIC ICI
        println!(
            "--- DIAGNOSTIC DES ACTIONS ---{}",
            datalog.get_reachable_actions().len()
        );
        // 1. On récupère les définitions
        let action_defs = lifted_problem.action_defs();

        // 2. On FORCE la récupération d'un registre FRAIS
        // (Cela devrait normalement reconstruire la table de correspondance)
        let registry = lifted_problem.action_symbols();

        ///println!("Vérification Registry: taille = {}", registry.len());
        for action_tuple in datalog.get_reachable_actions() {
            let action_label = action::render(&action_tuple, &lifted_problem);
            println!("Action: {}", action_label);

            let action_sk_id = datalog.action_id_to_skeleton(action_tuple.symbol());

            // 1. On récupère maintenant des couples (Atom, Cause)
            let effects_with_causes = datalog.get_effects_for_action(action_sk_id);

            /*for (effect, cause) in effects_with_causes {
                let prefix = if effect.is_negated() { "[-] " } else { "[+] " };
                let type_label = if let Cause::Pivot(_) = cause {
                    "[COND] "
                } else {
                    ""
                };
                let sk_id = effect.skeleton_id();
                let raw_id = sk_id.as_usize();

                // --- LOGIQUE DE DÉCODAGE ULTRA-SÉCURISÉE ---
                let (name, is_aux) = if datalog.is_negated_fluent(sk_id) {
                    let pos_id = datalog.pos_id_from_negated(sk_id);
                    let pos_raw = pos_id.as_usize();
                    if pos_raw < lifted_problem.predicate_defs().len() {
                        (
                            format!("not_{}", lifted_problem.predicate_defs()[pos_raw].symbol()),
                            false,
                        )
                    } else {
                        (format!("not_UNKNOWN_{}", pos_raw), false)
                    }
                } else if datalog.is_fluent(sk_id) {
                    if raw_id < lifted_problem.predicate_defs().len() {
                        (
                            lifted_problem.predicate_defs()[raw_id].symbol().to_string(),
                            false,
                        )
                    } else {
                        (format!("FLUENT_OUT_OF_BOUNDS_{}", raw_id), false)
                    }
                } else if datalog.is_auxiliary(sk_id) {
                    (format!("AUX_Piv_{}", raw_id), true)
                } else {
                    (format!("ID_{}", raw_id), false)
                };

                // --- ARGUMENTS AVEC FALLBACK ---
                let args: Vec<String> = effect
                    .terms()
                    .iter()
                    .map(|t| match t {
                        Term::Constant(c) => format!("{:?}", c),
                        Term::Variable(v) => action_tuple
                            .args()
                            .get(v.as_usize())
                            .map(|obj| format!("{:?}", obj))
                            .unwrap_or_else(|| format!("var_{}", v.as_usize())),
                    })
                    .collect();

                println!(
                    "  {}{}{} {}({}) [ID:{}]",
                    prefix,
                    type_label,
                    if effect.is_negated() { "Del" } else { "Add" },
                    name,
                    args.join(", "),
                    raw_id
                );
            }
            println!("---");*/
        }

        println!("\n--- DIAGNOSTIC DES AUXILIAIRES (Pivots Logiques) ---");
        let auxiliaries = datalog.get_reachable_auxiliaries();
        for aux_tuple in auxiliaries {
            // Utilisation de ta nouvelle fonction
            let label = auxiliary::render(&aux_tuple, &lifted_problem);
            println!("  {}", label);
        }

        println!("--- DIAGNOSTIC DES FLUENTS ACCESSIBLES ---");
        let reachable_fluents = datalog.get_reachable_fluents();
        println!(
            "Nombre total de faits accessibles (Datalog) : {}",
            reachable_fluents.len()
        );

        let mut fluent_count = 0;

        for fluent in reachable_fluents {
            // Si c'est un fluent, on l'affiche et on le compte pour le futur BitVector
            if evaluator.is_fluent(fluent.symbol()) {
                fluent_count += 1;
                let fluent_label = fluent::render(&fluent, &lifted_problem);
                println!(
                    "[DYNAMIC] Fluent ID {}: {}",
                    fluent.symbol().as_usize(),
                    fluent_label
                );
            } else {
                // Optionnel : log pour vérifier ce qui est éliminé (Inerties ou IDs 10/13)
                // println!("[STATIC] Fact ignored (Inertia/Synthetic): {:?}", fluent.symbol());
            }
        }

        println!(
            "Nombre final de fluents indexés (BitVector size) : {}",
            fluent_count
        );*/

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
