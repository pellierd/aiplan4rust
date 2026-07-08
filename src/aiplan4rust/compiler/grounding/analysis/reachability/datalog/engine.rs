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
use itertools::Itertools;
use rustc_hash::FxHashMap;
use toml::value::Index;

/// Maximum number of variables (parameters) allowed per action or rule.
///
/// This limit is set to 64 to allow high-performance variable tracking
/// using a single CPU register (u64 bitset).
pub const MAX_VARS: usize = 64;

// pre requis les types doivent faltten et les quantfier remove pas d'imply
pub struct DatalogEngine<'a> {
    pub(crate) problem: &'a mut LiftedProblem,
    pub(crate) value_registry: &'a ValueRegistry,
    pub(crate) inertia_table: &'a InertiaTable,
    pub(crate) negated_predicates: &'a Vec<AtomSkeletonId>,
    pub(crate) db: Database,
    pub(crate) rules: Vec<Rule>,
    pub(crate) current_env: [Option<ObjectId>; MAX_VARS],
    /// Pile de traçage pour le rollback des variables (Undo Stack)
    pub(crate) trailing_indices: Vec<usize>,
    // Buffer temporaire pour stocker les faits trouvés pour une règle
    pub(crate) discovered_facts: Vec<(AtomSkeletonId, Vec<ObjectId>)>,
    pub(crate) head_buffer: Vec<ObjectId>,
    pub(crate) fluence_threshold: usize,
    pub(crate) type_threshold: usize,
    pub(crate) type_segment_start: usize,
    pub(crate) action_base_id: usize,
    pub(crate) action_threshold: usize,
    pub(crate) builtin_threshold: usize,
    /// Cache pour ne pas dupliquer les prédicats d'union.
    /// Clé : La liste triée des TypeId. Valeur : L'ID du squelette Datalog.
    pub(crate) union_cache: FxHashMap<Vec<TypeId>, AtomSkeletonId>,

    //////////////////////////////////////////////////
    //// ENCODER
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

    /// Table de causalité : associe chaque effet à son origine (Action ou Pivot).
    pub(crate) action_effects: Vec<Vec<(Atom, Cause)>>,

    pub(crate) action_anchor: Option<Atom>,

    pub(crate) negation_offset: usize,
    pub(crate) type_to_skeleton: Vec<AtomSkeletonId>,
}

impl<'a> DatalogEngine<'a> {
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

        // On crée le State pour orchestrer les enregistrements mutables
        let mut state = DatalogState::new(
            &mut rules,
            &mut cache,
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
        // 🌟 Initialisation du contexte (maintenant que type_to_skeleton est prêt et figé)
        let ctx = DatalogContext::new(
            TypedListId::default(),
            &type_to_skeleton,
            fluence_threshold,
            inertia_table,
        );

        // 🌟 3. Nouvelle signature propre pour fill_db_from_objects (ctx + state)
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

    pub fn dump_database(&self) {
        // 1. On crée le Snapshot de données (le contexte)
        let ctx = self.render_context();

        // 2. On appelle le renderer spécialisé
        database::render(&ctx, &self.db);
    }

    // Affiche la logique compilée (Rules)
    // On ne prend plus de paramètres, on utilise self.rules
    pub fn dump_rules(&self) {
        let ctx = self.render_context();
        rules::render(&ctx, &self.rules);
    }

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

#[cfg(test)]
#[path = "tests/engine_tests.rs"]
mod engine_tests;
