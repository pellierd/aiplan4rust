use crate::aiplan4rust::interner::StringInterner;
use crate::aiplan4rust::lang::{FunctorID, ObjectID, PredicateID, StringID, TaskSymbolID, TypeID};
use crate::aiplan4rust::lir::problem::{LiftedProblem, SymbolTable};

pub struct RenderContext<'a> {
    interner: &'a StringInterner,
    type_symbols: &'a SymbolTable<TypeID>,
    predicate_symbols: &'a SymbolTable<PredicateID>,
    functor_symbols: &'a SymbolTable<FunctorID>,
    object_symbols: &'a SymbolTable<ObjectID>,
    task_symbols: &'a SymbolTable<TaskSymbolID>,
}

impl<'a> RenderContext<'a> {
    pub fn new(problem: &'a LiftedProblem) -> Self {
        Self {
            interner: &problem.interner(),
            type_symbols: &problem.type_symbol_table(),
            predicate_symbols: &problem.predicate_symbol_table(),
            functor_symbols: &problem.functor_symbol_table(),
            object_symbols: &problem.object_symbol_table(),
            task_symbols: &problem.task_symbol_table(),
        }
    }

    // --- Accesseurs directs aux tables ---
    pub fn types(&self) -> &SymbolTable<TypeID> { self.type_symbols }
    pub fn predicates(&self) -> &SymbolTable<PredicateID> { self.predicate_symbols }
    pub fn functors(&self) -> &SymbolTable<FunctorID> { self.functor_symbols }
    pub fn objects(&self) -> &SymbolTable<ObjectID> { self.object_symbols }
    pub fn tasks_symbol(&self) -> &SymbolTable<TaskSymbolID> { self.task_symbols }
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


}
