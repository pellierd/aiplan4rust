use crate::aiplan4rust::lang::{
    ArithmeticOp, AssignOp, AtomSkeletonId, CompareOp, FunctionSkeletonId, FunctionSymbolId,
    ObjectId, OptimizationOp, PredicateSymbolId, PreferenceSymbolId, TaskLabelSymbolId,
    TaskSkeletonId, TaskSymbolId, Type, TypeId, TypedList, TypedSymbol, VariableId,
};
use crate::aiplan4rust::lir::store::error::StorerError;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId, ExprNodeRef, ExprStore};
use ordered_float::OrderedFloat;

/// Le Builder sert de façade ergonomique au-dessus du ExprStore.
/// Il facilite la création d'expressions tout en garantissant le Hash-Consing.
pub struct ExprBuilder<'a> {
    store: &'a mut ExprStore,
}

impl<'a> ExprBuilder<'a> {
    /// Crée un nouveau builder lié à une référence mutable du store.
    pub fn new(store: &'a mut ExprStore) -> Self {
        Self { store }
    }

    /// Donne accès au store interne si une manipulation directe est requise.
    fn store(&mut self) -> &mut ExprStore {
        self.store
    }

    /// Raccourci pour l'internement dans le store.
    #[inline]
    pub fn intern(&mut self, kind: ExprEntryKind, children: Vec<ExprId>) -> ExprId {
        self.store().intern(kind, children)
    }

