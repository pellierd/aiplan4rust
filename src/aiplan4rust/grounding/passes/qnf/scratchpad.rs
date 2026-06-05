use crate::aiplan4rust::lir::expr::ExprId;
use fxhash::FxHashMap;

pub struct ExpansionScratchpad {
    pub(super) cache: FxHashMap<ExprId, ExprId>,
    pub(super) children_buffer: Vec<ExprId>,
    /// Pile explicite pour le parcours DFS Post-Order : (ID, enfants_empilés)
    pub(super) stack: Vec<(ExprId, bool)>,
}

impl ExpansionScratchpad {
    pub fn new() -> Self {
        Self {
            cache: FxHashMap::with_capacity_and_hasher(32, Default::default()),
            children_buffer: Vec::with_capacity(8),
            stack: Vec::with_capacity(32),
        }
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.children_buffer.clear();
        self.stack.clear();
    }
}
