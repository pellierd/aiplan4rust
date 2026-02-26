use smallvec::SmallVec;
use std::collections::HashMap;
use ordered_float::OrderedFloat;
use crate::aiplan4rust::arena::{ArenaNode, NodeId};
use crate::aiplan4rust::lang::{AtomSkeletonId, FunctionSkeletonId, ObjectId};
use crate::aiplan4rust::grounding::analysis::inertia::registry::{InertiaRegistryBuilder, InertiaRegistryError};
use crate::aiplan4rust::grounding::analysis::inertia::table::InertiaTable;
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lir::expr::{Expr, ExprKind, ExprNode};
use crate::aiplan4rust::lir::expr::ops::{StaticEvaluator, StaticValue};
use crate::aiplan4rust::lir::problem::atomic_skeleton::{AtomicFormulaSkeleton, AtomicFunctionSkeleton};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::tree::Node;

const DEFAULT_MAX_ARITY: usize = 15;
const DEFAULT_MAX_PROJ: usize = 3;
const ARGUMENT_BUFFER_SIZE: usize = 8;

// Buffer pour les arguments des prédicats/fonctions.
/// 8 éléments sur la pile suffisent pour presque tous les domaines.
type ArgumentBuffer = SmallVec<[ObjectId; ARGUMENT_BUFFER_SIZE]>;

#[derive(Debug)]
pub struct InertiaRegistry<'a> {
    counting_predicates: HashMap<AtomSkeletonId, HashMap<u16, HashMap<Box<[ObjectId]>, usize>>>,
    static_functions: HashMap<FunctionSkeletonId, HashMap<u16, HashMap<Box<[ObjectId]>, StaticValue>>>,
    inertia: &'a InertiaTable,

    // --- RÉFÉRENCES EMPRUNTÉES (Context) ---
    predicate_defs: &'a [AtomicFormulaSkeleton],
    function_defs: &'a [AtomicFunctionSkeleton], // Pour les signatures des fonctions
    value_registry: &'a ValueRegistry,

    consensus_values: HashMap<FunctionSkeletonId, StaticValue>,

    max_arity: usize,
    max_proj: usize,
}
impl<'a> InertiaRegistry<'a> {
    /// Interface simplifiée : utilise les valeurs par défaut (Arity 15, Proj 3).
    /// C'est celle que tu utiliseras 90% du temps.
    pub fn build(
        problem: &'a LiftedProblem,
        inertia: &'a InertiaTable,
        value_registry: &'a ValueRegistry,
    ) -> Result<Self, InertiaRegistryError> {
        // On délègue à la fonction expert avec les constantes par défaut
        Self::build_with_config(problem, inertia, value_registry, DEFAULT_MAX_ARITY, DEFAULT_MAX_PROJ)
    }

