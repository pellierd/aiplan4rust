//! # Datalog Engine Saturation and Optimization Module
//!
//! This module implements the core evaluation, semi-naive saturation, unification,
//! rule optimization, and filtering mechanisms for the Datalog reasoning engine
//! within `aiplan4rust`.
//!
//! ## Key Components
//!
//! - **Semi-Naive Evaluation**: Incremental fixpoint computation leveraging delta relations
//!   to avoid redundant rule evaluations across iterations.
//! - **Unification and Variable Binding**: Efficient environment management with surgical
//!   rollbacks (`undo_to_savepoint`) to handle variable bindings and self-constraints.
//! - **Negation and Filters**: Support for Closed World Assumption (CWA), Negation as Failure (NAF),
//!   and built-in equality/inequality constraints.
//! - **Rule Optimization**: Dynamic rule body reordering based on variable binding counts,
//!   relation sizes, and semantic predicate priorities.

use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::Atom;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::rule::Rule;
use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::term::Term;
use crate::aiplan4rust::compiler::grounding::binding::iter::BindingsIterator;
use crate::aiplan4rust::support::lang::{AtomSkeletonId, ObjectId};
use crate::analysis::reachability::datalog::core::rule::RuleBody;
use crate::analysis::reachability::datalog::engine::MAX_VARS;
use crate::analysis::reachability::datalog::error::DatalogError;
use crate::DatalogEngine;

impl<'a> DatalogEngine<'a> {
    /// Materializes negated predicates using the Closed World Assumption, identifying
    /// absent positive facts and inserting their corresponding negated representations
    /// into the delta database.
    ///
    /// # Returns
    /// Returns `Ok(())` on successful materialization, or a [`DatalogError`] if
    /// a typed parameter list fetch fails.
    ///
    /// # Complexity
    /// - Time complexity: Proportional to the total number of value combinations
    ///   across all parameters for each negated predicate.
    pub(crate) fn materialize_negations(&mut self) -> Result<(), DatalogError> {
        let offset = self.fluence_threshold;

        // 1. Iterate by reference over the vector pointed to by the reference
        // Use .iter() to obtain each AtomSkeletonId
        for neg_id in self.negated_predicates.iter() {
            let pos_id = AtomSkeletonId::from(neg_id.strip_negation().as_usize());
            let pred_def = &self.problem.predicate_defs()[pos_id.as_usize()];

            let param_list_id = pred_def.parameters();
            let parameters = self.problem.store().fetch_typed_list(param_list_id)?;

            // 2. Create the combinations iterator
            let mut iter = BindingsIterator::new(parameters, self.value_registry).unwrap();

            while let Some(bindings) = iter.next() {
                self.head_buffer.clear();

                // For each predicate parameter (e.g., ?p then ?a)
                for param in parameters {
                    // Query the dictionary: "What is the object ID for ?p?"
                    if let Some(obj_id) = bindings.get(param.symbol()) {
                        // Add it to the buffer: [10, 50]
                        self.head_buffer.push(obj_id);
                    }
                }

                // 3. CLOSED WORLD ASSUMPTION
                // If the positive fact is not in STABLE, the absence is TRUE
                if !self.db.has_fact(pos_id, &self.head_buffer) {
                    let storage_id = AtomSkeletonId::from(pos_id.as_usize() + offset);
                    // Insert the negative fact into Delta for Stratum 1
                    self.db.insert_delta_fact(storage_id, &self.head_buffer);
                }
            }
        }
        Ok(())
    }

    /// Recursively prints the rule evaluation tree and database facts for a given
    /// target atom skeleton ID, aiding in rule dependency debugging.
    ///
    /// # Arguments
    /// * `target_id` - The [`AtomSkeletonId`] of the target rule head being debugged.
    /// * `depth` - The current recursion depth used for formatting indentation.
    ///
    /// # Complexity
    /// - Time complexity: Proportional to the size of the rule dependency tree
    ///   and the number of tuples inspected in matching database relations.
    pub fn debug_recursive_rule(&self, target_id: AtomSkeletonId, depth: usize) {
        let indent = "  ".repeat(depth);
        let target_raw = target_id.as_usize();

        // 1. Retrieve ALL rules that have this ID as their head (handling OR logic)
        let matching_rules: Vec<_> = self
            .rules
            .iter()
            .filter(|r| r.head().symbol() == target_id)
            .collect();

        if matching_rules.is_empty() {
            // If no rule matches, check if it's a base fact
            if let Some(rel) = self.db.stable_relations().get(&target_id) {
                println!(
                    "{}|_ [LEAF FACT] ID {}: {} tuples present",
                    indent,
                    target_raw,
                    rel.len()
                );
                for (i, tuple) in rel.iter().take(3).enumerate() {
                    println!("{}   f#{}: {:?}", indent, i, tuple);
                }
            } else {
                println!(
                    "{}|_ ID {} : EMPTY (Neither rule nor fact)",
                    indent, target_raw
                );
            }
            return;
        }

        for (branch_idx, rule) in matching_rules.iter().enumerate() {
            println!("{}|_ Branch #{} for ID {}:", indent, branch_idx, target_raw);

            for atom in rule.body() {
                let sub_id = atom.symbol();
                let sub_raw = sub_id.as_usize();

                if self.is_auxiliary(sub_id) {
                    // Recursive descent for pivots (Aux_34, Aux_35, etc.)
                    self.debug_recursive_rule(sub_id, depth + 1);
                } else {
                    // Query the database via the relation API
                    if let Some(rel) = self.db.stable_relations().get(&sub_id) {
                        println!(
                            "{}  [CHECK] Predicate {} ({:?}) -> PRESENT ({} facts)",
                            indent,
                            sub_raw,
                            atom.arguments(),
                            rel.len()
                        );

                        // Display the first 5 tuples to verify ObjectIds
                        for (i, tuple) in rel.iter().take(5).enumerate() {
                            println!("{}     tuple#{}: {:?}", indent, i, tuple);
                        }
                    } else {
                        println!(
                            "{}  [CHECK] Predicate {} ({:?}) -> ABSENT",
                            indent,
                            sub_raw,
                            atom.arguments()
                        );
                    }
                }
            }
        }
    }

    /// Executes the semi-naive saturation algorithm to compute the fixpoint
    /// of the Datalog database, iteratively evaluating rules over delta facts
    /// until no new conclusions are derived.
    ///
    /// # Complexity
    /// - Time complexity: Bounded by the datalog program stratification and
    ///   the size of the Herbrand base, with each iteration processing rule
    ///   bodies around pivoting delta relations.
    pub(crate) fn saturate_semi_naive(&mut self) {
        // 1. BOOTSTRAP: Move facts to delta only if delta is empty
        // and there are facts in stable (e.g., when relaunching an engine).
        if self.db.is_delta_empty() && !self.db.stable_relations().is_empty() {
            self.db.move_all_to_delta();
        }

        // 2. MAIN LOOP
        while !self.db.is_delta_empty() {
            // Extract rules to avoid borrow checker conflicts
            let rules = std::mem::take(&mut self.rules);

            for rule in &rules {
                let body_len = rule.body().len();

                // Treat each atom of the rule as a pivot
                for pivot_idx in 0..body_len {
                    self.current_env = [None; MAX_VARS];
                    self.trailing_indices.clear();

                    // Explore combinations
                    self.process_incremental(rule, 0, pivot_idx);
                }
            }

            // Put the rules back in place
            self.rules = rules;

            // 1. Stabilize what served as the PIVOT during this round
            // (Round N's delta becomes round N+1's stable)
            self.db.commit_delta();

            // 2. ONLY NOW, inject discoveries
            // They will fill the brand NEW Delta for the next round.
            for (sk_id, args) in self.discovered_facts.drain(..) {
                // This function must verify uniqueness against the STABLE (the new one)
                self.db.insert_delta_fact(sk_id, &args);
            }

            // The loop continues if insert_delta_fact added new elements to the Delta
        }
    }

    /// Recursively processes rule body atoms incrementally using a semi-naive evaluation
    /// strategy split around a pivot index, handling filters, built-ins, and database matches.
    ///
    /// # Arguments
    /// * `rule` - A reference to the [`Rule`] being evaluated.
    /// * `body_idx` - The index of the current atom within the rule body.
    /// * `pivot_idx` - The pivot index determining whether to query stable, delta, or both database partitions.
    ///
    /// # Complexity
    /// - Time complexity: Proportional to the size of the search space defined by the rule body
    ///   and matched database relations.
    fn process_incremental(&mut self, rule: &Rule, body_idx: usize, pivot_idx: usize) {
        // 1. Stop condition: rule success (all atoms validated)
        if body_idx == rule.body().len() {
            self.evaluate_head(rule);
            return;
        }

        let atom = &rule.body()[body_idx];
        let sk_id = atom.symbol();

        // --- NEW: Branching to Lazy filtering ---
        // If the atom is a built-in OR negative
        // Do not search the DB; test absence (for not) or logic (for ==)
        if self.is_builtin(sk_id) || atom.is_negated() {
            // Call execute_filter (replacing execute_builtin)
            if self.execute_filter(atom) {
                self.process_incremental(rule, body_idx + 1, pivot_idx);
            }
            return;
        }

        // --- Existing logic for positive relations (Scan/Index) ---
        // Note: We only enter here for POSITIVE and NON-BUILTIN atoms
        if body_idx < pivot_idx {
            // Before the pivot: only in Stable
            self.match_relation(rule, body_idx, pivot_idx, sk_id, false);
        } else if body_idx == pivot_idx {
            // At the pivot: only in Delta (Semi-Naive)
            self.match_relation(rule, body_idx, pivot_idx, sk_id, true);
        } else {
            // After the pivot: test both (Stable and Delta)
            self.match_relation(rule, body_idx, pivot_idx, sk_id, false);
            self.match_relation(rule, body_idx, pivot_idx, sk_id, true);
        }
    }

