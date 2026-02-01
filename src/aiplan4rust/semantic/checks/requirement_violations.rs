use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager, Provider};
use crate::aiplan4rust::lang::Requirement::{
    ConditionalEffects, DerivedPredicates, DisjunctivePreconditions, DurativeActions, Equality,
    ExistentialPreconditions, Fluents, NegativePreconditions, NumericFluents, ObjectFluents,
    Preferences, Typing, UniversalPreconditions,
};
use crate::aiplan4rust::lang::{BinaryComp, LiteralID};
use crate::aiplan4rust::lang::Requirement;

use std::collections::HashSet;
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::syntax::ast::{AstNode, AstKind};
use crate::aiplan4rust::tree::Node;

pub fn check_requirement_violations(
    context: &CheckContext,
    requirements: &HashSet<Requirement>,
    provider: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    let mut checked = true;

    for (index, node) in context.syntax_tree().preorder().with_id() {
        match node.kind() {
            AstKind::PrimitiveType | AstKind::TypesDef => {
                checked &= report_warning_requirement_violation(
                    node,
                    requirements,
                    context.source_id(),
                    provider,
                    diagnostic_manager,
                    vec![Typing],
                );
            }

            AstKind::FunctionsDef | AstKind::FunctionTerm => {
                checked &= report_warning_requirement_violation(
                    node,
                    requirements,
                    context.source_id(),
                    provider,
                    diagnostic_manager,
                    vec![Fluents, NumericFluents, ObjectFluents],
                );
            }

            AstKind::Number => {
                checked &= report_warning_requirement_violation(
                    node,
                    requirements,
                    context.source_id(),
                    provider,
                    diagnostic_manager,
                    vec![NumericFluents],
                );
            }

            AstKind::DurativeActionDef => {
                checked &= report_warning_requirement_violation(
                    node,
                    requirements,
                    context.source_id(),
                    provider,
                    diagnostic_manager,
                    vec![DurativeActions],
                );
            }

            AstKind::DerivedDef => {
                checked &= report_warning_requirement_violation(
                    node,
                    requirements,
                    context.source_id(),
                    provider,
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
                    checked &= report_warning_requirement_violation(
                        node,
                        requirements,
                        context.source_id(),
                        provider,
                        diagnostic_manager,
                        vec![DisjunctivePreconditions],
                    );
                }
            }

            AstKind::Not => {
                checked &= report_warning_requirement_violation(
                    node,
                    requirements,
                    context.source_id(),
                    provider,
                    diagnostic_manager,
                    vec![NegativePreconditions],
                );
            }

            AstKind::Imply => {
                checked &= report_warning_requirement_violation(
                    node,
                    requirements,
                    context.source_id(),
                    provider,
                    diagnostic_manager,
                    vec![DisjunctivePreconditions],
                );
            }

            AstKind::Forall => {
                checked &= report_warning_requirement_violation(
                    node,
                    requirements,
                    context.source_id(),
                    provider,
                    diagnostic_manager,
                    vec![UniversalPreconditions],
                );
            }

            AstKind::Exists => {
                checked &= report_warning_requirement_violation(
                    node,
                    requirements,
                    context.source_id(),
                    provider,
                    diagnostic_manager,
                    vec![ExistentialPreconditions],
                );
            }

            AstKind::Preference => {
                checked &= report_warning_requirement_violation(
                    node,
                    requirements,
                    context.source_id(),
                    provider,
                    diagnostic_manager,
                    vec![Preferences],
                );
            }

            AstKind::When => {
                checked &= report_warning_requirement_violation(
                    node,
                    requirements,
                    context.source_id(),
                    provider,
                    diagnostic_manager,
                    vec![ConditionalEffects],
                );
            }

            AstKind::FComp => {
                match node.try_binary_comp()? {
                    BinaryComp::Equal => {
                        checked &= report_warning_requirement_violation(
                            node,
                            requirements,
                            context.source_id(),
                            provider,
                            diagnostic_manager,
                            vec![Equality, Fluents, NumericFluents,ObjectFluents],
                        );
                    }
                    _ => {
                        checked &= report_warning_requirement_violation(
                            node,
                            requirements,
                            context.source_id(),
                            provider,
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
/// - `source`: The [`LiteralID`] representing the name or label of the source file or input.
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
fn report_warning_requirement_violation(
    node: &AstNode,
    requirements: &HashSet<Requirement>,
    source_id: LiteralID,
    provider: Provider,
    diagnostic_manager: &mut DiagnosticManager,
    required: Vec<Requirement>,
) -> bool {
    if required.iter().any(|r| !requirements.contains(r)) {
        let warning = Diagnostic::warning_missing_requirement(
            node.kind().clone(),
            required,
            provider,
            source_id,
            node.span().clone(),
        );
        diagnostic_manager.add_diagnostic(warning);
        false
    } else {
        true
    }
}