    /// Récupère une vue ergonomique sur l'expression (ID + Entry).
    /// Utilise l'option pour les vérifications de routine.
    #[inline]
    pub fn get(&self, id: ExprId) -> Option<ExprNodeRef<'_>> {
        self.store.get(id)
    }

    /// Récupère l'entrée ou échoue avec une erreur spécifique aux opérations.
    /// On convertit ici la StorerError du store en ta ExprOpErrorHC.
    #[inline]
    pub fn fetch(&self, id: ExprId) -> Result<ExprNodeRef<'_>, StorerError> {
        self.store.fetch(id)
    }

    pub fn reconstruct(&mut self, kind: ExprEntryKind, children: Vec<ExprId>) -> ExprId {
        match kind {
            // --- 1. Opérateurs Logiques et Arithmétiques Variadiques ---
            // On utilise les Smart Constructors (and, or, add...) pour les simplifications
            ExprEntryKind::And => self.and(children),
            ExprEntryKind::Or => self.or(children),
            ExprEntryKind::Arithmetic(op) => self.arithmetic_exp(op, children),

            // --- 2. Opérateurs Binaires (2 enfants) ---
            ExprEntryKind::Imply => self.imply(children[0], children[1]),
            ExprEntryKind::When => self.when(children[0], children[1]),
            ExprEntryKind::Comparison(op) => self.comparison(op, children[0], children[1]),
            ExprEntryKind::Assignment(op) => self.assign_expr(op, children[0], children[1]),

            // TaskOrderingConstraint n'a pas de helper public direct prenant 2 IDs,
            // on utilise l'internement direct comme dans ton API.
            ExprEntryKind::TaskOrderingConstraint(op) => self
                .store
                .intern(ExprEntryKind::TaskOrderingConstraint(op), children),

            // --- 3. Opérateurs Unaires (1 enfant) ---
            ExprEntryKind::Not => self.not(children[0]),
            ExprEntryKind::AtStart => self.at_start(children[0]),
            ExprEntryKind::AtEnd => self.at_end(children[0]),
            ExprEntryKind::Overall => self.overall(children[0]),

            // Pour Preference, IsViolated et Metric, l'API utilise des helpers
            // qui créent des feuilles (symboles). Ici, children contient déjà les IDs.
            ExprEntryKind::Preference | ExprEntryKind::IsViolated | ExprEntryKind::Metric(_) => {
                self.store.intern(kind, children)
            }

            // --- 4. Quantificateurs (Données + 1 enfant) ---
            ExprEntryKind::Forall(vars) => self.forall(vars, children[0]),
            ExprEntryKind::Exists(vars) => self.exists(vars, children[0]),

            // --- 5. Noeuds avec Squelettes (AtomicFormula, Function, Task) ---
            // Crucial : children[0] est le symbole, children[1..] sont les arguments.
            ExprEntryKind::AtomicFormula(skel) => self
                .store
                .intern(ExprEntryKind::AtomicFormula(skel), children),
            ExprEntryKind::Function(skel) => {
                self.store.intern(ExprEntryKind::Function(skel), children)
            }
            ExprEntryKind::Task(skel) => self.store.intern(ExprEntryKind::Task(skel), children),

            // --- 6. Cas Temporels Complexes et HDN ---
            ExprEntryKind::Always
            | ExprEntryKind::Sometime
            | ExprEntryKind::Within
            | ExprEntryKind::AtMostOnce
            | ExprEntryKind::SometimeAfter
            | ExprEntryKind::SometimeBefore
            | ExprEntryKind::AlwaysWithin
            | ExprEntryKind::HoldDuring
            | ExprEntryKind::HoldAfter
            | ExprEntryKind::TimedInitialLiteral
            | ExprEntryKind::LabeledTask
            | ExprEntryKind::Serial
            | ExprEntryKind::Parallel
            | ExprEntryKind::Length => self.store.intern(kind, children),

            // --- 7. Cas Terminaux (Feuilles) ---
            // Object, Variable, symbols, Number, TotalTime...
            leaf_kind => self.leaf(leaf_kind),
        }
    }

    /// Helper interne pour garantir l'internement propre des feuilles.
    pub fn leaf(&mut self, kind: ExprEntryKind) -> ExprId {
        self.store.intern(kind, vec![])
    }

    /// Remplace `unary`. Appelle le Hash-Consing du store.
    fn unary(&mut self, kind: ExprEntryKind, child: ExprId) -> ExprId {
        self.store.intern(kind, vec![child])
    }

    /// Remplace `binary`.
    fn binary(&mut self, kind: ExprEntryKind, left: ExprId, right: ExprId) -> ExprId {
        self.store.intern(kind, vec![left, right])
    }

    /// Remplace `nary`.
    /// C'est ici que le tri et le dedup de ton LIR s'activeront pour And/Or.
    /// Pour AND/OR : Simplification au vol (Smart Constructor)
    fn nary(&mut self, kind: ExprEntryKind, mut children: Vec<ExprId>) -> ExprId {
        let (neutral, absorbing) = match kind {
            ExprEntryKind::And => (self.empty_and(), self.empty_or()),
            ExprEntryKind::Or => (self.empty_or(), self.empty_and()),
            _ => return self.intern(kind, children),
        };

        // 1. Si un seul enfant est "absorbant", toute l'expression prend cette valeur
        if children.iter().any(|&id| id == absorbing) {
            return absorbing;
        }

        // 2. On retire les neutres
        children.retain(|&id| id != neutral);

        // 3. Cas limites
        if children.is_empty() {
            neutral
        } else if children.len() == 1 {
            children[0]
        } else {
            self.intern(kind, children)
        }
    }

    /// Crée un nœud constante (objet) à partir d'un identifiant.
    pub fn constant<I: Into<ObjectId>>(&mut self, id: I) -> ExprId {
        let val: usize = id.into().into(); // Conversion en usize
        self.leaf(ExprEntryKind::Object(ObjectId::from(val)))
    }

    /// Crée un nœud variable à partir d'un identifiant.
    pub fn variable<I: Into<VariableId>>(&mut self, id: I) -> ExprId {
        let val: usize = id.into().into();
        self.leaf(ExprEntryKind::Variable(VariableId::from(val)))
    }

    /// Crée un nœud symbole de fonction (functor).
    pub fn function_symbol<I: Into<FunctionSymbolId>>(&mut self, id: I) -> ExprId {
        let val: usize = id.into().into();
        self.leaf(ExprEntryKind::FunctionSymbol(FunctionSymbolId::from(val)))
    }

    /// Crée un nœud symbole de prédicat.
    pub fn predicate<I: Into<PredicateSymbolId>>(&mut self, id: I) -> ExprId {
        let val: usize = id.into().into();
        self.leaf(ExprEntryKind::PredicateSymbol(PredicateSymbolId::from(val)))
    }

    /// Crée un nœud symbole de tâche (HTN).
    pub fn task_symbol<I: Into<TaskSymbolId>>(&mut self, id: I) -> ExprId {
        let val: usize = id.into().into();
        self.leaf(ExprEntryKind::TaskSymbol(TaskSymbolId::from(val)))
    }

    /// Crée un nœud de nom de préférence.
    pub fn pref_name<I: Into<PreferenceSymbolId>>(&mut self, id: I) -> ExprId {
        let val: usize = id.into().into();
        self.leaf(ExprEntryKind::PrefName(PreferenceSymbolId::from(val)))
    }

    /// Crée un nœud de littéral numérique.
    /// L'utilisation de OrderedFloat garantit que deux nombres identiques
    /// auront le même ExprId dans le store.
    pub fn number(&mut self, value: f64) -> ExprId {
        let val = OrderedFloat::from(value);
        self.leaf(ExprEntryKind::Number(val))
    }

    /// Crée une `AtomicFormula`.
    /// Grâce aux génériques, accepte aussi bien des IDs bruts (usize) que des types typés.
    pub fn atomic_formula<PID, SID>(
        &mut self,
        sym_id: PID,
        args: Vec<ExprId>,
        skel_id: SID,
    ) -> ExprId
    where
        PID: Into<PredicateSymbolId>,
        SID: Into<AtomSkeletonId>,
    {
        // On convertit les entrées immédiatement via .into()
        let sym_node = self.predicate(sym_id.into());
        let skel = skel_id.into();

        self.build_atomic_node(skel, sym_node, args)
    }

    /// Assemble le nœud atomique : [Symbole, ...Arguments].
    fn build_atomic_node(
        &mut self,
        skel_id: AtomSkeletonId,
        sym_node: ExprId,
        args: Vec<ExprId>,
    ) -> ExprId {
        let mut children = Vec::with_capacity(args.len() + 1);
        children.push(sym_node);
        children.extend(args);

        self.store
            .intern(ExprEntryKind::AtomicFormula(skel_id), children)
    }

    /// (Note : si tu n'as pas de squelette au parsing, il faudra soit une valeur par défaut,
    /// soit changer le Kind pour accepter Option).
    /// Crée un `FunctionTerm` (ou Fluid).
    /// Accepte aussi bien `FunctionSymbolId` que `usize`.
    pub fn function_term<FID, SID>(
        &mut self,
        sym_id: FID,
        args: Vec<ExprId>,
        skel_id: SID,
    ) -> ExprId
    where
        FID: Into<FunctionSymbolId>,
        SID: Into<FunctionSkeletonId>,
    {
        let sym_node = self.function_symbol(sym_id.into());
        let skel = skel_id.into();

        self.build_function_node(skel, sym_node, args)
    }

    /// Assemble le nœud de fonction : [Symbole, ...Arguments].
    fn build_function_node(
        &mut self,
        skel_id: FunctionSkeletonId,
        sym_node: ExprId,
        args: Vec<ExprId>,
    ) -> ExprId {
        let mut children = Vec::with_capacity(args.len() + 1);
        children.push(sym_node);
        children.extend(args);

        self.store
            .intern(ExprEntryKind::Function(skel_id), children)
    }

    /// Crée un nœud logique `AND` avec un ou plusieurs enfants.
    /// Grâce au Hash-Consing du store, les enfants seront triés et dédoublonnés.
    pub fn and(&mut self, children: Vec<ExprId>) -> ExprId {
        self.nary(ExprEntryKind::And, children)
    }

    /// Crée un nœud `AND` vide, représentant la constante "Vrai".
    pub fn empty_and(&mut self) -> ExprId {
        self.store.true_expr()
    }

    /// Crée un nœud logique `OR` avec un ou plusieurs enfants.
    pub fn or(&mut self, children: Vec<ExprId>) -> ExprId {
        self.nary(ExprEntryKind::Or, children)
    }

    /// Crée un nœud `OR` vide, représentant la constante "Faux".
    pub fn empty_or(&mut self) -> ExprId {
        self.store.false_expr()
    }

    /// Crée un nœud logique `NOT` (négation).
    pub fn not(&mut self, expr: ExprId) -> ExprId {
        self.unary(ExprEntryKind::Not, expr)
    }

    /// Crée un nœud `Imply` (A → B).
    /// Note : L'implication est binaire et n'est pas triée par le store (non-commutative).
    pub fn imply(&mut self, antecedent: ExprId, consequent: ExprId) -> ExprId {
        self.binary(ExprEntryKind::Imply, antecedent, consequent)
    }

    // Crée un nœud `Forall` : (forall (vars...) body)
    /// La liste des variables fait partie de la signature de Hash-Consing.
    pub fn forall(&mut self, vars: TypedList<VariableId, TypeId>, body: ExprId) -> ExprId {
        self.store.intern(ExprEntryKind::Forall(vars), vec![body])
    }

    /// Crée un nœud `Exists` : (exists (vars...) body)
    pub fn exists(&mut self, vars: TypedList<VariableId, TypeId>, body: ExprId) -> ExprId {
        self.store.intern(ExprEntryKind::Exists(vars), vec![body])
    }

    // --- Helpers de construction de types (inchangés car ils ne touchent pas au Store) ---

    pub fn typed_variable_list(
        &mut self,
        vars: Vec<TypedSymbol<VariableId, TypeId>>,
    ) -> TypedList<VariableId, TypeId> {
        let mut list = TypedList::new();
        for typed_var in vars {
            list.push(typed_var);
        }
        list
    }

    pub fn typed_variable(
        &mut self,
        id: usize,
        type_ids: &[usize],
    ) -> TypedSymbol<VariableId, TypeId> {
        let ty = self.ty(type_ids);
        TypedSymbol::new(VariableId::from(id), ty)
    }

    pub fn ty(&mut self, ids: &[usize]) -> Type<TypeId> {
        Type::either(ids.iter().map(|&id| TypeId::from(id)).collect())
    }

    /// Crée un nœud `Preference` : (preference name body)
    /// Le premier enfant est le symbole de préférence, le second est la formule.
    pub fn preference<I: Into<PreferenceSymbolId>>(&mut self, id: I, body: ExprId) -> ExprId {
        let pref_symbol_node = self.pref_name(id);
        self.binary(ExprEntryKind::Preference, pref_symbol_node, body)
    }

    /// Crée un nœud `When` : (when condition effect)
    /// Utilisé pour les effets conditionnels.
    pub fn when(&mut self, condition: ExprId, effect: ExprId) -> ExprId {
        self.binary(ExprEntryKind::When, condition, effect)
    }

    /// Crée une comparaison fonctionnelle (`FComp`) : (op left right)
    /// L'opérateur fait partie du Kind pour une identification structurelle rapide.
    pub fn comparison(&mut self, op: CompareOp, left: ExprId, right: ExprId) -> ExprId {
        self.store
            .intern(ExprEntryKind::Comparison(op), vec![left, right])
    }

    // --- Helpers de commodité pour les comparaisons ---

    pub fn less(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.comparison(CompareOp::Less, left, right)
    }

    pub fn less_eq(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.comparison(CompareOp::LessEq, left, right)
    }

    pub fn greater(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.comparison(CompareOp::Greater, left, right)
    }

    pub fn greater_eq(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.comparison(CompareOp::GreaterEq, left, right)
    }

    pub fn equal(&mut self, left: ExprId, right: ExprId) -> ExprId {
        self.comparison(CompareOp::Equal, left, right)
    }

    /// Crée un nœud d'assignation fonctionnelle : (op target value)
    /// L'opération (assign, increase, decrease, etc.) est stockée dans le Kind.
    fn assign_expr(&mut self, op: AssignOp, target: ExprId, value: ExprId) -> ExprId {
        self.store
            .intern(ExprEntryKind::Assignment(op), vec![target, value])
    }

    /// (assign target value)
    pub fn assign(&mut self, target: ExprId, value: ExprId) -> ExprId {
        self.assign_expr(AssignOp::Assign, target, value)
    }

    /// (increase target value)
    pub fn increase(&mut self, target: ExprId, value: ExprId) -> ExprId {
        self.assign_expr(AssignOp::Increase, target, value)
    }

    /// (decrease target value)
    pub fn decrease(&mut self, target: ExprId, value: ExprId) -> ExprId {
        self.assign_expr(AssignOp::Decrease, target, value)
    }

    /// (scale-up target value)
    pub fn scale_up(&mut self, target: ExprId, value: ExprId) -> ExprId {
        self.assign_expr(AssignOp::ScaleUp, target, value)
    }

    /// (scale-down target value)
    pub fn scale_down(&mut self, target: ExprId, value: ExprId) -> ExprId {
        self.assign_expr(AssignOp::ScaleDown, target, value)
    }

    /// Crée un nœud d'expression arithmétique : (op operands...)
    /// L'opérateur (Add, Sub, Mul, Div) est stocké dans le Kind.
    fn arithmetic_exp(&mut self, op: ArithmeticOp, operands: Vec<ExprId>) -> ExprId {
        self.store.intern(ExprEntryKind::Arithmetic(op), operands)
    }

    /// Addition : (+ operands...)
    /// Le store normalisera l'ordre des opérandes pour maximiser le Hash-Consing.
    pub fn add(&mut self, operands: Vec<ExprId>) -> ExprId {
        self.arithmetic_exp(ArithmeticOp::Add, operands)
    }

    /// Soustraction : (- a b c) => a - b - c
    /// Note : La soustraction n'est pas commutative, l'ordre des enfants est préservé.
    pub fn sub(&mut self, operands: Vec<ExprId>) -> ExprId {
        self.arithmetic_exp(ArithmeticOp::Sub, operands)
    }

    /// Multiplication : (* operands...)
    pub fn mul(&mut self, operands: Vec<ExprId>) -> ExprId {
        self.arithmetic_exp(ArithmeticOp::Mul, operands)
    }

    /// Division : (/ dividend divisor)
    pub fn div(&mut self, operands: Vec<ExprId>) -> ExprId {
        self.arithmetic_exp(ArithmeticOp::Div, operands)
    }

    pub fn at_start(&mut self, expr: ExprId) -> ExprId {
        self.unary(ExprEntryKind::AtStart, expr)
    }

    pub fn at_end(&mut self, expr: ExprId) -> ExprId {
        self.unary(ExprEntryKind::AtEnd, expr)
    }

    pub fn overall(&mut self, expr: ExprId) -> ExprId {
        self.unary(ExprEntryKind::Overall, expr)
    }

    // --- Trajectory Constraints (PDDL 3.0) ---

    pub fn always(&mut self, expr: ExprId) -> ExprId {
        self.unary(ExprEntryKind::Always, expr)
    }

    pub fn sometime(&mut self, expr: ExprId) -> ExprId {
        self.unary(ExprEntryKind::Sometime, expr)
    }

    pub fn at_most_once(&mut self, expr: ExprId) -> ExprId {
        self.unary(ExprEntryKind::AtMostOnce, expr)
    }

    // --- Opérateurs Binaires ---

    pub fn sometime_after(&mut self, first: ExprId, second: ExprId) -> ExprId {
        self.binary(ExprEntryKind::SometimeAfter, first, second)
    }

    pub fn sometime_before(&mut self, first: ExprId, second: ExprId) -> ExprId {
        self.binary(ExprEntryKind::SometimeBefore, first, second)
    }

    // --- Opérateurs avec Durées (N-aires) ---

    pub fn within(&mut self, value: f64, expr: ExprId) -> ExprId {
        let duration_node = self.number(value);
        self.intern(ExprEntryKind::Within, vec![duration_node, expr])
    }

    pub fn always_within(&mut self, duration: f64, first: ExprId, second: ExprId) -> ExprId {
        let number_node = self.number(duration);
        // On utilise intern pour garantir que les 3 enfants restent présents
        // même si l'un d'eux est un élément logiquement "neutre".
        self.intern(
            ExprEntryKind::AlwaysWithin,
            vec![number_node, first, second],
        )
    }

    pub fn hold_during(&mut self, start: f64, end: f64, expr: ExprId) -> ExprId {
        let start_node = self.number(start);
        let end_node = self.number(end);
        // On utilise intern au lieu de nary pour protéger les arguments
        self.intern(ExprEntryKind::HoldDuring, vec![start_node, end_node, expr])
    }

    pub fn hold_after(&mut self, time: f64, expr: ExprId) -> ExprId {
        let time_node = self.number(time);
        // Utilisation directe de intern car l'arité est fixe (2)
        self.intern(ExprEntryKind::HoldAfter, vec![time_node, expr])
    }

    /// Crée un `TimedInitialLiteral` (TIL) : une expression qui devient vraie à `time`.
    pub fn timed_initial_literal(&mut self, time: f64, expr: ExprId) -> ExprId {
        let time_node = self.number(time);
        // Utilisation directe de intern : on garantit que le temps et l'expression restent liés
        self.intern(ExprEntryKind::TimedInitialLiteral, vec![time_node, expr])
    }

    /// Crée une expression de métrique (optimisation).
    fn metric_exp(&mut self, opt: OptimizationOp, expr: ExprId) -> ExprId {
        self.store.intern(ExprEntryKind::Metric(opt), vec![expr])
    }

    pub fn minimize(&mut self, expr: ExprId) -> ExprId {
        self.metric_exp(OptimizationOp::Minimize, expr)
    }

    pub fn maximize(&mut self, expr: ExprId) -> ExprId {
        self.metric_exp(OptimizationOp::Maximize, expr)
    }

    /// Représente la variable `total-time` (makespan).
    pub fn total_time(&mut self) -> ExprId {
        self.leaf(ExprEntryKind::TotalTime)
    }

    /// Vérifie si une préférence est violée : (is-violated name).
    pub fn is_violated<I: Into<PreferenceSymbolId>>(&mut self, id: I) -> ExprId {
        let pref_node = self.pref_name(id);
        // On utilise intern directement.
        // Un IsViolated doit toujours avoir son PreferenceSymbolId en enfant.
        self.intern(ExprEntryKind::IsViolated, vec![pref_node])
    }

    // --- Plan Length / Structural Constraints ---

    pub fn length(&mut self, serial: Option<f64>, parallel: Option<f64>) -> ExprId {
        let mut children = Vec::new();
        if let Some(s) = serial {
            children.push(self.serial(s));
        }
        if let Some(p) = parallel {
            children.push(self.parallel(p));
        }
        self.nary(ExprEntryKind::Length, children)
    }

    pub fn serial(&mut self, value: f64) -> ExprId {
        let number = self.number(value);
        self.unary(ExprEntryKind::Serial, number)
    }

    pub fn parallel(&mut self, value: f64) -> ExprId {
        let number = self.number(value);
        self.unary(ExprEntryKind::Parallel, number)
    }

    /// Crée une instance de tâche avec son squelette (Optimisé LIR).
    /// Suit exactement le même pattern que les formules atomiques.
    /// Crée une instance de tâche avec son squelette (Optimisé LIR).
    /// Suit exactement le même pattern que les formules atomiques.
    pub fn task_with_skeleton<TID, SID>(
        &mut self,
        sym_id: TID,
        args: Vec<ExprId>,
        skel_id: SID,
    ) -> ExprId
    where
        TID: Into<TaskSymbolId>,
        SID: Into<TaskSkeletonId>,
    {
        let sym_node = self.task_symbol(sym_id);
        let skel = skel_id.into();
        self.build_task_node(skel, sym_node, args)
    }

    /// Assemble le nœud de tâche : [Symbole, ...Arguments].
    fn build_task_node(
        &mut self,
        skel_id: TaskSkeletonId,
        sym_node: ExprId,
        args: Vec<ExprId>,
    ) -> ExprId {
        let mut children = Vec::with_capacity(args.len() + 1);
        children.push(sym_node);
        children.extend(args);

        self.store.intern(ExprEntryKind::Task(skel_id), children)
    }

    /// Crée un identifiant de tâche (Label).
    pub fn task_id<I: Into<TaskLabelSymbolId>>(&mut self, id: I) -> ExprId {
        let val: usize = id.into().into();
        self.leaf(ExprEntryKind::TaskLabel(TaskLabelSymbolId::from(val)))
    }

    /// Associe un label à une tâche (LabeledTask).
    pub fn tagged_task<I: Into<TaskLabelSymbolId>>(&mut self, id: I, task_expr: ExprId) -> ExprId {
        let task_id_node = self.task_id(id);
        self.binary(ExprEntryKind::LabeledTask, task_id_node, task_expr)
    }

    /// Crée une contrainte d'ordonnancement (Task1 < Task2).
    /// Note : On réutilise CompareOp::Less pour la sémantique interne.
    pub fn task_ordering_constraint(&mut self, task1: ExprId, task2: ExprId) -> ExprId {
        // On stocke l'opérateur de comparaison dans le Kind pour la structure
        self.store.intern(
            ExprEntryKind::TaskOrderingConstraint(CompareOp::Less),
            vec![task1, task2],
        )
    }
}
