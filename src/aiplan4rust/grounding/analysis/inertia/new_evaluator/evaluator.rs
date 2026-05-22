use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::grounding::analysis::inertia::new_evaluator::InertiaRegistryError;
use crate::aiplan4rust::grounding::analysis::inertia::new_table::InertiaTable;
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lang::{AtomSkeletonId, CompareOp, FunctionSkeletonId, ObjectId};
use crate::aiplan4rust::lir::expr::ops::simplification::evaluator::StaticEvaluator;
use crate::aiplan4rust::lir::expr::ops::simplification::StaticValue;
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lir::expr::{ExprEntryKind, ExprNodeRef, ExprStore};
use crate::aiplan4rust::lir::problem::skeleton::{AtomicFormulaSkeleton, AtomicFunctionSkeleton};
use crate::aiplan4rust::lir::problem::NewLiftedProblem;
use ordered_float::OrderedFloat;
use smallvec::SmallVec;
use std::collections::HashMap;

const DEFAULT_MAX_ARITY: usize = 15;
const DEFAULT_MAX_PROJ: usize = 3;
const ARGUMENT_BUFFER_SIZE: usize = 8;

// Buffer pour les arguments des prédicats/fonctions.
/// 8 éléments sur la pile suffisent pour presque tous les domaines.
type ArgumentBuffer = SmallVec<[ObjectId; ARGUMENT_BUFFER_SIZE]>;

