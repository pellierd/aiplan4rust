// On suppose que ton itérateur est défini dans le module old
use crate::aiplan4rust::compiler::lir::expr::error::StorerError;
use crate::aiplan4rust::compiler::lir::expr::iter::{PostorderIter, PreorderIter};
use crate::aiplan4rust::compiler::lir::expr::{ExprId, ExprNode, ExprStore};

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

    /// Retourne l'identifiant de la racine dans le old.
    pub fn root_id(&self) -> ExprId {
        self.root
    }

    /// Retourne une référence vers le old associé.
    pub fn store(&self) -> &'a ExprStore {
        self.store
    }

    /// Accède directement au nœud racine sous forme de `ExprNodeRef`.
    pub fn root_node(&self) -> Result<ExprNode<'a>, StorerError> {
        self.store.fetch(self.root)
    }

    /// Crée une sous-expression `Expr` à partir d'un ID d'enfant.
    #[inline]
    pub fn sub_expr(&self, sub_root_id: ExprId) -> Self {
        Self {
            root: sub_root_id,
            store: self.store,
        }
    }

    /// Récupère directement le nœud d'un enfant (pratique dans les boucles).
    pub fn fetch_node(&self, id: ExprId) -> Result<ExprNode<'a>, StorerError> {
        self.store.fetch(id)
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
