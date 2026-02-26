use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use crate::aiplan4rust::lang::ObjectId;

#[derive(Debug, Clone)]
pub struct Relation {
    arity: usize,
    /// Stockage à plat : [arg1, arg2, arg1, arg2, ...]
    /// Idéal pour les itérateurs de jointure (localité du cache).
    tuples: Vec<ObjectId>,
    /// Index de présence pour le point fixe (évite les doublons).
    /// On stocke un Hash (u64) pour être très compact.
    index: HashSet<u64>,
}

impl Relation {
    /// Initialise une nouvelle relation avec une arité fixe.
    pub fn new(arity: usize) -> Self {
        Self {
            arity,
            tuples: Vec::new(),
            index: HashSet::new(),
        }
    }

    /// Ajoute un fait (tuple d'ObjectIDs).
    /// Retourne `true` si le fait est nouveau, `false` s'il existait déjà.
    pub fn add_fact(&mut self, tuple: &[ObjectId]) -> bool {
        debug_assert_eq!(tuple.len(), self.arity, "L'arité du tuple ne correspond pas à la relation");

        // 1. Calculer un hash rapide du tuple
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        tuple.hash(&mut hasher);
        let h = hasher.finish();

        // 2. Vérifier si on l'a déjà (O(1)) via le set de hashes
        if self.index.insert(h) {
            // 3. Si nouveau, on l'ajoute au stockage à plat
            self.tuples.extend_from_slice(tuple);
            true
        } else {
            false
        }
    }

    /// Permet de consulter l'index des hashes (lecture seule)
    pub fn index(&self) -> &HashSet<u64> {
        &self.index
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

    pub fn contains_tuple(&self, tuple: &[ObjectId]) -> bool {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        tuple.hash(&mut hasher);
        self.index.contains(&hasher.finish())
    }
}
