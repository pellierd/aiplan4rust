use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::atom::Atom;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::database::Database;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::error::DatalogError;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::encoder::DatalogEncoder;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::rule::Rule;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::term::Term;
use crate::aiplan4rust::lang::{AtomSkeletonId, Id, ObjectId};
use crate::aiplan4rust::lir::expr::ExprKind;
use crate::aiplan4rust::lir::problem::LiftedProblem;


// pre requis les types doivent faltten et les quantfier remove pas d'imply
pub struct DatalogEngine {
    db: Database,
    rules: Vec<Rule>,
    current_env: [Option<ObjectId>; 64],
    // Buffer temporaire pour stocker les faits trouvés pour une règle
    discovered_facts: Vec<(AtomSkeletonId, Vec<ObjectId>)>,
    type_to_skeleton: Vec<AtomSkeletonId>,
    root_type_sk: AtomSkeletonId,
    head_buffer: Vec<ObjectId>
}

impl DatalogEngine {
    pub fn new() -> Self {
        Self {
            db: Database::new(),
            rules: Vec::new(),
            current_env: [None; 64],
            discovered_facts: Vec::with_capacity(128),
            type_to_skeleton: Vec::new(),
            root_type_sk: AtomSkeletonId::from(0),
            head_buffer: Vec::with_capacity(16)
        }
    }


    pub fn setup_types(&mut self, problem: &LiftedProblem, flattener: &mut DatalogEncoder) {
        // 1. On enregistre le type racine à part
        self.root_type_sk = flattener.encode_type_as_unary_predicate();

        // 2. On prépare le vecteur pour les types du problème uniquement
        let num_types = problem.type_defs().len();
        self.type_to_skeleton = Vec::with_capacity(num_types);

        // 3. On remplit le mapping direct
        for _ in 0..num_types {
            let sk_id = flattener.encode_type_as_unary_predicate();
            self.type_to_skeleton.push(sk_id);
        }
    }

    pub fn fill_db_from_problem(&mut self, problem: &LiftedProblem) -> Result<(), DatalogError> {
        // 1. REMPLISSAGE DES TYPES (La hiérarchie)
        self.fill_db_from_problem_objects(problem)?;

        // 2. REMPLISSAGE DES FAITS INITIAUX (Prédicats du domaine)
        self.fill_db_from_problem_init(problem)?;
        Ok(())
    }

    fn fill_db_from_problem_objects(&mut self, problem: &LiftedProblem) -> Result<(), DatalogError> {
        for object in problem.object_defs() {
            let obj_id = object.symbol();


            // Le type_def contient maintenant tous les parents terminaux
            for &parent_type_id in object.ty() {
                // On récupère le skeleton ID pour ce type parent
                let sk_id = self.type_to_skeleton[parent_type_id.as_usize()];

                // On ajoute le fait : l'objet appartient à ce type racine
                self.db.insert_stable_fact(sk_id, &[obj_id]);
            }

            // Cas particulier : si members est vide, c'est l'objet racine (object)
            if object.ty().is_empty() {
                let sk_id = self.type_to_skeleton[0]; // Index de 'object'
                self.db.insert_stable_fact(sk_id, &[obj_id]);
            }
        }
        Ok(())
    }

