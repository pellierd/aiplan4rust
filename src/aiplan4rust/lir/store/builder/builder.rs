use crate::aiplan4rust::lir::store::error::StorerError;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId, ExprNodeRef, ExprStore};

const EPSILON: f64 = 1e-10;

/// Le Builder sert de façade ergonomique au-dessus du ExprStore.
/// Il facilite la création d'expressions tout en garantissant le Hash-Consing.
pub struct ExprBuilder<'a> {
    pub(crate) store: &'a mut ExprStore,
    pub(crate) primary_buffer: Vec<ExprId>,
    pub(crate) secondary_buffer: Vec<ExprId>,
}

impl<'a> ExprBuilder<'a> {
    /// Crée un nouveau builder lié à une référence mutable du store.
    pub fn new(store: &'a mut ExprStore) -> Self {
        Self {
            store,
            primary_buffer: Vec::with_capacity(32),
            secondary_buffer: Vec::with_capacity(32),
        }
    }

    /// Donne accès au store interne si une manipulation directe est requise.
    fn store(&mut self) -> &mut ExprStore {
        self.store
    }

    /// Raccourci pour l'internement dans le store.
    #[inline]
    pub fn intern(&mut self, kind: ExprEntryKind, children: &[ExprId]) -> ExprId {
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

    pub fn reconstruct(&mut self, kind: ExprEntryKind, children: &[ExprId]) -> ExprId {
        match kind {
            // --- 1. Opérateurs Logiques et Arithmétiques Variadiques ---
            // On utilise les Smart Constructors (and, or, add...) pour les simplifications
            ExprEntryKind::And => self.and(children),
            ExprEntryKind::Or => self.or(children),
            ExprEntryKind::Arithmetic(op) => self.arithmetic(op, children),

            // --- 2. Opérateurs Binaires (2 enfants) ---
            ExprEntryKind::Imply => self.imply(children[0], children[1]),
            ExprEntryKind::When => self.when(children[0], children[1]),
            ExprEntryKind::Comparison(op) => self.comparison(op, children[0], children[1]),
            ExprEntryKind::Assignment(op) => self.assignment(op, children[0], children[1]),

            // TaskOrderingConstraint n'a pas de helper public direct prenant 2 IDs,
            // on utilise l'internement direct comme dans ton API.
            ExprEntryKind::TaskOrderingConstraint(op) => {
                self.intern(ExprEntryKind::TaskOrderingConstraint(op), children)
            }

            // --- 3. Opérateurs Unaires (1 enfant) ---
            ExprEntryKind::Not => self.not(children[0]),
            ExprEntryKind::AtStart => self.at_start(children[0]),
            ExprEntryKind::AtEnd => self.at_end(children[0]),
            ExprEntryKind::Overall => self.overall(children[0]),

            // Pour Preference, IsViolated et Metric, l'API utilise des helpers
            // qui créent des feuilles (symboles). Ici, children contient déjà les IDs.
            ExprEntryKind::Preference | ExprEntryKind::IsViolated | ExprEntryKind::Metric(_) => {
                self.intern(kind, children)
            }

            // --- 4. Quantificateurs (Données + 1 enfant) ---
            ExprEntryKind::Forall(vars) => self.forall(vars, children[0]),
            ExprEntryKind::Exists(vars) => self.exists(vars, children[0]),

            // --- 5. Noeuds avec Squelettes (AtomicFormula, Function, Task) ---
            // Crucial : children[0] est le symbole, children[1..] sont les arguments.
            ExprEntryKind::AtomicFormula(skel) => {
                self.intern(ExprEntryKind::AtomicFormula(skel), children)
            }
            ExprEntryKind::Function(skel) => self.intern(ExprEntryKind::Function(skel), children),
            ExprEntryKind::Task(skel) => self.intern(ExprEntryKind::Task(skel), children),

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
            | ExprEntryKind::Length => self.intern(kind, children),

            // --- 7. Cas Terminaux (Feuilles) ---
            // Object, Variable, symbols, Number, TotalTime...
            leaf_kind => self.intern(leaf_kind, &[]),
        }
    }

    /// (==) Returns true if two values are nearly equal.
    #[inline]
    pub fn is_eq(&self, a: f64, b: f64) -> bool {
        (a - b).abs() <= f64::EPSILON
    }

    /// (>) Returns true if `a` is significantly greater than `b`.
    #[inline]
    pub fn is_gt(&self, a: f64, b: f64) -> bool {
        a > b + f64::EPSILON
    }

    /// (<) Returns true if `a` is significantly less than `b`.
    #[inline]
    pub fn is_lt(&self, a: f64, b: f64) -> bool {
        a < b - f64::EPSILON
    }

    /// (>=) Returns true if `a` is greater than or nearly equal to `b`.
    #[inline]
    pub fn is_ge(&self, a: f64, b: f64) -> bool {
        a >= b - f64::EPSILON
    }

    /// (<=) Returns true if `a` is less than or nearly equal to `b`.
    #[inline]
    pub fn is_le(&self, a: f64, b: f64) -> bool {
        a <= b + f64::EPSILON
    }

    /// (== 0) Special check for zero-equivalence.
    #[inline]
    pub fn is_zero(&self, a: f64) -> bool {
        a.abs() <= f64::EPSILON
    }

    /// (< 0) Check if significantly negative.
    #[inline]
    pub fn is_neg(&self, a: f64) -> bool {
        a < -f64::EPSILON
    }

    /// Sorts a given buffer by `ExprId` to enable deduplication and canonicalization.
    ///
    /// This is a specialized implementation of the Heapsort algorithm. Sorting operands
    /// by their unique internal identifiers ensures that expressions are stored
    /// in a consistent (canonical) order.
    ///
    /// # Arguments
    /// * `buffer` - A mutable reference to the buffer to sort (primary or secondary).
    /// * `len` - The number of elements in the buffer to sort.
    pub(crate) fn sort_buffer_by_id(buffer: &mut Vec<ExprId>, len: usize) {
        if len <= 1 {
            return;
        }

        // Phase 1: Build a max-heap
        for start in (0..len / 2).rev() {
            Self::sift_down_by_id(buffer, start, len);
        }

        // Phase 2: Extract elements
        for end in (1..len).rev() {
            buffer.swap(0, end);
            Self::sift_down_by_id(buffer, 0, end);
        }
    }

    /// Restores the max-heap property for a specific buffer.
    fn sift_down_by_id(buffer: &mut Vec<ExprId>, mut root: usize, end: usize) {
        while root * 2 + 1 < end {
            let mut child = root * 2 + 1;

            if child + 1 < end && buffer[child] < buffer[child + 1] {
                child += 1;
            }

            if buffer[root] < buffer[child] {
                buffer.swap(root, child);
                root = child;
            } else {
                break;
            }
        }
    }

    /// Extracts a literal floating-point value from an expression ID if it points to a number.
    ///
    /// This is a convenience helper that traverses the `ExprStore` to check if a specific
    /// [`ExprId`] corresponds to a [`ExprEntryKind::Number`].
    ///
    /// # Arguments
    ///
    /// * `id` - The [`ExprId`] of the expression to inspect.
    ///
    /// # Returns
    ///
    /// * `Some(f64)` - The inner value if the expression is a numeric literal.
    /// * `None` - If the expression does not exist or is not a number (e.g., it's a variable or another operation).
    pub(crate) fn get_number(&self, id: ExprId) -> Option<f64> {
        self.get(id).and_then(|n| {
            if let ExprEntryKind::Number(v) = n.kind() {
                Some(v.into_inner())
            } else {
                None
            }
        })
    }
}
