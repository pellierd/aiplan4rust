use std::collections::HashMap;
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::atom::Atom;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::database::Database;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::error::DatalogError;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::encoder::DatalogEncoder;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::rule::Rule;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::term::Term;
use crate::aiplan4rust::grounding::problem::Fluent;
use crate::aiplan4rust::lang::{ActionDefId, AtomSkeletonId, Id, ObjectId, TypeId, TypedSymbol};
use crate::aiplan4rust::lir::expr::{Expr, ExprKind};
use crate::aiplan4rust::lir::LiftedAction;
use crate::aiplan4rust::lir::problem::LiftedProblem;

// pre requis les types doivent faltten et les quantfier remove pas d'imply
pub struct DatalogEngine {
    db: Database,
    rules: Vec<Rule>,
    current_env: [Option<ObjectId>; 64],
    // Buffer temporaire pour stocker les faits trouvés pour une règle
    discovered_facts: Vec<(AtomSkeletonId, Vec<ObjectId>)>,
    type_to_skeleton: Vec<AtomSkeletonId>,
    head_buffer: Vec<ObjectId>,
    encoder: DatalogEncoder,
    fluence_threshold: usize,
    type_threshold: usize,
    action_threshold: usize,

}

impl DatalogEngine {

    /// The index in `type_to_skeleton` where the universal 'object' type is stored.
    const ROOT_TYPE_SKELETON_INDEX: usize = 0;

    pub fn new() -> Self {
        Self {
            db: Database::new(),
            rules: Vec::new(),
            current_env: [None; 64],
            discovered_facts: Vec::with_capacity(128),
            type_to_skeleton: Vec::new(),
            head_buffer: Vec::with_capacity(16),
            encoder: DatalogEncoder::new(0),
            fluence_threshold: 0,
            type_threshold: 0,
            action_threshold: 0,
        }
    }

    /*pub fn get_reachable_actions(&self) -> Vec<Atom> {
        let mut actions = Vec::new();

        // On parcourt toutes les relations stockées dans la base
        for (&sk_id, rel) in self.db.relations().iter() {
            // On vérifie si le SkeletonId appartient au segment des Actions
            if self.is_action(sk_id.as_usize()) {
                // Pour chaque tuple (liste d'objets) dans cette relation
                for tuple in rel.iter() {
                    actions.push(Atom::new(sk_id, tuple.to_vec()));
                }
            }
        }
        actions
    }

    pub fn get_reachable_fluents(&self) -> Vec<Fluent> {
        let mut fluents = Vec::new();

        for (&sk_id, rel) in self.db.relations().iter() {
            // 1. On filtre toujours par segment (Segment 1 = Fluents)
            if self.is_fluent(sk_id.as_usize()) {

                // 2. On récupère le symbole original via l'encodeur
                // Je suppose que ton encoder a une méthode pour ça
                let symbol = self.encoder.get_symbol(sk_id);

                for tuple in rel.iter() {
                    // tuple est un &[ObjectId], on le clone pour le Fluent
                    let parameters = tuple.to_vec();

                    // 3. On crée le Fluent avec le VRAI PredicateSymbolId
                    fluents.push(Fluent::new(symbol, parameters));
                }
            }
        }
        fluents
    }*/

