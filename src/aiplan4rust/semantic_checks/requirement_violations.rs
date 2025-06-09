use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, DiagnosticSource};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::parser::elements::Requirement::{
    ConditionalEffects, DerivedPredicates, DisjunctivePreconditions, DurativeActions, Equality,
    ExistentialPreconditions, Fluents, NegativePreconditions, NumericFluents, ObjectFluents,
    Preferences, Typing, UniversalPreconditions,
};
use crate::aiplan4rust::parser::elements::BinaryComp;
use crate::aiplan4rust::parser::elements::Requirement;
use crate::aiplan4rust::parser::syntax_tree::SyntaxNodeKind;
use crate::aiplan4rust::analyser::{AnnotatedSyntaxNode, AnnotatedSyntaxTree};
use std::collections::HashSet;

pub fn check_requirement_violations(
    syntax_tree: &AnnotatedSyntaxTree,
    requirements: &HashSet<Requirement>,
    source: DiagnosticSource,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let mut checked = true;

    for (index, node) in syntax_tree.iter() {
        match node.kind() {
            SyntaxNodeKind::PrimitiveType(_) | SyntaxNodeKind::TypesDef => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![Typing],
                );
            }

            SyntaxNodeKind::FunctionsDef | SyntaxNodeKind::FunctionTerm => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![Fluents, NumericFluents, ObjectFluents],
                );
            }

            SyntaxNodeKind::Number(_) => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![NumericFluents],
                );
            }

            SyntaxNodeKind::DurativeActionDef => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![DurativeActions],
                );
            }

            SyntaxNodeKind::DerivedDef => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![DerivedPredicates],
                );
            }

            SyntaxNodeKind::Or => {
                let parent = syntax_tree.get_parent(*index).unwrap();
                if *parent.kind() != SyntaxNodeKind::MethodPreconditionDef
                    && *parent.kind() != SyntaxNodeKind::PreconditionDef
                    && *parent.kind() != SyntaxNodeKind::EffectDef
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

            SyntaxNodeKind::Not => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![NegativePreconditions],
                );
            }

            SyntaxNodeKind::Imply => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![DisjunctivePreconditions],
                );
            }

            SyntaxNodeKind::Forall => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![UniversalPreconditions],
                );
            }

            SyntaxNodeKind::Exists => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![ExistentialPreconditions],
                );
            }

            SyntaxNodeKind::Preference => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![Preferences],
                );
            }

            SyntaxNodeKind::When => {
                checked &= report_requirement_violation(
                    node,
                    requirements,
                    syntax_tree.filename(),
                    source,
                    diagnostic_manager,
                    vec![ConditionalEffects],
                );
            }

            SyntaxNodeKind::FComp(op) => {
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
    node: &AnnotatedSyntaxNode,
    requirements: &HashSet<Requirement>,
    filename: &str,
    source: DiagnosticSource,
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