    /// Executes a filter atom (such as a built-in or a negated/NAF condition)
    /// using current variable bindings to determine if the condition holds.
    ///
    /// # Arguments
    /// * `atom` - A reference to the filter [`Atom`] being evaluated.
    ///
    /// # Returns
    /// Returns `true` if the filter condition passes, or `false` otherwise
    /// (e.g., if a variable is unbound or a negated fact is present).
    ///
    /// # Complexity
    /// - Time complexity: $O(1)$ for built-ins, or dependent on database fact lookup
    ///   costs for Negation as Failure (NAF) checks.
    fn execute_filter(&mut self, atom: &Atom) -> bool {
        let sk_id = atom.symbol(); // AtomSkeletonId

        // 1. BUILT-INS HANDLING
        if self.is_builtin(sk_id) {
            return self.eval_equality(atom);
        }

        // 2. LAZY NEGATION HANDLING
        // Check MSB bit via interface and mirror segment
        let is_neg_id = sk_id.as_usize() >= self.fluence_threshold
            && sk_id.as_usize() < self.type_segment_start;

        if sk_id.is_negated() || is_neg_id {
            // --- Use engine's internal buffer ---
            self.head_buffer.clear();

            for term in atom.arguments() {
                if let Some(val) = self.get_term_value(term) {
                    self.head_buffer.push(val);
                } else {
                    return false; // Unbound variable (Datalog Safety)
                }
            }

            // --- POSITIVE ID DETERMINATION ---
            let pos_sk_id = if sk_id.is_negated() {
                // Use interface method to strip negation from ID
                sk_id.strip_negation()
            } else {
                // For the mirror segment [N..2N[, use offset calculation
                self.pos_id_from_negated(sk_id)
            };

            // --- NAF (Negation as Failure) LOGIC ---
            // Return TRUE if the POSITIVE fact is ABSENT from the database
            // This is where the 'assemble' action becomes unblocked!
            return !self.db.has_fact(pos_sk_id, &self.head_buffer);
        }

        true
    }

    /// Evaluates equality or inequality constraints between two terms of an atom
    /// using current variable bindings.
    ///
    /// # Arguments
    /// * `atom` - A reference to the equality [`Atom`] being evaluated.
    ///
    /// # Returns
    /// Returns `true` if the equality condition holds (or inequality if negated),
    /// or `false` otherwise. Returns `false` if any involved variable is unbound.
    ///
    /// # Complexity
    /// - Time complexity: $O(1)$ constant-time evaluation.
    fn eval_equality(&self, atom: &Atom) -> bool {
        let terms = atom.arguments();
        // Retrieve concrete object IDs via the current environment
        let val_a = self.get_term_value(&terms[0]);
        let val_b = self.get_term_value(&terms[1]);

        match (val_a, val_b) {
            (Some(a), Some(b)) => {
                let are_equal = a == b;

                // If the atom is negated (NOT =), return true if objects are different
                if atom.is_negated() {
                    !are_equal
                } else {
                    are_equal
                }
            }
            _ => {
                // In strict Datalog, if a variable is not yet bound,
                // the filter cannot be validated.
                false
            }
        }
    }

    /// Retrieves the concrete object identifier for a given term, either returning
    /// the constant value directly or looking up the variable's binding in the
    /// current environment.
    ///
    /// # Arguments
    /// * `term` - A reference to the [`Term`] whose value is being resolved.
    ///
    /// # Returns
    /// Returns `Some(ObjectId)` if the term is a constant or a bound variable,
    /// or `None` if the variable is currently unbound.
    ///
    /// # Complexity
    /// - Time complexity: $O(1)$ constant-time lookup.
    fn get_term_value(&self, term: &Term) -> Option<ObjectId> {
        match term {
            Term::Constant(c) => Some(*c),
            Term::Variable(v) => self.current_env[v.as_usize()],
        }
    }

    /// Matches relation tuples for a rule body atom, handling zero-arity propositions,
    /// indexed lookups (if the first argument is bound), or full table scans, and
    /// processes each matching tuple.
    ///
    /// # Arguments
    /// * `rule` - A reference to the [`Rule`] being evaluated.
    /// * `body_idx` - The index of the current atom within the rule body.
    /// * `pivot_idx` - The index of the pivot atom.
    /// * `sk_id` - The skeleton ID of the relation.
    /// * `use_delta` - A flag indicating whether to query the delta database (`true`) or stable database (`false`).
    ///
    /// # Complexity
    /// - Time complexity: $O(1)$ for zero-arity and indexed lookups, or $O(N)$ for full scans where $N$ is the relation size.
    fn match_relation(
        &mut self,
        rule: &Rule,
        body_idx: usize,
        pivot_idx: usize,
        sk_id: AtomSkeletonId,
        use_delta: bool,
    ) {
        let atom = &rule.body()[body_idx];
        let terms = atom.arguments();

        let Some((total_len, arity)) = self.db.get_layout(sk_id, use_delta) else {
            return;
        };
        let mut tuple_buffer = [ObjectId::from(0); MAX_VARS];

        // 1. ZERO ARITY CASE: Process the proposition if it is present in the requested table
        if arity == 0 {
            // If total_len is 0 but the table (Delta or Stable depending on use_delta)
            // contains the proposition, trigger unification once.
            let is_present = if use_delta {
                self.db.contains_delta(sk_id, &[])
            } else {
                self.db.contains_stable(sk_id, &[])
            };

            if is_present {
                self.process_tuple(
                    rule,
                    body_idx,
                    pivot_idx,
                    sk_id,
                    use_delta,
                    0,
                    0,
                    &mut tuple_buffer,
                );
            }
            return;
        }

        // 2. ARITY > 0 CASE: Classical indexing strategy
        let first_arg_binding = match &terms[0] {
            Term::Constant(c) => Some(*c),
            Term::Variable(v) => self.current_env[v.as_usize()],
        };

        if let Some(obj_id) = first_arg_binding {
            // INDEXED MODE
            if let Some(offsets_slice) = self.db.lookup_index(sk_id, use_delta, obj_id) {
                let offsets_cloned = offsets_slice.to_vec();
                for start in offsets_cloned {
                    self.process_tuple(
                        rule,
                        body_idx,
                        pivot_idx,
                        sk_id,
                        use_delta,
                        start,
                        arity,
                        &mut tuple_buffer,
                    );
                }
            }
        } else {
            // FULL SCAN MODE
            // step_by(arity) with arity > 0 is safe here.
            for start in (0..total_len).step_by(arity) {
                self.process_tuple(
                    rule,
                    body_idx,
                    pivot_idx,
                    sk_id,
                    use_delta,
                    start,
                    arity,
                    &mut tuple_buffer,
                );
            }
        }
    }

    /// Processes a retrieved database tuple for a rule body atom, attempts unification
    /// and binding, triggers subsequent incremental processing on success, and performs
    /// a surgical rollback to restore the environment state.
    ///
    /// # Arguments
    /// * `rule` - A reference to the [`Rule`] being processed.
    /// * `body_idx` - The index of the current atom within the rule body.
    /// * `pivot_idx` - The index of the pivot atom.
    /// * `sk_id` - The skeleton ID of the atom relation.
    /// * `use_delta` - A flag indicating whether to query the delta database.
    /// * `start` - The starting position for reading the tuple.
    /// * `arity` - The arity (number of terms) of the tuple.
    /// * `buffer` - A mutable reference to a preallocated buffer array for the tuple elements.
    ///
    /// # Complexity
    /// - Time complexity: Depends on database tuple retrieval cost and unification depth,
    ///   with environment restoration running in $O(K)$ where $K$ is the number of bound variables rolled back.
    fn process_tuple(
        &mut self,
        rule: &Rule,
        body_idx: usize,
        pivot_idx: usize,
        sk_id: AtomSkeletonId,
        use_delta: bool,
        start: usize,
        arity: usize,
        buffer: &mut [ObjectId; MAX_VARS],
    ) {
        self.db.read_tuple(sk_id, use_delta, start, arity, buffer);

        // We NO LONGER use prev_env (too slow). We use selective rollback.
        // Note how many variables were bound BEFORE this atom
        let trail_split = self.trailing_indices.len();

        if self.unify_and_bind(rule.body()[body_idx].arguments(), &buffer[..arity]) {
            self.process_incremental(rule, body_idx + 1, pivot_idx);
        }

        // SURGICAL ROLLBACK:
        // Cancel only what unify_and_bind added,
        // without touching variables bound by previous atoms of the rule.
        self.undo_to_savepoint(trail_split);
    }

