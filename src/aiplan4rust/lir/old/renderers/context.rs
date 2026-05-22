use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::{
    ActionSymbolId, FunctionSymbolId, MethodSymbolId, ObjectId, PredicateSymbolId, SymbolId,
    TaskSymbolId, TypeId,
};
use crate::aiplan4rust::lir::old::problem::{LiftedProblem, SymbolRegistry};

pub struct RenderContext<'a> {
    interner: &'a SymbolInterner,
    type_symbols: &'a SymbolRegistry<TypeId>,
    predicate_symbols: &'a SymbolRegistry<PredicateSymbolId>,
    functor_symbols: &'a SymbolRegistry<FunctionSymbolId>,
    object_symbols: &'a SymbolRegistry<ObjectId>,
    task_symbols: &'a SymbolRegistry<TaskSymbolId>,
    action_symbols: &'a SymbolRegistry<ActionSymbolId>,
    method_symbols: &'a SymbolRegistry<MethodSymbolId>,
}

impl<'a> RenderContext<'a> {
    pub fn new(problem: &'a LiftedProblem) -> Self {
        Self {
            interner: &problem.interner(),
            type_symbols: &problem.type_symbols(),
            predicate_symbols: &problem.predicate_symbols(),
            functor_symbols: &problem.function_symbols(),
            object_symbols: &problem.object_symbols(),
            task_symbols: &problem.task_symbols(),
            action_symbols: &problem.action_symbols(),
            method_symbols: &problem.method_symbols(), // Aj
        }
    }

    // --- Accesseurs directs aux tables ---
    pub fn types(&self) -> &SymbolRegistry<TypeId> {
        self.type_symbols
    }
    pub fn predicates(&self) -> &SymbolRegistry<PredicateSymbolId> {
        self.predicate_symbols
    }
    pub fn functors(&self) -> &SymbolRegistry<FunctionSymbolId> {
        self.functor_symbols
    }
    pub fn objects(&self) -> &SymbolRegistry<ObjectId> {
        self.object_symbols
    }
    pub fn task_symbols(&self) -> &SymbolRegistry<TaskSymbolId> {
        self.task_symbols
    }

    pub fn action_symbols(&self) -> &SymbolRegistry<ActionSymbolId> {
        self.action_symbols
    }

    pub fn method_symbols(&self) -> &SymbolRegistry<MethodSymbolId> {
        self.method_symbols
    }

    pub fn interner(&self) -> &SymbolInterner {
        self.interner
    }

    // --- Résolution de noms via les SymbolTables ---

    /// La base : résout un StringID brut via l'interner.
    pub fn resolve_symbol(&self, id: SymbolId) -> &str {
        self.interner.resolve_symbol(id).unwrap_or("<unknown_id>")
    }

    /// Résout un TypeID en passant par sa table, puis en utilisant resolve_ident.
    pub fn resolve_type(&self, id: TypeId) -> &str {
        self.type_symbols
            .get_ident(id)
            .map(|&s_id| self.resolve_symbol(s_id)) // On réutilise la fonction de base
            .unwrap_or("<unknown_type>")
    }

    pub fn resolve_predicate(&self, id: PredicateSymbolId) -> &str {
        self.predicate_symbols
            .get_ident(id)
            .map(|&s_id| self.resolve_symbol(s_id))
            .unwrap_or("<unknown_pred>")
    }

    pub fn resolve_object(&self, id: ObjectId) -> &str {
        self.object_symbols
            .get_ident(id)
            .map(|&s_id| self.resolve_symbol(s_id))
            .unwrap_or("<unknown_obj>")
    }

    pub fn resolve_functor(&self, id: FunctionSymbolId) -> &str {
        self.functor_symbols
            .get_ident(id)
            .map(|&s_id| self.resolve_symbol(s_id))
            .unwrap_or("<unknown_func>")
    }

    pub fn resolve_task_symbol(&self, id: TaskSymbolId) -> &str {
        self.task_symbols
            .get_ident(id)
            .map(|&s_id| self.resolve_symbol(s_id))
            .unwrap_or("<unknown_task>")
    }

    pub fn resolve_action_symbol(&self, id: ActionSymbolId) -> &str {
        self.action_symbols
            .get_ident(id)
            .map(|&s_id| self.resolve_symbol(s_id))
            .unwrap_or("<unknown_action>")
    }

    pub fn resolve_method_symbol(&self, id: MethodSymbolId) -> &str {
        self.method_symbols
            .get_ident(id)
            .map(|&s_id| self.resolve_symbol(s_id))
            .unwrap_or("<unknown_method>")
    }
}
