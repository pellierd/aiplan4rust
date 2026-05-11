//! # PDDL Requirement Analysis Module
//!
//! This module provides tools to identify the PDDL/HDDL requirements necessary for a
//! given domain or problem. It distinguishes between requirements **explicitly declared**
//! by the user and those **effectively required** by the features used in the syntax tree.
//!
//! ## Core Architecture
//!
//! The analysis is performed by traversing the `Tree<AstNode>`. Since many PDDL
//! requirements depend on the context (e.g., whether a `(not ...)` appears in a
//! precondition or an effect), the module tracks the "Goal Description" (GD) context
//! during traversal.
//!
//! ### Requirement Detection Strategy
//! 1. **Explicit Extraction**: Parses the `:requirements` block to see what the user claimed.
//! 2. **Automated Detection**: Scans the AST for specific nodes (e.g., `AstKind::Forall`)
//!    and evaluates their context to determine the mandatory requirement.
//! 3. **Type Resolution**: Uses the `SymbolTable` to distinguish between `:numeric-fluents`
//!    and `:object-fluents` based on function return types.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::{AssignOp, CompareOp, Requirement, SymbolId};
use crate::aiplan4rust::semantic::passes::PassContext;
use crate::aiplan4rust::semantic::SemanticError;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::{NodeId, SyntaxContent, Tree};
use crate::SymbolTable;
use std::collections::{HashMap, HashSet};

/// Extracts the explicitly declared semantic requirements from the syntax tree.
///
/// This function locates the `:requirements` definition and collects only the
/// requirements explicitly listed by the user.
///
/// **Note:** This does not compute the transitive closure (implied requirements).
/// To get the full effective set, use [`Requirement::closure`] on the resulting set.
///
/// # Arguments
/// - `syntax_tree`: A reference to the arena-based syntax tree.
///
/// # Returns
/// A `HashSet` containing only the requirements explicitly found in the `RequireDef` node.
///
/// # Errors
/// Returns a [`SemanticError`] if tree traversal fails or nodes are unreachable.
pub fn extract_declared_requirements(
    context: &PassContext,
) -> Result<HashSet<Requirement>, SemanticError> {
    let mut requirements = HashSet::new();
    let syntax_tree = context.syntax_tree();

    // Find the first RequireDef node (the :requirements section)
    let mut requirement_def_node = None;
    for node in syntax_tree.preorder().values() {
        if matches!(node.kind(), AstKind::RequireDef) {
            requirement_def_node = Some(node);
            break;
        }
    }

    if let Some(req_def) = requirement_def_node {
        // Collect all explicit Requirement children
        for child in req_def.children() {
            let node = syntax_tree.try_node(*child)?;
            if matches!(node.kind(), AstKind::Requirement) {
                if let Ok(req) = node.try_requirement() {
                    // We only insert the requirement itself, not its implications
                    requirements.insert(req);
                }
            }
        }
    }

    Ok(requirements)
}