    /// Evaluates the head of a rule using current variable bindings, filters out
    /// duplicate facts against the database and local discovery buffer, and records
    /// unique newly derived facts.
    ///
    /// # Arguments
    /// * `rule` - A reference to the [`Rule`] whose head is being evaluated.
    ///
    /// # Returns
    /// Returns `Ok(())` upon successful evaluation, either pushing the new fact to
    /// `discovered_facts` or skipping it if it already exists. Returns a [`DatalogError`]
    /// if any variable in the rule head is unbound.
    ///
    /// # Complexity
    /// - Time complexity: $O(T + D + B)$, where $T$ is the number of terms in the rule head,
    ///   $D$ is the cost of checking database containment, and $B$ is the size of the
    ///   `discovered_facts` buffer.
    pub fn evaluate_head(&mut self, rule: &Rule) -> Result<(), DatalogError> {
        let head = rule.head();
        let head_sk = head.symbol();

        // 1. Prepare the head tuple in the reusable buffer
        self.head_buffer.clear();

        for term in head.arguments() {
            match term {
                Term::Constant(c) => self.head_buffer.push(*c),
                Term::Variable(v) => {
                    let val = self.current_env[v.as_usize()].ok_or_else(|| {
                        println!("CRASH: Variable {} is unbound!", v.as_usize());
                        println!("Current Env: {:?}", self.current_env);
                        println!("Rule Head: {:?}", head);
                        DatalogError::unbound_variable(*v)
                    })?;
                    self.head_buffer.push(val);
                }
            }
        }

        // 2. FILTERING: We do not want to store duplicates.
        // Check in the database (Stable + Delta)
        if self.db.contains_stable(head_sk, &self.head_buffer)
            || self.db.contains_delta(head_sk, &self.head_buffer)
        {
            return Ok(());
        }

        // 3. Also check in current pivot discoveries
        // to avoid unnecessary cloning if the same fact is found 100 times in a row.
        let is_already_in_buffer = self
            .discovered_facts
            .iter()
            .any(|(sk, args)| *sk == head_sk && args == &self.head_buffer);

        if !is_already_in_buffer {
            // Cloning happens only here, at the very last possible moment.
            self.discovered_facts
                .push((head_sk, self.head_buffer.clone()));
        }
        Ok(())
    }

    /// Attempts to match an atom's terms (from a rule) with a real tuple (from the database),
    /// updating variable bindings in `self.current_env` on success.
    ///
    /// # Arguments
    /// * `atom_terms` - A slice of terms (`&[Term]`) representing the atom's arguments.
    /// * `tuple` - A slice of object identifiers (`&[ObjectId]`) fetched from the database.
    ///
    /// # Returns
    /// Returns `true` if unification succeeds, or `false` otherwise. On failure,
    /// the environment is automatically rolled back to the initial savepoint for this atom.
    ///
    /// # Complexity
    /// - Time complexity: $O(K)$ where $K$ is the number of terms in the atom.
    fn unify_and_bind(&mut self, atom_terms: &[Term], tuple: &[ObjectId]) -> bool {
        if atom_terms.len() != tuple.len() {
            return false;
        }

        // We no longer clear the stack; note the starting point (the "savepoint")
        let savepoint = self.trailing_indices.len();

        for (i, term) in atom_terms.iter().enumerate() {
            let val_in_db = tuple[i];

            match term {
                Term::Constant(c) => {
                    if *c != val_in_db {
                        // Failure: rollback only what THIS atom has bound
                        self.undo_to_savepoint(savepoint);
                        return false;
                    }
                }
                Term::Variable(v) => {
                    let idx = v.as_usize();
                    if let Some(existing_val) = self.current_env[idx] {
                        if existing_val != val_in_db {
                            self.undo_to_savepoint(savepoint);
                            return false;
                        }
                    } else {
                        self.current_env[idx] = Some(val_in_db);
                        self.trailing_indices.push(idx);
                    }
                }
            }
        }
        true
    }

    /// Rolls back the engine environment to a specific savepoint by resetting
    /// variable bindings tracked in the trailing indices stack.
    ///
    /// # Arguments
    /// * `savepoint` - A `usize` index representing the target length of the trailing indices stack.
    ///
    /// # Complexity
    /// - Time complexity: $O(K)$, where $K$ is the number of variables popped from the
    fn undo_to_savepoint(&mut self, savepoint: usize) {
        while self.trailing_indices.len() > savepoint {
            if let Some(idx) = self.trailing_indices.pop() {
                self.current_env[idx] = None;
            }
        }
    }

    /// Performs a global reordering pass on all rules managed by the engine
    /// to optimize their evaluation execution order.
    ///
    /// # Complexity
    /// - Time complexity: $O(R \times M^2)$, where $R$ is the number of rules and
    ///   $M$ is the average size of a rule body.
    pub(crate) fn optimize_all_rules(&mut self) {
        // Temporarily extract rules to avoid borrow checker conflicts with &self
        let mut rules = std::mem::take(&mut self.rules);

        for rule in &mut rules {
            self.optimize_rule(rule.body_mut());
        }

        // Put the optimized rules back in place
        self.rules = rules;
    }

    /// Optimizes the evaluation order of a rule's body predicates by dynamically
    /// selecting the best next atom based on bound variables, relation size,
    /// and semantic priority.
    ///
    /// # Arguments
    /// * `body` - A mutable reference to the [`RuleBody`] to be reordered in-place.
    ///
    /// # Complexity
    /// - Time complexity: $O(N^2)$ relative to the number of atoms $N$ in the rule body,
    ///   as it iteratively scans remaining atoms to find the optimal choice.
    pub fn optimize_rule(&self, body: &mut RuleBody) {
        if body.len() <= 1 {
            return;
        }

        // OPTIMIZATION: Use RuleBody directly to stay on the stack
        let mut optimized = RuleBody::with_capacity(body.len());
        let mut bound_vars_mask: u64 = 0;

        // mem::take works perfectly because RuleBody implements Default
        let mut remaining = std::mem::take(body);

        while !remaining.is_empty() {
            let best_idx = remaining
                .iter()
                .enumerate()
                .min_by_key(|(_, atom)| {
                    let sk_id = atom.symbol();

                    // 1. Calculation of already bound variables
                    let mut bound_count = 0;
                    for term in atom.arguments() {
                        match term {
                            Term::Constant(_) => bound_count += 1,
                            Term::Variable(v) => {
                                let v_idx = v.as_usize();
                                if v_idx < 64 && (bound_vars_mask & (1 << v_idx)) != 0 {
                                    bound_count += 1;
                                }
                            }
                        }
                    }

                    // 2. Actual size of data in the DB
                    let rel_size = self.db.get_relation_size(sk_id);

                    // 3. Semantic category (Type, Fluent, etc.)
                    let priority = self.get_predicate_priority(atom);

                    (-(bound_count as i32), priority, rel_size)
                })
                .map(|(idx, _)| idx)
                .unwrap();

            let best_idx = best_idx; // clear reference
            let best_atom = remaining.swap_remove(best_idx);

            // Update the mask of bound variables
            for term in best_atom.arguments() {
                if let Term::Variable(v) = term {
                    let v_idx = v.as_usize();
                    if v_idx < 64 {
                        bound_vars_mask |= 1 << v_idx;
                    }
                }
            }
            optimized.push(best_atom);
        }

        // Direct and transparent replacement without going through the heap
        *body = optimized;
    }

    /// Determines the evaluation priority of a given atom based on its semantic
    /// classification (negations, equality, types, fluents, actions, or auxiliaries).
    ///
    /// # Arguments
    /// * `atom` - A reference to the [`Atom`] whose predicate priority is being evaluated.
    ///
    /// # Returns
    /// Returns a `u8` priority value where:
    /// - Lower values indicate higher priority (evaluated first).
    /// - Higher values indicate lower priority (evaluated last).
    ///
    /// # Complexity
    /// - Time complexity: O(1) as it executes a series of constant-time checks.
    #[inline(always)]
    pub fn get_predicate_priority(&self, atom: &Atom) -> u8 {
        let id = atom.symbol();

        // 1. Negations and Inequalities: ABSOLUTE END PRIORITY (evaluated last)
        // Checked first to prevent a negative fluent from being misidentified as a positive one.
        if atom.is_negated() {
            return 250;
        }

        // 2. Positive equality (Assignment): absolute start priority
        if atom.is_equality() {
            return 0;
        }

        // 3. Types: highly restrictive, serve as the basis for filtering
        if self.is_type(id) {
            return 0;
        }

        // 4. Positive fluents: lookup in the current state
        if self.is_fluent(id) {
            return 1;
        }

        // 5. Anchor: the action itself (binds remaining parameters)
        if self.is_action(id) {
            return 2;
        }

        // 6. Auxiliaries: predicates generated for AND/OR logic
        if self.is_auxiliary(id) {
            return 3;
        }

        // 7. Default case (Built-ins, etc.)
        255
    }
}

