use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::elements::Requirement::{
    ConditionalEffects, DerivedPredicates, DisjunctivePreconditions, DurativeActions, Equality,
    ExistentialPreconditions, Fluents, NegativePreconditions, NumericFluents, ObjectFluents,
    Preferences, Typing, UniversalPreconditions,
};
use crate::aiplan4rust::syntax::elements::BinaryComp;
use crate::aiplan4rust::syntax::elements::Requirement;
use crate::aiplan4rust::syntax::ast::AstKind;
use crate::aiplan4rust::semantic::hir::{HirNode, HirTree};
use std::collections::HashSet;

pub fn check_requirement_violations(
    syntax_tree: &HirTree,
    requirements: &HashSet<Requirement>,
    source: Provider,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let mut checked = true;

    for (index, node) in syntax_tree.iter() {
        match node.kind() {
            AstKind::PrimitiveType(_) | AstKind::TypesDef => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![Typing],
                );
            }

            AstKind::FunctionsDef | AstKind::FunctionTerm => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![Fluents, NumericFluents, ObjectFluents],
                );
            }

            AstKind::Number(_) => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![NumericFluents],
                );
            }

            AstKind::DurativeActionDef => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![DurativeActions],
                );
            }

            AstKind::DerivedDef => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![DerivedPredicates],
                );
            }

            AstKind::Or => {
                let parent = syntax_tree.get_parent(*index).unwrap();
                if *parent.kind() != AstKind::MethodPreconditionDef
                    && *parent.kind() != AstKind::PreconditionDef
                    && *parent.kind() != AstKind::EffectDef
                {
                    checked &= report_requirement_violation(
                        node,
                        requirements,
                        syntax_tree.filename(),
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
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![NegativePreconditions],
                );
            }

            AstKind::Imply => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![DisjunctivePreconditions],
                );
            }

            AstKind::Forall => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![UniversalPreconditions],
                );
            }

            AstKind::Exists => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![ExistentialPreconditions],
                );
            }

            AstKind::Preference => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![Preferences],
                );
            }

            AstKind::When => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![ConditionalEffects],
                );
            }

            AstKind::FComp(op) => {
                match op {
                    BinaryComp::Equal => {
                        checked &= report_requirement_violation(
                            node,
                            requirements,
                            syntax_tree.filename(),
                            source,
                            diagnostic_manager,
                            vec![Equality, Fluents, NumericFluents,ObjectFluents],
                        );
                    }
                    _ => {
                        checked &= report_requirement_violation(
                            node,
                            requirements,
                            syntax_tree.filename(),
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
    node: &HirNode,
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
