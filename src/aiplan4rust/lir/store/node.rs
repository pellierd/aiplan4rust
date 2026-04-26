use crate::aiplan4rust::lir::store::{ExprEntry, ExprEntryKind, ExprId};

/// Une référence légère et non-propriétaire vers une expression dans le Store.
///
/// `ExprRef` combine l'identifiant unique (`ExprId`) avec une référence aux données
/// de l'expression (`&ExprEntry`). Cela permet une inspection ergonomique sans
/// avoir à interroger le store manuellement à chaque accès.
#[derive(Debug, Clone, Copy)]
pub struct ExprNodeRef<'a> {
    id: ExprId,
    entry: &'a ExprEntry,
}

impl<'a> ExprNodeRef<'a> {
    /// Crée une nouvelle référence d'expression.
    pub fn new(id: ExprId, entry: &'a ExprEntry) -> Self {
        Self { id, entry }
    }

    /// Retourne l'ID de l'expression.
    pub fn id(&self) -> ExprId {
        self.id
    }

    /// Accès direct au genre de l'expression (Kind).
    pub fn kind(&self) -> &ExprEntryKind {
        self.entry.kind()
    }

    /// Accès direct aux IDs des enfants.
    pub fn children(&self) -> &[ExprId] {
        self.entry.children()
    }

    /// Retourne l'entrée brute stockée dans le store.
    pub fn entry(&self) -> &ExprEntry {
        self.entry
    }
}
