use crate::aiplan4rust::lir::store::error::StorerError;
use crate::aiplan4rust::lir::store::iter::postorder::PostorderIter;
use crate::aiplan4rust::lir::store::iter::preorder::PreorderIter;
use crate::aiplan4rust::lir::store::{ExprEntry, ExprEntryKind, ExprId, ExprNodeRef};
use fxhash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Default, Serialize)]
pub struct ExprStore {
    /// L'arène de stockage contiguë
    entries: Vec<ExprEntry>,

    /// L'index de recherche pour le Hash-Consing
    #[serde(skip)]
    lookup: FxHashMap<ExprEntry, ExprId>,
}

impl ExprStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// La méthode centrale : récupère l'ID existant ou crée une nouvelle entrée.
    pub fn intern(&mut self, kind: ExprEntryKind, mut children: Vec<ExprId>) -> ExprId {
        if matches!(kind, ExprEntryKind::And | ExprEntryKind::Or) && children.len() > 1 {
            children.sort_unstable();
            children.dedup();
            if children.len() == 1 {
                return children[0];
            }
        }

        let entry = ExprEntry::new(kind, children);

        // 2. Hash-Consing standard
        if let Some(&id) = self.lookup.get(&entry) {
            return id;
        }

        let id = ExprId::new(self.entries.len());
        self.lookup.insert(entry.clone(), id);
        self.entries.push(entry);
        id
    }

    /// Le "Vrai" est défini comme un AND vide (élément neutre universel).
    pub fn true_expr(&mut self) -> ExprId {
        self.empty_and()
    }

    /// Le "Faux" est défini comme un OR vide (élément neutre universel).
    pub fn false_expr(&mut self) -> ExprId {
        self.empty_or()
    }

    /// Alias parfaits pour refléter ta conception :
    pub fn empty_and(&mut self) -> ExprId {
        self.intern(ExprEntryKind::And, vec![])
    }

    pub fn empty_or(&mut self) -> ExprId {
        self.intern(ExprEntryKind::Or, vec![])
    }

    /// Accès principal : combine l'ID et l'entrée dans une Ref ergonomique.
    pub fn get(&self, id: ExprId) -> Option<ExprNodeRef<'_>> {
        self.entries
            .get(id.as_usize())
            .map(|entry| ExprNodeRef::new(id, entry))
    }

    /// Version Result pour la propagation d'erreurs (opérateur ?).
    pub fn fetch(&self, id: ExprId) -> Result<ExprNodeRef<'_>, StorerError> {
        self.get(id).ok_or_else(|| StorerError::expr_not_found(id))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Crée un itérateur de parcours en profondeur d'abord (Preorder).
    /// Utile pour la recherche ou le filtrage descendant.
    pub fn preorder(&self, root: ExprId) -> PreorderIter<'_> {
        PreorderIter::new(self, root)
    }

    /// Crée un itérateur de parcours post-fixé (Postorder).
    /// Indispensable pour l'évaluation ou la compilation "bottom-up".
    pub fn postorder(&self, root: ExprId) -> PostorderIter<'_> {
        PostorderIter::new(self, root)
    }

    /// Retourne vrai si aucune expression n'a encore été internée.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Retourne le nombre d'entrées dans l'index de recherche (devrait être égal à len()).
    /// Utile pour vérifier l'intégrité du Hash-Consing.
    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.lookup.clear();
    }

    /// Reconstruit l'index de recherche (Hash-Consing) à partir des entrées du store.
    ///
    /// Cette méthode doit être appelée impérativement après la désérialisation
    /// car le champ `lookup` est ignoré par Serde pour optimiser la taille du fichier.
    fn rebuild_cache(&mut self) {
        // On s'assure que la map est vide et on pré-alloue pour éviter les copies
        self.lookup.clear();
        self.lookup.reserve(self.entries.len());

        for (index, entry) in self.entries.iter().enumerate() {
            // On insère chaque entrée avec son ID correspondant (l'index dans le Vec)
            // On utilise clone() ici car la clé de la map est une ExprEntry
            self.lookup.insert(entry.clone(), ExprId::new(index));
        }
    }
}

/// Permet d'écrire `store[id]` pour obtenir l'entrée brute.
impl std::ops::Index<ExprId> for ExprStore {
    type Output = ExprEntry;

    fn index(&self, id: ExprId) -> &Self::Output {
        // On accède directement au Vec pour pouvoir retourner une référence &ExprEntry
        &self.entries[id.as_usize()]
    }
}

impl<'de> Deserialize<'de> for ExprStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // On définit une structure intermédiaire interne pour récupérer les données sérialisées
        #[derive(Deserialize)]
        struct ExprStoreData {
            entries: Vec<ExprEntry>,
        }

        let data = ExprStoreData::deserialize(deserializer)?;

        let mut store = Self {
            entries: data.entries,
            lookup: FxHashMap::default(),
        };

        // Reconstruction automatique et transparente
        store.rebuild_cache();

        Ok(store)
    }
}
