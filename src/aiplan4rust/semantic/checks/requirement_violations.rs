use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::lang::Requirement::{
    ConditionalEffects, DerivedPredicates, DisjunctivePreconditions, DurativeActions, Equality,
    ExistentialPreconditions, Fluents, NegativePreconditions, NumericFluents, ObjectFluents,
    Preferences, Typing, UniversalPreconditions,
};
use crate::aiplan4rust::lang::BinaryComp;
use crate::aiplan4rust::lang::Requirement;

use std::collections::HashSet;
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::semantic::checks::CheckContext;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::tree::TreeNode;

pub fn check_requirement_violations(
    context: &CheckContext,
    requirements: &HashSet<Requirement>,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let mut checked = true;

    for (index, node) in context.ast().preorder_with_index() {
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
                let parent = context.ast().get_parent(index).unwrap();
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
fn report_requirement_violation(
    node: &AstArenaNode,
    requirements: &HashSet<Requirement>,
    filename: &str,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
    required: Vec<Requirement>,
) -> bool {
    if required.iter().any(|r| !requirements.contains(r)) {
        let error = Diagnostic::new(
            DiagnosticKind::RequirementViolation { node_kind: node.kind().clone(), required},
            source,
            filename.to_string(),
            node.span().clone(),
        );
        diagnostic_manager.add_diagnostic(error);
        false
    } else {
        true
    }
}
