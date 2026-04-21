use crate::aiplan4rust::lang::{CompareOp, TaskLabelSymbolId, TaskSkeletonId, TaskSymbolId};
use crate::aiplan4rust::lir::store::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    /// Crée un nœud symbole de tâche (HTN).
    pub fn task_symbol<I: Into<TaskSymbolId>>(&mut self, id: I) -> ExprId {
        let val: usize = id.into().into();
        self.intern(ExprEntryKind::TaskSymbol(TaskSymbolId::from(val)), &[])
    }

    /// Crée un identifiant de tâche (Label).
    pub fn task_label<I: Into<TaskLabelSymbolId>>(&mut self, id: I) -> ExprId {
        let val: usize = id.into().into();
        self.intern(ExprEntryKind::TaskLabel(TaskLabelSymbolId::from(val)), &[])
    }

    /// Associe un label à une tâche (LabeledTask).
    pub fn labeled_task<I: Into<TaskLabelSymbolId>>(&mut self, id: I, task_expr: ExprId) -> ExprId {
        let task_id_node = self.task_label(id);
        self.intern(ExprEntryKind::LabeledTask, &[task_id_node, task_expr])
    }

    /// Crée une contrainte d'ordonnancement (Task1 < Task2).
    /// Note : On réutilise CompareOp::Less pour la sémantique interne.
    pub fn task_ordering_constraint(&mut self, task1: ExprId, task2: ExprId) -> ExprId {
        // On stocke l'opérateur de comparaison dans le Kind pour la structure
        self.intern(
            ExprEntryKind::TaskOrderingConstraint(CompareOp::Less),
            &[task1, task2],
        )
    }

    /// Crée une instance de tâche avec son squelette (HTN).
    pub fn task_with_skeleton<TID, SID>(
        &mut self,
        sym_id: TID,
        args: &[ExprId],
        skel_id: SID,
    ) -> ExprId
    where
        TID: Into<TaskSymbolId>,
        SID: Into<TaskSkeletonId>,
    {
        let sym_node = self.task_symbol(sym_id);
        let skel = skel_id.into();

        let mut children = Vec::with_capacity(args.len() + 1);
        children.push(sym_node);
        children.extend_from_slice(args);

        self.intern(ExprEntryKind::Task(skel), &children)
    }
}
