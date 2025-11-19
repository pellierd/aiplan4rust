use std::fmt;
use std::fmt::Formatter;
use std::ptr::write;
use crate::aiplan4rust::core::arena::ArenaNode;
use crate::aiplan4rust::interner::{InternerDisplay, StringInterner};

use crate::aiplan4rust::syntax::tree::{SyntaxNode, SyntaxTree};
use crate::aiplan4rust::syntax::lexer::token::{ORDER, TOTAL_TIME};
use crate::aiplan4rust::syntax::SyntaxDisplay;
use crate::aiplan4rust::syntax::tree::renderers::RenderKind;
use crate::aiplan4rust::syntax::tree::renderers::syntax::{task, typed_list};

pub fn render<T: SyntaxNode>(
    node: &T,
    f: &mut Formatter<'_>,
    arena: &SyntaxTree<T>,
    interner: &StringInterner,
) -> fmt::Result {
    render_with_indent(node, f, arena, interner, 0)?;
    Ok(())
}

pub fn render_with_indent<T: SyntaxNode>(
    node: &T,
    f: &mut Formatter<'_>,
    arena: &SyntaxTree<T>,
    interner: &StringInterner,
    indent: usize,
) -> fmt::Result {
    let indent_str = T::make_indent(indent);
    match node.render_kind() {
        RenderKind::Domain => {
            // Write the opening line with base indentation
            write!(f, "{}(define (domain ", indent_str)?;

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
            write!(f, "{})\n", indent_str)?;

            Ok(())
        }

        RenderKind::Problem => {
            let children = node.children();

            // Opening line with base indentation
            write!(f, "{}(define (problem ", indent_str)?;

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
                    let child_indent = T::make_indent(indent + 1);
                    write!(f, "{}(:domain ", child_indent)?;
                    domain_node.fmt_syntax(f, arena, interner)?;
                    write!(f, ")")?;
                } else {
                    writeln!(f)?;
                    let child_indent = T::make_indent(indent + 1);
                    write!(f, "{}<invalid-domain-name>", child_indent)?;
                }
            } else {
                writeln!(f)?;
                let child_indent = T::make_indent(indent + 1);
                write!(f, "{}<missing-domain-name>", child_indent)?;
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
            write!(f, "{})", indent_str)?;

            Ok(())
        }

        RenderKind::RequireDef => {
            // Indentation de la ligne d'ouverture
            f.write_str(&indent_str)?;
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

        RenderKind::TypesDef => {
            f.write_str(&indent_str)?;
            write!(f, "(")?;
            node.render_kind().fmt_syntax(f, interner)?;
            writeln!(f)?;

            if let Some(child_id) = node.children().first() {
                if let Some(child_node) = arena.get_node(*child_id) {
                    // Pass indent + 1 to indent children 2 spaces more than the opening line
                    typed_list::render(child_node, f, arena, interner, true, indent + 1)?;
                }
            } else {
                // No children: optionally write a comment or just an empty indented line
                let empty_indent = T::make_indent(indent + 1);
                writeln!(f, "{}; <missing-typed-list>", empty_indent)?;
            }

            f.write_str(&indent_str)?; // root indentation for closing parenthesis
            writeln!(f, ")")
        }

        RenderKind::TypedList => typed_list::render(node, f, arena, interner, false, indent),

        RenderKind::TypedItemElements => {
            // Write the indentation once before the list of elements
            write!(f, "{}", indent_str)?;

            // Iterate over children nodes
            for (i, child_id) in node.children().iter().enumerate() {
                // Separate elements with spaces
                if i > 0 {
                    write!(f, " ")?;
                }

                // Try to get the syntax
                if let Some(child_node) = arena.get_node(*child_id) {
                    write!(f, "{}",  child_node.content().to_syntax_string(interner))?;
                } else {
                    write!(f, "<invalid_node>")?;
                }
            }

            Ok(())
        }

        RenderKind::TypedItem => {
            let indent_str = T::make_indent(indent);

            // Handle first child (elements)
            if let Some(first_child_id) = node.get_child(0) {
                if let Some(first_child_node) = arena.get_node(first_child_id) {
                    first_child_node
                        .fmt_syntax_with_indent(f, arena, interner, indent)?;
                } else {
                    write!(f, "{}<invalid_node>", indent_str)?;
                }
            } else {
                write!(f, "{}<missing_child>", indent_str)?;
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

        RenderKind::Type => {
            match node.arity() {
                0 => write!(f, ""),
                1 => {
                    let ty_node_id = node.children()[0];
                    if let Some(ty_node) = arena.get_node(ty_node_id) {
                        // Write indentation before the single type_checker
                        write!(f, "{}", indent_str)?;
                        // Recursively format the type_checker content with the current indentation
                        write!(f, "{}",  ty_node.content().to_string_with_interner(interner))
                    } else {
                        write!(f, "{}<invalid_node>", indent_str)
                    }
                }

                _ => {
                    // Write indentation and opening either keyword
                    write!(f, "{}(either", indent_str)?;

                    for child_id in node.children() {
                        write!(f, " ")?;
                        if let Some(ty_node) = arena.get_node(*child_id) {
                            // Format each type_checker content recursively, no extra indent here since on the same line
                            write!(f, "{}",  ty_node.content().to_string_with_interner(interner))?;
                        } else {
                            write!(f, "<invalid_node>")?;
                        }
                    }
                    write!(f, ")")
                }
            }
        }

        RenderKind::ConstantsDef => {
            // Write the opening line with current indentation
            writeln!(f, "{}(:constants", indent_str)?;

            // Format the constants list line by line with increased indentation for readability
            if let Some(child_id) = node.children().first() {
                if let Some(child_node) = arena.get_node(*child_id) {
                    // Assuming fmt_typed_list now takes indent parameter to handle multiline indent
                    typed_list::render(child_node, f, arena, interner, true, indent + 1)?;
                }
            }

            // Write the closing parenthesis aligned with the opening line
            writeln!(f, "{})", indent_str)
        }

        RenderKind::PredicatesDef => {
            // Write the opening line with current indentation
            writeln!(f, "{}(:predicates", indent_str)?;

            // Iterate over all children and format each predicate with increased indentation
            for child_id in node.children() {
                if let Some(child_node) = arena.get_node(*child_id) {
                    // Write indent for predicates' lines (one level deeper)
                    let child_indent_str = T::make_indent(indent + 1);
                    write!(f, "{}", child_indent_str)?;
                    child_node.fmt_syntax(f, arena, interner)?;
                    writeln!(f)?;
                }
            }

            // Write the closing parenthesis aligned with the opening line
            writeln!(f, "{})", indent_str)
        }

        RenderKind::FunctionsDef => {
            // Write the opening line with current indentation
            write!(f, "{}(:functions", indent_str)?;

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

        RenderKind::AtomicFormulaSkeleton | RenderKind::AtomicFunctionSkeleton => {
            // Write the opening parenthesis with the given indentation
            write!(f, "{}(", indent_str)?;

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

        RenderKind::TaskDef => {
            // Write opening line with indentation
            write!(f, "{}(:task ", indent_str)?;

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
                    write!(f, "{}:parameters (", T::make_indent(indent + 1))?;

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
                    write!(f, "{}<invalid_parameters>", T::make_indent(indent + 1))?;
                }
            } else {
                // parameters syntax is missing: error
                writeln!(f)?;
                write!(f, "{}<missing_parameters>", T::make_indent(indent + 1))?;
            }

            // Close the task definition
            writeln!(f, "\n{})", indent_str)
        }

        RenderKind::MethodDef => {
            let children = node.children();

            // Base indentation for :parameters etc.
            let indent_param = T::make_indent(indent + 1);

            // Write the opening line with base indentation
            write!(f, "{}(:method ", indent_str)?;

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
                    write!(f, "{}:parameters (", indent_param)?;
                    typed_list::render(params_node, f, arena, interner, false, indent + 1)?;
                    write!(f, ")")?;
                } else {
                    writeln!(f)?;
                    write!(f, "{}<invalid-parameters>", indent_param)?;
                }
            } else {
                writeln!(f)?;
                write!(f, "{}<missing-parameters>", indent_param)?;
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
                    let indent_body = T::make_indent(indent + 1);
                    write!(f, "{}<invalid-method-body>", indent_body)?;
                }
            } else {
                writeln!(f)?;
                let indent_body = T::make_indent(indent + 1);
                write!(f, "{}<missing-method-body>", indent_body)?;
            }

            // Closing parenthesis
            writeln!(f, "{})", indent_str)?;

            Ok(())
        }

        RenderKind::Task => {
            // Call fmt_task with increased indentation level for nested formatting
            task::render(node, f, arena, interner, true, 0)
        }

        RenderKind::PreconditionDef
        | RenderKind::EffectDef
        | RenderKind::MethodPreconditionDef
        | RenderKind::TaskLogicalConstraintDef => {
            // Write the kind label
            write!(f, "{}", indent_str)?;
            node.render_kind().fmt_syntax(f, interner)?;

            // Newline after the label
            writeln!(f)?;

            let child_indent_str = T::make_indent(indent + 1);

            // Write the indentation for the child syntax
            write!(f, "{}", child_indent_str)?;

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

        RenderKind::AtomicFormula | RenderKind::FunctionTerm => {
            // Write the opening parenthesis with current indentation
            write!(f, "{}(", indent_str)?;

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

        RenderKind::Assign | RenderKind::FComp => {
            // Write the opening parenthesis with current indentation
            write!(f, "{}(", indent_str)?;

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

        RenderKind::And
        | RenderKind::Or
        | RenderKind::Not
        | RenderKind::Imply
        | RenderKind::AtStart
        | RenderKind::AtEnd
        | RenderKind::Overall => {
            // Write the opening parenthesis with current indentation
            write!(f, "{}(", indent_str)?;

            // Write the operator keyword using syntax syntax formatting
            node.render_kind().fmt_syntax(f, interner)?;

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

        RenderKind::Forall | RenderKind::Exists => {
            let children = node.children();

            // 1. (forall / (exists + début ligne
            write!(f, "{}(", indent_str)?;
            node.render_kind().fmt_syntax(f, interner)?;
            write!(f, " ")?;

            // 2. Variables quantifiées (sur la même ligne)
            if let Some(&vars_id) = children.get(0) {
                if let Some(vars_node) = arena.get_node(vars_id) {
                    write!(f, "(")?;
                    vars_node.fmt_syntax(f, arena, interner)?;
                    write!(f, ")")?;
                } else {
                    write!(f, "<invalid-variables>")?;
                }
            } else {
                write!(f, "<missing-variables>")?;
            }

            // 3. Expression (indentée d’un cran)
            if let Some(&expr_id) = children.get(1) {
                if let Some(expr_node) = arena.get_node(expr_id) {
                    expr_node.fmt_syntax_with_indent(
                        f,
                        arena,
                        interner,
                        indent + 1,
                    )?;
                } else {
                    writeln!(f, "{}<invalid-expression>", indent_str)?;
                }
            } else {
                writeln!(f, "{}<missing-expression>", indent_str)?;
            }
            write!(f, "{})", indent_str)?; // fermeture

            Ok(())
        }

        RenderKind::When => {
            let children = node.children();

            // 1. (when + début ligne
            write!(f, "{}(", indent_str)?;
            node.render_kind().fmt_syntax(f, interner)?;
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

            writeln!(f)?; // retour à la ligne après la condition

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
                    writeln!(f, "{}<invalid-effect>", indent_str)?;
                }
            } else {
                writeln!(f, "{}<missing-effect>", indent_str)?;
            }

            // 4. Fermeture sans retour ligne après
            write!(f, "{})", indent_str)?;

            Ok(())
        }

        RenderKind::TaskNetworkDef => {
            let children = node.children();

            for (i, &child_id) in children.iter().enumerate() {
                let is_last = i == children.len() - 1;

                if let Some(child_node) = arena.get_node(child_id) {
                    match child_node.render_kind() {
                        RenderKind::OrderedSubtaskDef | RenderKind::PartiallyOrderedSubtaskDef => {
                            child_node
                                .fmt_syntax_with_indent(f, arena, interner, indent)?;
                            if !is_last {
                                writeln!(f)?;
                            }
                        }
                        RenderKind::TaskOrderingConstraintDef => {
                            child_node
                                .fmt_syntax_with_indent(f, arena, interner, indent)?;
                            if !is_last {
                                writeln!(f)?;
                            }
                        }
                        RenderKind::TaskLogicalConstraintDef => {
                            child_node
                                .fmt_syntax_with_indent(f, arena, interner, indent)?;
                            if !is_last {
                                writeln!(f)?;
                            }
                        }
                        _ => {
                            writeln!(f, "{}<unexpected-child-kind>", indent_str)?;
                        }
                    }
                } else {
                    writeln!(f, "{}<invalid-child-syntax>", indent_str)?;
                }
            }

            Ok(())
        }

        RenderKind::OrderedSubtaskDef | RenderKind::PartiallyOrderedSubtaskDef => {
            // Write the type_checker of subtask with current indentation
            write!(f, "{}", indent_str)?;
            node.render_kind().fmt_syntax(f, interner)?;
            writeln!(f)?;

            let mut children = node.children().iter();

            // The first child is expected to be the logical AND (or similar operator)
            if let Some(and_id) = children.next() {
                if let Some(and_node) = arena.get_node(*and_id) {
                    let and_children = and_node.children();

                    // Begin the clause with increased indentation: (and
                    write!(f, "{}(", RenderKind::make_indent(indent + 1))?;
                    and_node.render_kind().fmt_syntax(f, interner)?; // prints "and"

                    if and_children.is_empty() {
                        // Empty case, close the clause on the same line
                        write!(f, ")")?;
                    } else {
                        writeln!(f)?;
                        for child_id in and_children {
                            if let Some(task_node) = arena.get_node(*child_id) {
                                match task_node.render_kind() {
                                    RenderKind::Task => {
                                        // Print task without prefix with one more indentation level
                                        write!(f, "{}", T::make_indent(indent + 2))?;
                                        task::render(task_node, f, arena, interner, false, 0)?;
                                        writeln!(f)?;
                                    }
                                    RenderKind::TaggedTask => {
                                        // Print tagged task using fmt_planning with indentation
                                        write!(f, "{}", T::make_indent(indent + 2))?;
                                        task_node.fmt_syntax(f, arena, interner)?;
                                        writeln!(f)?;
                                    }
                                    other => {
                                        // Unexpected syntax kind
                                        writeln!(
                                            f,
                                            "{}<unexpected-{}>",
                                            T::make_indent(indent + 2),
                                            other
                                        )?;
                                    }
                                }
                            } else {
                                writeln!(f, "{}<invalid>", T::make_indent(indent + 1))?;
                            }
                        }
                        // Close the clause with one level less indentation
                        write!(f, "{})", T::make_indent(indent + 1))?;
                    }
                } else {
                    writeln!(f, "{}<invalid-and-syntax>", indent_str)?;
                }
            } else {
                writeln!(f, "{}<no-children>", indent_str)?;
            }

            Ok(())
        }

        RenderKind::TaggedTask => {
            let children = node.children();

            // Validate that there are exactly 2 children for TaggedTask
            if children.len() != 2 {
                // Print invalid placeholder with current indentation
                write!(f, "{}<invalid-tagged-task>", indent_str)?;
                return Ok(());
            }

            // Write opening parenthesis with current indentation
            write!(f, "{}(", indent_str)?;

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

        RenderKind::TaskOrderingConstraintDef => {
            // Write the kind line with current indentation
            writeln!(f, "{}{}", indent_str, ORDER)?;

            let mut children = node.children().iter();

            if let Some(and_id) = children.next() {
                if let Some(and_node) = arena.get_node(*and_id) {
                    let and_children = and_node.children();

                    // Write opening line for the 'and' clause with increased indentation
                    write!(f, "{}(", T::make_indent(indent + 1))?;
                    and_node.render_kind().fmt_syntax(f, interner)?; // prints "and"

                    if and_children.is_empty() {
                        write!(f, ")")?;
                    } else {
                        writeln!(f)?;
                        for child_id in and_children {
                            if let Some(ordering_node) = arena.get_node(*child_id) {
                                // Print each ordering child with further indentation
                                write!(f, "{}", T::make_indent(indent + 2))?;
                                ordering_node.fmt_syntax(f, arena, interner)?;
                                writeln!(f)?;
                            } else {
                                writeln!(f, "{}<invalid>", T::make_indent(indent + 2))?;
                            }
                        }
                        // Closing parenthesis with increased indentation
                        write!(f, "{})", T::make_indent(indent + 1))?;
                    }
                } else {
                    writeln!(f, "{}<invalid-and>", indent_str)?;
                }
            } else {
                writeln!(f, "{}<no-children>", indent_str)?;
            }

            Ok(())
        }

        RenderKind::TaskOrderingConstraint => {
            let indent_str = T::make_indent(indent); // Compute indent string once

            // Write opening parenthesis with indentation
            write!(f, "{}(", indent_str)?;

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

        RenderKind::MethodDefBody => {
            let children = node.children();

            // Task (toujours présent en premier)
            if let Some(&task_id) = children.get(0) {
                write!(f, "{}", indent_str)?;
                if let Some(task_node) = arena.get_node(task_id) {
                    task::render(task_node, f, arena, interner, true, 0)?;
                } else {
                    write!(f, "<invalid-task>")?;
                }
            } else {
                writeln!(f)?;
                write!(f, "{}<missing-task>", indent_str)?;
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
                write!(f, "{}<missing-task-network>", indent_str)?;
            }

            Ok(())
        }

        RenderKind::ActionDef => {
            let children = node.children();

            // Base indentation for :parameters etc.
            let indent_param = T::make_indent(indent + 1);

            // Write the opening line with base indentation
            write!(f, "{}(:action ", indent_str)?;

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
                    write!(f, "{}:parameters (", indent_param)?;
                    typed_list::render(params_node, f, arena, interner, false, indent + 1)?;
                    write!(f, ")")?;
                } else {
                    writeln!(f)?;
                    write!(f, "{}<invalid-parameters>", indent_param)?;
                }
            } else {
                writeln!(f)?;
                write!(f, "{}<missing-parameters>", indent_param)?;
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
                    let indent_body = T::make_indent(indent + 1);
                    write!(f, "{}<invalid-action-body>", indent_body)?;
                    writeln!(f)?;
                }
            } else {
                writeln!(f)?;
                let indent_body = T::make_indent(indent + 1);
                write!(f, "{}<missing-action-body>", indent_body)?;
            }

            // Closing parenthesis
            writeln!(f, "{})", indent_str)?;

            Ok(())
        }

        RenderKind::ActionDefBody => {
            let children = node.children();
            let indent_child = T::make_indent(indent);

            let mut wrote_something = false;

            for (i, &child_id) in children.iter().enumerate() {
                // Si ce n’est pas le premier élément écrit, on insère un saut de ligne avant
                if wrote_something {
                    writeln!(f)?;
                }

                if let Some(child_node) = arena.get_node(child_id) {
                    child_node.fmt_syntax_with_indent(f, arena, interner, indent)?;
                } else {
                    match i {
                        0 => write!(f, "{}<invalid-precondition>", indent_child)?,
                        1 => write!(f, "{}<invalid-effect>", indent_child)?,
                        _ => write!(f, "{}<unexpected-child>", indent_child)?,
                    }
                }

                wrote_something = true;
            }

            Ok(())
        }

        RenderKind::ObjectsDef => {
            let indent_str = T::make_indent(indent);

            // Opening line
            write!(f, "{}(", indent_str)?;
            node.render_kind().fmt_syntax(f, interner)?;
            writeln!(f)?;

            // Children on their own line(s), indented
            if let Some(child_id) = node.children().first() {
                if let Some(child_node) = arena.get_node(*child_id) {
                    // On peut appeler fmt_typed_list si c’est une liste typée, sinon fmt_planning_syntax_with_indent
                    typed_list::render(child_node, f, arena, interner, true, indent + 1)?;
                }
            } else {
                let empty_indent = T::make_indent(indent + 1);
                writeln!(f, "{}; <missing-objects>", empty_indent)?;
            }

            // Closing line
            write!(f, "{})", indent_str)?;

            Ok(())
        }

        RenderKind::Init => {
            let indent_str = T::make_indent(indent);
            let child_indent_str = T::make_indent(indent + 1);

            // Opening line: (:init
            writeln!(f, "{}(:init", indent_str)?;

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
                            write!(f, "{}<invalid-init-child>", child_indent_str)?;
                        }
                        writeln!(f)?;
                    }
                } else {
                    // Invalid AND syntax
                    writeln!(f, "{}<invalid-init-syntax>", child_indent_str)?;
                }
            } else {
                // Missing AND syntax
                writeln!(f, "{}<missing-init-expression>", child_indent_str)?;
            }

            // Closing parenthesis
            write!(f, "{})", indent_str)
        }

        RenderKind::InitialTaskNetwork => {
            let indent_str = T::make_indent(indent);
            let child_indent_str = T::make_indent(indent + 1);
            let children = node.children();

            writeln!(f, "{}(:htn", indent_str)?;

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
                        write!(f, "{}:parameters ", child_indent_str)?;
                        typed_list::render(param_node, f, arena, interner, false, indent + 2)?;
                        writeln!(f)?;
                    }
                    None => {
                        writeln!(f, "{}<invalid-parameters>", child_indent_str)?;
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
                        writeln!(f, "{}<invalid-task-network>", child_indent_str)?;
                    }
                }
            } else {
                writeln!(f, "{}<missing-task-network>", child_indent_str)?;
            }

            write!(f, "\n{})", indent_str)
        }

        RenderKind::Goal => {
            // Write the opening line with indentation
            writeln!(f, "{}(:goal", indent_str)?;

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
                    let child_indent = T::make_indent(indent + 1);
                    writeln!(f, "{}<invalid-goal>", child_indent)?;
                }
            } else {
                // Missing child syntax, print placeholder with indentation
                let child_indent = T::make_indent(indent + 1);
                writeln!(f, "{}<missing-goal>", child_indent)?;
            }

            // Write the closing parenthesis aligned with the opening indentation
            write!(f, "\n{})", indent_str)
        }

        RenderKind::Metric => {
            // Write the opening line for the metric section with base indentation
            write!(f, "{}(:metric", indent_str)?;

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

        RenderKind::Constant
        | RenderKind::Variable
        | RenderKind::FunctionSymbol
        | RenderKind::PrimitiveType
        | RenderKind::DomainName
        | RenderKind::ProblemName
        | RenderKind::Number
        | RenderKind::Predicate
        | RenderKind::ActionSymbol
        | RenderKind::DASymbol
        | RenderKind::MethodSymbol
        | RenderKind::TaskSymbol
        | RenderKind::PrefName
        | RenderKind::Requirement
        | RenderKind::TaskID => {
            write!(f, "{}{}", indent_str, node.content().to_syntax_string(interner))
        }

        RenderKind::TotalTime => {
            write!(f, "{}{}", indent_str, TOTAL_TIME)
        }

        RenderKind::Error => {
            write!(f, "{}<error>", indent_str)
        }

        _ => {
            write!(f, "(DEFAULT{}", node.render_kind())?;
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
            Kind::Operation => {}
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
