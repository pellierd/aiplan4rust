use crate::aiplan4rust::compiler::lir::expr::{ExprEntry, ExprId, ExprKind};

/// Une référence légère et non-propriétaire vers une expression dans le Store.
///
/// `ExprRef` combine l'identifiant unique (`ExprId`) avec une référence aux données
/// de l'expression (`&ExprEntry`). Cela permet une inspection ergonomique sans
/// avoir à interroger le old manuellement à chaque accès.
#[derive(Debug, Clone, Copy)]
pub struct ExprNode<'a> {
    id: ExprId,
    entry: &'a ExprEntry,
}

impl<'a> ExprNode<'a> {
    /// Crée une nouvelle référence d'expression.
    pub fn new(id: ExprId, entry: &'a ExprEntry) -> Self {
        Self { id, entry }
    }

    /// Retourne l'ID de l'expression.
    pub fn id(&self) -> ExprId {
        self.id
    }

    /// Accès direct au genre de l'expression (Kind).
    pub fn kind(&self) -> &ExprKind {
        self.entry.kind()
    }

    /// Accès direct aux IDs des enfants.
    pub fn children(&self) -> &[ExprId] {
        self.entry.children()
    }

    /// Retourne l'entrée brute stockée dans le old.
    pub fn entry(&self) -> &ExprEntry {
        self.entry
    }
}
