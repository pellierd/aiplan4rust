use crate::aiplan4rust::grounding::analysis::reachability::datalog::atom::Atom;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::error::DatalogError;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::rule::Rule;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::term::Term;
use crate::aiplan4rust::lang::{ActionSymbolId, CompareOp};
use crate::aiplan4rust::lang::{
    AtomSkeletonId, PredicateSymbolId, Type, TypeId, TypedList, TypedSymbol, VariableId,
};
use crate::aiplan4rust::lir::old::expr::{Expr, ExprError, ExprKind, ExprNode};
use crate::aiplan4rust::lir::old::problem::atomic_skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::lir::ActionDef;
use crate::aiplan4rust::tree::{NodeId, SyntaxContent};
use crate::analysis::reachability::datalog::cause::Cause;
use std::collections::HashMap;
////// ATENTION JE NE GERE PAS les AXIOMS

/// A transformation engine that encodes complex PDDL formulas into Datalog rules.
///
/// The `DatalogEncoder` is responsible for **Normalization** and the final flattening
/// of logical connectives into Horn clauses.
///
/// # Pre-conditions (Crucial)
///
/// For this encoder to function correctly, the input [`Expr`] must have already passed
/// through the following transformation finalization (see `crate::finalization`):
///
/// 1. **Quantifier Expansion**: All `FORALL` and `EXISTS` nodes must be expanded into
///    their respective `AND`/`OR` equivalent grounded structures.
/// 2. **Type Flattening**: The PDDL typing hierarchy must be flattened. Datalog operates
///    on flat sets; any complex inheritance must be resolved amont.
/// 3. **Object Fluent Flattening**: Functions (object fluents) must be converted into
///    relational predicates (e.g., `(at ?robot (location_of ?target))` -> `(at ?robot ?loc) ^ (is_at ?target ?loc)`).
///
/// # Internal Architecture
///
/// - **ID Space Separation**: Clearly partitions the predicate ID space into "Base"
///   (original PDDL domain) and "Auxiliary" (generated during encoding).
/// - **Structural Deduplication**: Uses a cache to ensure that logically identical
///   sub-formulas within the same context are mapped to the same auxiliary predicate,
///   minimizing the number of rules.
/// - **Schema Tracking**: Maintains a monotonic evaluator of generated signatures
///   ([`AtomicFormulaSkeleton`]), allowing the grounded results to be mapped back
///   to human-readable names after saturation.
pub struct DatalogEncoder {
    /// The starting offset for auxiliary predicate IDs.
    /// Typically set to the count of original predicates in the domain.
    base_aux_id: usize,
    /// Monotonic counter for generating the next unique auxiliary ID.
    next_aux_id: usize,
    /// Registry of auxiliary predicate signatures (skeletons).
    /// Used for debugging and reconstructing the logical state post-saturation.
    aux_defs: Vec<AtomicFormulaSkeleton>,
    /// Structural cache mapping a set of body atoms to a head atom.
    /// Prevents the redundant creation of multiple auxiliary predicates
    /// for the same logical sub-expression (Common Subexpression Elimination).
    cache: HashMap<Vec<Atom>, Atom>,

    // Ajout du champ interne
    // On utilise un champ membre pour éviter de le passer partout
    current_aliases: HashMap<VariableId, Term>,

    /// Table de causalité : associe chaque effet à son origine (Action ou Pivot).
    action_effects: Vec<Vec<(Atom, Cause)>>,

    action_anchor: Option<Atom>,

    negation_offset: usize,
    type_to_skeleton: Vec<AtomSkeletonId>,
}

impl DatalogEncoder {
    /// Initializes a new Datalog Encoder.
    ///
    /// # Arguments
    ///
    /// * `base_id` - The starting index for auxiliary predicates. This should
    ///   begin after the last standard predicate ID in the PDDL domain to
    ///   avoid ID collisions.
    /// * `action_count` - The total number of actions in the domain. Used to
    ///   pre-allocate the causality table.
    /// * `negation_offset` - The offset used to derive negated fluent IDs.
    ///
    /// # Process
    ///
    /// The encoder manages two distinct predicate ID spaces:
    /// 1. **Base Predicates**: IDs below `base_id`, which correspond to the
    ///    original domain symbols resolved via the global interner.
    /// 2. **Auxiliary Predicates**: IDs starting from `base_id`, which are
    ///    generated during the encoding process (e.g., for actions, types, or
    ///    logical connectives).
    ///
    /// # Causality Tracking
    ///
    /// The `action_effects` table is initialized as a dense `Vec` of size `action_count`.
    /// Since `ActionDefId`s are contiguous and zero-based, this allows for **O(1)
    /// mapping** between a derived action atom and its original lifted effects
    /// during the grounding phase, avoiding expensive hash lookups.
    ///
    /// # Performance
    ///
    /// This constructor pre-allocates space for auxiliary definitions, the structural
    /// cache, and the causality table. This strategy minimizes heap reallocations
    /// and ensures that causality data is stored in a cache-friendly, contiguous
    /// memory layout.
    pub fn new(
        base_id: usize,
        action_count: usize,
        negation_offset: usize,
        type_to_skeleton: Vec<AtomSkeletonId>,
    ) -> Self {
        Self {
            base_aux_id: base_id,
            next_aux_id: base_id,
            // Pre-allocation strategy to handle typical PDDL domain complexity
            aux_defs: Vec::with_capacity(256),
            cache: HashMap::with_capacity(256),
            current_aliases: HashMap::with_capacity(256),
            // Dense table for O(1) causality lookups
            action_effects: vec![Vec::new(); action_count],
            action_anchor: None,
            negation_offset,
            type_to_skeleton,
        }
    }

