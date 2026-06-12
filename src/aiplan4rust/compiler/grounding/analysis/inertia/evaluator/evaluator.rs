use crate::aiplan4rust::compiler::grounding::analysis::inertia::evaluator::InertiaEvaluatorError;
use crate::aiplan4rust::compiler::grounding::analysis::inertia::table::InertiaTable;
use crate::aiplan4rust::compiler::grounding::binding::evaluator::evaluator::ExprEvaluator;
use crate::aiplan4rust::compiler::grounding::binding::evaluator::ExprConstant;
use crate::aiplan4rust::compiler::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::compiler::lir::expr::Expr;
use crate::aiplan4rust::compiler::lir::expr::{ExprKind, ExprNode, ExprStore};
use crate::aiplan4rust::compiler::lir::problem::skeleton::{
    AtomicFormulaSkeleton, AtomicFunctionSkeleton,
};
use crate::aiplan4rust::compiler::lir::problem::LiftedProblem;
use crate::aiplan4rust::compiler::syntax::ast::arena::ArenaNode;
use crate::aiplan4rust::support::lang::{AtomSkeletonId, FunctionSkeletonId, ObjectId};
use smallvec::SmallVec;
use std::collections::HashMap;

const DEFAULT_MAX_ARITY: usize = 15;
const DEFAULT_MAX_PROJ: usize = 3;
const ARGUMENT_BUFFER_SIZE: usize = 8;

// Buffer pour les arguments des prédicats/fonctions.
/// 8 éléments sur la pile suffisent pour presque tous les domaines.
pub(crate) type ArgumentBuffer = SmallVec<[ObjectId; ARGUMENT_BUFFER_SIZE]>;

#[derive(Debug)]
pub struct InertiaEvaluator<'a> {
    pub(super) counting_predicates:
        HashMap<AtomSkeletonId, HashMap<u16, HashMap<Box<[ObjectId]>, usize>>>,
    pub(super) static_functions:
        HashMap<FunctionSkeletonId, HashMap<u16, HashMap<Box<[ObjectId]>, ExprConstant>>>,
    pub(super) inertia: &'a InertiaTable,

    // --- RÉFÉRENCES EMPRUNTÉES (Context) ---
    pub(super) predicate_defs: Box<[AtomicFormulaSkeleton]>,
    pub(super) function_defs: Box<[AtomicFunctionSkeleton]>, // Pour les signatures des fonctions
    pub(super) value_registry: &'a ValueRegistry,

    pub(super) consensus_values: HashMap<FunctionSkeletonId, ExprConstant>,

    pub(super) max_arity: usize,
    pub(super) max_proj: usize,
}
impl<'a> InertiaEvaluator<'a> {
    pub fn build(
        predicate_defs: &[AtomicFormulaSkeleton],
        function_defs: &[AtomicFunctionSkeleton],
        init: Expr<'_>,
        inertia: &'a InertiaTable,
        value_registry: &'a ValueRegistry,
        max_arity: usize,
        max_proj: usize,
    ) -> Result<Self, InertiaEvaluatorError> {
        // 1. Initialisation du registre avec copie des définitions
        let mut registry = Self {
            predicate_defs: predicate_defs.to_vec().into_boxed_slice(),
            function_defs: function_defs.to_vec().into_boxed_slice(),
            inertia,
            value_registry,
            counting_predicates: HashMap::new(),
            static_functions: HashMap::new(),
            consensus_values: Default::default(),
            max_arity,
            max_proj,
        };

        // 2. Parcours de l'état initial (init)
        // tree_preorder renvoie (ExprId, depth, is_last, &ExprEntry)
        let mut it = init.store().tree_preorder(init.root_id());

        while let Some((id, _, _, entry)) = it.next() {
            match entry.kind() {
                // On traite les faits atomiques et les assignations de fonctions
                ExprKind::AtomicFormula(_) | ExprKind::Comparison(_) => {
                    // On encapsule l'entrée courante dans un ExprNodeRef pour faciliter le traitement
                    let node_ref = ExprNode::new(id, entry);

                    registry.process_init(node_ref, init.store())?;

                    // On saute les enfants car process_init s'occupe de descendre
                    // pour extraire les arguments et symboles.
                    it.skip_children(entry.children().len());
                }

                // En PDDL, l'init est une liste de faits positifs.
                // On ignore les négations et les TILs (gérés par la table d'inertie).
                ExprKind::Not | ExprKind::TimedInitialLiteral => {
                    it.skip_children(entry.children().len());
                }

                _ => {}
            }
        }

        Ok(registry)
    }

    // --- Internal Helpers ---

