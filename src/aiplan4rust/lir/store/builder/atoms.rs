use crate::aiplan4rust::lang::{
    AtomSkeletonId, FunctionSkeletonId, FunctionSymbolId, PredicateSymbolId,
};
use crate::aiplan4rust::lir::store::builder::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    /// Crée une `AtomicFormula` (Prédicat avec arguments).
    pub fn atomic_formula<PID, SID>(&mut self, sym_id: PID, args: &[ExprId], skel_id: SID) -> ExprId
    where
        PID: Into<PredicateSymbolId>,
        SID: Into<AtomSkeletonId>,
    {
        let sym_node = self.predicate(sym_id.into());
        let skel = skel_id.into();

        let mut children = Vec::with_capacity(args.len() + 1);
        children.push(sym_node);
        children.extend_from_slice(args);

        self.intern(ExprEntryKind::AtomicFormula(skel), &children)
    }

    /// Crée un `FunctionTerm` (Fonction/Fluid avec arguments).
    pub fn function_term<FID, SID>(&mut self, sym_id: FID, args: &[ExprId], skel_id: SID) -> ExprId
    where
        FID: Into<FunctionSymbolId>,
        SID: Into<FunctionSkeletonId>,
    {
        let sym_node = self.function_symbol(sym_id.into());
        let skel = skel_id.into();

        let mut children = Vec::with_capacity(args.len() + 1);
        children.push(sym_node);
        children.extend_from_slice(args);

        self.intern(ExprEntryKind::Function(skel), &children)
    }

    /// Crée un `TimedInitialLiteral` (TIL) : une expression qui devient vraie à `time`.
    pub fn timed_initial_literal(&mut self, time: f64, expr: ExprId) -> ExprId {
        let time_node = self.number(time);
        // Utilisation directe de intern : on garantit que le temps et l'expression restent liés
        self.intern(ExprEntryKind::TimedInitialLiteral, &[time_node, expr])
    }
}
