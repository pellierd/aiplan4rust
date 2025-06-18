use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringInterner {
    string_pool: Vec<Box<str>>,
    string_index: HashMap<&'static str, usize>,
}

impl StringInterner {
    /// Crée un nouveau contexte d'interning
    pub fn new() -> Self {
        Self {
            string_pool: Vec::new(),
            string_index: HashMap::new(),
        }
    }

    /// Ajoute une chaîne au pool si elle n'existe pas déjà, et retourne son index
    pub fn intern(&mut self, s: String) -> usize {
        if let Some(&idx) = self.string_index.get(s.as_str()) {
            return idx;
        }

        let boxed: Box<str> = s.into_boxed_str();
        let static_str: &'static str = Box::leak(boxed);
        let idx = self.string_pool.len();
        self.string_pool.push(static_str.into()); // On garde la mémoire vivante
        self.string_index.insert(static_str, idx);
        idx
    }

    /// Récupère la chaîne à partir de son index
    pub fn get_str(&self, idx: usize) -> Option<&str> {
        self.string_pool.get(idx).map(|s| s.as_ref())
    }

    /// Accès direct au pool (utile pour affichage/debug)
    pub fn all_strings(&self) -> &[Box<str>] {
        &self.string_pool
    }
}