#[derive(Debug)]
pub struct InertiaEvaluator<'a> {
    counting_predicates: HashMap<AtomSkeletonId, HashMap<u16, HashMap<Box<[ObjectId]>, usize>>>,
    static_functions:
        HashMap<FunctionSkeletonId, HashMap<u16, HashMap<Box<[ObjectId]>, StaticValue>>>,
    inertia: &'a InertiaTable,

    // --- RÉFÉRENCES EMPRUNTÉES (Context) ---
    predicate_defs: Box<[AtomicFormulaSkeleton]>,
    function_defs: Box<[AtomicFunctionSkeleton]>, // Pour les signatures des fonctions
    value_registry: &'a ValueRegistry,

    consensus_values: HashMap<FunctionSkeletonId, StaticValue>,

    max_arity: usize,
    max_proj: usize,
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
    ) -> Result<Self, InertiaRegistryError> {
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
                ExprEntryKind::AtomicFormula(_) | ExprEntryKind::Comparison(_) => {
                    // On encapsule l'entrée courante dans un ExprNodeRef pour faciliter le traitement
                    let node_ref = ExprNodeRef::new(id, entry);

                    registry.process_init(node_ref, init.store())?;

                    // On saute les enfants car process_init s'occupe de descendre
                    // pour extraire les arguments et symboles.
                    it.skip_children(entry.children().len());
                }

                // En PDDL, l'init est une liste de faits positifs.
                // On ignore les négations et les TILs (gérés par la table d'inertie).
                ExprEntryKind::Not | ExprEntryKind::TimedInitialLiteral => {
                    it.skip_children(entry.children().len());
                }

                _ => {}
            }
        }

        Ok(registry)
    }

    fn process_init(
        &mut self,
        node: ExprNodeRef<'_>,
        store: &ExprStore,
    ) -> Result<(), InertiaRegistryError> {
        match node.kind() {
            // Cas d'un fait atomique (ex: (at robby room1))
            ExprEntryKind::AtomicFormula(skeleton_id) => {
                self.process_predicate(*skeleton_id, node, store)
            }

            // Cas d'une initialisation de fonction (ex: (= (fuel-level car) 100))
            ExprEntryKind::Comparison(op) => {
                if matches!(op, CompareOp::Equal) {
                    self.process_function(node, store)
                } else {
                    Ok(())
                }
            }

            _ => Ok(()),
        }
    }

    fn process_predicate(
        &mut self,
        skeleton_id: AtomSkeletonId,
        node: ExprNodeRef<'_>,
        store: &ExprStore,
    ) -> Result<(), InertiaRegistryError> {
        let children = node.children();

        // On ne traite que les prédicats qui ne changent jamais (Inerte Positif)
        // pour peupler notre table de comptage IPP.
        if self.inertia.is_predicate_positive_inertia(skeleton_id)? {
            // Règle : children[0] est le symbole, les arguments commencent à l'index 1.
            let arity = children.len().saturating_sub(1);
            let mut args = Vec::with_capacity(arity);

            // On parcourt les enfants à partir de l'index 1 (les arguments)
            for (i, &arg_id) in children.iter().enumerate().skip(1) {
                let arg_entry = store.fetch(arg_id)?;

                // Dans l'état initial, les arguments doivent être des objets (constantes)
                if let ExprEntryKind::Object(obj_id) = arg_entry.kind() {
                    args.push(*obj_id);
                } else {
                    // Si l'argument n'est pas un Object valide (ex: une variable résiduelle),
                    // on log l'erreur et on ignore ce fait mal formé.
                    println!(
                        "[ERREUR-INIT] Prédicat {:?} : l'enfant {} (ExprId {:?}) n'est pas un Object (Kind: {:?})",
                        skeleton_id, i, arg_id, arg_entry.kind()
                    );
                    return Ok(());
                }
            }

            // --- LE LOG DE VÉRITÉ ---
            // Crucial pour vérifier que les faits du domaine sont bien capturés.
            println!(
                "[INIT-REGISTRY] Succès : Predicate {:?} | Args: {:?}",
                skeleton_id, args
            );

            // On délègue la génération des masques de bits pour l'instanciation partielle
            self.generate_predicate_masks(skeleton_id, arity, &args);
        }

        Ok(())
    }

    fn process_function(
        &mut self,
        node: ExprNodeRef<'_>, // Le nœud Comparison(Equal)
        store: &ExprStore,
    ) -> Result<(), InertiaRegistryError> {
        let children = node.children();

        // Dans un Comparison(Equal), on s'attend à 2 enfants réels (LHS et RHS).
        // Si ton Comparison suit aussi la règle "premier fils = symbole",
        // alors children[0] est le symbole '=', children[1] est le LHS, children[2] est le RHS.
        // D'après ton code original, tu accèdes à [0] et [1], donc le Comparison
        // n'a probablement PAS le symbole en enfant, contrairement aux Atoms/Functions.
        if children.len() < 2 {
            return Ok(());
        }

        // 1. Analyse du FunctionTerm (LHS)
        let lhs_id = children[0];
        let lhs_entry = store.fetch(lhs_id)?;

        // On vérifie que c'est bien une fonction
        if let ExprEntryKind::Function(func_id) = lhs_entry.kind() {
            let func_id = *func_id;

            // On ne traite que si la fonction est inerte positive
            if self.inertia.is_function_positive_inertia(func_id)? {
                // 2. Extraction des arguments de la fonction (LHS)
                let func_children = lhs_entry.children();

                // RÈGLE : func_children[0] est le symbole de fonction, on l'ignore.
                let arity = func_children.len().saturating_sub(1);
                let mut args = Vec::with_capacity(arity);

                for &arg_id in func_children.iter().skip(1) {
                    let arg_entry = store.fetch(arg_id)?;
                    if let ExprEntryKind::Object(obj_id) = arg_entry.kind() {
                        args.push(*obj_id);
                    } else {
                        // Si un argument n'est pas un objet constant, on ignore.
                        return Ok(());
                    }
                }

                // 3. Extraction de la valeur (RHS : children[1])
                let rhs_id = children[1];
                let rhs_entry = store.fetch(rhs_id)?;

                let value = match rhs_entry.kind() {
                    ExprEntryKind::Number(n) => StaticValue::Number(*n),
                    ExprEntryKind::Object(obj_id) => StaticValue::Object(*obj_id),
                    _ => {
                        // Si la valeur n'est ni un nombre ni un objet, structure invalide pour l'init
                        return Ok(());
                    }
                };

                // 4. Enregistrement pour l'analyse d'inertie
                println!(
                    "[INIT-REGISTRY] Function Success: {:?} | Args: {:?} | Val: {:?}",
                    func_id, args, value
                );

                self.generate_function_masks(func_id, arity, &args, value);
            }
        }

        Ok(())
    }

    /// Évalue une formule atomique selon les règles de simplification du papier IPP (Section 3.2).
    fn evaluate_predicate_internal(
        &self,
        node: ExprNodeRef<'_>,
        store: &ExprStore,
        buffer: &mut ArgumentBuffer,
    ) -> Result<Option<bool>, InertiaRegistryError> {
        // Dans le nouveau LIR, l'ID du squelette est porté par le Kind
        let pred_id = match node.kind() {
            ExprEntryKind::AtomicFormula(id) => *id,
            _ => return Ok(None),
        };

        // 1. Sécurité ID : On ne traite pas les prédicats générés dynamiquement (hors définitions PDDL)
        if pred_id.as_usize() >= self.predicate_defs.len() {
            return Ok(None);
        }

        // 2. Détection de l'inertie (Section 3.4)
        let is_negative = self.inertia.is_predicate_negative_inertia(pred_id)?;
        let is_positive = self.inertia.is_predicate_positive_inertia(pred_id)?;

        // Si le prédicat n'est PAS inerte, c'est un Fluent (il change).
        // On doit retourner None pour qu'il soit géré dynamiquement plus tard.
        if !is_negative && !is_positive {
            return Ok(None);
        }

        // --- ÉTAPE A : Calcul de N(p, ~a) ---
        // Rappel : extract_mask_dynamic saute l'index 0 (symbole)
        let mask = self.extract_mask_dynamic(node, store, buffer);

        // Vérification de la limite de projection (max_proj)
        let bit_count = (mask as u32).count_ones() as usize;
        if mask != 0 && bit_count > self.max_proj {
            // Trop de constantes pour nos tables pré-calculées
            return Ok(None);
        }

        let lookup_slice = buffer.as_slice();

        // Debug ciblé (optionnel)
        if pred_id.as_usize() == 2 {
            // println!("[LOOKUP-DEBUG] Predicate ID 2 | Mask: {:b} | Args: {:?}", mask, lookup_slice);
        }

        // Récupération de la valeur N(p, a) dans les tables de comptage
        let n_p_a = self
            .counting_predicates
            .get(&pred_id)
            .and_then(|masks| masks.get(&mask))
            .and_then(|entries| entries.get(lookup_slice))
            .copied();

        // --- ÉTAPE B : Application de la Définition 6 (Simplifications Atomiques) ---
        let n_val = n_p_a.unwrap_or(0);
        let grounded = self.all_args_grounded(node, store);

        // Règle 1 : Inertie Positive (ex: "at", "connected")
        // Ces faits sont vrais au début et ne peuvent que devenir FAUX (ou rester vrais).
        // Mais en PDDL standard, l'inertie positive signifie qu'ils ne changent JAMAIS.
        if is_positive {
            if n_val == 0 {
                // Si aucune instance n'existe dans l'init avec ces constantes -> Toujours FAUX
                return Ok(Some(false));
            }

            // Si totalement instancié et trouvé dans l'init -> Toujours VRAI
            if grounded && n_val > 0 {
                return Ok(Some(true));
            }

            return Ok(None);
        }

        // Règle 2 : Inertie Négative
        // Faits qui ne sont jamais vrais au début mais peuvent le devenir (non-applicable ici),
        // OU faits "statiques" qui couvrent tout leur domaine.
        if is_negative {
            let max_val = self.calculate_max_instances(node, store)?;

            if n_val == max_val {
                // Si le nombre d'instances vraies est égal au maximum possible -> Toujours VRAI
                return Ok(Some(true));
            }

            if grounded && n_val == 0 {
                // Si grounded et non trouvé -> Toujours FAUX
                return Ok(Some(false));
            }

            return Ok(None);
        }

        Ok(None)
    }

    /// Calculates MAX(p, ~a) according to Definition 5 of the IPP paper.
    /// MAX is the total number of ground instances consistent with the types
    /// that unify with the argument vector ~a.
    fn calculate_max_instances(
        &self,
        node: ExprNodeRef<'_>,
        store: &ExprStore,
    ) -> Result<usize, InertiaRegistryError> {
        let mut max_val: usize = 1;
        let children = node.children();

        // Extraction de l'ID du prédicat depuis le Kind du nœud
        if let ExprEntryKind::AtomicFormula(pred_id) = node.kind() {
            // On récupère la définition du prédicat pour avoir accès aux types des paramètres
            if let Some(def) = self.predicate_defs.get(pred_id.as_usize()) {
                let arg_types = def.parameters();

                // RÈGLE : children[0] est le symbole. Les arguments commencent à l'index 1.
                // On utilise .enumerate() après le .skip(1) pour que i=0 corresponde au 1er argument.
                for (i, &child_id) in children.iter().skip(1).enumerate() {
                    let child_entry = store.fetch(child_id)?;

                    // V(~a) est l'ensemble des positions occupées par des variables.
                    if let ExprEntryKind::Variable(_) = child_entry.kind() {
                        // On récupère le type attendu pour cette position
                        if let Some(param) = arg_types.get(i) {
                            let type_id = param.ty();

                            // On multiplie par la taille du domaine du type |dom(Ti)|
                            let domain_size = self.value_registry.get_type_domain(type_id)?.len();
                            max_val *= domain_size;
                        }
                    }
                    // Si c'est un Object (constante), la position est fixée, on ne multiplie par rien (x1).
                }
            }
        }

        // Si l'atome est totalement instancié ou n'a pas d'arguments, le produit vaut 1.
        Ok(max_val)
    }

    fn evaluate_function_internal(
        &self,
        node: ExprNodeRef<'_>, // Le nœud de type Function(id)
        store: &ExprStore,
        buffer: &mut ArgumentBuffer,
    ) -> Result<Option<StaticValue>, InertiaRegistryError> {
        // Dans le nouveau LIR, l'ID est dans le Kind
        let func_id = match node.kind() {
            ExprEntryKind::Function(id) => *id,
            _ => return Ok(None),
        };

        // 1. Check d'inertie : Si la fonction peut changer (fluent), on ne simplifie rien ici.
        if !self.inertia.is_function_positive_inertia(func_id)? {
            return Ok(None);
        }

        // 2. Extraction du masque et des arguments constants
        // On rappelle que extract_mask_dynamic saute l'index 0 (le symbole)
        let mask = self.extract_mask_dynamic(node, store, buffer);

        // Protection contre les projections trop larges (IPP Section 3.4)
        let n_limit = buffer.len().min(self.max_proj);
        let lookup_slice = &buffer[..n_limit];

        // 3. Recherche de la valeur dans le registre statique construit au build()
        let mut value = self
            .static_functions
            .get(&func_id)
            .and_then(|masks| masks.get(&mask))
            .and_then(|entries| entries.get(lookup_slice))
            .copied();

        // 4. Logique de décision et Fallback PDDL
        if self.all_args_grounded(node, store) {
            if value.is_none() {
                // Si aucune valeur n'est trouvée dans l'init pour une fonction totalement instanciée
                let def = &self.function_defs[func_id.as_usize()];

                // Standard PDDL : une fonction numérique non initialisée vaut 0.0 par défaut
                if def.ty().is_number() {
                    value = Some(StaticValue::Number(OrderedFloat(0.0)));
                } else {
                    // Pour les fonctions d'objets (Object-Fluents), on renvoie None.
                    // Note : Si ton pipeline a déjà effectué le "object-fluent flattening",
                    // il ne devrait plus rester de fonctions d'objets ici.
                    value = None;
                }
            }
            return Ok(value);
        } else {
            // Cas avec Variables (instanciation partielle) :
            // On ne simplifie pas sans analyse d'unanimité (non implémentée pour l'instant).
            Ok(None)
        }
    }

    // --- Internal Helpers ---

    /// Retourne vrai si la formule atomique ou le terme de fonction est totalement instancié (aucune variable).
    ///
    /// Un terme "grounded" peut être simplifié en une constante s'il est présent
    /// dans le registre de l'état initial.
    fn all_args_grounded(&self, node: ExprNodeRef<'_>, store: &ExprStore) -> bool {
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
                if let ExprEntryKind::Variable(_) = child_entry.kind() {
                    return false;
                }
            }
        }

        true
    }

    /// Checks if the problem's predicates and functions exceed the evaluator's capacity.
    fn check_limits(
        max_arity: usize,
        problem: &NewLiftedProblem,
    ) -> Result<(), InertiaRegistryError> {
        for (i, p) in problem.predicate_defs().iter().enumerate() {
            if p.arity() > max_arity {
                return Err(InertiaRegistryError::predicate_arity_too_high(
                    AtomSkeletonId::from(i),
                    p.arity(),
                ));
            }
        }
        for (i, f) in problem.function_defs().iter().enumerate() {
            if f.arity() > max_arity {
                return Err(InertiaRegistryError::function_arity_too_high(
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
    fn extract_mask_dynamic(
        &self,
        node: ExprNodeRef<'_>,
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
                    ExprEntryKind::Object(obj_id) => {
                        // Encodage Big Endian :
                        // i=0 (1er arg) -> bit (n_args - 1)
                        // i=(n_args-1)  -> bit 0
                        mask |= 1 << (n_args - 1 - i);
                        buffer.push(*obj_id);
                    }
                    // Si c'est une Variable, on ne fait rien (le bit reste à 0).
                    // C'est le cœur de l'instanciation partielle d'IPP.
                    ExprEntryKind::Variable(_) => {}

                    _ => {}
                }
            }
        }
        mask
    }

    pub fn generate_predicate_masks(
        &mut self,
        key: AtomSkeletonId,
        arity: usize,
        args: &[ObjectId],
    ) {
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

    /// Retourne true si le fait doit être inclus dans le BitVector d'état.
    ///
    /// Selon la section 3.4 du papier IPP, un fait est un **fluent** s'il n'est PAS une inertie.
    /// Si cette fonction renvoie `false`, le fait est considéré comme une constante
    /// (toujours vrai ou toujours faux) et ne doit pas consommer de bit dans l'état.
    pub fn is_fluent(&self, pred_id: AtomSkeletonId) -> bool {
        // 1. Sécurité : Si l'ID est hors des définitions connues, ce n'est pas un fluent
        // géré par le domaine (ex: prédicats synthétiques, égalités).
        if pred_id.as_usize() >= self.predicate_defs.len() {
            return false;
        }

        // 2. Un prédicat est statique (inertie) s'il est marqué dans la table d'inertie.
        // On récupère les deux types d'inertie (positve et négative).
        let is_static = self
            .inertia
            .is_predicate_positive_inertia(pred_id)
            .unwrap_or(false)
            || self
                .inertia
                .is_predicate_negative_inertia(pred_id)
                .unwrap_or(false);

        // 3. Si ce n'est pas statique, c'est un fluent (dynamique).
        !is_static
    }
}

impl<'a> StaticEvaluator for InertiaEvaluator<'a> {
    fn evaluate(&self, expr: Expr<'_>) -> Option<StaticValue> {
        // 1. On récupère directement le node_ref via le old
        // Si old.fetch(id) renvoie déjà un ExprNodeRef, on l'utilise tel quel.
        let node_ref = expr.store().fetch(expr.root_id()).ok()?;

        // On récupère le old pour les appels internes
        let store = expr.store();

        // 2. Préparation du buffer
        let mut buffer = ArgumentBuffer::new();

        // 3. Dispatch (on utilise node_ref directement)
        let result = match node_ref.kind() {
            ExprEntryKind::AtomicFormula(_) => self
                .evaluate_predicate_internal(node_ref, store, &mut buffer)
                .ok()
                .flatten()
                .map(StaticValue::Boolean),

            ExprEntryKind::Function(_) => self
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

//#[cfg(test)]
//#[path = "tests/evaluator_tests.rs"]
//mod evaluator_tests;
