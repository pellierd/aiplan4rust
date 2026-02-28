use std::collections::HashSet;
use crate::aiplan4rust::lang::Requirement;
use crate::aiplan4rust::semantic::SemanticError;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::Tree;

/// Extracts all semantic requirements from the syntax tree.
///
/// This assumes all requirements are grouped under a single `RequireDef` node.
///
/// # Arguments
/// - `syntax_tree`: Reference to the arena-based syntax tree.
///
/// # Returns
/// A set of all declared and implied `Requirement`s.
///
/// # Errors
/// Returns `SemanticError` if traversing the tree fails.
pub fn extract_declared_requirements(
    syntax_tree: &Tree<AstNode>,
) -> Result<HashSet<Requirement>, SemanticError> {
    let mut requirements = HashSet::new();

    // Find the first RequireDef node
    let mut requirement_def_node = None;
    for node in syntax_tree.preorder().values() {
        if matches!(node.kind(), AstKind::RequireDef) {
            requirement_def_node = Some(node);
            break;
        }
    }

    if let Some(req_def) = requirement_def_node {
        // Collect all Requirement children
        for child in req_def.children() {
            let node = syntax_tree.try_node(*child)?;
            if matches!(node.kind(), AstKind::Requirement) {
                if let Ok(req) = node.try_requirement() {
                    requirements.extend(req.imply());
                }
            }
        }
    }

    Ok(requirements)
}

/// Extracts the set of semantic requirements actually required by the given syntax tree.
///
/// Iterates over all nodes in the syntax tree and collects `Requirement`s corresponding
/// to the features used in the AST.
///
/// # Arguments
///
/// * `syntax_tree` - The syntax tree to analyze.
///
/// # Returns
///
/// A `HashSet<Requirement>` containing all requirements that are actually used.
pub fn extract_required_requirements(
    syntax_tree: &Tree<AstNode>,
) -> HashSet<Requirement> {
    let mut needed: HashSet<Requirement> = HashSet::new();

    for node in syntax_tree.preorder().values() {
        match node.kind() {
            // :typing
            AstKind::TypesDef | AstKind::Type | AstKind::PrimitiveType => {
                needed.insert(Requirement::Typing);
            }

            // :fluents (:numeric-fluents and :object-fluents)
            AstKind::FunctionsDef
            | AstKind::Function
            | AstKind::AtomicFunctionSkeleton
            | AstKind::FunctionSymbol
            | AstKind::Number
            | AstKind::Metric
            | AstKind::TotalTime
            | AstKind::Arithmetic => {
                needed.insert(Requirement::NumericFluents);
                // Optional: ObjectFluents can be added if needed
                // needed.insert(Requirement::ObjectFluents);
            }

            // :durative-actions
            AstKind::DurativeActionDef
            | AstKind::DASymbol
            | AstKind::DADefBody
            | AstKind::AtStart
            | AstKind::AtEnd
            | AstKind::Overall
            | AstKind::TimedInitialLiteral => {
                needed.insert(Requirement::DurativeActions);
            }

            // :htn / hierarchy
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
                needed.insert(Requirement::Hierarchy);
            }

            AstKind::MethodPreconditionDef => {
                needed.insert(Requirement::Hierarchy);
                needed.insert(Requirement::MethodPreconditions);
            }

            // :derived-predicates
            AstKind::DerivedDef => {
                needed.insert(Requirement::DerivedPredicates);
            }

            // :negative-preconditions
            AstKind::Not => {
                needed.insert(Requirement::NegativePreconditions);
            }

            // :disjunctive-preconditions + :negative-preconditions
            AstKind::Imply => {
                needed.insert(Requirement::DisjunctivePreconditions);
                needed.insert(Requirement::NegativePreconditions);
            }

            // :universal-preconditions
            AstKind::Forall => {
                needed.insert(Requirement::UniversalPreconditions);
            }

            // :existential-preconditions
            AstKind::Exists => {
                needed.insert(Requirement::ExistentialPreconditions);
            }

            // :preferences
            AstKind::Preference | AstKind::PrefName | AstKind::IsViolated => {
                needed.insert(Requirement::Preferences);
            }

            // :conditional-effects
            AstKind::When => {
                needed.insert(Requirement::ConditionalEffects);
            }

            // :constraints
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
                needed.insert(Requirement::Constraints);
            }

            // :strips actions
            AstKind::Parallel => {
                needed.insert(Requirement::Strips);
            }

            // These nodes do not imply a requirement
            AstKind::ObjectsDef
            | AstKind::Domain
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
            | AstKind::AtomicFormulaSkeleton
            | AstKind::ActionDef
            | AstKind::ParametersDef
            | AstKind::ActionDefBody
            | AstKind::PreconditionDef
            | AstKind::EffectDef
            | AstKind::AtomicFormula
            | AstKind::And
            | AstKind::Object
            | AstKind::Variable
            | AstKind::Init
            | AstKind::Goal
            | AstKind::Length
            | AstKind::Serial
            | AstKind::Error
            | AstKind::Requirement
            | AstKind::Comparison
            | AstKind::Assignment
            | AstKind::Or => {
                // No requirement associated
            }
        }
    }

    needed
}