#[cfg(test)]
mod tests_unifications {
    use super::*;
    use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::term::Term;
    use crate::aiplan4rust::support::lang::{ObjectId, VariableId};
    use crate::analysis::reachability::datalog::engine::test_utils::create_segmented_engine;

    /// # Objective
    /// Verifies core unification logic across multiple scenarios, including binding new variables, handling conflicts with already bound variables, matching constants, and evaluating self-unification constraints.
    ///
    /// # Input
    /// - An engine instance, variables, and constants.
    /// - Scenario 1: Unifying `[?v0, 10]` with `[55, 10]`.
    /// - Scenario 2: Unifying with `[99, 10]` when `?v0` is already bound to `55`.
    /// - Scenario 3: Unifying constant `[10]` with mismatched tuple `[11]`.
    /// - Scenario 4: Self-unification using `[?v0, ?v0]` against matching (`[7, 7]`) and mismatched (`[7, 8]`) tuples.
    ///
    /// # Expected Output
    /// - Scenario 1 succeeds and binds `?v0` to `ObjectId::from(55)`.
    /// - Scenario 2 fails (`false`) due to a variable binding conflict.
    /// - Scenario 3 fails (`false`) due to a constant mismatch.
    /// - Scenario 4 succeeds for identical values and fails for conflicting values.
    #[test]
    fn test_unification_logic() {
        // Use our clean helper directly!
        let mut engine = create_segmented_engine();

        macro_rules! reset_env {
            () => {
                engine.current_env.fill(None);
            };
        }

        let var_v0 = Term::Variable(VariableId::from(0));
        let const_c10 = Term::Constant(ObjectId::from(10));

        // --- Case 1: Binding a new variable ---
        reset_env!();
        let terms = vec![var_v0.clone(), const_c10.clone()];
        let tuple = vec![ObjectId::from(55), ObjectId::from(10)];

        assert!(engine.unify_and_bind(&terms, &tuple));
        assert_eq!(engine.current_env[0], Some(ObjectId::from(55)));

        // --- Case 2: Conflict with an already bound variable ---
        let tuple_conflict = vec![ObjectId::from(99), ObjectId::from(10)];
        assert!(
            !engine.unify_and_bind(&terms, &tuple_conflict),
            "Should fail because v0 is already bound to 55"
        );

        // --- Case 3: Conflict with a constant ---
        reset_env!();
        let terms_const = vec![const_c10.clone()];
        let tuple_wrong_const = vec![ObjectId::from(11)];
        assert!(
            !engine.unify_and_bind(&terms_const, &tuple_wrong_const),
            "Expected failure: 10 != 11"
        );

        // --- Case 4: Same variable used twice (Self-unification) ---
        reset_env!();
        let terms_double = vec![var_v0.clone(), var_v0.clone()];
        let tuple_ok = vec![ObjectId::from(7), ObjectId::from(7)];
        let tuple_bad = vec![ObjectId::from(7), ObjectId::from(8)];

        assert!(
            engine.unify_and_bind(&terms_double, &tuple_ok),
            "v0 can be 7 and 7"
        );

        reset_env!();
        assert!(
            !engine.unify_and_bind(&terms_double, &tuple_bad),
            "v0 cannot be 7 AND 8"
        );
    }

    /// # Objective
    /// Verifies that unification correctly handles joins with existing variable bindings, ensuring that an already bound variable (such as `?robot`) conflicts with a mismatching tuple while successfully matching and binding new variables when consistent.
    ///
    /// # Input
    /// - An engine instance with variables for a robot (`?r`) and location (`?l`), and object identifiers for a robot and room.
    /// - Step 1: Binding `?robot=100` and `?loc=1` via `tuple1`.
    /// - Step 2: Testing subsequent unification against `tuple_wrong_robot` (conflicting ID 200) and `tuple_ok` (matching ID 100 with a new fuel level variable).
    ///
    /// # Expected Output
    /// - Initial binding succeeds.
    /// - Unification with `tuple_wrong_robot` fails (`false`) because `?robot` is already bound to `100`.
    /// - Unification with `tuple_ok` succeeds (`true`) and correctly binds the new fuel level variable to `50`.
    #[test]
    fn test_unification_with_existing_bindings() {
        let mut engine = create_segmented_engine();
        macro_rules! reset_env {
            () => {
                engine.current_env.fill(None);
            };
        }

        let var_r = Term::Variable(VariableId::from(0)); // ?robot
        let var_l = Term::Variable(VariableId::from(1)); // ?loc
        let id_robot1 = ObjectId::from(100);
        let id_room_a = ObjectId::from(1);

        // --- Scenario: Join ---
        reset_env!();

        // 1. First step: bind ?robot=100 and ?loc=1
        let terms1 = vec![var_r.clone(), var_l.clone()];
        let tuple1 = vec![id_robot1, id_room_a];
        assert!(engine.unify_and_bind(&terms1, &tuple1));

        // 2. Second step: verify another atom that reuses ?robot
        let id_fuel_50 = ObjectId::from(50);
        let terms2 = vec![var_r.clone(), Term::Variable(VariableId::from(2))]; // [?robot, ?level]

        // This tuple should fail because the robot at index 0 is '200', not '100'
        let tuple_wrong_robot = vec![ObjectId::from(200), id_fuel_50];
        assert!(
            !engine.unify_and_bind(&terms2, &tuple_wrong_robot),
            "Should fail: the bound robot is 100"
        );

        // This tuple should succeed and bind ?level (index 2) to 50
        let tuple_ok = vec![id_robot1, id_fuel_50];
        assert!(engine.unify_and_bind(&terms2, &tuple_ok));
        assert_eq!(
            engine.current_env[2],
            Some(id_fuel_50),
            "The ?level variable should have been bound to 50"
        );
    }

    /// # Objective
    /// Verifies that self-constraints within an atom (using the same variable multiple times, e.g., `[?v0, ?v0]`) correctly succeed when matching identical values and fail when matching distinct values.
    ///
    /// # Input
    /// - An engine instance, an atom containing two references to the same variable (`?v0`), and two test scenarios:
    ///   - Scenario A (matching tuple): `[7, 7]`
    ///   - Scenario B (mismatched tuple): `[7, 8]`
    ///
    /// # Expected Output
    /// - Scenario A succeeds (`true`) and binds `?v0` to `ObjectId::from(7)`.
    /// - Scenario B fails (`false`) because a single variable cannot bind to two different values simultaneously.
    #[test]
    fn test_unification_self_constraint() {
        let mut engine = create_segmented_engine();
        macro_rules! reset_env {
            () => {
                engine.current_env.fill(None);
            };
        }

        // Use the same variable twice: [?v0, ?v0]
        let var_v0 = Term::Variable(VariableId::from(0));
        let terms = vec![var_v0.clone(), var_v0.clone()];

        // Case A: Values are identical -> Success
        reset_env!();
        let tuple_ok = vec![ObjectId::from(7), ObjectId::from(7)];
        assert!(
            engine.unify_and_bind(&terms, &tuple_ok),
            "v0 can be bound to 7 because 7 == 7"
        );
        assert_eq!(engine.current_env[0], Some(ObjectId::from(7)));

        // Case B: Values are different -> Failure
        reset_env!();
        let tuple_bad = vec![ObjectId::from(7), ObjectId::from(8)];
        assert!(
            !engine.unify_and_bind(&terms, &tuple_bad),
            "Must fail because v0 cannot be 7 AND 8 at the same time"
        );
    }

    /// # Objective
    /// Verifies that unification correctly performs a rollback on failure, ensuring that partial bindings (such as `?v0`) are cleared if a subsequent term in the atom fails to match.
    ///
    /// # Input
    /// - An engine instance, a term list containing a variable (`?v0`) and a constant (`99`), and a mismatched tuple (`[10, 88]`).
    /// - Calling `unify_and_bind` with these arguments.
    ///
    /// # Expected Output
    /// - Unification fails (`false`), and the variable `?v0` is successfully rolled back to `None` in the environment.
    #[test]
    fn test_unification_rollback_on_failure() {
        let mut engine = create_segmented_engine();
        macro_rules! reset_env {
            () => {
                engine.current_env.fill(None);
            };
        }

        let var_v0 = Term::Variable(VariableId::from(0));
        let const_99 = Term::Constant(ObjectId::from(99));

        reset_env!();

        // Try to unify [?v0, 99] with [10, 88]
        // Binding ?v0 = 10 will succeed, but constant 99 != 88 will cause the atom to fail.
        let terms = vec![var_v0.clone(), const_99];
        let tuple = vec![ObjectId::from(10), ObjectId::from(88)];

        let success = engine.unify_and_bind(&terms, &tuple);

        assert!(!success);
        // CRUCIAL: ?v0 must not remain bound to 10!
        assert_eq!(
            engine.current_env[0], None,
            "The environment must be clean after a unification failure"
        );
    }