    /// Point d'entrée pour une configuration fluide (Pattern Builder).
    pub fn builder(problem: &'a LiftedProblem, inertia: &'a InertiaTable, value_registry: &'a ValueRegistry) -> InertiaRegistryBuilder<'a> {
        InertiaRegistryBuilder::new(problem, inertia, value_registry)
    }

    /// Interface "Expert" : permet de régler précisément les limites.
    /// Utile pour les tests ou les domaines avec des prédicats hors-normes.
    pub(crate) fn build_with_config(
        problem: &'a LiftedProblem,
        inertia: &'a InertiaTable,
        value_registry: &'a ValueRegistry,
        max_arity: usize,
        max_proj: usize,
    ) -> Result<Self, InertiaRegistryError> {
        // 1. Vérifications d'arité dynamiques
        Self::check_limits(max_arity, problem)?;

        let mut registry = Self {
            counting_predicates: HashMap::new(),
            static_functions: HashMap::new(),
            inertia,
            predicate_defs: problem.predicate_defs(),
            function_defs: problem.function_defs(),
            consensus_values: HashMap::new(),
            value_registry,
            max_arity,
            max_proj,
        };

        let init = problem.init();
        let mut iter = init.preorder().values();
        while let Some(node) = iter.next() {
            match node.kind() {
                ExprKind::AtomicFormula | ExprKind::FComp => {
                    registry.process_init(node, init)?;
                    iter.skip_subtree();

                }
                ExprKind::Not => {
                    // C'est un fait négatif (not (at x y))
                    // On skip TOUT le sous-arbre (incluant l'AtomicFormula à l'intérieur)
                    // car un fait négatif ne doit pas être compté dans N(p, a)
                    iter.skip_subtree();
                }
                _ => {}
            }
        }
        Ok(registry)
    }

    fn process_init(&mut self, node: &ExprNode, init: &Expr) -> Result<(), InertiaRegistryError> {
        match node.kind() {
            ExprKind::AtomicFormula => self.process_predicate(node, init),
            ExprKind::FComp => self.process_function(node, init),
            _ => Ok(()),
        }
    }

    fn process_predicate(&mut self, node: &ExprNode, init: &Expr) -> Result<(), InertiaRegistryError> {
        let children = node.children();

        // 1. On récupère l'ID du SQUELETTE (la définition du prédicat)
        // C'est cet ID qui permet de savoir si "at(truck, place)" est un prédicat d'inertie.
        let skeleton_id = node.try_atom_skeleton()?;

        if self.inertia.is_predicate_positive(skeleton_id)? {
            // 2. L'arité réelle des données (les arguments du fait initial)
            // Puisque le premier enfant est le "symbole" (le nom), on l'exclut.
            let arity = children.len().saturating_sub(1);

            if arity == 0 {
                // Prédicat propositionnel (arité 0 dans la définition)
                self.generate_predicate_masks(skeleton_id, 0, &[]);
                return Ok(());
            }

            // 3. Extraction des arguments (les constantes)
            let mut args = Vec::with_capacity(arity);

            // On commence à 1 car children[0] est le symbole (le nom),
            // pas une donnée membre de l'instance du prédicat.
            for &arg_id in &children[1..] {
                let arg_node = init.try_node(arg_id)?;
                // On récupère la valeur concrète (ex: l'ID de l'objet 'truck1')
                args.push(arg_node.try_constant()?);
            }

            // 4. On lie la définition (skeleton_id) aux valeurs concrètes (args)
            self.generate_predicate_masks(skeleton_id, arity, &args);
        }
        Ok(())
    }

    fn process_function(&mut self, node: &ExprNode, init: &Expr) -> Result<(), InertiaRegistryError> {
        let children = node.children();
        // 1. On récupère la définition de la fonction (le squelette)
        let func_id = node.try_function_skeleton()?;

        if self.inertia.is_function_positive(func_id)? {
            // 2. Dans un FComp (=), le premier enfant (children[0]) est le BasicFunctionTerm
            let func_term_node = init.try_node(children[0])?;
            let func_children = func_term_node.children();

            // L'arité exclut le symbole de la fonction (le nom) à l'index 0
            let arity = func_children.len().saturating_sub(1);

            // 3. Extraction des arguments de la fonction (ex: le 'x' dans '(f x)')
            let mut args = Vec::with_capacity(arity);
            for &arg_id in &func_children[1..] {
                args.push(init.try_node(arg_id)?.try_constant()?);
            }

            // 4. Extraction de la valeur (le membre de droite du '=' : children[1])
            let val_node = init.try_node(children[1])?;
            let value = if let Ok(num) = val_node.try_float() {
                StaticValue::Number(num)
            } else {
                StaticValue::Object(val_node.try_constant()?)
            };

            // 5. Enregistrement pour l'analyse d'inertie
            self.generate_function_masks(func_id, arity, &args, value);
        }
        Ok(())
    }

    /// Évalue une formule atomique selon les règles de simplification du papier IPP (Section 3.2).
    ///
    /// Cette fonction implémente la Définition 6 (Atomic Simplifications) et le Théorème 1
    /// pour décider si un prédicat inerte peut être remplacé par TRUE ou FALSE,
    /// même s'il contient des variables (instanciation partielle).
    fn evaluate_predicate_internal(
        &self,
        node_id: NodeId,
        expr: &Expr,
        buffer: &mut ArgumentBuffer,
    ) -> Result<Option<bool>, InertiaRegistryError> {
        let node = expr.try_node(node_id)?;
        let pred_id = node.try_atom_skeleton()?;

        // Définition : Un prédicat est inerte s'il n'apparaît dans aucun effet d'opérateur.
        // - Positive Inertia : N'apparaît dans aucun effet positif (ne peut pas devenir VRAI s'il est FAUX).
        // - Negative Inertia : N'apparaît dans aucun effet négatif (ne peut pas devenir FAUX s'il est VRAI).
        let is_negative = self.inertia.is_predicate_negative(pred_id)?;
        let is_positive = self.inertia.is_predicate_positive(pred_id)?;


        // Si le prédicat n'est pas inerte (fluents), on ne peut rien simplifier à ce stade.
        if !is_negative && !is_positive {
            return Ok(None);
        }

        // --- ÉTAPE A : Calcul de N(p, ~a) (Définition 5) ---
        // N(p, ~a) est le nombre d'instances dans l'état initial I qui unifient avec l'atome partiel (p, ~a).
        // Le papier utilise des tables pré-calculées (Section 3.3) pour obtenir ce compte en O(arity).
        let mask = self.extract_mask_dynamic(node, expr, buffer);
        let n_limit = buffer.len().min(self.max_proj);
        let lookup_slice = &buffer[..n_limit];

        let n_p_a = self.counting_predicates.get(&pred_id)
            .and_then(|masks| masks.get(&mask))
            .and_then(|entries| entries.get(lookup_slice))
            .copied()
            .unwrap_or(0);

        // --- ÉTAPE B : Application des règles de simplification (Définition 6) ---

        // RÈGLE 1 : "If p is a positive inertia and N(p, ~a) = 0 then (p, ~a) is simplified to FALSE."
        // Justification (Théorème 1.1) : Comme p est inerte positif, aucune action ne peut l'ajouter.
        // Si aucune instance n'existe initialement (N=0), aucune ne sera jamais vraie dans les états atteignables.
        if is_positive && n_p_a == 0 {
            return Ok(Some(false));
        }

        // RÈGLE 2 : "If p is a negative inertia and N(p, ~a) = MAX(p, ~a) then (p, ~a) is simplified to TRUE."
        // MAX(p, ~a) (Définition 5) est le nombre total de combinaisons typées possibles pour les variables de ~a.
        if is_negative {
            let max_p_a = self.calculate_max_instances(node, expr);

            // Justification (Théorème 1.2) : Si toutes les instances possibles sont déjà dans l'état initial
            // et que p est inerte négatif (aucune action ne peut le supprimer), alors toutes les
            // instances possibles de (p, ~a) seront VRAIES dans tous les états atteignables.
            println!("DEBUG: Predicate {:?}", pred_id);
            println!("DEBUG: n_p_a = {}, max_p_a = {}, mask = {}", n_p_a, max_p_a, mask);
            if n_p_a == max_p_a {
                return Ok(Some(true));
            }
        }

        // --- CAS PARTICULIER : Atome totalement instantié (Grounded) ---
        // Pour un atome sans variables, MAX(p, ~a) est toujours égal à 1.
        if self.all_args_grounded(node, expr) {
            if is_positive {
                // Si N=1, l'atome est présent initialement. Comme il est inerte négatif (par défaut
                // si on ne le précise pas ou si testé ici), il reste VRAI.
                // Si N=0, il a déjà été capturé par la Règle 1.
                return Ok(Some(n_p_a > 0));
            }

            // Note technique : Pour une inertie purement négative (sans être positive),
            // si N=0, on ne peut PAS simplifier à FALSE car une action pourrait
            // techniquement l'ajouter si elle n'est pas inerte positive.
        }

        // "In all other cases (p, ~a) cannot (yet) be simplified and remains in the formula tree."
        Ok(None)
    }

    /// Calcule MAX(p, ~a) selon la Définition 5 du papier IPP.
    /// MAX est le nombre de toutes les instances terrestres (ground instances)
    /// cohérentes avec les types qui unifient avec le vecteur d'arguments ~a.
    fn calculate_max_instances(&self, node: &ExprNode, expr: &Expr) -> usize {
        let mut max_val: usize = 1;
        let children = node.children();

        if let Ok(pred_id) = node.try_atom_skeleton() {
            // On récupère la signature (types des arguments) définie au build
            if let Some(arg_types) = self.predicate_defs.get(pred_id.as_usize()).map(|s| s.parameters()) {

                // On itère sur les positions i de 1 à n
                for (i, &child_id) in children[1..].iter().enumerate() {
                    if let Ok(child_node) = expr.try_node(child_id) {
                        // V(~a) est l'ensemble des positions occupées par des variables.
                        // Pour chaque i appartenant à V(~a), on multiplie par |dom(Ti)|.
                        if child_node.kind() == ExprKind::Variable {
                            let type_id = arg_types[i].ty();
                            let domain_size = self.value_registry.get_type_domain(type_id).cardinality();
                            max_val *= domain_size;
                        }
                    }
                }
            }
        }
        // Si l'atome est totalement instancié, V(~a) est vide, le produit vide vaut 1.
        max_val
    }

    fn evaluate_function_internal(
        &self,
        node_id: NodeId,
        expr: &Expr,
        buffer: &mut ArgumentBuffer,
    ) -> Result<Option<StaticValue>, InertiaRegistryError> {
        let node = expr.try_node(node_id)?;
        let func_id = node.try_function_skeleton()?;

        // 1. Check d'inertie : Si la fonction peut changer, on ne simplifie rien. 🧊
        if !self.inertia.is_function_positive(func_id)? {
            return Ok(None);
        }

        // 2. Extraction du masque et des arguments
        let mask = self.extract_mask_dynamic(node, expr, buffer);
        let n_limit = buffer.len().min(self.max_proj);
        let lookup_slice = &buffer[..n_limit];

        // 3. Recherche de la valeur injectée dans le registre 🔍
        let mut value = self.static_functions.get(&func_id)
            .and_then(|masks| masks.get(&mask))
            .and_then(|entries| entries.get(lookup_slice))
            .copied();

        // 4. Logique de décision et Fallback PDDL
        if self.all_args_grounded(node, expr) {
            if value.is_none() {
                // Si aucune valeur n'est trouvée, on vérifie le type de retour
                let def = &self.function_defs[func_id.as_usize()];

                // Standard PDDL : une fonction numérique non initialisée vaut 0.0
                if def.ty().is_number() {
                    value = Some(StaticValue::Number(OrderedFloat(0.0)));
                } else {
                    // Pour les fonctions d'objets, on laisse None (undefined)
                    // ON doit lancer une erreur ici si on a effectuer le flattening de obejct-flent il ne devarit pas en avoir.
                    value = None;
                }
                // Note : Pour les fonctions d'objets, on laisse None (undefined)
            }
            return Ok(value);
        } else {
            // Cas avec Variables : On ne simplifie pas sans analyse d'unanimité
            // Amélioration possible mais non implementer
            Ok(None)
        }
    }

    // --- Internal Helpers ---

    fn all_args_grounded(&self, node: &ExprNode, expr: &Expr) -> bool {
        let children = node.children();

        // Si on n'a qu'un seul enfant (le symbole), il n'y a pas d'arguments.
        // Pas d'arguments = pas de variables = grounded.
        if children.len() <= 1 {
            return true;
        }

        // On vérifie tous les enfants à partir de l'index 1 (les arguments)
        for &child_id in &children[1..] {
            if let Ok(child_node) = expr.try_node(child_id) {
                // Si l'un des arguments est une variable (non instanciée),
                // on ne peut pas simplifier.
                if child_node.kind() == ExprKind::Variable {
                    return false;
                }
            }
        }

        true
    }

    /// Checks if the problem's predicates and functions exceed the registry's capacity.
    fn check_limits(max_arity: usize, problem: &LiftedProblem) -> Result<(), InertiaRegistryError> {
        for (i, p) in problem.predicate_defs().iter().enumerate() {
            if p.arity() > max_arity {
                return Err(InertiaRegistryError::predicate_arity_too_high(AtomSkeletonId::from(i), p.arity()));
            }
        }
        for (i, f) in problem.function_defs().iter().enumerate() {
            if f.arity() > max_arity {
                return Err(InertiaRegistryError::function_arity_too_high(FunctionSkeletonId::from(i), f.arity()));
            }
        }
        Ok(())
    }

    fn extract_mask_dynamic(
        &self,
        node: &ExprNode,
        expr: &Expr,
        buffer: &mut ArgumentBuffer,
    ) -> u16 {
        buffer.clear();
        let children = node.children();
        if children.len() <= 1 { return 0; }

        let args = &children[1..];
        let mut mask = 0u16;

        for (i, &arg_id) in args.iter().enumerate() {
            if let Some(obj) = expr.try_node(arg_id).ok().and_then(|n| n.try_constant().ok()) {
                mask |= 1 << (args.len() - 1 - i);
                buffer.push(obj);
            }
        }
        mask
    }
    pub fn generate_predicate_masks(&mut self, key: AtomSkeletonId, arity: usize, args: &[ObjectId]) {
        // 1. Garde contre l'arité 0 et les erreurs de calcul potentielles
        if arity == 0 {
            let mask_table = self.counting_predicates.entry(key).or_default();
            let entries = mask_table.entry(0).or_default();
            let count = entries.entry(Box::from([])).or_insert(0);
            *count += 1;
            return;
        }

        // 2. Génération des masques (2^arity combinaisons)
        for mask in 0..(1 << arity) {
            // Filtre de projection : évite l'explosion mémoire si trop de variables sont fixées
            let bit_count = (mask as u32).count_ones() as usize;
            if mask != 0 && bit_count > self.max_proj {
                continue;
            }

            let mut combo: SmallVec<[ObjectId; 8]> = SmallVec::new();

            // 3. Construction de la clé de manière "Safe"
            // On itère sur les arguments et on calcule la position du bit
            // de façon à ce que le premier argument soit le bit le plus fort (Big Endian)
            for (i, &obj) in args.iter().enumerate() {
                // Puisque arity >= 1 et i < arity, (arity - 1 - i) ne peut pas être négatif
                let bit_pos = arity - 1 - i;
                if (mask & (1 << bit_pos)) != 0 {
                    combo.push(obj);
                }
            }

            // 4. Insertion dans le registre
            let mask_table = self.counting_predicates.entry(key).or_default();
            let entries = mask_table.entry(mask as u16).or_default();

            let count = entries.entry(Box::from(combo.as_slice())).or_insert(0);
            *count += 1;
        }
    }

    pub fn generate_function_masks(
        &mut self,
        key: FunctionSkeletonId,
        arity: usize,
        args: &[ObjectId],
        val: StaticValue,
    ) {
        // Cas arité 0 : un seul masque possible (0)
        if arity == 0 {
            let mask_table = self.static_functions.entry(key).or_default();
            let entries = mask_table.entry(0).or_default();
            entries.insert(Box::from([]), val);
            return;
        }

        // On itère sur tous les masques possibles pour les arguments de la fonction
        for mask in 0..(1 << arity) {
            // --- FILTRE DE PROJECTION ---
            let bit_count = (mask as u32).count_ones() as usize;
            if mask != 0 && bit_count > self.max_proj {
                continue;
            }

            // --- CONSTRUCTION DE LA CLÉ ---
            let mut combo: SmallVec<[ObjectId; 8]> = SmallVec::new();
            for i in 0..arity {
                // Logique Big Endian identique aux prédicats
                let bit_pos = arity - 1 - i;
                if (mask & (1 << bit_pos)) != 0 {
                    // On vérifie que l'argument existe bien à cet index
                    // (Sécurité si args est plus court que l'arité théorique)
                    if let Some(&obj) = args.get(i) {
                        combo.push(obj);
                    }
                }
            }

            // --- STOCKAGE ---
            let mask_table = self.static_functions.entry(key).or_default();
            let entries = mask_table.entry(mask as u16).or_default();

            // On insère ou met à jour la valeur statique
            if let Some(existing_val) = entries.get_mut(combo.as_slice()) {
                *existing_val = val;
            } else {
                entries.insert(Box::from(combo.as_slice()), val);
            }
        }
    }

}

impl<'a> StaticEvaluator for InertiaRegistry<'a> {
    fn evaluate(&self, node_id: NodeId, expr: &Expr) -> Option<StaticValue> {
        let node = expr.try_node(node_id).ok()?;

        // Création d'un buffer local sur la pile (Stack allocation)
        // C'est ultra-rapide et propre à chaque thread.
        let mut buffer = ArgumentBuffer::new();

        match node.kind() {
            ExprKind::AtomicFormula => {
                self.evaluate_predicate_internal(node_id, expr, &mut buffer)
                    .ok()
                    .flatten()
                    .map(StaticValue::Boolean)
            }
            ExprKind::FunctionTerm => {
                self.evaluate_function_internal(node_id, expr, &mut buffer)
                    .ok()
                    .flatten()
            }
            _ => None,
        }
    }
}


#[cfg(test)]
impl<'a> InertiaRegistry<'a> {
    /// Crée un registre mocké.
    /// Note : l'InertiaTable doit être créée à l'extérieur (dans le test)
    /// pour garantir la durée de vie 'a.
    pub fn mock(
        predicate_defs: &'a [AtomicFormulaSkeleton],
        function_defs: &'a [AtomicFunctionSkeleton],
        value_registry: &'a ValueRegistry,
        inertia: &'a InertiaTable,
    ) -> Self {
        Self {
            counting_predicates: HashMap::new(),
            static_functions: HashMap::new(),
            inertia,
            predicate_defs,
            function_defs,
            value_registry,
            consensus_values: HashMap::new(),
            max_arity: DEFAULT_MAX_ARITY,
            max_proj: DEFAULT_MAX_PROJ,
        }
    }

