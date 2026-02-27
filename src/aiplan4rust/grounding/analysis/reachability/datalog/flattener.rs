use std::collections::HashMap;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::atom::Atom;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::error::DatalogError;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::rule::Rule;
use crate::aiplan4rust::grounding::analysis::reachability::datalog::term::Term;
use crate::aiplan4rust::lang::{AtomSkeletonId,  PredicateSymbolId, Type, TypeId, TypedList, TypedSymbol, VariableId};
use crate::aiplan4rust::lir::expr::{Expr, ExprKind, ExprNode};
use crate::aiplan4rust::lir::LiftedAction;
use crate::aiplan4rust::lir::problem::atomic_skeleton::AtomicFormulaSkeleton;

pub struct Flattener {
    /// L'ID de départ pour les auxiliaires (généralement problem.predicates.len())
    base_aux_id: usize,
    /// Compteur interne pour le prochain ID disponible
    next_aux_id: usize,
    /// Liste locale des définitions de squelettes auxiliaires
    pub aux_defs: Vec<AtomicFormulaSkeleton>,
    /// Cache pour la réutilisation des prédicats auxiliaires
    cache: HashMap<Vec<Atom>, Atom>,
}
impl Flattener {

    /// Nombre maximum de variables (paramètres) par action.
    /// Limité à 64 pour tenir dans un registre CPU (u64).
    ///
    /// if id < base_count {
    ///     print!("{}", interner.resolve(registry.get(id)));
    /// } else {
    ///     print!("aux_{}", id); // Nom généré à la volée uniquement pour l'affichage
    /// }
    const MAX_VARS: usize = 64;

    pub fn new(base_id: usize) -> Self {
        Self {
            base_aux_id: base_id,
            next_aux_id: base_id,
            aux_defs: Vec::with_capacity(256),
            cache: HashMap::with_capacity(256),
        }
    }


    /// Génère un nouvel ID de squelette pour représenter un Type PDDL dans le Datalog.
    pub fn register_type_skeleton(&mut self) -> AtomSkeletonId {
        let id = self.next_aux_id;
        self.next_aux_id += 1;

        // Un prédicat de type (ex: Robot(?x)) n'a qu'un paramètre.
        // On crée une signature simple : [Type_de_l_objet]
        // Note : Si tu ne veux pas t'embêter avec les types des arguments
        // du squelette de type, tu peux laisser une liste vide ou un type générique.

        let predicate_id = PredicateSymbolId::from(id);
        // On enregistre la définition (utile pour le debug)
        // L'arité est de 1 (un seul objet est testé)
        let arg = TypedSymbol::new(VariableId::from(0), Type::root());
        let mut arguments = TypedList::new();
        arguments.push(arg);
        self.aux_defs.push(AtomicFormulaSkeleton::new(predicate_id, arguments));

        AtomSkeletonId::from(id)
    }

    /// Crée un squelette (signature) pour une action et l'enregistre.
    /// Cela permet de traiter l'action elle-même comme un prédicat dans la DB.
    pub fn register_action_skeleton(&mut self, action: &LiftedAction) -> AtomSkeletonId {
        let id = self.next_aux_id;
        self.next_aux_id += 1;

        // 1. On récupère les paramètres de l'action pour définir la signature
        let parameters = action.parameters().clone();

        // 2. On crée l'ID de prédicat (virtuel)
        let predicate_id = PredicateSymbolId::from(id);

        // 3. On enregistre la définition du squelette
        // (Utile si tu dois exporter ou débugger les noms d'actions plus tard)
        self.aux_defs.push(AtomicFormulaSkeleton::new(predicate_id, parameters));

        // 4. On retourne l'AtomSkeletonId (qui est l'équivalent numérique de l'ID de prédicat)
        AtomSkeletonId::from(id)
    }

