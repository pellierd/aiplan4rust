use crate::aiplan4rust::lir::store::{ExprEntry, ExprId, ExprNodeRef, ExprStore};
use fxhash::FxHashSet;

/// Un itérateur postorder pour les expressions dans le `ExprStore`.
///
/// Produit `(ExprId, depth, &ExprEntry)` pour chaque nœud unique (traversée de DAG).
pub struct PostorderIter<'a> {
    store: &'a ExprStore,
    stack: Vec<(ExprId, usize, bool)>, // (ID, depth, children_pushed_flag)
    visited: FxHashSet<ExprId>,        // Pour garantir qu'on ne visite chaque ID unique qu'une fois
}

impl<'a> PostorderIter<'a> {
    /// Crée un nouvel itérateur postorder à partir d'une racine donnée.
    pub fn new(store: &'a ExprStore, root: ExprId) -> Self {
        let mut stack = Vec::with_capacity(16); // Petite réserve initiale
        stack.push((root, 0, false));
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

    /// LE CHANGEMENT : with_depth renvoie maintenant un ExprRef avec la profondeur.
    /// C'est beaucoup plus puissant car l'utilisateur a accès à tout via le Ref.
    pub fn with_depth(self) -> impl Iterator<Item = (usize, ExprNodeRef<'a>)> + 'a {
        self.map(|(id, depth, entry)| (depth, ExprNodeRef::new(id, entry)))
    }
}

impl<'a> Iterator for PostorderIter<'a> {
    type Item = (ExprId, usize, &'a ExprEntry);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((id, depth, children_pushed)) = self.stack.pop() {
            if children_pushed {
                // Utilisation de l'indexation directe (&self.store[id])
                // pour obtenir une &'a ExprEntry au lieu du wrapper ExprRef
                let entry = &self.store[id];
                return Some((id, depth, entry));
            } else {
                if self.visited.insert(id) {
                    self.stack.push((id, depth, true));

                    // Ici aussi, on utilise l'indexation pour accéder aux enfants
                    let entry = &self.store[id];
                    for &child_id in entry.children().iter().rev() {
                        self.stack.push((child_id, depth + 1, false));
                    }
                }
            }
        }
        None
    }
}
