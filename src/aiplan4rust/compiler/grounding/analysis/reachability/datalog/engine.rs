//! # Datalog Engine Module
//!
//! This module provides the core [`DatalogEngine`] struct, which serves as the central
//! orchestrator for PDDL problem grounding and reachability analysis using Datalog saturation.
//!
//! ## Key Responsibilities
//! - **Problem Encoding:** Transforms lifted PDDL planning problems, types, actions, and initial states
//!   into relational facts, Datalog rules, and auxiliary definitions via [`DatalogEngine::encode`].
//! - **Multi-Stratum Execution:** Drives the semi-naive evaluation loop through multiple distinct strata
//!   (positive saturation, negation materialization, and conditional action activation) via [`DatalogEngine::run`].
//! - **Diagnostics & Rendering:** Provides utilities to dump database contents and compiled rules for inspection.
//! - **Testing Support:** Exposes internal mock utilities (`test_utils::create_segmented_engine`) for isolated unit testing.

use crate::aiplan4rust::compiler::grounding::analysis::inertia::table::InertiaTable;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::cause::Cause;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::database::Database;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::rule::Rule;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::error::DatalogError;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::renderers::{
    database, rules, DatalogRenderContext,
};
use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
use crate::aiplan4rust::compiler::lir::renderers::LiftedSyntaxDisplay;
use crate::aiplan4rust::support::lang::{AtomSkeletonId, Id, ObjectId, TypeId, TypedListId};
use crate::analysis::reachability::datalog::context::DatalogContext;
use crate::analysis::reachability::datalog::encoder;
use crate::analysis::reachability::datalog::encoder::aliasing;
use crate::analysis::reachability::datalog::scratchpad::DatalogScratchpad;
use crate::analysis::reachability::datalog::state::DatalogState;
use rustc_hash::FxHashMap;

/// Maximum number of variables (parameters) allowed per action or rule.
///
/// This limit is set to 64 to allow high-performance variable tracking
/// using a single CPU register (u64 bitset).
pub const MAX_VARS: usize = 64;

/// Core engine orchestrating the Datalog reachability analysis, grounding,
/// and fixed-point saturation process for PDDL planning problems.
pub struct DatalogEngine<'a> {
    /// Mutable reference to the lifted planning problem being processed.
    pub(crate) problem: &'a mut LiftedProblem,

    /// Registry mapping constants, objects, and values to unique internal identifiers.
    pub(crate) value_registry: &'a ValueRegistry,

    /// Inertia table classifying predicates as static or dynamic (fluent).
    pub(crate) inertia_table: &'a InertiaTable,

    /// Collection of atom skeleton IDs corresponding to negated predicates.
    pub(crate) negated_predicates: &'a Vec<AtomSkeletonId>,

    /// The relational database storing stable facts and working memory (deltas).
    pub(crate) db: Database,

    /// List of compiled Datalog inference rules.
    pub(crate) rules: Vec<Rule>,

    /// Active variable environment mapping variable indices to object IDs.
    pub(crate) current_env: [Option<ObjectId>; MAX_VARS],

    /// Undo stack tracking variable bindings for efficient backtracking and rollbacks.
    pub(crate) trailing_indices: Vec<usize>,

    /// Temporary buffer used to store facts discovered during rule evaluation.
    pub(crate) discovered_facts: Vec<(AtomSkeletonId, Vec<ObjectId>)>,

    /// Temporary buffer for constructing rule head argument lists.
    pub(crate) head_buffer: Vec<ObjectId>,

    /// Threshold boundary separating standard predicates from subsequent segments.
    pub(crate) fluence_threshold: usize,

    /// Threshold marking the boundary for type-related predicates.
    pub(crate) type_threshold: usize,

    /// Starting index offset for the type segment partition.
    pub(crate) type_segment_start: usize,

    /// Base identifier offset allocated for action representations.
    pub(crate) action_base_id: usize,

    /// Threshold marking the end of the action segment.
    pub(crate) action_threshold: usize,

    /// Threshold marking the boundary for built-in evaluation predicates.
    pub(crate) builtin_threshold: usize,

    /// Structural cache preventing duplicate union predicates.
    /// Key: Sorted list of `TypeId`s. Value: Corresponding Datalog skeleton ID.
    pub(crate) union_cache: FxHashMap<Vec<TypeId>, AtomSkeletonId>,

    // ENCODER //
    /// The starting offset for auxiliary predicate IDs.
    /// Typically set to the count of original predicates in the domain.
    pub(crate) base_aux_id: usize,

    /// Monotonic counter for generating the next unique auxiliary ID.
    pub(crate) next_aux_id: usize,

    /// Registry of auxiliary predicate signatures (skeletons).
    /// Used for debugging and reconstructing the logical state post-saturation.
    pub(crate) aux_defs: Vec<AtomicFormulaSkeleton>,

    /// Structural cache mapping a set of body atoms to a head atom.
    /// Prevents the redundant creation of multiple auxiliary predicates
    /// for the same logical sub-expression (Common Subexpression Elimination).
    pub(crate) cache: FxHashMap<Vec<Atom>, Atom>,

    /// Causality table associating each effect to its origin (Action or Pivot).
    pub(crate) action_effects: Vec<Vec<(Atom, Cause)>>,

    /// Optional anchor atom representing the active action context during evaluation.
    pub(crate) action_anchor: Option<Atom>,

    /// Offset applied to handle predicate negation correctly within the database.
    pub(crate) negation_offset: usize,

    /// Mapping linking type identifiers to their respective atom skeletons.
    pub(crate) type_to_skeleton: Vec<AtomSkeletonId>,
}

