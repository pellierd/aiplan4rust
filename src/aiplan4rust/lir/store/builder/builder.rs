use crate::aiplan4rust::lir::store::error::StorerError;
use crate::aiplan4rust::lir::store::{ExprEntryKind, ExprId, ExprNodeRef, ExprStore};

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
            ExprEntryKind::Assignment(op) => self.assign_expr(op, children[0], children[1]),

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
}
