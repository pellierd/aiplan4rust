use std::collections::HashMap;
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::atom::Atom;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::database::Database;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::error::DatalogError;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::encoder::DatalogEncoder;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::rule::Rule;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::term::Term;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::tuple::Tuple;
use crate::aiplan4rust::lang::{ActionDefId, AtomSkeletonId, Id, ObjectId, TypeId, TypedSymbol, VariableId};
use crate::aiplan4rust::lir::expr::{Expr, ExprKind};
use crate::aiplan4rust::lir::ActionDef;
use crate::aiplan4rust::lir::problem::LiftedProblem;

/// Maximum number of variables (parameters) allowed per action or rule.
///
/// This limit is set to 64 to allow high-performance variable tracking
/// using a single CPU register (u64 bitset).
pub const MAX_VARS: usize = 64;

// pre requis les types doivent faltten et les quantfier remove pas d'imply
pub struct DatalogEngine {
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
    action_threshold: usize,
    builtin_threshold: usize,
    /// Cache pour ne pas dupliquer les prédicats d'union.
    /// Clé : La liste triée des TypeId. Valeur : L'ID du squelette Datalog.
    union_cache: HashMap<Vec<TypeId>, AtomSkeletonId>,
}

impl DatalogEngine {


    pub fn new() -> Self {
        Self {
            db: Database::new(),
            rules: Vec::new(),
            current_env: [None; MAX_VARS],
            trailing_indices: Vec::with_capacity(MAX_VARS),
            discovered_facts: Vec::with_capacity(1024),
            type_to_skeleton: Vec::new(),
            head_buffer: Vec::with_capacity(16),
            encoder: DatalogEncoder::new(0),
            fluence_threshold: 0,
            type_threshold: 0,
            action_threshold: 0,
            builtin_threshold: 0,
            union_cache: HashMap::new(),
        }
    }

    pub fn get_reachable_fluents(&self) -> Vec<Tuple<AtomSkeletonId>> {
        let mut fluents = Vec::new();

        // On parcourt les relations de la DB (le stockage Datalog)
        for (&sk_id, rel) in self.db.relations().iter() {

            // On ne garde que ce qui appartient aux Fluents (Prédicats)
            if self.is_fluent(sk_id) {

                // Le sk_id est déjà notre AtomSkeletonId interne
                let skeleton_id = AtomSkeletonId::from(sk_id);

                for tuple_data in rel.iter() {
                    // On crée un Tuple pour chaque ligne de la relation
                    fluents.push(Tuple::new(
                        skeleton_id,
                        tuple_data.to_vec()
                    ));
                }
            }
        }
        fluents
    }

