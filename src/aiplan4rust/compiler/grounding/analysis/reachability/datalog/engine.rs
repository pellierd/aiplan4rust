use crate::aiplan4rust::compiler::grounding::analysis::inertia::table::InertiaTable;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::atom::Atom;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::cause::Cause;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::database::Database;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::error::DatalogError;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::renderers::{
    database, rules, DatalogRenderContext,
};
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::rule::Rule;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::term::Term;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::tuple::Tuple;
use crate::aiplan4rust::compiler::grounding::binding::iter::BindingsIterator;
use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
use crate::aiplan4rust::compiler::lir::renderers::LiftedSyntaxDisplay;
use crate::aiplan4rust::support::lang::{
    ActionDefId, AtomSkeletonId, Id, ObjectId, TypeId, VariableId,
};
use crate::analysis::reachability::datalog::encoder;
use itertools::Itertools;
use std::collections::HashMap;
use toml::value::Index;

/// Maximum number of variables (parameters) allowed per action or rule.
///
/// This limit is set to 64 to allow high-performance variable tracking
/// using a single CPU register (u64 bitset).
pub const MAX_VARS: usize = 64;

// pre requis les types doivent faltten et les quantfier remove pas d'imply
pub struct DatalogEngine<'a> {
    pub(crate) problem: &'a mut LiftedProblem,
    value_registry: &'a ValueRegistry,
    inertia_table: &'a InertiaTable,
    negated_predicates: &'a Vec<AtomSkeletonId>,
    db: Database,
    pub(crate) rules: Vec<Rule>,
    current_env: [Option<ObjectId>; MAX_VARS],
    /// Pile de traçage pour le rollback des variables (Undo Stack)
    trailing_indices: Vec<usize>,
    // Buffer temporaire pour stocker les faits trouvés pour une règle
    discovered_facts: Vec<(AtomSkeletonId, Vec<ObjectId>)>,
    head_buffer: Vec<ObjectId>,
    fluence_threshold: usize,
    type_threshold: usize,
    type_segment_start: usize,
    action_base_id: usize,
    action_threshold: usize,
    builtin_threshold: usize,
    /// Cache pour ne pas dupliquer les prédicats d'union.
    /// Clé : La liste triée des TypeId. Valeur : L'ID du squelette Datalog.
    union_cache: HashMap<Vec<TypeId>, AtomSkeletonId>,

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
    pub(crate) cache: HashMap<Vec<Atom>, Atom>,

    // Ajout du champ interne
    // On utilise un champ membre pour éviter de le passer partout
    pub(crate) current_aliases: HashMap<VariableId, Term>,

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
        // 2. REPRODUCTION DE L'ORDRE DES SEUILS (STRICT SANS ENCODEUR)
        // =========================================================================
        let mut type_to_skeleton = Vec::new();
        let mut dummy_id = type_segment_start;

        // Remplit type_to_skeleton
        encoder::facts::declare_type_defs(type_defs_slice, &mut type_to_skeleton, &mut dummy_id);

        // 🔥 Alignement crucial avec ton ancien code :
        // L'encodeur définitif écrasait le compteur pour repartir de `type_segment_start`
        let type_threshold = type_segment_start;
        let action_base_id = type_threshold;

        // Les actions consomment les IDs à partir de action_base_id
        let mut current_id = action_base_id;
        let mut aux_defs = Vec::with_capacity(256);
        encoder::facts::declare_action_defs(
            action_defs_slice,
            &mut local_store,
            &mut current_id,
            &mut aux_defs,
        )?;

        let action_threshold = current_id;
        let builtin_threshold = action_threshold;

        // =========================================================================
        // 3. INGESTION DES DONNÉES ET RÈGLES
        // =========================================================================
        let mut db = Database::new();
        let mut rules = Vec::with_capacity(1024);
        let mut cache = HashMap::with_capacity(256);
        let mut current_aliases = HashMap::with_capacity(256);
        let mut action_effects = vec![Vec::new(); action_count];
        let union_cache = HashMap::new();

        // Remplissage de la DB
        encoder::facts::fill_db_from_objects(
            &mut db,
            &type_to_skeleton,
            object_defs_slice,
            type_defs_slice,
        )?;

        encoder::facts::fill_db_from_init(&mut db, init_expr_id, &mut local_store)?;

        // Compilation des règles (génère les aux_XX >= builtin_threshold)
        encoder::action::encode_action_defs(
            &mut rules,
            &mut db,
            &mut action_effects,
            &mut cache,
            &mut aux_defs,
            &mut current_id, // S'incrémente dynamiquement pour chaque aux_XX créé
            fluence_threshold,
            action_defs_slice,
            action_base_id,
            inertia_table,
            &type_to_skeleton,
            &mut local_store,
        )?;

        let final_builtin_threshold = current_id;

        // =========================================================================
        // 4. RESTAURATION ET EMPEQUETAGE
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

            base_aux_id: type_segment_start, // Reste calqué sur la structure originale
            next_aux_id: current_id,
            aux_defs,
            cache,
            current_aliases,
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

    pub fn get_reachable_fluents(&self) -> Vec<Tuple<AtomSkeletonId>> {
        let mut fluents = Vec::new();

        // On parcourt les relations de la DB (le stockage Datalog)
        for (&sk_id, rel) in self.db.stable_relations().iter() {
            // On ne garde que ce qui appartient aux Fluents (Prédicats)
            if self.is_fluent(sk_id) {
                // Le sk_id est déjà notre AtomSkeletonId interne
                let skeleton_id = AtomSkeletonId::from(sk_id);

                for tuple_data in rel.iter() {
                    // On crée un Tuple pour chaque ligne de la relation
                    fluents.push(Tuple::new(skeleton_id, tuple_data.to_vec()));
                }
            }
        }
        fluents
    }

    pub fn get_reachable_actions(&self) -> Vec<Tuple<ActionDefId>> {
        let mut actions = Vec::with_capacity(self.db.stable_relations().len());

        for (&sk_id, rel) in self.db.stable_relations().iter() {
            if self.is_action(sk_id) {
                let action_def_id = self.atom_id_to_action_def_id(sk_id);

                // --- CORRECTION ICI ---
                if rel.arity() == 0 {
                    // Pour l'arité 0, si la relation n'est pas vide,
                    // c'est que l'action est vraie (1 seule instance possible).
                    if !rel.is_empty() {
                        actions.push(Tuple::new(action_def_id, vec![]));
                    }
                } else {
                    // Pour l'arité > 0, on itère normalement sur les arguments
                    for tuple_data in rel.iter() {
                        actions.push(Tuple::new(action_def_id, tuple_data.to_vec()));
                    }
                }
            }
        }
        actions
    }

    pub fn get_reachable_auxiliaries(&self) -> Vec<Tuple<AtomSkeletonId>> {
        let mut axioms = Vec::new();

        for (&sk_id, rel) in self.db.stable_relations().iter() {
            // On cible uniquement le segment des auxiliaires (pivots When, Derived, etc.)
            if self.is_auxiliary(sk_id) {
                let skeleton_id = AtomSkeletonId::from(sk_id);

                for tuple_data in rel.iter() {
                    axioms.push(Tuple::new(skeleton_id, tuple_data.to_vec()));
                }
            }
        }
        axioms
    }

    pub fn get_type_extensions(&self) -> Vec<Tuple<TypeId>> {
        // On pré-alloue par rapport au nombre de relations, comme pour les actions
        let mut types = Vec::with_capacity(self.db.stable_relations().len());

        for (&sk_id, rel) in self.db.stable_relations().iter() {
            let id_val = sk_id;

            // 1. Utilisation de la méthode de segment pour les Types
            if self.is_type(id_val) {
                // 2. Traduction arithmétique inline (O(1))
                // On soustrait le fluence_threshold pour retrouver l'index du typing
                let type_id = self.atom_id_to_type_id(sk_id);

                for tuple_data in rel.iter() {
                    // 3. Création du Tuple (souvent unaire pour les types)
                    types.push(Tuple::new(type_id, tuple_data.to_vec()));
                }
            }
        }
        types
    }

    /// Récupère les effets (Add et Delete) et leur causalité associés à une action spécifique.
    ///
    /// L'identifiant fourni doit être celui de l'atome d'action produit
    /// par le moteur Datalog.
    ///
    /// Retourne une tranche de couples (Atome d'effet, Cause de l'effet).
    /// Retourne la tranche (slice) d'effets pour l'index d'action donné à partir de son squelette d'ID.
    pub fn get_effects_for_action(&self, action_sk_id: AtomSkeletonId) -> &[(Atom, Cause)] {
        // 1. On vérifie que c'est bien une action (via ton mécanisme de segmentation d'ID)
        debug_assert!(self.is_action(action_sk_id));

        // 2. On calcule l'index relatif pour accéder au Vec dense
        // Assure-toi que action_base_id correspond bien au premier ID alloué aux actions.
        let action_index = action_sk_id.as_usize() - self.action_base_id;

        // 3. On accède directement à notre tableau interne d'effets sans passer par l'encodeur
        self.action_effects
            .get(action_index)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn get_rule_for_action(&self, action_index: usize) -> &Rule {
        // L'ID interne est calculé directement ici
        let target_sk_id = AtomSkeletonId::from(self.type_threshold + action_index);

        self.rules
            .iter()
            .find(|r| r.head().skeleton_id() == target_sk_id)
            .expect("Aucune règle trouvée pour cet index d'action")
    }

    pub fn get_rule_for_auxiliary(&self, sk_id: AtomSkeletonId) -> &Rule {
        debug_assert!(self.is_auxiliary(sk_id));

        self.rules
            .iter()
            .find(|r| r.head().skeleton_id() == sk_id)
            .expect("Inconsistance : fait auxiliaire trouvé sans règle correspondante")
    }

    #[inline]
    pub fn is_fluent(&self, id: AtomSkeletonId) -> bool {
        // Tout ce qui est avant le début des types est un fluent
        // (cela inclut le bloc positif et le bloc négatif optionnel)
        id.as_usize() < self.type_segment_start
    }

    #[inline]
    pub fn is_negated_fluent(&self, id: AtomSkeletonId) -> bool {
        let val = id.as_usize();
        val >= self.fluence_threshold && val < self.type_segment_start
    }

    #[inline]
    pub fn is_type(&self, id: AtomSkeletonId) -> bool {
        let p = id.as_usize();
        // Le segment des types est coincé entre sa frontière propre et le début des actions
        p >= self.type_segment_start && p < self.type_threshold
    }

    #[inline]
    pub fn atom_id_to_type_id(&self, sk_id: AtomSkeletonId) -> TypeId {
        let id_val = sk_id.as_usize();
        debug_assert!(self.is_type(sk_id));
        // L'offset de soustraction est maintenant dynamique
        TypeId::from(id_val - self.type_segment_start)
    }

    #[inline]
    pub fn negate_id(&self, id: AtomSkeletonId) -> AtomSkeletonId {
        let val = id.as_usize();
        // On part du principe que id est un fluent positif < fluence_threshold
        debug_assert!(val < self.fluence_threshold);
        AtomSkeletonId::from(val + self.fluence_threshold)
    }

    #[inline]
    pub fn pos_id_from_negated(&self, id: AtomSkeletonId) -> AtomSkeletonId {
        let val = id.as_usize();
        // On part du principe que id est un fluent négatif [N..2N[
        debug_assert!(val >= self.fluence_threshold && val < self.type_segment_start);
        AtomSkeletonId::from(val - self.fluence_threshold)
    }

    /// Vérifie si un ID appartient au segment des Actions.
    #[inline]
    pub fn is_action(&self, id: AtomSkeletonId) -> bool {
        let p = id.as_usize();
        p >= self.type_threshold && p < self.action_threshold
    }

    #[inline]
    pub fn atom_id_to_action_def_id(&self, sk_id: AtomSkeletonId) -> ActionDefId {
        let id_val = sk_id.as_usize();
        // On ne touche à rien ici, le calcul est mathématiquement juste pour le vecteur
        ActionDefId::from(id_val - self.type_threshold)
    }

    /// Convertit un ActionDefId (public) en AtomSkeletonId (interne).
    pub fn action_def_id_to_atom_id(&self, def_id: ActionDefId) -> AtomSkeletonId {
        AtomSkeletonId::from(def_id.as_usize() + self.action_base_id)
    }

    /// Convertit un ActionDefId (public) en l'ID interne (AtomSkeletonId)
    /// utilisé par le moteur Datalog.
    pub fn action_id_to_skeleton(&self, action_id: ActionDefId) -> AtomSkeletonId {
        // On utilise le même calcul que ton atom_id_to_action_def_id mais à l'envers
        AtomSkeletonId::from(action_id.as_usize() + self.type_threshold)
    }

    #[inline]
    pub fn is_builtin(&self, id: AtomSkeletonId) -> bool {
        id.as_usize() >= Atom::BUILTIN_ZONE_START
    }

    #[inline]
    pub fn is_auxiliary(&self, id: AtomSkeletonId) -> bool {
        let p = id.as_usize();
        // Un auxiliaire est tout ce qui se trouve entre la fin des actions
        // et le début de la zone réservée aux built-ins (égalité).
        p >= self.action_threshold && p < Atom::BUILTIN_ZONE_START
    }

    /// La SEULE méthode autorisée pour ajouter une règle au moteur.
    pub fn push_rule(&mut self, mut rule: Rule) {
        // 1. On répare l'ordre (Priorité 4 pour l'égalité l'enverra à la fin)
        self.optimize_body(rule.body_mut());

        // 2. On l'ajoute au stockage
        self.rules.push(rule);
    }

    ///////////////////////////////////////////////////////////////////////////////////////////////
    // FONCTIONS FOR RUNNING THE ENGINE
    ///////////////////////////////////////////////////////////////////////////////////////////////

    /// Lance le calcul de l'atteignabilité (Interface publique)
    pub fn run(&mut self) {
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

    fn materialize_negations(&mut self) -> Result<(), DatalogError> {
        let offset = self.fluence_threshold;

        // 1. On itère par référence sur le vecteur pointé par la référence
        // On utilise .iter() pour obtenir chaque AtomSkeletonId
        for neg_id in self.negated_predicates.iter() {
            let pos_id = AtomSkeletonId::from(neg_id.strip_negation().as_usize());
            let pred_def = &self.problem.predicate_defs()[pos_id.as_usize()];

            let param_list_id = pred_def.parameters();
            let parameters = self.problem.store().fetch_typed_list(param_list_id)?;

            // 2. On crée l'itérateur de combinaisons
            let mut iter = BindingsIterator::new(parameters, self.value_registry).unwrap();

            while let Some(bindings) = iter.next() {
                self.head_buffer.clear();

                // Pour chaque paramètre du prédicat (ex: ?p puis ?a)
                for param in parameters {
                    // On demande au dictionnaire : "C'est quoi l'ID de l'objet pour ?p ?"
                    if let Some(obj_id) = bindings.get(param.symbol()) {
                        // On l'ajoute au buffer : [10, 50]
                        self.head_buffer.push(obj_id);
                    }
                }

                // 3. CLOSED WORLD ASSUMPTION
                // Si le fait positif n'est pas dans le STABLE, l'absence est VRAIE
                if !self.db.has_fact(pos_id, &self.head_buffer) {
                    let storage_id = AtomSkeletonId::from(pos_id.as_usize() + offset);
                    // On insère le fait négatif dans le Delta pour la Strate 1
                    self.db.insert_delta_fact(storage_id, &self.head_buffer);
                }
            }
        }
        Ok(())
    }

    pub fn debug_recursive_rule(&self, target_id: AtomSkeletonId, depth: usize) {
        let indent = "  ".repeat(depth);
        let target_raw = target_id.as_usize();

        // 1. On récupère TOUTES les règles qui ont cet ID en tête (gestion du OR)
        let matching_rules: Vec<_> = self
            .rules
            .iter()
            .filter(|r| r.head().skeleton_id() == target_id)
            .collect();

        if matching_rules.is_empty() {
            // Si aucune règle, on regarde si c'est un fait de base (Segment 0, 1 ou Types)
            if let Some(rel) = self.db.stable_relations().get(&target_id) {
                println!(
                    "{}|_ [LEAF FACT] ID {}: {} tuples présents",
                    indent,
                    target_raw,
                    rel.len()
                );
                for (i, tuple) in rel.iter().take(3).enumerate() {
                    println!("{}   f#{}: {:?}", indent, i, tuple);
                }
            } else {
                println!("{}|_ ID {} : VIDE (Ni règle, ni fait)", indent, target_raw);
            }
            return;
        }

        for (branch_idx, rule) in matching_rules.iter().enumerate() {
            println!("{}|_ Branch #{} for ID {}:", indent, branch_idx, target_raw);

            for atom in rule.body() {
                let sub_id = atom.skeleton_id();
                let sub_raw = sub_id.as_usize();

                if self.is_auxiliary(sub_id) {
                    // Descente récursive pour les pivots (Aux_34, Aux_35, etc.)
                    self.debug_recursive_rule(sub_id, depth + 1);
                } else {
                    // On interroge la Database via ton API Relation
                    if let Some(rel) = self.db.stable_relations().get(&sub_id) {
                        println!(
                            "{}  [CHECK] Predicate {} ({:?}) -> PRESENT ({} faits)",
                            indent,
                            sub_raw,
                            atom.terms(),
                            rel.len()
                        );

                        // On affiche les 5 premiers tuples pour vérifier les ObjectId
                        for (i, tuple) in rel.iter().take(5).enumerate() {
                            println!("{}     tuple#{}: {:?}", indent, i, tuple);
                        }
                    } else {
                        println!(
                            "{}  [CHECK] Predicate {} ({:?}) -> ABSENT",
                            indent,
                            sub_raw,
                            atom.terms()
                        );
                    }
                }
            }
        }
    }

    fn saturate_semi_naive(&mut self) {
        // 1. BOOTSTRAP : On ne déplace vers delta que si le delta est vide
        // et qu'on a des choses en stable (cas d'un moteur qu'on relancerait).
        if self.db.is_delta_empty() && !self.db.stable_relations().is_empty() {
            self.db.move_all_to_delta();
        }

        // 2. BOUCLE PRINCIPALE
        while !self.db.is_delta_empty() {
            // On récupère les règles pour éviter les problèmes de borrow checker
            let rules = std::mem::take(&mut self.rules);

            for rule in &rules {
                let body_len = rule.body().len();

                // On traite chaque atome de la règle comme un pivot
                for pivot_idx in 0..body_len {
                    self.current_env = [None; MAX_VARS];
                    self.trailing_indices.clear();

                    // On explore les combinaisons
                    self.process_incremental(rule, 0, pivot_idx);
                }
            }

            // On remet les règles en place
            self.rules = rules;

            // 1. On stabilise ce qui a servi de PIVOT durant ce tour
            // (Le delta du tour N devient le stable du tour N+1)
            self.db.commit_delta();

            // 2. SEULEMENT MAINTENANT, on injecte les découvertes
            // Elles vont remplir le Delta TOUT NEUF pour le tour suivant.
            for (sk_id, args) in self.discovered_facts.drain(..) {
                // Cette fonction doit vérifier l'unicité contre le STABLE (le nouveau)
                self.db.insert_delta_fact(sk_id, &args);
            }

            // La boucle continue si insert_delta_fact a ajouté de nouveaux éléments au Delta
        }
    }

    // Note : db est maintenant &Database (immutable)
    fn process_incremental(&mut self, rule: &Rule, body_idx: usize, pivot_idx: usize) {
        // 1. Condition d'arrêt : succès de la règle (tous les atomes validés)
        if body_idx == rule.body().len() {
            self.evaluate_head(rule);
            return;
        }

        let atom = &rule.body()[body_idx];
        let sk_id = atom.skeleton_id();

        // --- NOUVEAU : Branchement vers le filtrage Lazy ---
        // Si l'atome est un built-in OU s'il est négatif (bit MSB à 1)
        // On ne cherche pas dans la DB, on teste l'absence (pour not) ou la logique (pour ==)
        if self.is_builtin(sk_id) || atom.is_negated() {
            // On appelle execute_filter (qui remplace execute_builtin)
            if self.execute_filter(atom) {
                self.process_incremental(rule, body_idx + 1, pivot_idx);
            }
            return;
        }

        // --- Logique existante pour les relations positives (Scan/Index) ---
        // Note : On ne rentre ici que pour des atomes POSITIFS et NON-BUILTIN
        if body_idx < pivot_idx {
            // Avant le pivot : uniquement dans le Stable
            self.match_relation(rule, body_idx, pivot_idx, sk_id, false);
        } else if body_idx == pivot_idx {
            // Au pivot : uniquement dans le Delta (Semi-Naïf)
            self.match_relation(rule, body_idx, pivot_idx, sk_id, true);
        } else {
            // Après le pivot : on teste les deux (Stable et Delta)
            self.match_relation(rule, body_idx, pivot_idx, sk_id, false);
            self.match_relation(rule, body_idx, pivot_idx, sk_id, true);
        }
    }

    fn execute_filter(&mut self, atom: &Atom) -> bool {
        let sk_id = atom.skeleton_id(); // C'est un AtomSkeletonId

        // 1. GESTION DES BUILT-INS
        if self.is_builtin(sk_id) {
            return self.eval_equality(atom);
        }

        // 2. GESTION DE LA NÉGATION LAZY
        // On vérifie le bit MSB via ton interface et ton segment miroir
        let is_neg_id = sk_id.as_usize() >= self.fluence_threshold
            && sk_id.as_usize() < self.type_segment_start;

        if sk_id.is_negated() || is_neg_id {
            // --- Utilisation du buffer interne du moteur ---
            self.head_buffer.clear();

            for term in atom.terms() {
                if let Some(val) = self.get_term_value(term) {
                    self.head_buffer.push(val);
                } else {
                    return false; // Variable non liée (Safety Datalog)
                }
            }

            // --- DÉTERMINATION DE L'ID POSITIF ---
            let pos_sk_id = if sk_id.is_negated() {
                // Utilisation de ta méthode d'interface pour "nettoyer" l'ID
                sk_id.strip_negation()
            } else {
                // Pour le segment miroir [N..2N[, on utilise ton calcul de décalage
                self.pos_id_from_negated(sk_id)
            };

            // --- LOGIQUE NAF (Negation as Failure) ---
            // On renvoie VRAI si le fait POSITIF est ABSENT de la base
            // C'est ici que l'action 'assemble' se débloque !
            return !self.db.has_fact(pos_sk_id, &self.head_buffer);
        }

        true
    }

    /// Logique spécifique pour l'égalité
    fn eval_equality(&self, atom: &Atom) -> bool {
        let terms = atom.terms();
        // On récupère les IDs concrets des objets via l'environnement actuel
        let val_a = self.get_term_value(&terms[0]);
        let val_b = self.get_term_value(&terms[1]);

        match (val_a, val_b) {
            (Some(a), Some(b)) => {
                let are_equal = a == b;

                // Si l'atome est inversé (NOT =), on renvoie Vrai si les objets sont différents
                if atom.is_negated() {
                    !are_equal
                } else {
                    are_equal
                }
            }
            _ => {
                // En Datalog strict, si une variable n'est pas encore liée,
                // le filtre ne peut pas être validé.
                false
            }
        }
    }
    fn get_term_value(&self, term: &Term) -> Option<ObjectId> {
        match term {
            Term::Constant(c) => Some(*c),
            Term::Variable(v) => self.current_env[v.as_usize()],
        }
    }

    fn match_relation(
        &mut self,
        rule: &Rule,
        body_idx: usize,
        pivot_idx: usize,
        sk_id: AtomSkeletonId,
        use_delta: bool,
    ) {
        let atom = &rule.body()[body_idx];
        let terms = atom.terms();

        let Some((total_len, arity)) = self.db.get_layout(sk_id, use_delta) else {
            return;
        };
        let mut tuple_buffer = [ObjectId::from(0); MAX_VARS];

        // 1. CAS ARITÉ 0 : On traite la proposition si elle est présente dans la table demandée
        if arity == 0 {
            // Si total_len est 0 mais que la table (Delta ou Stable selon use_delta)
            // contient la proposition, on déclenche l'unification une fois.
            let is_present = if use_delta {
                self.db.contains_delta(sk_id, &[])
            } else {
                self.db.contains_stable(sk_id, &[])
            };

            if is_present {
                self.process_tuple(
                    rule,
                    body_idx,
                    pivot_idx,
                    sk_id,
                    use_delta,
                    0,
                    0,
                    &mut tuple_buffer,
                );
            }
            return;
        }

        // 2. CAS ARITÉ > 0 : Stratégie d'indexation classique
        let first_arg_binding = match &terms[0] {
            Term::Constant(c) => Some(*c),
            Term::Variable(v) => self.current_env[v.as_usize()],
        };

        if let Some(obj_id) = first_arg_binding {
            // MODE INDEXÉ
            if let Some(offsets) = self.db.lookup_index(sk_id, use_delta, obj_id) {
                for start in offsets {
                    self.process_tuple(
                        rule,
                        body_idx,
                        pivot_idx,
                        sk_id,
                        use_delta,
                        start,
                        arity,
                        &mut tuple_buffer,
                    );
                }
            }
        } else {
            // MODE SCAN COMPLET
            // step_by(arity) avec arity > 0 est sûr ici.
            for start in (0..total_len).step_by(arity) {
                self.process_tuple(
                    rule,
                    body_idx,
                    pivot_idx,
                    sk_id,
                    use_delta,
                    start,
                    arity,
                    &mut tuple_buffer,
                );
            }
        }
    }

    // Petite fonction utilitaire pour éviter la duplication de code
    fn process_tuple(
        &mut self,
        rule: &Rule,
        body_idx: usize,
        pivot_idx: usize,
        sk_id: AtomSkeletonId,
        use_delta: bool,
        start: usize,
        arity: usize,
        buffer: &mut [ObjectId; MAX_VARS],
    ) {
        self.db.read_tuple(sk_id, use_delta, start, arity, buffer);

        // On n'utilise PLUS prev_env (trop lent). On utilise le rollback sélectif.
        // On note combien de variables étaient liées AVANT cet atome
        let trail_split = self.trailing_indices.len();

        if self.unify_and_bind(rule.body()[body_idx].terms(), &buffer[..arity]) {
            self.process_incremental(rule, body_idx + 1, pivot_idx);
        }

        // ROLLBACK CHIRURGICAL :
        // On n'annule que ce que unify_and_bind a ajouté,
        // sans toucher aux variables liées par les atomes précédents de la règle.
        self.undo_to_savepoint(trail_split);
    }

    fn evaluate_head(&mut self, rule: &Rule) -> Result<(), DatalogError> {
        let head = rule.head();
        let head_sk = head.skeleton_id();

        // 1. On prépare le tuple de la tête dans le buffer réutilisable
        self.head_buffer.clear();

        for term in head.terms() {
            match term {
                Term::Constant(c) => self.head_buffer.push(*c),
                Term::Variable(v) => {
                    let val = self.current_env[v.as_usize()].ok_or_else(|| {
                        println!("CRASH: Variable {} is unbound!", v.as_usize());
                        println!("Current Env: {:?}", self.current_env);
                        println!("Rule Head: {:?}", head);
                        DatalogError::unbound_variable(*v)
                    })?;
                    self.head_buffer.push(val);
                }
            }
        }

        // 2. FILTRAGE : On ne veut pas stocker de doublons.
        // On vérifie dans la DB (Stable + Delta)
        if self.db.contains_stable(head_sk, &self.head_buffer)
            || self.db.contains_delta(head_sk, &self.head_buffer)
        {
            return Ok(());
        }

        // 3. On vérifie aussi dans les découvertes du pivot en cours
        // pour éviter de cloner inutilement si le même fait est trouvé 100 fois de suite.
        let is_already_in_buffer = self
            .discovered_facts
            .iter()
            .any(|(sk, args)| *sk == head_sk && args == &self.head_buffer);

        if !is_already_in_buffer {
            // Le clone n'arrive qu'ici, au dernier moment possible.
            self.discovered_facts
                .push((head_sk, self.head_buffer.clone()));
        }
        Ok(())
    }

    /// Tente de faire correspondre les termes d'un atome (venant d'une règle)
    /// avec un tuple réel (venant de la Database).
    ///
    /// Retourne `true` si l'unification réussit, `false` sinon.
    /// Met à jour `self.current_env` avec les nouvelles liaisons de variables.
    fn unify_and_bind(&mut self, atom_terms: &[Term], tuple: &[ObjectId]) -> bool {
        if atom_terms.len() != tuple.len() {
            return false;
        }

        // On ne vide plus la pile, on note le point de départ (le "savepoint")
        let savepoint = self.trailing_indices.len();

        for (i, term) in atom_terms.iter().enumerate() {
            let val_in_db = tuple[i];

            match term {
                Term::Constant(c) => {
                    if *c != val_in_db {
                        // Échec : on annule uniquement ce que CET atome a lié
                        self.undo_to_savepoint(savepoint);
                        return false;
                    }
                }
                Term::Variable(v) => {
                    let idx = v.as_usize();
                    if let Some(existing_val) = self.current_env[idx] {
                        if existing_val != val_in_db {
                            self.undo_to_savepoint(savepoint);
                            return false;
                        }
                    } else {
                        self.current_env[idx] = Some(val_in_db);
                        self.trailing_indices.push(idx);
                    }
                }
            }
        }
        true
    }

    /// Nettoie les variables liées depuis le savepoint donné
    fn undo_to_savepoint(&mut self, savepoint: usize) {
        while self.trailing_indices.len() > savepoint {
            if let Some(idx) = self.trailing_indices.pop() {
                self.current_env[idx] = None;
            }
        }
    }

    fn optimize_body(&self, body: &mut Vec<Atom>) {
        if body.len() <= 1 {
            return;
        }

        let mut optimized = Vec::with_capacity(body.len());
        let mut bound_vars_mask: u64 = 0;
        let mut remaining = std::mem::take(body);

        while !remaining.is_empty() {
            let best_idx = remaining
                .iter()
                .enumerate()
                .min_by_key(|(_, atom)| {
                    let sk_id = atom.skeleton_id();

                    // 1. Calcul des variables déjà liées (Indispensable pour éviter les produits cartésiens)
                    let mut bound_count = 0;
                    for term in atom.terms() {
                        match term {
                            Term::Constant(_) => bound_count += 1,
                            Term::Variable(v) => {
                                let v_idx = v.as_usize();
                                if v_idx < 64 && (bound_vars_mask & (1 << v_idx)) != 0 {
                                    bound_count += 1;
                                }
                            }
                        }
                    }

                    // 2. Taille réelle des données dans la DB
                    let rel_size = self.db.get_relation_size(sk_id);

                    // 3. Catégorie sémantique (Type, Fluent, etc.)
                    let priority = self.get_predicate_priority(atom);

                    // L'ORDRE DU TUPLE EST CRUCIAL :
                    // a) On maximise bound_count (d'où le signe moins)
                    // b) On minimise rel_size (pour traiter le moins de faits possible)
                    // c) On minimise priority (Types < Fluents < Actions)
                    (-(bound_count as i32), rel_size, priority)
                })
                .map(|(idx, _)| idx)
                .unwrap();

            let best_atom = remaining.remove(best_idx);

            // Mise à jour du masque des variables liées par l'atome choisi
            for term in best_atom.terms() {
                if let Term::Variable(v) = term {
                    let v_idx = v.as_usize();
                    if v_idx < 64 {
                        bound_vars_mask |= 1 << v_idx;
                    }
                }
            }
            optimized.push(best_atom);
        }
        *body = optimized;
    }

    #[inline(always)]
    pub fn get_predicate_priority(&self, atom: &Atom) -> u8 {
        let id = atom.skeleton_id();

        // 1. Égalité positive (Affectation) : priorité absolue
        if atom.is_equality() && !atom.is_negated() {
            return 0;
        }

        // 2. Types : très restrictifs, servent de base au filtrage
        if self.is_type(id) {
            return 0;
        }

        // 3. Fluents positifs : recherche dans l'état actuel
        if self.is_fluent(id) {
            return 1;
        }

        // 4. Anchor : l'action elle-même (lie les paramètres restants)
        if self.is_action(id) {
            return 2;
        }

        // 5. Auxiliaires : prédicats générés pour la logique AND/OR
        if self.is_auxiliary(id) {
            return 3;
        }

        // 6. Négations et Inégalités : ne lient rien, donc on attend la fin
        if atom.is_negated() {
            return 250;
        }

        // 7. Cas par défaut (Built-ins, etc.)
        255
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