    pub fn mock_with_config(
        predicate_defs: &'a [AtomicFormulaSkeleton],
        function_defs: &'a [AtomicFunctionSkeleton],
        value_registry: &'a ValueRegistry,
        inertia: &'a InertiaTable,
        max_arity: usize,
        max_proj: usize,
    ) -> Self {
        Self {
            counting_predicates: HashMap::new(),
            static_functions: HashMap::new(),
            inertia,
            predicate_defs,
            function_defs,
            value_registry,
            consensus_values: HashMap::new(),
            max_arity,
            max_proj,
        }
    }

    /// Injecte manuellement des données de comptage pour simuler l'état initial.
    pub fn inject_predicate_count(&mut self, id: usize, mask: u16, args: Vec<ObjectId>, count: usize) {
        self.counting_predicates
            .entry(AtomSkeletonId::from(id))
            .or_default()
            .entry(mask)
            .or_default()
            .insert(args.into_boxed_slice(), count);
    }
}

#[cfg(test)]
mod tests {
    use ordered_float::OrderedFloat;
    use super::*;
    use crate::aiplan4rust::grounding::analysis::inertia::inertia::Inertia;
    use crate::aiplan4rust::lang::{FunctionSymbolId, PredicateSymbolId, Type, TypeId, TypedList, TypedSymbol, VariableId};
    use crate::aiplan4rust::lir::expr::builder::ExprBuilder;
    use crate::aiplan4rust::lir::problem::atomic_skeleton::AtomicFormulaSkeleton;

