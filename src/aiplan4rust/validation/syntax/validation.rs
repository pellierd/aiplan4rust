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
            checks::check_all_children_kind(ast, node, &[AstKind::Requirement])?;
        }
        AstKind::Type => {
            checks::check_min_children_count(node.children().len(), 1, node)?;
            checks::check_all_children_kind(ast, node, &[AstKind::PrimitiveType])?;
        }
        AstKind::TypesDef => {
            checks::check_min_children_count(node.children().len(), 1, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::TypedList])?;
        }
        AstKind::TypedList => {
            checks::check_all_children_kind(ast, node, &[AstKind::TypedItem])?;
        }
        AstKind::TypedItem => {
            checks::check_children_count_range(node.children().len(), 1, 2, node)?;

            match node.children().len() {
                1 => {
                    // Si un seul enfant, il doit être TypedItemElements
                    checks::check_child_kind(ast, node, 0, &[AstKind::TypedItemElements])
                }
                2 => {
                    // Si deux enfants :
                    // - le premier est TypedItemElements
                    // - le second est optionnellement Type
                    checks::check_child_kind(ast, node, 0, &[AstKind::TypedItemElements])?;
                    checks::check_child_kind(ast, node, 1, &[AstKind::Type])
                }
                _ => unreachable!(),
            }?;
        }
        AstKind::TypedItemElements => {
            checks::check_min_children_count(node.children().len(), 1, node)?;
            checks::check_all_children_kind(ast, node, &[
                AstKind::PrimitiveType,
                AstKind::Variable,
                AstKind::Constant,
                AstKind::FunctionTerm,
                AstKind::AtomicFunctionSkeleton,
            ])?;
        }
        AstKind::ConstantsDef
        | AstKind::ObjectsDef => {
            checks::check_children_count(children_ids.len(), 1, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::TypedList])?;
        }
        AstKind::PredicatesDef => {
            checks::check_all_children_kind(ast, node, &[AstKind::AtomicFormulaSkeleton])?;
        }
        AstKind::AtomicFormulaSkeleton => {
            checks::check_children_count_range(node.children().len(), 1, 2, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::Predicate])?;
            checks::check_child_kind(ast, node, 1, &[AstKind::TypedList])?;
        }
        AstKind::FunctionsDef => {
            checks::check_children_count(children_ids.len(), 1, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::TypedList])?;
        }
        AstKind::AtomicFunctionSkeleton => {
            checks::check_children_count_range(node.children().len(), 1, 2, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::FunctionSymbol])?;
            checks::check_child_kind(ast, node, 1, &[AstKind::TypedList])?;
        }
        AstKind::ActionDef => {
            checks::check_children_count(children_ids.len(), 3, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::ActionSymbol])?;
            checks::check_child_kind(ast, node, 1, &[AstKind::ParametersDef])?;
            checks::check_child_kind(ast, node, 2, &[AstKind::ActionDefBody])?;
        }
        AstKind::ActionDefBody => {
            checks::check_children_count_range(node.children().len(), 0, 2, node)?;
            match node.children().len() {
                0 => Ok(()),
                1 => {
                    // Un seul enfant, c’est forcément la précondition (indice 0)
                    checks::check_child_kind(ast, node, 0, &[AstKind::PreconditionDef, AstKind::EffectDef])
                }
                2 => {
                    // Deux enfants : 0 = précondition, 1 = effet
                    checks::check_child_kind(ast, node, 0, &[AstKind::PreconditionDef])?;
                    checks::check_child_kind(ast, node, 1, &[AstKind::EffectDef])
                }
                _ => unreachable!(),
            }?;
        }
        AstKind::MethodDef => {
            checks::check_children_count(children_ids.len(), 3, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::MethodSymbol])?;
            checks::check_child_kind(ast, node, 1, &[AstKind::ParametersDef])?;
            checks::check_child_kind(ast, node, 2, &[AstKind::MethodDefBody])?;
        }
        AstKind::ParametersDef => {
            checks::check_children_count(node.children().len(), 1, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::TypedList])?;
        }
        AstKind::MethodDefBody => {
            checks::check_children_count_range(node.children().len(), 2, 3, node)?;
            match node.children().len() {
                2 => {
                    checks::check_child_kind(ast, node, 0, &[AstKind::Task])?;
                    checks::check_child_kind(ast, node, 1, &[AstKind::TaskNetworkDef])
                }
                3 => {
                    checks::check_child_kind(ast, node, 0, &[AstKind::Task])?;
                    checks::check_child_kind(ast, node, 1, &[AstKind::MethodPreconditionDef])?;
                    checks::check_child_kind(ast, node, 2, &[AstKind::TaskNetworkDef])
                }
                _ => unreachable!(),
            }?;
        }
        AstKind::Task => {
            checks::check_min_children_count(node.children().len(), 1, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::TaskSymbol])?;
            checks::check_children_kind_from(ast, node, 1, &[AstKind::Variable, AstKind::Constant])?;
        }
        AstKind::PreconditionDef
        | AstKind::MethodPreconditionDef => {
            checks::check_children_count(node.children().len(), 1, node)?;
            checks::check_child_kind(ast, node, 0, EXPRESSION)?;
        }
        AstKind::EffectDef => {
            checks::check_children_count(node.children().len(), 1, node)?;
            checks::check_child_kind(ast, node, 0, EXPRESSION)?;
        }
        AstKind::FunctionTerm => {
            checks::check_min_children_count(node.children().len(), 1, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::FunctionSymbol])?;
            checks::check_children_kind_from(ast, node, 1, &[AstKind::Variable, AstKind::Constant])?;
        }
        AstKind::AtomicFormula => {
            checks::check_min_children_count(node.children().len(), 1, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::Predicate])?;
            checks::check_children_kind_from(ast, node, 1, &[AstKind::Variable, AstKind::Constant])?;
        }
        AstKind::Or
        | AstKind::And => {
            checks::check_all_children_kind(ast, node, EXPRESSION)?;
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
        | AstKind::Metric => {
            checks::check_min_children_count(node.children().len(), 1, node)?;
            checks::check_child_kind(ast, node, 0, EXPRESSION)?;
        }
        | AstKind::Imply
        | AstKind::When
        | AstKind::SometimeAfter
        | AstKind::SometimeBefore => {
            checks::check_min_children_count(node.children().len(), 2, node)?;
            checks::check_child_kind(ast, node, 0, EXPRESSION)?;
            checks::check_child_kind(ast, node, 1, EXPRESSION)?;
        }
        AstKind::Forall
        | AstKind::Exists => {
            checks::check_min_children_count(node.children().len(), 2, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::TypedList])?;
            checks::check_child_kind(ast, node, 1, EXPRESSION)?;
        }
        AstKind::Preference => {
            checks::check_min_children_count(node.children().len(), 2, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::PrefName])?;
            checks::check_child_kind(ast, node, 1, EXPRESSION)?;
        }
        AstKind::FComp => {
            checks::check_children_count_range(node.children().len(), 1, 2, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::Number, AstKind::FComp, AstKind::FunctionTerm, AstKind::Variable, AstKind::Constant])?;
            checks::check_child_kind(ast, node, 1, &[AstKind::FComp, AstKind::FunctionTerm, AstKind::Variable, AstKind::Constant])?;
        }
        AstKind::Assign => {
            checks::check_min_children_count(node.children().len(), 2, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::FunctionTerm])?;
            checks::check_child_kind(ast, node, 1, &[AstKind::FComp, AstKind::Variable, AstKind::Constant, AstKind::FunctionTerm])?; // Todo: Adding undefined
        }
        AstKind::Operation => {
            checks::check_children_count_range(node.children().len(), 1, 2, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::FComp])?;
            checks::check_child_kind(ast, node, 1, &[AstKind::FComp])?;
        }
        AstKind::Within
        | AstKind::HoldAfter => {
            checks::check_children_count(node.children().len(), 2, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::Number])?;
            checks::check_child_kind(ast, node, 1, EXPRESSION)?;
        }
        AstKind::AlwaysWithin => {
            checks::check_children_count(node.children().len(), 3, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::Number])?;
            checks::check_child_kind(ast, node, 1, EXPRESSION)?;
            checks::check_child_kind(ast, node, 2, EXPRESSION)?;
        }
        AstKind::HoldDuring => {
            checks::check_children_count(node.children().len(), 3, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::Number])?;
            checks::check_child_kind(ast, node, 1, &[AstKind::Number])?;
            checks::check_child_kind(ast, node, 2, EXPRESSION)?;
        }
        AstKind::Init => {
            checks::check_children_count(node.children().len(), 1, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::And])?;
            let init_elements = checks::get_child_node(ast, node, 0)?;
            checks::check_all_children_kind(ast, init_elements, &[AstKind::TimedInitialLiteral, AstKind::FComp, AstKind::AtomicFormula, AstKind::Not])?;
            // Todo: check that not contains only atomic formula
        }
        AstKind::TimedInitialLiteral => {
            checks::check_children_count(node.children().len(), 2, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::Number])?;
            checks::check_child_kind(ast, node, 1, &[AstKind::FComp, AstKind::Not])?;
            // Todo: check that not contains only atomic formula
        }
        AstKind::DerivedDef => {
            checks::check_children_count(node.children().len(), 2, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::AtomicFormulaSkeleton])?;
            checks::check_child_kind(ast, node, 1, EXPRESSION)?;
        }

        AstKind::OrderedSubtaskDef
        | AstKind::PartiallyOrderedSubtaskDef => {
            checks::check_children_count(node.children().len(), 1, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::And])?;
            let tasks = checks::get_child_node(ast, node, 0)?;
            checks::check_all_children_kind(ast, tasks, &[AstKind::TaggedTask, AstKind::Task])?;
        }
        AstKind::TaggedTask => {
            checks::check_children_count(node.children().len(), 2, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::TaskID])?;
            checks::check_child_kind(ast, node, 1, &[AstKind::Task])?;
        }
        AstKind::TaskOrderingConstraintDef => {
            checks::check_children_count(node.children().len(), 1, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::And])?;
            let ordering = checks::get_child_node(ast, node, 0)?;
            checks::check_all_children_kind(ast, ordering, &[AstKind::TaskOrderingConstraint])?;
        }
        AstKind::TaskOrderingConstraint => {
            checks::check_children_count(node.children().len(), 2, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::TaskID])?;
            checks::check_child_kind(ast, node, 1, &[AstKind::TaskID])?;
        }
        AstKind::TaskLogicalConstraintDef => {
            checks::check_all_children_kind(ast, node, EXPRESSION)?;
        }
        AstKind::TaskNetworkDef => {
            checks::check_children_count_range(node.children().len(), 0, 3, node)?;
            match node.children().len() {
                0 => Ok(()),
                1 => {
                    checks::check_child_kind(ast, node, 0, &[
                        AstKind::OrderedSubtaskDef, AstKind::PartiallyOrderedSubtaskDef, AstKind::TaskLogicalConstraintDef
                    ])
                },
                2 => {
                    checks::check_child_kind(ast, node, 0, &[
                        AstKind::OrderedSubtaskDef, AstKind::PartiallyOrderedSubtaskDef
                    ])?;
                    checks::check_child_kind(ast, node, 1, &[
                        AstKind::TaskOrderingConstraintDef, AstKind::TaskLogicalConstraintDef
                    ])
                },
                3 => {
                    checks::check_child_kind(ast, node, 0, &[
                        AstKind::OrderedSubtaskDef, AstKind::PartiallyOrderedSubtaskDef
                    ])?;
                    checks::check_child_kind(ast, node, 1, &[
                        AstKind::TaskOrderingConstraintDef
                    ])?;
                    checks::check_child_kind(ast, node, 2, &[
                        AstKind::TaskLogicalConstraintDef
                    ])
                },
                _ => { unreachable!() }
            }?;

        }
        AstKind::InitialTaskNetwork => {
            checks::check_children_count_range(node.children().len(),1, 2, node)?;
            match node.children().len() {
                1 => {
                    checks::check_child_kind(ast, node, 0, &[AstKind::TaskNetworkDef])
                },
                2 => {
                    checks::check_child_kind(ast, node, 0, &[AstKind::ParametersDef])?;
                    checks::check_child_kind(ast, node, 1, &[AstKind::TaskNetworkDef])
                },
                _ => { unreachable!() }
            }?;
        }
        AstKind::TaskDef => {
            checks::check_children_count(node.children().len(),2, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::TaskSymbol])?;
            checks::check_child_kind(ast, node, 1, &[AstKind::ParametersDef])?;
        }
        AstKind::TotalTime => {
            checks::check_children_count(node.children().len(),0, node)?;
        }
        AstKind::IsViolated => {
            checks::check_children_count(node.children().len(),1, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::PrefName])?;
        }
        AstKind::Length => {
            checks::check_children_count_range(node.children().len(), 0, 2, node)?;
            checks::check_all_children_kind(ast, node, &[AstKind::Serial, AstKind::Parallel])?;
        }
        AstKind::Serial | AstKind::Parallel => {
            checks::check_children_count(node.children().len(),1, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::Number])?;
        }
        AstKind::DurativeActionDef => {
            checks::check_children_count(children_ids.len(), 3, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::DASymbol])?;
            checks::check_child_kind(ast, node, 1, &[AstKind::ParametersDef])?;
            checks::check_child_kind(ast, node, 2, &[AstKind::DADefBody])?;

        }
        AstKind::DADefBody => {
            checks::check_children_count(node.children().len(), 3, node)?;
            checks::check_child_kind(ast, node, 0, EXPRESSION)?;
            checks::check_child_kind(ast, node, 1, EXPRESSION)?;
            checks::check_child_kind(ast, node, 2, EXPRESSION)?;
        }
        AstKind::Domain => {}
        AstKind::Problem => {}

        _ => {
            println!("ERROR: {}", node.kind());
        }

    }

    for child_id in children_ids {
        let child_node = checks::get_node(ast, node, child_id.as_usize())?;
        check_well_formed_from(child_node, ast)?;
    }

    Ok(())
}


pub fn check_typed_list(node: &AstNode, ast: &Ast, expected: &[AstKind]) -> Result<(), WellFormedError> {
    let children_ids = node.children();
    match node.kind() {
       AstKind::TypedList => {
           checks::check_all_children_kind(ast, node, &[AstKind::TypedItem])?;
        }
        AstKind::TypedItem => {
            checks::check_children_count_range(node.children().len(), 1, 2, node)?;
            checks::check_child_kind(ast, node, 0, &[AstKind::TypedItemElements])?;
            checks::check_child_kind(ast, node, 1, &[AstKind::Type])?;
        }
        AstKind::TypedItemElements => {
            checks::check_min_children_count(node.children().len(), 1, node)?;
            checks::check_all_children_kind(ast, node, expected)?;
        }
        _ => {
            check_well_formed_from(node, ast)?;
        }
    }
    for child_id in children_ids {
        let child_node = checks::get_node(ast, node, child_id.as_usize())?;
        check_typed_list(child_node, ast, expected)?;
    }

    Ok(())
}
