use crate::aiplan4rust::compiler::lir::expr::ExprId;
use crate::aiplan4rust::support::lang::typed_list::OPTIMAL_LIST_CAPACITY;
use crate::aiplan4rust::support::lang::{TypeId, TypedList, VariableId};
use fxhash::FxHashMap;

pub struct ExpansionScratchpad {
    pub(in crate::aiplan4rust) cache: FxHashMap<ExprId, ExprId>,
    pub(in crate::aiplan4rust) children_buffer: Vec<ExprId>,
    /// Pile explicite pour le parcours DFS Post-Order : (ID, enfants_empilés)
    pub(in crate::aiplan4rust) stack: Vec<(ExprId, bool)>,
    pub(in crate::aiplan4rust) variables: TypedList<VariableId, TypeId>,
}

impl ExpansionScratchpad {
    pub fn new() -> Self {
        Self {
            cache: FxHashMap::with_capacity_and_hasher(32, Default::default()),
            children_buffer: Vec::with_capacity(8),
            stack: Vec::with_capacity(32),
            variables: TypedList::with_capacity(OPTIMAL_LIST_CAPACITY),
        }
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.children_buffer.clear();
        self.stack.clear();
        self.variables.clear();
    }
}