    pub fn flatten(
        &mut self,
        expr: &Expr,
        rules_sink: &mut Vec<Rule>,
        parameters: &TypedList<VariableId, TypeId>,
    ) -> Result<Option<Atom>, DatalogError> {
        // Pile de travail : (ID du nœud, est_visité)
        // On commence par la racine, non visitée.
        let mut work_stack = vec![(expr.try_root_id()?, false)];
        let mut results_stack: Vec<Option<Atom>> = Vec::with_capacity(32);

        while let Some((node_id, visited)) = work_stack.pop() {
            let node = expr.try_node(node_id)?;
            let kind = node.kind();

            if !visited {
                // --- PHASE D'ENTRÉE (Descente) ---

                match kind {
                    // --- 1. LE CŒUR : On explore ces branches ---
                    ExprKind::AtomicFormula | ExprKind::And | ExprKind::Or | ExprKind::AtStart
                        | ExprKind::AtEnd | ExprKind::Overall=> {
                        work_stack.push((node_id, true));
                        for &child_id in node.children().iter().rev() {
                            work_stack.push((child_id, false));
                        }
                    }

                    // --- 2. LES IGNORÉS : On coupe la branche ici (Performance) ---
                    // On ignore la négation, les effets (Assign), les calculs (Operation, Metric)
                    // et tout ce qui est temporel/HDN (Task, AtStart, etc.)
                    ExprKind::Not | ExprKind::Assign | ExprKind::Operation | ExprKind::Metric
                        | ExprKind::Task | ExprKind::FunctionTerm | ExprKind::FComp  => {
                        results_stack.push(None);
                        continue;
                    }

                    // --- 3. LES ERREURS : Ne devraient pas être rencontrés ici ---
                    // - Constant/Variable/Number : traités par extract_atom, donc ne doivent pas être parents
                    // - Forall/Exists : doivent être supprimés par ton expand()
                    _ => {
                        return Err(DatalogError::UnsupportedNode { kind, node_id });
                    }
                }
            } else {
                // --- PHASE DE SORTIE (Remontée / Synthèse) ---



                let num_children = node.children().len();
                // On vérifie si la pile contient assez d'éléments avant de calculer l'index
                if results_stack.len() < num_children {
                    return Err(DatalogError::InconsistentStack(kind.clone()));
                }
                let start_idx = results_stack.len() - num_children;

                let result = match kind {
                    ExprKind::AtomicFormula => Some(self.extract_atom(expr, node)?),

                    ExprKind::And => {
                        let mut body: Vec<Atom> = results_stack.drain(start_idx..).flatten().collect();

                        if body.is_empty() { None }
                        else if body.len() == 1 { Some(body[0].clone()) }
                        else {
                            // Optionnel mais recommandé : trier pour que Or(A, B) == Or(B, A)
                            body.sort_by_key(|a| a.skeleton_id());

                            // 1. Vérifier si ce corps existe déjà dans le cache
                            if let Some(existing_head) = self.cache.get(&body) {
                                Some(existing_head.clone())
                            } else {
                                // 2. Sinon, créer l'auxiliaire comme avant
                                let vars = self.collect_variables(&body);
                                let head = self.create_aux_atom(vars, parameters);
                                rules_sink.push(Rule::new(head.clone(), body.clone()));

                                // 3. Mémoriser pour la prochaine fois
                                self.cache.insert(body, head.clone());
                                Some(head)
                            }
                        }
                    }

                    ExprKind::Or => {

                        let mut atoms: Vec<Atom> = results_stack.drain(start_idx..).flatten().collect();

                        if atoms.is_empty() { None }
                        else if atoms.len() == 1 { Some(atoms[0].clone()) }
                        else {
                            // Optionnel mais recommandé : trier pour que Or(A, B) == Or(B, A)
                            atoms.sort_by_key(|a| a.skeleton_id());

                            // Vérification du cache
                            if let Some(existing_head) = self.cache.get(&atoms) {
                                Some(existing_head.clone())
                            } else {
                                // Création d'une SEULE tête pour toutes les branches du OR
                                let vars = self.collect_variables(&atoms);
                                let head = self.create_aux_atom(vars, parameters);

                                // Génération d'une règle Datalog pour chaque option du OR
                                for atom in &atoms {
                                    rules_sink.push(Rule::new(head.clone(), vec![atom.clone()]));
                                }

                                // On stocke la liste des atomes comme clé pointant vers cette tête
                                self.cache.insert(atoms, head.clone());
                                Some(head)
                            }
                        }
                    }

                    // AJOUT ICI : Propager le contenu des noeuds temporels
                    ExprKind::AtStart | ExprKind::AtEnd | ExprKind::Overall => {
                        // On récupère l'unique enfant (Option<Atom>) et on le remonte
                        results_stack.drain(start_idx..).next().flatten()
                    }

                    _ => {
                        let start_idx = results_stack.len() - num_children;
                        results_stack.drain(start_idx..);
                        None
                    }
                };
                results_stack.push(result);
            }
        }

        Ok(results_stack.pop().flatten())
    }