impl<'a> DatalogEngine<'a> {
    /// Encodes a lifted planning problem into a Datalog-based representation,
    /// populating initial facts, type hierarchies, action schemas, and inference rules.
    ///
    /// This method performs the initial compilation pipeline phases, including store extraction,
    /// threshold allocation, schema declaration, database ingestion, and rule generation.
    ///
    /// # Arguments
    /// * `problem` - Mutable reference to the `LiftedProblem` to be encoded and analyzed.
    /// * `value_registry` - Reference to the `ValueRegistry` mapping constants and values.
    /// * `inertia_table` - Reference to the `InertiaTable` classifying predicates as static or fluent.
    /// * `negated_predicates` - Reference to a vector of `AtomSkeletonId` corresponding to negated predicates.
    ///
    /// # Returns
    /// * `Ok(Self)` - A fully initialized and populated `DatalogEngine` ready for fixed-point execution.
    /// * `Err(DatalogError)` - Returns a datalog error if any schema declaration, fact ingestion, or rule encoding step fails.
    ///
    /// # Complexity
    /// * Time complexity: Proportional to the size of the problem domain, number of actions, and initial expression tree complexity.
    /// * Space complexity: Proportional to the number of generated rules, auxiliary definitions, and total facts inserted into the database.
    pub fn encode(
        problem: &'a mut LiftedProblem,
        value_registry: &'a ValueRegistry,
        inertia_table: &'a InertiaTable,
        negated_predicates: &'a Vec<AtomSkeletonId>,
    ) -> Result<Self, DatalogError> {
        // =========================================================================
        // 1. EXTRACTION DU STORE ET THRESHOLDS INITIALES
        // =========================================================================
        let mut local_store = problem.take_store();

        let fluence_threshold = problem.predicate_defs().len();
        let type_segment_start = if negated_predicates.is_empty() {
            fluence_threshold
        } else {
            fluence_threshold * 2
        };
        let action_count = problem.action_defs().len();
        let init_expr_id = problem.init();

        let type_defs_slice = problem.type_defs().as_slice();
        let action_defs_slice = problem.action_defs();
        let object_defs_slice = problem.object_defs().as_slice();

        // =========================================================================
        // 2. REPRODUCTION DE L'ORDRE DES SEUILS (ARCHITECTURE PROPRE)
        // =========================================================================
        let mut type_to_skeleton = Vec::new();
        let mut current_id = type_segment_start;
        let mut aux_defs = Vec::with_capacity(256);

        let mut db = Database::new();
        let mut rules = Vec::with_capacity(1024);
        let mut cache = FxHashMap::with_capacity_and_hasher(256, Default::default());
        let mut current_aliases = aliasing::new_alias_table();
        let mut action_effects = vec![Vec::new(); action_count];
        let union_cache = FxHashMap::default();

        let mut when_cache = FxHashMap::default();

        // On crée le State pour orchestrer les enregistrements mutables
        let mut state = DatalogState::new(
            &mut rules,
            &mut cache,
            &mut when_cache,
            &mut aux_defs,
            &mut current_id,
            &mut db,
            &mut current_aliases,
        );

        // 1. Déclaration et remplissage de type_to_skeleton via le State unifié
        encoder::facts::declare_type_defs(type_defs_slice, &mut type_to_skeleton, &mut state);

        let type_threshold = type_segment_start;
        let action_base_id = type_threshold;

        // 2. Déclaration des actions via le State unifié
        encoder::facts::declare_action_defs(action_defs_slice, &mut state)?;

        let action_threshold = *state.next_aux_id;
        let builtin_threshold = action_threshold;

        // =========================================================================
        // 3. INGESTION DES DONNÉES ET COMPILATION DES RÈGLES
        // =========================================================================
        // Initialisation du contexte (maintenant que type_to_skeleton est prêt et figé)
        let ctx = DatalogContext::new(
            TypedListId::default(),
            &type_to_skeleton,
            fluence_threshold,
            inertia_table,
        );

        // 3. Nouvelle signature propre pour fill_db_from_objects (ctx + state)
        encoder::facts::fill_db_from_objects(ctx, &mut state, object_defs_slice, type_defs_slice)?;

        // Dans pub fn encode, section 3 :
        encoder::facts::fill_db_from_init(&mut state, init_expr_id, &mut local_store)?;

        // Compilation des règles
        let mut scratchpad = DatalogScratchpad::with_capacity(local_store.len());
        encoder::action::encode_action_defs(
            ctx,
            &mut state,
            &mut action_effects,
            action_defs_slice,
            action_base_id,
            &mut scratchpad,
            &mut local_store,
        )?;

        let final_builtin_threshold = *state.next_aux_id;

        // =========================================================================
        // 4. RESTAURATION ET EMPAQUETAGE
        // =========================================================================
        problem.set_store(local_store);

        let engine = Self {
            problem,
            value_registry,
            inertia_table,
            negated_predicates,

            db,
            rules,
            current_env: [None; MAX_VARS],
            trailing_indices: Vec::with_capacity(MAX_VARS),
            discovered_facts: Vec::with_capacity(1024),
            head_buffer: Vec::with_capacity(16),
            union_cache,

            base_aux_id: type_segment_start,
            next_aux_id: final_builtin_threshold,
            aux_defs,
            cache,
            action_effects,
            action_anchor: None,
            negation_offset: fluence_threshold,
            type_to_skeleton,

            fluence_threshold,
            type_threshold,
            type_segment_start,
            action_base_id,
            action_threshold,
            builtin_threshold: final_builtin_threshold,
        };

        #[cfg(debug_assertions)]
        engine.dump_database();

        #[cfg(debug_assertions)]
        engine.dump_rules();

        Ok(engine)
    }

