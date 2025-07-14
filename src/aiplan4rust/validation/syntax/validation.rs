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
//! Semantic correctness, type checking, and domain-specific validations are out of scope here and must be
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
use crate::aiplan4rust::validation::common::checks::EXPRESSION;
use crate::aiplan4rust::validation::{common, syntax};

/// Checks that the AST is structurally well-formed starting from its root node.
///
/// This function verifies the presence of a root node and then recursively checks
/// the structure of each node and its children.
///
/// # Arguments
/// * `ast` - The AST to validate.
///
/// # Returns
/// * `Ok(())` if the AST is structurally well-formed.
/// * `Err(WellFormedError)` if structural issues are found.
///
/// # Note
/// This function only checks structural correctness, not semantic validity.
pub fn is_well_formed(ast: &Ast) -> Result<(), WellFormedError> {
    match ast.arena().root_node() {
        Some(root) => {
            check_well_formed_from(root, ast)
        }
        None => Ok(()), // No root node means empty tree which can be considered well-formed
    }
}

/// Checks that the AST is structurally well-formed starting from its root node.
///
/// This function verifies the presence of a root node and then recursively checks
/// the structure of each node and its children.
///
/// # Arguments
/// * `ast` - The AST to validate.
///
/// # Returns
/// * `Ok(())` if the AST is structurally well-formed.
/// * `Err(WellFormedError)` if structural issues are found.
///
/// # Note
/// This function only checks structural correctness, not semantic validity.
pub fn check_well_formed(ast: &Ast) -> Result<(), WellFormedError> {
    match ast.arena().root_node() {
        Some(root) => {
            check_well_formed_from(root, ast)
        }
        None => Ok(()),
    }
}

