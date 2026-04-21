use crate::aiplan4rust::lir::store::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    pub fn at_start(&mut self, expr: ExprId) -> ExprId {
        self.intern(ExprEntryKind::AtStart, &[expr])
    }

    pub fn at_end(&mut self, expr: ExprId) -> ExprId {
        self.intern(ExprEntryKind::AtEnd, &[expr])
    }

    pub fn overall(&mut self, expr: ExprId) -> ExprId {
        self.intern(ExprEntryKind::Overall, &[expr])
    }
}
