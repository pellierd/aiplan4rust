use crate::aiplan4rust::compiler::grounding::analysis::inertia::table::InertiaTable;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::atom::Atom;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::cause::Cause;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::database::Database;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::encoder::DatalogEncoder;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::error::DatalogError;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::renderers::{
    database, rules, RenderContext,
};
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::rule::Rule;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::term::Term;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::tuple::Tuple;
use crate::aiplan4rust::compiler::grounding::binding::iter::BindingsIterator;
use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::compiler::lir::expr::{Expr, ExprKind, ExprNode};
use crate::aiplan4rust::compiler::lir::problem::ActionDef;
use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
use crate::aiplan4rust::support::lang::{
    ActionDefId, AtomSkeletonId, Id, ObjectId, TypeId, TypedSymbol, VariableId,
};
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
    problem: &'a LiftedProblem,
    value_registry: &'a ValueRegistry,
    inertia_table: &'a InertiaTable,
    negated_predicates: &'a Vec<AtomSkeletonId>,
    db: Database,
    rules: Vec<Rule>,
    current_env: [Option<ObjectId>; MAX_VARS],
    /// Pile de traçage pour le rollback des variables (Undo Stack)
    trailing_indices: Vec<usize>,
    // Buffer temporaire pour stocker les faits trouvés pour une règle
    discovered_facts: Vec<(AtomSkeletonId, Vec<ObjectId>)>,
    type_to_skeleton: Vec<AtomSkeletonId>,
    head_buffer: Vec<ObjectId>,
    encoder: DatalogEncoder,
    fluence_threshold: usize,
    type_threshold: usize,
    type_segment_start: usize,
    action_base_id: usize,
    action_threshold: usize,
    builtin_threshold: usize,
    /// Cache pour ne pas dupliquer les prédicats d'union.
    /// Clé : La liste triée des TypeId. Valeur : L'ID du squelette Datalog.
    union_cache: HashMap<Vec<TypeId>, AtomSkeletonId>,
}