    /// Prépare le moteur pour un nouveau problème.
    /// Configure la base de faits, définit les types et compile le domaine en règles Datalog.
    pub fn load_problem(&mut self, problem: &LiftedProblem) -> Result<(), DatalogError> {
        // 1. Internal State Reset
        // Reset the fact database and clear existing inference rules.
        self.db = Database::new();
        self.rules.clear();

        // Segment 1: PDDL Fluents
        // Define the first ID segment based on domain predicates.
        self.fluence_threshold = problem.predicate_defs().len();
        self.encoder = DatalogEncoder::new(self.fluence_threshold);

        // 2. Type Schema Declaration
        // Segment 2: Reserve IDs for unary predicates representing types.
        self.declare_types_as_unary_predicates(problem.type_defs());
        self.type_threshold = self.fluence_threshold + self.type_to_skeleton.len();

        // 3. Action Signature Declaration
        // Segment 3: Reserve IDs for action atoms.
        // This freezes the boundary for any future auxiliary predicates.
        self.declare_action_as_predicates(problem.action_defs());
        self.action_threshold = self.type_threshold + problem.action_defs().len();

        // 4. Problem Data Ingestion
        // Populate the Database with concrete facts.

        // 4.1. Type Instantiation (Facts: Type(Object))
        self.fill_db_from_objects(problem.object_defs())?;

        // 4.2. Initial State Instantiation (Facts: Predicate(Objects))
        self.fill_db_from_init(problem.init())?;

        // 5. Domain Logic Compilation
        // Generate Datalog rules (Preconditions -> Action -> Effects).
        // Any dynamically created predicates (auxiliaries) will have IDs >= action_threshold.
        self.compile_domain_actions_as_rules(problem.action_defs())?;

        Ok(())
    }

    /// Vérifie si un ID appartient au segment des Fluents (prédicats d'état PDDL).
    #[inline]
    pub fn is_fluent(&self, id: usize) -> bool {
        // Logique : tout ce qui est avant le premier seuil
        id < self.fluence_threshold
    }

    /// Vérifie si un ID appartient au segment des Types (prédicats unaires).
    #[inline]
    pub fn is_type(&self, id: usize) -> bool {
        // Dépend du seuil des fluents et de celui des types
        id >= self.fluence_threshold && id < self.type_threshold
    }

    /// Vérifie si un ID appartient au segment des Actions.
    #[inline]
    pub fn is_action(&self, id: usize) -> bool {
        // Dépend du seuil des types et de celui des actions
        id >= self.type_threshold && id < self.action_threshold
    }

    /// Vérifie si un ID est un prédicat auxiliaire (créé lors de la compilation).
    #[inline]
    pub fn is_auxiliary(&self, id: usize) -> bool {
        // Tout ce qui dépasse le dernier seuil fixé
        id >= self.action_threshold
    }

    fn declare_types_as_unary_predicates(&mut self, type_defs: &[TypedSymbol<TypeId, TypeId>]) {
        let num_types = type_defs.len();

        // On réserve N + 1 places (Racine + Types du domaine)
        self.type_to_skeleton = Vec::with_capacity(num_types + 1);

        // 1. Encode the ROOT type (sentinel ID)
        // It will be stored at index 0 of our internal vector by convention.
        let root_type_sk = self.encoder.encode_type_as_unary_predicate(TypeId::root());
        self.type_to_skeleton.push(root_type_sk);

        // 2. Encode all other domain types
        for id in 0..num_types {
            let type_id = TypeId::from(id);
            let sk_id = self.encoder.encode_type_as_unary_predicate(type_id);
            self.type_to_skeleton.push(sk_id);
        }
    }

