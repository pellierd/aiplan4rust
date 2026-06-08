//! This module provides structural well-formedness checks on an Abstract Syntax Tree (AST).
//!
//! The checks performed ensure that the AST is structurally sound:
//! - Nodes have the expected kind and number of children.
//! - Node content and child node types match the expected format.
//! - Basic tree invariants such as presence of a root node are respected.
//!
//! **Important:**
//! This module does *not* perform any semantic validation.
//! It only guarantees that the AST structure follows basic syntactic rules and node composition constraints.
//! Semantic correctness, type_checker checking, and domain-specific validations are out of scope here and must be
//! handled in separate validation phases.
//!
//! # Example
//! ```ignore
//! let ast = parse_pddl_domain("...");
//! check_structural_well_formedness(&ast)?;
//! ```

use crate::aiplan4rust::syntax::ast::arena::ArenaNode;
use crate::aiplan4rust::syntax::ast::{Ast, AstKind, AstNode};
use crate::aiplan4rust::syntax::validation::checks::METRIC_EXPRESSION;
use crate::aiplan4rust::syntax::validation::{checks, WellFormedError};

/// Checks whether the AST is structurally well-formed.
///
/// This function verifies that:
/// - the underlying arena forms a valid tree (no cycles),
/// - the AST structure is valid starting from the root node.
///
/// # Arguments
/// * `ast` - The AST to check.
///
/// # Returns
/// * `true` if the AST is structurally well-formed.
/// * `false` otherwise.
///
/// # Note
/// This function only checks structural correctness, not semantic validity.
pub fn is_well_formed(ast: &Ast) -> bool {
    check_well_formed(ast).is_ok()
}

/// Checks that the AST is structurally well-formed starting from its root node.
///
/// This validation ensures that the AST:
/// 1. Forms a valid tree (no cycles detected).
/// 2. Contains a non-empty root node.
/// 3. Starts with either a `Domain` or a `Problem` kind, as required by PDDL.
///
/// This is a structural sanity check typically run in debug mode to ensure
/// subsequent compiler finalization (like normalization) can safely use direct access
/// methods (e.g., `.unwrap()`) on expected nodes.
///
/// # Arguments
///
/// * `ast` - The AST to validate.
///
/// # Returns
///
/// * `Ok(())` if the AST root and global structure are valid.
/// * `Err(ValidationError)` if the tree is cyclic, empty, or has an invalid root.
///
/// # Note
///
/// This function only checks basic structural requirements. It does not perform
/// full semantic analysis or check PDDL logic.
pub fn check_well_formed(ast: &Ast) -> Result<(), WellFormedError> {
    let arena = ast.syntax_tree();

    // 1. Basic structural integrity: Must be a valid tree (no cycles)
    if !arena.is_tree() {
        return Err(WellFormedError::cycle_detected());
    }

    // 2. Root validation
    let root = match arena.root_node() {
        Some(root) => root,
        // An empty AST is NOT a valid PDDL file.
        // This prevents the pipeline from proceeding with "nothing".
        None => return Err(WellFormedError::missing_root()),
    };

    // 3. Type validation: Must be a Domain or a Problem
    let root_kind = root.kind();
    if root_kind != AstKind::Domain && root_kind != AstKind::Problem {
        return Err(WellFormedError::invalid_root(root_kind));
    }

    // 4. Recursive check for children
    check_well_formed_from(root, ast)
}

