use std::collections::HashSet;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use crate::aiplan4rust::lang::ObjectId;

#[derive(Debug, Clone)]
pub struct Relation {
    arity: usize,
    tuples: Vec<ObjectId>,
    /// On stocke des vecteurs pour la sécurité, mais on va optimiser l'ajout.
    index: HashSet<Vec<ObjectId>>,
    first_arg_index: HashMap<ObjectId, Vec<usize>>,
}
impl Relation {
    /// Initialise une nouvelle relation avec une arité fixe.
    /// Initialise une nouvelle relation avec une arité fixe.
    pub fn new(arity: usize) -> Self {
        Self {
            arity,
            tuples: Vec::new(),
            index: HashSet::new(),
            first_arg_index: HashMap::new(),
        }
    }

    pub fn contains_tuple(&self, tuple: &[ObjectId]) -> bool {
        // HashSet<Vec<T>> permet de chercher avec un &[T] grâce à l'implémentation de Borrow
        // C'est O(1), Garanti sans collision, et ZÉRO allocation.
        self.index.contains(tuple)
    }

    pub fn add_fact(&mut self, tuple: &[ObjectId]) -> bool {
        if !self.index.contains(tuple) {
            let v = tuple.to_vec();

            // Calcul de l'offset AVANT l'insertion
            let start_offset = self.tuples.len();

            // Mise à jour de l'index sur le premier argument
            if let Some(&first_obj) = tuple.first() {
                self.first_arg_index
                    .entry(first_obj)
                    .or_default()
                    .push(start_offset);
            }

            self.tuples.extend_from_slice(tuple);
            self.index.insert(v);
            return true;
        }
        false
    }

    pub fn first_arg_index(&self) -> &HashMap<ObjectId, Vec<usize>> {
        &self.first_arg_index
    }

    /// Retourne l'arité (le nombre d'arguments par fait).
    #[inline]
    pub fn arity(&self) -> usize {
        self.arity
    }

    /// Retourne le nombre total de faits (tuples) stockés.
    #[inline]
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Vérifie si la relation ne contient aucun fait.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Accès brut aux données à plat.
    pub fn raw_data(&self) -> &[ObjectId] {
        &self.tuples
    }

    /// L'itérateur principal : fournit chaque fait sous forme de slice de taille `arity`.
    pub fn iter(&self) -> std::slice::ChunksExact<'_, ObjectId> {
        self.tuples.chunks_exact(self.arity)
    }

    /// Permet de récupérer un fait spécifique par son index (de 0 à len-1).
    pub fn get_tuple(&self, index: usize) -> Option<&[ObjectId]> {
        let start = index * self.arity;
        let end = start + self.arity;
        self.tuples.get(start..end)
    }


}
