use crate::aiplan4rust::support::lang::{ActionDefId, AtomSkeletonId, TypeId};
use crate::analysis::reachability::datalog::core::tuple::TupleArgs;
use crate::analysis::reachability::datalog::core::{Atom, Cause, Rule, Tuple};
use crate::DatalogEngine;

impl<'a> DatalogEngine<'a> {
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
                    let args = TupleArgs::from_slice(tuple_data);
                    fluents.push(Tuple::new(skeleton_id, args));
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
                        actions.push(Tuple::new(action_def_id, TupleArgs::new()));
                    }
                } else {
                    // Pour l'arité > 0, on itère normalement sur les arguments
                    for tuple_data in rel.iter() {
                        let args = TupleArgs::from_slice(tuple_data);
                        actions.push(Tuple::new(action_def_id, args));
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
                    let args = TupleArgs::from_slice(tuple_data);
                    axioms.push(Tuple::new(skeleton_id, args));
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
                    let args = TupleArgs::from_slice(tuple_data);
                    types.push(Tuple::new(type_id, args));
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
            .find(|r| r.head().symbol() == target_sk_id)
            .expect("Aucune règle trouvée pour cet index d'action")
    }

    pub fn get_rule_for_auxiliary(&self, sk_id: AtomSkeletonId) -> &Rule {
        debug_assert!(self.is_auxiliary(sk_id));

        self.rules
            .iter()
            .find(|r| r.head().symbol() == sk_id)
            .expect("Inconsistance : fait auxiliaire trouvé sans règle correspondante")
    }
}