    /// Retourne vrai si la formule atomique ou le terme de fonction est totalement instancié (aucune variable).
    ///
    /// Un terme "grounded" peut être simplifié en une constante s'il est présent
    /// dans le registre de l'état initial.
    pub(crate) fn all_args_grounded(&self, node: ExprNode<'_>, store: &ExprStore) -> bool {
        let children = node.children();

        // RÈGLE : children[0] est le symbole.
        // S'il n'y a qu'un enfant ou aucun, il n'y a pas d'arguments, donc c'est "grounded" par défaut.
        if children.len() <= 1 {
            return true;
        }

        // On vérifie tous les enfants à partir de l'index 1 (les arguments)
        for &child_id in children.iter().skip(1) {
            // Accès sécurisé au old
            if let Ok(child_entry) = store.fetch(child_id) {
                // Si l'un des arguments est une variable (non encore instanciée),
                // l'expression n'est pas "grounded".
                if let ExprKind::Variable(_) = child_entry.kind() {
                    return false;
                }
            }
        }

        true
    }

    /// Checks if the problem's predicates and functions exceed the evaluator's capacity.
    fn check_limits(
        max_arity: usize,
        problem: &LiftedProblem,
    ) -> Result<(), InertiaEvaluatorError> {
        for (i, p) in problem.predicate_defs().iter().enumerate() {
            if p.arity() > max_arity {
                return Err(InertiaEvaluatorError::predicate_arity_too_high(
                    AtomSkeletonId::from(i),
                    p.arity(),
                ));
            }
        }
        for (i, f) in problem.function_defs().iter().enumerate() {
            if f.arity() > max_arity {
                return Err(InertiaEvaluatorError::function_arity_too_high(
                    FunctionSkeletonId::from(i),
                    f.arity(),
                ));
            }
        }
        Ok(())
    }

    /// Extrait le masque d'instanciation et les constantes associées d'un atome.
    ///
    /// Cette fonction implémente la logique de la **Définition 8** du papier IPP :
    /// `C(a) := {i | ai est une constante}`.
    ///
    /// # Logique du Papier IPP
    ///
    /// Selon le papier, pour évaluer $N(p, \vec{a})$, nous devons identifier quelles positions
    /// du vecteur d'arguments $\vec{a}$ sont occupées par des constantes afin de choisir
    /// la table de comptage appropriée $T(p, C)$.
    ///
    /// - **Le Masque (`u16`)** : Représente l'ensemble $C$. Chaque bit correspond à une position.
    ///   Si l'argument à la position $i$ est une constante, le bit correspondant est mis à 1.
    ///   L'implémentation utilise un encodage *Big Endian* (le premier argument est le bit de poids fort).
    /// - **Le Buffer (`ArgumentBuffer`)** : Implémente la restriction $\vec{a}|_{C(\vec{a})}$ (Définition 7).
    ///   Il contient uniquement les identifiants des objets constants, en préservant leur ordre
    ///   relatif, tout en ignorant (sautant) les variables.
    ///
    /// # Gestion de l'ADL et de l'Instanciation Partielle
    ///
    /// Conformément à la **Section 3.2**, cette fonction est "variable-aware".
    /// Si un argument est une `Variable`, son bit reste à `0` dans le masque et il n'est pas
    /// ajouté au buffer. Cela permet d'obtenir le compte $N$ pour n'importe quelle
    /// combinaison de constantes, ce qui est le cœur de la simplification atomique.
    ///
    /// # Sécurité Arithmétique
    ///
    /// Une garde est présente pour `children.len() <= 1` (atome sans arguments ou symbole seul),
    /// garantissant que le calcul de `args.len() - 1 - i` ne provoque jamais de sous-dépassement
    /// (*underflow*) sur les types non signés.
    /// Extrait le masque d'instanciation et les constantes associées d'un atome ou d'une fonction.
    ///
    /// Implémente la Définition 8 du papier IPP : `C(a) := {i | ai est une constante}`.
    pub(crate) fn extract_mask_dynamic(
        &self,
        node: ExprNode<'_>,
        store: &ExprStore,
        buffer: &mut ArgumentBuffer,
    ) -> u16 {
        buffer.clear();
        let children = node.children();

        // RÈGLE : children[0] est le symbole.
        // S'il n'y a qu'un enfant ou aucun, il n'y a pas d'arguments.
        if children.len() <= 1 {
            return 0;
        }

        // On isole les arguments pour simplifier le calcul du masque
        let args = &children[1..];
        let n_args = args.len();
        let mut mask = 0u16;

        for (i, &arg_id) in args.iter().enumerate() {
            if let Ok(arg_entry) = store.fetch(arg_id) {
                match arg_entry.kind() {
                    // Si l'argument est un objet constant
                    ExprKind::Object(obj_id) => {
                        // Encodage Big Endian :
                        // i=0 (1er arg) -> bit (n_args - 1)
                        // i=(n_args-1)  -> bit 0
                        mask |= 1 << (n_args - 1 - i);
                        buffer.push(*obj_id);
                    }
                    // Si c'est une Variable, on ne fait rien (le bit reste à 0).
                    // C'est le cœur de l'instanciation partielle d'IPP.
                    ExprKind::Variable(_) => {}

                    _ => {}
                }
            }
        }
        mask
    }
}

