// On suppose que ton itérateur est défini dans le module store
use crate::aiplan4rust::lir::store::iter::postorder::PostorderIter;
use crate::aiplan4rust::lir::store::iter::preorder::PreorderIter;
use crate::aiplan4rust::lir::store::{ExprId, ExprNodeRef, ExprStore};

/// Un "Handle" (poignée) vers une expression complète stockée dans un `ExprStore`.
///
/// `Expr` lie un identifiant de racine (`ExprId`) à son magasin de stockage.
/// C'est l'objet principal que tu passeras à tes fonctions métier (taxes,
/// évaluation, etc.) car il permet de parcourir l'arbre tout en restant
/// très léger en mémoire.
#[derive(Clone, Copy)]
pub struct Expr<'a> {
    root: ExprId,
    store: &'a ExprStore,
}

impl<'a> Expr<'a> {
    /// Crée une nouvelle poignée d'expression.
    pub fn new(root: ExprId, store: &'a ExprStore) -> Self {
        Self { root, store }
    }

    /// Retourne l'identifiant de la racine dans le store.
    pub fn root_id(&self) -> ExprId {
        self.root
    }

    /// Retourne une référence vers le store associé.
    pub fn store(&self) -> &'a ExprStore {
        self.store
    }

    /// Accède directement au nœud racine sous forme de `ExprNodeRef`.
    pub fn root_node(&self) -> Option<ExprNodeRef<'_>> {
        // On suppose que ton store a une méthode get(id) qui renvoie un ExprNodeRef
        self.store.get(self.root)
    }

    /// Crée un itérateur pour parcourir l'expression en ordre "Postorder".
    ///
    /// C'est cette méthode que tu utiliseras pour tes calculs de taxes :
    /// elle remontera des feuilles vers la racine.
    pub fn postorder(&self) -> PostorderIter<'a> {
        self.store.postorder(self.root)
    }

    /// Crée un itérateur pour parcourir l'expression en ordre "Preorder".
    ///
    /// Ordre : Parent -> Enfants.
    /// Utile pour : Affichage (debug), recherche de motifs, vérifications syntaxiques descendantes.
    pub fn preorder(&self) -> PreorderIter<'a> {
        self.store.preorder(self.root)
    }
}

// Implémentation facultative pour faciliter le debug
impl<'a> std::fmt::Debug for Expr<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Expr").field("root_id", &self.root).finish()
    }
}
