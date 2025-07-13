use crate::aiplan4rust::arena::{ArenaNode, NodeId};
use crate::aiplan4rust::syntax::ast::{Ast, AstContent, AstKind, AstNode};
use crate::aiplan4rust::syntax::ast::AstKind::{And, AtomicFormula, AtomicFormulaSkeleton, Constant, DADefBody, DASymbol, FComp, FunctionTerm, HoldAfter, Not, Number, OrderedSubtaskDef, Parallel, ParametersDef, PartiallyOrderedSubtaskDef, PrefName, Serial, TaggedTask, Task, TaskID, TaskLogicalConstraintDef, TaskNetworkDef, TaskOrderingConstraint, TaskOrderingConstraintDef, TaskSymbol, TimedInitialLiteral, Variable};
use crate::aiplan4rust::syntax::ast::kind::Kind;
// adapte selon tes imports


const EXPRESSION: &[Kind] = &[
    Kind::Or,
    Kind::And,
    Kind::Not,
    Kind::Imply,
    Kind::Exists,
    Kind::Forall,
    Kind::AtomicFormula,
    Kind::Preference,
    Kind::When,
    Kind::FComp,
    Kind::Assign,
    Kind::Operation,
    Kind::AtStart,
    Kind::AtEnd,
    Kind::Overall,
    Kind::Always,
    Kind::Sometime,
    Kind::Within,
    Kind::AtMostOnce,
    Kind::SometimeAfter,
    Kind::SometimeBefore,
    Kind::AlwaysWithin,
    Kind::HoldDuring,
    Kind::HoldAfter,
    Kind::TimedInitialLiteral,
    Kind::TaggedTask,
    Kind::TaskOrderingConstraint,
];

#[derive(Debug)]
pub enum ValidationError {
    WrongChildCount {
        expected: usize,
        found: usize,
        kind: AstKind,
    },
    UnexpectedKind {
        index: usize,
        expected: Vec<AstKind>, // <- passé de AstKind à Vec<AstKind>
        found: AstKind,
        parent: AstKind,
    },
    MissingNode {
        child_id: NodeId,
        parent: AstKind,
    },
    UnexpectedContent {
        found: AstContent,
        node_kind: AstKind,
    },
    InvalidKind {
        found: AstKind,
    },
    WrongChildCountRange {
        expected_min: usize,
        expected_max: usize,
        found: usize,
        kind: AstKind
    },
    UnexpectedChildKind {
        index: Vec<usize>,
        expected: Vec<Kind>,
        found: Vec<Kind>,
        parent: Kind,
    },
    Custom(String),
}

use std::fmt;
use crate::aiplan4rust::interner::InternerDisplay;

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::WrongChildCount { expected, found, kind } => {
                write!(
                    f,
                    "Wrong number of children for node {:?}: expected {}, found {}.",
                    kind, expected, found
                )
            }
            ValidationError::UnexpectedKind { index, expected, found, parent } => {
                write!(
                    f,
                    "Unexpected child kind at index {} in parent {:?}: expected one of {:?}, found {:?}.",
                    index, parent, expected, found
                )
            }
            ValidationError::MissingNode { child_id, parent } => {
                write!(
                    f,
                    "Missing child node with ID {:?} in parent {:?}.",
                    child_id, parent
                )
            }
            ValidationError::UnexpectedContent { found, node_kind } => {
                write!(
                    f,
                    "Unexpected content in node {:?}: found {:?}.",
                    node_kind, found
                )
            }
            ValidationError::InvalidKind { found } => {
                write!(
                    f,
                    "Invalid node kind encountered: {:?}.",
                    found
                )
            }
            ValidationError::WrongChildCountRange { expected_min, expected_max, found, kind } => {
                write!(
                    f,
                    "Wrong number of children for node {:?}: expected between {} and {}, found {}.",
                    kind, expected_min, expected_max, found
                )
            }
            ValidationError::UnexpectedChildKind { index, expected, found, parent } => {
                write!(
                    f,
                    "Unexpected child kinds in parent {:?}:\n  Indices: {:?}\n  Expected kinds: {:?}\n  Found kinds: {:?}.",
                    parent, index, expected, found
                )
            }
            ValidationError::Custom(message) => {
                write!(f, "{}", message)
            }
        }
    }
}