impl<'a> ExprEvaluator for InertiaEvaluator<'a> {
    fn evaluate(&self, expr: Expr<'_>) -> Option<ExprConstant> {
        // 1. On récupère directement le node_ref via le old
        // Si old.fetch(id) renvoie déjà un ExprNodeRef, on l'utilise tel quel.
        let node_ref = expr.store().fetch(expr.root_id()).ok()?;

        // On récupère le old pour les appels internes
        let store = expr.store();

        // 2. Préparation du buffer
        let mut buffer = ArgumentBuffer::new();

        // 3. Dispatch (on utilise node_ref directement)
        let result = match node_ref.kind() {
            ExprKind::AtomicFormula(_) => self
                .evaluate_predicate_internal(node_ref, store, &mut buffer)
                .ok()
                .flatten()
                .map(ExprConstant::Boolean),

            ExprKind::Function(_) => self
                .evaluate_function_internal(node_ref, store, &mut buffer)
                .ok()
                .flatten(),

            _ => None,
        };

        if let Some(val) = &result {
            println!(
                "[Inertia] Simplified {:?} (ID: {:?}) -> {:?}",
                node_ref.kind(),
                expr.root_id(),
                val
            );
        }

        result
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::aiplan4rust::compiler::lir::problem::skeleton::AtomicFormulaSkeleton;
    use crate::aiplan4rust::support::lang::{FunctionSymbolId, PredicateSymbolId, TypedList};

    // Local test extension to provide the missing `mock` constructor for InertiaEvaluator
    impl<'a> InertiaEvaluator<'a> {
        /// Mock disponible pour les tests unitaires des fichiers frères
        pub(crate) fn mock(
            predicate_defs: &[AtomicFormulaSkeleton],
            function_defs: &[AtomicFunctionSkeleton],
            value_registry: &'a ValueRegistry,
            inertia: &'a InertiaTable,
        ) -> Self {
            Self {
                predicate_defs: predicate_defs.to_vec().into_boxed_slice(),
                function_defs: function_defs.to_vec().into_boxed_slice(),
                inertia,
                value_registry,
                counting_predicates: HashMap::new(),
                static_functions: HashMap::new(),
                consensus_values: Default::default(),
                max_arity: 15,
                max_proj: 3,
            }
        }

        /// Helper de mock spécifique permettant de configurer finement les limites de projection
        pub(crate) fn mock_with_config(
            p_defs: &[AtomicFormulaSkeleton],
            f_defs: &[AtomicFunctionSkeleton],
            v_reg: &'a ValueRegistry,
            i_table: &'a InertiaTable,
            max_arity: usize,
            max_proj: usize,
        ) -> Self {
            Self {
                predicate_defs: p_defs.to_vec().into_boxed_slice(),
                function_defs: f_defs.to_vec().into_boxed_slice(),
                inertia: i_table,
                value_registry: v_reg,
                counting_predicates: HashMap::new(),
                static_functions: HashMap::new(),
                consensus_values: HashMap::new(),
                max_arity,
                max_proj,
            }
        }
    }

    /// Helper function to generate mock predicate definitions for testing purposes.
    ///
    /// Creates a vector of `AtomicFormulaSkeleton` instances with sequential `PredicateSymbolId`s
    /// and empty parameter lists.
    pub(crate) fn mock_predicate_defs(count: usize) -> Vec<AtomicFormulaSkeleton> {
        (0..count)
            .map(|i| AtomicFormulaSkeleton::new(PredicateSymbolId::from(i), TypedList::new()))
            .collect()
    }

    /// Helper function to generate mock function definitions for testing purposes.
    ///
    /// Creates a vector of `AtomicFunctionSkeleton` instances with sequential `FunctionSymbolId`s,
    /// empty parameter lists, and default return types.
    pub(crate) fn mock_function_defs(count: usize) -> Vec<AtomicFunctionSkeleton> {
        (0..count)
            .map(|i| {
                // Adjust here if your AtomicFunctionSkeleton::new requires a different return type structure
                AtomicFunctionSkeleton::new(
                    FunctionSymbolId::from(i),
                    TypedList::new(),
                    Default::default(),
                )
            })
            .collect()
    }
}
