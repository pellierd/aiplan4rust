use crate::aiplan4rust::lir::expr::{ExprEntry, ExprId, ExprNodeRef, ExprStore};
use fxhash::FxHashSet;

/// Un itérateur preorder pour les expressions dans le `ExprStore`.
///
/// Produit `(ExprId, depth, &ExprEntry)` pour chaque nœud unique (traversée de DAG).
pub struct PreorderIter<'a> {
    store: &'a ExprStore,
    stack: Vec<(ExprId, usize)>, // (ID, depth)
    visited: FxHashSet<ExprId>,  // Pour garantir qu'on ne visite chaque ID unique qu'une fois
}

impl<'a> PreorderIter<'a> {
    /// Crée un nouvel itérateur preorder à partir d'une racine donnée.
    pub fn new(store: &'a ExprStore, root: ExprId) -> Self {
        let mut stack = Vec::with_capacity(16);
        stack.push((root, 0));
        Self {
            store,
            stack,
            visited: FxHashSet::default(),
        }
    }

    /// Transforme l'itérateur pour ne produire que des `ExprRef`.
    pub fn references(self) -> impl Iterator<Item = ExprNodeRef<'a>> + 'a {
        self.map(|(id, _, entry)| ExprNodeRef::new(id, entry))
    }

    /// Renvoie un ExprRef avec la profondeur.
    pub fn with_depth(self) -> impl Iterator<Item = (usize, ExprNodeRef<'a>)> + 'a {
        self.map(|(id, depth, entry)| (depth, ExprNodeRef::new(id, entry)))
    }
}

impl<'a> Iterator for PreorderIter<'a> {
    type Item = (ExprId, usize, &'a ExprEntry);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((id, depth)) = self.stack.pop() {
            // Dans un DAG (Hash-Consing), on ne visite chaque nœud unique qu'une seule fois
            if self.visited.insert(id) {
                // Utilisation de l'indexation directe via l'impl Index du store
                // Cela garantit une référence &'a ExprEntry
                let entry = &self.store[id];

                // On ajoute les enfants sur la pile en sens inverse pour
                // maintenir l'ordre de lecture gauche -> droite
                for &child_id in entry.children().iter().rev() {
                    self.stack.push((child_id, depth + 1));
                }

                // En Preorder, on retourne le nœud dès qu'on le rencontre
                return Some((id, depth, entry));
            }
        }
        None
    }
}
