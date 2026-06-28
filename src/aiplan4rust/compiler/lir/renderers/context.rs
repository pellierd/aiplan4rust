use crate::aiplan4rust::compiler::lir::expr::ExprStore;
use crate::aiplan4rust::compiler::lir::problem::{LiftedProblem, SymbolRegistry};
use crate::aiplan4rust::support::interner::SymbolInterner;
use crate::aiplan4rust::support::lang::{
    ActionSymbolId, FunctionSymbolId, MethodSymbolId, ObjectId, PredicateSymbolId, SymbolId,
    TaskSymbolId, TypeId, VariableId,
};
use std::sync::LazyLock;

pub struct RenderContext<'a> {
    interner: &'a SymbolInterner,
    store: &'a ExprStore,
    type_symbols: &'a SymbolRegistry<TypeId>,
    predicate_symbols: &'a SymbolRegistry<PredicateSymbolId>,
    functor_symbols: &'a SymbolRegistry<FunctionSymbolId>,
    object_symbols: &'a SymbolRegistry<ObjectId>,
    task_symbols: &'a SymbolRegistry<TaskSymbolId>,
    action_symbols: &'a SymbolRegistry<ActionSymbolId>,
    method_symbols: &'a SymbolRegistry<MethodSymbolId>,

    variable_symbols: Option<&'a SymbolRegistry<VariableId>>,
}

impl<'a> RenderContext<'a> {
    pub fn new(problem: &'a LiftedProblem) -> Self {
        Self {
            interner: &problem.interner(),
            store: &problem.store(),
            type_symbols: problem.type_symbols(),
            predicate_symbols: &problem.predicate_symbols(),
            functor_symbols: &problem.function_symbols(),
            object_symbols: &problem.object_symbols(),
            task_symbols: &problem.task_symbols(),
            action_symbols: &problem.action_symbols(),
            method_symbols: &problem.method_symbols(), // Aj
            variable_symbols: None,
        }
    }

    /// Creates a fallback, symbol-free rendering context for structural diagnostics.
    ///
    /// This constructor is specifically designed for isolation testing, internal pipeline
    /// debugging, and post-condition validations (e.g., NNF/FNF passes) where a full
    /// `LiftedProblem` instance is unavailable or unnecessary.
    ///
    /// All internal registries are backed by globally cached, empty static instances.
    /// As a result, resolving any identifier through this context will safely fall back
    /// to default placeholders (e.g., `<unknown_pred>`) without disrupting the structural
    /// layout or layout geometry of the expression tree.
    ///
    /// # Performance
    ///
    /// Uses [`std::sync::LazyLock`] internally to lazily initialize empty static registries
    /// on the first call. Subsequent allocations are zero-cost as references are coerced
    /// from `'static` to the lifetime `'a` of the provided [`ExprStore`].
    ///
    /// # Examples
    ///
    /// ```text
    /// use crate::aiplan4rust::compiler::lir::renderers::RenderContext;
    /// use crate::aiplan4rust::compiler::lir::expr::ExprStore;
    ///
    /// fn validate_tree(store: &ExprStore, root: ExprId) {
    ///     // Instantiate a minimal structural context
    ///     let ctx = RenderContext::debug(store);
    ///
    ///     // Safe structural rendering without full problem dependencies
    ///     println!("{}", root.as_debug(&ctx));
    /// }
    /// ```
    pub fn debug(store: &'a ExprStore) -> Self {
        static EMPTY_INTERNER: LazyLock<SymbolInterner> = LazyLock::new(SymbolInterner::default);
        static EMPTY_REGISTRY_TYPE: LazyLock<SymbolRegistry<TypeId>> =
            LazyLock::new(SymbolRegistry::default);
        static EMPTY_REGISTRY_PRED: LazyLock<SymbolRegistry<PredicateSymbolId>> =
            LazyLock::new(SymbolRegistry::default);
        static EMPTY_REGISTRY_FUNC: LazyLock<SymbolRegistry<FunctionSymbolId>> =
            LazyLock::new(SymbolRegistry::default);
        static EMPTY_REGISTRY_OBJ: LazyLock<SymbolRegistry<ObjectId>> =
            LazyLock::new(SymbolRegistry::default);
        static EMPTY_REGISTRY_TASK: LazyLock<SymbolRegistry<TaskSymbolId>> =
            LazyLock::new(SymbolRegistry::default);
        static EMPTY_REGISTRY_ACT: LazyLock<SymbolRegistry<ActionSymbolId>> =
            LazyLock::new(SymbolRegistry::default);
        static EMPTY_REGISTRY_METH: LazyLock<SymbolRegistry<MethodSymbolId>> =
            LazyLock::new(SymbolRegistry::default);

        Self {
            interner: &EMPTY_INTERNER,
            store,
            type_symbols: &EMPTY_REGISTRY_TYPE,
            predicate_symbols: &EMPTY_REGISTRY_PRED,
            functor_symbols: &EMPTY_REGISTRY_FUNC,
            object_symbols: &EMPTY_REGISTRY_OBJ,
            task_symbols: &EMPTY_REGISTRY_TASK,
            action_symbols: &EMPTY_REGISTRY_ACT,
            method_symbols: &EMPTY_REGISTRY_METH,
            variable_symbols: None,
        }
    }

    /// Crée un nouveau contexte de rendu incluant des variables locales.
    pub fn with_variables(&self, vars: &'a SymbolRegistry<VariableId>) -> Self {
        Self {
            variable_symbols: Some(vars),
            ..*self
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

    /// Retourne la table des variables locales si elle existe.
    pub fn variable_symbols(&self) -> Option<&SymbolRegistry<VariableId>> {
        self.variable_symbols
    }

    pub fn interner(&self) -> &SymbolInterner {
        self.interner
    }

    /// Returns a reference to the expression old used by the problem.
    pub fn store(&self) -> &ExprStore {
        &self.store
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

    // --- Résolution de noms ---

    /// Résout un VariableID en passant par sa table locale, puis l'interner.
    /// Retourne le nom brut sans le préfixe '?' (pour laisser le choix au renderer).
    pub fn resolve_variable(&self, id: VariableId) -> &str {
        self.variable_symbols
            .and_then(|table| table.get_ident(id))
            .map(|&s_id| self.resolve_symbol(s_id))
            .unwrap_or("<unknown_var>")
    }
}
