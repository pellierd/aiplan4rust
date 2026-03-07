use itertools::Itertools;
use crate::aiplan4rust::grounding::error::GroundingError;
use crate::aiplan4rust::grounding::{config, problem, GroundingResult};
use crate::aiplan4rust::grounding::analysis::inertia::evaluator::InertiaEvaluator;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::DatalogEngine;
use crate::aiplan4rust::grounding::problem::Problem;
use crate::aiplan4rust::grounding::passes::{positive_form_normalization, quantifier_expansion, type_flattening};
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lang::ids;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::renderers;
use crate::aiplan4rust::lir::renderers::{LiftedSyntaxDisplay, RenderContext};
use crate::analysis::inertia::InertiaTable;
use crate::analysis::reachability::datalog::error::DatalogError;
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

        // 6. PNF

        let negated_predicates = positive_form_normalization::to_pnf(&mut lifted_problem)?;

        let mut datalog = DatalogEngine::new();
        datalog.load_problem(&lifted_problem)?;

        datalog.run();

        // Calcul de l'atteignabilité
        //let actions = datalog.get_reachable_actions();
        //let fluents = datalog.get_reachable_fluents();
        let types = datalog.get_type_extensions();


        print!("{}", lifted_problem.domain_view().to_syntax_string());
        print!("{}", lifted_problem.problem_view().to_syntax_string());

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
            // 1. On récupère l'ActionDefId calculé par Datalog (ex: 5)
            let action_def_id = action_tuple.symbol();

            // 2. On récupère la définition de l'action dans le problème "Lifted"
            // On utilise .get() pour être ultra sécurisé
            if let Some(action_def) = lifted_problem.action_defs().get(action_def_id.as_usize()) {

                // 3. On extrait le NameID que l'action possède elle-même.
                // C'est cet ID qui est synchronisé avec le Registry du LIR.
                let action_name_id = action_def.name();

                // 4. On demande au registre de traduire ce NameID précis.
                // On utilise try_get_ident car c'est la méthode qui log l'erreur au lieu de paniquer.
                let symbol_id = lifted_problem.action_symbols().try_get_ident(action_name_id)?;

                // 5. On résout le nom final via l'interner
                let action_name = lifted_problem.interner()
                    .try_resolve_symbol(*symbol_id)
                    .unwrap_or("Action_Inconnue");


                // --- DÉBUT DU TRAITEMENT DES ARGUMENTS ---

                // 1. On prépare un vecteur pour stocker les noms des objets
                let mut arg_names = Vec::with_capacity(action_tuple.args().len());

                // 2. On parcourt les ObjectIds contenus dans la tuple Datalog
                for (i, &obj_id) in action_tuple.args().iter().enumerate() {

                    // 3. Traduction : ObjectId (Datalog) -> SymbolId (Interner)
                    // On utilise le registre des objets du problème
                    let obj_symbol_id = match lifted_problem.object_symbol().try_get_ident(obj_id) {
                        Ok(sym_id) => sym_id,
                        Err(e) => {
                            // Si on arrive ici, c'est que Datalog a trouvé un ID d'objet
                            // qui n'existe pas dans le registre initial (très rare avec ton nouveau moteur)
                            log::error!("Erreur d'argument pour {}: index {} introuvable", action_name, obj_id.as_usize());
                            continue;
                        }
                    };

                    // 4. Résolution : SymbolId -> String (ex: "ball1")
                    let arg_name = lifted_problem.interner()
                        .try_resolve_symbol(*obj_symbol_id)
                        .unwrap_or("<objet_inconnu>");

                    arg_names.push(arg_name.to_string());
                }

                // 5. Affichage final formaté
                if arg_names.is_empty() {
                    println!("{} (Action sans paramètres)", action_name);
                } else {
                    println!("{}({})", action_name, arg_names.join(", "));
                }


                // Continue ici ton traitement des arguments (action_tuple.args()...)
            } else {
                println!("Action inconnue avec ID {}", action_def_id.as_usize());
            }
        }


        println!("--- DIAGNOSTIC DES FLUENTS ACCESSIBLES ---");
        let reachable_fluents = datalog.get_reachable_fluents();
        println!("Nombre de fluents trouvés : {}", reachable_fluents.len());

        for fluent in reachable_fluents {
            // 1. Extraction de l'ID du prédicat (qui peut avoir le MSB à 1)
            let sk_id = fluent.symbol(); // Supposons que cela retourne ton type PredicateId

            if let Some(predicat_def) = lifted_problem.predicate_defs().get(sk_id.as_usize()) {
                let predicate_id = predicat_def.symbol();
                let pred_name_symbol_id = lifted_problem.predicate_symbols().try_get_ident(predicate_id)?;
                let pred_name = lifted_problem.interner().try_resolve_symbol(*pred_name_symbol_id)?;
                if sk_id.is_negated() {
                    format!("not {}", pred_name);
                }

                // 4. Traitement des arguments (ObjectIds -> Noms)
                let mut arg_names = Vec::with_capacity(fluent.args().len());
                for &obj_id in fluent.args() {
                    let arg_name = if let Ok(sym_id) = lifted_problem.object_symbol().try_get_ident(obj_id) {
                        lifted_problem.interner()
                            .try_resolve_symbol(*sym_id)
                            .unwrap_or("<objet_inconnu>")
                    } else {
                        "<id_objet_invalide>"
                    };
                    arg_names.push(arg_name);
                }


                if arg_names.is_empty() {
                    println!("{}()", pred_name);
                } else {
                    println!("{}({})", pred_name, arg_names.join(", "));
                }
            } else {
                println!("erreur: fluent inconnu avec ID {}", sk_id.as_usize());
            }
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