    /// Executes the multi-stratum Datalog fixed-point saturation process.
    ///
    /// This method orchestrates the evaluation pipeline in three distinct strata:
    /// 1. **Rule Optimization**: Refines and reorders rules for efficient execution.
    /// 2. **Stratum 0 (Positive Saturation)**: Computes all physical and positive facts using semi-naive evaluation.
    /// 3. **Stratum 1 (Negation Materialization)**: Evaluates negated predicates and default negations via the value registry.
    /// 4. **Stratum 2 (Conditional Actions)**: Re-runs semi-naive saturation to incorporate negative facts and trigger conditional action rules.
    ///
    /// # Complexity
    /// * Time complexity: Bounded by the number of possible ground facts and rule evaluation rounds until fixed-point convergence.
    /// * Space complexity: Proportional to the size of the working database, delta sets, and active rule bindings.
    pub fn run(&mut self) {
        // 'OPTIMISATION SE FAIT ICI, SANS EFFORT, CAR ENGINE EXISTE ENFIN !
        self.optimize_all_rules();

        // --- STRATE 0 : Le Positif ---
        // On calcule tout ce qui est "vrai" physiquement (at, part_of, etc.)
        self.saturate_semi_naive();

        // --- STRATE 1 : Le Pivot (Négation) ---
        // On utilise le ValueRegistry pour combler les trous.
        // Cette fonction va remplir le Delta avec les "not_incorporated".
        self.materialize_negations();

        // --- STRATE 2 : Les Actions conditionnelles ---
        // On relance. Maintenant, le moteur voit les faits négatifs
        // et peut enfin activer Aux_34 et Aux_35 pour l'Action 22.
        self.saturate_semi_naive();

        /*println!("--- DIAGNOSTIC POST-SATURATION (Action 22) ---");
        let dep_1 = AtomSkeletonId::from(35);
        let dep_2 = AtomSkeletonId::from(34);

        // 1. Vérification rapide
        println!(
            "  - Aux_35 (Partie A) est présent ? {}",
            self.db.relations().contains_key(&dep_1)
        );
        println!(
            "  - Aux_34 (Partie B) est présent ? {}",
            self.db.relations().contains_key(&dep_2)
        );

        // 2. Autopsie récursive pour comprendre QUEL fait manque
        println!("\n--- AUTOPSIE DE L'ÉCHEC POUR ASSEMBLE ---");
        if !self.db.relations().contains_key(&dep_1) {
            println!("Analyse de la branche 35 :");
            self.debug_recursive_rule(dep_1, 1);
        }
        if !self.db.relations().contains_key(&dep_2) {
            println!("Analyse de la branche 34 :");
            self.debug_recursive_rule(dep_2, 1);
        }*/
    }

