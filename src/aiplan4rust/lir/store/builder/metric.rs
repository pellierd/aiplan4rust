use crate::aiplan4rust::lang::OptimizationOp;
use crate::aiplan4rust::lir::store::builder::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    /// Crée une expression de métrique (optimisation).
    fn metric_exp(&mut self, opt: OptimizationOp, expr: ExprId) -> ExprId {
        self.intern(ExprEntryKind::Metric(opt), &[expr])
    }

    pub fn minimize(&mut self, expr: ExprId) -> ExprId {
        self.metric_exp(OptimizationOp::Minimize, expr)
    }

    pub fn maximize(&mut self, expr: ExprId) -> ExprId {
        self.metric_exp(OptimizationOp::Maximize, expr)
    }

    /// Représente la variable `total-time` (makespan).
    pub fn total_time(&mut self) -> ExprId {
        self.intern(ExprEntryKind::TotalTime, &[])
    }

    // --- Plan Length / Structural Constraints ---

    pub fn length(&mut self, serial: Option<f64>, parallel: Option<f64>) -> ExprId {
        let mut buf = [ExprId::default(); 2];
        let mut count = 0;

        if let Some(s) = serial {
            buf[count] = self.serial(s);
            count += 1;
        }
        if let Some(p) = parallel {
            buf[count] = self.parallel(p);
            count += 1;
        }

        self.intern(ExprEntryKind::Length, &buf[..count])
    }

    pub fn serial(&mut self, value: f64) -> ExprId {
        let number = self.number(value);
        self.intern(ExprEntryKind::Serial, &[number])
    }

    pub fn parallel(&mut self, value: f64) -> ExprId {
        let number = self.number(value);
        self.intern(ExprEntryKind::Parallel, &[number])
    }
}