    fn fill_db_from_objects(&mut self, object_defs: &[TypedSymbol<ObjectId, TypeId>]) -> Result<(), DatalogError> {

        // On extrait l'ID une seule fois avant de commencer la boucle
        let root_sk_id = self.type_to_skeleton[Self::ROOT_TYPE_SKELETON_INDEX];

        for object in object_defs {
            let obj_id = object.symbol();
            let obj_type = object.ty();

            if !obj_type.is_empty() {
                // Le type_def contient maintenant tous les parents terminaux
                for &parent_type_id in obj_type {
                    // On récupère le skeleton ID pour ce type parent
                    let sk_id = self.type_to_skeleton[parent_type_id.as_usize()];
                    // On ajoute le fait : l'objet appartient à ce type racine
                    self.db.insert_stable_fact(sk_id, &[obj_id]);
                }
            } else {
                // Cas particulier : si members est vide, c'est l'objet racine (object)
                self.db.insert_stable_fact(root_sk_id, &[obj_id]);
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
                    // On demande au registry de nous donner l'ID de la signature (Nom + Types)
                    let sk_id = node.content().try_atom_skeleton()?;

                    // 2. Extraction des ObjectIds avec une boucle explicite
                    let children = node.children();
                    let mut args = Vec::with_capacity(children.len());

                    for &arg_id in children {
                        // On récupère le noeud enfant
                        let child_node = init.try_node(arg_id)?;
                        // On extrait la constante (l'ObjectId)
                        let object_id = child_node.content().try_constant()?;
                        // On l'ajoute à notre liste d'arguments
                        args.push(object_id);
                    }

                    // 3. Ajouter le fait à la Database interne
                    self.db.insert_delta_fact(sk_id, &args);

                    // On a traité l'atome, on saute ses enfants
                    iter.skip_subtree();
                }
                ExprKind::FComp | ExprKind::Not => {
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
    fn declare_action_as_predicates(&mut self, action_defs: &[LiftedAction]) {
        for (id, action) in action_defs.iter().enumerate() {
            // Cette méthode dans ton encoder doit simplement créer l'ID
            // et l'ajouter à son mapping interne (ex: action_to_skeleton).
            self.encoder.encode_action_as_predicate(action, ActionDefId::from(id));
        }
    }

    fn compile_domain_actions_as_rules(
        &mut self,
        action_defs: &[LiftedAction] // On ne passe que les définitions d'actions
    ) -> Result<(), DatalogError> {
        for (id, action) in action_defs.iter().enumerate() {
            // L'ID est toujours basé sur le threshold + l'index dans la liste
            let action_sk_id = AtomSkeletonId::from(self.type_threshold + id);

            // Compilation de l'unité (on garde action_as_rule au singulier ici)
            self.compile_action_as_rules(action, action_sk_id)?;
        }

        // 1. On extrait les règles de self (O(1) - simple échange de pointeurs)
        // self.rules devient temporairement un Vec vide.
        let mut rules_to_optimize = std::mem::take(&mut self.rules);

        // 2. On itère sur les règles extraites
        for rule in &mut rules_to_optimize {
            // MAGIE DU BORROW CHECKER :
            // - 'self' est disponible car il ne possède plus le vecteur qu'on itère.
            // - 'rule' est mutable, donc optimize_body peut modifier le corps.
            self.optimize_body(rule.body_mut());
        }

        // 3. On remet les règles optimisées dans le moteur
        self.rules = rules_to_optimize;

        Ok(())
    }

    fn compile_action_as_rules(
        &mut self,
        action: &LiftedAction,
        action_sk_id: AtomSkeletonId
    ) -> Result<(), DatalogError> {
        // A. Générer l'atome de nom (Pivot : action(?p1, ?p2...))
        let action_atom = self.compile_action_name_as_rules(action, action_sk_id);

        // B. Générer la règle de déclenchement (Preconditions -> Action)
        self.compile_action_body_as_rules(action, action_atom.clone())?;

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
        action: &LiftedAction,
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
        action: &LiftedAction,
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
        for (i, param) in action.parameters().iter().enumerate() {
            // Récupère la variable correspondante (?p_i)
            let var_term = head.terms()[i].clone();

            // Récupère l'ID du prédicat de type associé
            let type_sk_id = self.type_to_skeleton[param.ty().members()[0].as_usize()];
            body.push(Atom::new(type_sk_id, vec![var_term]));
        }

        // 3. Ajouter l'atome de précondition aplatie si nécessaire
        if let Some(body_atom) = precond_opt {
            body.push(body_atom);
        }

        // On optimise le corps localement pour l'ordre des prédicats
        self.optimize_body(&mut body);

        // On enregistre la règle finale dans le moteur
        self.rules.push(Rule::new(head, body));

        Ok(())
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
        // 1. Amorçage : on déplace l'état initial (Init) dans le delta
        self.db.move_all_to_delta();

        // Tant qu'il y a de nouveaux faits à traiter
        while !self.db.is_delta_empty() {
            // Astuce pour contourner le borrow checker : on extrait les règles temporairement
            let rules = std::mem::take(&mut self.rules);

            for rule in &rules {
                let body_len = rule.body().len();

                // Le semi-naïf impose de tester chaque atome du corps comme "pivot"
                for pivot_idx in 0..body_len {
                    self.current_env = [None; 64];
                    self.discovered_facts.clear();

                    // On lance l'exploration incrémentale
                    self.process_incremental(rule, 0, pivot_idx);

                    // On stocke les découvertes dans le delta du PROCHAIN tour
                    for (sk_id, args) in self.discovered_facts.drain(..) {
                        self.db.insert_delta_fact(sk_id, &args);
                    }
                }
            }

            // On remet les règles en place
            self.rules = rules;

            // 2. Fin de tour : les nouveaux faits deviennent anciens
            self.db.commit_delta();
        }
    }



    // Note : db est maintenant &Database (immutable)
    fn process_incremental(&mut self, rule: &Rule, body_idx: usize, pivot_idx: usize) {
        if body_idx == rule.body().len() {
            self.evaluate_head(rule);
            return;
        }

        let sk_id = rule.body()[body_idx].skeleton_id();

        if body_idx < pivot_idx {
            // On cherche dans le STABLE
            self.match_relation(rule, body_idx, pivot_idx, sk_id, false);
        } else if body_idx == pivot_idx {
            // On cherche dans le DELTA
            self.match_relation(rule, body_idx, pivot_idx, sk_id, true);
        } else {
            // On cherche dans les DEUX
            self.match_relation(rule, body_idx, pivot_idx, sk_id, false);
            self.match_relation(rule, body_idx, pivot_idx, sk_id, true);
        }
    }

    fn match_relation(&mut self, rule: &Rule, body_idx: usize, pivot_idx: usize, sk_id: AtomSkeletonId, use_delta: bool) {
        let atom = &rule.body()[body_idx];
        let terms = atom.terms();
        let Some((total_len, arity)) = self.db.get_layout(sk_id, use_delta) else { return; };
        let mut tuple_buffer = [ObjectId::from(0); 12];

        // --- STRATÉGIE D'INDEXATION ---
        // On regarde si le premier argument est déjà "fixé"
        let first_arg_binding = match &terms[0] {
            Term::Constant(c) => Some(*c),
            Term::Variable(v) => self.current_env[v.as_usize()],
        };

        if let Some(obj_id) = first_arg_binding {
            // MODE INDEXÉ : On ne récupère que les offsets où le premier argument match
            if let Some(offsets) = self.db.lookup_index(sk_id, use_delta, obj_id) {
                for start in offsets {
                    self.process_tuple(rule, body_idx, pivot_idx, sk_id, use_delta, start, arity, &mut tuple_buffer);
                }
            }
        } else {
            // MODE SCAN COMPLET : On parcourt tout (quand le premier argument est une variable libre)
            for start in (0..total_len).step_by(arity) {
                self.process_tuple(rule, body_idx, pivot_idx, sk_id, use_delta, start, arity, &mut tuple_buffer);
            }
        }
    }

    // Petite fonction utilitaire pour éviter la duplication de code
    fn process_tuple(&mut self, rule: &Rule, body_idx: usize, pivot_idx: usize, sk_id: AtomSkeletonId, use_delta: bool, start: usize, arity: usize, buffer: &mut [ObjectId; 12]) {
        self.db.read_tuple(sk_id, use_delta, start, arity, buffer);
        let prev_env = self.current_env;
        if self.unify_and_bind(rule.body()[body_idx].terms(), &buffer[..arity]) {
            self.process_incremental(rule, body_idx + 1, pivot_idx);
        }
        self.current_env = prev_env;
    }

    fn evaluate_head(&mut self, rule: &Rule) {
        let head = rule.head();
        let head_sk = head.skeleton_id();

        // On vide le buffer sans désallouer la mémoire (capacity conservée)
        self.head_buffer.clear();

        for term in head.terms() {
            match term {
                Term::Constant(c) => self.head_buffer.push(*c),
                Term::Variable(v) => {
                    let val = self.current_env[v.as_usize()]
                        .expect("Variable non liée dans la tête");
                    self.head_buffer.push(val);
                }
            }
        }

        // On ne clone que si le fait est VRAIMENT nouveau et va être inséré
        if !self.db.contains_stable(head_sk, &self.head_buffer) {
            // Ici le clone est nécessaire car on stocke le fait pour le futur
            self.discovered_facts.push((head_sk, self.head_buffer.clone()));
        }
    }

    /// Tente de faire correspondre les termes d'un atome (venant d'une règle)
    /// avec un tuple réel (venant de la Database).
    ///
    /// Retourne `true` si l'unification réussit, `false` sinon.
    /// Met à jour `self.current_env` avec les nouvelles liaisons de variables.
    fn unify_and_bind(&mut self, atom_terms: &[Term], tuple: &[ObjectId]) -> bool {
        // Sécurité : l'arité doit correspondre (normalement garanti par le flattener)
        if atom_terms.len() != tuple.len() {
            return false;
        }

        for (i, term) in atom_terms.iter().enumerate() {
            let val_in_db = tuple[i];

            match term {
                // 1. Si c'est une constante dans la règle, elle doit être égale à la valeur en DB
                Term::Constant(c) => {
                    if *c != val_in_db {
                        return false;
                    }
                }
                // 2. Si c'est une variable
                Term::Variable(v) => {
                    let idx = v.as_usize();
                    if let Some(existing_val) = self.current_env[idx] {
                        // Si la variable est déjà liée, elle doit pointer vers le même objet
                        if existing_val != val_in_db {
                            return false;
                        }
                    } else {
                        // Sinon, on crée une nouvelle liaison
                        self.current_env[idx] = Some(val_in_db);
                    }
                }
            }
        }
        true
    }


    fn optimize_body(&self, body: &mut Vec<Atom>) {
        if body.len() <= 1 { return; }

        let mut optimized = Vec::with_capacity(body.len());
        let mut bound_vars = std::collections::HashSet::new();
        let mut remaining = std::mem::take(body);

        while !remaining.is_empty() {
            let best_idx = remaining.iter().enumerate().min_by_key(|(_, atom)| {
                let sk_id = atom.skeleton_id().as_usize();

                // 1. Priority calculation based on Engine segments
                // Segment 2 (Types) > Segment 1 (Fluents) > Segment 3 (Actions) > Segment 4 (Aux)
                let priority = self.get_predicate_priority(sk_id);

                // 2. Calcul du nombre de termes fixés (Constantes + Variables déjà liées)
                let bound_count = atom.terms().iter().filter(|t| match t {
                    Term::Variable(v) => bound_vars.contains(v),
                    Term::Constant(_) => true,
                }).count();

                // On trie d'abord par catégorie (priority),
                // puis par le plus grand nombre de variables liées (via le signe négatif)
                (priority, -(bound_count as i32))
            }).map(|(idx, _)| idx).unwrap();

            let best_atom = remaining.remove(best_idx);

            // Mise à jour des variables liées pour les prochains atomes de la boucle
            for term in best_atom.terms() {
                if let Term::Variable(v) = term {
                    bound_vars.insert(*v);
                }
            }
            optimized.push(best_atom);
        }
        *body = optimized;
    }

    /// Retourne la priorité de tri pour l'optimisation (plus bas = plus prioritaire).
    #[inline(always)]
    fn get_predicate_priority(&self, id: usize) -> u8 {
        if self.is_type(id) {
            0 // Les types filtrent le plus, on les veut en premier.
        } else if self.is_fluent(id) {
            1 // Les fluents (At, On) sont la base de l'état.
        } else if self.is_action(id) {
            2 // L'action elle-même sert de pivot.
        } else {
            3 // Les auxiliaires sont évalués en dernier (coût potentiel élevé).
        }
    }
}