    /// Retourne la tranche (slice) d'effets pour l'index d'action donné.
    pub fn get_action_effects(&self, action_index: usize) -> &[(Atom, Cause)] {
        self.action_effects
            .get(action_index)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn reset_with_start_id(&mut self, start_id: usize) {
        // On aligne les deux compteurs sur le nouveau seuil (après les actions)
        self.base_aux_id = start_id;
        self.next_aux_id = start_id;
    }

    // Dans DatalogEncoder
    pub fn current_id(&self) -> usize {
        self.next_aux_id
    }
    /// Encodes a PDDL Type as a unary Datalog predicate and maintains a semantic mapping.
    /// Encodes a PDDL Type as a unary Datalog predicate using sequential allocation.
    ///
    /// This function is a core component of the **ID Segmentation** strategy. Type IDs
    /// are allocated contiguously, enabling O(1) conversion between Datalog
    /// `AtomSkeletonId` and PDDL `TypeId` through pointer-free arithmetic.
    ///
    /// # Returns
    ///
    /// The unique [`AtomSkeletonId`] representing this typing.
    /// The mapping to the original `TypeId` is implicit:
    /// `TypeId = sk_id - fluence_threshold`.
    ///
    /// # Process
    ///
    /// 1. **Monotonic Allocation**: Uses the `next_aux_id` counter to ensure the ID
    ///    falls within the reserved segment for types.
    /// 2. **Signature Definition**: Creates a unary predicate schema `(type_name ?v0)`.
    ///    The argument `?v0` uses `Type::root()` because this predicate itself
    ///    defines the domain membership for objects.
    /// 3. **Schema Consistency**: Registers the skeleton in `aux_defs` to allow the
    ///    rule compiler to verify predicate arity (always 1 for types).
    #[inline]
    pub fn encode_type_as_unary_predicate(&mut self) -> AtomSkeletonId {
        self.encode_auxiliary_predicate(1, None)
    }

    /// Crée un prédicat auxiliaire de manière flexible.
    /// - Si `types` est `Some`: utilise la liste fournie (zéro boucle inutile).
    /// - Si `types` est `None`: génère une signature générique de taille `arity`.
    pub fn encode_auxiliary_predicate(
        &mut self,
        arity: usize,
        types: Option<TypedList<VariableId, TypeId>>,
    ) -> AtomSkeletonId {
        let id = self.next_aux_id;
        self.next_aux_id += 1;

        let sk_id = AtomSkeletonId::from(id);
        let predicate_id = PredicateSymbolId::from(id);

        let final_parameters = match types {
            // Cas 1 : On a déjà les types (ex: une Action)
            Some(p) => p,
            // Cas 2 : On doit générer des types root (ex: une Union ou un AND)
            None => {
                let mut arguments = TypedList::new();
                for i in 0..arity {
                    arguments.push(TypedSymbol::new(VariableId::from(i), Type::root()));
                }
                arguments
            }
        };

        // Enregistrement unique
        self.aux_defs
            .push(AtomicFormulaSkeleton::new(predicate_id, final_parameters));

        sk_id
    }

    /// Encodes a lifted action as a Datalog predicate using sequential allocation.
    ///
    /// This allows the Datalog engine to represent action applicability as a relation.
    /// If the saturation process derives a fact for this predicate, the action is
    /// considered reachable with that specific grounding of parameters.
    ///
    /// # Arguments
    ///
    /// * `action` - A reference to the [`LiftedAction`] whose parameters define the
    ///   predicate's signature (arity and types).
    ///
    /// # Returns
    ///
    /// The unique [`AtomSkeletonId`] assigned to this action's predicate.
    /// The mapping to the original `ActionDefId` is implicit:
    /// `ActionDefId = sk_id - type_threshold`.
    ///
    /// # Process
    ///
    /// 1. **ID Allocation**: Reserves a unique ID within the action segment using
    ///    the `next_aux_id` counter.
    /// 2. **Signature Extraction**: Clones the action's typed parameters to define the
    ///    predicate's structure (arity and argument types) within the Datalog engine.
    /// 3. **Registration**: Stores the [`AtomicFormulaSkeleton`] in `aux_defs` to ensure
    ///    schema consistency during rule compilation.
    ///
    /// # Performance
    ///
    /// - **Complexity**: $O(P)$ where $P$ is the number of parameters (cloning overhead).
    /// - **Arithmetic**: Enables $O(1)$ decoding of reachable actions without hash lookups.
    #[inline]
    pub fn encode_action_as_predicate(&mut self, action: &ActionDef) -> AtomSkeletonId {
        let params = action.parameters().clone();
        self.encode_auxiliary_predicate(params.len(), Some(params))
    }

    /// Encodes action effects into Datalog rules by propagating causality from the action to its consequences.
    ///
    /// This function performs an iterative top-down traversal of the effect expression tree.
    /// It establishes a logical chain between the "cause" (the action atom) and the resulting
    /// "facts" (atomic formulas).
    ///
    /// # Conditional Effects (WHEN)
    ///
    /// When encountering a `When` node, the function:
    /// 1. Uses `encode_expr` to flatten the condition into an auxiliary atom.
    /// 2. Creates a "pivot" auxiliary predicate representing the conjunction of the action
    ///    and the condition.
    /// 3. Propagates this new auxiliary atom as the "cause" for all nested sub-effects.
    ///
    /// # Relaxed Semantics
    ///
    /// For reachability analysis, this encoder follows a positive-only Datalog model:
    /// * **Delete-effects (`Not`)** are explicitly ignored.
    /// * **Numerical effects** (Assign, Operation, etc.) are skipped as they do not contribute
    ///   to atomic fact reachability.
    ///
    /// # Arguments
    ///
    /// * `root_effect` - The expression tree representing the action's effects.
    /// * `action_atom` - The atom representing the execution of the action (the initial cause).
    /// * `rules_sink` - A vector where newly generated Datalog rules are stored.
    /// * `parameters` - The typed parameters of the action, used for auxiliary predicate signatures.
    ///
    /// # Errors
    ///
    /// Returns a [`DatalogError`] if:
    /// * An unsupported node kind is encountered (e.g., `Forall` or `Exists` not yet implemented).
    /// * There is a failure in auxiliary predicate generation or variable collection.
    pub fn encode_effects(
        &mut self,
        root_effect: &Expr,
        action_atom: &Atom,
        rules_sink: &mut Vec<Rule>,
        parameters: &TypedList<VariableId, TypeId>,
        action_index: usize,
    ) -> Result<(), DatalogError> {
        // --- MODIFICATION : ALIASING SUR L'ACTION RACINE ---
        // On doit appliquer resolve_var sur l'atome d'action initial pour que
        // toutes les "causes" utilisent les représentants canoniques.
        let mut root_cause = action_atom.clone();
        for term in root_cause.terms_mut() {
            if let Term::Variable(v) = *term {
                *term = self.resolve_var(v);
            }
        }

        // 2. ON SAUVEGARDE L'ID ICI (il est Copy, donc pas de souci)
        let root_cause_id = root_cause.skeleton_id();

        // Work stack: (Node ID, Current Cause)
        let mut work_stack = vec![(root_effect.try_root_id()?, root_cause)];

        while let Some((node_id, current_cause)) = work_stack.pop() {
            let node = root_effect.try_node(node_id)?;
            let kind = node.kind();

            match kind {
                ExprKind::AtomicFormula => {
                    let mut effect_atom = self.extract_atom(root_effect, node)?;

                    // 1. Canonisation (Aliasing)
                    for term in effect_atom.terms_mut() {
                        if let Term::Variable(v) = *term {
                            *term = self.resolve_var(v);
                        }
                    }

                    // 2. Traçabilité (Causalité)
                    let cause = if current_cause.skeleton_id() == root_cause_id {
                        Cause::Action
                    } else {
                        Cause::Pivot(current_cause.clone())
                    };
                    self.action_effects[action_index].push((effect_atom.clone(), cause));

                    // 3. Règle Datalog : Effet :- Cause
                    // Simple, efficace, et préserve les variables.
                    rules_sink.push(Rule::new(effect_atom, vec![current_cause.clone()]));
                }

                // 2. Conjunction: Propagate the cause to all sub-effects
                ExprKind::And => {
                    for &child_id in node.children().iter().rev() {
                        work_stack.push((child_id, current_cause.clone()));
                    }
                }

                // 3. Conditional Effect: Create a pivot between Action and Condition
                // 3. Conditional Effect: Create a pivot between Action and Condition
                ExprKind::When => {
                    let children = node.children();
                    let condition_id = children[0];
                    let sub_effect_id = children[1];

                    // On encode la condition (peut renvoyer un atome auxiliaire ou un atome simple)
                    if let Some(cond_atom) =
                        self.encode_expr(root_effect, condition_id, rules_sink, parameters)?
                    {
                        // 1. Corps de la règle : on lie la cause actuelle ET la condition.
                        // C'est ce qui assure que toutes les variables sont "bindées".
                        let mut combined_body = vec![current_cause.clone(), cond_atom.clone()];
                        combined_body.sort_by_key(|a| a.skeleton_id());

                        let aux_when_atom = if let Some(existing_head) =
                            self.cache.get(&combined_body)
                        {
                            existing_head.clone()
                        } else {
                            // --- LA MAGIE EST ICI ---
                            // 2. Tête de la règle : on ne projette QUE les variables de la condition.
                            // Cela transforme l'arité 2 en 1 si seule ?v1 est utilisée dans la condition.
                            let head = self.encode_new_aux_predicate(&[cond_atom], parameters)?;

                            /*println!(
                                "  |_ Création pivot optimisé (arité réduite) : {:?}",
                                head.0.skeleton_id()
                            );*/

                            // 3. On enregistre la règle avec la tête légère et le corps complet.
                            rules_sink.push(Rule::new(head.0.clone(), combined_body.clone()));
                            self.cache.insert(combined_body, head.0.clone());
                            head.0
                        };

                        // On continue la propagation avec le nouveau pivot
                        work_stack.push((sub_effect_id, aux_when_atom));
                    } else {
                        // Condition triviale : on passe directement la cause aux sous-effets
                        work_stack.push((sub_effect_id, current_cause));
                    }
                }

                // 4. Temporal Wrappers: Simply traverse through
                ExprKind::AtStart | ExprKind::AtEnd | ExprKind::Overall => {
                    if let Some(&child_id) = node.children().first() {
                        work_stack.push((child_id, current_cause));
                    }
                }

                // 5. Explicitly Ignored Nodes (Numerical / Metrics)
                // --- MODIFICATION : AJOUT DE NOT ET COMPARISON ---
                ExprKind::Assignment | ExprKind::Arithmetic => {
                    continue;
                }

                // --- MODIFICATION PRÉCISE : BRANCH NOT ---
                // --- BRANCH NOT DANS encode_effects ---
                ExprKind::Not => {
                    let children = node.children();
                    if let Some(&child_id) = children.first() {
                        let child_node = root_effect.try_node(child_id)?;
                        if child_node.kind() == ExprKind::AtomicFormula {
                            let mut del_atom = self.extract_atom(root_effect, child_node)?;

                            for term in del_atom.terms_mut() {
                                if let Term::Variable(v) = *term {
                                    *term = self.resolve_var(v);
                                }
                            }

                            del_atom.set_negated(true);
                            let raw_id: usize = del_atom.skeleton_id().into();
                            del_atom.set_skeleton_id(AtomSkeletonId::from(
                                raw_id + self.negation_offset,
                            ));

                            let cause = if current_cause.skeleton_id() == root_cause_id {
                                Cause::Action
                            } else {
                                Cause::Pivot(current_cause.clone())
                            };

                            // ON GARDE ÇA : Utile pour ton Datalogologue/BitVector final
                            self.action_effects[action_index].push((del_atom.clone(), cause));

                            // ON SUPPRIME ÇA (ou on commente) :
                            // C'est ça qui crée la règle en trop dans le rules_sink !
                            //rules_sink.push(Rule::new(del_atom, vec![current_cause.clone()]));
                        }
                    }
                }

                // --- FEATURES (VALIDE PDDL MAIS NÉCESSITE PREPROCESSING) ---
                // Si l'un de ceux-là arrive ici, c'est l'Expander/PNF qui est en cause.
                ExprKind::Forall | ExprKind::Exists | ExprKind::Imply => {
                    return Err(DatalogError::feature_not_supported(
                        format!("ADL construct {:?} in effects", kind),
                        node_id,
                    ));
                }

                // 6. Safety: Any other node kind triggers an error (e.g., Forall, Exists)
                _ => {
                    return Err(DatalogError::incompatible_node(kind.clone(), node_id));
                }
            }
        }
        Ok(())
    }

    /// Encodes the preconditions of an action into Datalog atoms and rules.
    ///
    /// This function serves as the public entry point for flattening action preconditions.
    /// It delegates the iterative post-order traversal to `encode_expr`, starting from
    /// the root of the provided expression tree.
    ///
    /// Complex logical structures (AND/OR) are decomposed into auxiliary predicates
    /// using a Tseitin-like transformation to maintain a flat Horn-clause structure.
    ///
    /// # Arguments
    ///
    /// * `logic` - The expression tree representing the action's preconditions.
    /// * `rules_sink` - A vector where newly generated Datalog rules (auxiliary definitions) are stored.
    /// * `parameters` - The typed parameters of the action, used to define the signature of auxiliary predicates.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Atom))` - The head atom representing the unified precondition logic.
    /// * `Ok(None)` - If the expression is empty or contains only ignored nodes (e.g., empty AND).
    /// * `Err(DatalogError)` - If the expression tree is malformed or contains unsupported nodes.
    pub fn encode_preconditions(
        &mut self,
        expr: &Expr,
        rules_sink: &mut Vec<Rule>,
        parameters: &TypedList<VariableId, TypeId>,
    ) -> Result<Option<Atom>, DatalogError> {
        // 1. Nettoyage et préparation des alias (pour gérer les ?x = ?y)
        self.current_aliases = self.extract_variable_aliases(expr)?;

        // 2. Encodage récursif/itératif de l'expression
        // On retourne simplement l'atome racine (souvent un Aux_N)
        if let Some(root_id) = expr.root_id() {
            self.encode_expr(expr, root_id, rules_sink, parameters)
        } else {
            Ok(None)
        }
    }

    /// Génère un atome d'ancre unique pour une action.
    /// Cet atome sert de pivot statique (Facts inertes + Type Guards).
    pub fn generate_anchor_atom(
        &mut self,
        _action_id: ActionSymbolId, // Utile pour le debug/nommage futur
        _parameters: &TypedList<VariableId, TypeId>,
        action_head: &Atom,
    ) -> Atom {
        // 1. On alloue un nouvel ID auxiliaire via la méthode existante
        // L'arité de l'ancre est exactement celle de l'action
        let arity = action_head.terms().len();

        // On n'utilise pas encode_auxiliary_predicate ici car on veut
        // une gestion propre de l'ID sans forcément recréer une définition complexe
        let anchor_id = self.next_aux_id;
        self.next_aux_id += 1;

        let sk_id = AtomSkeletonId::from(anchor_id);

        // 2. On récupère les termes (variables) de l'atome de tête.
        // On les clone pour que l'ancre porte EXACTEMENT les mêmes variables.
        let terms = action_head.terms().to_vec();

        // 3. On crée l'atome d'ancre
        Atom::new(sk_id, terms)
    }

    /// Encodes a sub-expression starting from a specific node into Datalog atoms and rules.
    ///
    /// This is the internal engine used by both `encode_preconditions` and `encode_effects`.
    /// It performs an iterative post-order traversal starting at `node_id` to flatten
    /// complex logical structures (AND/OR) into auxiliary predicates.
    ///
    /// # Arguments
    ///
    /// * `logic` - The global expression tree containing the node.
    /// * `node_id` - The starting point for the encoding (root of the sub-tree).
    /// * `rules_sink` - A vector where newly generated Datalog rules (auxiliary definitions) are stored.
    /// * `parameters` - The typed parameters available in the current context (e.g., action parameters).
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Atom))` - The head atom representing the encoded sub-expression.
    /// * `Ok(None)` - If the branch contains no logical content (e.g., empty AND, ignored nodes).
    /// * `Err(DatalogError)` - If the stack is inconsistent or an unsupported node is encountered.
    pub fn encode_expr(
        &mut self,
        expr: &Expr,
        node_id: NodeId,
        rules_sink: &mut Vec<Rule>,
        parameters: &TypedList<VariableId, TypeId>,
    ) -> Result<Option<Atom>, DatalogError> {
        // ÉTAPE 1 : On nettoie et on collecte les alias pour cet arbre précis
        self.current_aliases = self.extract_variable_aliases(expr)?;

        let mut work_stack = vec![(node_id, false)];
        let mut results_stack: Vec<Option<Atom>> = Vec::with_capacity(32);

        while let Some((node_id, visited)) = work_stack.pop() {
            let node = expr.try_node(node_id)?;
            let kind = node.kind();

            if !visited {
                match kind {
                    // FILTRAGE EN DESCENTE
                    ExprKind::Not => {
                        let children = node.children();

                        // 1. Vérification de l'arité (1 seul enfant)
                        if children.len() != 1 {
                            return Err(ExprError::invalid_expr_node(node_id, kind.clone()).into());
                        }

                        let child_id = children[0];
                        let child_node = expr.try_node(child_id)?;
                        let child_kind = child_node.kind();

                        // 2. Vérification du typing (Comparison) et de l'opérateur (Equal uniquement)
                        let is_valid_comparison = if child_kind == ExprKind::Comparison {
                            // On vérifie si l'opérateur est bien "Equal"
                            child_node.content().try_compare_op()? == CompareOp::Equal
                        } else {
                            false
                        };

                        if !is_valid_comparison {
                            let feature_desc = format!("Negation of {:?}", child_kind);
                            return Err(DatalogError::feature_not_supported(
                                feature_desc,
                                child_id,
                            ));
                        }

                        // Si c'est bon, on continue la visite
                        work_stack.push((node_id, true));
                        work_stack.push((child_id, false));
                    }

                    ExprKind::And
                    | ExprKind::Or
                    | ExprKind::AtStart
                    | ExprKind::AtEnd
                    | ExprKind::Overall => {
                        work_stack.push((node_id, true));
                        for &child_id in node.children().iter().rev() {
                            work_stack.push((child_id, false));
                        }
                    }

                    ExprKind::AtomicFormula | ExprKind::Comparison => {
                        work_stack.push((node_id, true));
                    }

                    ExprKind::Arithmetic => {
                        results_stack.push(None);
                    }

                    _ => return Err(DatalogError::incompatible_node(kind.clone(), node_id)),
                }
            } else {
                // --- PHASE 2 : Synthèse ---
                let num_children = node.children().len();
                let result = match kind {
                    ExprKind::AtomicFormula => {
                        let mut atom = self.extract_atom(expr, node)?;
                        for term in atom.terms_mut() {
                            if let Term::Variable(v) = *term {
                                *term = self.resolve_var(v); // Utilise la version récursive !
                            }
                        }
                        // 2. GESTION DE LA NÉGATION
                        // Si le nœud PDDL est marqué comme négatif (via NNF ou ton flag)
                        // On doit activer le bit MSB ici pour que le AND/OR parent le sache.
                        if node.try_atom_skeleton()?.is_negated() {
                            // Ou la condition que tu utilises pour détecter 'not'
                            let mut sk = atom.skeleton_id();
                            sk.set_negated(true);
                            atom.set_skeleton_id(sk);
                        }
                        Some(atom)
                    }

                    ExprKind::Not => {
                        // On sait que c'est une égalité positive grâce à la Phase 1
                        match results_stack.pop().flatten() {
                            Some(mut atom) => {
                                let terms = atom.terms();
                                if terms[0] == terms[1] {
                                    // NOT(c1 = c1) -> False
                                    None
                                } else {
                                    // (= c1 c2) -> (!= c1 c2)
                                    // On utilise set_negated(true) car on SAIT qu'on veut une inégalité
                                    atom.set_negated(true);
                                    Some(atom)
                                }
                            }
                            None => {
                                // NOT(False) -> True (Tautologie témoin)
                                let v = VariableId::from(0);
                                Some(Atom::equality(Term::Variable(v), Term::Variable(v)))
                            }
                        }
                    }

                    ExprKind::And => {
                        let start_idx = results_stack.len() - num_children;
                        let child_results: Vec<Option<Atom>> =
                            results_stack.drain(start_idx..).collect();

                        // 1. Si un enfant est None (branche impossible), le AND est None
                        if child_results.iter().any(|r| r.is_none()) {
                            None
                        } else {
                            let mut atoms: Vec<Atom> = Vec::new();
                            for a in child_results.into_iter().flatten() {
                                if a.is_equality() {
                                    let terms = a.terms();
                                    if a.is_negated() {
                                        if terms[0] == terms[1] {
                                            return Ok(None);
                                        } // Contradiction: ?x != ?x
                                        atoms.push(a);
                                    } else {
                                        if terms[0] != terms[1] {
                                            atoms.push(a);
                                        } // On garde ?x = constante
                                    }
                                } else {
                                    atoms.push(a);
                                }
                            }

                            // 2. Synthèse
                            if atoms.is_empty() {
                                // Tautologie (1=1), on renvoie un témoin
                                Some(Atom::equality(
                                    Term::Variable(VariableId::from(0)),
                                    Term::Variable(VariableId::from(0)),
                                ))
                            } else if atoms.len() == 1 {
                                Some(atoms[0].clone())
                            } else {
                                // 3. Cache & Auxiliaires
                                atoms.sort_by_key(|a| a.skeleton_id());
                                if let Some(existing_head) = self.cache.get(&atoms) {
                                    Some(existing_head.clone())
                                } else {
                                    // On récupère la tête ET le corps sécurisé (avec les Type Guards)
                                    let (head, secured_body) =
                                        self.encode_new_aux_predicate(&atoms, parameters)?;

                                    // CRUCIAL : On pousse la version sécurisée dans le moteur Datalog
                                    rules_sink.push(Rule::new(head.clone(), secured_body));

                                    // On garde les 'atoms' originaux comme clé de cache
                                    self.cache.insert(atoms, head.clone());
                                    Some(head)
                                }
                            }
                        }
                    }

                    ExprKind::Or => {
                        let start_idx = results_stack.len() - num_children;
                        let mut atoms: Vec<Atom> =
                            results_stack.drain(start_idx..).flatten().collect();

                        if atoms.is_empty() {
                            None
                        } else if atoms.len() == 1 {
                            Some(atoms[0].clone())
                        } else {
                            atoms.sort_by_key(|a| a.skeleton_id());
                            atoms.dedup();

                            if let Some(existing_head) = self.cache.get(&atoms) {
                                Some(existing_head.clone())
                            } else {
                                // 1. On récupère la tête (on ignore le secured_body global car le OR
                                // nécessite une sécurisation par branche).
                                let (head, _) =
                                    self.encode_new_aux_predicate(&atoms, parameters)?;

                                for atom in &atoms {
                                    /*println!(
                                        "DEBUG OR BRANCH: Head {:?} <- Atom {:?} (negated: {})",
                                        head.skeleton_id(),
                                        atom.skeleton_id(),
                                        atom.is_negated()
                                    );*/
                                    // Chaque règle du OR est : Aux_Or(?x) :- Branche_N(?x)
                                    let mut branch_body = vec![atom.clone()];

                                    // --- SÉCURISATION DE LA BRANCHE ---
                                    // On vérifie quelles variables de la tête sont couvertes par CETTE branche précise
                                    let mut covered_vars = std::collections::HashSet::new();
                                    if !atom.is_negated() {
                                        for term in atom.terms() {
                                            if let Term::Variable(v) = term {
                                                covered_vars.insert(*v);
                                            }
                                        }
                                    }

                                    // Pour chaque variable de la tête, si elle est "orpheline" dans cette branche,
                                    // on injecte son Type Guard.
                                    for term in head.terms() {
                                        if let Term::Variable(v) = term {
                                            if !covered_vars.contains(v) {
                                                let type_id =
                                                    parameters[v.as_usize()].ty().members()[0]
                                                        .as_usize();
                                                let type_sk = self.type_to_skeleton[type_id];

                                                // Ajout du Type Guard spécifique à la branche
                                                branch_body.push(Atom::new(
                                                    type_sk,
                                                    vec![Term::Variable(*v)],
                                                ));
                                                covered_vars.insert(*v);
                                            }
                                        }
                                    }

                                    // On enregistre la règle de la branche sécurisée
                                    rules_sink.push(Rule::new(head.clone(), branch_body));
                                }

                                self.cache.insert(atoms, head.clone());
                                Some(head)
                            }
                        }
                    }

                    ExprKind::Comparison => {
                        if node.content().try_compare_op()? == CompareOp::Equal {
                            // Utilise ta fonction centrale !
                            let mut atom = self.extract_atom(expr, node)?;

                            // Résolution des termes (important !)
                            for term in atom.terms_mut() {
                                if let Term::Variable(v) = *term {
                                    *term = self.resolve_var(v);
                                }
                            }

                            let terms = atom.terms();
                            if terms[0] == terms[1] {
                                Some(atom) // Tautologie
                            } else if let (Term::Constant(c1), Term::Constant(c2)) =
                                (&terms[0], &terms[1])
                            {
                                None // Conflit de constantes
                            } else {
                                Some(atom)
                            }
                        } else {
                            None
                        }
                    }

                    ExprKind::AtStart | ExprKind::AtEnd | ExprKind::Overall => {
                        if num_children > 0 {
                            results_stack.pop().flatten()
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                results_stack.push(result);
            }
        }
        Ok(results_stack.pop().flatten())
    }

    /// Extracts a logical [`Atom`] from a specific expression node.
    ///
    /// This function serves as the bridge between the high-level expression tree ([`Expr`][`ExprNode`][`Atom`]).
    /// It resolves the predicate identity and maps each child argument to its concrete Datalog representation.
    ///
    /// # Arguments
    ///
    /// * `logic` - The global expression tree used to resolve the nature of child nodes.
    /// * `node` - A reference to the current [`ExprNode`], which must represent an `AtomicFormula` or a `Comparison`.
    ///
    /// # Returns
    ///
    /// A [`Result`] containing the grounded or lifted [`Atom`], or a [`DatalogError`] if resolution fails.
    ///
    /// # Process
    ///
    /// 1. **ID & Offset Resolution**: Determines if the node is a `Comparison` (using a fixed `EQUALITY_ID` with no offset)
    ///    or an `AtomicFormula` (fetching the `AtomSkeletonId` from content and skipping the predicate name at index 0).
    /// 2. **Term Mapping**: Iterates over child nodes to categorize arguments:
    ///    - **Variables**: Parameters (e.g., `?v0`) are converted to [`Term::Variable`].
    ///    - **Constants**: Fixed objects (e.g., `room_a`) are converted to [`Term::Constant`].
    /// 3. **Validation**: Ensures that all arguments are valid terminals for a Datalog relation.
    ///
    /// # Performance
    ///
    /// * **Complexity**: $O(N)$ where $N$ is the number of arguments (arity of the predicate).
    /// * **Efficiency**:
    ///    - Uses a branchless-friendly tuple assignment for the ID and skip offset.
    ///    - Allocates the `Vec<Term>` with exact capacity using `saturating_sub` to avoid reallocations.
    ///    - Skips the predicate name efficiently using the `children.iter().skip(n)` iterator.
    ///
    /// # Errors
    ///
    /// Returns a [`DatalogError`] if:
    /// * The node content cannot be converted to an atom skeleton.
    /// * An argument node is neither a `Variable` nor a `Constant` (e.g., a nested expression).
    /// * A node reference within the `logic` tree is invalid.
    fn extract_atom(&self, expr: &Expr, node: &ExprNode) -> Result<Atom, DatalogError> {
        let kind = node.kind();
        let children = node.children();

        // 1. Fast determination of the Skeleton ID and the child skip offset.
        // Comparisons (equality) use a fixed ID and start terms at index 0.
        // Atomic formulas fetch their ID from content and skip the predicate name at index 0.
        let (mut skeleton_id, skip_count) = if kind == ExprKind::Comparison {
            (AtomSkeletonId::from(Atom::EQUALITY_ID), 0)
        } else {
            (node.content().try_atom_skeleton()?, 1)
        };

        // 2. Exact allocation to prevent vector resizing during the loop.
        let capacity = children.len().saturating_sub(skip_count);
        let mut terms = Vec::with_capacity(capacity);

        // 3. Optimized term collection.
        for &arg_id in children.iter().skip(skip_count) {
            let arg_node = expr.try_node(arg_id)?;

            // Match on Enum is highly optimized by the Rust compiler.
            let term = match arg_node.kind() {
                ExprKind::Variable => Term::Variable(arg_node.content().try_variable()?),
                ExprKind::Object => Term::Constant(arg_node.content().try_object()?),
                _ => return Err(DatalogError::invalid_atom_argument(arg_id)),
            };
            terms.push(term);
        }

        Ok(Atom::new(skeleton_id, terms))
    }

    /// Encodes a new auxiliary predicate based on a collection of atoms.
    ///
    /// This is a helper function used during the transformation of complex formulas
    /// (like AND/OR) into Horn clauses. It performs two main steps:
    /// 1. It collects all unique variables from the provided `atoms` to determine
    ///    the signature (arity and types) of the new predicate.
    /// 2. It allocates a new unique predicate ID and returns the corresponding [`Atom`].
    ///
    /// # Arguments
    /// * `atoms` - The list of atoms that will form the body (for AND) or the options (for OR)
    ///             of the rules associated with this auxiliary predicate.
    /// * `parameters` - The typed list of parameters from the current scope (e.g., action parameters).
    ///
    /// # Errors
    /// Returns a [`DatalogError`] if variable collection fails or if there is an
    /// inconsistency in the typed list.
    ///
    fn encode_new_aux_predicate(
        &mut self,
        atoms: &[Atom],
        parameters: &TypedList<VariableId, TypeId>,
    ) -> Result<(Atom, Vec<Atom>), DatalogError> {
        // <--- Retourne le tuple (Tête, Corps Sécurisé)
        // 1. Collecte et résolution des variables
        let used_vars = self.collect_variables(atoms)?;
        let mut resolved_terms: Vec<Term> = used_vars
            .into_iter()
            .map(|v_id| self.resolve_var(v_id))
            .collect();

        resolved_terms.sort();
        resolved_terms.dedup();

        let final_vars: Vec<VariableId> = resolved_terms
            .iter()
            .filter_map(|t| {
                if let Term::Variable(v) = t {
                    Some(*v)
                } else {
                    None
                }
            })
            .collect();

        // --- 2. LA RÉPARATION (Type Guard Injection) ---
        let mut covered_vars = std::collections::HashSet::new();
        for atom in atoms {
            if !atom.is_negated() {
                for term in atom.terms() {
                    if let Term::Variable(v) = term {
                        covered_vars.insert(*v);
                    }
                }
            }
        }

        let mut secured_body = atoms.to_vec();

        for v_id in &final_vars {
            if !covered_vars.contains(v_id) {
                // Récupération sécurisée du type
                let type_id = parameters[v_id.as_usize()].ty().members()[0].as_usize();
                let type_sk = self.type_to_skeleton[type_id];

                // On injecte l'atome de type pour lier la variable (v#1)
                secured_body.push(Atom::new(type_sk, vec![Term::Variable(*v_id)]));
                covered_vars.insert(*v_id);
            }
        }

        // 3. Création de l'atome de tête
        // Note : on passe final_vars et resolved_terms qui sont déjà minimalistes
        let head = self.create_aux_atom(final_vars, resolved_terms, parameters);

        Ok((head, secured_body)) // <--- On renvoie les deux !
    }
    /*fn encode_new_aux_predicate(
        &mut self,
        atoms: &[Atom],
        parameters: &TypedList<VariableId, TypeId>,
    ) -> Result<Atom, DatalogError> {
        // 1. On collecte les variables REELLEMENT présentes dans les atomes enfants
        // Cela garantit que l'arité est minimale (fixe les erreurs d'arité 3 vs 1)
        let mut used_vars = self.collect_variables(atoms)?;

        // 2. On transforme les variables selon les alias et on déduplique
        // C'est l'étape CRUCIALE pour le test d'aliasing (?v0 = ?v1 => ?v0)
        let mut resolved_terms: Vec<Term> = used_vars
            .into_iter()
            .map(|v_id| self.resolve_var(v_id))
            .collect();

        // On trie et déduplique les Termes (pas les VariableId)
        resolved_terms.sort();
        resolved_terms.dedup();

        // 3. On extrait les VariableId restants après déduplication pour créer le squelette
        let final_vars: Vec<VariableId> = resolved_terms
            .iter()
            .filter_map(|t| {
                if let Term::Variable(v) = t {
                    Some(*v)
                } else {
                    None
                }
            })
            .collect();

        // 4. On crée l'atome avec cette signature minimale
        Ok(self.create_aux_atom(final_vars, resolved_terms, parameters))
    }*/

    /// Creates a new auxiliary atom and registers its skeleton locally.
    ///
    /// This method is a core part of the **Skolemization** process during flattening.
    /// It generates a unique predicate ID for a sub-formula and maps the provided
    /// variables to their respective types based on the action's parameter list.
    ///
    /// # Arguments
    /// * `vars` - The subset of variables that will become the terms of this auxiliary atom.
    /// * `parameters` - The master list of typed variables from the current action/context
    ///   used to resolve the types of `vars`.
    ///
    /// # Returns
    /// A new [`Atom`] configured with an auxiliary [`AtomSkeletonId`] and variable terms.
    ///
    /// # Performance
    /// - **ID Management**: Increments an internal counter in $O(1)$.
    /// - **Type Resolution**: Direct $O(1)$ lookup per variable using the `parameters` list.
    /// - **Allocation**: Performs one allocation for the `aux_defs` storage and one for the `Atom` terms.
    fn create_aux_atom(
        &mut self,
        skeleton_vars: Vec<VariableId>,
        resolved_terms: Vec<Term>,
        parameters: &TypedList<VariableId, TypeId>,
    ) -> Atom {
        let id = self.next_aux_id;
        self.next_aux_id += 1;

        let mut aux_params = TypedList::new();
        for &v_id in &skeleton_vars {
            let ty = parameters[v_id.as_usize()].ty();
            aux_params.push(TypedSymbol::new(v_id, ty.clone()));
        }

        self.aux_defs.push(AtomicFormulaSkeleton::new(
            PredicateSymbolId::from(id),
            aux_params,
        ));
        Atom::new(AtomSkeletonId::from(id), resolved_terms)
    }

    fn secure_aux_rule(
        &self,
        body: &mut Vec<Atom>,
        head: &Atom,
        parameters: &TypedList<VariableId, TypeId>,
    ) {
        let mut covered_vars = std::collections::HashSet::new();

        // 1. On regarde quelles variables sont déjà liées par des atomes POSITIFS
        for atom in body.iter() {
            if !atom.is_negated() {
                for term in atom.terms() {
                    if let Term::Variable(v) = term {
                        covered_vars.insert(*v);
                    }
                }
            }
        }

        // 2. Pour chaque variable de la tête, si elle n'est pas couverte, on injecte le Type Guard
        for term in head.terms() {
            if let Term::Variable(v_id) = term {
                if !covered_vars.contains(v_id) {
                    // On récupère le skeleton du type (aplatit)
                    let type_id = parameters[v_id.as_usize()].ty().members()[0].as_usize();
                    let type_sk = self.type_to_skeleton[type_id];

                    // On ajoute l'atome de type au corps de la règle
                    body.push(Atom::new(type_sk, vec![Term::Variable(*v_id)]));
                    covered_vars.insert(*v_id);
                }
            }
        }
    }

    /// Collects all unique variables from a slice of atoms and returns them as a sorted vector.
    ///
    /// This function identifies every `VariableId` present in the terms of the provided atoms
    /// and produces a compact, deduplicated list.
    ///
    /// # Logic and Implementation
    /// This function uses a **Bitset** (a `u64` mask) to perform a "Union" operation of all
    /// variables in a single pass.
    /// 1. It iterates through all terms of all atoms.
    /// 2. For each variable, it sets the corresponding bit in a 64-bit integer.
    /// 3. It then extracts the set bits to reconstruct the `VariableId` list.
    ///
    /// # Performance
    /// - **Time Complexity**: $O(T + V)$, where $T$ is the total number of terms across all atoms
    ///   and $V$ is the number of unique variables. The bitwise extraction is extremely fast
    ///   thanks to CPU-level instructions.
    /// - **Space Complexity**: $O(V)$ for the resulting `Vec`. The internal bitset resides
    ///   entirely on the stack (`u64`).
    /// - **Instruction Level Optimization**:
    ///     - Uses `count_ones()` (POP_CNT) to pre-allocate the exact capacity of the `Vec`,
    ///       preventing reallocations.
    ///     - Uses `trailing_zeros()` (TZ_CNT) to jump directly to the next set bit, avoiding
    ///       a full 64-iteration loop.
    ///
    /// # Constraints & Safety
    /// - **Variable Limit**: This implementation is strictly limited to **64 variables**
    ///   (indexed 0 to 63). This is a design trade-off to ensure the bitset fits into
    ///   a single CPU register.
    /// - **Debug Assertions**: In debug builds, the function will panic if a `VariableId`
    ///   exceeds `MAX_VARS`. In release builds, it uses a modulo wrap-around to prevent
    ///   undefined behavior, though this would indicate a logic error in the caller.
    ///
    /// # Returns
    /// A `Vec<VariableId>` sorted by ID in ascending order (due to the nature of bit-scanning).
    pub fn collect_variables(&self, atoms: &[Atom]) -> Result<Vec<VariableId>, DatalogError> {
        let mask = self.collect_mask(atoms);
        Ok(self.mask_to_vars(mask))
    }

    fn mask_to_vars(&self, mut mask: u64) -> Vec<VariableId> {
        let mut vars = Vec::with_capacity(mask.count_ones() as usize);
        while mask != 0 {
            let bit = mask.trailing_zeros();
            vars.push(VariableId::from(bit as usize));
            mask &= mask - 1;
        }
        vars
    }

    fn scan_required_terms_mask(&self, expr: &Expr, start_node_id: NodeId) -> u64 {
        let mut mask: u64 = 0;
        let mut stack = vec![start_node_id];

        while let Some(node_id) = stack.pop() {
            if let Ok(node) = expr.try_node(node_id) {
                match node.kind() {
                    // On scanne directement les enfants pour trouver les variables
                    ExprKind::AtomicFormula | ExprKind::Comparison => {
                        for &child_id in node.children().iter().skip(1) {
                            if let Ok(child_node) = expr.try_node(child_id) {
                                if child_node.kind() == ExprKind::Variable {
                                    if let Ok(v) = child_node.content().try_variable() {
                                        // Résolution ici aussi
                                        if let Term::Variable(rv) = self.resolve_var(v) {
                                            mask |= 1 << rv.as_usize();
                                        }
                                    }
                                }
                            }
                        }
                    }
                    ExprKind::And
                    | ExprKind::When
                    | ExprKind::AtStart
                    | ExprKind::AtEnd
                    | ExprKind::Overall
                    | ExprKind::Not => {
                        for &child_id in node.children() {
                            stack.push(child_id);
                        }
                    }
                    _ => {}
                }
            }
        }
        mask
    }

    fn collect_mask(&self, atoms: &[Atom]) -> u64 {
        let mut mask: u64 = 0;
        for atom in atoms {
            for term in atom.terms() {
                // AJOUT : Résolution systématique
                let resolved_term = match term {
                    Term::Variable(v) => self.resolve_var(*v),
                    Term::Constant(_) => term.clone(),
                };

                if let Term::Variable(v) = resolved_term {
                    mask |= 1 << v.as_usize();
                }
            }
        }
        mask
    }

    /// Résout une variable vers son représentant canonique (le plus petit ID du groupe d'égalité)
    #[inline(always)]
    fn resolve_var(&self, v: VariableId) -> Term {
        // Si la fermeture a bien aplati la map, un seul get suffit.
        // C'est beaucoup plus rapide que de boucler à chaque fois.
        self.current_aliases
            .get(&v)
            .cloned()
            .unwrap_or(Term::Variable(v))
    }

    pub fn extract_variable_aliases(
        &self,
        expr: &Expr,
    ) -> Result<HashMap<VariableId, Term>, DatalogError> {
        let mut aliases = HashMap::new();

        // On récupère l'ID racine. Si l'expression est vide, on sort.
        let root_id = match expr.root_id() {
            Some(id) => id,
            None => return Ok(aliases),
        };

        let mut stack = vec![root_id];

        while let Some(node_id) = stack.pop() {
            let node = expr.try_node(node_id)?;
            let kind = node.kind();

            match kind {
                // SI C'EST UN NOT : On ne descend pas dedans !
                // Les égalités à l'intérieur d'un NOT sont des inégalités.
                ExprKind::Not => {
                    continue;
                }

                // SI C'EST UNE ÉGALITÉ : On extrait l'alias
                ExprKind::Comparison => {
                    if node.content().try_compare_op()? == CompareOp::Equal {
                        let children = node.children();
                        if children.len() == 2 {
                            let t1 = self.node_to_term(expr, children[0])?;
                            let t2 = self.node_to_term(expr, children[1])?;

                            match (t1, t2) {
                                (Some(Term::Variable(v1)), Some(Term::Variable(v2)))
                                    if v1 != v2 =>
                                {
                                    aliases.insert(v1.max(v2), Term::Variable(v1.min(v2)));
                                }
                                (Some(Term::Variable(v)), Some(Term::Constant(c)))
                                | (Some(Term::Constant(c)), Some(Term::Variable(v))) => {
                                    aliases.insert(v, Term::Constant(c));
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // POUR LES AUTRES : On continue de descendre (And, Or, When, etc.)
                _ => {
                    for &child_id in node.children().iter().rev() {
                        stack.push(child_id);
                    }
                }
            }
        }

        // Une fois la collecte finie, on applique la fermeture
        self.compute_transitive_closure(&mut aliases);
        Ok(aliases)
    }

    /// Helper pour transformer un Node en Term atomique
    fn node_to_term(&self, expr: &Expr, node_id: NodeId) -> Result<Option<Term>, DatalogError> {
        let n = expr.try_node(node_id)?;
        Ok(match n.kind() {
            ExprKind::Variable => Some(Term::Variable(n.content().try_variable()?)),
            ExprKind::Object => Some(Term::Constant(n.content().try_object()?)),
            _ => None,
        })
    }

    fn compute_transitive_closure(&self, aliases: &mut HashMap<VariableId, Term>) {
        let keys: Vec<VariableId> = aliases.keys().cloned().collect();

        for start_var in keys {
            // On récupère le terme cible initial
            let mut current_term = aliases.get(&start_var).unwrap().clone();
            let mut visited = std::collections::HashSet::new();
            visited.insert(start_var);

            // On suit la chaîne des variables aliasées
            while let Term::Variable(v) = current_term {
                if let Some(next_term) = aliases.get(&v) {
                    // Sécurité anti-cycle (ex: v1 = v2 et v2 = v1)
                    if !visited.insert(v) {
                        break;
                    }
                    current_term = next_term.clone();
                } else {
                    break;
                }
            }

            // On "aplatit" la structure (Path Compression)
            if let Some(alias) = aliases.get_mut(&start_var) {
                *alias = current_term;
            }
        }
    }

    //---------------------------- API for tests only  ----------------------------//
    #[cfg(test)]
    pub fn set_action_anchor(&mut self, anchor: Atom) {
        self.action_anchor = Some(anchor);
    }

    pub fn action_anchor(&self) -> Option<&Atom> {
        self.action_anchor.as_ref()
    }
}

#[cfg(test)]
#[path = "tests/encoder_tests.rs"]
mod encoder_tests;