/// Analyzes the AST to detect which PDDL/HDDL features are actually utilized.
///
/// This function performs a deep scan of the syntax tree to automatically identify
/// necessary extensions. It distinguishes between logical contexts (Goal Descriptions)
/// and effect contexts to accurately map features like `:negative-preconditions`
/// versus standard STRIPS deletions.
///
/// ### Context Awareness
/// The function maintains a "logical context" state during traversal:
/// - **Goal Description (GD) Scopes**: Triggered by nodes like `PreconditionDef`, `Goal`, or
///   `Constraints`. Inside these, operators like `not` or `forall` trigger
///   precondition-specific requirements (e.g., `:negative-preconditions`).
/// - **Effect Scopes**: Outside of GD scopes, the same operators might simply
///   be part of basic STRIPS or `:conditional-effects`.
///
/// ### Requirement Mapping
/// - **Typing**: Detected via `:types` definitions or typed variable/parameter declarations.
/// - **Fluents**: Distinguishes between `:numeric-fluents` and `:object-fluents`
///   by resolving function return types in the `SymbolTable`.
/// - **Action Costs**: Identified by specific manipulations of the `total-cost`
///   function that don't involve complex numeric math.
/// - **HTN/Hierarchy**: Triggered by `MethodDef`, `TaskDef`, and ordering constraints.
///
/// # Arguments
/// * `context` - The `PassContext` providing access to the syntax tree and source metadata.
/// * `symbol_table` - The table used to resolve function signatures and type information
///   during requirement inference.
///
/// # Returns
/// * `Ok(HashMap<Requirement, Vec<NodeId>>)` - A mapping where each key is an inferred
///   requirement and the value is a list of AST `NodeId`s that triggered its necessity.
/// * `Err(SemanticError)` - If tree traversal or symbol resolution fails.
///
/// # Default Behavior
/// If no specialized features are detected, the function returns a map containing
/// only `Requirement::Strips` with the root node as its trigger.
pub fn extract_required_requirements(
    context: &PassContext,
    symbol_table: &SymbolTable,
) -> Result<HashMap<Requirement, Vec<NodeId>>, SemanticError> {
    let mut triggers: HashMap<Requirement, Vec<NodeId>> = HashMap::new();
    let syntax_tree = context.syntax_tree();

    // INTERNAL MACRO: Tracks feature usage by simultaneously updating the set of
    // required capabilities and mapping the specific NodeId that triggered the need.
    macro_rules! add_req {
        ($req:expr, $id:expr) => {{
            // Link the specific AST node to this requirement for diagnostics
            triggers.entry($req).or_default().push($id);
        }};
    }

    // The 'gd' (Goal Description) variable tracks the starting depth of a
    // logical context. It is used as a sentinel to determine if the current
    // node is within a scope that requires specific requirement checks.
    let mut gd: Option<usize> = None;

    /// Sentinel for the durative action context.
    ///
    /// This variable tracks the starting depth of a `:duration` constraint block.
    /// It is used to suppress certain requirements that are usually mandatory for
    /// numeric expressions (like raw numbers) but are implicitly covered by
    /// `:durative-actions` when used within a duration specification.
    let mut duration_depth: Option<usize> = None;

    // Perform a preorder traversal (root-to-leaves) of the syntax tree.
    // - 'id': Unique identifier for the node.
    // - 'depth': Vertical position in the tree hierarchy.
    // - 'is_last': Boolean flag indicating if this is the final child of its parent.
    // - 'node': The AST node data currently being visited.
    let mut iter = syntax_tree.preorder();

    while let Some((id, depth, is_last, node)) = iter.next() {
        // --- 1. EXIT GOAL DESCRIPTION (GD) CONTEXT ---
        // If the current traversal depth is less than or equal to the recorded depth
        // of the GD start, we have moved out of the logical scope and must reset the flag.
        if let Some(d) = gd {
            if depth <= d {
                gd = None;
            }
        }

        // Exit the Duration context if the current traversal depth is less than or equal
        // to the recorded duration start depth.
        if let Some(d) = duration_depth {
            if depth <= d {
                duration_depth = None;
            }
        }

        // --- 2. WHEN-CLAUSE REFINEMENT (Transition from Condition to Effect) ---
        // A 'When' node contains a condition (GD) followed by an effect (non-GD).
        // If we are at the last child of a 'When' node, we must exit the GD context
        // to ensure the next sibling or subsequent nodes are treated as effects.
        if gd.is_some() && is_last {
            let node = syntax_tree.try_node(id)?;
            if let Some(parent_id) = node.parent() {
                let parent_node = syntax_tree.try_node(parent_id)?;
                if parent_node.kind() == AstKind::When {
                    // Terminate GD context: the next part of the 'When' is an effect.
                    gd = None;
                }
            }
        }

        // --- 3. ENTER LOGICAL CONTEXT ---
        // If not already in a GD context, check if the current node starts a block
        // that requires logical constraint validation (Preconditions, Goals, etc.).
        // Note: 'EffectDef' is excluded to keep the context 'None' during effect processing.
        if gd.is_none() {
            match node.kind() {
                AstKind::PreconditionDef
                | AstKind::Goal
                | AstKind::MethodPreconditionDef
                | AstKind::When
                | AstKind::Imply
                | AstKind::Comparison
                | AstKind::DerivedDef
                | AstKind::Preference
                | AstKind::PrefName
                | AstKind::Always
                | AstKind::Sometime
                | AstKind::Within
                | AstKind::AtMostOnce
                | AstKind::SometimeAfter
                | AstKind::SometimeBefore
                | AstKind::AlwaysWithin
                | AstKind::HoldDuring
                | AstKind::HoldAfter
                | AstKind::Constraints
                | AstKind::TaskNetworkDef
                | AstKind::TaskLogicalConstraintDef => {
                    // Start tracking the GD context starting from this specific depth.
                    gd = Some(depth);
                }
                _ => {}
            }
        }

        // --- 4. ENTER DURATION CONTEXT ---
        // We now have a dedicated DurationConstraint node from the parser.
        // When entering this node, we set the sentinel to handle numeric
        // literals differently (PDDL 2.1 exception for :durative-actions).
        if duration_depth.is_none() && node.kind() == AstKind::DurationConstraint {
            duration_depth = Some(depth);
        }

        match node.kind() {
            // --- :typing ---
            // This requirement is triggered by any mention of a type system.
            // We capture it at the definition level (:types) and at the usage level
            // (type declarations for constants or variables).
            AstKind::TypesDef | AstKind::Type | AstKind::PrimitiveType => {
                // Record the requirement and the NodeId where the type usage was detected.
                add_req!(Requirement::Typing, id);
            }

            // --- :fluents (:numeric-fluents and :object-fluents) ---
            // This block processes the (:functions ...) definition. PDDL distinguishes
            // between numeric functions and object-returning functions.
            AstKind::FunctionsDef => {
                // Si on a déjà les deux, on peut sauter l'analyse de ce bloc pour gagner du temps
                if triggers.contains_key(&Requirement::NumericFluents)
                    && triggers.contains_key(&Requirement::ObjectFluents)
                {
                    continue;
                }

                for child_id in node.children() {
                    let child_node = syntax_tree.try_node(*child_id)?;

                    // On récupère la liste des TypedItem (on traverse la TypedList si elle existe)
                    let typed_items = if child_node.kind() == AstKind::TypedList {
                        child_node.children()
                    } else {
                        std::slice::from_ref(child_id)
                    };

                    for typed_item_id in typed_items {
                        // 1. On descend au Skeleton
                        let item_node = syntax_tree.try_node(*typed_item_id).ok();
                        let skel_id = item_node.and_then(|n| n.try_child(0).ok());

                        // 2. On descend au Symbole (ex: 'slew_time', NodeId 118 dans tes logs)
                        let sym_node_id = skel_id
                            .and_then(|id| syntax_tree.try_node(id).ok())
                            .and_then(|n| n.try_child(0).ok());

                        if let Some(s_node_id) = sym_node_id {
                            let sym_node = syntax_tree.try_node(s_node_id)?;

                            if let Ok(sym_id) = sym_node.try_ident() {
                                // --- CORRECTION ICI ---
                                // On ne résout pas (recherche d'usage), on récupère la déclaration directe
                                // car nous sommes au moment de la définition dans l'AST.
                                let decl = symbol_table.try_get_declaration(s_node_id)?;
                                if let Some(ty) = decl.ty() {
                                    // Si le type est explicitement 'number'
                                    if ty.is_number() {
                                        if !triggers.contains_key(&Requirement::NumericFluents) {
                                            add_req!(Requirement::NumericFluents, *typed_item_id);
                                        }
                                    } else {
                                        // Si c'est un autre type (Object Fluent)
                                        if !triggers.contains_key(&Requirement::ObjectFluents) {
                                            add_req!(Requirement::ObjectFluents, *typed_item_id);
                                        }
                                    }
                                } else {
                                    // En PDDL, une fonction sans type est Numeric par défaut
                                    if !triggers.contains_key(&Requirement::NumericFluents) {
                                        add_req!(Requirement::NumericFluents, *typed_item_id);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            AstKind::Metric => {
                // A metric usually has one child representing the expression to be optimized.
                let expr_id = node.try_child(0)?;
                let expr_node = syntax_tree.try_node(expr_id)?;

                match expr_node.kind() {
                    // Case 1: Simple function call, e.g., (minimize (total-cost))
                    AstKind::Function => {
                        if let Ok(sym) = self::get_function_symbol(expr_id, syntax_tree) {
                            if sym == SymbolInterner::TOTAL_COST_SYMBOL_ID {
                                // Using 'total-cost' specifically triggers :action-costs.
                                add_req!(Requirement::ActionCosts, id)
                            } else {
                                // Any other function used in a metric triggers :numeric-fluents.
                                add_req!(Requirement::NumericFluents, id);
                            }
                        }
                    }

                    AstKind::Arithmetic => {
                        // Recursive analysis of the arithmetic expression using a preorder iterator.
                        // We check if the expression remains within the strict scope of :action-costs.
                        let is_pure_action_cost_combination = syntax_tree
                            .preorder_from(expr_id)
                            .ids() // Start from the expression, not the whole metric
                            .all(|(c_id, c_node)| {
                                match c_node.kind() {
                                    // 1. For functions, verify if they are specifically 'total-cost'
                                    AstKind::Function => {
                                        self::get_function_symbol(c_id, syntax_tree)
                                            .map(|sym| sym == SymbolInterner::TOTAL_COST_SYMBOL_ID)
                                            .unwrap_or(false)
                                    }

                                    // 3. Explicitly allow elements permitted in a linear cost combination.
                                    // Standard operators (+, -, *, /) are accepted by debug via the '_' arm.
                                    AstKind::Number
                                    | AstKind::TotalTime
                                    | AstKind::Arithmetic
                                    | AstKind::Variable
                                    | AstKind::Object => true,

                                    // 4. Accept structural nodes (parentheses, operators, etc.)
                                    _ => true,
                                }
                            });

                        if is_pure_action_cost_combination {
                            // The expression is a linear combination of total-cost, total-time, and constants.
                            // This categorizes the requirement as :action-costs.
                            add_req!(Requirement::ActionCosts, id);

                            // Note: If a 'total-time' node was encountered, :durative-actions
                            // will be triggered by its own dedicated block in the main match.
                        } else {
                            // Presence of user-defined functions (e.g., fuel-level) or complex
                            // structures requires full :numeric-fluents.
                            add_req!(Requirement::NumericFluents, id);
                        }
                    }

                    // Case 3: Optimizing total-time directly.
                    AstKind::TotalTime => {
                        // total-time alone does not activate :numeric-fluents;
                        // it is intrinsically linked to :durative-actions.
                        add_req!(Requirement::DurativeActions, id);
                    }

                    // Default: Any other metric expression type defaults to :numeric-fluents.
                    _ => {
                        add_req!(Requirement::NumericFluents, id);
                    }
                }
            }

            // --- :numeric-fluents ---
            // This section handles the requirements for numeric expressions.
            // In PDDL, using numbers or math typically requires :numeric-fluents,
            // but there are structural exceptions for durative actions.
            AstKind::Number => {
                // PDDL 2.1+ Exception: Raw numeric literals (e.g., (= ?duration 2))
                // used within a duration constraint are implicitly allowed by
                // :durative-actions and do not require :numeric-fluents.
                if duration_depth.is_none() {
                    // Outside of a duration context, any number triggers :numeric-fluents.
                    add_req!(Requirement::NumericFluents, id);
                }
            }

            AstKind::Arithmetic => {
                // Arithmetic operators (+, -, *, /) represent advanced numeric
                // capabilities. Unlike raw numbers, these ALWAYS trigger
                // :numeric-fluents, even when used to calculate a duration
                // (e.g., (= ?duration (* 2 ?t))).
                add_req!(Requirement::NumericFluents, id);
            }

            // --- :durative-actions ---
            // This requirement is triggered by any construct related to temporal actions.
            // This includes the action definitions themselves, temporal qualifiers (at start,
            // at end, over all), and the specific bodies of durative actions.
            AstKind::DurativeActionDef
            | AstKind::DASymbol
            | AstKind::DADefBody
            | AstKind::AtStart
            | AstKind::AtEnd
            | AstKind::Overall => {
                // Record the need for durative action support based on the detected
                // temporal syntax or keyword.
                add_req!(Requirement::DurativeActions, id);
            }

            // --- :hierarchy / :htn ---
            // This requirement is triggered by any construct related to Hierarchical
            // Task Networks (HTN). This includes task definitions, methods used to
            // decompose them, and the specific constraints (ordering or logical)
            // that govern how subtasks are executed.
            AstKind::InitialTaskNetwork
            | AstKind::MethodDef
            | AstKind::MethodDefBody
            | AstKind::TaskDef
            | AstKind::Task
            | AstKind::TaskSymbol
            | AstKind::TaskLabel
            | AstKind::TaskLogicalConstraintDef
            | AstKind::TaskOrderingConstraintDef
            | AstKind::LabeledTask
            | AstKind::OrderedSubtaskDef
            | AstKind::PartiallyOrderedSubtaskDef
            | AstKind::TaskOrderingConstraint
            | AstKind::TaskNetworkDef
            | AstKind::MethodSymbol => {
                // Record the requirement for Hierarchy support based on the
                // detected HTN-specific syntax or keyword.
                add_req!(Requirement::Hierarchy, id);
            }

            // --- :method-preconditions ---
            // This block handles preconditions specific to HTN methods.
            // Using a method precondition implies both the general hierarchical
            // structure and the specific capability to evaluate conditions for methods.
            AstKind::MethodPreconditionDef => {
                // Methods are inherently part of hierarchical planning.
                add_req!(Requirement::Hierarchy, id);
                // Explicitly record the requirement for preconditions within methods.
                add_req!(Requirement::MethodPreconditions, id);
            }

            // --- :derived-predicates ---
            // This requirement is triggered by the definition of derived predicates (axioms).
            // Derived predicates are symbols whose truth value is calculated from a formula
            // rather than being directly modified by action effects.
            AstKind::DerivedDef => {
                // Record the requirement for derived predicates, linking it to the
                // specific definition node in the AST.
                add_req!(Requirement::DerivedPredicates, id);
            }

            // --- :negative-preconditions ---
            // This requirement is triggered when a negation ('not') is used within a
            // Goal Description context (preconditions, goals, or constraints).
            AstKind::Not => {
                // We only trigger this if we are inside a logical context (gd is some).
                // In PDDL, simple negations in effects are part of :strips, but
                // negations in preconditions require :negative-preconditions.
                if gd.is_some() {
                    // Record the requirement and the NodeId of the negation.
                    add_req!(Requirement::NegativePreconditions, id);
                }
            }

            // --- :disjunctive-preconditions & :negative-preconditions ---
            // The 'imply' construct (A -> B) is logically equivalent to (not A or B).
            // Therefore, it fundamentally requires both disjunctive and negative
            // precondition support to be evaluated by a planner.
            AstKind::Imply => {
                // Record the requirement for disjunction (OR logic).
                add_req!(Requirement::DisjunctivePreconditions, id);
                // Record the requirement for negation (NOT logic).
                add_req!(Requirement::NegativePreconditions, id);
            }

            // --- :disjunctive-preconditions ---
            // The 'or' construct allows for multiple alternative conditions to satisfy
            // a goal or precondition, which requires the :disjunctive-preconditions capability.
            AstKind::Or => {
                // Record the requirement for disjunction, linking it to the specific
                // 'or' node in the AST.
                add_req!(Requirement::DisjunctivePreconditions, id);
            }

            // --- :universal-preconditions or :conditional-effects ---
            // The 'forall' quantifier has two different meanings depending on its context.
            AstKind::Forall => {
                if gd.is_some() {
                    // If 'forall' appears within a Goal Description (preconditions or goals),
                    // it triggers the :universal-preconditions requirement.
                    add_req!(Requirement::UniversalPreconditions, id);
                } else {
                    // If 'forall' appears outside of a GD context (typically in an effect),
                    // it represents a quantified effect, triggering :conditional-effects.
                    add_req!(Requirement::ConditionalEffects, id);
                }
            }

            // --- :existential-preconditions ---
            // The 'exists' quantifier is used to check if at least one object
            // satisfies a condition within a Goal Description (preconditions or goals).
            AstKind::Exists => {
                // Existential quantification is only valid/required in a logical
                // context. If found outside (which is non-standard for effects),
                // it is ignored by this specific requirement tracker.
                if gd.is_some() {
                    // Record the requirement for existential preconditions,
                    // linking it to the 'exists' node in the AST.
                    add_req!(Requirement::ExistentialPreconditions, id);
                }
            }

            // --- :preferences ---
            // This requirement is triggered by the use of soft constraints (preferences).
            // Preferences allow the planner to explore plans that satisfy "desirable"
            // conditions without making them mandatory for success.
            AstKind::Preference | AstKind::PrefName => {
                // Record the requirement for preferences, covering both the anonymous
                // preference definitions and named preferences used in complex metrics.
                add_req!(Requirement::Preferences, id);
            }

            // --- :preferences ---
            // The 'is-violated' function is used in the metric section to count
            // how many times a named preference was not satisfied.
            AstKind::IsViolated => {
                // We maintain a strict mapping: 'is-violated' specifically concerns
                // preferences. If the user performs arithmetic with this value (e.g.,
                // multiplying by a penalty), the arithmetic operators themselves
                // will trigger :numeric-fluents in their own logic blocks.
                add_req!(Requirement::Preferences, id);
            }

            // --- :conditional-effects ---
            // The 'when' construct defines an effect that only occurs if a specific
            // condition is met at the time of action execution.
            AstKind::When => {
                // Record the requirement for conditional effects. This allows planners
                // to handle actions where the outcome depends on the current state.
                add_req!(Requirement::ConditionalEffects, id);
            }

            // --- :constraints ---
            // This requirement is triggered by state trajectory constraints.
            // These operators define conditions that must hold true over the entire
            // execution of the plan (e.g., "always", "sometime before"), rather than
            // just at the start or end of specific actions.
            AstKind::Always
            | AstKind::Sometime
            | AstKind::Within
            | AstKind::AtMostOnce
            | AstKind::SometimeAfter
            | AstKind::SometimeBefore
            | AstKind::AlwaysWithin
            | AstKind::HoldDuring
            | AstKind::HoldAfter
            | AstKind::Constraints => {
                // Record the requirement for constraints, linking it to the specific
                // temporal logic node in the AST.
                add_req!(Requirement::Constraints, id);
            }

            AstKind::Assignment => {
                // Optimization: If all related requirements are already detected, skip the node.
                if triggers.contains_key(&Requirement::NumericFluents)
                    && triggers.contains_key(&Requirement::ObjectFluents)
                    && triggers.contains_key(&Requirement::ContinuousEffects)
                {
                    continue;
                }

                let op = node.content().try_assign_op()?;
                let l_id = node.try_child(0)?;
                let r_id = node.try_child(1)?;

                // --- 1. Detect Continuous Effects (:continuous-effects) ---
                // According to PDDL BNF: <f-exp-t> ::= (* <f-exp> #t) | #t | ...
                // We check if the R-Value contains the reserved variable #t (continuous time).
                let involves_continuous_time =
                    syntax_tree.preorder_from(r_id).values().any(|c_node| {
                        c_node.kind() == AstKind::Variable
                            && c_node.try_ident().ok()
                                == Some(SymbolInterner::CONTINUOUS_VARIABLE_SYMBOL_ID)
                    });

                if involves_continuous_time {
                    add_req!(Requirement::ContinuousEffects, id);
                    add_req!(Requirement::NumericFluents, id);
                }

                // --- 2. Action-Costs vs Numeric-Fluents Logic ---
                let target_is_total_cost = self::get_function_symbol(l_id, syntax_tree).ok()
                    == Some(SymbolInterner::TOTAL_COST_SYMBOL_ID);

                if target_is_total_cost {
                    // Manipulating 'total-cost' triggers :action-costs.
                    add_req!(Requirement::ActionCosts, id);

                    // If it's not a simple (increase total-cost ...) or an initial assignment,
                    // or if #t is involved (costs cannot be continuous without full fluents),
                    // we upgrade the requirement to :numeric-fluents.
                    if !matches!(op, AssignOp::Increase | AssignOp::Assign)
                        || involves_continuous_time
                    {
                        add_req!(Requirement::NumericFluents, id);
                    }
                } else {
                    // If the target is not 'total-cost', determine if it's an object or a numeric fluent.
                    match self::get_term_requirement(l_id, syntax_tree, symbol_table)? {
                        Some(Requirement::ObjectFluents) => {
                            add_req!(Requirement::ObjectFluents, id);
                        }
                        _ => {
                            // Default to numeric fluents for other assignment targets.
                            add_req!(Requirement::NumericFluents, id);
                        }
                    }
                }

                // --- 3. R-Value (Source) Analysis ---
                // If NumericFluents isn't active yet, check if the source expression necessitates it.
                if !triggers.contains_key(&Requirement::NumericFluents) {
                    if let Some(Requirement::NumericFluents) =
                        self::get_term_requirement(r_id, syntax_tree, symbol_table)?
                    {
                        // Only trigger if we aren't in a pure action-cost total-cost update.
                        if !target_is_total_cost {
                            add_req!(Requirement::NumericFluents, id);
                        }
                    }
                }
            }

            AstKind::Comparison => {
                // Early exit: if all potential requirements from this node are already found, skip.
                if triggers.contains_key(&Requirement::NumericFluents)
                    && triggers.contains_key(&Requirement::ObjectFluents)
                    && triggers.contains_key(&Requirement::DurationInequalities)
                {
                    continue;
                }

                let op = node.content().try_compare_op()?;
                let l_id = node.try_child(0)?;
                let r_id = node.try_child(1)?;

                // Check if the comparison involves the '?duration' variable (temporal context).
                let is_l_dur = is_duration_variable(l_id, syntax_tree);
                let is_r_dur = is_duration_variable(r_id, syntax_tree);
                let is_dur_context = is_l_dur || is_r_dur;

                // --- 1. Specific Handling for :durative-actions & :duration-inequalities ---
                if is_dur_context {
                    // Inequality operators (>=, <=) on ?duration trigger :duration-inequalities.
                    if op != CompareOp::Equal {
                        add_req!(Requirement::DurationInequalities, id);
                    }
                    // NOTE: Simple equality (= ?duration 2) does NOT trigger :numeric-fluents
                    // as it is inherently covered by the :durative-actions requirement.
                }

                // --- 2. Term Type Analysis (Numeric vs Object) ---
                let req_l = self::get_term_requirement(l_id, syntax_tree, symbol_table)?;

                // We only add NumericFluents if we are NOT in a simple duration constraint
                // (e.g., ?duration compared to a constant).
                if let Some(Requirement::NumericFluents) = req_l {
                    if !is_dur_context {
                        add_req!(Requirement::NumericFluents, id);
                    }
                } else {
                    let req_r = self::get_term_requirement(r_id, syntax_tree, symbol_table)?;

                    match (req_l, req_r) {
                        // If the right side is numeric but we are not in a duration context.
                        (_, Some(Requirement::NumericFluents)) if !is_dur_context => {
                            add_req!(Requirement::NumericFluents, id);
                        }
                        // Special Case: if we are in a duration context, only trigger NumericFluents
                        // if the other side is a complex numeric expression (e.g., (* 2 (fuel))).
                        (None, Some(Requirement::NumericFluents)) if is_dur_context => {
                            add_req!(Requirement::NumericFluents, id);
                        }

                        // Rule 2: Handling object comparisons (requires :object-fluents).
                        (Some(Requirement::ObjectFluents), _)
                        | (_, Some(Requirement::ObjectFluents)) => {
                            add_req!(Requirement::ObjectFluents, id);
                        }
                        // Rule 3: Simple equality (=) between standard symbols.
                        _ if op == CompareOp::Equal => {
                            // Standard equality only triggers :equality if not involving ?duration.
                            if !is_dur_context {
                                add_req!(Requirement::Equality, id);
                            }
                        }
                        // Rule 4: Inequality operators on numbers outside of simple ?duration constraints.
                        _ => {
                            if !is_dur_context {
                                add_req!(Requirement::NumericFluents, id);
                            }
                        }
                    }
                }
            }

            AstKind::Init => {
                continue;
            }

            // --- :timed-initial-literals ---
            // This requirement is triggered by the presence of facts that become true
            // at specific time points in the initial state (e.g., (at 10 (sun-rises))).
            AstKind::TimedInitialLiteral => {
                // Record the requirement for Timed Initial Literals.
                // Note: While this often appears in domains with Durative Actions,
                // it is technically a distinct requirement for the problem file.
                add_req!(Requirement::TimedInitialLiterals, id);
            }

            AstKind::AtomicFormula | AstKind::AtomicFormulaSkeleton | AstKind::ObjectsDef => {
                iter.skip_subtree();
            }

            // These nodes do not imply a requirement
            AstKind::Domain
            | AstKind::DomainName
            | AstKind::ProblemName
            | AstKind::PredicateSymbol
            | AstKind::ActionSymbol
            | AstKind::RequireDef
            | AstKind::ConstantsDef
            | AstKind::TypedList
            | AstKind::TypedItem
            | AstKind::TypedItemElements
            | AstKind::Problem
            | AstKind::PredicatesDef
            | AstKind::ActionDef
            | AstKind::ParametersDef
            | AstKind::ActionDefBody
            | AstKind::PreconditionDef
            | AstKind::EffectDef
            | AstKind::And
            | AstKind::Object
            | AstKind::Variable
            | AstKind::Goal
            | AstKind::Length
            | AstKind::Serial
            | AstKind::Error
            | AstKind::Requirement
            | AstKind::Parallel
            | AstKind::TotalTime
            | AstKind::Function
            | AstKind::FunctionSymbol
            | AstKind::AtomicFunctionSkeleton
            | AstKind::DurationConstraint => {
                // No requirement associated
            }
        }
    }

    Ok(triggers)
}

/// Attempts to determine the PDDL requirement implied by a specific term node.
///
/// This function inspects variables, numbers, arithmetic expressions, and function
/// applications to identify whether they necessitate `:numeric-fluents`,
/// `:object-fluents`, or `:continuous-effects`.
///
/// ### Logic Flow
///
/// 1.  **Reserved Variables**:
///     * `#t`: Specifically triggers `:continuous-effects`.
///     * `?duration`: Triggers `:numeric-fluents` as it represents a numeric quantity.
/// 2.  **Arithmetic & Numbers**:
///     * Any literal number or arithmetic operator (`+`, `-`, `*`, `/`) defaults to
///         requiring `:numeric-fluents`.
/// 3.  **Functions**:
///     * If the function is `total-cost`, it returns `None`. This allows the calling
///         logic to decide if it falls under `:action-costs` or the broader `:numeric-fluents`.
///     * For other functions, it resolves the declaration in the `SymbolTable`.
///         If the return type is numeric, it triggers `:numeric-fluents`; otherwise,
///         it triggers `:object-fluents`.
///
/// ### Returns
///
/// * `Ok(Some(Requirement))`: The specific requirement necessitated by this term.
/// * `Ok(None)`: No specific requirement identified (or deferred to parent logic).
/// * `Err(SemanticError)`: If node retrieval or symbol resolution fails.
///
/// ### Errors
///
/// Returns a `SemanticError` if the `id` does not exist in the `tree` or if
/// identifier access fails for expected nodes.
fn get_term_requirement(
    id: NodeId,
    tree: &Tree<AstNode>,
    table: &SymbolTable,
) -> Result<Option<Requirement>, SemanticError> {
    let node = tree.try_node(id)?;

    // 1. Handle Reserved Variables (?duration and #t)
    if node.kind() == AstKind::Variable {
        if let Ok(sym) = node.try_ident() {
            if sym == SymbolInterner::CONTINUOUS_VARIABLE_SYMBOL_ID {
                // #t implies continuous effects over time.
                return Ok(Some(Requirement::ContinuousEffects));
            }
            if sym == SymbolInterner::DURATION_VARIABLE_SYMBOL_ID {
                // ?duration is numeric by nature (often used in duration constraints).
                return Ok(Some(Requirement::NumericFluents));
            }
        }
    }

    // 2. Arithmetic and Numeric Literals
    if matches!(node.kind(), AstKind::Arithmetic | AstKind::Number) {
        // Any numeric expression requires the numeric-fluents extension.
        return Ok(Some(Requirement::NumericFluents));
    }

    // 3. Functions (Fluent lookups)
    if node.kind() == AstKind::Function {
        // Extraction directe : on sait que l'arbre est valide à ce stade
        let functor_node_id = node.try_child(0)?;
        let functor_node = tree.try_node(functor_node_id)?;
        let functor = functor_node.try_ident()?;

        // Cas particulier : total-cost (Action Costs)
        if functor == SymbolInterner::TOTAL_COST_SYMBOL_ID {
            return Ok(None);
        }

        // Vissage direct O(1) pour tous les autres fluents
        // On utilise functor_node_id pour la résolution précise dans la table
        let decl = table.resolve_usage(functor_node_id)?;
        if let Some(ty) = decl.ty() {
            return Ok(Some(if ty.is_number() {
                Requirement::NumericFluents
            } else {
                Requirement::ObjectFluents
            }));
        }

        // Fallback par défaut
        return Ok(Some(Requirement::NumericFluents));
    }

    Ok(None)
}

/// Extracts a [`SymbolId`] from a node, even if it is wrapped within a function application.
///
/// This helper handles the structural difference between a raw identifier node and
/// a `Function` node (where the identifier is the first child).
///
/// ### Logical Steps
/// 1. If the node is an `AstKind::Function`, it descends to the first child to find the functor.
/// 2. Otherwise, it attempts to extract the identifier directly from the provided node.
///
/// ### Errors
/// Returns a [`SemanticError`] if:
/// * The `NodeId` does not exist in the tree.
/// * A `Function` node lacks a child.
/// * The resulting node is not an identifier/symbol.
fn get_function_symbol(id: NodeId, tree: &Tree<AstNode>) -> Result<SymbolId, SemanticError> {
    let node = tree.try_node(id)?;
    let functor = if node.kind() == AstKind::Function {
        let first_child = node.try_child(0)?;
        tree.try_node(first_child)?.try_ident()?
    } else {
        node.try_ident()?
    };
    Ok(functor)
}

/// Checks if a given node represents the reserved PDDL variable `?duration`.
///
/// This is primarily used during duration constraint analysis to determine if
/// an inequality should trigger the `:duration-inequalities` requirement.
///
/// ### Parameters
/// * `id`: The [`NodeId`] of the term to check.
/// * `tree`: The AST containing the node.
///
/// ### Returns
/// * `true` if the node is a variable and its symbol matches [`SymbolInterner::DURATION_VARIABLE_SYMBOL_ID`].
/// * `false` otherwise (including cases where the node does not exist or is not a variable).
fn is_duration_variable(id: NodeId, tree: &Tree<AstNode>) -> bool {
    if let Ok(node) = tree.try_node(id) {
        if node.kind() == AstKind::Variable {
            if let Ok(sym) = node.try_ident() {
                // Compare against the fixed ID for the "?duration" symbol.
                return sym == SymbolInterner::DURATION_VARIABLE_SYMBOL_ID;
            }
        }
    }
    false
}
