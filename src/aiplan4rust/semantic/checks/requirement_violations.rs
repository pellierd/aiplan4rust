use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::lang::Requirement::{
    ConditionalEffects, DerivedPredicates, DisjunctivePreconditions, DurativeActions, Equality,
    ExistentialPreconditions, Fluents, NegativePreconditions, NumericFluents, ObjectFluents,
    Preferences, Typing, UniversalPreconditions,
};
use crate::aiplan4rust::lang::BinaryComp;
use crate::aiplan4rust::lang::Requirement;

use std::collections::HashSet;
use crate::aiplan4rust::interner::Literal;
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::syntax::ast::{AstNode, AstKind};
use crate::aiplan4rust::syntax::tree::SyntaxNode;

pub fn check_requirement_violations(
    context: &CheckContext,
    requirements: &HashSet<Requirement>,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    let mut checked = true;

    for (index, node) in context.syntax_tree().preorder().with_id() {
        match node.kind() {
            AstKind::PrimitiveType | AstKind::TypesDef => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![Typing],
                );
            }

            AstKind::FunctionsDef | AstKind::FunctionTerm => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![Fluents, NumericFluents, ObjectFluents],
                );
            }

            AstKind::Number => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![NumericFluents],
                );
            }

            AstKind::DurativeActionDef => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![DurativeActions],
                );
            }

            AstKind::DerivedDef => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![DerivedPredicates],
                );
            }

            AstKind::Or => {
                let parent = context.syntax_tree().get_parent(index).unwrap();
                if parent.kind() != AstKind::MethodPreconditionDef
                    && parent.kind() != AstKind::PreconditionDef
                    && parent.kind() != AstKind::EffectDef
                {
                    checked &= report_requirement_violation(
                        node,
                        requirements,
                        context.source_name(),
                        source,
                        diagnostic_manager,
                        vec![DisjunctivePreconditions],
                    );
                }
            }

            AstKind::Not => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![NegativePreconditions],
                );
            }

            AstKind::Imply => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![DisjunctivePreconditions],
                );
            }

            AstKind::Forall => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![UniversalPreconditions],
                );
            }

            AstKind::Exists => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![ExistentialPreconditions],
                );
            }

            AstKind::Preference => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![Preferences],
                );
            }

            AstKind::When => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![ConditionalEffects],
                );
            }

            AstKind::FComp => {
                match node.try_binary_comp()? {
                    BinaryComp::Equal => {
                        checked &= report_requirement_violation(
                            node,
                            requirements,
                            context.source_name(),
                            source,
                            diagnostic_manager,
                            vec![Equality, Fluents, NumericFluents,ObjectFluents],
                        );
                    }
                    _ => {
                        checked &= report_requirement_violation(
                            node,
                            requirements,
                            context.source_name(),
                            source,
                            diagnostic_manager,
                            vec![Fluents, NumericFluents,ObjectFluents],
                        );
                    }
                }
            }
            _ => {}
        }
    }
    Ok(checked)
}

/// Reports a requirement violation if one or more required features are not enabled.
///
/// This function checks whether all `required` [`Requirement`]s are present in the given set of
/// active `requirements`. If any required feature is missing, a [`DiagnosticKind::RequirementViolation`]
/// diagnostic is emitted, identifying the offending AST node and the missing requirements.
///
/// # Parameters
/// - `node`: The [`AstNode`] associated with the feature requiring specific requirements.
/// - `requirements`: The set of currently active or declared requirements in the context.
/// - `source`: The [`Literal`] representing the name or label of the source file or input.
/// - `provider`: The [`Provider`] indicating which analysis phase is reporting the violation.
/// - `diagnostic_manager`: The mutable reference to the [`DiagnosticManager`] collecting diagnostics.
/// - `required`: The list of [`Requirement`]s that must be satisfied for the feature to be valid.
///
/// # Returns
/// - `true` if all required features are present (i.e., no violation occurred).
/// - `false` if any required feature is missing and a diagnostic was emitted.
///
/// # Example
/// ```rust
/// let valid = report_requirement_violation(
///     node,
///     &requirements_set,
///     context.source_name(),
///     Provider::Normalizer,
///     &mut diagnostic_manager,
///     vec![Requirement::Typing],
/// );
/// if !valid {
///     return Err(NormalizationPassError::missing_requirement("Typing"));
/// }
/// ```
///
/// # See Also
/// - [`Requirement`]
/// - [`DiagnosticKind::RequirementViolation`]
/// - [`DiagnosticManager`]
fn report_requirement_violation(
    node: &AstNode,
    requirements: &HashSet<Requirement>,
    source: Literal,
    provider: Provider,
    diagnostic_manager: &mut DiagnosticManager,
    required: Vec<Requirement>,
) -> bool {
    if required.iter().any(|r| !requirements.contains(r)) {
        let error = Diagnostic::new(
            DiagnosticKind::RequirementViolation { node_kind: node.kind().clone(), required},
            provider,
            source,
            node.span().clone(),
        );
        diagnostic_manager.add_diagnostic(error);
        false
    } else {
        true
    }
}