impl<'a> DatalogEngine<'a> {
    pub fn new(
        problem: &'a LiftedProblem,
        value_registry: &'a ValueRegistry,
        inertia_table: &'a InertiaTable,
        negated_predicates: &'a Vec<AtomSkeletonId>,
    ) -> Self {
        Self {
            problem,
            value_registry,
            inertia_table,
            negated_predicates,
            db: Database::new(),
            rules: Vec::new(),
            current_env: [None; MAX_VARS],
            trailing_indices: Vec::with_capacity(MAX_VARS),
            discovered_facts: Vec::with_capacity(1024),
            type_to_skeleton: Vec::new(),
            head_buffer: Vec::with_capacity(16),
            encoder: DatalogEncoder::new(0, 0, 0, Vec::new()),
            fluence_threshold: 0,
            type_threshold: 0,
            action_base_id: 0,
            action_threshold: 0,
            builtin_threshold: 0,
            type_segment_start: 0,
            union_cache: HashMap::new(),
        }
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
    pub fn get_effects_for_action(&self, action_sk_id: AtomSkeletonId) -> &[(Atom, Cause)] {
        // 1. On vérifie que c'est bien une action (via ton mécanisme de segmentation d'ID)
        debug_assert!(self.is_action(action_sk_id));

        // 2. On calcule l'index relatif pour accéder au Vec dense de l'encodeur
        // Assure-toi que action_base_id correspond bien au premier ID alloué aux actions.
        let action_index = action_sk_id.as_usize() - self.action_base_id;

        // 3. On demande à l'encodeur de nous donner le segment correspondant (Vec<(Atom, Cause)>)
        self.encoder.get_action_effects(action_index)
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

    pub fn load_problem(&mut self) -> Result<(), DatalogError> {
        // 1. Internal State Reset
        // Reset the fact database and clear existing inference rules.
        self.db = Database::new();
        self.rules.clear();
        self.union_cache.clear();
        self.type_to_skeleton.clear();

        // Segment 1: PDDL Fluents
        // Define the first ID segment based on domain predicates.
        self.fluence_threshold = self.problem.predicate_defs().len();

        // 3. Calcul de la frontière (Dynamique)
        // Si la liste des négations est vide, les types commencent à N.
        // Sinon, on réserve le miroir et les types commencent à 2N.
        self.type_segment_start = if self.negated_predicates.is_empty() {
            self.fluence_threshold
        } else {
            self.fluence_threshold * 2
        };
        // 2. Initialisation d'un encodeur temporaire ou reset de l'existant
        // pour qu'il commence au bon ID.
        self.encoder.reset_with_start_id(self.type_segment_start);

        // 3. Déclaration des types (remplit type_to_skeleton avec les bons IDs)
        self.declare_types_as_unary_predicates(self.problem.type_defs().as_slice());

        // 4. MAINTENANT, on crée l'encodeur définitif avec le vecteur rempli
        self.encoder = DatalogEncoder::new(
            self.type_segment_start,
            self.problem.action_defs().len(),
            self.fluence_threshold,
            self.type_to_skeleton.clone(), // Le vecteur est maintenant peuplé
        );

        // We now use self.encoder.current_id() instead of manual length calculation.
        //
        // WHY: Our internal vector includes BOTH the PDDL domain types AND
        // the sentinel ROOT typing, but more importantly, the encoder might have
        // generated hidden auxiliary predicates for typing hierarchies or unions.
        // current_id() captures the REAL end of this segment in the encoder.
        self.type_threshold = self.encoder.current_id();

        // On fixe la base AVANT de déclarer les actions pour que l'ID de la
        // première action (index 0) corresponde exactement à cette base.
        self.action_base_id = self.type_threshold;

        // 3. Action Signature Declaration
        // Segment 3: Reserve IDs for action atoms.
        // This freezes the boundary for any future auxiliary predicates.
        // We declare actions first, then capture the new ID state.
        self.declare_action_as_predicates(self.problem.action_defs());
        self.action_threshold = self.encoder.current_id();

        // 4. Seuil des Auxiliaires (Nouveau & Simplifié)
        // Puisque l'égalité est une constante (0xFFFF_FC00),
        // les auxiliaires peuvent commencer immédiatement après les actions.
        self.builtin_threshold = self.action_threshold;

        // On informe l'encodeur de ce point de départ pour ses IDs générés.
        // L'encodeur utilisera Atom::EQUALITY_ID pour les comparaisons de manière autonome.
        self.encoder.reset_with_start_id(self.builtin_threshold);

        // 4. Problem Data Ingestion
        // Populate the Database with concrete facts.

        // 4.1. Type Instantiation (Facts: Type(Object))
        self.fill_db_from_objects(
            self.problem.object_defs().as_slice(),
            self.problem.type_defs().as_slice(),
        )?;

        // 4.2. Initial State Instantiation (Facts: Predicate(Objects))
        let init = Expr::new(self.problem.init(), self.problem.store());
        self.fill_db_from_init(init)?;

        self.dump_database();
        // 5. Domain Logic Compilation
        // Generate Datalog rules (Preconditions -> Action -> Effects).
        // Any dynamically created predicates (auxiliaries) will have IDs >= action_threshold.
        self.compile_domain_actions_as_rules(self.problem.action_defs())?;

        self.dump_rules();

        // On synchronise le seuil sur la réalité de ce que l'encodeur a produit
        // après la génération des règles (qui peut avoir créé de nouveaux auxiliaires).
        self.builtin_threshold = self.encoder.current_id();

        Ok(())
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

    fn declare_types_as_unary_predicates(&mut self, type_defs: &[TypedSymbol<TypeId, TypeId>]) {
        let num_types = type_defs.len();

        // Capacity for N domain types + 1 sentinel Root typing
        self.type_to_skeleton = Vec::with_capacity(num_types + 1);

        // 1. Encode domain types first to ensure a 1:1 mapping with PDDL indices.
        // By keeping domain types at the start of the segment [0..num_types[,
        // we avoid a +1 offset in translation functions like `atom_id_to_type_id`.
        // Example: PDDL Type index 0 maps directly to Datalog ID (fluence_threshold + 0).
        for _ in 0..num_types {
            let sk_id = self.encoder.encode_type_as_unary_predicate();
            self.type_to_skeleton.push(sk_id);
        }

        // 2. Encode the ROOT typing as a sentinel in the LAST slot.
        // This places the Root ID at the very end of the typing segment (or start of auxiliary).
        // It remains accessible for internal rules but does not shift the domain indices.
        let root_type_sk = self.encoder.encode_type_as_unary_predicate();
        self.type_to_skeleton.push(root_type_sk);
    }

    fn fill_db_from_objects(
        &mut self,
        object_defs: &[TypedSymbol<ObjectId, TypeId>],
        type_defs: &[TypedSymbol<TypeId, TypeId>], // Ajouté pour voir la hiérarchie
    ) -> Result<(), DatalogError> {
        let root_sk_id = *self.type_to_skeleton.last().ok_or_else(|| {
            DatalogError::internal_state("Root typing skeleton missing".to_string())
        })?;

        for object in object_defs {
            let obj_id = object.symbol();
            //println!("DEBUG: Processing object ID={:?}", obj_id);
            // 1. On l'insère dans la sentinelle ROOT (ton garde-fou universel)
            self.db.insert_stable_fact(root_sk_id, &[obj_id]);

            let types = object.ty();
            //println!("  |_ Types found: {:?}", types); // <--- EST-CE QUE C'EST VIDE ?

            // 2. Pour chaque typing déclaré de l'objet (ex: [ball])
            for &type_id in object.ty() {
                // On l'insère dans le typing lui-même
                let sk_id = self.type_to_skeleton[type_id.as_usize()];
                /*println!(

                    "  |_ Inserting into type_id={}, sk_id={}",
                    type_id.as_usize(),
                    sk_id.as_usize()
                );*/
                self.db.insert_stable_fact(sk_id, &[obj_id]);

                // 3. On l'insère dans TOUS les parents/membres identifiés par le flattener
                // Si members() est vide, cette boucle ne fait rien (c'est correct, ROOT suffit)
                if let Some(ty_def) = type_defs.get(type_id.as_usize()) {
                    for &parent_id in ty_def.ty().members() {
                        let parent_sk_id = self.type_to_skeleton[parent_id.as_usize()];
                        self.db.insert_stable_fact(parent_sk_id, &[obj_id]);
                    }
                }
            }
        }
        Ok(())
    }

    // --- ÉTAPE 1 : INITIALISATION (L'Ingestion) ---
    /// Centralise ici la conversion du PDDL vers la Database interne.
    /// On passe un Registry ou le Problem pour mapper les IDs vers les SkeletonIds.
    /// Parcourt l'état initial du problème pour remplir la Database.
    fn fill_db_from_init(&mut self, init: Expr) -> Result<(), DatalogError> {
        let mut iter = init.preorder();

        while let Some((id, _depth, entry)) = iter.next() {
            // On construit le ExprNodeRef à la volée
            let node = ExprNode::new(id, entry);

            match node.kind() {
                ExprKind::AtomicFormula(sk_id) => {
                    // 1. L'ID du Skeleton est directement extrait du variant de l'enum
                    let sk_id = *sk_id;

                    // 2. Extraction des ObjectIds avec une boucle explicite
                    let children = node.children();
                    let mut args = Vec::with_capacity(children.len());

                    for &arg_id in children.iter() {
                        // On récupère le nœud enfant
                        let child_node = init.fetch_node(arg_id)?;

                        // On extrait la constante (l'ObjectId) par pattern matching direct
                        if let ExprKind::Object(object_id) = child_node.kind() {
                            args.push(*object_id);
                        } else {
                            return Err(DatalogError::invalid_atom_argument_(arg_id));
                        }
                    }

                    // 3. Ajouter le fait à la Database interne
                    self.db.insert_delta_fact(sk_id, &args);
                }
                // Tout le reste (Comparison, Not, etc.) n'ayant pas de descendance logique
                // pertinente pour l'état initial, l'itérateur avance naturellement au nœud suivant.
                _ => {}
            }
        }
        Ok(())
    }

    /// Crée les squelettes de prédicats pour chaque action du problème.
    /// Cela permet de fixer les IDs des actions avant de générer les auxiliaires.
    fn declare_action_as_predicates(&mut self, action_defs: &[ActionDef]) {
        for (id, action) in action_defs.iter().enumerate() {
            // Cette méthode dans ton encoder doit simplement créer l'ID
            // et l'ajouter à son mapping interne (ex: action_to_skeleton).
            self.encoder.encode_action_as_predicate(action);
        }
    }

    fn compile_domain_actions_as_rules(
        &mut self,
        action_defs: &[ActionDef], // On ne passe que les définitions d'actions
    ) -> Result<(), DatalogError> {
        for (id, action) in action_defs.iter().enumerate() {
            // L'ID est toujours basé sur le threshold + l'index dans la liste
            let action_sk_id = AtomSkeletonId::from(self.type_threshold + id);

            // Compilation de l'unité (on garde action_as_rule au singulier ici)
            self.compile_action_as_rules(action, action_sk_id)?;
        }

        Ok(())
    }

    fn compile_action_as_rules(
        &mut self,
        action: &ActionDef,
        action_sk_id: AtomSkeletonId,
    ) -> Result<(), DatalogError> {
        let action_index = action_sk_id.as_usize() - self.action_base_id;

        // --- LOG DE DEBUG ---
        /*println!(
            "CHECK COMPILATION: Action '{}' (ID: {})",
            action.name(),
            action_sk_id.as_usize()
        );*/

        // A. Générer l'atome de nom (Pivot : action(?p1, ?p2...))
        let action_atom = self.compile_action_name_as_rules(action, action_sk_id)?;

        // B. Générer la règle de déclenchement (Preconditions -> Action)
        self.compile_action_body_as_rules(action, action_atom.clone())?;

        // --- LOG DE DEBUG FINALISATION ---
        if let Some(rule) = self.rules.last() {
            /*print!(
                "  |_ Règle générée pour {}: Aux_SK_{:?} :- ",
                action.name(),
                rule.head().skeleton_id().as_usize()
            );*/
            for atom in rule.body() {
                /*print!(
                    "Aux_SK_{:?}(arity:{}) ",
                    atom.skeleton_id().as_usize(),
                    atom.terms().len()
                );*/
            }
            println!();
        }

        // --- LE BOOTSTRAP EST ICI ---
        // On vérifie la règle de déclenchement qu'on vient de pousser
        // 1. On extrait le TypedListId de l'action
        let param_list_id = action.parameters();
        // 2. On interroge le store du problème pour récupérer la liste typée concrète
        let parameters = self.problem.store().fetch_typed_list(param_list_id)?;

        if let Some(trigger_rule) = self.rules.last() {
            // Si le corps est vide et l'action n'a pas de paramètres (?x)
            if trigger_rule.body().is_empty() && parameters.is_empty() {
                // On l'injecte comme un fait car elle est "toujours vraie"
                self.db.insert_delta_fact(action_sk_id, &[]);
            }
        }

        // C. Générer les règles de causalité (Action -> Effets)
        let effect = Expr::new(action.effect(), self.problem.store());
        self.encoder.encode_effects(
            effect,
            &action_atom,
            &mut self.rules,
            parameters,
            action_index,
        )?;

        Ok(())
    }

    /// Génère l'atome de tête représentant l'action avec ses paramètres.
    fn compile_action_name_as_rules(
        &self,
        action: &ActionDef,
        action_sk_id: AtomSkeletonId,
    ) -> Result<Atom, DatalogError> {
        // 1. On récupère l'ID de la liste de paramètres
        let param_list_id = action.parameters();

        // 2. On extrait la liste concrète depuis le store du problème
        let parameters = self.problem.store().fetch_typed_list(param_list_id)?;

        let head_terms: Vec<Term> = parameters
            .iter()
            .map(|param| Term::Variable(param.symbol()))
            .collect();

        Ok(Atom::new(action_sk_id, head_terms))
    }

    pub fn compile_action_body_as_rules(
        &mut self,
        action: &ActionDef,
        head: Atom,
    ) -> Result<(), DatalogError> {
        // 1. On récupère l'ID de la liste de paramètres
        let param_list_id = action.parameters();
        // 2. On extrait la liste concrète depuis le store du problème
        let parameters = self.problem.store().fetch_typed_list(param_list_id)?;

        let precondition = self.problem.store().fetch_expr(action.precondition())?;

        let precond_opt =
            self.encoder
                .encode_preconditions(precondition, &mut self.rules, parameters)?;

        // 2. On récupère les paramètres et on prépare l'ancre "intelligente"
        let mut final_action_body = Vec::new();
        let mut anchor_elements = Vec::new();
        let mut covered_vars = std::collections::HashSet::new();

        // --- LOGIQUE FD : Utiliser l'inertie pour lier les variables ---

        // Récupération des atomes en postorder
        let atoms = precondition
            .postorder()
            .references() // Utilise .references() ou .values() qui renvoie les ExprNodeRef
            .filter(|node| matches!(node.kind(), ExprKind::AtomicFormula(_)));

        for atom_node in atoms {
            // Extraction directe du SkeletonId depuis le variant de l'enum
            let skel_id = match atom_node.kind() {
                ExprKind::AtomicFormula(sk) => *sk,
                _ => unreachable!(),
            };

            // On récupère l'ID "propre" (sans bit de négation)
            let positive_id = skel_id.strip_negation();

            if self
                .inertia_table
                .is_predicate_positive_negative_inertia(positive_id)?
            {
                let children = atom_node.children();
                let mut terms = Vec::with_capacity(children.len());

                // On itère sur tous les enfants (arguments de l'atome)
                for &term_id in children.iter() {
                    let term_node = precondition.fetch_node(term_id)?;

                    let term = match term_node.kind() {
                        ExprKind::Variable(var_id) => {
                            let var_id = *var_id;
                            // IMPORTANT : On note que cette variable est couverte par un fait statique
                            covered_vars.insert(var_id);
                            Term::Variable(var_id)
                        }
                        ExprKind::Object(obj_id) => Term::Constant(*obj_id),
                        _ => {
                            return Err(DatalogError::invalid_atom_argument_(term_id));
                        }
                    };
                    terms.push(term);
                }

                // 3. ON STOCK l'atome dans les éléments de l'ancre
                let static_atom = Atom::new(skel_id, terms);
                anchor_elements.push(static_atom);
            }
        }

        // ==========================================================
        // ICI : TON BLOC DE SÉCURITÉ (TYPE GUARD)
        // ==========================================================
        for (i, param) in parameters.iter().enumerate() {
            let var_id = VariableId::from(i);
            if !covered_vars.contains(&var_id) {
                let var_term = Term::Variable(var_id);
                let type_id = param.ty().members()[0].as_usize();
                let type_sk = self.type_to_skeleton[type_id];

                // 1. AJOUT PHYSIQUE À L'ANCRE
                anchor_elements.push(Atom::new(type_sk, vec![var_term]));

                // 2. MARQUAGE LOGIQUE (Indispensable pour le Datalog)
                covered_vars.insert(var_id);
            }
        }
        // ==========================================================

        // 4. Génération de l'Ancre et de la règle finale
        if !anchor_elements.is_empty() {
            // On crée l'atome de tête de l'ancre (ex: anchor_move(?r, ?l))
            let anchor_head = self
                .encoder
                .generate_anchor_atom(action.name(), parameters, &head);

            // Règle : anchor_move(...) :- at-rob(?r, ?l), is-robot(?r)...
            self.push_rule(Rule::new(anchor_head.clone(), anchor_elements));

            // L'action dépend maintenant de son ancre
            final_action_body.push(anchor_head);
        }

        // 5. On ajoute la partie dynamique (les Aux_N générés par encode_preconditions)
        if let Some(p_atom) = precond_opt {
            final_action_body.push(p_atom);
        }

        // Règle finale : move(...) :- anchor_move(...), aux_precond(...)
        self.push_rule(Rule::new(head, final_action_body));

        Ok(())
    }

    /*fn compile_action_body_as_rules(
        &mut self,
        action: &ActionDef,
        head: Atom,
    ) -> Result<(), DatalogError> {
        // 1. Aplatissement des préconditions
        // Génère les prédicats auxiliaires (Aux_N) et l'ancre potentielle
        let precond_opt = self.encoder.encode_preconditions(
            action.precondition(),
            &mut self.rules,
            action.parameters(),
            head.skeleton_id(),
        )?;

        // On clone l'ancre pour libérer l'emprunt (borrow) sur self.encoder
        let anchor_opt = self.encoder.action_anchor().cloned();

        // 2. Initialisation du corps de la règle finale de l'action
        let mut final_action_body = Vec::new();

        // 3. Gestion de l'ANCRE ou des TYPE GUARDS directes
        if let Some(anchor) = anchor_opt {
            let mut anchor_definition_body = Vec::new();

            // Définition de l'ancre par les types des paramètres
            for (i, param) in action.parameters().iter().enumerate() {
                let var_term = head.terms()[i].clone();
                let members = param.ty().members();

                let type_sk = if members.is_empty() {
                    // Cas Type Racine (Root)
                    self.type_to_skeleton
                        .last()
                        .copied()
                        .ok_or_else(|| DatalogError::internal_state("Root Type skeleton missing"))?
                } else if members.len() == 1 {
                    // Cas Type simple
                    self.type_to_skeleton[members[0].as_usize()]
                } else {
                    // Cas Union de types (nécessite self mutable)
                    self.get_or_create_union_predicate(members)
                };
                anchor_definition_body.push(Atom::new(type_sk, vec![var_term]));
            }

            // Règle : Ancre(?params) :- Type(?params)
            // On clone anchor ici car on va l'ajouter à final_action_body juste après
            self.push_rule(Rule::new(anchor.clone(), anchor_definition_body));

            // L'action dépend de son ancre
            final_action_body.push(anchor);
        } else {
            // Si l'action n'a pas d'ancre (ex: pas de préconditions ou paramètres simples),
            // on injecte les Type Guards directement dans le corps de l'action.
            for (i, param) in action.parameters().iter().enumerate() {
                let var_term = head.terms()[i].clone();
                let members = param.ty().members();

                let type_sk = if members.is_empty() {
                    self.type_to_skeleton.last().copied().unwrap()
                } else if members.len() == 1 {
                    self.type_to_skeleton[members[0].as_usize()]
                } else {
                    self.get_or_create_union_predicate(members)
                };
                final_action_body.push(Atom::new(type_sk, vec![var_term]));
            }
        }

        // 4. Ajout de l'atome de précondition (résultat de l'aplatissement)
        if let Some(p_atom) = precond_opt {
            final_action_body.push(p_atom);
        }

        // 5. Règle finale de l'action
        // Exemple : assemble(?x, ?y) :- action_anchor_assemble(?x, ?y), aux_precond_1(?x, ?y)
        // 'head' est déplacé (moved) ici dans sa règle finale.
        self.push_rule(Rule::new(head, final_action_body));

        Ok(())
    }*/

    /// Compile la règle : Action :- Types, Preconditions.
    /*fn compile_action_body_as_rules(
        &mut self,
        action: &ActionDef,
        head: Atom,
    ) -> Result<(), DatalogError> {
        // 1. Aplatir la précondition (Génère les AUXILIAIRES > action_threshold)
        let precond_opt = self.encoder.encode_preconditions(
            action.precondition(),
            &mut self.rules,
            action.parameters(),
            head.skeleton_id(),
        )?;

        // 2. Préparer le corps avec les TYPE GUARDS
        let mut body = Vec::new();
        // On utilise le constructeur statique .internal_state() pour générer l'erreur avec la trace
        let root_type_sk_id = self.type_to_skeleton.last().copied().ok_or_else(|| {
            DatalogError::internal_state(
                "DatalogEngine must have at least a Root Type skeleton before compiling actions",
            )
        })?;

        for (i, param) in action.parameters().iter().enumerate() {
            let var_term = head.terms()[i].clone();
            let members = param.ty().members();

            if members.is_empty() {
                // CAS 1 : Type Racine (STRIPS ou Feuille directe)
                // On utilise l'ID du squelette Root
                body.push(Atom::new(root_type_sk_id, vec![var_term]));
            } else {
                // CAS 2 : Type Pivot (Union de racines)
                // Comme ton flattener garantit que members contient des types racines,
                // on DOIT générer un "OU" (Union) pour que l'objet soit valide
                // s'il appartient à n'importe lequel de ces types.

                if members.len() == 1 {
                    let type_sk_id = self.type_to_skeleton[members[0].as_usize()];
                    body.push(Atom::new(type_sk_id, vec![var_term]));
                } else {
                    // Utilise la fonction de cache d'union qu'on a vue précédemment
                    let union_sk_id = self.get_or_create_union_predicate(members);
                    body.push(Atom::new(union_sk_id, vec![var_term]));
                }
            }
        }

        // 3. Ajouter l'atome de précondition aplatie si nécessaire
        if let Some(body_atom) = precond_opt {
            body.push(body_atom);
        }

        self.push_rule(Rule::new(head, body));

        Ok(())
    }*/

    /// Récupère ou crée un prédicat unaire auxiliaire qui représente
    /// l'union de plusieurs types primitifs.
    fn get_or_create_union_predicate(&mut self, members: &[TypeId]) -> AtomSkeletonId {
        // 1. On vérifie si cette union exacte existe déjà
        // (Note : Ton flattener trie déjà les members, donc la clé est stable)
        if let Some(&existing_id) = self.union_cache.get(members) {
            return existing_id;
        }

        // 2. On crée un nouveau prédicat auxiliaire (unaire)
        let union_sk_id = self.encoder.encode_type_as_unary_predicate();

        // 3. Pour chaque typing de l'union, on crée une règle :
        // Union(?x) :- Type_i(?x)
        for &type_id in members {
            let type_sk_id = self.type_to_skeleton[type_id.as_usize()];

            // On utilise une variable standard (ex: index 0) pour le corps
            let var_x = Term::Variable(VariableId::from(0));

            let head = Atom::new(union_sk_id, vec![var_x.clone()]);
            let body = vec![Atom::new(type_sk_id, vec![var_x])];

            // On ajoute la règle au moteur
            //self.rules.push(Rule::new(head, body));
            self.push_rule(Rule::new(head, body));
        }

        // 4. On met en cache et on retourne
        self.union_cache.insert(members.to_vec(), union_sk_id);
        union_sk_id
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
                    if let Some(obj_id) = bindings.get(&param.symbol()) {
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

    // Pour vérifier l'état (lecture seule)
    #[cfg(test)]
    pub fn encoder(&self) -> &DatalogEncoder {
        &self.encoder
    }

    #[cfg(test)]
    pub fn encoder_mut(&mut self) -> &mut DatalogEncoder {
        &mut self.encoder
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

    fn render_context(&self) -> RenderContext {
        RenderContext::new(
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