    /// Extracts a logical [`Atom`] from a specific expression node.
    ///
    /// This function acts as the bridge between the high-level expression tree ([`Expr`][`ExprNode`][`Atom`]).
    /// It identifies the predicate and maps each child argument to its respective type.
    ///
    /// # Arguments
    ///
    /// * `expr` - The global expression tree used to resolve the nature of child nodes (variables vs. constants).
    /// * `node` - A reference to the current [`ExprNode`] which must be of kind `AtomicFormula`.
    ///
    /// # Process
    ///
    /// 1. **Skeleton Extraction**: Retrieves the `AtomSkeletonId` from the node, which identifies the predicate symbol.
    /// 2. **Term Mapping**: Iterates over the node's arguments using a fallible mapping:
    ///    - If the argument node is a variable (e.g., a parameter like `?robot`), it is wrapped as [`Term::Variable`].
    ///    - If the argument node is a constant (e.g., a specific object like `kitchen`), it is wrapped as [`Term::Constant`].
    /// 3. **Construction**: Returns a new `Atom` containing the predicate ID and the collected terms.
    ///
    /// # Performance
    ///
    /// Passing the `ExprNode` by reference avoids a costly lookup in the expression tree's internal storage.
    /// The function uses `.collect::<Result<Vec<_>, _>>()` to efficiently build the term list in a single
    /// pass, maintaining $O(N)$ complexity where $N$ is the number of arguments.
    ///
    /// # Errors
    ///
    /// Returns a [`DatalogError`] if:
    /// * The node content is not a valid atom skeleton.
    /// * An argument node is neither a variable nor a constant (returns `InvalidAtomArgument`).
    /// * Any internal `try_` conversion fails.
    fn extract_atom(&self, expr: &Expr, node: &ExprNode) -> Result<Atom, DatalogError> {
        let skeleton_id = node.content().try_atom_skeleton()?;

        // On transforme les arguments en Terms, en propageant les erreurs éventuelles
        let terms: Vec<Term> = node.children()
            .iter()
            .map(|&arg_id| {
                let arg_node = expr.try_node(arg_id)?;

                match arg_node.kind() {
                    ExprKind::Variable => {
                        let var_id = arg_node.content().try_variable()?;
                        Ok(Term::Variable(var_id))
                    }
                    ExprKind::Constant => {
                        let cons_id = arg_node.content().try_constant()?;
                        Ok(Term::Constant(cons_id))
                    }
                    // Cas de sécurité si un argument n'est ni une variable ni une constante
                    _ => Err(DatalogError::InvalidAtomArgument(arg_id)),
                }
            })
            .collect::<Result<Vec<Term>, DatalogError>>()?;

        Ok(Atom::new(skeleton_id, terms))
    }


    /// Creates a new auxiliary atom with a unique ID and the specified variables.
    ///
    /// This is used during the flattening process to represent sub-formulas
    /// (the "head" of a new Datalog rule).
    ///
    /// # Performance
    /// - **Allocation**: Performs one allocation for the `Vec<Term>`.
    /// - **ID Management**: Increments the internal counter. Ensure `base_id` provided
    ///   at construction starts after the last predicate ID of the original domain.
    /// Crée un atome auxiliaire et enregistre son squelette localement
    fn create_aux_atom(
        &mut self,
        vars: Vec<VariableId>,
        parameters: &TypedList<VariableId, TypeId>,
    ) -> Atom {
        let id = self.next_aux_id;
        self.next_aux_id += 1;

        // 1. On construit la signature (types) à partir des paramètres de l'action
        let mut aux_params = TypedList::new();
        for &v_id in &vars {
            // Accès direct O(1) car les VariableId sont des index
            let ty = parameters[v_id.as_usize()].ty();
            aux_params.push(TypedSymbol::new(v_id, ty.clone()));
        }

        // 2. On crée le PredicateSymbolId (virtuel, pas besoin d'interner)
        let predicate_id = PredicateSymbolId::from(id);

        // 3. On enregistre la définition dans notre liste locale
        self.aux_defs.push(AtomicFormulaSkeleton::new(predicate_id, aux_params));

        // 4. On crée l'Atome pour Datalog
        let skeleton_id = AtomSkeletonId::from(id);
        let terms: Vec<Term> = vars.into_iter().map(Term::Variable).collect();

        Atom::new(skeleton_id, terms)
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
    fn collect_variables(&self, atoms: &[Atom]) -> Vec<VariableId> {
        let mut mask: u64 = 0;

        for atom in atoms {
            for term in atom.terms() {
                if let Term::Variable(v) = term {
                    let id = v.as_usize();

                    // Safety check for development
                    debug_assert!(
                        id < Flattener::MAX_VARS,
                        "VariableId {} exceeds bitset capacity (64)", id
                    );

                    // Map variable ID to bit position
                    mask |= 1 << id;
                }
            }
        }

        // Pre-allocate the exact size needed
        let mut vars = Vec::with_capacity(mask.count_ones() as usize);
        let mut temp_mask = mask;

        // Efficient bit-scan loop
        while temp_mask != 0 {
            let bit = temp_mask.trailing_zeros();
            vars.push(VariableId::from(bit as usize));
            // Clear the lowest set bit
            temp_mask &= temp_mask - 1;
        }

        vars
    }
}