    /// Helper pour créer des définitions de prédicats de test
    pub fn mock_predicate_defs(count: usize) -> Vec<AtomicFormulaSkeleton> {
        (0..count).map(|i| AtomicFormulaSkeleton::new(
            PredicateSymbolId::from(i),
            TypedList::new()
        )).collect()
    }

    /// Helper pour créer des définitions de fonctions de test
    pub fn mock_function_defs(count: usize) -> Vec<AtomicFunctionSkeleton> {
        (0..count).map(|i| AtomicFunctionSkeleton::new(
            FunctionSymbolId::from(i),
            TypedList::new(),
            // On fournit au moins un TypeId pour éviter le panic
            Type::either(vec![TypeId::from(0)])
        )).collect()
    }

    #[test]
    fn test_empty_registry_returns_false_for_positive_inertia() {
        let mut builder = ExprBuilder::new();
        let skel_id_raw = 1;

        let arg1 = builder.constant(10);
        let arg2 = builder.constant(20);

        // Utilisation de la méthode LIR avec skeleton ID
        let atom_node = builder.atomic_formula_with_skeleton(
            0, // PredicateSymbolId
            vec![arg1, arg2],
            skel_id_raw // AtomSkeletonId
        );
        let expr = builder.finish();

        // --- CONTEXTE DE TEST ---
        // On crée 2 définitions pour que l'index [1] soit valide
        let p_defs = mock_predicate_defs(2);
        let f_defs = vec![];
        let v_reg = ValueRegistry::new();
        let mut i_table = InertiaTable::new();

        // On marque le squelette 1 comme Inerte Positif
        i_table.insert_predicate(AtomSkeletonId::from(skel_id_raw), Inertia::Positive);

        // On crée le registre lié à ces données
        let registry = InertiaRegistry::mock(&p_defs, &f_defs, &v_reg, &i_table);

        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer);

        // Selon IPP : Inerte Positif + Absent de l'état initial (N=0) => FALSE
        assert_eq!(
            res.unwrap(),
            Some(false),
            "Un prédicat inerte positif absent de l'état initial doit être simplifié à False"
        );
    }

    #[test]
    fn test_static_function_evaluation() {
        let mut builder = ExprBuilder::new();
        let func_id_raw = 5;
        let skel_id_raw = 5;
        let obj_id = 100;

        let arg = builder.constant(obj_id);

        // Construction du nœud avec le squelette LIR
        let term_node = builder.function_term_with_skeleton(
            func_id_raw,
            vec![arg],
            skel_id_raw
        );
        let expr = builder.finish();

        // --- PRÉPARATION DES DÉPENDANCES (Lifetimes 'a) ---
        let p_defs = mock_predicate_defs(0);
        let f_defs = mock_function_defs(6); // Index 5 inclus
        let v_reg = ValueRegistry::new();
        let mut i_table = InertiaTable::new();

        // Configuration de l'inertie sur la table (avant l'emprunt par le registre)
        i_table.insert_function(FunctionSkeletonId::from(skel_id_raw), Inertia::Positive);

        // Création du registre avec les références
        let mut registry = InertiaRegistry::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // Injection manuelle de la valeur : distance(obj100) = 42.0
        let val = StaticValue::Number(OrderedFloat::from(42.0));
        registry.generate_function_masks(
            FunctionSkeletonId::from(skel_id_raw),
            1, // arity
            &[ObjectId::from(obj_id)],
            val
        );

        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_function_internal(term_node, &expr, &mut buffer);

        // Vérification du résultat
        assert_eq!(
            res.unwrap(),
            Some(StaticValue::Number(OrderedFloat::from(42.0))),
            "La fonction statique doit retourner sa valeur initiale enregistrée"
        );
    }