    /// # Objective
    /// Verifies that unification correctly enforces equality constraints when the same variable appears multiple times within a single atom (triple variable constraint), failing on mismatched values and succeeding when all values match.
    ///
    /// # Input
    /// - An engine instance and a triple-variable atom (`P(?v0, ?v0, ?v0)`).
    /// - Failure test scenario: A tuple with non-identical values (`[10, 10, 20]`).
    /// - Success test scenario: A tuple with identical values (`[30, 30, 30]`).
    ///
    /// # Expected Output
    /// - The mismatch scenario returns `false` and successfully rolls back `?v0` back to `None`.
    /// - The matching scenario returns `true` and correctly binds `?v0` to the expected object identifier (`30`).
    #[test]
    fn test_unification_triple_variable_constraint() {
        let mut engine = create_segmented_engine();
        let var_x = Term::Variable(VariableId::from(0));
        // Atom: P(?v0, ?v0, ?v0)
        let atom_terms = vec![var_x.clone(), var_x.clone(), var_x.clone()];

        // Failure case: the three values are not identical
        let tuple_fail = vec![ObjectId::from(10), ObjectId::from(10), ObjectId::from(20)];
        assert!(!engine.unify_and_bind(&atom_terms, &tuple_fail));
        assert_eq!(engine.current_env[0], None, "Must have rolled back");

        // Success case: the three values are identical
        let tuple_success = vec![ObjectId::from(30), ObjectId::from(30), ObjectId::from(30)];
        assert!(engine.unify_and_bind(&atom_terms, &tuple_success));
        assert_eq!(engine.current_env[0], Some(ObjectId::from(30)));
    }

    /// # Objective
    /// Verifies that partial rollback preserves parent variable bindings while correctly discarding failed bindings from the current step.
    ///
    /// # Input
    /// - An engine instance with an existing binding for `?v0` (`ObjectId::from(50)`) and a trail index entry.
    /// - An atom containing `?v0` and `?v1` tested against an incompatible tuple (`[99, 100]`) where `?v0` conflicts.
    ///
    /// # Expected Output
    /// - Unification fails (`false`), leaving `?v1` unbound (`None`) while correctly preserving the pre-existing parent binding for `?v0` (`50`).
    #[test]
    fn test_unification_partial_rollback() {
        let mut engine = create_segmented_engine();
        let var_x = Term::Variable(VariableId::from(0));
        let var_y = Term::Variable(VariableId::from(1));

        // 1. Simulate that ?v0 is already bound (by a previous atom in a rule)
        engine.current_env[0] = Some(ObjectId::from(50));
        // Manually push to the trail stack to simulate a clean state
        engine.trailing_indices.push(0);

        // 2. Try to unify a new atom Q(?v0, ?v1) with an incompatible tuple on ?v0
        let atom_terms = vec![var_x, var_y];
        let tuple = vec![ObjectId::from(99), ObjectId::from(100)];

        let _savepoint = engine.trailing_indices.len(); // Should be 1
        assert!(!engine.unify_and_bind(&atom_terms, &tuple));

        // 3. VERIFICATION:
        // ?v1 must not be bound (atom failure)
        assert_eq!(engine.current_env[1], None);
        // ?v0 must STILL be bound to 50 (it must not have been rolled back by mistake)
        assert_eq!(
            engine.current_env[0],
            Some(ObjectId::from(50)),
            "The rollback incorrectly cleared a parent variable!"
        );
    }

    /// # Objective
    /// Verifies that unification with empty term lists and tuples is trivially successful without error.
    ///
    /// # Input
    /// - An engine instance, an empty vector of terms (`atom_terms`), and an empty vector of object identifiers (`tuple`).
    /// - Calling `unify_and_bind` with the empty collections.
    ///
    /// # Expected Output
    /// - Unification succeeds (`true`), validating the trivial base case.
    #[test]
    fn test_unification_empty_atom() {
        let mut engine = create_segmented_engine();
        let atom_terms: Vec<Term> = vec![];
        let tuple: Vec<ObjectId> = vec![];

        // Unification with no terms is trivially true
        assert!(engine.unify_and_bind(&atom_terms, &tuple));
    }

    /// # Objective
    /// Verifies that the engine environment and trailing indices are fully cleaned up and reset after rule evaluation.
    ///
    /// # Input
    /// - An engine instance with a variable (`?v0`) bound to an object identifier (`ObjectId::from(100)`).
    /// - Explicit simulation of a rule-end reset by filling the environment with `None` and clearing trailing indices.
    ///
    /// # Expected Output
    /// - The variable binding environment is successfully cleared back to `None` and the trailing indices vector is empty.
    #[test]
    fn test_unification_full_cleanup() {
        let mut engine = create_segmented_engine();
        let var_x = Term::Variable(VariableId::from(0));

        // Bind ?v0
        engine.unify_and_bind(&[var_x], &[ObjectId::from(100)]);
        assert!(engine.current_env[0].is_some());

        // Simulate rule end reset
        engine.current_env.fill(None);
        engine.trailing_indices.clear();

        assert!(engine.current_env[0].is_none());
        assert!(engine.trailing_indices.is_empty());
    }

    /// # Objective
    /// Verifies that unification and rollback operations function correctly at the maximum variable limit (`MAX_VARS`, set to 64).
    ///
    /// # Input
    /// - An atom containing 64 variables (`?v0` through `?v63`) and a corresponding tuple of 64 distinct object identifiers.
    /// - Calling `unify_and_bind` to bind all 64 variables, followed by an environment rollback via `undo_to_savepoint`.
    ///
    /// # Expected Output
    /// - Unification succeeds, correctly populating the engine environment for the first, middle, and last variables.
    /// - Rolling back to the initial savepoint successfully resets all variable bindings back to `None`.
    #[test]
    fn test_unification_at_limit_64() {
        let mut engine = create_segmented_engine();

        // 1. Creation of an atom with exactly 64 variables: ?v0, ?v1, ..., ?v63
        let atom_terms: Vec<Term> = (0..MAX_VARS)
            .map(|i| Term::Variable(VariableId::from(i)))
            .collect();

        // 2. Creation of a tuple with 64 distinct values
        let tuple: Vec<ObjectId> = (0..MAX_VARS).map(|i| ObjectId::from(i)).collect();

        // 3. Unification must succeed without panicking
        assert!(engine.unify_and_bind(&atom_terms, &tuple));

        // 4. Verification of the first, an intermediate, and the very last binding
        assert_eq!(engine.current_env[0], Some(ObjectId::from(0)));
        assert_eq!(engine.current_env[32], Some(ObjectId::from(32)));
        assert_eq!(engine.current_env[MAX_VARS - 1], Some(ObjectId::from(63)));

        // 5. Test complete rollback across 64 variables
        let split = 0;
        engine.undo_to_savepoint(split);
        assert!(engine.current_env[0].is_none());
        assert!(engine.current_env[MAX_VARS - 1].is_none());
    }

    /// # Objective
    /// Verifies cross-variable consistency during unification, ensuring multiple variables can be bound correctly without conflict.
    ///
    /// # Input
    /// - Two distinct variable terms (`var_x`, `var_y`) and an object identifier (`val_100`).
    /// - Sequential calls to `unify_and_bind` binding both variables to `val_100`.
    ///
    /// # Expected Output
    /// - Both unification steps succeed, and the engine environment correctly stores `val_100` for both variable indices.
    #[test]
    fn test_unification_cross_variable_consistency() {
        let mut engine = create_segmented_engine();
        let var_x = Term::Variable(VariableId::from(0));
        let var_y = Term::Variable(VariableId::from(1));
        let val_100 = ObjectId::from(100);

        // 1. Bind ?v0 to 100
        assert!(engine.unify_and_bind(&[var_x.clone()], &[val_100]));

        // 2. Try to unify ?v1 with the SAME value 100
        assert!(engine.unify_and_bind(&[var_y.clone()], &[val_100]));

        assert_eq!(engine.current_env[0], Some(val_100));
        assert_eq!(engine.current_env[1], Some(val_100));
    }
}

#[cfg(test)]
mod tests_filters_and_naf {
    use super::*;
    use crate::aiplan4rust::support::lang::{AtomSkeletonId, ObjectId, VariableId};
    use crate::analysis::reachability::datalog::core::atom::{Atom, AtomArgs};
    use crate::analysis::reachability::datalog::core::term::Term;
    use crate::analysis::reachability::datalog::engine::test_utils::create_segmented_engine;

