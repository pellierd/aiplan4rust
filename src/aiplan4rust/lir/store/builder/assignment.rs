use crate::aiplan4rust::lang::AssignOp;
use crate::aiplan4rust::lir::store::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    /// Crée un nœud d'assignation fonctionnelle : (op target value)
    /// L'opération (assign, increase, decrease, etc.) est stockée dans le Kind.
    pub fn assign_expr(&mut self, op: AssignOp, target: ExprId, value: ExprId) -> ExprId {
        // Tentative de simplification immédiate (Trivial Assignment)
        if let Some(node) = self.get(value) {
            if let ExprEntryKind::Number(n) = node.kind() {
                let val = n.into_inner();

                let is_trivial = match op {
                    // increase/decrease de 0 ne change rien
                    AssignOp::Increase | AssignOp::Decrease => val == 0.0,
                    // scale-up/scale-down par 1 ne change rien
                    AssignOp::ScaleUp | AssignOp::ScaleDown => val == 1.0,
                    // Un assign f = x n'est jamais trivial même si x=0
                    AssignOp::Assign => false,
                };

                if is_trivial {
                    // Au lieu de créer un nœud Assign, on renvoie "True" (And vide)
                    return self.empty_and();
                }
            }
        }

        // Sinon, on procède à l'internement normal
        self.intern(ExprEntryKind::Assignment(op), &[target, value])
    }

    /// (assign target value)
    pub fn assign(&mut self, target: ExprId, value: ExprId) -> ExprId {
        self.assign_expr(AssignOp::Assign, target, value)
    }

    /// (increase target value)
    pub fn increase(&mut self, target: ExprId, value: ExprId) -> ExprId {
        self.assign_expr(AssignOp::Increase, target, value)
    }

    /// (decrease target value)
    pub fn decrease(&mut self, target: ExprId, value: ExprId) -> ExprId {
        self.assign_expr(AssignOp::Decrease, target, value)
    }

    /// (scale-up target value)
    pub fn scale_up(&mut self, target: ExprId, value: ExprId) -> ExprId {
        self.assign_expr(AssignOp::ScaleUp, target, value)
    }

    /// (scale-down target value)
    pub fn scale_down(&mut self, target: ExprId, value: ExprId) -> ExprId {
        self.assign_expr(AssignOp::ScaleDown, target, value)
    }
}
