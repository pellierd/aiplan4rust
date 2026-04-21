// --- Trajectory Constraints (PDDL 3.0) ---

use crate::aiplan4rust::lir::store::builder::ExprBuilder;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId};

impl<'a> ExprBuilder<'a> {
    // --- Opérateurs Unaires ---
    pub fn always(&mut self, expr: ExprId) -> ExprId {
        self.intern(ExprEntryKind::Always, &[expr])
    }

    pub fn sometime(&mut self, expr: ExprId) -> ExprId {
        self.intern(ExprEntryKind::Sometime, &[expr])
    }

    pub fn at_most_once(&mut self, expr: ExprId) -> ExprId {
        self.intern(ExprEntryKind::AtMostOnce, &[expr])
    }

    // --- Opérateurs Binaires ---

    pub fn sometime_after(&mut self, first: ExprId, second: ExprId) -> ExprId {
        self.intern(ExprEntryKind::SometimeAfter, &[first, second])
    }

    pub fn sometime_before(&mut self, first: ExprId, second: ExprId) -> ExprId {
        self.intern(ExprEntryKind::SometimeBefore, &[first, second])
    }

    // --- Opérateurs avec Durées (N-aires) ---

    pub fn within(&mut self, value: f64, expr: ExprId) -> ExprId {
        let duration_node = self.number(value);
        self.intern(ExprEntryKind::Within, &[duration_node, expr])
    }

    pub fn always_within(&mut self, duration: f64, first: ExprId, second: ExprId) -> ExprId {
        let number_node = self.number(duration);
        // On utilise intern pour garantir que les 3 enfants restent présents
        // même si l'un d'eux est un élément logiquement "neutre".
        self.intern(ExprEntryKind::AlwaysWithin, &[number_node, first, second])
    }

    pub fn hold_during(&mut self, start: f64, end: f64, expr: ExprId) -> ExprId {
        let start_node = self.number(start);
        let end_node = self.number(end);
        // On utilise intern au lieu de nary pour protéger les arguments
        self.intern(ExprEntryKind::HoldDuring, &[start_node, end_node, expr])
    }

    pub fn hold_after(&mut self, time: f64, expr: ExprId) -> ExprId {
        let time_node = self.number(time);
        // Utilisation directe de intern car l'arité est fixe (2)
        self.intern(ExprEntryKind::HoldAfter, &[time_node, expr])
    }
}
