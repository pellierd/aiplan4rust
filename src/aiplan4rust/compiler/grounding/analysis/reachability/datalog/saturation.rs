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

#[cfg(test)]
mod tests_unifications {
    use super::*;
    use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::term::Term;
    use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
    use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
    use crate::aiplan4rust::support::lang::{ObjectId, VariableId};
    use crate::analysis::inertia::table::InertiaTable;
    use crate::analysis::reachability::datalog::core::Database;
    use rustc_hash::FxHashMap;
    // Imports additionnels si besoin pour LiftedProblem, ValueRegistry, Database, etc.

    /// Helper to create a pre-configured engine with custom segment boundaries for testing.
    fn create_segmented_engine<'a>() -> DatalogEngine<'a> {
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
            current_env: [None; crate::analysis::reachability::datalog::engine::MAX_VARS],
            trailing_indices: Vec::new(),
            discovered_facts: Vec::new(),
            head_buffer: Vec::new(),
            fluence_threshold: 2,
            type_segment_start: 4, // Negative/mirror zone from 2 to 4
            type_threshold: 7,     // 3 types (indices 4, 5, 6)
            action_threshold: 10,  // 3 actions (indices 7, 8, 9)
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

    #[test]
    fn test_unification_logic() {
        // On utilise directement notre helper propre !
        let mut engine = create_segmented_engine();

        macro_rules! reset_env {
            () => {
                engine.current_env.fill(None);
            };
        }

        let var_v0 = Term::Variable(VariableId::from(0));
        let const_c10 = Term::Constant(ObjectId::from(10));

        // --- Cas 1 : Liaison d'une nouvelle variable ---
        reset_env!();
        let terms = vec![var_v0.clone(), const_c10.clone()];
        let tuple = vec![ObjectId::from(55), ObjectId::from(10)];

        assert!(engine.unify_and_bind(&terms, &tuple));
        assert_eq!(engine.current_env[0], Some(ObjectId::from(55)));

        // --- Cas 2 : Conflit avec une variable déjà liée ---
        let tuple_conflict = vec![ObjectId::from(99), ObjectId::from(10)];
        assert!(
            !engine.unify_and_bind(&terms, &tuple_conflict),
            "Devrait échouer car v0 est déjà lié à 55"
        );

        // --- Cas 3 : Conflit avec une constante ---
        reset_env!();
        let terms_const = vec![const_c10.clone()];
        let tuple_wrong_const = vec![ObjectId::from(11)];
        assert!(
            !engine.unify_and_bind(&terms_const, &tuple_wrong_const),
            "Échec attendu : 10 != 11"
        );

        // --- Cas 4 : Même variable utilisée deux fois (Auto-unification) ---
        reset_env!();
        let terms_double = vec![var_v0.clone(), var_v0.clone()];
        let tuple_ok = vec![ObjectId::from(7), ObjectId::from(7)];
        let tuple_bad = vec![ObjectId::from(7), ObjectId::from(8)];

        assert!(
            engine.unify_and_bind(&terms_double, &tuple_ok),
            "v0 peut être 7 et 7"
        );

        reset_env!();
        assert!(
            !engine.unify_and_bind(&terms_double, &tuple_bad),
            "v0 ne peut pas être 7 ET 8"
        );
    }

    #[test]
    fn test_unification_with_existing_bindings() {
        let mut engine = create_segmented_engine();
        macro_rules! reset_env {
            () => {
                engine.current_env.fill(None);
            };
        }

        let var_r = Term::Variable(VariableId::from(0)); // ?robot
        let var_l = Term::Variable(VariableId::from(1)); // ?loc
        let id_robot1 = ObjectId::from(100);
        let id_room_a = ObjectId::from(1);

        // --- Scénario : Jointure ---
        reset_env!();

        // 1. Première étape : on lie ?robot=100 et ?loc=1
        let terms1 = vec![var_r.clone(), var_l.clone()];
        let tuple1 = vec![id_robot1, id_room_a];
        assert!(engine.unify_and_bind(&terms1, &tuple1));

        // 2. Deuxième étape : on vérifie un autre atome qui réutilise ?robot
        let id_fuel_50 = ObjectId::from(50);
        let terms2 = vec![var_r.clone(), Term::Variable(VariableId::from(2))]; // [?robot, ?level]

        // Ce tuple devrait échouer car le robot à l'index 0 est '200', pas '100'
        let tuple_wrong_robot = vec![ObjectId::from(200), id_fuel_50];
        assert!(
            !engine.unify_and_bind(&terms2, &tuple_wrong_robot),
            "Devrait échouer : le robot lié est le 100"
        );

        // Ce tuple devrait réussir et lier ?level (index 2) à 50
        let tuple_ok = vec![id_robot1, id_fuel_50];
        assert!(engine.unify_and_bind(&terms2, &tuple_ok));
        assert_eq!(
            engine.current_env[2],
            Some(id_fuel_50),
            "La variable ?level aurait dû être liée à 50"
        );
    }

    #[test]
    fn test_unification_self_constraint() {
        let mut engine = create_segmented_engine();
        macro_rules! reset_env {
            () => {
                engine.current_env.fill(None);
            };
        }

        // On utilise la même variable deux fois : [?v0, ?v0]
        let var_v0 = Term::Variable(VariableId::from(0));
        let terms = vec![var_v0.clone(), var_v0.clone()];

        // Cas A : Les valeurs sont identiques -> Succès
        reset_env!();
        let tuple_ok = vec![ObjectId::from(7), ObjectId::from(7)];
        assert!(
            engine.unify_and_bind(&terms, &tuple_ok),
            "v0 peut être lié à 7 car 7 == 7"
        );
        assert_eq!(engine.current_env[0], Some(ObjectId::from(7)));

        // Cas B : Les valeurs sont différentes -> Échec
        reset_env!();
        let tuple_bad = vec![ObjectId::from(7), ObjectId::from(8)];
        assert!(
            !engine.unify_and_bind(&terms, &tuple_bad),
            "Doit échouer car v0 ne peut pas être 7 ET 8 en même temps"
        );
    }

    #[test]
    fn test_unification_rollback_on_failure() {
        let mut engine = create_segmented_engine();
        macro_rules! reset_env {
            () => {
                engine.current_env.fill(None);
            };
        }

        let var_v0 = Term::Variable(VariableId::from(0));
        let const_99 = Term::Constant(ObjectId::from(99));

        reset_env!();

        // On tente d'unifier [?v0, 99] avec [10, 88]
        // La liaison ?v0 = 10 va réussir, mais la constante 99 != 88 va faire échouer l'atome.
        let terms = vec![var_v0.clone(), const_99];
        let tuple = vec![ObjectId::from(10), ObjectId::from(88)];

        let success = engine.unify_and_bind(&terms, &tuple);

        assert!(!success);
        // CRUCIAL : ?v0 ne doit pas être resté lié à 10 !
        assert_eq!(
            engine.current_env[0], None,
            "L'environnement doit être propre après un échec d'unification"
        );
    }

    #[test]
    fn test_unification_triple_variable_constraint() {
        let mut engine = create_segmented_engine();
        let var_x = Term::Variable(VariableId::from(0));
        // Atome : P(?v0, ?v0, ?v0)
        let atom_terms = vec![var_x.clone(), var_x.clone(), var_x.clone()];

        // Cas échec : les trois ne sont pas identiques
        let tuple_fail = vec![ObjectId::from(10), ObjectId::from(10), ObjectId::from(20)];
        assert!(!engine.unify_and_bind(&atom_terms, &tuple_fail));
        assert_eq!(engine.current_env[0], None, "Doit avoir rollback");

        // Cas succès : les trois sont identiques
        let tuple_success = vec![ObjectId::from(30), ObjectId::from(30), ObjectId::from(30)];
        assert!(engine.unify_and_bind(&atom_terms, &tuple_success));
        assert_eq!(engine.current_env[0], Some(ObjectId::from(30)));
    }

    #[test]
    fn test_unification_partial_rollback() {
        let mut engine = create_segmented_engine();
        let var_x = Term::Variable(VariableId::from(0));
        let var_y = Term::Variable(VariableId::from(1));

        // 1. On simule que ?v0 est déjà lié (par un atome précédent dans une règle)
        engine.current_env[0] = Some(ObjectId::from(50));
        // On ajoute manuellement à la pile de trail pour simuler un état propre
        engine.trailing_indices.push(0);

        // 2. On tente d'unifier un nouvel atome Q(?v0, ?v1) avec un tuple incompatible sur ?v0
        let atom_terms = vec![var_x, var_y];
        let tuple = vec![ObjectId::from(99), ObjectId::from(100)];

        let _savepoint = engine.trailing_indices.len(); // Devrait être 1
        assert!(!engine.unify_and_bind(&atom_terms, &tuple));

        // 3. VERIFICATION :
        // ?v1 ne doit pas être lié (échec de l'atome)
        assert_eq!(engine.current_env[1], None);
        // ?v0 doit être TOUJOURS lié à 50 (il ne doit pas avoir été "rollbacké" par erreur)
        assert_eq!(
            engine.current_env[0],
            Some(ObjectId::from(50)),
            "Le rollback a effacé une variable parente !"
        );
    }

    #[test]
    fn test_unification_empty_atom() {
        let mut engine = create_segmented_engine();
        let atom_terms: Vec<Term> = vec![];
        let tuple: Vec<ObjectId> = vec![];

        // Une unification sans termes est trivialement vraie
        assert!(engine.unify_and_bind(&atom_terms, &tuple));
    }

    #[test]
    fn test_unification_full_cleanup() {
        let mut engine = create_segmented_engine();
        let var_x = Term::Variable(VariableId::from(0));

        // On lie ?v0
        engine.unify_and_bind(&[var_x], &[ObjectId::from(100)]);
        assert!(engine.current_env[0].is_some());

        // On simule le reset de fin de règle
        engine.current_env.fill(None);
        engine.trailing_indices.clear();

        assert!(engine.current_env[0].is_none());
        assert!(engine.trailing_indices.is_empty());
    }

    #[test]
    fn test_unification_at_limit_64() {
        let mut engine = create_segmented_engine();

        // 1. Création d'un atome avec exactement 64 variables : ?v0, ?v1, ..., ?v63
        let atom_terms: Vec<Term> = (0..MAX_VARS)
            .map(|i| Term::Variable(VariableId::from(i)))
            .collect();

        // 2. Création d'un tuple avec 64 valeurs distinctes
        let tuple: Vec<ObjectId> = (0..MAX_VARS).map(|i| ObjectId::from(i)).collect();

        // 3. L'unification doit réussir sans paniquer
        assert!(engine.unify_and_bind(&atom_terms, &tuple));

        // 4. Vérification de la première, d'une intermédiaire et de la toute dernière liaison
        assert_eq!(engine.current_env[0], Some(ObjectId::from(0)));
        assert_eq!(engine.current_env[32], Some(ObjectId::from(32)));
        assert_eq!(engine.current_env[MAX_VARS - 1], Some(ObjectId::from(63)));

        // 5. Test du rollback complet sur 64 variables
        let split = 0;
        engine.undo_to_savepoint(split);
        assert!(engine.current_env[0].is_none());
        assert!(engine.current_env[MAX_VARS - 1].is_none());
    }

    #[test]
    fn test_unification_cross_variable_consistency() {
        let mut engine = create_segmented_engine();
        let var_x = Term::Variable(VariableId::from(0));
        let var_y = Term::Variable(VariableId::from(1));
        let val_100 = ObjectId::from(100);

        // 1. On lie ?v0 à 100
        assert!(engine.unify_and_bind(&[var_x.clone()], &[val_100]));

        // 2. On essaie d'unifier ?v1 avec la MEME valeur 100
        assert!(engine.unify_and_bind(&[var_y.clone()], &[val_100]));

        assert_eq!(engine.current_env[0], Some(val_100));
        assert_eq!(engine.current_env[1], Some(val_100));
    }
}