// 1) On définit l’enum des “kinds” de contenu :
#[derive(Debug)]
pub enum ContentKind {
    None,
    Ident,
    Float,
    Requirement,
    BinaryComp,
    AssignOp,
    ArithmeticOp,
    Optimization,
}

fn check_children_kind(
    ast: &Ast,
    parent: &AstNode,
    indices: &[usize],
    expected_kinds: &[AstKind],
) -> Result<(), ValidationError> {
    let children = parent.children();

    let mut bad_indices = Vec::new();
    let mut bad_found = Vec::new();

    for &index in indices {
        if index >= children.len() {
            continue;
        }

        let child = get_node(ast, parent.kind(), children[index].as_usize())?;
        let found = child.kind();

        if !expected_kinds.contains(&found) {
            bad_indices.push(index);
            bad_found.push(found);
        }
    }

    if bad_indices.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::UnexpectedChildKind {
            index: bad_indices,
            expected: expected_kinds.to_vec(),
            found: bad_found,
            parent: parent.kind(),
        })
    }
}



fn check_children_count(
    children_len: usize,
    expected: usize,
    kind: AstKind,
) -> Result<(), ValidationError> {
    if children_len != expected {
        Err(ValidationError::WrongChildCount {
            expected,
            found: children_len,
            kind,
        })
    } else {
        Ok(())
    }
}

// Vérifie qu'il y a au moins `min` enfants.
fn check_min_children_count(
    children_len: usize,
    min: usize,
    kind: AstKind,
) -> Result<(), ValidationError> {
    if children_len < min {
        Err(ValidationError::WrongChildCount {
            expected: min,
            found: children_len,
            kind,
        })
    } else {
        Ok(())
    }
}

/// Vérifie que le nombre de fils est compris entre `min` et `max` inclus.
///
/// # Erreur
/// Retourne `ValidationError::WrongChildCount` si le nombre n'est pas dans l'intervalle.
fn check_children_count_range(
    children_count: usize,
    min: usize,
    max: usize,
    kind: AstKind,
) -> Result<(), ValidationError> {
    if children_count < min || children_count > max {
        Err(ValidationError::WrongChildCountRange {
            expected_min: min,
            expected_max: max,
            found: children_count,
            kind,
        })
    } else {
        Ok(())
    }
}

fn check_content(
    node: &AstNode,
    expected: ContentKind,
) -> Result<(), ValidationError> {
    match (node.content(), expected) {
        (AstContent::None, ContentKind::None) => Ok(()),
        (AstContent::Ident(_), ContentKind::Ident) => Ok(()),
        (AstContent::Float(_), ContentKind::Float) => Ok(()),
        (AstContent::Requirement(_), ContentKind::Requirement) => Ok(()),
        (AstContent::BinaryComp(_), ContentKind::BinaryComp) => Ok(()),
        (AstContent::AssignOp(_), ContentKind::AssignOp) => Ok(()),
        (AstContent::ArithmeticOp(_), ContentKind::ArithmeticOp) => Ok(()),
        (AstContent::Optimization(_), ContentKind::Optimization) => Ok(()),

        (found, _) => Err(ValidationError::UnexpectedContent {
            found: found.clone(),
            node_kind: node.kind(),
        }),
    }
}
fn invalid(
    found: AstKind,
) -> Result<(), ValidationError> {
    Err(ValidationError::InvalidKind {
        found,
    })
}

