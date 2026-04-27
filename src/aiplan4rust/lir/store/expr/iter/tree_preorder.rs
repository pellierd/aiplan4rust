use crate::aiplan4rust::lir::store::expr::{ExprEntry, ExprId, ExprStore};

/// Un itérateur pour le rendu visuel qui déplie le DAG en arbre.
/// Contrairement à PreorderIter, il visite les nœuds partagés à chaque occurrence.
pub struct TreePreorderIter<'a> {
    store: &'a ExprStore,
    // Pile de (ID, depth, is_last)
    stack: Vec<(ExprId, usize, bool)>,
}

impl<'a> TreePreorderIter<'a> {
    pub fn new(store: &'a ExprStore, root: ExprId) -> Self {
        let mut stack = Vec::with_capacity(16);
        // La racine est considérée comme "dernière" de son propre niveau (profondeur 0)
        stack.push((root, 0, true));
        Self { store, stack }
    }
}

impl<'a> Iterator for TreePreorderIter<'a> {
    /// Retourne (ID, profondeur, est_le_dernier_enfant, référence_entrée)
    type Item = (ExprId, usize, bool, &'a ExprEntry);

    fn next(&mut self) -> Option<Self::Item> {
        let (id, depth, is_last) = self.stack.pop()?;

        // On récupère l'entrée via l'indexation directe du store
        let entry = &self.store[id];
        let children = entry.children();

        // On empile les enfants à l'envers (pour sortir gauche -> droite)
        // On marque le dernier enfant (index n-1) pour le renderer
        for (i, &child_id) in children.iter().enumerate().rev() {
            let child_is_last = i == children.len() - 1;
            self.stack.push((child_id, depth + 1, child_is_last));
        }

        Some((id, depth, is_last, entry))
    }
}