    // --- ÉTAPE 1 : INITIALISATION (L'Ingestion) ---
    /// Centralise ici la conversion du PDDL vers la Database interne.
    /// On passe un Registry ou le Problem pour mapper les IDs vers les SkeletonIds.
    /// Parcourt l'état initial du problème pour remplir la Database.
    fn fill_db_from_problem_init(&mut self, problem: &LiftedProblem) -> Result<(), DatalogError> {
        let init = problem.init();
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
                    self.db.insert_stable_fact(sk_id, &args);

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

    // --- ÉTAPE 2 : COMPILATION (Le Flattening) ---
    /// Transforme toutes les actions en règles et les stocke en interne.
    pub fn compile_domain(&mut self, problem: &LiftedProblem, flattener: &mut DatalogEncoder) -> Result<(), DatalogError> {

        // Premier seuil : Tout ce qui est avant ça est un TYPE
        let type_threshold = self.type_to_skeleton.len();

        // Deuxième seuil : Tout ce qui est avant ça (et après type) est un PREDICAT DU DOMAINE
        let domain_threshold = type_threshold + problem.predicate_defs().len();

        for action in problem.action_defs() {
            // 1. Aplatir la précondition
            let precond_opt = flattener.encode_expr(
                action.precondition(),
                &mut self.rules,
                action.parameters()
            )?;

            // 2. Créer l'identifiant unique pour l'action
            let action_sk_id = flattener.encode_action_as_predicate(action);

            // 3. Préparer les termes de la tête et les TYPE GUARDS
            let mut body = Vec::new();
            let mut head_terms = Vec::new();

            for param in action.parameters().iter() {
                let var_term = Term::Variable(param.symbol());
                head_terms.push(var_term.clone());

                // --- INJECTION DES TYPES ---
                // On récupère le skeleton ID correspondant au type du paramètre
                // param.ty() est un vect<TypeId> qui indexe directement self.type_to_skeleton
                //Grâce à ton module de flattening, param.ty() est maintenant
                // garanti d'être un type "Pivot" (primitive pointant vers une liste de feuilles)
                let type_sk_id = self.type_to_skeleton[param.ty().members()[0].as_usize()];

                // On ajoute l'atome de garde : Type_Robot(?v0), etc.
                body.push(Atom::new(type_sk_id, vec![var_term]));
            }

            let head = Atom::new(action_sk_id, head_terms);

            // 4. Finaliser la règle Maîtresse
            if let Some(body_atom) = precond_opt {
                // Le corps contient maintenant : [Type_1(?v1), Type_2(?v2), ..., Precond_Aux(...)]
                body.push(body_atom);
            }

            // On trie le corps pour être sûr que les types sont traités en premier par le moteur
            // (Même si ici ils sont déjà au début, c'est une bonne sécurité)
            // Le seuil correspond aux types + prédicats du domaine.
            // Tout ce qui est au-dessus de ce nombre est un auxiliaire créé par le flattener.
            // Appel de l'optimiseur avec les deux seuils
            Self::optimize_body(&mut body, type_threshold, domain_threshold);

            self.rules.push(Rule::new(head, body));
        }

        // --- L'ÉTAPE CRUCIALE ---
        // Maintenant que TOUTES les règles (actions + auxiliaires du flattener) sont là,
        // on les trie toutes une par une selon la stratégie FD (3 niveaux).
        for rule in &mut self.rules {
            // Supposons que tu as ajouté un accesseur mutable : rule.body_mut()
            Self::optimize_body(rule.body_mut(), type_threshold, domain_threshold);
        }

        Ok(())
    }

    pub fn saturate_semi_naive(&mut self) {
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


    fn optimize_body(body: &mut Vec<Atom>, type_threshold: usize, domain_threshold: usize) {
        if body.len() <= 1 { return; }
        let mut optimized = Vec::with_capacity(body.len());
        let mut bound_vars = std::collections::HashSet::new();
        let mut remaining = std::mem::take(body);

        while !remaining.is_empty() {
            let best_idx = remaining.iter().enumerate().min_by_key(|(_, atom)| {
                let sk_id = atom.skeleton_id().as_usize();

                // 1. Calcul de la priorité (plus petit = plus tôt)
                let priority = if sk_id < type_threshold {
                    0 // TYPE : Filtre unaire ultra-rapide
                } else if sk_id < domain_threshold {
                    1 // DOMAINE : Faits réels (At, In, On)
                } else {
                    2 // AUX : Logique complexe générée (Sous-requêtes)
                };

                // 2. Calcul du nombre de termes fixés (Constantes + Variables liées)
                let bound_count = atom.terms().iter().filter(|t| match t {
                    Term::Variable(v) => bound_vars.contains(v),
                    Term::Constant(_) => true,
                }).count();

                // On trie par (Priorité, Moins de variables libres)
                // Le i32 négatif permet de prendre le "max" de bound_count avec un "min" global
                (priority, -(bound_count as i32))
            }).map(|(idx, _)| idx).unwrap();

            let best_atom = remaining.remove(best_idx);

            // Mise à jour des variables liées pour les prochains atomes
            for term in best_atom.terms() {
                if let Term::Variable(v) = term {
                    bound_vars.insert(*v);
                }
            }
            optimized.push(best_atom);
        }
        *body = optimized;
    }

}