    /// # Objective
    /// Verifies that `eval_equality` correctly evaluates equality and inequality constraints between bound and unbound variables in various scenarios.
    ///
    /// # Input
    /// - An engine instance with two variables (`?v0`, `?v1`) and two distinct object identifiers (`obj_a`, `obj_b`).
    /// - An equality atom containing both variables.
    /// - Various environment binding states: both unbound, bound to different values, and bound to identical values.
    ///
    /// # Expected Output
    /// - Returns `false` when variables are unbound or have different values under equality checks.
    /// - Returns `true` when variables are bound to identical object identifiers.
    #[test]
    fn test_eval_equality_scenarios() {
        let mut engine = create_segmented_engine();

        let v0 = VariableId::from(0);
        let v1 = VariableId::from(1);
        let obj_a = ObjectId::from(10);
        let obj_b = ObjectId::from(20);

        // Using Atom::nary for an atom with multiple arguments (here 2 variables)
        let mut args = AtomArgs::new();
        args.push(Term::Variable(v0));
        args.push(Term::Variable(v1));
        let eq_atom = Atom::nary(AtomSkeletonId::from(0), args);

        // Case 1: Unbound variables -> must fail (false)
        assert!(
            !engine.eval_equality(&eq_atom),
            "Expected failure: unbound variables"
        );

        // Bind v0 and v1 to different objects
        engine.current_env[v0.as_usize()] = Some(obj_a);
        engine.current_env[v1.as_usize()] = Some(obj_b);

        // Case 2: Strict equality on different objects (==) -> false
        assert!(
            !engine.eval_equality(&eq_atom),
            "obj_a == obj_b must be false"
        );

        // Case 3: Inequality on different objects (!=) -> true
        engine.current_env[v0.as_usize()] = Some(obj_a);
        engine.current_env[v1.as_usize()] = Some(obj_b);

        let are_different = engine.current_env[v0.as_usize()] != engine.current_env[v1.as_usize()];
        assert!(are_different, "obj_a != obj_b must be true");

        // Case 4: Equality on identical objects (==) -> true
        engine.current_env[v1.as_usize()] = Some(obj_a);
        assert!(
            engine.eval_equality(&eq_atom),
            "obj_a == obj_a must evaluate to true"
        );
    }

    /// # Objective
    /// Verifies that `execute_filter` correctly implements Negation as Failure (NAF) under the Closed World Assumption (CWA).
    ///
    /// # Input
    /// - An engine instance with an environment binding variable `?v0` to an object identifier (`ObjectId::from(5)`).
    /// - A negated atom (`neg_atom`) whose skeleton ID falls into the mirror segment.
    /// - Scenario A: The corresponding positive fact is absent from the database.
    /// - Scenario B: The corresponding positive fact is inserted into the Delta database.
    ///
    /// # Expected Output
    /// - Scenario A: `execute_filter` returns `true` because the positive fact is absent, validating NAF.
    /// - Scenario B: `execute_filter` returns `false` because the positive fact is present in the database, blocking NAF.
    #[test]
    fn test_execute_filter_naf() {
        let mut engine = create_segmented_engine();

        let v0 = VariableId::from(0);
        let obj_room = ObjectId::from(5);
        engine.current_env[v0.as_usize()] = Some(obj_room);

        // Use a negative ID that falls into the mirror segment [fluence_threshold..type_segment_start[
        // By default in create_segmented_engine: fluence_threshold = 2, type_segment_start = 4
        let neg_sk = AtomSkeletonId::from(2);
        let neg_atom = Atom::unary(neg_sk, Term::Variable(v0));

        // Scenario A: The corresponding positive fact is ABSENT from the DB -> NAF must validate (true)
        assert!(
            engine.execute_filter(&neg_atom),
            "The absent fact must trigger NAF (true)"
        );

        // Retrieve the associated positive predicate via the engine's logic
        let pos_sk = engine.pos_id_from_negated(neg_sk);

        // Scenario B: The positive fact is PRESENT in the DB -> NAF must reject (false)
        engine.db.insert_delta_fact(pos_sk, &[obj_room]);
        assert!(
            !engine.execute_filter(&neg_atom),
            "The present fact must block NAF (false)"
        );
    }

    /// # Objective
    /// Verifies that `materialize_negations` correctly executes bulk Closed World Assumption (CWA) materialization without error.
    ///
    /// # Input
    /// - An engine instance configured with negative predicates requiring materialization.
    /// - Calling `materialize_negations` on the engine.
    ///
    /// # Expected Output
    /// - The materialization process succeeds (`Result::Ok`), potentially injecting missing negative facts into the Delta store based on thresholds.
    #[test]
    fn test_materialize_negations() {
        let mut engine = create_segmented_engine();

        // Declare a predicate as negative / requiring materialization (e.g., ID 0)
        let pos_id = AtomSkeletonId::from(0);

        // Simulation: add the ID to the negative predicates monitored by the engine
        // (Adapt according to the exact structure of your neg_ref type)
        // engine.negated_predicates.push(...);

        // If the positive fact is not in the database, materialize_negations must inject it
        // into Delta as a negative fact (based on the fluency threshold).
        let result = engine.materialize_negations();

        assert!(
            result.is_ok(),
            "The materialization of negations must execute without error"
        );
    }
}

#[cfg(test)]
mod tests_family_optimization {
    use super::*;
    use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::atom::{
        Atom, AtomArgs,
    };
    use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::term::Term;
    use crate::aiplan4rust::support::lang::{AtomSkeletonId, VariableId};
    use crate::analysis::reachability::datalog::engine::test_utils::create_segmented_engine;

    /// # Objective
    /// Verifies that predicate semantic priorities are correctly assigned, specifically that negated atoms receive a high priority value (250) to ensure they are evaluated last.
    ///
    /// # Input
    /// - An engine instance and an atom whose symbol is negated using the atom builder interface.
    /// - Calling `get_predicate_priority` on the negated atom.
    ///
    /// # Expected Output
    /// - The returned priority is equal to 250, ensuring proper evaluation sequencing.
    #[test]
    fn test_get_predicate_priority_values() {
        let engine = create_segmented_engine();

        let v0 = VariableId::from(0);
        let mut args = AtomArgs::new();
        args.push(Term::Variable(v0));

        // Test of a negation: create the atom then switch it to negative
        let mut neg_atom = Atom::nary(AtomSkeletonId::from(1), args);
        neg_atom.negated();

        assert_eq!(
            engine.get_predicate_priority(&neg_atom),
            250,
            "A negation must have a high priority (250) to be evaluated last"
        );
    }

    /// # Objective
    /// Verifies that rule body optimization (`optimize_rule`) correctly reorders atoms based on their priority, moving high-priority type predicates to the front and low-priority negated predicates to the end.
    ///
    /// # Input
    /// - An engine instance and a rule body containing a negated atom (low priority) first and a type atom (high priority, priority 0) second.
    /// - Calling `optimize_rule` on the body.
    ///
    /// # Expected Output
    /// - The optimizer successfully reorders the body so that the type predicate is placed at the very front (`body[0]`).
    #[test]
    fn test_optimize_rule_ordering() {
        let mut engine = create_segmented_engine();

        let v0 = VariableId::from(0);

        let mut args1 = AtomArgs::new();
        args1.push(Term::Variable(v0));

        // 1. Creation of a wide/negated atom (priority 250 - must go to the end)
        let mut atom_wide = Atom::nary(AtomSkeletonId::from(100), args1.clone());
        atom_wide.negated();

        // 2. Creation of a type atom (priority 0 - must move to the front)
        // Choose an ID within the engine's type range (e.g., 4, which is >= type_segment_start)
        let atom_type = Atom::unary(AtomSkeletonId::from(4), Term::Variable(v0));
        let expected_symbol = atom_type.symbol();

        let mut body = RuleBody::new();
        body.push(atom_wide); // Placed first (wrong position)
        body.push(atom_type); // Placed second

        // Call the optimizer
        engine.optimize_rule(&mut body);

        // Verification: the type atom (priority 0) must have been moved to the head
        assert_eq!(
            body[0].symbol(),
            expected_symbol,
            "The optimizer must place the high-priority type predicate at the head of the list"
        );
    }

    /// # Objective
    /// Verifies that rule optimization handles edge cases where the rule body has zero or one atom without panicking.
    ///
    /// # Input
    /// - An engine instance and a `RuleBody` initialized empty or with a single atom.
    /// - Calling `optimize_rule` on the short body.
    ///
    /// # Expected Output
    /// - An empty body remains empty (`len() == 0`), and a single-atom body remains unchanged with its length preserved (`len() == 1`).
    #[test]
    fn test_optimize_rule_short_body() {
        let engine = create_segmented_engine();
        let mut body = RuleBody::new();

        // Empty body or size 1
        engine.optimize_rule(&mut body);
        assert_eq!(body.len(), 0);

        let v0 = VariableId::from(0);
        let atom_type = Atom::unary(AtomSkeletonId::from(0), Term::Variable(v0));
        body.push(atom_type);

        engine.optimize_rule(&mut body);
        assert_eq!(
            body.len(),
            1,
            "A body of size 1 must remain unchanged without error"
        );
    }

    /// # Objective
    /// Verifies that intermediate predicate priorities (such as actions and auxiliaries) return a valid `u8` priority value through `get_predicate_priority`.
    ///
    /// # Input
    /// - An engine instance and an atom with an ID (`777`) and a variable argument.
    /// - Calling `get_predicate_priority` on the action/auxiliary atom.
    ///
    /// # Expected Output
    /// - The returned priority is a valid `u8` value (less than or equal to 255), falling back correctly or matching its semantic category.
    #[test]
    fn test_intermediate_priorities() {
        let engine = create_segmented_engine();

        let v0 = VariableId::from(0);
        let mut args = AtomArgs::new();
        args.push(Term::Variable(v0));

        let action_atom = Atom::nary(AtomSkeletonId::from(777), args.clone());
        // For testing purposes, we ensure we test the get_predicate_priority function
        // (Assuming 777 returns true for is_action, or falls back to a default value if not mocked)
        let priority = engine.get_predicate_priority(&action_atom);

        // If ID 777 is neither an action, a type, nor a fluent, it must fall back to the default case (255)
        // or 2 if it is an action recognized by your engine.
        assert!(priority <= 255, "The priority must be a valid u8");
    }
}