    #[test]
    fn test_positive_inertia_pruning() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id) = (1, 1);

        // On crée un atome sans arguments (arité 0)
        let atom_node = builder.atomic_formula_with_skeleton(pred_id, vec![], skel_id);
        let expr = builder.finish();

        // --- SETUP DU CONTEXTE (Lifetimes 'a) ---
        // On crée 2 définitions pour que l'index 1 soit valide
        let p_defs = mock_predicate_defs(2);
        let f_defs = mock_function_defs(0);
        let v_reg = ValueRegistry::new();
        let mut i_table = InertiaTable::new();

        // On définit le prédicat comme Inerte Positif
        // (Rappel : Inerte Positif = n'apparaît dans aucun effet positif = ne peut pas être ajouté)
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::Positive);

        // Création du registre avec les références
        let registry = InertiaRegistry::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // L'état initial est vide par défaut dans le mock : N(p, a) = 0
        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer).unwrap();

        // Selon la Définition 6 de Koehler : Si p est positive inertia et N(p, a) = 0, alors FALSE.
        assert_eq!(
            res,
            Some(false),
            "Un prédicat inerte positif absent de l'état initial doit être simplifié à False"
        );
    }

    #[test]
    fn test_negative_inertia_simplification() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id) = (1, 1);

        // Pour un prédicat d'arité 0, MAX est toujours 1.
        let atom_node = builder.atomic_formula_with_skeleton(pred_id, vec![], skel_id);
        let expr = builder.finish();

        // --- SETUP DU CONTEXTE (Lifetimes 'a) ---
        let p_defs = mock_predicate_defs(2);
        let f_defs = mock_function_defs(0);
        let v_reg = ValueRegistry::new();
        let mut i_table = InertiaTable::new();

        // On définit le prédicat comme Inerte Négatif
        // (N'apparaît dans aucun effet négatif = ne peut pas être supprimé)
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::Negative);

        // Création du registre avec les références
        let mut registry = InertiaRegistry::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // Pour satisfaire N = MAX, on insère le fait dans l'état initial.
        // Pour l'arité 0, un seul masque d'arguments vides [] suffit.
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id), 0, &[]);

        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer).unwrap();

        // Selon la Définition 6 de Koehler : Si p est negative inertia et N(p, a) = MAX(p, a), alors TRUE.
        assert_eq!(
            res,
            Some(true),
            "Un prédicat inerte négatif dont toutes les instances sont initialement vraies doit être simplifié à True"
        );
    }

    /// Test pour l'isolation des prédicats (vérifie que les squelettes ne se mélangent pas).
    ///
    /// Ce test vérifie que le registre distingue correctement deux prédicats différents (s1 et s2)
    /// même s'ils partagent des arguments identiques.
    /// Selon les principes de Koehler, la simplification doit être locale aux entrées de l'état
    /// initial propres à chaque prédicat (calcul de N).
    #[test]
    fn test_predicate_isolation() {
        let mut builder = ExprBuilder::new();
        let (p1, s1) = (1, 1);
        let (p2, s2) = (2, 2);
        let obj_id = ObjectId::from(10);

        // On crée l'argument une seule fois pour les deux atomes
        let node_arg = builder.constant(obj_id);
        let args = vec![node_arg];

        let node1 = builder.atomic_formula_with_skeleton(p1, args.clone(), s1);
        let node2 = builder.atomic_formula_with_skeleton(p2, args, s2);
        let expr = builder.finish();

        // --- SETUP DU CONTEXTE (Lifetimes 'a) ---
        let p_defs = mock_predicate_defs(3); // On a besoin d'index jusqu'à 2
        let f_defs = mock_function_defs(0);
        let v_reg = ValueRegistry::new();
        let mut i_table = InertiaTable::new();

        // On définit les deux prédicats comme Inertes Positifs
        i_table.insert_predicate(AtomSkeletonId::from(s1), Inertia::Positive);
        i_table.insert_predicate(AtomSkeletonId::from(s2), Inertia::Positive);

        // Création du registre avec les références
        let mut registry = InertiaRegistry::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // On enregistre uniquement s1(10). s2(10) reste absent (N=0).
        // arity = 1
        registry.generate_predicate_masks(AtomSkeletonId::from(s1), 1, &[obj_id]);

        let mut buffer = ArgumentBuffer::new();

        // Évaluation de s1(10)
        // Comme il est Inerte Positif ET présent dans l'état initial, il est simplifié à True.
        let res1 = registry.evaluate_predicate_internal(node1, &expr, &mut buffer)
            .expect("L'évaluation a échoué pour s1");
        assert_eq!(res1, Some(true), "Le prédicat s1(10) devrait être trouvé et simplifié à True");

        // Évaluation de s2(10)
        // Comme il est Inerte Positif ET absent de l'état initial (N=0), il est simplifié à False.
        let res2 = registry.evaluate_predicate_internal(node2, &expr, &mut buffer)
            .expect("L'évaluation a échoué pour s2");
        assert_eq!(res2, Some(false), "Le prédicat s2(10) devrait être False (Inertie Positive + Absent)");
    }

    #[test]
    fn test_returns_none_on_variable_argument() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id) = (1, 1);
        let type_id = TypeId::from(0);
        let obj_10 = ObjectId::from(10);
        let obj_11 = ObjectId::from(11);

        // 1. On peuple le ValueRegistry avec 2 objets (MAX = 2)
        let v_reg = ValueRegistry::new().with_typed_list(TypedList::from_iter(vec![
            TypedSymbol::new(obj_10, Type::either(vec![type_id])),
            TypedSymbol::new(obj_11, Type::either(vec![type_id])),
        ]));

        // 2. On définit manuellement le squelette pour inclure le type de l'argument
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()),
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(1),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::either(vec![type_id]))
                ])
            ),
        ];
        let f_defs = Vec::new();

        // 3. Construction de l'expression P(?x)
        let var_node = builder.variable(0);
        let atom_node = builder.atomic_formula_with_skeleton(pred_id, vec![var_node], skel_id);
        let expr = builder.finish();

        let mut i_table = InertiaTable::new();
        // On met Positive ET Negative pour simuler une constante parfaite
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::Positive);
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::Negative);

        let mut registry = InertiaRegistry::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // 4. On injecte SEULEMENT P(10).
        // N = 1 (P(10) est vrai)
        // MAX = 2 (Le domaine de Type 0 contient {10, 11})
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id), 1, &[obj_10]);

        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer).unwrap();

        // Comme N(1) != 0 et N(1) != MAX(2), le registre doit répondre "Je ne sais pas"
        assert!(
            res.is_none(),
            "L'évaluation doit être None car l'atome n'est vrai que pour une partie du domaine"
        );
    }

    #[test]
    fn test_negative_inertia_n_equals_max_with_variable() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id) = (1, 1);
        let type_id = TypeId::from(0);

        // IDs des objets pour le test
        let obj_100 = ObjectId::from(100);
        let obj_101 = ObjectId::from(101);

        // 1. Setup du ValueRegistry (Domaine de taille 2)
        let v_reg = ValueRegistry::new().with_typed_list(TypedList::from_iter(vec![
            TypedSymbol::new(obj_100, Type::either(vec![type_id])),
            TypedSymbol::new(obj_101, Type::either(vec![type_id])),
        ]));

        // 2. Setup des définitions (Squelette typé pour calculer MAX)
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()),
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(1),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::either(vec![type_id]))
                ])
            ),
        ];
        let f_defs = Vec::new();

        // 3. Construction de l'expression P(?x)
        let var_node = builder.variable(0);
        let atom_node = builder.atomic_formula_with_skeleton(pred_id, vec![var_node], skel_id);
        let expr = builder.finish();

        // 4. Configuration de l'Inertie
        let mut i_table = InertiaTable::new();
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::Negative);

        // Initialisation du registre
        let mut registry = InertiaRegistry::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // 5. REMPLISSAGE VIA LA FONCTION (N=2 pour le masque 0)
        // Cela va automatiquement créer les entrées HashMap avec les bons types (Box<[ObjectId]>, u16, usize)
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id), 1, &[obj_100]);
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id), 1, &[obj_101]);

        // 6. Évaluation
        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer).unwrap();

        // Analyse : N(2) == MAX(2) + Inertie Négative => TRUE
        assert_eq!(
            res,
            Some(true),
            "Si N=MAX pour un inerte négatif, P(?x) doit être simplifié à True"
        );
    }

    #[test]
    fn test_negative_inertia_arity_0_simplification() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id) = (1, 1);

        // Atome sans arguments
        let atom_node = builder.atomic_formula_with_skeleton(pred_id, vec![], skel_id);
        let expr = builder.finish();

        let p_defs = mock_predicate_defs(2);
        let f_defs = Vec::new();
        let v_reg = ValueRegistry::new();
        let mut i_table = InertiaTable::new();
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::Negative);

        let mut registry = InertiaRegistry::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // N = 1 (pour arité 0, on n'envoie pas d'objets)
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id), 0, &[]);

        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer).unwrap();

        // Ici MAX doit être 1 (produit vide), N est 1. Résultat : True.
        assert_eq!(res, Some(true));
    }

    #[test]
    fn test_partial_instantiation_returns_none() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id) = (1, 1);
        let type_id = TypeId::from(0);
        let obj10 = ObjectId::from(10);
        let obj20 = ObjectId::from(20);
        let obj21 = ObjectId::from(21); // Second objet pour que MAX = 2

        // 1. Setup du ValueRegistry (pour que le type de ?y ait 2 objets)
        let v_reg = ValueRegistry::new().with_typed_list(TypedList::from_iter(vec![
            TypedSymbol::new(obj10, Type::either(vec![type_id])),
            TypedSymbol::new(obj20, Type::either(vec![type_id])),
            TypedSymbol::new(obj21, Type::either(vec![type_id])),
        ]));

        // 2. Setup des définitions (P prend deux arguments de Type 0)
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()),
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(1),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::either(vec![type_id])),
                    TypedSymbol::new(VariableId::from(1), Type::either(vec![type_id])),
                ])
            ),
        ];
        let f_defs = Vec::new();

        // 3. Construction de P(10, ?var1)
        // Note : On utilise variable(1) car c'est le 2ème argument du squelette
        let arg_const = builder.constant(obj10);
        let arg_var = builder.variable(1);
        let atom_node = builder.atomic_formula_with_skeleton(pred_id, vec![arg_const, arg_var], skel_id);
        let expr = builder.finish();

        // 4. Setup Inertie (Inerte Positif)
        let mut i_table = InertiaTable::new();
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::Positive);

        let mut registry = InertiaRegistry::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // 5. Enregistrement d'un fait : P(10, 20)
        // Cela va incrémenter le masque pour P(10, ?y) à N=1
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id), 2, &[obj10, obj20]);

        // 6. Évaluation
        let mut buffer = ArgumentBuffer::new();
        // On simule que la constante '10' est déjà résolue dans le buffer
        buffer.push(obj10);

        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer)
            .expect("Evaluation should not fail");

        // ANALYSE :
        // - Masque calculé pour P(10, ?y) : Le premier argument est fixé, le second est variable.
        // - N(P, [10, ?]) = 1  (car P(10, 20) existe)
        // - MAX(P, [10, ?]) = 2 (car ?y peut être 20 ou 21)
        // - 0 < N < MAX => On ne peut pas simplifier.
        assert!(
            res.is_none(),
            "Une instanciation partielle avec N < MAX doit retourner None"
        );
    }


    #[test]
    fn test_fix_projection_beyond_first_argument() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id) = (1, 1);
        let type_id = TypeId::from(0);
        let obj10 = ObjectId::from(10); // Robot 1
        let obj51 = ObjectId::from(51); // Zone 51 (Second argument)

        // 1. Setup du ValueRegistry
        let v_reg = ValueRegistry::new().with_typed_list(TypedList::from_iter(vec![
            TypedSymbol::new(obj10, Type::either(vec![type_id])),
            TypedSymbol::new(obj51, Type::either(vec![type_id])),
        ]));

        // 2. Définitions : P(?x, ?y)
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()), // Dummy
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(1),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::either(vec![type_id])),
                    TypedSymbol::new(VariableId::from(1), Type::either(vec![type_id])),
                ])
            ),
        ];
        let f_defs = Vec::new();

        // 3. Setup Inertie (Inerte Positif pour la Règle 1 : N=0 => FALSE)
        let mut i_table = InertiaTable::new();
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::Positive);

        // 4. INITIALISATION DU REGISTRE AVEC MAX_PROJ = 1
        // C'est ici que le test devient intéressant.
        let mut registry = InertiaRegistry::mock_with_config(&p_defs, &f_defs, &v_reg, &i_table, 2, 1);

        // 5. ENREGISTREMENT DU FAIT : P(10, 51)
        // On simule l'appel corrigé qui envoie TOUS les arguments
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id), 2, &[obj10, obj51]);

        // --- CAS DE TEST : Évaluer P(?var0, 51) ---
        // On veut savoir si quelqu'un est en Zone 51, mais on ne précise pas qui (?var0).
        // Le filtre max_proj = 1 autorise l'indexation de l'objet 51 (1 seul argument fixe).

        let arg_var = builder.variable(0);
        let arg_const = builder.constant(obj51);
        let atom_node = builder.atomic_formula_with_skeleton(pred_id, vec![arg_var, arg_const], skel_id);
        let expr = builder.finish();

        let mut buffer = ArgumentBuffer::new();
        // extract_mask_dynamic va trouver la constante 51 en deuxième position.
        // Masque attendu (Big Endian) : 0b01 (décimal 1)

        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer)
            .expect("Evaluation should not fail");

        // ANALYSE :
        // Si la correction ".take(n_proj)" a été faite :
        // - Le registre a reçu [10, 51].
        // - Il a généré le masque 0b01 pour l'objet 51.
        // - N(P, [?, 51]) = 1.
        // - Comme N > 0 et que c'est une inertie positive, ce n'est pas FALSE.
        // - Comme N=1 et MAX=2 (car ?x peut être 10 ou 51), ce n'est pas TRUE non plus.
        // - Résultat attendu : None.

        // Si la correction n'est PAS faite :
        // - Le process_predicate n'a envoyé que [10] (à cause du .take(1)).
        // - Le masque 0b01 (deuxième position) n'a jamais été enregistré.
        // - N(P, [?, 51]) sera 0.
        // - Résultat : Some(false) <--- ERREUR !

        assert!(
            res.is_none(),
            "Le registre devrait trouver l'objet 51 en deuxième position et retourner None (pas False)"
        );
    }

    #[test]
    fn test_projection_full_simplification_to_true() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id) = (1, 1);

        // On définit deux types distincts pour isoler les domaines
        let type_robot = TypeId::from(0);
        let type_room = TypeId::from(1);

        let obj10 = ObjectId::from(10); // Le seul robot
        let obj51 = ObjectId::from(51); // La salle

        // 1. Setup du ValueRegistry :
        // ?x (type_robot) n'aura qu'un seul objet possible dans son domaine : obj10.
        let v_reg = ValueRegistry::new().with_typed_list(TypedList::from_iter(vec![
            TypedSymbol::new(obj10, Type::either(vec![type_robot])),
            TypedSymbol::new(obj51, Type::either(vec![type_room])),
        ]));

        // 2. Définitions : P(?x:robot, ?y:room)
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()),
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(1),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::either(vec![type_robot])),
                    TypedSymbol::new(VariableId::from(1), Type::either(vec![type_room])),
                ])
            ),
        ];
        let f_defs = Vec::new();

        // 3. Setup Inertie : NEGATIVE (pour la règle N=MAX => TRUE)
        let mut i_table = InertiaTable::new();
        i_table.insert_predicate(AtomSkeletonId::from(skel_id), Inertia::Negative);

        // 4. Initialisation avec une config permettant de stocker le masque 0b01
        let mut registry = InertiaRegistry::mock_with_config(&p_defs, &f_defs, &v_reg, &i_table, 2, 2);

        // 5. Enregistrement du fait initial : P(10, 51)
        registry.generate_predicate_masks(AtomSkeletonId::from(skel_id), 2, &[obj10, obj51]);

        // --- CAS DE TEST : Évaluer P(?x, 51) ---
        // ?x est une variable (non instanciée), 51 est une constante.
        let arg_var = builder.variable(0);
        let arg_const = builder.constant(obj51);
        let atom_node = builder.atomic_formula_with_skeleton(pred_id, vec![arg_var, arg_const], skel_id);
        let expr = builder.finish();

        let mut buffer = ArgumentBuffer::new();

        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer)
            .expect("Evaluation should not fail");

        // ANALYSE :
        // - extract_mask_dynamic voit que seul le 2ème argument est fixe => masque 0b01.
        // - N = 1 (P(10, 51) est présent).
        // - MAX = domaine du type de la variable ?x (type_robot) = {obj10} => cardinality 1.
        // - N(1) == MAX(1) et Inerte Négatif => TRUE.

        assert_eq!(
            res,
            Some(true),
            "Le registre devrait simplifier à TRUE car l'unique instance possible du domaine est initialement vraie"
        );
    }

    #[test]
    fn test_perfect_constant_missing_is_always_false() {
        let mut builder = ExprBuilder::new();
        let type_id = TypeId::from(0);

        // On définit un ID de squelette unique
        let skel_id_val = 1;
        let skel_id = AtomSkeletonId::from(skel_id_val);

        // 1. Définition des squelettes
        // Le prédicat 1 est lié au squelette 1
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()),
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(1),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::either(vec![type_id]))
                ])
            ),
        ];

        // 2. ValueRegistry : indispensable pour calculate_max_instances
        let v_reg = ValueRegistry::new().with_typed_list(TypedList::from_iter(vec![
            TypedSymbol::new(ObjectId::from(100), Type::either(vec![type_id])),
        ]));

        // 3. Inertie : On marque explicitement le SQUELETTE 1 comme Inerte Positif
        let mut i_table = InertiaTable::new();
        i_table.insert_predicate(skel_id, Inertia::Positive);

        // 4. Création du registre
        let f_defs = Vec::new();
        let registry = InertiaRegistry::mock(&p_defs, &f_defs, &v_reg, &i_table);

        // 5. Construction de l'atome
        // On s'assure que le nœud d'atome porte BIEN le skel_id 1
        let var_node = builder.variable(0);
        let atom_node = builder.atomic_formula_with_skeleton(
            1,             // PredicateId
            vec![var_node], // Arguments (?x)
            skel_id_val    // AtomSkeletonId (stocké dans le nœud)
        );
        let expr = builder.finish();

        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_predicate_internal(atom_node, &expr, &mut buffer)
            .expect("Evaluation failed");

        // ANALYSE :
        // Si is_positive est true et n_p_a est 0 => Some(false)
        assert_eq!(
            res,
            Some(false),
            "Le prédicat devrait être simplifié à FALSE (Inerte Positif + Absent)"
        );
    }

    #[test]
    fn test_arity_zero_flag_behavior() {
        let mut builder = ExprBuilder::new();
        let (p1, s1_val) = (1, 1);
        let (p2, s2_val) = (2, 2);

        let skel1 = AtomSkeletonId::from(s1_val);
        let skel2 = AtomSkeletonId::from(s2_val);

        // 1. Définitions : Squelettes d'arité 0
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()),
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(p1), TypedList::new()),
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(p2), TypedList::new()),
        ];

        // 2. Setup Inertie : Inerte Positif
        let mut i_table = InertiaTable::new();
        i_table.insert_predicate(skel1, Inertia::Positive);
        i_table.insert_predicate(skel2, Inertia::Positive);

        // 3. Initialisation du ValueRegistry
        // On utilise la méthode de test pour s'assurer que le vecteur interne
        // est au moins initialisé, évitant le panic si le code cherche un TypeId.
        let value_registry = ValueRegistry::new().with_typed_list(TypedList::new());

        let f_defs = Vec::new();
        let mut registry = InertiaRegistry::mock(&p_defs, &f_defs, &value_registry, &i_table);

        // On enregistre P1 comme présent à l'état initial
        // N(P1, []) = 1
        registry.generate_predicate_masks(skel1, 0, &[]);

        // 4. Construction des atomes LIR (Arité 0)
        let node1 = builder.atomic_formula_with_skeleton(p1, vec![], s1_val);
        let node2 = builder.atomic_formula_with_skeleton(p2, vec![], s2_val);
        let expr = builder.finish();

        let mut buffer = ArgumentBuffer::new();

        // 5. Évaluation
        // node1 (s1) : N=1, MAX=1 -> Inerte + Présent = TRUE
        let res1 = registry.evaluate_predicate_internal(node1, &expr, &mut buffer)
            .expect("Evaluation node1 failed");

        // node2 (s2) : N=0 -> Inerte Positif + Absent = FALSE
        let res2 = registry.evaluate_predicate_internal(node2, &expr, &mut buffer)
            .expect("Evaluation node2 failed");

        assert_eq!(res1, Some(true), "Le flag s1 devrait être TRUE (présent + inerte)");
        assert_eq!(res2, Some(false), "Le flag s2 devrait être FALSE (absent + inerte positif)");
    }

    #[test]
    fn test_mask_differentiation_same_object_different_positions() {
        let mut builder = ExprBuilder::new();
        let (pred_id, skel_id_raw) = (1, 1);
        let skel_id = AtomSkeletonId::from(skel_id_raw);
        let type_id = TypeId::from(0);
        let obj10 = ObjectId::from(10);
        let obj99 = ObjectId::from(99);

        // 1. Setup des définitions : P(?x:type0, ?y:type0)
        let p_defs = vec![
            AtomicFormulaSkeleton::new(PredicateSymbolId::from(0), TypedList::new()),
            AtomicFormulaSkeleton::new(
                PredicateSymbolId::from(pred_id),
                TypedList::from_iter(vec![
                    TypedSymbol::new(VariableId::from(0), Type::either(vec![type_id])),
                    TypedSymbol::new(VariableId::from(1), Type::either(vec![type_id])),
                ])
            ),
        ];

        // 2. Setup du ValueRegistry (Indispensable pour que le type soit connu)
        let v_reg = ValueRegistry::new().with_typed_list(TypedList::from_iter(vec![
            TypedSymbol::new(obj10, Type::either(vec![type_id])),
            TypedSymbol::new(obj99, Type::either(vec![type_id])),
        ]));

        // 3. Initialisation du registre avec max_arity=2 et max_projection=2
        let i_table = InertiaTable::new();
        let f_defs = Vec::new();
        let mut registry = InertiaRegistry::mock_with_config(
            &p_defs,
            &f_defs,
            &v_reg,
            &i_table,
            2,
            2
        );

        // 4. On enregistre le fait P(10, 99) à l'état initial
        // Cela va générer, entre autres, le masque 0b10 pour l'objet 10 (position 0)
        registry.generate_predicate_masks(skel_id, 2, &[obj10, obj99]);

        // 5. Cas de test : On évalue l'atome P(?var0, 10)
        // Ici, le premier argument est une variable, le second est la constante 10.
        let arg_var = builder.variable(0);
        let arg_const = builder.constant(obj10);
        let node_id = builder.atomic_formula_with_skeleton(pred_id, vec![arg_var, arg_const], skel_id_raw);
        let expr = builder.finish();

        let mut buffer = ArgumentBuffer::new();
        let node = expr.try_node(node_id).unwrap();

        // Extraction dynamique du masque pour P(?, 10)
        let mask = registry.extract_mask_dynamic(node, &expr, &mut buffer);

        // ANALYSE :
        // - Argument 0 est Variable -> bit 0 (poids fort) = 0
        // - Argument 1 est Constante -> bit 1 (poids faible) = 1
        // - Masque attendu : 0b01 (1)
        assert_eq!(mask, 1, "Le masque pour le second argument fixe doit être 0b01 (1)");
        assert_eq!(buffer.len(), 1, "Le buffer doit contenir exactement 1 constante (obj10)");
        assert_eq!(buffer[0], obj10);

        // 6. Vérification de la non-collision
        // On cherche dans la table si on a une entrée pour P avec le masque 0b01 et l'objet [10]
        let n = registry.counting_predicates.get(&skel_id)
            .and_then(|m| m.get(&mask))
            .and_then(|e| e.get(&buffer[..]))
            .copied()
            .unwrap_or(0);

        // On ne doit RIEN trouver.
        // Pourquoi ? Parce que l'objet 10 a été enregistré avec le masque 0b10 (position 0).
        // Ici on interroge la position 1.
        assert_eq!(n, 0, "Collision détectée ! L'objet 10 en position 0 ne doit pas être trouvé pour la position 1");
    }

    #[test]
    /// **Objective:** Verify the evaluation of a static (inert) numeric function.
    ///
    /// This test ensures that when a function is marked as positive inertia (its value never changes),
    /// the registry correctly retrieves and returns the constant numeric value associated with
    /// grounded (constant) arguments.
    ///
    /// **Input:**
    /// - A function `f` marked as `Inertia::Positive`.
    /// - An initial assignment: `f(obj_10) = 42.5`.
    /// - An expression node representing the grounded call `f(10)`.
    ///
    /// **Expected Output:**
    /// - `Some(StaticValue::Number(42.5))` representing the successfully simplified constant value.
    fn test_evaluate_function_static_numeric() {
        let mut builder = ExprBuilder::new();
        let (func_id_val, skel_id_val) = (1, 1);
        let skel_id = FunctionSkeletonId::from(skel_id_val);
        let obj_a = ObjectId::from(10);
        let val = 42.5;

        // 1. Inertia: Mark the function as positive inertia (static) 🧊
        let mut i_table = InertiaTable::new();
        i_table.insert_function(skel_id, Inertia::Positive);

        // 2. Direct definition of function skeletons 🛠️
        let f_defs = vec![
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(0),
                TypedList::new(),
                Type::either(vec![TypeId::from(0)])
            ),
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(func_id_val),
                TypedList::new(),
                Type::either(vec![TypeId::from(0)])
            ),
        ];

        // 3. Setup the registry and inject initial state: f(obj_10) = 42.5 🔢
        let p_defs = Vec::new();
        let value_registry = ValueRegistry::new();
        let mut registry = InertiaRegistry::mock(&p_defs, &f_defs, &value_registry, &i_table);
        registry.generate_function_masks(skel_id, 1, &[obj_a], StaticValue::Number(OrderedFloat::from(val)));

        // 4. Build the LIR expression: f(10)
        let arg = builder.constant(obj_a);
        let node_id = builder.function_term_with_skeleton(func_id_val, vec![arg], skel_id_val);
        let expr = builder.finish();

        // 5. Evaluation of the internal logic
        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_function_internal(node_id, &expr, &mut buffer)
            .expect("Evaluation should not fail for valid grounded inputs");

        assert_eq!(res, Some(StaticValue::Number(OrderedFloat(val))));
    }

    #[test]
    fn test_evaluate_function_non_grounded_diverging_values() {
        let mut builder = ExprBuilder::new();
        let skel_id_val = 1;
        let skel_id = FunctionSkeletonId::from(skel_id_val);
        let obj_10 = ObjectId::from(10);
        let obj_20 = ObjectId::from(20);

        // 1. Inertia: Mark as static 🧊
        let mut i_table = InertiaTable::new();
        i_table.insert_function(skel_id, Inertia::Positive);

        // 2. Local definitions for the mock 🛠️
        let f_defs = vec![
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(skel_id_val),
                TypedList::new(),
                Type::either(vec![TypeId::from(0)])
            ),
        ];

        // 3. Setup registry and inject TWO different values
        let p_defs = Vec::new();
        let value_registry = ValueRegistry::new();
        let mut registry = InertiaRegistry::mock(&p_defs, &f_defs, &value_registry, &i_table);

        registry.generate_function_masks(skel_id, 1, &[obj_10], StaticValue::Number(OrderedFloat::from(42.5)));
        registry.generate_function_masks(skel_id, 1, &[obj_20], StaticValue::Number(OrderedFloat::from(100.0)));

        // 4. Build expression with a variable: f(?var0) ❓
        let var_node = builder.variable(0);
        let node_id = builder.function_term_with_skeleton(skel_id_val, vec![var_node], skel_id_val);
        let expr = builder.finish();

        // 5. Evaluation
        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_function_internal(node_id, &expr, &mut buffer)
            .expect("Evaluation should not fail");

        // The result MUST be None because the function is not grounded
        // and values are not unanimous.
        assert_eq!(res, None);
    }

    #[test]
    /// **Objective:** Ensure non-grounded functions are not prematurely simplified.
    ///
    /// **Input:**
    /// - A static function `f`.
    /// - Injected data for a specific instance: `f(obj_10) = 42.5`.
    /// - An expression node with a variable: `f(?var0)`.
    ///
    /// **Expected Output:**
    /// - `None`, verifying the registry correctly identifies the expression as non-simplifiable.
    fn test_evaluate_function_non_grounded_returns_none() {
        let mut builder = ExprBuilder::new();
        let skel_id_val = 1;
        let skel_id = FunctionSkeletonId::from(skel_id_val);
        let obj_10 = ObjectId::from(10);
        let val = 42.5;

        // 1. Inertia: Mark the function as static 🧊
        let mut i_table = InertiaTable::new();
        i_table.insert_function(skel_id, Inertia::Positive);

        // 2. Define the function skeleton 🛠️
        let f_defs = vec![
            AtomicFunctionSkeleton::new(
                FunctionSymbolId::from(skel_id_val),
                TypedList::new(),
                Type::either(vec![TypeId::from(0)])
            ),
        ];

        // 3. Setup registry and inject data for ONE specific object 🔢
        let p_defs = Vec::new();
        let value_registry = ValueRegistry::new();
        let mut registry = InertiaRegistry::mock(&p_defs, &f_defs, &value_registry, &i_table);
        registry.generate_function_masks(skel_id, 1, &[obj_10], StaticValue::Number(OrderedFloat::from(val)));

        // 4. Build expression with a variable: f(?var0)
        let var_node = builder.variable(0);
        let node_id = builder.function_term_with_skeleton(skel_id_val, vec![var_node], skel_id_val);
        let expr = builder.finish();

        // 5. Evaluation
        let mut buffer = ArgumentBuffer::new();
        let res = registry.evaluate_function_internal(node_id, &expr, &mut buffer)
            .expect("Evaluation should handle non-grounded nodes gracefully");

        assert_eq!(res, None, "Should not simplify a function call containing variables");
    }
}
