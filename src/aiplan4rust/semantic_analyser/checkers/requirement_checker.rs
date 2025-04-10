use crate::aiplan4rust::error::{ErrorManager, ParserErrorKind, ParsingError};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::parser::elements::Requirement::{
    ConditionalEffects, DerivedPredicates, DisjunctivePreconditions, DurativeActions, Equality,
    ExistentialPreconditions, Fluents, NegativePreconditions, NumericFluents, ObjectFluents,
    Preferences, Typing, UniversalPreconditions,
};
use crate::aiplan4rust::parser::elements::{BinaryComp, Requirement};
use crate::aiplan4rust::parser::syntax_tree::SyntaxNodeKind;
use crate::aiplan4rust::semantic_analyser::{AnnotatedSyntaxNode, AnnotatedSyntaxTree};
use std::collections::HashSet;

pub fn check(
    syntax_tree: &AnnotatedSyntaxTree,
    requirements: &HashSet<Requirement>,
    errors: &mut ErrorManager,
) -> Result<bool, ParserInternalError> {
    let mut checked = true;

    for (index, node) in syntax_tree.iter() {
        match node.kind() {
            SyntaxNodeKind::PrimitiveType(_) | SyntaxNodeKind::TypesDef => {
                checked &= check_requirements(
                    node,
                    syntax_tree,
                    requirements,
                    errors,
                    &[Typing],
                    "Typing feature requires requirement",
                );
            }

            SyntaxNodeKind::FunctionsDef | SyntaxNodeKind::FunctionTerm => {
                checked &= check_requirements(
                    node,
                    syntax_tree,
                    requirements,
                    errors,
                    &[Fluents, NumericFluents, ObjectFluents],
                    "Functions require requirement",
                );
            }

            SyntaxNodeKind::Number(_) => {
                checked &= check_requirements(
                    node,
                    syntax_tree,
                    requirements,
                    errors,
                    &[NumericFluents],
                    "Number usage requires requirement",
                );
            }

            SyntaxNodeKind::DurativeActionDef => {
                checked &= check_requirements(
                    node,
                    syntax_tree,
                    requirements,
                    errors,
                    &[DurativeActions],
                    "Durative actions require requirement",
                );
            }

            SyntaxNodeKind::DerivedDef => {
                checked &= check_requirements(
                    node,
                    syntax_tree,
                    requirements,
                    errors,
                    &[DerivedPredicates],
                    "Derived predicates require requirement",
                );
            }

            SyntaxNodeKind::Or => {
                let parent = syntax_tree.get_parent(*index).unwrap();
                if *parent.kind() != SyntaxNodeKind::MethodPreconditionDef
                    && *parent.kind() != SyntaxNodeKind::PreconditionDef
                    && *parent.kind() != SyntaxNodeKind::EffectDef
                {
                    checked &= check_requirements(
                        node,
                        syntax_tree,
                        requirements,
                        errors,
                        &[DisjunctivePreconditions],
                        "Or expressions require requirement",
                    );
                }
            }

            SyntaxNodeKind::Not => {
                checked &= check_requirements(
                    node,
                    syntax_tree,
                    requirements,
                    errors,
                    &[NegativePreconditions],
                    "Negative expressions require requirement",
                );
            }

            SyntaxNodeKind::Imply => {
                checked &= check_requirements(
                    node,
                    syntax_tree,
                    requirements,
                    errors,
                    &[DisjunctivePreconditions],
                    "Imply expressions require requirement",
                );
            }

            SyntaxNodeKind::Forall => {
                checked &= check_requirements(
                    node,
                    syntax_tree,
                    requirements,
                    errors,
                    &[UniversalPreconditions],
                    "Universal expressions require requirement",
                );
            }

            SyntaxNodeKind::Exists => {
                checked &= check_requirements(
                    node,
                    syntax_tree,
                    requirements,
                    errors,
                    &[ExistentialPreconditions],
                    "Existential expressions require requirement",
                );
            }

            SyntaxNodeKind::Preference => {
                checked &= check_requirements(
                    node,
                    syntax_tree,
                    requirements,
                    errors,
                    &[Preferences],
                    "Preferences require requirement",
                );
            }

            SyntaxNodeKind::When => {
                checked &= check_requirements(
                    node,
                    syntax_tree,
                    requirements,
                    errors,
                    &[ConditionalEffects],
                    "Conditional effects require requirement",
                );
            }

            SyntaxNodeKind::FComp(op) => {
                match op {
                    BinaryComp::Equal => {
                        // Il faut au moins Equality OU un des trois autres
                        if !requirements.contains(&Equality)
                            && !requirements.contains(&Fluents)
                            && !requirements.contains(&NumericFluents)
                            && !requirements.contains(&ObjectFluents)
                        {
                            add_requirement_error(
                                node,
                                &syntax_tree.filename(),
                                errors,
                                Equality,
                                "Equality comparison requires either Equality or a fluents-related requirement",
                            );
                            checked = false;
                        }
                    }
                    _ => {
                        // Tous les autres opérateurs nécessitent un fluent numérique ou objet
                        if !requirements.contains(&Fluents)
                            && !requirements.contains(&NumericFluents)
                            && !requirements.contains(&ObjectFluents)
                        {
                            add_requirement_error(
                                node,
                                &syntax_tree.filename(),
                                errors,
                                NumericFluents,
                                "Comparison operator requires a numeric or fluent-related requirement",
                            );
                            checked = false;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(checked)
}
fn add_requirement_error(
    node: &AnnotatedSyntaxNode,
    filename: &str,
    errors: &mut ErrorManager,
    requirement: Requirement,
    message: &str,
) {
    let (line, column) = node.span().start_position();
    let content = format!("{} '{}'.", message, requirement);
    let error = ParsingError::new(
        ParserErrorKind::ParseError,
        Some(filename.to_string()),
        line,
        column,
        content,
    );
    errors.add_error(error);
}

fn check_requirements(
    node: &AnnotatedSyntaxNode,
    syntax_tree: &AnnotatedSyntaxTree,
    requirements: &HashSet<Requirement>,
    errors: &mut ErrorManager,
    required: &[Requirement],
    message: &str,
) -> bool {
    if required.iter().any(|r| !requirements.contains(r)) {
        add_requirement_error(
            node,
            &syntax_tree.filename(),
            errors,
            required[0].clone(),
            message,
        );
        false
    } else {
        true
    }
}
