///////////////////////////////////////////////////////////////////////////////////////////////
// FONCTIONS FOR RUNNING THE ENGINE
///////////////////////////////////////////////////////////////////////////////////////////////

use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::rule::Rule;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::term::Term;
use crate::aiplan4rust::compiler::grounding::binding::iter::BindingsIterator;
use crate::aiplan4rust::support::lang::{AtomSkeletonId, ObjectId};
use crate::analysis::reachability::datalog::core::rule::RuleBody;
use crate::analysis::reachability::datalog::engine::MAX_VARS;
use crate::analysis::reachability::datalog::error::DatalogError;
use crate::DatalogEngine;

impl<'a> DatalogEngine<'a> {
    /// Lance le calcul de l'atteignabilité (Interface publique)

    pub(crate) fn materialize_negations(&mut self) -> Result<(), DatalogError> {
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
            .filter(|r| r.head().symbol() == target_id)
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
                let sub_id = atom.symbol();
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
                            atom.arguments(),
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
                            atom.arguments()
                        );
                    }
                }
            }
        }
    }

    pub(crate) fn saturate_semi_naive(&mut self) {
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
        let sk_id = atom.symbol();

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
        let sk_id = atom.symbol(); // C'est un AtomSkeletonId

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

            for term in atom.arguments() {
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
        let terms = atom.arguments();
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
        let terms = atom.arguments();

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
            if let Some(offsets_slice) = self.db.lookup_index(sk_id, use_delta, obj_id) {
                let offsets_cloned = offsets_slice.to_vec();
                for start in offsets_cloned {
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

        if self.unify_and_bind(rule.body()[body_idx].arguments(), &buffer[..arity]) {
            self.process_incremental(rule, body_idx + 1, pivot_idx);
        }

        // ROLLBACK CHIRURGICAL :
        // On n'annule que ce que unify_and_bind a ajouté,
        // sans toucher aux variables liées par les atomes précédents de la règle.
        self.undo_to_savepoint(trail_split);
    }

    fn evaluate_head(&mut self, rule: &Rule) -> Result<(), DatalogError> {
        let head = rule.head();
        let head_sk = head.symbol();

        // 1. On prépare le tuple de la tête dans le buffer réutilisable
        self.head_buffer.clear();

        for term in head.arguments() {
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

    /// Passe de réordonnancement globale de toutes les règles du moteur.
    pub(crate) fn optimize_all_rules(&mut self) {
        // On extrait temporairement les règles pour éviter les conflits de borrow checker avec &self
        let mut rules = std::mem::take(&mut self.rules);

        for rule in &mut rules {
            self.optimize_rule(rule.body_mut());
        }

        // On remet les règles optimisées en place
        self.rules = rules;
    }

    fn optimize_rule(&self, body: &mut RuleBody) {
        if body.len() <= 1 {
            return;
        }

        // 🚀 OPTIMISATION : On utilise directement RuleBody pour rester sur la pile
        let mut optimized = RuleBody::with_capacity(body.len());
        let mut bound_vars_mask: u64 = 0;

        // mem::take fonctionne parfaitement car RuleBody implémente Default
        let mut remaining = std::mem::take(body);

        while !remaining.is_empty() {
            let best_idx = remaining
                .iter()
                .enumerate()
                .min_by_key(|(_, atom)| {
                    let sk_id = atom.symbol();

                    // 1. Calcul des variables déjà liées
                    let mut bound_count = 0;
                    for term in atom.arguments() {
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

                    (-(bound_count as i32), rel_size, priority)
                })
                .map(|(idx, _)| idx)
                .unwrap();

            let best_atom = remaining.remove(best_idx);

            // Mise à jour du masque des variables liées
            for term in best_atom.arguments() {
                if let Term::Variable(v) = term {
                    let v_idx = v.as_usize();
                    if v_idx < 64 {
                        bound_vars_mask |= 1 << v_idx;
                    }
                }
            }
            optimized.push(best_atom);
        }

        // 🚀 Remplacement direct et transparent sans passer par la heap
        *body = optimized;
    }

    #[inline(always)]
    pub fn get_predicate_priority(&self, atom: &Atom) -> u8 {
        let id = atom.symbol();

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
}
