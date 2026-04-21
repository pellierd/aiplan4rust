use crate::aiplan4rust::lang::PreferenceSymbolId;
use crate::aiplan4rust::lir::store::builder::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    /// Crée un nœud de nom de préférence.
    pub fn pref_name<I: Into<PreferenceSymbolId>>(&mut self, id: I) -> ExprId {
        let val: usize = id.into().into();
        self.intern(ExprEntryKind::PrefName(PreferenceSymbolId::from(val)), &[])
    }

    /// Crée un nœud `Preference` : (preference name body).
    pub fn preference<I: Into<PreferenceSymbolId>>(&mut self, id: I, body: ExprId) -> ExprId {
        let pref_symbol_node = self.pref_name(id);
        // Utilisation directe de intern pour un nœud binaire
        self.intern(ExprEntryKind::Preference, &[pref_symbol_node, body])
    }

    /// Vérifie si une préférence est violée : (is-violated name).
    pub fn is_violated<I: Into<PreferenceSymbolId>>(&mut self, id: I) -> ExprId {
        let pref_node = self.pref_name(id);
        // Un IsViolated doit toujours avoir son PreferenceSymbolId en enfant.
        self.intern(ExprEntryKind::IsViolated, &[pref_node])
    }
}