    pub fn get_reachable_actions(&self) -> Vec<Tuple<ActionDefId>> {
        let mut actions = Vec::with_capacity(self.db.relations().len());

        for (&sk_id, rel) in self.db.relations().iter() {
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
                        actions.push(Tuple::new(
                            action_def_id,
                            tuple_data.to_vec()
                        ));
                    }
                }
            }
        }
        actions
    }

    pub fn get_type_extensions(&self) -> Vec<Tuple<TypeId>> {
        // On pré-alloue par rapport au nombre de relations, comme pour les actions
        let mut types = Vec::with_capacity(self.db.relations().len());

        for (&sk_id, rel) in self.db.relations().iter() {
            let id_val = sk_id;

            // 1. Utilisation de la méthode de segment pour les Types
            if self.is_type(id_val) {

                // 2. Traduction arithmétique inline (O(1))
                // On soustrait le fluence_threshold pour retrouver l'index du type
                let type_id = self.atom_id_to_type_id(sk_id);

                for tuple_data in rel.iter() {
                    // 3. Création du Tuple (souvent unaire pour les types)
                    types.push(Tuple::new(
                        type_id,
                        tuple_data.to_vec()
                    ));
                }
            }
        }
        types
    }

    pub fn get_rule_for_action(&self, action_index: usize) -> &Rule {
        // L'ID interne est calculé directement ici
        let target_sk_id = AtomSkeletonId::from(self.type_threshold + action_index);

        self.rules.iter()
            .find(|r| r.head().skeleton_id() == target_sk_id)
            .expect("Aucune règle trouvée pour cet index d'action")
    }

    pub fn load_problem(&mut self, problem: &LiftedProblem) -> Result<(), DatalogError> {
        // 1. Internal State Reset
        // Reset the fact database and clear existing inference rules.
        self.db = Database::new();
        self.rules.clear();

        // Segment 1: PDDL Fluents
        // Define the first ID segment based on domain predicates.
        self.fluence_threshold = problem.predicate_defs().len();
        self.encoder = DatalogEncoder::new(self.fluence_threshold);

        // Segment 2: Types & Hierarchy Auxiliaries
        self.declare_types_as_unary_predicates(problem.type_defs());

        // We now use self.encoder.current_id() instead of manual length calculation.
        //
        // WHY: Our internal vector includes BOTH the PDDL domain types AND
        // the sentinel ROOT type, but more importantly, the encoder might have
        // generated hidden auxiliary predicates for type hierarchies or unions.
        // current_id() captures the REAL end of this segment in the encoder.
        self.type_threshold = self.encoder.current_id();

        // 3. Action Signature Declaration
        // Segment 3: Reserve IDs for action atoms.
        // This freezes the boundary for any future auxiliary predicates.
        // We declare actions first, then capture the new ID state.
        self.declare_action_as_predicates(problem.action_defs());
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
        self.fill_db_from_objects(problem.object_defs(), problem.type_defs())?;

        // 4.2. Initial State Instantiation (Facts: Predicate(Objects))
        self.fill_db_from_init(problem.init())?;

        // 5. Domain Logic Compilation
        // Generate Datalog rules (Preconditions -> Action -> Effects).
        // Any dynamically created predicates (auxiliaries) will have IDs >= action_threshold.
        self.compile_domain_actions_as_rules(problem.action_defs())?;

        // On synchronise le seuil sur la réalité de ce que l'encodeur a produit
        // après la génération des règles (qui peut avoir créé de nouveaux auxiliaires).
        self.builtin_threshold = self.encoder.current_id();

        Ok(())
    }

    #[inline]
    pub fn is_fluent(&self, id: AtomSkeletonId) -> bool {
        // .as_usize() fait un MASQUE (id & !NEGATION_FLAG)
        // On compare donc TOUJOURS l'index de base, peu importe le signe.
        id.as_usize() < self.fluence_threshold
    }

    /// Vérifie si un ID appartient au segment des Types (prédicats unaires).
    #[inline]
    pub fn is_type(&self, id: AtomSkeletonId) -> bool {
        let p = id.as_usize();
        p >= self.fluence_threshold && p < self.type_threshold
    }

    #[inline]
    pub fn atom_id_to_type_id(&self, sk_id: AtomSkeletonId) -> TypeId {
        let id_val = sk_id.as_usize();
        debug_assert!(self.is_type(sk_id)); // Utilise l'ID complet pour le check
        TypeId::from(id_val - self.fluence_threshold)
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

    #[inline]
    pub fn is_builtin(&self, id: AtomSkeletonId) -> bool {
        id.as_usize() >= Atom::BUILTIN_ZONE_START
    }

    #[inline]
    pub fn is_auxiliary(&self, id: AtomSkeletonId) -> bool {
        let p = id.as_usize();
        p >= self.builtin_threshold && p < Atom::BUILTIN_ZONE_START
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

        // Capacity for N domain types + 1 sentinel Root type
        self.type_to_skeleton = Vec::with_capacity(num_types + 1);

        // 1. Encode domain types first to ensure a 1:1 mapping with PDDL indices.
        // By keeping domain types at the start of the segment [0..num_types[,
        // we avoid a +1 offset in translation functions like `atom_id_to_type_id`.
        // Example: PDDL Type index 0 maps directly to Datalog ID (fluence_threshold + 0).
        for _ in 0..num_types {
            let sk_id = self.encoder.encode_type_as_unary_predicate();
            self.type_to_skeleton.push(sk_id);
        }

        // 2. Encode the ROOT type as a sentinel in the LAST slot.
        // This places the Root ID at the very end of the type segment (or start of auxiliary).
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
            DatalogError::internal_state("Root type skeleton missing".to_string())
        })?;

        for object in object_defs {
            let obj_id = object.symbol();

            // 1. On l'insère dans la sentinelle ROOT (ton garde-fou universel)
            self.db.insert_delta_fact(root_sk_id, &[obj_id]);

            // 2. Pour chaque type déclaré de l'objet (ex: [ball])
            for &type_id in object.ty() {
                // On l'insère dans le type lui-même
                let sk_id = self.type_to_skeleton[type_id.as_usize()];
                self.db.insert_delta_fact(sk_id, &[obj_id]);

                // 3. On l'insère dans TOUS les parents/membres identifiés par le flattener
                // Si members() est vide, cette boucle ne fait rien (c'est correct, ROOT suffit)
                if let Some(ty_def) = type_defs.get(type_id.as_usize()) {
                    for &parent_id in ty_def.ty().members() {
                        let parent_sk_id = self.type_to_skeleton[parent_id.as_usize()];
                        self.db.insert_delta_fact(parent_sk_id, &[obj_id]);
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
    fn fill_db_from_init(&mut self, init: &Expr) -> Result<(), DatalogError> {
        let mut iter = init.preorder().values();

        while let Some(node) = iter.next() {
            match node.kind() {
                ExprKind::AtomicFormula => {
                    // 1. Obtenir le SkeletonId correspondant à cet atome
                    // On demande au evaluator de nous donner l'ID de la signature (Nom + Types)
                    let sk_id = node.content().try_atom_skeleton()?;

                    // 2. Extraction des ObjectIds avec une boucle explicite
                    let children = node.children();
                    let mut args = Vec::with_capacity(children.len());

                    for &arg_id in children.iter().skip(1) {
                        // On récupère le noeud enfant
                        let child_node = init.try_node(arg_id)?;
                        // On extrait la constante (l'ObjectId)
                        let object_id = child_node.content().try_object()?;
                        // On l'ajoute à notre liste d'arguments
                        args.push(object_id);
                    }

                    // 3. Ajouter le fait à la Database interne
                    self.db.insert_delta_fact(sk_id, &args);

                    // On a traité l'atome, on saute ses enfants
                    iter.skip_subtree();
                }
                ExprKind::Comparison | ExprKind::Not => {
                    // Optionnel : Gestion des fonctions numériques si ton domaine en a
                    // Pour l'instant, on peut skip si on se concentre sur le logique
                    iter.skip_subtree();
                }
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
        action_defs: &[ActionDef] // On ne passe que les définitions d'actions
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
        action_sk_id: AtomSkeletonId
    ) -> Result<(), DatalogError> {
        // A. Générer l'atome de nom (Pivot : action(?p1, ?p2...))
        let action_atom = self.compile_action_name_as_rules(action, action_sk_id);

        // B. Générer la règle de déclenchement (Preconditions -> Action)
        self.compile_action_body_as_rules(action, action_atom.clone())?;

        // --- LE BOOTSTRAP EST ICI ---
        // On vérifie la règle de déclenchement qu'on vient de pousser
        if let Some(trigger_rule) = self.rules.last() {
            // Si le corps est vide et l'action n'a pas de paramètres (?x)
            if trigger_rule.body().is_empty() && action.parameters().is_empty() {
                // On l'injecte comme un fait car elle est "toujours vraie"
                self.db.insert_delta_fact(action_sk_id, &[]);
            }
        }

        // C. Générer les règles de causalité (Action -> Effets)
        self.encoder.encode_effects(
            action.effect(),
            &action_atom,
            &mut self.rules,
            action.parameters()
        )?;

        Ok(())
    }

    /// Génère l'atome de tête représentant l'action avec ses paramètres.
    fn compile_action_name_as_rules(
        &self,
        action: &ActionDef,
        action_sk_id: AtomSkeletonId,
    ) -> Atom {
        let head_terms: Vec<Term> = action
            .parameters()
            .iter()
            .map(|param| Term::Variable(param.symbol()))
            .collect();

        Atom::new(action_sk_id, head_terms)
    }

    /// Compile la règle : Action :- Types, Preconditions.
    fn compile_action_body_as_rules(
        &mut self,
        action: &ActionDef,
        head: Atom,
    ) -> Result<(), DatalogError> {
        // 1. Aplatir la précondition (Génère les AUXILIAIRES > action_threshold)
        let precond_opt = self.encoder.encode_preconditions(
            action.precondition(),
            &mut self.rules,
            action.parameters()
        )?;

        // 2. Préparer le corps avec les TYPE GUARDS
        let mut body = Vec::new();
        // On utilise le constructeur statique .internal_state() pour générer l'erreur avec la trace
        let root_type_sk_id = self.type_to_skeleton.last()
            .copied()
            .ok_or_else(|| DatalogError::internal_state(
                "DatalogEngine must have at least a Root Type skeleton before compiling actions"
            ))?;

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
    }

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

        // 3. Pour chaque type de l'union, on crée une règle :
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
        // On pourrait imaginer ici des étapes de pré-calcul
        // avant de lancer la saturation proprement dite.
        self.saturate_semi_naive();
    }

    fn saturate_semi_naive(&mut self) {
        // 1. BOOTSTRAP : On ne déplace vers delta que si le delta est vide
        // et qu'on a des choses en stable (cas d'un moteur qu'on relancerait).
        if self.db.is_delta_empty() && !self.db.relations().is_empty() {
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
        if body_idx == rule.body().len() {
            self.evaluate_head(rule);
            return;
        }

        let atom = &rule.body()[body_idx];
        let sk_id = atom.skeleton_id();


        // --- NOUVEAU : Gestion des Built-ins (Filtres) ---
        if self.is_builtin(sk_id) {
            // L'égalité n'est jamais un pivot car elle n'est pas dans la DB.
            // On l'exécute simplement comme un test.
            if self.execute_builtin(atom) {
                self.process_incremental(rule, body_idx + 1, pivot_idx);
            }
            return;
        }

        // --- Logique existante pour les relations standards ---
        if body_idx < pivot_idx {
            self.match_relation(rule, body_idx, pivot_idx, sk_id, false);
        } else if body_idx == pivot_idx {
            self.match_relation(rule, body_idx, pivot_idx, sk_id, true);
        } else {
            self.match_relation(rule, body_idx, pivot_idx, sk_id, false);
            self.match_relation(rule, body_idx, pivot_idx, sk_id, true);
        }
    }

    fn execute_builtin(&self, atom: &Atom) -> bool {
        let sk_id = atom.skeleton_id().as_usize();


        // CENTRALISATION : On dispatch selon l'ID du prédicat
        match sk_id {
            // Cas 1 : L'ÉGALITÉ (Le cas qui nous intéresse pour Gripper)
            Atom::EQUALITY_ID => {
                self.eval_equality(atom)
            }

            // Cas 2 : On peut imaginer d'autres built-ins ici plus tard
            // Atom::GREATER_THAN_ID => self.eval_numeric_gt(atom),

            // Par défaut, si on ne connaît pas, on laisse passer (ou on log une erreur)
            _ => true,
        }
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

    fn match_relation(&mut self, rule: &Rule, body_idx: usize, pivot_idx: usize, sk_id: AtomSkeletonId, use_delta: bool) {
        let atom = &rule.body()[body_idx];
        let terms = atom.terms();

        let Some((total_len, arity)) = self.db.get_layout(sk_id, use_delta) else { return; };
        let mut tuple_buffer = [ObjectId::from(0); MAX_VARS];

        // 1. CAS ARITÉ 0 : On traite la proposition si elle est présente dans la table demandée
        if arity == 0 {
            // Si total_len est 0 mais que la table (Delta ou Stable selon use_delta)
            // contient la proposition, on déclenche l'unification une fois.
            let is_present = if use_delta { self.db.contains_delta(sk_id, &[]) }
            else { self.db.contains_stable(sk_id, &[]) };

            if is_present {
                self.process_tuple(rule, body_idx, pivot_idx, sk_id, use_delta, 0, 0, &mut tuple_buffer);
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
                    self.process_tuple(rule, body_idx, pivot_idx, sk_id, use_delta, start, arity, &mut tuple_buffer);
                }
            }
        } else {
            // MODE SCAN COMPLET
            // step_by(arity) avec arity > 0 est sûr ici.
            for start in (0..total_len).step_by(arity) {
                self.process_tuple(rule, body_idx, pivot_idx, sk_id, use_delta, start, arity, &mut tuple_buffer);
            }
        }
    }

    // Petite fonction utilitaire pour éviter la duplication de code
    fn process_tuple(&mut self, rule: &Rule, body_idx: usize, pivot_idx: usize, sk_id: AtomSkeletonId, use_delta: bool, start: usize, arity: usize, buffer: &mut [ObjectId; MAX_VARS]) {
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
                    // Si ton grounding est correct, toute variable en tête doit être liée dans le corps
                    let val = self.current_env[v.as_usize()]
                        .ok_or_else(|| DatalogError::unbound_variable(*v))?;
                    self.head_buffer.push(val);
                }
            }
        }

        // 2. FILTRAGE : On ne veut pas stocker de doublons.
        // On vérifie dans la DB (Stable + Delta)
        if self.db.contains_stable(head_sk, &self.head_buffer) ||
            self.db.contains_delta(head_sk, &self.head_buffer) {
            return Ok(());
        }

        // 3. On vérifie aussi dans les découvertes du pivot en cours
        // pour éviter de cloner inutilement si le même fait est trouvé 100 fois de suite.
        let is_already_in_buffer = self.discovered_facts
            .iter()
            .any(|(sk, args)| *sk == head_sk && args == &self.head_buffer);

        if !is_already_in_buffer {
            // Le clone n'arrive qu'ici, au dernier moment possible.
            self.discovered_facts.push((head_sk, self.head_buffer.clone()));
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
        if body.len() <= 1 { return; }

        let mut optimized = Vec::with_capacity(body.len());
        let mut bound_vars_mask: u64 = 0;
        let mut remaining = std::mem::take(body);

        while !remaining.is_empty() {
            let best_idx = remaining.iter().enumerate().min_by_key(|(_, atom)| {
                let sk_id = atom.skeleton_id();
                let priority = self.get_predicate_priority(sk_id);

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

                let rel_size = self.db.get_relation_size(sk_id);

                // CHANGEMENT ICI : Le bound_count est le critère ROI
                (-(bound_count as i32), priority, rel_size)
            }).map(|(idx, _)| idx).unwrap();

            let best_atom = remaining.remove(best_idx);

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
    pub fn get_predicate_priority(&self, id: AtomSkeletonId) -> u8 {
        // Note : On passe l'objet `id` complet aux fonctions.
        // Elles utiliseront id.as_usize() en interne pour ignorer la négation.

        if self.is_builtin(id) {
            4
        } else if self.is_type(id) {
            0
        } else if self.is_fluent(id) {
            1
        } else if self.is_action(id) {
            2
        } else {
            // Cas des prédicats auxiliaires générés par le flattener
            3
        }
    }

}

#[cfg(test)]
#[path = "tests/engine_tests.rs"]
mod engine_tests;