    /// Dumps and renders the current contents of the Datalog database for diagnostic purposes.
    ///
    /// This method creates a rendering context snapshot of the current engine state
    /// and delegates the output formatting to the specialized database renderer.
    ///
    /// # Complexity
    /// * Time complexity: Proportional to the number of stored relations and facts in the database.
    /// * Space complexity: $O(1)$ auxiliary memory overhead during rendering.
    pub fn dump_database(&self) {
        // 1. On crée le Snapshot de données (le contexte)
        let ctx = self.render_context();

        // 2. On appelle le renderer spécialisé
        database::render(&ctx, &self.db);
    }

    /// Dumps and renders the compiled Datalog rules for inspection and debugging.
    ///
    /// Utilizes the current rendering context snapshot to format and output
    /// all active inference rules managed by the engine.
    ///
    /// # Complexity
    /// * Time complexity: Proportional to the number of compiled rules and their body sizes.
    /// * Space complexity: $O(1)$ auxiliary memory overhead during rendering.
    // Affiche la logique compilée (Rules)
    // On ne prend plus de paramètres, on utilise self.rules
    pub fn dump_rules(&self) {
        let ctx = self.render_context();
        rules::render(&ctx, &self.rules);
    }

    /// Constructs and returns a new rendering context snapshot (`DatalogRenderContext`).
    ///
    /// Bundles the problem reference, type skeletons, and critical structural thresholds
    /// required by external renderers to interpret internal identifiers correctly.
    ///
    /// # Returns
    /// * `DatalogRenderContext` - A configured render context snapshot of the current engine state.
    ///
    /// # Complexity
    /// * Time complexity: $O(1)$
    /// * Space complexity: $O(1)$
    fn render_context(&self) -> DatalogRenderContext {
        DatalogRenderContext::new(
            self.problem,
            &self.type_to_skeleton,
            self.fluence_threshold,
            self.action_base_id,
            self.action_threshold,
        )
    }
}

/// Test utilities providing mock or pre-configured engine instances for unit testing.
#[cfg(test)]
pub(crate) mod test_utils {
    use super::*;
    use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
    use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
    use crate::analysis::inertia::table::InertiaTable;
    use crate::analysis::reachability::datalog::core::Database;
    use rustc_hash::FxHashMap;

    /// Creates a pre-segmented mock `DatalogEngine` instance with predefined thresholds
    /// and leaked default references for isolated unit testing.
    ///
    /// This utility avoids calling the full `encode` pipeline when testing individual
    /// engine components or structural invariants, supplying safe default boundaries.
    ///
    /// # Returns
    /// * `DatalogEngine<'a>` - A configured engine instance ready for lightweight unit tests.
    ///
    /// # Complexity
    /// * Time complexity: $O(1)$
    /// * Space complexity: $O(1)$ (with heap-allocated leaked references designed for tests)
    pub fn create_segmented_engine<'a>() -> DatalogEngine<'a> {
        let problem_ref = Box::leak(Box::new(LiftedProblem::default()));
        let registry_ref = Box::leak(Box::new(ValueRegistry::default()));
        let table_ref = Box::leak(Box::new(InertiaTable::empty()));
        let neg_ref = Box::leak(Box::new(Vec::new()));

        DatalogEngine {
            problem: problem_ref,
            value_registry: registry_ref,
            inertia_table: table_ref,
            negated_predicates: neg_ref,
            db: Database::new(),
            rules: Vec::new(),
            current_env: [None; MAX_VARS],
            trailing_indices: Vec::new(),
            discovered_facts: Vec::new(),
            head_buffer: Vec::new(),
            fluence_threshold: 2,
            type_segment_start: 4,
            type_threshold: 7,
            action_threshold: 10,
            action_base_id: 7,
            builtin_threshold: 0,
            union_cache: FxHashMap::default(),
            base_aux_id: 0,
            next_aux_id: 0,
            aux_defs: Vec::new(),
            cache: FxHashMap::default(),
            action_effects: Vec::new(),
            action_anchor: None,
            negation_offset: 0,
            type_to_skeleton: Vec::new(),
        }
    }
}