/// Recursively checks that the given node and its descendants are structurally well-formed.
///
/// It ensures the node has the expected number and kind of children,
/// and that each child node recursively satisfies these structural rules.
///
/// # Arguments
/// * `node` - The starting AST node for the check.
/// * `ast` - Reference to the whole AST structure.
///
/// # Returns
/// * `Ok(())` if this subtree is structurally valid.
/// * `Err(WellFormedError)` if any structural violation is detected.
///
/// # Note
/// This function assumes the AST nodes are logically consistent and does not perform semantic checks.
fn check_well_formed_from(node: &AstNode, ast: &Ast) -> Result<(), WellFormedError> {
    //println!("Validating {}", node.to_string_with_interner(ast.arena(), ast.interner()));
    //println!("Validating {}", node);
    let children_ids = node.children();

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
            syntax::checks::check_symbol(node)?;
        }
        AstKind::Number => {
            syntax::checks::check_number(node)?;
        }
        AstKind::Requirement => {
            syntax::checks::check_requirement(node)?;
        }
        AstKind::Error => {
            common::checks::throw_invalid(node)?;
        }
        AstKind::RequireDef => {
            syntax::checks::check_require_def(ast, node)?;
        }
        AstKind::Type => {
            syntax::checks::check_type(ast, node)?;
        }
        AstKind::TypesDef => {
            syntax::checks::check_types_def(ast, node)?;
        }
        AstKind::TypedList => {
            syntax::checks::check_typed_list(ast, node)?;
        }
        AstKind::TypedItem => {
            syntax::checks::check_typed_item(ast, node)?;
        }
        AstKind::TypedItemElements => {
            syntax::checks::check_typed_item_elements(ast, node)?;
        }
        AstKind::ConstantsDef
        | AstKind::ObjectsDef => {
            syntax::checks::check_constants_def(ast, node)?;
        }
        AstKind::PredicatesDef => {
            syntax::checks::check_predicates_def(ast, node)?;
        }
        AstKind::AtomicFormulaSkeleton => {
            syntax::checks::check_atomic_formula_skeleton(ast, node)?;
        }
        AstKind::FunctionsDef => {
            syntax::checks::check_functions_def(ast, node)?;
        }
        AstKind::AtomicFunctionSkeleton => {
            syntax::checks::check_atomic_function_skeleton(ast, node)?;
        }
        AstKind::ActionDef => {
            syntax::checks::check_action_def(ast, node)?;
        }
        AstKind::ActionDefBody => {
           syntax::checks::check_action_def_body(ast, node)?;
        }
        AstKind::MethodDef => {
           syntax::checks::check_method_def(ast, node)?;
        }
        AstKind::ParametersDef => {
           syntax::checks::check_parameters_def(ast, node)?;
        }
        AstKind::MethodDefBody => {
           syntax::checks::check_method_def_body(ast, node)?;
        }
        AstKind::Task => {
            syntax::checks::check_task(ast, node)?;
        }
        AstKind::PreconditionDef
        | AstKind::MethodPreconditionDef => {
            syntax::checks::check_precondition_def(ast, node)?;
        }
        AstKind::EffectDef => {
           syntax::checks::check_effect_def(ast, node)?;
        }
        AstKind::FunctionTerm => {
            syntax::checks::check_function_term(ast, node)?;
        }
        AstKind::AtomicFormula => {
            syntax::checks::check_atomic_formula(ast, node)?;
        }
        AstKind::Or
        | AstKind::And => {
            syntax::checks::check_all_children_expression(ast, node)?;
        }
        | AstKind::Not
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
           syntax::checks::check_unary_child_expression(ast, node)?;
        }
        | AstKind::Imply
        | AstKind::When
        | AstKind::SometimeAfter
        | AstKind::SometimeBefore => {
            syntax::checks::check_binary_child_expression(ast, node)?;
        }
        AstKind::Forall
        | AstKind::Exists => {
            syntax::checks::check_quantifier_expression(ast, node)?;
        }
        AstKind::Preference => {
            syntax::checks::check_preference_expression(ast, node)?;
        }
        AstKind::FComp => {
            syntax::checks::check_fcomp_expression(ast, node)?;
        }
        AstKind::Assign => {
            syntax::checks::check_assign_expression(ast, node)?;
        }
        AstKind::Operation => {
            syntax::checks::check_arithmetic_expression(ast, node)?;
        }
        AstKind::Within
        | AstKind::HoldAfter => {
            syntax::checks::check_within_hold_after_expression(ast, node)?;
        }
        AstKind::AlwaysWithin => {
            syntax::checks::check_always_within_expression(ast, node)?;
        }
        AstKind::HoldDuring => {
            syntax::checks::check_hold_during_expression(ast, node)?;
        }
        AstKind::Init => {
           syntax::checks::check_init_expression(ast, node)?;
        }
        AstKind::TimedInitialLiteral => {
            syntax::checks::check_timed_initial_literal(ast, node)?;
        }
        AstKind::DerivedDef => {
            syntax::checks::check_derived_def(ast, node)?;
        }
        AstKind::OrderedSubtaskDef
        | AstKind::PartiallyOrderedSubtaskDef => {
            syntax::checks::check_ordered_subtask_def(ast, node)?;
        }
        AstKind::TaggedTask => {
            syntax::checks::check_tagged_task(ast, node)?;
        }
        AstKind::TaskOrderingConstraintDef => {
           syntax::checks::check_task_ordering_def(ast, node)?;
        }
        AstKind::TaskOrderingConstraint => {
           syntax::checks::check_task_ordering_constraint(ast, node)?;
        }
        AstKind::TaskNetworkDef => {
           syntax::checks::check_task_network_def(ast, node)?;
        }
        AstKind::InitialTaskNetwork => {
            syntax::checks::check_initial_task_network(ast, node)?;
        }
        AstKind::TaskDef => {
           syntax::checks::check_task_def(ast, node)?;
        }
        AstKind::TotalTime => {
           syntax::checks::check_total_time(ast, node)?;
        }
        AstKind::IsViolated => {
           syntax::checks::check_is_violated(ast, node)?;
        }
        AstKind::Length => {
           syntax::checks::check_length_spec(ast, node)?;
        }
        AstKind::Serial | AstKind::Parallel => {
           syntax::checks::check_serial_parallel_expression(ast, node)?;
        }
        AstKind::DurativeActionDef => {
            syntax::checks::check_durative_action_def(ast, node)?;
        }
        AstKind::DADefBody => {
            syntax::checks::check_duartive_action_def_body(ast, node)?;
        }
        AstKind::Domain => {
            // TODO
        }
        AstKind::Problem => {
            // TODO
        }
    }

    for child_id in children_ids {
        let child_node = checks::get_node(ast, node, child_id.as_usize())?;
        check_well_formed_from(child_node, ast)?;
    }

    Ok(())
}
