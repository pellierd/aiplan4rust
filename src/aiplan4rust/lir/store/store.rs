use crate::aiplan4rust::lir::store::error::StorerError;
use crate::aiplan4rust::lir::store::iter::postorder::PostorderIter;
use crate::aiplan4rust::lir::store::iter::preorder::PreorderIter;
use crate::aiplan4rust::lir::store::{ExprEntry, ExprEntryKind, ExprId, ExprNodeRef};
use fxhash::FxBuildHasher;
use hashbrown::HashMap;
use serde::{Deserialize, Deserializer, Serialize};
use std::hash::{Hash, Hasher};

/// Structure de recherche temporaire pour le Zero-Copy.
/// Elle permet de chercher dans la HashMap avec des références sans allouer de Vec.
#[derive(Hash, PartialEq, Eq)]
struct ExprLookup<'a> {
    kind: &'a ExprEntryKind,
    children: &'a [ExprId],
}

/// On indique à hashbrown comment comparer notre structure temporaire
/// avec les entrées stockées.
impl<'a> hashbrown::Equivalent<ExprEntry> for ExprLookup<'a> {
    fn equivalent(&self, key: &ExprEntry) -> bool {
        key.kind() == self.kind && key.children() == self.children
    }
}

#[derive(Default, Serialize)]
pub struct ExprStore {
    /// Stockage contigu des données (Arène)
    entries: Vec<ExprEntry>,

    /// Index de recherche pour le Hash-Consing.
    /// Utilise FxHash pour des performances maximales sur les petits types.
    #[serde(skip)]
    lookup: HashMap<ExprEntry, ExprId, FxBuildHasher>,
}

impl ExprStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// La méthode centrale : récupère l'ID existant ou crée une nouvelle entrée.
    /// Garanti Zero-Allocation si l'expression existe déjà.
    pub fn intern(&mut self, kind: ExprEntryKind, children: &[ExprId]) -> ExprId {
        // RECHERCHE ZERO-COPY (Directe et rapide)
        let query = ExprLookup {
            kind: &kind,
            children,
        };

        if let Some(&id) = self.lookup.get(&query) {
            return id;
        }

        // CRÉATION
        let id = ExprId::new(self.entries.len());
        let entry = ExprEntry::new(kind, children.to_vec());

        self.entries.push(entry.clone());
        self.lookup.insert(entry, id);

        id
    }

    pub fn get(&self, id: ExprId) -> Option<ExprNodeRef<'_>> {
        self.entries
            .get(id.as_usize())
            .map(|entry| ExprNodeRef::new(id, entry))
    }

    pub fn fetch(&self, id: ExprId) -> Result<ExprNodeRef<'_>, StorerError> {
        self.get(id).ok_or_else(|| StorerError::expr_not_found(id))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn preorder(&self, root: ExprId) -> PreorderIter<'_> {
        PreorderIter::new(self, root)
    }

    pub fn postorder(&self, root: ExprId) -> PostorderIter<'_> {
        PostorderIter::new(self, root)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.lookup.clear();
    }

    /// Reconstruit l'index de recherche après désérialisation.
    fn rebuild_cache(&mut self) {
        self.lookup.clear();
        self.lookup.reserve(self.entries.len());
        for (index, entry) in self.entries.iter().enumerate() {
            self.lookup.insert(entry.clone(), ExprId::new(index));
        }
    }
}

impl std::ops::Index<ExprId> for ExprStore {
    type Output = ExprEntry;

    fn index(&self, id: ExprId) -> &Self::Output {
        &self.entries[id.as_usize()]
    }
}

impl<'de> Deserialize<'de> for ExprStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ExprStoreData {
            entries: Vec<ExprEntry>,
        }

        let data = ExprStoreData::deserialize(deserializer)?;

        let mut store = Self {
            entries: data.entries,
            lookup: HashMap::with_hasher(FxBuildHasher::default()),
        };

        store.rebuild_cache();

        Ok(store)
    }
}
