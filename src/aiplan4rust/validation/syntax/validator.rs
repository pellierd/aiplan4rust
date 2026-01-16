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

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::syntax::ast::{Ast, AstKind, AstNode};
use crate::aiplan4rust::validation::common::{checks, WellFormedError};
use crate::aiplan4rust::validation::{common, syntax};

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
/// This function verifies that the AST forms a valid tree (no cycles, at most one parent per node)
/// and that all nodes satisfy structural well-formedness rules.
///
/// # Arguments
/// * `ast` - The AST to validate.
///
/// # Returns
/// * `Ok(())` if the AST is structurally well-formed.
/// * `Err(WellFormedError)` if structural issues are found (including cycles).
///
/// # Note
/// This function only checks structural correctness, not semantic validity.
pub fn check_well_formed(ast: &Ast) -> Result<(), WellFormedError> {
    let arena = ast.syntax_tree();

    // Must be a valid tree
    if !arena.is_tree() {
        return Err(WellFormedError::cycle_detected());
    }

    match arena.root_node() {
        Some(root) => check_well_formed_from(root, ast),
        None => Ok(()), // An empty AST is considered well-formed
    }
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
/// * `Ok(())` if the node passes all structural checks for its kind.
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
        AstKind::Constant
        | AstKind::Variable
        | AstKind::FunctionSymbol
        | AstKind::PrimitiveType
        | AstKind::DomainName
        | AstKind::ProblemName
        | AstKind::Predicate
        | AstKind::ActionSymbol
        | AstKind::DASymbol
        | AstKind::MethodSymbol
        | AstKind::TaskSymbol
        | AstKind::PrefName
        | AstKind::TaskID => {
            syntax::checks::check_symbol(node)
        }
        AstKind::Number => {
            syntax::checks::check_number(node)
        }
        AstKind::Requirement => {
            syntax::checks::check_requirement(node)
        }
        AstKind::Error => {
            common::checks::throw_invalid(node)
        }
        AstKind::RequireDef => {
            syntax::checks::check_require_def(ast, node)
        }
        AstKind::Type => {
            syntax::checks::check_type(ast, node)
        }
        AstKind::TypesDef => {
            syntax::checks::check_types_def(ast, node)
        }
        AstKind::TypedList => {
            syntax::checks::check_typed_list(ast, node)
        }
        AstKind::TypedItem => {
            syntax::checks::check_typed_item(ast, node)
        }
        AstKind::TypedItemElements => {
            syntax::checks::check_typed_item_elements(ast, node)
        }
        AstKind::ConstantsDef | AstKind::ObjectsDef => {
            syntax::checks::check_constants_def(ast, node)
        }
        AstKind::PredicatesDef => {
            syntax::checks::check_predicates_def(ast, node)
        }
        AstKind::AtomicFormulaSkeleton => {
            syntax::checks::check_atomic_formula_skeleton(ast, node)
        }
        AstKind::FunctionsDef => {
            syntax::checks::check_functions_def(ast, node)
        }
        AstKind::AtomicFunctionSkeleton => {
            syntax::checks::check_atomic_function_skeleton(ast, node)
        }
        AstKind::ActionDef => {
            syntax::checks::check_action_def(ast, node)
        }
        AstKind::ActionDefBody => {
            syntax::checks::check_action_def_body(ast, node)
        }
        AstKind::MethodDef => {
            syntax::checks::check_method_def(ast, node)
        }
        AstKind::ParametersDef => {
            syntax::checks::check_parameters_def(ast, node)
        }
        AstKind::MethodDefBody => {
            syntax::checks::check_method_def_body(ast, node)
        }
        AstKind::Task => {
            syntax::checks::check_task(ast, node)
        }
        AstKind::PreconditionDef | AstKind::MethodPreconditionDef => {
            syntax::checks::check_precondition_def(ast, node)
        }
        AstKind::EffectDef => {
            syntax::checks::check_effect_def(ast, node)
        }
        AstKind::FunctionTerm => {
            syntax::checks::check_function_term(ast, node)
        }
        AstKind::AtomicFormula => {
            syntax::checks::check_atomic_formula(ast, node)
        }
        AstKind::Or | AstKind::And => {
            syntax::checks::check_all_children_expression(ast, node)
        }
        AstKind::Not
        | AstKind::AtStart
        | AstKind::AtEnd
        | AstKind::Overall
        | AstKind::Always
        | AstKind::Sometime
        | AstKind::AtMostOnce
        | AstKind::Goal
        | AstKind::Constraints
        | AstKind::Metric
        | AstKind::TaskLogicalConstraintDef => {
            syntax::checks::check_unary_child_expression(ast, node)
        }
        AstKind::Imply | AstKind::When | AstKind::SometimeAfter | AstKind::SometimeBefore => {
            syntax::checks::check_binary_child_expression(ast, node)
        }
        AstKind::Forall | AstKind::Exists => {
            syntax::checks::check_quantified_expression(ast, node)
        }
        AstKind::Preference => {
            syntax::checks::check_preference_expression(ast, node)
        }
        AstKind::FComp => {
            syntax::checks::check_fcomp_expression(ast, node)
        }
        AstKind::Assign => {
            syntax::checks::check_assign_expression(ast, node)
        }
        AstKind::Operation => {
            syntax::checks::check_arithmetic_expression(ast, node)
        }
        AstKind::Within | AstKind::HoldAfter => {
            syntax::checks::check_within_hold_after_expression(ast, node)
        }
        AstKind::AlwaysWithin => {
            syntax::checks::check_always_within_expression(ast, node)
        }
        AstKind::HoldDuring => {
            syntax::checks::check_hold_during_expression(ast, node)
        }
        AstKind::Init => {
            syntax::checks::check_init_expression(ast, node)
        }
        AstKind::TimedInitialLiteral => {
            syntax::checks::check_timed_initial_literal(ast, node)
        }
        AstKind::DerivedDef => {
            syntax::checks::check_derived_def(ast, node)
        }
        AstKind::OrderedSubtaskDef | AstKind::PartiallyOrderedSubtaskDef => {
            syntax::checks::check_ordered_subtask_def(ast, node)
        }
        AstKind::TaggedTask => {
            syntax::checks::check_tagged_task(ast, node)
        }
        AstKind::TaskOrderingConstraintDef => {
            syntax::checks::check_task_ordering_def(ast, node)
        }
        AstKind::TaskOrderingConstraint => {
            syntax::checks::check_task_ordering_constraint(ast, node)
        }
        AstKind::TaskNetworkDef => {
            syntax::checks::check_task_network_def(ast, node)
        }
        AstKind::InitialTaskNetwork => {
            syntax::checks::check_initial_task_network(ast, node)
        }
        AstKind::TaskDef => {
            syntax::checks::check_task_def(ast, node)
        }
        AstKind::TotalTime => {
            syntax::checks::check_total_time(node)
        }
        AstKind::IsViolated => {
            syntax::checks::check_is_violated(ast, node)
        }
        AstKind::Length => {
            syntax::checks::check_length_spec(ast, node)
        }
        AstKind::Serial | AstKind::Parallel => {
            syntax::checks::check_serial_parallel_expression(ast, node)
        }
        AstKind::DurativeActionDef => {
            syntax::checks::check_durative_action_def(ast, node)
        }
        AstKind::DADefBody => {
            syntax::checks::check_duartive_action_def_body(ast, node)
        }
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
