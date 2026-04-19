use crate::aiplan4rust::arena::{ArenaNode, NodeId};
use crate::aiplan4rust::grounding::analysis::inertia::evaluator::InertiaRegistryError;
use crate::aiplan4rust::grounding::analysis::inertia::table::InertiaTable;
use crate::aiplan4rust::grounding::problem::registry::value::ValueRegistry;
use crate::aiplan4rust::lang::{AtomSkeletonId, FunctionSkeletonId, ObjectId};
use crate::aiplan4rust::lir::expr::ops::{StaticEvaluator, StaticValue};
use crate::aiplan4rust::lir::expr::{Expr, ExprKind, ExprNode};
use crate::aiplan4rust::lir::problem::atomic_skeleton::{
    AtomicFormulaSkeleton, AtomicFunctionSkeleton,
};
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::tree::Node;
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
        init: &Expr,
        inertia: &'a InertiaTable,
        value_registry: &'a ValueRegistry,
        max_arity: usize,
        max_proj: usize,
    ) -> Result<Self, InertiaRegistryError> {
        // 1. On CLONE les définitions dans des Box (Zéro lifetime 'a sur Problem)
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

        // 2. Traitement de l'init (on utilise init_expr passé en argument)
        let mut iter = init.preorder().values();
        while let Some(node) = iter.next() {
            match node.kind() {
                ExprKind::AtomicFormula | ExprKind::Comparison => {
                    registry.process_init(node, init)?;
                    iter.skip_subtree();
                }
                ExprKind::Not => {
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
            ExprKind::Comparison => self.process_function(node, init),
            _ => Ok(()),
        }
    }

    fn process_predicate(
        &mut self,
        node: &ExprNode,
        init: &Expr,
    ) -> Result<(), InertiaRegistryError> {
        let children = node.children();

        // Ton analyse est juste : le skeleton_id est lié à l'AtomicFormula
        let skeleton_id = node.try_atom_skeleton()?;

        // On ne traite que les prédicats qui ne changent jamais (Inerte Positif)
        if self.inertia.is_predicate_positive_inertia(skeleton_id)? {
            let arity = children.len().saturating_sub(1);
            let mut args = Vec::with_capacity(arity);

            // On parcourt les enfants à partir de l'index 1 (les arguments)
            for (i, &arg_id) in children.iter().enumerate().skip(1) {
                let arg_node = init.try_node(arg_id)?;

                // Tentative d'extraction de l'ID de l'objet
                match arg_node.try_object() {
                    Ok(obj_id) => args.push(obj_id),
                    Err(_) => {
                        // Si on arrive ici, l'argument n'est pas un ObjectID valide
                        println!("[ERREUR-INIT] Prédicat {:?} : l'enfant {} (NodeId {:?}) n'est pas un Object (Kind: {:?})",
                                 skeleton_id, i, arg_id, arg_node.kind());
                        return Ok(()); // On ignore ce fait mal formé
                    }
                }
            }

            // --- LE LOG DE VÉRITÉ ---
            // Si ce log n'apparaît pas pour les IDs 2, 5, 7, 8, 9,
            // alors la table reste vide et l'évaluateur renverra toujours False.
            println!(
                "[INIT-REGISTRY] Succès : Predicate {:?} | Args: {:?}",
                skeleton_id, args
            );

            self.generate_predicate_masks(skeleton_id, arity, &args);
        }
        Ok(())
    }

    fn process_function(
        &mut self,
        node: &ExprNode,
        init: &Expr,
    ) -> Result<(), InertiaRegistryError> {
        let children = node.children();
        // 1. On récupère la définition de la fonction (le squelette)
        let func_id = node.try_function_skeleton()?;

        if self.inertia.is_function_positive_inertia(func_id)? {
            // 2. Dans un FComp (=), le premier enfant (children[0]) est le BasicFunctionTerm
            let func_term_node = init.try_node(children[0])?;
            let func_children = func_term_node.children();

            // L'arité exclut le symbole de la fonction (le nom) à l'index 0
            let arity = func_children.len().saturating_sub(1);

            // 3. Extraction des arguments de la fonction (ex: le 'x' dans '(f x)')
            let mut args = Vec::with_capacity(arity);
            for &arg_id in &func_children[1..] {
                args.push(init.try_node(arg_id)?.try_object()?);
            }

            // 4. Extraction de la valeur (le membre de droite du '=' : children[1])
            let val_node = init.try_node(children[1])?;
            let value = if let Ok(num) = val_node.try_number() {
                StaticValue::Number(num)
            } else {
                StaticValue::Object(val_node.try_object()?)
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

        // --- MODIFICATION 1 : Sécurité ID (Correction IDs 10/13) ---
        // Si l'ID est supérieur au nombre de définitions, c'est un prédicat
        // généré (égalité, etc.). On ne le simplifie pas ici.
        if pred_id.as_usize() >= self.predicate_defs.len() {
            return Ok(None);
        }

        // --- MODIFICATION 2 : Détection des Fluents (Section 3.4) ---
        let is_negative = self.inertia.is_predicate_negative_inertia(pred_id)?;
        let is_positive = self.inertia.is_predicate_positive_inertia(pred_id)?;

        // Si le prédicat n'est PAS inerte, c'est un Fluent.
        // On doit retourner None pour qu'il soit conservé dans le BitVector.
        if !is_negative && !is_positive {
            return Ok(None);
        }

        // --- ÉTAPE A : Calcul de N(p, ~a) ---
        let mask = self.extract_mask_dynamic(node, expr, buffer);

        // NOUVEAU : Vérification de la limite de projection
        let bit_count = (mask as u32).count_ones() as usize;
        if mask != 0 && bit_count > self.max_proj {
            // On a trop de constantes par rapport à ce qu'on a pré-calculé.
            // On ne peut pas simplifier, on renvoie None au lieu de laisser n_p_a tomber à 0.
            return Ok(None);
        }

        // NOUVEAU : On utilise tout le buffer (lookup_slice doit correspondre exactement au masque)
        let lookup_slice = buffer.as_slice();

        // On cible le prédicat 'requires' (ID 2)
        if pred_id.as_usize() == 2 {
            println!("[LOOKUP-DEBUG] Predicate: requires");
            println!("  -> Mask (bin): {:b}", mask);
            println!("  -> Args in Buffer: {:?}", lookup_slice);

            // Test manuel : Est-ce que le prédicat existe avec ce masque ?
            let has_mask = self
                .counting_predicates
                .get(&pred_id)
                .map(|m| m.contains_key(&mask))
                .unwrap_or(false);
            println!("  -> Mask exists in table? {}", has_mask);
        }

        let n_p_a = self
            .counting_predicates
            .get(&pred_id)
            .and_then(|masks| masks.get(&mask))
            .and_then(|entries| entries.get(lookup_slice))
            .copied();

        // --- ÉTAPE B : Application rigoureuse de la Définition 6 ---
        let n_val = n_p_a.unwrap_or(0);
        let grounded = self.all_args_grounded(node, expr);

        // Règle 1 : Positive Inertia (ex: requires, part-of)
        if is_positive {
            if n_val == 0 {
                // "If p is a positive inertia and N(p, a) = 0 then simplified to FALSE"
                return Ok(Some(false));
            }

            // Cas particulier : si c'est totalement instancié (grounded)
            // et que n_val > 0, alors c'est forcément 1 (Vrai).
            if grounded && n_val > 0 {
                return Ok(Some(true));
            }

            // "In all other cases (p, a) cannot (yet) be simplified"
            // (Cela inclut le cas N > 0 avec des variables)
            return Ok(None);
        }

        // Règle 2 : Negative Inertia (ex: complete)
        if is_negative {
            let max_val = self.calculate_max_instances(node, expr)?;

            if n_val == max_val {
                // "If p is a negative inertia and N(p, a) = MAX(p, a) then simplified to TRUE"
                return Ok(Some(true));
            }

            if grounded && n_val == 0 {
                return Ok(Some(false));
            }

            return Ok(None);
        }

        // Si on arrive ici, on ne peut pas conclure avec certitude (ex: Inerte Négatif non-grounded)
        Ok(None)
    }

    /// Calcule MAX(p, ~a) selon la Définition 5 du papier IPP.
    /// MAX est le nombre de toutes les instances terrestres (ground instances)
    /// cohérentes avec les types qui unifient avec le vecteur d'arguments ~a.
    fn calculate_max_instances(
        &self,
        node: &ExprNode,
        expr: &Expr,
    ) -> Result<usize, InertiaRegistryError> {
        let mut max_val: usize = 1;
        let children = node.children();

        if let Ok(pred_id) = node.try_atom_skeleton() {
            // On récupère la signature (types des arguments) définie au build
            if let Some(arg_types) = self
                .predicate_defs
                .get(pred_id.as_usize())
                .map(|s| s.parameters())
            {
                // On itère sur les positions i de 1 à n
                for (i, &child_id) in children[1..].iter().enumerate() {
                    if let Ok(child_node) = expr.try_node(child_id) {
                        // V(~a) est l'ensemble des positions occupées par des variables.
                        // Pour chaque i appartenant à V(~a), on multiplie par |dom(Ti)|.
                        if child_node.kind() == ExprKind::Variable {
                            let type_id = arg_types[i].ty();
                            let domain_size = self.value_registry.get_type_domain(type_id)?.len();
                            max_val *= domain_size;
                        }
                    }
                }
            }
        }
        // Si l'atome est totalement instancié, V(~a) est vide, le produit vide vaut 1.
        Ok(max_val)
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
        if !self.inertia.is_function_positive_inertia(func_id)? {
            return Ok(None);
        }

        // 2. Extraction du masque et des arguments
        let mask = self.extract_mask_dynamic(node, expr, buffer);
        let n_limit = buffer.len().min(self.max_proj);
        let lookup_slice = &buffer[..n_limit];

        // 3. Recherche de la valeur injectée dans le registre 🔍
        let mut value = self
            .static_functions
            .get(&func_id)
            .and_then(|masks| masks.get(&mask))
            .and_then(|entries| entries.get(lookup_slice))
            .copied();

        // 4. Logique de décision et Fallback PDDL
        if self.all_args_grounded(node, expr) {
            if value.is_none() {
                // Si aucune valeur n'est trouvée, on vérifie le typing de retour
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

    /// Checks if the problem's predicates and functions exceed the evaluator's capacity.
    fn check_limits(max_arity: usize, problem: &LiftedProblem) -> Result<(), InertiaRegistryError> {
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
    fn extract_mask_dynamic(
        &self,
        node: &ExprNode,
        expr: &Expr,
        buffer: &mut ArgumentBuffer,
    ) -> u16 {
        buffer.clear();
        let children = node.children();
        if children.len() <= 1 {
            return 0;
        }

        let args = &children[1..];
        let mut mask = 0u16;

        for (i, &arg_id) in args.iter().enumerate() {
            if let Ok(arg_node) = expr.try_node(arg_id) {
                // ON VÉRIFIE LE GENRE AVANT D'ESSAYER D'EXTRAIRE
                if arg_node.kind() == ExprKind::Object {
                    // Ici, try_constant() ne peut PAS échouer
                    if let Ok(obj) = arg_node.try_object() {
                        mask |= 1 << (args.len() - 1 - i);
                        buffer.push(obj);
                    }
                }
                // Si c'est ExprKind::Variable, on ne fait rien (le bit reste à 0)
                // C'est exactement ce que demande le papier IPP.
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
    fn evaluate(&self, node_id: NodeId, expr: &Expr) -> Option<StaticValue> {
        let node = expr.try_node(node_id).ok()?;
        let mut buffer = ArgumentBuffer::new();

        let result = match node.kind() {
            ExprKind::AtomicFormula => self
                .evaluate_predicate_internal(node_id, expr, &mut buffer)
                .ok()
                .flatten()
                .map(StaticValue::Boolean),
            ExprKind::Function => self
                .evaluate_function_internal(node_id, expr, &mut buffer)
                .ok()
                .flatten(),
            _ => None,
        };

        // --- LE DEBUG ---
        if let Some(val) = &result {
            // On n'affiche que si c'est un atome (pour éviter de polluer avec les constantes brutes)
            if node.kind() == ExprKind::AtomicFormula {
                // Utilise ta méthode pour récupérer le nom du prédicat si possible
                println!("[Inertia] Evaluated Node {:?} -> {:?}", node_id, val);
            }
        }

        result
    }
}

/*impl<'a> StaticEvaluator for InertiaEvaluator<'a> {
    fn evaluate(&self, node_id: NodeId, logic: &Expr) -> Option<StaticValue> {
        let node = logic.try_node(node_id).ok()?;
        let mut buffer = ArgumentBuffer::new();

        match node.kind() {
            ExprKind::AtomicFormula => {
                // 1. try_atom_skeleton est inchangé, il renvoie l'ID brut (tagué ou non)
                let id = node.try_atom_skeleton().ok()?;

                // 2. On appelle ta fonction originale.
                // À l'intérieur, quand elle fait id.as_usize(), le bit MSB est ignoré.
                // Donc elle calcule toujours la vérité du fait "positif".
                let res = self.evaluate_predicate_internal(node_id, logic, &mut buffer)
                    .ok()
                    .flatten();

                // 3. ICI on applique la logique de négation si le bit était présent
                res.map(|b| {
                    let final_bool = if id.is_negated() { !b } else { b };
                    StaticValue::Boolean(final_bool)
                })
            }
            ExprKind::Function => {
                self.evaluate_function_internal(node_id, logic, &mut buffer)
                    .ok()
                    .flatten()
            }
            _ => None,
        }
    }
}*/

#[cfg(test)]
#[path = "tests/evaluator_tests.rs"]
mod evaluator_tests;