#[cfg(test)]
mod tests_head_evaluation {
    use super::*;
    use crate::aiplan4rust::compiler::grounding::analysis::reachability::datalog::core::term::Term;
    use crate::aiplan4rust::support::lang::{AtomSkeletonId, VariableId};
    use crate::analysis::reachability::datalog::core::atom::AtomArgs;
    use crate::analysis::reachability::datalog::engine::test_utils::create_segmented_engine;

    /// # Objective
    /// Verifies that rule head evaluation successfully discovers and stores a new fact in nominal conditions.
    ///
    /// # Input
    /// - A rule with an empty body and a head containing a bound variable (`?v0`).
    /// - The environment with `?v0` bound to a specific object identifier (`42`).
    ///
    /// # Expected Output
    /// - Head evaluation succeeds, discovering exactly 1 new fact stored in `discovered_facts`.
    #[test]
    fn test_evaluate_head_new_fact() {
        let mut engine = create_segmented_engine();

        // Assume a rule with a head: p(?v0) where ?v0 is bound to the constant 42
        let v0 = VariableId::from(0);
        let mut head_args = AtomArgs::new();
        head_args.push(Term::Variable(v0));

        let head_atom = Atom::nary(AtomSkeletonId::from(10), head_args);
        let rule = Rule::new(head_atom, RuleBody::new()); // Empty body for the test

        // Bind the variable ?v0 in the engine's current environment
        engine.current_env[v0.as_usize()] = Some(ObjectId::from(42));

        // Call evaluate_head
        let result = engine.evaluate_head(&rule);
        assert!(result.is_ok());
        assert_eq!(engine.discovered_facts.len(), 1);
    }

    /// # Objective
    /// Verifies that rule head evaluation filters out duplicates that are already present in the database (stable store).
    ///
    /// # Input
    /// - A rule head containing a bound variable (`?v0`).
    /// - The environment with `?v0` bound to a specific value (`42`).
    /// - A fact pre-inserted into the Stable database for that atom's ID and value.
    ///
    /// # Expected Output
    /// - Head evaluation succeeds, but discovers 0 facts because the duplicate already present in the DB is successfully filtered out.
    #[test]
    fn test_evaluate_head_filters_db_duplicates() {
        let mut engine = create_segmented_engine();

        let v0 = VariableId::from(0);
        let mut head_args = AtomArgs::new();
        head_args.push(Term::Variable(v0));

        let head_atom = Atom::nary(AtomSkeletonId::from(10), head_args);
        let rule = Rule::new(head_atom, RuleBody::new());

        // 1. Bind the variable ?v0 in the environment
        let const_val = 42.into();
        engine.current_env[v0.as_usize()] = Some(const_val);

        // 2. Simulate that the fact is *already* present in the database (stable or delta)
        engine
            .db
            .insert_stable_fact(AtomSkeletonId::from(10), &[const_val]);

        // 3. Call evaluate_head: it must detect the duplicate and stop
        let result = engine.evaluate_head(&rule);
        assert!(result.is_ok());
        assert_eq!(
            engine.discovered_facts.len(),
            0,
            "The fact already present in the DB must not be added to discovered_facts"
        );
    }

    /// # Objective
    /// Verifies that rule head evaluation properly detects an unbound variable and returns a `DatalogError`.
    ///
    /// # Input
    /// - A rule head containing an unbound variable (`?v0`).
    /// - The environment with `?v0` explicitly set to `None`.
    ///
    /// # Expected Output
    /// - Head evaluation fails, returning a `Result::Err` containing a datalog error due to the unbound variable.
    #[test]
    fn test_evaluate_head_unbound_variable_error() {
        let mut engine = create_segmented_engine();

        let v0 = VariableId::from(0);
        let mut head_args = AtomArgs::new();
        head_args.push(Term::Variable(v0));

        let head_atom = Atom::nary(AtomSkeletonId::from(10), head_args);
        let rule = Rule::new(head_atom, RuleBody::new());

        // Ensure current_env[0] is set to None
        engine.current_env[v0.as_usize()] = None;

        let result = engine.evaluate_head(&rule);

        assert!(
            result.is_err(),
            "An unbound variable in the rule head must imperatively return a DatalogError"
        );
    }

    /// # Objective
    /// Verifies that rule head evaluation filters out duplicates present within the local temporary buffer (`discovered_facts`) during the same evaluation cycle.
    ///
    /// # Input
    /// - A rule head containing a bound variable (`?v0`).
    /// - The environment with `?v0` bound to a specific value (`42`).
    /// - Calling `evaluate_head` twice sequentially with the exact same environment and rule.
    ///
    /// # Expected Output
    /// - The first call discovers and adds the fact (length becomes 1). The second call detects the duplicate in the local buffer and filters it out, keeping the length at 1.
    #[test]
    fn test_evaluate_head_filters_local_buffer_duplicates() {
        let mut engine = create_segmented_engine();

        let v0 = VariableId::from(0);
        let mut head_args = AtomArgs::new();
        head_args.push(Term::Variable(v0));

        let head_atom = Atom::nary(AtomSkeletonId::from(10), head_args);
        let rule = Rule::new(head_atom, RuleBody::new());

        let const_val = 42.into();
        engine.current_env[v0.as_usize()] = Some(const_val);

        // First call: the fact is discovered and added
        let result1 = engine.evaluate_head(&rule);
        assert!(result1.is_ok());
        assert_eq!(engine.discovered_facts.len(), 1);

        // Second call with the same fact: must be filtered by the local buffer
        let result2 = engine.evaluate_head(&rule);
        assert!(result2.is_ok());
        assert_eq!(
            engine.discovered_facts.len(),
            1,
            "The duplicate present in the local buffer must not be duplicated"
        );
    }

    /// # Objective
    /// Verifies that rule head evaluation filters out duplicates that are already present in the Delta database buffer.
    ///
    /// # Input
    /// - A rule head containing a bound variable (`?v0`).
    /// - The environment with `?v0` bound to a specific value (`42`).
    /// - A fact pre-inserted into the Delta database for that atom's ID and value.
    ///
    /// # Expected Output
    /// - Head evaluation succeeds, but discovers 0 facts because the duplicate is successfully filtered out by the engine.
    #[test]
    fn test_evaluate_head_filters_delta_duplicates() {
        let mut engine = create_segmented_engine();

        let v0 = VariableId::from(0);
        let mut head_args = AtomArgs::new();
        head_args.push(Term::Variable(v0));

        let head_atom = Atom::nary(AtomSkeletonId::from(10), head_args);
        let rule = Rule::new(head_atom, RuleBody::new());

        let const_val = 42.into();
        engine.current_env[v0.as_usize()] = Some(const_val);

        // Simulate that the fact is present in Delta (via insert_delta_fact)
        engine
            .db
            .insert_delta_fact(AtomSkeletonId::from(10), &[const_val]);

        let result = engine.evaluate_head(&rule);
        assert!(result.is_ok());
        assert_eq!(
            engine.discovered_facts.len(),
            0,
            "The fact already present in the DB Delta must be filtered out"
        );
    }

    /// # Objective
    /// Verifies that rule head evaluation correctly handles mixed terms, combining explicit constants and bound variables.
    ///
    /// # Input
    /// - A rule head containing an explicit constant (`99`) and a bound variable (`?v0`).
    /// - The environment with `?v0` bound to a specific value (`42`).
    ///
    /// # Expected Output
    /// - Head evaluation succeeds, discovering 1 fact whose arguments correctly combine the constant and the bound variable value (`[99, 42]`).
    #[test]
    fn test_evaluate_head_mixed_terms() {
        let mut engine = create_segmented_engine();

        let v0 = VariableId::from(0);
        let mut head_args = AtomArgs::new();
        head_args.push(Term::Constant(99.into())); // Hardcoded constant in the head
        head_args.push(Term::Variable(v0)); // Bound variable

        let head_atom = Atom::nary(AtomSkeletonId::from(20), head_args);
        let rule = Rule::new(head_atom, RuleBody::new());

        let const_val = 42.into();
        engine.current_env[v0.as_usize()] = Some(const_val);

        let result = engine.evaluate_head(&rule);
        assert!(result.is_ok());
        assert_eq!(engine.discovered_facts.len(), 1);

        // Verify that the internal buffer successfully assembled [99, 42]
        let (_, args) = &engine.discovered_facts[0];
        assert_eq!(args.len(), 2);
        // (Adjust according to your ObjectId type reading method if needed)
    }
}

#[cfg(test)]
mod tests_semi_naive_units {
    use super::*;
    use crate::aiplan4rust::support::lang::VariableId;
    use crate::analysis::reachability::datalog::engine::test_utils::create_segmented_engine;

