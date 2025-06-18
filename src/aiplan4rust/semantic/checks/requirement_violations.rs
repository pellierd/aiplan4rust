use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::elements::Requirement::{
    ConditionalEffects, DerivedPredicates, DisjunctivePreconditions, DurativeActions, Equality,
    ExistentialPreconditions, Fluents, NegativePreconditions, NumericFluents, ObjectFluents,
    Preferences, Typing, UniversalPreconditions,
};
use crate::aiplan4rust::syntax::elements::BinaryComp;
use crate::aiplan4rust::syntax::elements::Requirement;
use crate::aiplan4rust::syntax::ast_old::AstKindOld;
use std::collections::HashSet;
use crate::aiplan4rust::semantic::arena::ArenaAstNode;
use crate::aiplan4rust::semantic::SemanticContext;

pub fn check_requirement_violations(
    context: &SemanticContext,
    requirements: &HashSet<Requirement>,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let mut checked = true;

    for (index, node) in context.ast().preorder_with_index() {
        match node.kind() {
            AstKindOld::PrimitiveType(_) | AstKindOld::TypesDef => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![Typing],
                );
            }

            AstKindOld::FunctionsDef | AstKindOld::FunctionTerm => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![Fluents, NumericFluents, ObjectFluents],
                );
            }

            AstKindOld::Number(_) => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![NumericFluents],
                );
            }

            AstKindOld::DurativeActionDef => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![DurativeActions],
                );
            }

            AstKindOld::DerivedDef => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![DerivedPredicates],
                );
            }

            AstKindOld::Or => {
                let parent = context.ast().get_parent(index).unwrap();
                if *parent.kind() != AstKindOld::MethodPreconditionDef
                    && *parent.kind() != AstKindOld::PreconditionDef
                    && *parent.kind() != AstKindOld::EffectDef
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

            AstKindOld::Not => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![NegativePreconditions],
                );
            }

            AstKindOld::Imply => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![DisjunctivePreconditions],
                );
            }

            AstKindOld::Forall => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![UniversalPreconditions],
                );
            }

            AstKindOld::Exists => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![ExistentialPreconditions],
                );
            }

            AstKindOld::Preference => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![Preferences],
                );
            }

            AstKindOld::When => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    context.source_name(),
                    source,
                    diagnostic_manager,
                    vec![ConditionalEffects],
                );
            }

            AstKindOld::FComp(op) => {
                match op {
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
    node: &ArenaAstNode,
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