fn get_node(
    ast: &Ast,
    parent_kind: AstKind,
    node_id: usize,
) -> Result<&AstNode, ValidationError> {
    match ast.arena().get_node(NodeId::new(node_id)) {
        Some(node) => Ok(node),
        None => Err(ValidationError::MissingNode {
            child_id : NodeId::new(node_id),
            parent: parent_kind,
        }),
    }
}

pub fn check_all_children_kind(
    ast: &Ast,
    parent: &AstNode,
    expected_kinds: &[AstKind],
) -> Result<(), ValidationError> {
    check_children_kind_from(ast, parent, 0, expected_kinds)
}

fn check_children_kind_from(
    ast: &Ast,
    parent: &AstNode,
    start_index: usize,
    expected_kinds: &[AstKind],
) -> Result<(), ValidationError> {
    let children = parent.children();

    let mut bad_indices = Vec::new();
    let mut bad_found = Vec::new();

    for (i, &child_id) in children.iter().enumerate().skip(start_index) {
        let child = get_node(ast, parent.kind(), child_id.as_usize())?;
        let found = child.kind();

        if !expected_kinds.contains(&found) {
            bad_indices.push(i);
            bad_found.push(found);
        }
    }

    if bad_indices.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::UnexpectedChildKind {
            index: bad_indices,
            expected: expected_kinds.to_vec(),
            found: bad_found,
            parent: parent.kind(),
        })
    }
}

pub fn validate(ast: &Ast) -> Result<(), ValidationError> {
    match ast.arena().root_node() {
        Some(root) => {
            validate_from(root, ast)
        }
        None => Ok(()),
    }
}

