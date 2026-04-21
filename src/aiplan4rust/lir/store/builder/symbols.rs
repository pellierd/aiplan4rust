use crate::aiplan4rust::lang::{
    FunctionSymbolId, ObjectId, PredicateSymbolId, VariableId,
};
use crate::aiplan4rust::lir::store::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    /// Crée un nœud constante (objet) à partir d'un identifiant.
    pub fn constant<I: Into<ObjectId>>(&mut self, id: I) -> ExprId {
        let val: usize = id.into().into();
        self.intern(ExprEntryKind::Object(ObjectId::from(val)), &[])
    }

    /// Crée un nœud variable à partir d'un identifiant.
    pub fn variable<I: Into<VariableId>>(&mut self, id: I) -> ExprId {
        let val: usize = id.into().into();
        self.intern(ExprEntryKind::Variable(VariableId::from(val)), &[])
    }

    /// Crée un nœud symbole de fonction (functor).
    pub fn function_symbol<I: Into<FunctionSymbolId>>(&mut self, id: I) -> ExprId {
        let val: usize = id.into().into();
        self.intern(
            ExprEntryKind::FunctionSymbol(FunctionSymbolId::from(val)),
            &[],
        )
    }

    /// Crée un nœud symbole de prédicat.
    pub fn predicate<I: Into<PredicateSymbolId>>(&mut self, id: I) -> ExprId {
        let val: usize = id.into().into();
        self.intern(
            ExprEntryKind::PredicateSymbol(PredicateSymbolId::from(val)),
            &[],
        )
    }
}
