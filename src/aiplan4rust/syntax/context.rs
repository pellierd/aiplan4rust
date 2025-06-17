use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Context {
    string_pool: Vec<String>,
    string_index: HashMap<String, usize>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            string_pool: Vec::new(),
            string_index: HashMap::new(),
        }
    }

    /// Ajoute une chaîne au pool si elle n'existe pas déjà, et retourne son index
    pub fn intern(&mut self, s: String) -> usize {
        if let Some(&idx) = self.string_index.get(&s) {
            return idx;
        }

        let idx = self.string_pool.len();
        self.string_index.insert(s.clone(), idx); // clone pour l'index
        self.string_pool.push(s);                // on garde l'original ici
        idx
    }

    /// Récupère la chaîne à partir de son index
    pub fn get_str(&self, idx: usize) -> Option<&str> {
        self.string_pool.get(idx).map(|s| s.as_str())
    }

    /// Accès direct au pool (utile pour affichage/debug)
    pub fn all_strings(&self) -> &[String] {
        &self.string_pool
    }
}
