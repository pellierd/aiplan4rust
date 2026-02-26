use std::collections::HashMap;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::relation::Relation;
use crate::aiplan4rust::lang::{AtomSkeletonId, ObjectId};

#[derive(Default, Debug, Clone)]
pub struct Database {
    /// Stockage des relations indexées par l'ID du squelette (signature unique).
    /// Chaque entrée correspond à une table de faits spécifique (ex: at(Robot, Location)).
    relations: HashMap<AtomSkeletonId, Relation>,
}

impl Database {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute un fait à la base de données pour un squelette donné.
    /// Crée la relation si elle n'existe pas encore.
    pub fn add_fact(&mut self, skeleton_id: AtomSkeletonId, args: &[ObjectId]) -> bool {
        let arity = args.len();
        self.relations
            .entry(skeleton_id)
            .or_insert_with(|| Relation::new(arity))
            .add_fact(args)
    }

    /// Vérifie l'existence d'un fait spécifique dans une relation donnée.
    pub fn contains(&self, skeleton_id: AtomSkeletonId, args: &[ObjectId]) -> bool {
        self.relations
            .get(&skeleton_id)
            .map_or(false, |rel| rel.contains_tuple(args))
    }

    /// Récupère une référence vers une relation pour itérer sur ses faits.
    pub fn get_relation(&self, skeleton_id: AtomSkeletonId) -> Option<&Relation> {
        self.relations.get(&skeleton_id)
    }

    /// Retourne toutes les relations de la base.
    /// Indispensable pour l'algorithme de saturation (calcul du point fixe).
    pub fn relations(&self) -> &HashMap<AtomSkeletonId, Relation> {
        &self.relations
    }
}
