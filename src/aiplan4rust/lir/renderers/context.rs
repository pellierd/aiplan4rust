use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::{ActionSymbolID, FunctorID, MethodSymbolID, ObjectID, PredicateID, StringID, TaskSymbolID, TypeID};
use crate::aiplan4rust::lir::problem::{LiftedProblem, SymbolRegistry};

pub struct RenderContext<'a> {
    interner: &'a StringInterner,
    type_symbols: &'a SymbolRegistry<TypeID>,
    predicate_symbols: &'a SymbolRegistry<PredicateID>,
    functor_symbols: &'a SymbolRegistry<FunctorID>,
    object_symbols: &'a SymbolRegistry<ObjectID>,
    task_symbols: &'a SymbolRegistry<TaskSymbolID>,
    action_symbols: &'a SymbolRegistry<ActionSymbolID>,
    method_symbols: &'a SymbolRegistry<MethodSymbolID>,
}

impl<'a> RenderContext<'a> {
    pub fn new(problem: &'a LiftedProblem) -> Self {
        Self {
            interner: &problem.interner(),
            type_symbols: &problem.type_symbols(),
            predicate_symbols: &problem.predicate_symbols(),
            functor_symbols: &problem.function_symbols(),
            object_symbols: &problem.object_symbol(),
            task_symbols: &problem.task_symbols(),
            action_symbols: &problem.action_symbols(),
            method_symbols: &problem.method_symbols(), // Aj
        }
    }

    // --- Accesseurs directs aux tables ---
    pub fn types(&self) -> &SymbolRegistry<TypeID> { self.type_symbols }
    pub fn predicates(&self) -> &SymbolRegistry<PredicateID> { self.predicate_symbols }
    pub fn functors(&self) -> &SymbolRegistry<FunctorID> { self.functor_symbols }
    pub fn objects(&self) -> &SymbolRegistry<ObjectID> { self.object_symbols }
    pub fn task_symbols(&self) -> &SymbolRegistry<TaskSymbolID> { self.task_symbols }

    pub fn action_symbols(&self) -> &SymbolRegistry<ActionSymbolID> { self.action_symbols }

    pub fn method_symbols(&self) -> &SymbolRegistry<MethodSymbolID> { self.method_symbols }

    pub fn interner(&self) -> &StringInterner { self.interner }

    // --- Résolution de noms via les SymbolTables ---

    /// La base : résout un StringID brut via l'interner.
    pub fn resolve_symbol(&self, id: StringID) -> &str {
        self.interner.resolve_ident(id).unwrap_or("<unknown_id>")
    }

    /// Résout un TypeID en passant par sa table, puis en utilisant resolve_ident.
    pub fn resolve_type(&self, id: TypeID) -> &str {
        self.type_symbols
            .get_ident(id)
            .map(|&s_id| self.resolve_symbol(s_id)) // On réutilise la fonction de base
            .unwrap_or("<unknown_type>")
    }

    pub fn resolve_predicate(&self, id: PredicateID) -> &str {
        self.predicate_symbols
            .get_ident(id)
            .map(|&s_id| self.resolve_symbol(s_id))
            .unwrap_or("<unknown_pred>")
    }

    pub fn resolve_object(&self, id: ObjectID) -> &str {
        self.object_symbols
            .get_ident(id)
            .map(|&s_id| self.resolve_symbol(s_id))
            .unwrap_or("<unknown_obj>")
    }

    pub fn resolve_functor(&self, id: FunctorID) -> &str {
        self.functor_symbols
            .get_ident(id)
            .map(|&s_id| self.resolve_symbol(s_id))
            .unwrap_or("<unknown_func>")
    }

    pub fn resolve_task_symbol(&self, id: TaskSymbolID) -> &str {
        self.task_symbols
            .get_ident(id)
            .map(|&s_id| self.resolve_symbol(s_id))
            .unwrap_or("<unknown_task>")
    }

    pub fn resolve_action_symbol(&self, id: ActionSymbolID) -> &str {
        self.action_symbols
            .get_ident(id)
            .map(|&s_id| self.resolve_symbol(s_id))
            .unwrap_or("<unknown_action>")
    }

    pub fn resolve_method_symbol(&self, id: MethodSymbolID) -> &str {
        self.method_symbols
            .get_ident(id)
            .map(|&s_id| self.resolve_symbol(s_id))
            .unwrap_or("<unknown_method>")
    }

}
