use crate::aiplan4rust::compiler::lir::expr::ExprId;
// Il te suffit d'ajouter la dépendance `fxhash` dans ton Cargo.toml
use fxhash::FxHashMap;

#[derive(Default)]
pub struct PnfScratchpad {
    pub(in crate::aiplan4rust) stack: Vec<(ExprId, bool, bool)>,
    /// Remplacement par FxHashMap pour un hachage d'entiers ultra-rapide
    pub(in crate::aiplan4rust) cache: FxHashMap<ExprId, ExprId>,
    pub(in crate::aiplan4rust) children_buffer: Vec<ExprId>,
}

impl PnfScratchpad {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(32),
            cache: FxHashMap::with_capacity_and_hasher(64, Default::default()),
            children_buffer: Vec::with_capacity(8),
        }
    }

    pub fn clear(&mut self) {
        self.stack.clear();
        self.cache.clear(); // Conserve la capacité allouée sans réallouer
        self.children_buffer.clear();
    }
}