/// Recursively checks that the given AST node and all its descendants are structurally well-formed.
///
/// This function validates the node’s kind and delegates to the appropriate
/// specialized check function for each specific AST node kind.
/// It also recursively validates all child nodes.
///
/// # Arguments
///
/// * `node` - The AST node to validate.
/// * `ast` - Reference to the full AST containing all nodes.
///
/// # Returns
///
/// * `Ok(())` if the node and all descendants are structurally valid.
/// * `Err(WellFormedError)` if any node fails its structural validation.
///
/// # Behavior
///
/// The function matches on the node’s kind and calls the corresponding
/// `syntax::checks::check_*` function for detailed validation.
/// After validating the current node, it recursively checks each child node.
///
/// Note: This function focuses on structural correctness and does not
/// perform semantic validation.
///
/// # Panics
///
/// The function does not panic but returns errors wrapped in `WellFormedError`
/// if structural violations are detected.
///
/// # Examples
///
/// ```ignore
/// check_well_formed_from(root_node, &ast)?;
/// ```
fn check_well_formed_from(node: &AstNode, ast: &Ast) -> Result<(), WellFormedError> {
    check_well_formed_node(node, ast)?;
    let children_ids = node.children();
    for child_id in children_ids {
        let child_node = checks::get_node(ast, node, child_id.as_usize())?;
        check_well_formed_from(child_node, ast)?;
    }

    Ok(())
}
/// Checks that the given AST node is well-formed according to its kind.
///
/// This function performs kind-specific validations on the node, delegating
/// to specialized checks depending on the `AstKind` of the node.
///
/// # Arguments
/// * `node` - The AST node to validate.
/// * `ast` - Reference to the entire AST, used when context or additional data is needed for validation.
///
/// # Returns
/// * `Ok(())` if the node logic all structural checks for its kind.
/// * `Err(WellFormedError)` if the node violates expected structure or semantic rules.
///
/// # Notes
/// - This function only validates a single node, not its children recursively.
/// - Recursive validation should be handled by a separate function that
///   calls `check_well_formed_node` on this node and its descendants.
/// - The function assumes that the AST node kind is known and handled;
///   if a node kind is not matched, the function should return an error.
///
/// # Examples
/// ```
/// let result = check_well_formed_node(&node, &ast);
/// match result {
///     Ok(()) => println!("Node is well-formed."),
///     Err(e) => println!("Node validation failed: {:?}", e),
/// }
/// ```
pub fn check_well_formed_node(node: &AstNode, ast: &Ast) -> Result<(), WellFormedError> {
    match node.kind() {
        AstKind::Object
        | AstKind::Variable
        | AstKind::FunctionSymbol
        | AstKind::PrimitiveType
        | AstKind::DomainName
        | AstKind::ProblemName
        | AstKind::PredicateSymbol
        | AstKind::ActionSymbol
        | AstKind::DASymbol
        | AstKind::MethodSymbol
        | AstKind::TaskSymbol
        | AstKind::PrefName
        | AstKind::TaskLabel => checks::check_symbol(node),
        AstKind::Number => checks::check_number(node),
        AstKind::Requirement => checks::check_requirement(node),
        AstKind::Error => checks::throw_invalid(node),
        AstKind::RequireDef => checks::check_require_def(ast, node),
        AstKind::Type => checks::check_type(ast, node),
        AstKind::TypesDef => checks::check_types_def(ast, node),
        AstKind::TypedList => checks::check_typed_list(ast, node),
        AstKind::TypedItem => checks::check_typed_item(ast, node),
        AstKind::TypedItemElements => checks::check_typed_item_elements(ast, node),
        AstKind::ConstantsDef | AstKind::ObjectsDef => checks::check_constants_def(ast, node),
        AstKind::PredicatesDef => checks::check_predicates_def(ast, node),
        AstKind::AtomicFormulaSkeleton => checks::check_atomic_formula_skeleton(ast, node),
        AstKind::FunctionsDef => checks::check_functions_def(ast, node),
        AstKind::AtomicFunctionSkeleton => checks::check_atomic_function_skeleton(ast, node),
        AstKind::ActionDef => checks::check_action_def(ast, node),
        AstKind::ActionDefBody => checks::check_action_def_body(ast, node),
        AstKind::MethodDef => checks::check_method_def(ast, node),
        AstKind::ParametersDef => checks::check_parameters_def(ast, node),
        AstKind::MethodDefBody => checks::check_method_def_body(ast, node),
        AstKind::Task => checks::check_task(ast, node),
        AstKind::PreconditionDef | AstKind::MethodPreconditionDef => {
            checks::check_precondition_def(ast, node)
        }
        AstKind::EffectDef => checks::check_effect_def(ast, node),
        AstKind::Function => checks::check_function_term(ast, node),
        AstKind::AtomicFormula => checks::check_atomic_formula(ast, node),
        AstKind::Or | AstKind::And => checks::check_all_children_expression(ast, node),
        AstKind::Not
        | AstKind::AtStart
        | AstKind::AtEnd
        | AstKind::Overall
        | AstKind::Always
        | AstKind::Sometime
        | AstKind::AtMostOnce
        | AstKind::Goal
        | AstKind::Constraints
        | AstKind::TaskLogicalConstraintDef
        | AstKind::DurationConstraint => checks::check_unary_child_expression(ast, node),
        AstKind::Metric => {
            checks::check_children_count(node.arity(), 1, node)?;
            checks::check_child_kind(ast, node, 0, METRIC_EXPRESSION)
        }
        AstKind::Imply | AstKind::When | AstKind::SometimeAfter | AstKind::SometimeBefore => {
            checks::check_binary_child_expression(ast, node)
        }
        AstKind::Forall | AstKind::Exists => checks::check_quantified_expression(ast, node),
        AstKind::Preference => checks::check_preference_expression(ast, node),
        AstKind::Comparison => checks::check_fcomp_expression(ast, node),
        AstKind::Assignment => checks::check_assign_expression(ast, node),
        AstKind::Arithmetic => checks::check_arithmetic_expression(ast, node),
        AstKind::Within | AstKind::HoldAfter => {
            checks::check_within_hold_after_expression(ast, node)
        }
        AstKind::AlwaysWithin => checks::check_always_within_expression(ast, node),
        AstKind::HoldDuring => checks::check_hold_during_expression(ast, node),
        AstKind::Init => checks::check_init_expression(ast, node),
        AstKind::TimedInitialLiteral => checks::check_timed_initial_literal(ast, node),
        AstKind::DerivedDef => checks::check_derived_def(ast, node),
        AstKind::OrderedSubtaskDef | AstKind::PartiallyOrderedSubtaskDef => {
            checks::check_ordered_subtask_def(ast, node)
        }
        AstKind::LabeledTask => checks::check_tagged_task(ast, node),
        AstKind::TaskOrderingConstraintDef => checks::check_task_ordering_def(ast, node),
        AstKind::TaskOrderingConstraint => checks::check_task_ordering_constraint(ast, node),
        AstKind::TaskNetworkDef => checks::check_task_network_def(ast, node),
        AstKind::InitialTaskNetwork => checks::check_initial_task_network(ast, node),
        AstKind::TaskDef => checks::check_task_def(ast, node),
        AstKind::TotalTime => checks::check_total_time(node),
        AstKind::IsViolated => checks::check_is_violated(ast, node),
        AstKind::Length => checks::check_length_spec(ast, node),
        AstKind::Serial | AstKind::Parallel => checks::check_serial_parallel_expression(ast, node),
        AstKind::DurativeActionDef => checks::check_durative_action_def(ast, node),
        AstKind::DADefBody => checks::check_duartive_action_def_body(ast, node),
        AstKind::Domain => {
            //eprintln!("TODO: handling AstKind::Domain is not implemented yet");
            Ok(())
        }

        AstKind::Problem => {
            //eprintln!("TODO: handling AstKind::Problem is not implemented yet");
            Ok(())
        }
    }
}
