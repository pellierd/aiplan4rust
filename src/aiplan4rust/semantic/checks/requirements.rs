//! Module for validating PDDL requirement compliance.
//!
//! This module ensures that all features used in the PDDL source (represented by
//! `required_requirements`) have been explicitly enabled in the `:requirements`
//! section (represented by `declared_requirements`).

use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::support::diagnostic::{Diagnostic, DiagnosticManager, Provider};
use crate::aiplan4rust::support::lang::LiteralId;
use crate::aiplan4rust::support::lang::Requirement;
use crate::aiplan4rust::syntax::ast::tree::{Node, NodeId};
use crate::aiplan4rust::syntax::ast::AstNode;
use std::collections::{HashMap, HashSet};

/// Validates that all requirements triggered by the AST are covered by the declared ones.
///
/// This function cross-references the atomic requirements extracted during semantic analysis
/// (`requirement_triggers`) against the set of declared requirements, including those
/// implied by meta-requirements like `:adl` (via `compute_effective_requirements`).
///
/// # Reporting Behavior
/// To prevent diagnostic flooding, this function **only reports a warning for the first
/// occurrence** of each missing requirement. While only the first problematic node is
/// flagged in the `DiagnosticManager`, the complete list of all triggering nodes remains
/// available within the `context.requirement_triggers()` for tools that require
/// exhaustive mapping.
///
/// # Parameters
/// - `context`: The semantic [`CheckContext`] containing the AST, requirements, and triggers.
/// - `diagnostic_manager`: A mutable reference to collect the reported warnings.
///
/// # Returns
/// - `Ok(true)` if all used features are covered by declarations.
/// - `Ok(false)` if violations were found (one warning per missing requirement type).
/// - `Err(SemanticCheckError)` if an AST node cannot be resolved.
pub fn check_requirements(
    context: &CheckContext,
    requirements: &HashMap<Requirement, Vec<NodeId>>,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    let mut checked = true;

    // 1. Resolve total coverage (Explicit + Implicit requirements)
    let effective_capabilities = Requirement::closure(context.declared_requirements());

    let mut reported_in_this_pass = HashSet::new();
    let mut deprecated_reported = HashSet::new();

    // 2. Cross-reference atomic triggers against effective capabilities
    for (req, nodes) in requirements {
        // --- GESTION DES DÉPRÉCIATIONS ---
        if req.is_deprecated() && deprecated_reported.insert(req.clone()) {
            if let Some(first_node_id) = nodes.first() {
                let node = context.syntax_tree().try_node(*first_node_id)?;
                report_warning_deprecated_requirement(
                    node,
                    context.source(),
                    context.provider(),
                    diagnostic_manager,
                    req,
                );
            }
        }

        // --- GESTION DES VIOLATIONS ---
        if !effective_capabilities.contains(req) {
            checked = false;
            if let Some(first_node_id) = nodes.first() {
                if reported_in_this_pass.insert(req.clone()) {
                    let node = context.syntax_tree().try_node(*first_node_id)?;
                    report_warning_requirement_violation(
                        node,
                        context.declared_requirements(),
                        context.source(),
                        context.provider(),
                        diagnostic_manager,
                        vec![req.clone()],
                    );
                }
            }
        }
    }

    Ok(checked)
}

/// Reports a warning for the use of a deprecated requirement.
///
/// # Parameters
/// - `node`: The [`AstNode`] where the deprecated feature is used.
/// - `source`: The interned source identifier.
/// - `provider`: The analysis phase reporting this.
/// - `diagnostic_manager`: The manager collecting the diagnostic.
/// - `requirement`: The specific [`Requirement`] that is obsolete.
fn report_warning_deprecated_requirement(
    node: &AstNode,
    source: LiteralId,
    provider: Provider,
    diagnostic_manager: &mut DiagnosticManager,
    requirement: &Requirement,
) {
    let warning = Diagnostic::warning_deprecated_requirement(
        requirement.clone(),
        provider,
        source,
        node.span(),
    );
    diagnostic_manager.report(warning);
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
/// - `source`: The [`LiteralId`] representing the name or label of the source file or input.
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
    source: LiteralId,
    provider: Provider,
    diagnostic_manager: &mut DiagnosticManager,
    required: Vec<Requirement>,
) -> bool {
    if required.iter().any(|r| !requirements.contains(r)) {
        let warning = Diagnostic::warning_missing_requirement(
            node.kind(),
            required,
            provider,
            source,
            node.span(),
        );
        diagnostic_manager.report(warning);
        false
    } else {
        true
    }
}