pub fn validate_from(node: &AstNode, ast: &Ast) -> Result<(), ValidationError> {
    //println!("Validating {}", node.to_string_with_interner(ast.arena(), ast.interner()));
    println!("Validating {}", node);
    let children_ids = node.children();

    match node.kind() {
        Kind::Constant
        | Kind::Variable
        | Kind::FunctionSymbol
        | Kind::PrimitiveType
        | Kind::DomainName
        | Kind::ProblemName
        | Kind::Predicate
        | Kind::ActionSymbol
        | Kind::DASymbol
        | Kind::MethodSymbol
        | Kind::TaskSymbol
        | Kind::PrefName
        | Kind::TaskID => {
            check_content(node, ContentKind::Ident)?;
            check_children_count(children_ids.len(), 0, node.kind())?;
        }
        Kind::Number => {
            check_content(node, ContentKind::Float)?;
            check_children_count(children_ids.len(), 0, node.kind())?;
        }
        Kind::Requirement => {
            check_content(node, ContentKind::Requirement)?;
            check_children_count(children_ids.len(), 0, node.kind())?;
        }
        Kind::Error => {
            invalid(node.kind())?;
        }
        Kind::RequireDef => {
            check_all_children_kind(ast, node, &[AstKind::Requirement])?;
        }
        Kind::Type => {
            check_min_children_count(node.children().len(), 1, node.kind())?;
            check_all_children_kind(ast, node, &[AstKind::PrimitiveType])?;
        }
        Kind::TypesDef => {
            check_min_children_count(node.children().len(), 1, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::TypedList])?;
        }
        Kind::TypedList => {
            check_all_children_kind(ast, node, &[Kind::TypedItem])?;
        }
        Kind::TypedItem => {
            check_children_count_range(node.children().len(), 1, 2, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::TypedItemElements])?;
            check_children_kind(ast, node, &[1], &[Kind::Type])?;
        }
        Kind::TypedItemElements => {
            check_min_children_count(node.children().len(), 1, node.kind())?;
            check_all_children_kind(ast, node, &[
                AstKind::PrimitiveType,
                AstKind::Variable,
                AstKind::Constant,
                AstKind::FunctionTerm,
                AstKind::AtomicFunctionSkeleton,
            ])?;
        }
        Kind::ConstantsDef
        | Kind::ObjectsDef => {
            check_children_count(children_ids.len(), 1, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::TypedList])?;
        }
        Kind::PredicatesDef => {
            check_all_children_kind(ast, node, &[AstKind::AtomicFormulaSkeleton])?;
        }
        Kind::AtomicFormulaSkeleton => {
            check_children_count_range(node.children().len(), 1, 2, node.kind())?;
            check_children_kind(ast, node, &[0], &[AstKind::Predicate])?;
            check_children_kind(ast, node, &[1], &[AstKind::TypedList])?;
        }
        Kind::FunctionsDef => {
            check_children_count(children_ids.len(), 1, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::TypedList])?;
        }
        Kind::AtomicFunctionSkeleton => {
            check_children_count_range(node.children().len(), 1, 2, node.kind())?;
            check_children_kind(ast, node, &[0], &[AstKind::FunctionSymbol])?;
            check_children_kind(ast, node, &[1], &[Kind::TypedList])?;
        }
        Kind::ActionDef => {
            check_children_count(children_ids.len(), 3, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::ActionSymbol])?;
            check_children_kind(ast, node, &[1], &[Kind::ParametersDef])?;
            check_children_kind(ast, node, &[2], &[Kind::ActionDefBody])?;
        }
        Kind::ActionDefBody => {
            check_children_count_range(node.children().len(), 0, 2, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::PreconditionDef])?;
            check_children_kind(ast, node, &[0, 1], &[Kind::EffectDef])?;
        }
        AstKind::MethodDef => {
            check_children_count(children_ids.len(), 3, node.kind())?;
            check_children_kind(ast, node, &[0], &[AstKind::MethodSymbol])?;
            check_children_kind(ast, node, &[1], &[AstKind::ParametersDef])?;
            check_children_kind(ast, node, &[2], &[AstKind::MethodDefBody])?;
        }
        AstKind::ParametersDef => {
            check_children_count(node.children().len(), 1, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::TypedList])?;
        }
        Kind::MethodDefBody => {
            check_children_count(node.children().len(), 2, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::Task])?;
            check_children_kind(ast, node, &[1], &[Kind::TaskNetworkDef])?;
        }
        Kind::Task => {
            check_min_children_count(node.children().len(), 1, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::TaskSymbol])?;
            check_children_kind_from(ast, node, 1, &[Kind::Variable, Kind::Constant])?;
        }
        Kind::PreconditionDef
        | Kind::MethodPreconditionDef => {
            check_children_count(node.children().len(), 1, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::Or])?;
        }
        Kind::EffectDef => {
            check_children_count(node.children().len(), 1, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::And])?;
        }
        Kind::FunctionTerm => {
            check_min_children_count(node.children().len(), 1, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::FunctionSymbol])?;
            check_children_kind_from(ast, node, 1, &[Kind::Variable, Kind::Constant])?;
        }
        Kind::AtomicFormula => {
            check_min_children_count(node.children().len(), 1, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::Predicate])?;
            check_children_kind_from(ast, node, 1, &[Kind::Variable, Kind::Constant])?;
        }
        Kind::Or
        | Kind::And => {
            check_all_children_kind(ast, node, EXPRESSION)?;
        }
        | Kind::Not
        | Kind::AtStart
        | Kind::AtEnd
        | Kind::Overall
        | Kind::Always
        | Kind::Sometime
        | Kind::AtMostOnce
        | Kind::Goal
        | Kind::Constraints
        | Kind::Metric => {
            check_min_children_count(node.children().len(), 1, node.kind())?;
            check_children_kind(ast, node, &[0], EXPRESSION)?;
        }
        | Kind::Imply
        | Kind::When
        | Kind::SometimeAfter
        | Kind::SometimeBefore => {
            check_min_children_count(node.children().len(), 2, node.kind())?;
            check_children_kind(ast, node, &[0], EXPRESSION)?;
            check_children_kind(ast, node, &[1], EXPRESSION)?;
        }
        Kind::Forall
        | Kind::Exists => {
            check_min_children_count(node.children().len(), 2, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::TypedList])?;
            check_children_kind(ast, node, &[1], EXPRESSION)?;
        }
        Kind::Preference => {
            check_min_children_count(node.children().len(), 2, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::PrefName])?;
            check_children_kind(ast, node, &[1], EXPRESSION)?;
        }
        Kind::FComp => {
            check_children_count_range(node.children().len(), 1, 2, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::Number, FComp, FunctionTerm, Variable, Constant])?;
            check_children_kind(ast, node, &[1], &[FComp, FunctionTerm, Variable, Constant])?;
        }
        Kind::Assign => {
            check_min_children_count(node.children().len(), 2, node.kind())?;
            check_children_kind(ast, node, &[0], &[FunctionTerm])?;
            check_children_kind(ast, node, &[0], &[FComp, Variable, Constant, FunctionTerm])?; // Todo: Adding undefined
        }
        Kind::Operation => {
            check_children_count_range(node.children().len(), 1, 2, node.kind())?;
            check_children_kind(ast, node, &[0], &[FComp])?;
            check_children_kind(ast, node, &[1], &[FComp])?;
        }
        Kind::Within
        | Kind::HoldAfter => {
            check_children_count(node.children().len(), 2, node.kind())?;
            check_children_kind(ast, node, &[0], &[Number])?;
            check_children_kind(ast, node, &[1], EXPRESSION)?;
        }
        Kind::AlwaysWithin => {
            check_children_count(node.children().len(), 3, node.kind())?;
            check_children_kind(ast, node, &[0], &[Number])?;
            check_children_kind(ast, node, &[1], EXPRESSION)?;
            check_children_kind(ast, node, &[2], EXPRESSION)?;
        }
        Kind::HoldDuring => {
            check_children_count(node.children().len(), 3, node.kind())?;
            check_children_kind(ast, node, &[0], &[Number])?;
            check_children_kind(ast, node, &[1], &[Number])?;
            check_children_kind(ast, node, &[2], EXPRESSION)?;
        }
        Kind::Init => {
            check_children_count(node.children().len(), 1, node.kind())?;
            check_children_kind(ast, node, &[0], &[And])?;
            let init_elements = get_node(ast, node.kind(), 0)?;
            check_all_children_kind(ast, init_elements, &[TimedInitialLiteral, FComp, AtomicFormula, Not])?;
            // Todo: check that not contains only atomic formula
        }
        Kind::TimedInitialLiteral => {
            check_children_count(node.children().len(), 2, node.kind())?;
            check_children_kind(ast, node, &[0], &[Number])?;
            check_children_kind(ast, node, &[1], &[FComp, Not])?;
            // Todo: check that not contains only atomic formula
        }
        Kind::DerivedDef => {
            check_children_count(node.children().len(), 2, node.kind())?;
            check_children_kind(ast, node, &[0], &[AtomicFormulaSkeleton])?;
            check_children_kind(ast, node, &[1], EXPRESSION)?;
        }

        Kind::OrderedSubtaskDef
        | Kind::PartiallyOrderedSubtaskDef => {
            check_children_count(node.children().len(), 1, node.kind())?;
            check_children_kind(ast, node, &[0], &[And])?;
            let tasks = get_node(ast, node.kind(), 0)?;
            check_all_children_kind(ast, tasks, &[TaggedTask, Task])?;
        }
        Kind::TaggedTask => {
            check_children_count(node.children().len(), 2, node.kind())?;
            check_children_kind(ast, node, &[0], &[TaskID])?;
            check_children_kind(ast, node, &[1], &[Task])?;
        }
        Kind::TaskOrderingConstraintDef => {
            check_children_count(node.children().len(), 1, node.kind())?;
            check_children_kind(ast, node, &[0], &[And])?;
            let ordering = get_node(ast, node.kind(), 0)?;
            check_all_children_kind(ast, ordering, &[TaskOrderingConstraint])?;
        }
        Kind::TaskOrderingConstraint => {
            check_children_count(node.children().len(), 2, node.kind())?;
            check_children_kind(ast, node, &[0], &[TaskID])?;
            check_children_kind(ast, node, &[1], &[TaskID])?;
        }
        Kind::TaskLogicalConstraintDef => {
            check_all_children_kind(ast, node, EXPRESSION)?;
        }
        Kind::TaskNetworkDef => {
            check_children_count_range(node.children().len(), 0, 3, node.kind())?;
            check_children_kind(ast, node, &[0], &[OrderedSubtaskDef, PartiallyOrderedSubtaskDef])?;
            check_children_kind(ast, node, &[0, 1], &[TaskOrderingConstraintDef])?;
            check_children_kind(ast, node, &[0, 1, 2], &[TaskOrderingConstraintDef])?;
        }
        Kind::InitialTaskNetwork => {
            check_children_count(node.children().len(),2, node.kind())?;
            check_children_kind(ast, node, &[0], &[ParametersDef])?;
            check_children_kind(ast, node, &[1], &[TaskNetworkDef])?;
        }
        Kind::TaskDef => {
            check_children_count(node.children().len(),2, node.kind())?;
            check_children_kind(ast, node, &[0], &[TaskSymbol])?;
            check_children_kind(ast, node, &[1], &[ParametersDef])?;
        }
        Kind::TotalTime => {
            check_children_count(node.children().len(),0, node.kind())?;
        }
        Kind::IsViolated => {
            check_children_count(node.children().len(),1, node.kind())?;
            check_children_kind(ast, node, &[0], &[PrefName])?;
        }
        Kind::Length => {
            check_children_count_range(node.children().len(), 0, 2, node.kind())?;
            check_all_children_kind(ast, node, &[Serial, Parallel])?;
        }
        Serial | Parallel => {
            check_children_count(node.children().len(),1, node.kind())?;
            check_children_kind(ast, node, &[0], &[Number])?;
        }
        Kind::DurativeActionDef => {
            check_children_count(children_ids.len(), 3, node.kind())?;
            check_children_kind(ast, node, &[0], &[DASymbol])?;
            check_children_kind(ast, node, &[1], &[ParametersDef])?;
            check_children_kind(ast, node, &[2], &[DADefBody])?;

        }
        DADefBody => {
            check_children_count(node.children().len(), 3, node.kind())?;
            check_children_kind(ast, node, &[0], EXPRESSION)?;
            check_children_kind(ast, node, &[1], EXPRESSION)?;
            check_children_kind(ast, node, &[2], EXPRESSION)?;
        }
        Kind::Domain => {}
        Kind::Problem => {}

        _ => {
            println!("ERROR: {}", node.kind());
        }

    }

    for child_id in children_ids {
        let child_node = get_node(ast, node.kind(), child_id.as_usize())?;
        validate_from(child_node, ast)?;
    }

    Ok(())
}


pub fn check_typed_list(node: &AstNode, ast: &Ast, expected: &[AstKind]) -> Result<(), ValidationError> {
    let children_ids = node.children();
    match node.kind() {
       Kind::TypedList => {
            check_all_children_kind(ast, node, &[Kind::TypedItem])?;
        }
        Kind::TypedItem => {
            check_children_count_range(node.children().len(), 1, 2, node.kind())?;
            check_children_kind(ast, node, &[0], &[Kind::TypedItemElements])?;
            check_children_kind(ast, node, &[1], &[Kind::Type])?;
        }
        Kind::TypedItemElements => {
            check_min_children_count(node.children().len(), 1, node.kind())?;
            check_all_children_kind(ast, node, expected)?;
        }
        _ => {
            validate_from(node, ast)?;
        }
    }
    for child_id in children_ids {
        let child_node = get_node(ast, node.kind(), child_id.as_usize())?;
        check_typed_list(child_node, ast, expected)?;
    }

    Ok(())
}
