use std::fmt;
use std::fmt::Formatter;
use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};

use crate::aiplan4rust::tree::{SyntaxContent, Node, Tree};
use crate::aiplan4rust::syntax::lexer::token::{ORDER, TOTAL_TIME};
use crate::aiplan4rust::syntax::{write_indent, SyntaxInternerDisplay};
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::syntax::ast::renderer::syntax::{quantifiers, task, typed_list};

pub fn render(
    node: &AstNode,
    f: &mut Formatter<'_>,
    arena: &Tree<AstNode>,
    interner: &StringInterner,
) -> fmt::Result {
    render_with_indent(node, f, arena, interner, 0)?;
    Ok(())
}

pub fn render_with_indent(
    node: &AstNode,
    f: &mut Formatter<'_>,
    arena: &Tree<AstNode>,
    interner: &StringInterner,
    indent: usize,
) -> fmt::Result {

    match node.kind() {
        AstKind::Domain => {
            // Write the opening line with base indentation
            write_indent(f, indent)?;
            write!(f, "(define (domain ")?;

            // Domain name (mandatory)
            if let Some(name_id) = node.children().get(0) {
                if let Some(name_node) = arena.get_node(*name_id) {
                    name_node.fmt_syntax_with_indent(f, arena, interner, 0)?;
                } else {
                    write!(f, "<invalid-domain-name>")?;
                }
            } else {
                write!(f, "<missing-domain-name>")?;
            }

            write!(f, ")")?;

            // Other children (optional), starting from index 1
            for child_id in node.children().iter().skip(1) {
                if let Some(child_node) = arena.get_node(*child_id) {
                    writeln!(f)?;
                    // Recurse with increased indentation
                    child_node.fmt_syntax_with_indent(
                        f,
                        arena,
                        interner,
                        indent + 1,
                    )?;
                }
            }

            // Closing parenthesis of the domain
            writeln!(f)?;
            write_indent(f, indent)?;
            write!(f, ")\n")?;

            Ok(())
        }

        AstKind::Problem => {
            let children = node.children();

            // Opening line with base indentation
            write_indent(f, indent)?;
            write!(f, "(define (problem ")?;

            // Problem name (mandatory)
            if let Some(name_id) = children.get(0) {
                if let Some(name_node) = arena.get_node(*name_id) {
                    name_node.fmt_syntax(f, arena, interner)?;
                } else {
                    write!(f, "<invalid-problem-name>")?;
                }
            } else {
                write!(f, "<missing-problem-name>")?;
            }
            write!(f, ")")?;

            // Domain name (second child)
            if let Some(domain_id) = children.get(1) {
                if let Some(domain_node) = arena.get_node(*domain_id) {
                    // On met le :domain à la ligne suivante
                    writeln!(f)?;
                    write_indent(f, indent + 1)?;
                    write!(f, "(:domain ")?;
                    domain_node.fmt_syntax(f, arena, interner)?;
                    write!(f, ")")?;
                } else {
                    writeln!(f)?;
                    write_indent(f, indent + 1)?;
                    write!(f, "<invalid-domain-name>")?;
                }
            } else {
                writeln!(f)?;
                write_indent(f, indent + 1)?;
                write!(f, "<missing-domain-name>")?;
            }

            // Remaining children (index >=2)
            for child_id in children.iter().skip(2) {
                writeln!(f)?;
                if let Some(child_node) = arena.get_node(*child_id) {
                    child_node.fmt_syntax_with_indent(
                        f,
                        arena,
                        interner,
                        indent + 1,
                    )?;
                } else {
                    write!(f, "<invalid-child-syntax>")?;
                }
            }

            // Closing line
            writeln!(f)?;
            write_indent(f, indent)?;
            write!(f, ")")?;

            Ok(())
        }

        AstKind::RequireDef => {
            // Indentation de la ligne d'ouverture
            write_indent(f, indent)?;
            write!(f, "(:requirements")?;

            // Chaque enfant est écrit sur la même ligne, séparé par un espace
            for child_id in node.children() {
                write!(f, " ")?;
                if let Some(child_node) = arena.get_node(*child_id) {
                    child_node.fmt_syntax(f, arena, interner)?;
                }
            }

            // Fermeture sur la même ligne
            writeln!(f, ")")
        }

        AstKind::TypesDef => {
            write_indent(f, indent)?;
            write!(f, "(")?;
            node.kind().fmt_syntax_with_interner(f, interner)?;
            writeln!(f)?;

            if let Some(child_id) = node.children().first() {
                if let Some(child_node) = arena.get_node(*child_id) {
                    // Pass indent + 1 to indent children 2 spaces more than the opening line
                    typed_list::render(child_node, f, arena, interner, true, indent + 1)?;
                }
            } else {
                // No children: optionally write a comment or just an empty indented line
                write_indent(f, indent + 1)?;
                writeln!(f, "<missing-typed-list>")?;
            }
            write_indent(f, indent)?;
            writeln!(f, ")")
        }

        AstKind::TypedList => typed_list::render(node, f, arena, interner, false, indent),

        AstKind::TypedItemElements => {
            // Write the indentation once before the list of elements
            write_indent(f, indent)?;

            // Iterate over children nodes
            for (i, child_id) in node.children().iter().enumerate() {
                // Separate elements with spaces
                if i > 0 {
                    write!(f, " ")?;
                }

                // Try to get the syntax
                if let Some(child_node) = arena.get_node(*child_id) {
                    write!(f, "{}",  child_node.content().to_syntax_string_with_interner(interner))?;
                } else {
                    write!(f, "<invalid_node>")?;
                }
            }

            Ok(())
        }

        AstKind::TypedItem => {

            // Handle first child (elements)
            if let Some(first_child_id) = node.get_child(0) {
                if let Some(first_child_node) = arena.get_node(first_child_id) {
                    first_child_node
                        .fmt_syntax_with_indent(f, arena, interner, indent)?;
                } else {
                    write_indent(f, indent)?;
                    write!(f, "<invalid_node>")?;
                }
            } else {
                write_indent(f, indent)?;
                write!(f, "<missing_child>")?;
            }

            // Handle optional second child (type_checker)
            if let Some(ty_id) = node.get_child(1) {
                write!(f, " - ")?;
                if let Some(ty_node) = arena.get_node(ty_id) {
                    ty_node.fmt_syntax_with_indent(f, arena, interner, indent)?
                } else {
                    write!(f, "<invalid_node>")?;
                }
            }
            Ok(())
        }

        AstKind::Type => {
            match node.arity() {
                0 => write!(f, ""),
                1 => {
                    let ty_node_id = node.children()[0];
                    if let Some(ty_node) = arena.get_node(ty_node_id) {
                        // Write indentation before the single type_checker
                        write_indent(f, indent)?;
                        // Recursively format the type_checker content with the current indentation
                        write!(f, "{}",  ty_node.content().to_syntax_string_with_interner(interner))
                    } else {
                        write_indent(f, indent)?;
                        write!(f, "<invalid_node>")
                    }
                }

                _ => {
                    // Write indentation and opening either keyword
                    write_indent(f, indent)?;
                    write!(f, "(either")?;

                    for child_id in node.children() {
                        write!(f, " ")?;
                        if let Some(ty_node) = arena.get_node(*child_id) {
                            // Format each type_checker content recursively, no extra indent here since on the same line
                            write!(f, "{}",  ty_node.content().to_syntax_string_with_interner(interner))?;
                        } else {
                            write!(f, "<invalid_node>")?;
                        }
                    }
                    write!(f, ")")
                }
            }
        }

        AstKind::ConstantsDef => {
            // Write the opening line with current indentation
            write_indent(f, indent)?;
            writeln!(f, "(:constants")?;

            // Format the constants list line by line with increased indentation for readability
            if let Some(child_id) = node.children().first() {
                if let Some(child_node) = arena.get_node(*child_id) {
                    // Assuming fmt_typed_list now takes indent parameter to handle multiline indent
                    typed_list::render(child_node, f, arena, interner, true, indent + 1)?;
                }
            }

            // Write the closing parenthesis aligned with the opening line
            write_indent(f, indent)?;
            writeln!(f, ")")
        }

        AstKind::PredicatesDef => {
            // Write the opening line with current indentation
            write_indent(f, indent)?;
            writeln!(f, "(:predicates")?;

            // Iterate over all children and format each predicate with increased indentation
            for child_id in node.children() {
                if let Some(child_node) = arena.get_node(*child_id) {
                    // Write indent for predicates' lines (one level deeper)
                    write_indent(f, indent + 1)?;
                    child_node.fmt_syntax(f, arena, interner)?;
                    writeln!(f)?;
                }
            }

            // Write the closing parenthesis aligned with the opening line
            write_indent(f, indent)?;
            writeln!(f, ")")
        }

        AstKind::FunctionsDef => {
            // Write the opening line with current indentation
            write_indent(f, indent)?;
            write!(f, "(:functions")?;

            // Iterate over children and format each function inline separated by spaces
            for child_id in node.children() {
                write!(f, " ")?;
                if let Some(child_node) = arena.get_node(*child_id) {
                    child_node.fmt_syntax(f, arena, interner)?;
                }
            }

            // Write the closing parenthesis aligned with the opening line
            write!(f, ")")
        }

        AstKind::AtomicFormulaSkeleton | AstKind::AtomicFunctionSkeleton => {
            // Write the opening parenthesis with the given indentation
            write_indent(f, indent)?;
            write!(f, "(")?;

            let children = node.children();

            for (i, child_id) in children.iter().enumerate() {
                if let Some(child_node) = arena.get_node(*child_id) {
                    // Write a space before this child only if:
                    // - it's not the first child
                    // - AND the child syntax's typed list is not empty (i.e., it has children)
                    if i > 0 && !child_node.children().is_empty() {
                        write!(f, " ")?;
                    }
                    // Format the child syntax recursively
                    child_node.fmt_syntax(f, arena, interner)?;
                } else {
                    // For invalid child nodes, write a space before it if not the first child
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    // Write an error placeholder for invalid nodes
                    write!(f, "<invalid>")?;
                }
            }

            // Write the closing parenthesis without any extra space
            write!(f, ")")
        }

        AstKind::TaskDef => {
            // Write opening line with indentation
            write_indent(f, indent)?;
            write!(f, "(:task ")?;

            let children = node.children();
            let mut idx = 0;

            // 1. The task name (mandatory)
            if let Some(&name_id) = children.get(idx) {
                if let Some(name_node) = arena.get_node(name_id) {
                    name_node.fmt_syntax(f, arena, interner)?;
                } else {
                    write!(f, "<invalid_name>")?;
                }
            } else {
                write!(f, "<missing_name>")?;
            }
            idx += 1;

            // 2. The parameters (mandatory)
            if let Some(&params_id) = children.get(idx) {
                if let Some(params_node) = arena.get_node(params_id) {
                    writeln!(f)?;
                    write_indent(f, indent + 1)?;
                    write!(f, ":parameters (")?;

                    let mut first = true;
                    for child_id in params_node.children() {
                        if !first {
                            write!(f, " ")?;
                        }
                        if let Some(child_node) = arena.get_node(*child_id) {
                            child_node.fmt_syntax(f, arena, interner)?;
                        } else {
                            write!(f, "<invalid-parameter>")?;
                        }
                        first = false;
                    }

                    write!(f, ")")?;
                } else {
                    writeln!(f)?;
                    write_indent(f, indent + 1)?;
                    write!(f, "<invalid_parameters>")?;
                }
            } else {
                // parameters syntax is missing: error
                writeln!(f)?;
                write_indent(f, indent + 1)?;
                write!(f, "<missing_parameters>")?;
            }

            // Close the task definition
            writeln!(f, "\n)")?;
            write_indent(f, indent)
        }

        AstKind::MethodDef => {
            let children = node.children();


            // Write the opening line with base indentation
            write_indent(f, indent)?;
            write!(f, "(:method ")?;

            // === 1. Name (mandatory) ===
            if let Some(&name_id) = children.get(0) {
                if let Some(name_node) = arena.get_node(name_id) {
                    name_node.fmt_syntax(f, arena, interner)?;
                } else {
                    write!(f, "<invalid-method-name>")?;
                }
            } else {
                write!(f, "<missing-method-name>")?;
            }

            // === 2. Parameters (mandatory) ===
            if let Some(&params_id) = children.get(1) {
                if let Some(params_node) = arena.get_node(params_id) {
                    writeln!(f)?;
                    write_indent(f, indent + 1)?;
                    write!(f, ":parameters (")?;
                    typed_list::render(params_node, f, arena, interner, false, indent + 1)?;
                    write!(f, ")")?;
                } else {
                    writeln!(f)?;
                    write_indent(f, indent + 1)?;
                    write!(f, "<invalid-parameters>")?;
                }
            } else {
                writeln!(f)?;
                write_indent(f, indent + 1)?;
                write!(f, "<missing-parameters>")?;
            }

            // === 3. MethodDefBody (mandatory) ===
            if let Some(&body_id) = children.get(2) {
                writeln!(f)?;
                if let Some(body_node) = arena.get_node(body_id) {
                    body_node.fmt_syntax_with_indent(
                        f,
                        arena,
                        interner,
                        indent + 1,
                    )?;
                } else {
                    write_indent(f, indent + 1)?;
                    write!(f, "<invalid-method-body>")?;
                }
            } else {
                writeln!(f)?;
                write_indent(f, indent + 1)?;
                write!(f, "<missing-method-body>")?;
            }

            // Closing parenthesis
            write_indent(f, indent)?;
            writeln!(f, ")")?;

            Ok(())
        }

        AstKind::Task => {
            // Call fmt_task with increased indentation level for nested formatting
            task::render(node, f, arena, interner, false, 0)
        }

        AstKind::PreconditionDef
        | AstKind::EffectDef
        | AstKind::MethodPreconditionDef
        | AstKind::TaskLogicalConstraintDef => {
            // Write the kind label
            write_indent(f, indent)?;
            node.kind().fmt_syntax_with_interner(f, interner)?;

            // Newline after the label
            writeln!(f)?;


            // Write the indentation for the child syntax
            write_indent(f, indent + 1)?;

            // Format the first child if it exists, else print <no-children>
            if let Some(first_child_id) = node.children().first() {
                if let Some(first_child_node) = arena.get_node(*first_child_id) {
                    first_child_node.fmt_syntax(f, arena, interner)?;
                } else {
                    write!(f, "<invalid>")?;
                }
            } else {
                // No children: print <no-children>
                write!(f, "<no-children>")?;
            }
            Ok(())
        }

        AstKind::AtomicFormula | AstKind::FunctionTerm => {
            // Write the opening parenthesis with current indentation
            write_indent(f, indent)?;
            write!(f, "(")?;

            let mut first = true;

            // Iterate over all children and format them with a space separator
            for child_id in node.children() {
                if !first {
                    write!(f, " ")?;
                }
                if let Some(child_node) = arena.get_node(*child_id) {
                    child_node.fmt_syntax(f, arena, interner)?;
                } else {
                    write!(f, "<invalid>")?;
                }
                first = false;
            }

            // Write the closing parenthesis
            write!(f, ")")
        }

        AstKind::Assign | AstKind::FComp => {
            // Write the opening parenthesis with current indentation
            write_indent(f, indent)?;
            write!(f, "(")?;

            // Write the operator keyword using syntax syntax formatting
            write!(f, "{}", node.content().to_string_with_interner(interner))?;

            // Format each child syntax, separated by spaces
            for child_id in node.children() {
                write!(f, " ")?;
                if let Some(child_node) = arena.get_node(*child_id) {
                    child_node.fmt_syntax(f, arena, interner)?;
                } else {
                    write!(f, "<invalid>")?;
                }
            }

            // Write the closing parenthesis
            write!(f, ")")
        }

        AstKind::And
        | AstKind::Or
        | AstKind::Not
        | AstKind::Imply
        | AstKind::AtStart
        | AstKind::AtEnd
        | AstKind::Overall => {
            // Write the opening parenthesis with current indentation
            write_indent(f, indent)?;
            write!(f, "(")?;

            // Write the operator keyword using syntax syntax formatting
            node.kind().fmt_syntax_with_interner(f, interner)?;

            // Format each child syntax, separated by spaces
            for child_id in node.children() {
                write!(f, " ")?;
                if let Some(child_node) = arena.get_node(*child_id) {
                    child_node.fmt_syntax(f, arena, interner)?;
                } else {
                    write!(f, "<invalid>")?;
                }
            }

            // Write the closing parenthesis
            write!(f, ")")
        }

        AstKind::Forall | AstKind::Exists => {
            quantifiers::render(node, f, arena, interner, indent)
        }

        AstKind::When => {
            let children = node.children();

            // 1. (when + début ligne
            write_indent(f, indent)?;
            write!(f, "(")?;
            node.kind().fmt_syntax_with_interner(f, interner)?;
            write!(f, " ")?;

            // 2. Condition sur la même ligne
            if let Some(&cond_id) = children.get(0) {
                if let Some(cond_node) = arena.get_node(cond_id) {
                    cond_node.fmt_syntax(f, arena, interner)?;
                } else {
                    write!(f, "<invalid-condition>")?;
                }
            } else {
                write!(f, "<missing-condition>")?;
            }

            write!(f, " ")?; // juste un espace, pas de writeln

            // 3. Effet indenté
            if let Some(&effect_id) = children.get(1) {
                if let Some(effect_node) = arena.get_node(effect_id) {
                    effect_node.fmt_syntax_with_indent(
                        f,
                        arena,
                        interner,
                        indent + 1,
                    )?;
                } else {
                    write!(f, "<invalid-effect>")?;
                }
            } else {
                write!(f, "<missing-effect>")?;
            }

            // 4. Fermeture de la parenthèse
            write!(f, ")")?;

            Ok(())
        }


        AstKind::TaskNetworkDef => {
            let children = node.children();

            for (i, &child_id) in children.iter().enumerate() {
                let is_last = i == children.len() - 1;

                if let Some(child_node) = arena.get_node(child_id) {
                    match child_node.kind() {
                        AstKind::OrderedSubtaskDef | AstKind::PartiallyOrderedSubtaskDef => {
                            child_node
                                .fmt_syntax_with_indent(f, arena, interner, indent)?;
                            if !is_last {
                                writeln!(f)?;
                            }
                        }
                        AstKind::TaskOrderingConstraintDef => {
                            child_node
                                .fmt_syntax_with_indent(f, arena, interner, indent)?;
                            if !is_last {
                                writeln!(f)?;
                            }
                        }
                        AstKind::TaskLogicalConstraintDef => {
                            child_node
                                .fmt_syntax_with_indent(f, arena, interner, indent)?;
                            if !is_last {
                                writeln!(f)?;
                            }
                        }
                        _ => {
                            write_indent(f, indent)?;
                            writeln!(f, "<unexpected-child-kind>")?;
                        }
                    }
                } else {
                    write_indent(f, indent)?;
                    writeln!(f, "<invalid-child-syntax>")?;
                }
            }

            Ok(())
        }

        AstKind::OrderedSubtaskDef | AstKind::PartiallyOrderedSubtaskDef => {
            // Write the type_checker of subtask with current indentation
            write_indent(f, indent)?;
            node.kind().fmt_syntax_with_interner(f, interner)?;
            writeln!(f)?;

            let mut children = node.children().iter();

            // The first child is expected to be the logical AND (or similar operator)
            if let Some(and_id) = children.next() {
                if let Some(and_node) = arena.get_node(*and_id) {
                    let and_children = and_node.children();

                    // Begin the clause with increased indentation: (and
                    write_indent(f, indent + 1)?;
                    write!(f, "(")?;
                    and_node.kind().fmt_syntax_with_interner(f, interner)?; // prints "and"

                    if and_children.is_empty() {
                        // Empty case, close the clause on the same line
                        write!(f, ")")?;
                    } else {
                        writeln!(f)?;
                        for child_id in and_children {
                            if let Some(task_node) = arena.get_node(*child_id) {
                                match task_node.kind() {
                                    AstKind::Task => {
                                        // Print task without prefix with one more indentation level
                                        write_indent(f, indent + 2)?;
                                        task::render(task_node, f, arena, interner, false, 0)?;
                                        writeln!(f)?;
                                    }
                                    AstKind::TaggedTask => {
                                        // Print tagged task using fmt_planning with indentation
                                        write_indent(f, indent + 2)?;
                                        task_node.fmt_syntax(f, arena, interner)?;
                                        writeln!(f)?;
                                    }
                                    other => {
                                        // Unexpected syntax kind
                                        write_indent(f, indent + 2)?;
                                        writeln!(f, "<unexpected-{}>", other)?;
                                    }
                                }
                            } else {
                                write_indent(f, indent + 1)?;
                                writeln!(f, "<invalid>")?;
                            }
                        }
                        // Close the clause with one level less indentation
                        write_indent(f, indent + 1)?;
                        write!(f, ")")?;
                    }
                } else {
                    write_indent(f, indent)?;
                    writeln!(f, "<invalid-and-syntax>")?;
                }
            } else {
                write_indent(f, indent)?;
                writeln!(f, "<no-children>")?;
            }

            Ok(())
        }

        AstKind::TaggedTask => {
            let children = node.children();

            // Validate that there are exactly 2 children for TaggedTask
            if children.len() != 2 {
                // Print invalid placeholder with current indentation
                write_indent(f, indent)?;
                write!(f, "<invalid-tagged-task>")?;
                return Ok(());
            }

            // Write opening parenthesis with current indentation
            write_indent(f, indent)?;
            write!(f, "(")?;

            // Print the TaskID (first child)
            if let Some(task_id_node) = arena.get_node(children[0]) {
                task_id_node.fmt_syntax(f, arena, interner)?;
            } else {
                write!(f, "<invalid-task-id>")?;
            }

            write!(f, " ")?;

            // Print the actual task (second child) without ":task" prefix
            if let Some(task_node) = arena.get_node(children[1]) {
                task::render(task_node, f, arena, interner, false, 0)?;
            } else {
                write!(f, "<invalid-task>")?;
            }

            // Close the parenthesis
            write!(f, ")")
        }

        AstKind::TaskOrderingConstraintDef => {
            // Write the kind line with current indentation
            write_indent(f, indent)?;
            writeln!(f, "{}", ORDER)?;

            let mut children = node.children().iter();

            if let Some(and_id) = children.next() {
                if let Some(and_node) = arena.get_node(*and_id) {
                    let and_children = and_node.children();

                    // Write opening line for the 'and' clause with increased indentation
                    write_indent(f, indent + 1)?;
                    write!(f, "(")?;
                    and_node.kind().fmt_syntax_with_interner(f, interner)?; // prints "and"

                    if and_children.is_empty() {
                        write!(f, ")")?;
                    } else {
                        writeln!(f)?;
                        for child_id in and_children {
                            if let Some(ordering_node) = arena.get_node(*child_id) {
                                // Print each ordering child with further indentation
                                write_indent(f, indent + 2)?;
                                ordering_node.fmt_syntax(f, arena, interner)?;
                                writeln!(f)?;
                            } else {
                                write_indent(f, indent + 2)?;
                                writeln!(f, "<invalid>")?;
                            }
                        }
                        // Closing parenthesis with increased indentation
                        write_indent(f, indent + 1)?;
                        write!(f, ")")?;
                    }
                } else {
                    write_indent(f, indent)?;
                    writeln!(f, "<invalid-and>")?;
                }
            } else {
                write_indent(f, indent)?;
                writeln!(f, "<no-children>")?;
            }

            Ok(())
        }

        AstKind::TaskOrderingConstraint => {
            // Write opening parenthesis with indentation
            write_indent(f, indent)?;
            write!(f, "(")?;

            // Print the content (e.g., "<")
            write!(f, "{}", node.content())?;

            let children = node.children();
            if children.len() == 2 {
                // Print first child with a space before
                write!(f, " ")?;
                if let Some(first_node) = arena.get_node(children[0]) {
                    first_node.fmt_syntax(f, arena, interner)?;
                } else {
                    write!(f, "<invalid>")?;
                }

                // Print second child with a space before
                write!(f, " ")?;
                if let Some(second_node) = arena.get_node(children[1]) {
                    second_node.fmt_syntax(f, arena, interner)?;
                } else {
                    write!(f, "<invalid>")?;
                }
            } else {
                // If not exactly 2 children, print an error placeholder
                write!(f, "<unexpected-children>")?;
            }

            // Close the parenthesis without additional indent (since it closes the current line)
            write!(f, ")")
        }

        AstKind::MethodDefBody => {
            let children = node.children();

            // Task (toujours présent en premier)
            if let Some(&task_id) = children.get(0) {
                write_indent(f, indent)?;
                if let Some(task_node) = arena.get_node(task_id) {
                    task::render(task_node, f, arena, interner, true, 0)?;
                } else {
                    write!(f, "<invalid-task>")?;
                }
            } else {
                writeln!(f)?;
                write_indent(f, indent)?;
                write!(f, "<missing-task>")?;
            }

            // Preconditions (optionnel, uniquement s'il y a 3 enfants)
            if children.len() == 3 {
                if let Some(&precond_id) = children.get(1) {
                    if let Some(precond_node) = arena.get_node(precond_id) {
                        precond_node
                            .fmt_syntax_with_indent(f, arena, interner, indent)?;
                        writeln!(f)?;
                    }
                }
            }

            // Task Network (toujours le dernier enfant)
            if let Some(&task_network_id) = children.last() {
                if let Some(task_network_node) = arena.get_node(task_network_id) {
                    task_network_node
                        .fmt_syntax_with_indent(f, arena, interner, indent)?;
                    writeln!(f)?;
                } else {
                    write!(f, "<invalid-task-network>")?;
                }
            } else {
                writeln!(f)?;
                write_indent(f, indent)?;
                write!(f, "<missing-task-network>")?;
            }

            Ok(())
        }

        AstKind::ActionDef => {
            let children = node.children();

            // Write the opening line with base indentation
            write_indent(f, indent)?;
            write!(f, "(:action ")?;

            // === 1. Name (mandatory) ===
            if let Some(&name_id) = children.get(0) {
                if let Some(name_node) = arena.get_node(name_id) {
                    name_node.fmt_syntax(f, arena, interner)?;
                } else {
                    write!(f, "<invalid-action-name>")?;
                }
            } else {
                write!(f, "<missing-action-name>")?;
            }

            // === 2. Parameters (mandatory) ===
            if let Some(&params_id) = children.get(1) {
                if let Some(params_node) = arena.get_node(params_id) {
                    writeln!(f)?;
                    write_indent(f, indent + 1)?;
                    write!(f, ":parameters (")?;
                    typed_list::render(params_node, f, arena, interner, false, indent + 1)?;
                    write!(f, ")")?;
                } else {
                    writeln!(f)?;
                    write_indent(f, indent + 1)?;
                    write!(f, "<invalid-parameters>")?;
                }
            } else {
                writeln!(f)?;
                write_indent(f, indent +1 )?;
                write!(f, "<missing-parameters>")?;
            }

            // === 3. ActionDefBody (mandatory) ===
            if let Some(&body_id) = children.get(2) {
                writeln!(f)?;
                if let Some(body_node) = arena.get_node(body_id) {
                    body_node.fmt_syntax_with_indent(
                        f,
                        arena,
                        interner,
                        indent + 1,
                    )?;
                    writeln!(f)?;
                } else {
                    write_indent(f, indent + 1)?;
                    write!(f, "<invalid-action-body>")?;
                    writeln!(f)?;
                }
            } else {
                writeln!(f)?;
                write_indent(f, indent + 1)?;
                write!(f, "<missing-action-body>")?;
            }

            // Closing parenthesis
            write_indent(f, indent)?;
            writeln!(f, ")")?;

            Ok(())
        }

        AstKind::ActionDefBody => {
            let children = node.children();

            let mut wrote_something = false;

            for (i, &child_id) in children.iter().enumerate() {
                // Si ce n’est pas le premier élément écrit, on insère un saut de ligne avant
                if wrote_something {
                    writeln!(f)?;
                }

                if let Some(child_node) = arena.get_node(child_id) {
                    child_node.fmt_syntax_with_indent(f, arena, interner, indent)?;
                } else {
                    write_indent(f, indent)?;
                    match i {
                        0 => write!(f, "<invalid-precondition>")?,
                        1 => write!(f, "<invalid-effect>")?,
                        _ => write!(f, "<unexpected-child>")?,
                    }
                }

                wrote_something = true;
            }

            Ok(())
        }

        AstKind::ObjectsDef => {

            // Opening line
            write_indent(f, indent)?;
            write!(f, "(")?;
            node.kind().fmt_syntax_with_interner(f, interner)?;
            writeln!(f)?;

            // Children on their own line(s), indented
            if let Some(child_id) = node.children().first() {
                if let Some(child_node) = arena.get_node(*child_id) {
                    // On peut appeler fmt_typed_list si c’est une liste typée, sinon fmt_planning_syntax_with_indent
                    typed_list::render(child_node, f, arena, interner, true, indent + 1)?;
                }
            } else {
                write_indent(f, indent + 1)?;
                writeln!(f, "<missing-objects>")?;
            }

            // Closing line
            write_indent(f, indent)?;
            write!(f, ")")?;

            Ok(())
        }

        AstKind::Init => {

            // Opening line: (:init
            write_indent(f, indent)?;
            writeln!(f, "(:init")?;

            // Init should have exactly 1 child: the root AND syntax
            if let Some(and_node_id) = node.children().first() {
                if let Some(and_node) = arena.get_node(*and_node_id) {
                    // Instead of formatting the AND itnode, iterate over its children
                    for grandchild_id in and_node.children() {
                        if let Some(grandchild_node) = arena.get_node(*grandchild_id) {
                            // Child syntax already indents itnode
                            grandchild_node.fmt_syntax_with_indent(
                                f,
                                arena,
                                interner,
                                indent + 1,
                            )?;
                        } else {
                            // Manually indent the error line
                            write_indent(f, indent + 1)?;
                            write!(f, "<invalid-init-child>")?;
                        }
                        writeln!(f)?;
                    }
                } else {
                    // Invalid AND syntax
                    write_indent(f, indent + 1)?;
                    writeln!(f, "<invalid-init-syntax>")?;
                }
            } else {
                // Missing AND syntax
                write_indent(f, indent + 1)?;
                writeln!(f, "<missing-init-expression>")?;
            }

            // Closing parenthesis
            write_indent(f, indent)?;
            write!(f, ")")
        }

        AstKind::InitialTaskNetwork => {


            let children = node.children();

            write_indent(f, indent)?;
            writeln!(f, "(:htn")?;

            // Si on a au moins deux enfants : premier = paramètres, deuxième = task network
            // Si un seul enfant : task network seulement
            // Si aucun : erreur

            let (param_opt, tn_opt) = match children.len() {
                0 => (None, None),
                1 => (None, Some(children[0])),
                _ => (Some(children[0]), Some(children[1])),
            };

            // Paramètres (s'il y en a)
            if let Some(param_id) = param_opt {
                match arena.get_node(param_id) {
                    Some(param_node) => {
                        write_indent(f, indent + 1)?;
                        write!(f, ":parameters ")?;
                        typed_list::render(param_node, f, arena, interner, false, indent + 2)?;
                        writeln!(f)?;
                    }
                    None => {
                        write_indent(f, indent + 1)?;
                        writeln!(f, "<invalid-parameters>")?;
                    }
                }
            }

            // Task network
            if let Some(tn_id) = tn_opt {
                match arena.get_node(tn_id) {
                    Some(tn_node) => {
                        tn_node.fmt_syntax_with_indent(
                            f,
                            arena,
                            interner,
                            indent + 1,
                        )?;
                    }
                    None => {
                        write_indent(f, indent + 1)?;
                        writeln!(f, "<invalid-task-network>")?;
                    }
                }
            } else {
                write_indent(f, indent + 1)?;
                writeln!(f, "<missing-task-network>")?;
            }

            write!(f, "\n)")?;
            write_indent(f, indent)
        }

        AstKind::Goal => {
            // Write the opening line with indentation
            write_indent(f, indent)?;
            writeln!(f, "(:goal")?;

            // Expect exactly one child syntax (the goal expression)
            if let Some(&child_id) = node.children().get(0) {
                if let Some(child_node) = arena.get_node(child_id) {
                    // Format the child syntax with increased indentation
                    child_node.fmt_syntax_with_indent(
                        f,
                        arena,
                        interner,
                        indent + 1,
                    )?;
                } else {
                    // Child syntax is invalid, print placeholder with indentation
                    write_indent(f, indent + 1)?;
                    writeln!(f, "<invalid-goal>")?;
                }
            } else {
                // Missing child syntax, print placeholder with indentation
                write_indent(f, indent + 1)?;
                writeln!(f, "<missing-goal>")?;
            }

            // Write the closing parenthesis aligned with the opening indentation
            write!(f, "\n)")?;
            write_indent(f, indent)
        }

        AstKind::Metric => {
            // Write the opening line for the metric section with base indentation
            write_indent(f, indent)?;
            write!(f, "(:metric")?;

            // Iterate over all children, separated by spaces (no newlines)
            for child_id in node.children() {
                write!(f, " ")?; // space before each child
                if let Some(child_node) = arena.get_node(*child_id) {
                    child_node.fmt_syntax_with_indent(
                        f,
                        arena,
                        interner,
                        indent + 1,
                    )?;
                }
            }

            // Write the closing parenthesis with no extra indentation (same line)
            write!(f, ")")
        }

        AstKind::Constant
        | AstKind::Variable
        | AstKind::FunctionSymbol
        | AstKind::PrimitiveType
        | AstKind::DomainName
        | AstKind::ProblemName
        | AstKind::Number
        | AstKind::Predicate
        | AstKind::ActionSymbol
        | AstKind::DASymbol
        | AstKind::MethodSymbol
        | AstKind::TaskSymbol
        | AstKind::PrefName
        | AstKind::Requirement
        | AstKind::TaskID => {
            write_indent(f, indent)?;
            write!(f, "{}", node.content().to_syntax_string_with_interner(interner))
        }

        AstKind::TotalTime => {
            write_indent(f, indent)?;
            write!(f, "{}", TOTAL_TIME)
        }

        AstKind::Error => {
            write_indent(f, indent)?;
            write!(f, "<error>")
        }
        AstKind::Operation => {
            let children = node.children();

            // Write opening parenthesis with current indentation
            write_indent(f, indent)?;
            write!(f, "(")?;

            if let Some(op) = node.content().as_arithmetic_op() {
                write!(f, "{}", op)?;
            } else {
                write!(f, "<non-arithmetic>")?;
            }

            // Render each child
            for &child_id in children {
                if let Some(child_node) = arena.get_node(child_id) {
                    write!(f, " ")?; // separator
                    child_node.fmt_syntax(f, arena, interner)?;
                } else {
                    write!(f, " <invalid-child>")?;
                }
            }

            // Close parenthesis
            write!(f, ")")
        }


        _ => {
            write!(f, "(DEFAULT {}", node.kind())?;
            for child_id in node.children() {
                write!(f, " ")?;
                if let Some(child_node) = arena.get_node(*child_id) {
                    child_node.fmt_syntax_with_indent(f, arena, interner, indent)?;
                }
            }
            write!(f, ")")
            /*Kind::None => {}
            Kind::DurativeActionDef => {}
            Kind::DADefBody => {}
            Kind::DerivedDef => {}
            Kind::Preference => {}
            Kind::Constraints => {}
            Kind::Always => {}
            Kind::Sometime => {}
            Kind::Within => {}
            Kind::AtMostOnce => {}
            Kind::SometimeAfter => {}
            Kind::SometimeBefore => {}
            Kind::AlwaysWithin => {}
            Kind::HoldDuring => {}
            Kind::HoldAfter => {}
            Kind::TimedInitialLiteral => {}
            Kind::IsViolated => {}
            Kind::Length => {}
            Kind::Serial => {}
            Kind::Parallel => {}
            Kind::InitialTaskNetwork => {}*/
        }

    }
}