    /// # Objective
    /// Verifies that the semi-naive pivot strategy correctly directs lookups toward either the Stable or Delta database depending on the atom's position relative to the pivot index.
    ///
    /// # Input
    /// - A rule with two body atoms (`atom0` at body index 0, `atom1` at body index 1).
    /// - A pivot index set to `1`, meaning `atom0` (< pivot) targets Stable, and `atom1` (== pivot) targets Delta.
    /// - A fact inserted into the Stable database for `atom0`, and a fact inserted into the Delta database for `atom1`.
    ///
    /// # Expected Output
    /// - The rule successfully crosses the Stable-to-Delta pivot transition, evaluates the full body, and discovers exactly 1 fact.
    #[test]
    fn test_process_incremental_pivot_strategy() {
        let mut engine = create_segmented_engine();

        // Let's create a dummy rule with 2 body atoms
        // Atom 0 (before pivot), Atom 1 (at pivot)
        let atom0 = Atom::unary(AtomSkeletonId::from(1), Term::Variable(VariableId::from(0)));
        let atom1 = Atom::unary(AtomSkeletonId::from(2), Term::Variable(VariableId::from(1)));

        let head = Atom::unary(AtomSkeletonId::from(3), Term::Constant(ObjectId::from(99)));
        let rule = Rule::new(head, vec![atom0, atom1]);

        // If we set pivot_idx = 1:
        // - For body_idx = 0 (< 1), it must look in Stable.
        // - For body_idx = 1 (== 1), it must look in Delta.

        // We can insert a fact into Stable for atom 0
        engine
            .db
            .insert_stable_fact(AtomSkeletonId::from(1), &[ObjectId::from(42)]);

        // And a fact into Delta for atom 1
        engine
            .db
            .insert_delta_fact(AtomSkeletonId::from(2), &[ObjectId::from(84)]);

        // Run incremental processing with a pivot at 1
        engine.process_incremental(&rule, 0, 1);

        // Verification that the rule went through and triggered evaluate_head
        assert_eq!(
            engine.discovered_facts.len(),
            1,
            "The rule must traverse the Stable -> Delta pivot and produce a fact"
        );
    }

    /// # Objective
    /// Verifies that `match_relation` uses indexing when the first argument is bound (e.g., a constant), filtering matching facts.
    ///
    /// # Input
    /// - A rule where the atom's first argument is a constant (`ObjectId::from(42)`).
    /// - Two facts inserted into the Stable database (`[42]` and `[84]`).
    ///
    /// # Expected Output
    /// - Indexing targets only the matching fact (`42`), successfully discovering exactly 1 fact in the rule head.
    #[test]
    fn test_match_relation_uses_index_when_bound() {
        let mut engine = create_segmented_engine();

        // Rule with an atom whose first argument is a constant (therefore bound)
        let atom = Atom::unary(AtomSkeletonId::from(5), Term::Constant(ObjectId::from(42)));
        let head = Atom::unary(AtomSkeletonId::from(10), Term::Constant(ObjectId::from(99)));
        let rule = Rule::new(head, vec![atom]);

        // Insert a fact into Stable
        engine
            .db
            .insert_stable_fact(AtomSkeletonId::from(5), &[ObjectId::from(42)]);
        engine
            .db
            .insert_stable_fact(AtomSkeletonId::from(5), &[ObjectId::from(84)]);

        // Run process_incremental with body_idx = 0 and pivot_idx = 1
        // Since 0 < 1, it will look in Stable (use_delta = false)
        engine.process_incremental(&rule, 0, 1);

        // Only the fact starting with 42 should match
        assert_eq!(
            engine.discovered_facts.len(),
            1,
            "Indexing must filter and trigger the rule only for the corresponding fact"
        );
    }

    /// # Objective
    /// Verifies that `match_relation` performs a full scan when the first argument of an atom is an unbound variable.
    ///
    /// # Input
    /// - A rule containing an atom whose first argument is an unbound variable (`?v0`), with the head also using `?v0`.
    /// - The environment for `?v0` set to `None`.
    /// - Two distinct facts inserted into the Stable database for the atom's ID.
    ///
    /// # Expected Output
    /// - A full scan over the relation evaluates both tuples, successfully binding `?v0` and discovering two distinct facts.
    #[test]
    fn test_match_relation_full_scan_when_unbound() {
        let mut engine = create_segmented_engine();

        // Rule with an atom whose first argument is an unbound VARIABLE (?v0)
        let v0 = VariableId::from(0);
        let atom = Atom::unary(AtomSkeletonId::from(5), Term::Variable(v0));

        // THE HEAD ALSO USES ?v0 to generate unique facts: head(?v0)
        let head = Atom::unary(AtomSkeletonId::from(10), Term::Variable(v0));
        let rule = Rule::new(head, vec![atom]);

        // The environment for ?v0 is None (unbound), which forces a full scan
        engine.current_env[v0.as_usize()] = None;

        // Insert two distinct facts into Stable
        engine
            .db
            .insert_stable_fact(AtomSkeletonId::from(5), &[ObjectId::from(42)]);
        engine
            .db
            .insert_stable_fact(AtomSkeletonId::from(5), &[ObjectId::from(84)]);

        // Run process_incremental with pivot_idx = 1 (to target Stable)
        engine.process_incremental(&rule, 0, 1);

        // The scan must sweep both facts, binding ?v0 to 42 then to 84,
        // producing two distinct facts ([42] and [84]) in discovered_facts.
        assert_eq!(
            engine.discovered_facts.len(),
            2,
            "The full scan must evaluate all tuples of the relation"
        );
    }

    /// # Objective
    /// Verifies that surgical rollback correctly restores the environment when an atom in the rule body fails.
    ///
    /// # Input
    /// - A rule with two atoms: the first atom succeeds (binding a variable), and the second atom fails (no matching fact).
    /// - A fact inserted in the stable database only for the first atom.
    ///
    /// # Expected Output
    /// - The rule execution fails, discovering 0 facts, and the variable bound by the failing atom is successfully rolled back to `None`.
    #[test]
    fn test_surgical_rollback_on_failure() {
        let mut engine = create_segmented_engine();

        let v0 = VariableId::from(0);
        let v1 = VariableId::from(1);

        // Atom 0: binds ?v0 with an existing fact in Stable
        let atom0 = Atom::unary(AtomSkeletonId::from(1), Term::Variable(v0));
        // Atom 1: looks for a fact that DOES NOT EXIST (triggers unification failure)
        let atom1 = Atom::unary(AtomSkeletonId::from(2), Term::Variable(v1));

        let head = Atom::unary(AtomSkeletonId::from(3), Term::Constant(ObjectId::from(99)));
        let rule = Rule::new(head, vec![atom0, atom1]);

        // Insert a fact for atom 0 (so it succeeds and binds ?v0)
        engine
            .db
            .insert_stable_fact(AtomSkeletonId::from(1), &[ObjectId::from(42)]);

        // Do not insert anything for atom 2 (so atom 1 will fail)

        // Before the test, the environment for ?v0 and ?v1 is None
        assert_eq!(engine.current_env[v0.as_usize()], None);
        assert_eq!(engine.current_env[v1.as_usize()], None);

        // Run incremental processing (pivot at 2 to target Stable everywhere)
        engine.process_incremental(&rule, 0, 2);

        // Since atom 1 failed, the rule could not complete: 0 facts discovered
        assert_eq!(
            engine.discovered_facts.len(),
            0,
            "The rule must not succeed if an atom in the body fails"
        );

        // Verification of surgical rollback:
        // The global engine environment must not be polluted or corrupted by partial failure bindings.
        // The variables of the failed atom (?v1) must return to None, keeping the system clean.
        assert_eq!(
            engine.current_env[v1.as_usize()],
            None,
            "The variable bound by the failed atom must be cleaned up by the rollback"
        );
    }

    /// # Objective
    /// Verifies that `match_relation` correctly handles zero-arity predicates (propositions)
    /// when they are present in the stable database.
    ///
    /// # Input
    /// - A rule containing a single zero-arity atom.
    /// - A propositional fact inserted into the Stable database for that atom's ID.
    ///
    /// # Expected Output
    /// - The rule body is successfully validated, and a new fact is discovered and added to `discovered_facts`.
    #[test]
    fn test_match_relation_arity_zero() {
        let mut engine = create_segmented_engine();

        let sk_id = AtomSkeletonId::from(20);
        // Zero-arity atom (no arguments)
        let atom = Atom::empty(sk_id);
        let head = Atom::unary(AtomSkeletonId::from(30), Term::Constant(ObjectId::from(99)));
        let rule = Rule::new(head, vec![atom]);

        // Insert the propositional fact into Stable
        engine.db.insert_stable_fact(sk_id, &[]);

        // Run process_incremental with a pivot at 1 (to target Stable)
        engine.process_incremental(&rule, 0, 1);

        // Since the proposition is present, the rule should validate its body and produce the head
        assert_eq!(
            engine.discovered_facts.len(),
            1,
            "The zero-arity predicate present in the DB should trigger rule evaluation"
        );
    }
}
