//! This module handles the syntax rendering of PDDL/HDDL expressions using an iterative stack machine.

use crate::aiplan4rust::lang::{TypeId, TypedSymbol, VariableId};
use crate::aiplan4rust::lir::store::expr::{ExprEntryKind, ExprId};
use crate::aiplan4rust::lir::store::renderers::syntax::typed_list;
use crate::aiplan4rust::lir::store::renderers::RenderContext;
use std::fmt;
use std::fmt::Formatter;

/// Opérations de la machine d'état de rendu itératif.
enum RenderOp {
    Process(ExprId, usize),
    Write(&'static str),
    WriteDynamic(String),
    WriteContent(ExprId),
    /// On transporte un vecteur de symboles typés correspondants à la signature attendue
    WriteVariables(Vec<TypedSymbol<VariableId, TypeId>>),
    Indent(usize),
    Newline,
}

pub fn render(f: &mut Formatter<'_>, root_id: ExprId, context: &RenderContext) -> fmt::Result {
    render_with_indent(f, root_id, context, 0)
}

pub fn render_with_indent(
    f: &mut Formatter<'_>,
    root_id: ExprId,
    context: &RenderContext,
    indent: usize,
) -> fmt::Result {
    if root_id.is_none() {
        return write!(f, "()");
    }

    // On initialise la pile avec l'indentation reçue en paramètre
    let mut stack = vec![RenderOp::Process(root_id, indent)];

    while let Some(op) = stack.pop() {
        match op {
            RenderOp::Write(s) => write!(f, "{}", s)?,
            RenderOp::WriteDynamic(s) => write!(f, "{}", s)?,
            RenderOp::WriteContent(id) => render_terminal_node(f, id, context)?,
            RenderOp::Indent(n) => {
                for _ in 0..n {
                    write!(f, "  ")?;
                }
            }
            RenderOp::Newline => writeln!(f)?,
            RenderOp::WriteVariables(vars) => {
                typed_list::render_typed_variable_list(f, vars.as_slice(), context)?;
            }

            RenderOp::Process(id, indent) => {
                let entry = &context.store()[id];
                let children = entry.children();
                let kind = entry.kind();

                match kind {
                    // --- FORMULES ATOMIQUES / SQUELETTES / TÂCHES ---
                    ExprEntryKind::AtomicFormula(_)
                    | ExprEntryKind::Function(_)
                    | ExprEntryKind::Task(_) => {
                        stack.push(RenderOp::Write(")"));

                        for (i, &child_id) in children.iter().enumerate().rev() {
                            stack.push(RenderOp::Process(child_id, 0));
                            if i > 0 {
                                stack.push(RenderOp::Write(" "));
                            }
                        }

                        stack.push(RenderOp::Write("("));
                        stack.push(RenderOp::Indent(indent));
                    }

                    // --- CONNECTEURS LOGIQUES N-AIRES (and, or) ---
                    ExprEntryKind::And | ExprEntryKind::Or => {
                        stack.push(RenderOp::Write(")"));

                        let is_multiline = children.len() > 1;

                        for (i, &child_id) in children.iter().enumerate().rev() {
                            stack.push(RenderOp::Process(
                                child_id,
                                if is_multiline { indent + 1 } else { 0 },
                            ));

                            if is_multiline {
                                // CRUCIAL : On veut d'abord exécuter l'Indent, puis le Newline.
                                // Comme c'est une pile (LIFO), on doit push le Newline EN PREMIER.
                                stack.push(RenderOp::Newline);
                                stack.push(RenderOp::Indent(indent + 1));
                            } else if i > 0 {
                                stack.push(RenderOp::Write(" "));
                            }
                        }

                        if !is_multiline && !children.is_empty() {
                            stack.push(RenderOp::Write(" "));
                        }

                        stack.push(RenderOp::Write(kind.to_pddl_keyword()));
                        stack.push(RenderOp::Write("("));

                        // On applique l'indentation initiale reçue sous le mot-clé (ex: :precondition)
                        stack.push(RenderOp::Indent(indent));
                    }

                    // --- CONNECTEURS UNRESETS / BINAIRES (not, imply) ---
                    ExprEntryKind::Not | ExprEntryKind::Imply => {
                        stack.push(RenderOp::Write(")"));

                        let child_is_complex = children.first().map_or(false, |&c| {
                            let k = context.store()[c].kind();
                            matches!(
                                k,
                                ExprEntryKind::And
                                    | ExprEntryKind::Or
                                    | ExprEntryKind::Forall(_)
                                    | ExprEntryKind::Exists(_)
                                    | ExprEntryKind::When
                            )
                        });

                        for &child_id in children.iter().rev() {
                            if child_is_complex {
                                stack.push(RenderOp::Process(child_id, indent + 1));
                                stack.push(RenderOp::Indent(indent + 1));
                                stack.push(RenderOp::Newline);
                            } else {
                                stack.push(RenderOp::Process(child_id, 0));
                                stack.push(RenderOp::Write(" "));
                            }
                        }

                        stack.push(RenderOp::Write(kind.to_pddl_keyword()));
                        stack.push(RenderOp::Write("("));
                        stack.push(RenderOp::Indent(indent));
                    }

                    // --- QUANTIFICATEURS (forall / exists) ---
                    ExprEntryKind::Forall(typed_list) | ExprEntryKind::Exists(typed_list) => {
                        stack.push(RenderOp::Write(")"));

                        if let Some(&body_id) = children.first() {
                            stack.push(RenderOp::Process(body_id, indent + 1));
                            stack.push(RenderOp::Indent(indent + 1));
                            stack.push(RenderOp::Newline);
                        }

                        stack.push(RenderOp::Write(")"));
                        stack.push(RenderOp::WriteVariables(typed_list.as_slice().to_vec()));
                        stack.push(RenderOp::Write("("));

                        stack.push(RenderOp::Write(" "));
                        stack.push(RenderOp::Write(kind.to_pddl_keyword()));
                        stack.push(RenderOp::Write("("));
                        stack.push(RenderOp::Indent(indent));
                    }

                    // --- COMPARAISONS, ASSIGNATIONS & ARITHMÉTIQUE ---
                    ExprEntryKind::Comparison(op) => {
                        render_infix_operation(op.to_string(), children, 0, &mut stack);
                        stack.push(RenderOp::Indent(indent));
                    }
                    ExprEntryKind::Assignment(op) => {
                        render_infix_operation(op.to_string(), children, 0, &mut stack);
                        stack.push(RenderOp::Indent(indent));
                    }
                    ExprEntryKind::Arithmetic(op) => {
                        render_infix_operation(op.to_string(), children, 0, &mut stack);
                        stack.push(RenderOp::Indent(indent));
                    }

                    // --- TEMPORELS & MODAUX (at start, overall...) ---
                    ExprEntryKind::AtStart
                    | ExprEntryKind::AtEnd
                    | ExprEntryKind::Overall
                    | ExprEntryKind::Always
                    | ExprEntryKind::Sometime
                    | ExprEntryKind::Within
                    | ExprEntryKind::AtMostOnce
                    | ExprEntryKind::SometimeAfter
                    | ExprEntryKind::SometimeBefore
                    | ExprEntryKind::AlwaysWithin
                    | ExprEntryKind::HoldDuring
                    | ExprEntryKind::HoldAfter => {
                        stack.push(RenderOp::Write(")"));
                        for &child_id in children.iter().rev() {
                            stack.push(RenderOp::Write(" "));
                            stack.push(RenderOp::Process(child_id, 0));
                        }
                        stack.push(RenderOp::Write(kind.to_pddl_keyword()));
                        stack.push(RenderOp::Write("("));
                        stack.push(RenderOp::Indent(indent));
                    }

                    // --- EFFETS CONDITIONNELS (when) ---
                    ExprEntryKind::When => {
                        stack.push(RenderOp::Write(")"));

                        if let Some(&effect_id) = children.get(1) {
                            stack.push(RenderOp::Process(effect_id, indent + 1));
                            stack.push(RenderOp::Indent(indent + 1));
                            stack.push(RenderOp::Newline);
                        }

                        if let Some(&cond_id) = children.get(0) {
                            stack.push(RenderOp::Process(cond_id, indent + 1));
                            stack.push(RenderOp::Indent(indent + 1));
                            stack.push(RenderOp::Newline);
                        }

                        stack.push(RenderOp::Write("when"));
                        stack.push(RenderOp::Write("("));
                        stack.push(RenderOp::Indent(indent));
                    }

                    // --- MÉTRIQUES ---
                    ExprEntryKind::Metric(op) => {
                        stack.push(RenderOp::Write(")"));
                        if let Some(&goal_id) = children.get(1) {
                            stack.push(RenderOp::Process(goal_id, 0));
                            stack.push(RenderOp::Write(" "));
                        }
                        if let Some(&opt_id) = children.get(0) {
                            stack.push(RenderOp::WriteContent(opt_id));
                        }
                        stack.push(RenderOp::WriteDynamic(format!("(:metric {} ", op)));
                        stack.push(RenderOp::Indent(indent));
                    }

                    // --- HTN & CONTRAINTES D'ORDONNANCEMENT ---
                    ExprEntryKind::LabeledTask => {
                        stack.push(RenderOp::Write(")"));
                        if let Some(&task_id) = children.get(1) {
                            stack.push(RenderOp::Process(task_id, 0));
                            stack.push(RenderOp::Write(" "));
                        }
                        if let Some(&id_id) = children.get(0) {
                            stack.push(RenderOp::Process(id_id, 0));
                        }
                        stack.push(RenderOp::Write("("));
                        stack.push(RenderOp::Indent(indent));
                    }
                    ExprEntryKind::TimedInitialLiteral => {
                        stack.push(RenderOp::Write(")"));
                        if let Some(&effect_id) = children.get(1) {
                            stack.push(RenderOp::Process(effect_id, 0));
                            stack.push(RenderOp::Write(" "));
                        }
                        if let Some(&time_id) = children.get(0) {
                            stack.push(RenderOp::Process(time_id, 0));
                        }
                        stack.push(RenderOp::Write(" (at "));
                        stack.push(RenderOp::Indent(indent));
                    }
                    ExprEntryKind::TaskOrderingConstraint(op) => {
                        stack.push(RenderOp::Write(")"));
                        for (i, &child_id) in children.iter().enumerate().rev() {
                            stack.push(RenderOp::Process(child_id, 0));
                            if i > 0 {
                                stack.push(RenderOp::Write(" "));
                            }
                        }
                        stack.push(RenderOp::WriteDynamic(op.to_string()));
                        stack.push(RenderOp::Write("("));
                        stack.push(RenderOp::Indent(indent));
                    }

                    // --- FEUILLES TERMINALES STANDARD ---
                    ExprEntryKind::Variable(_)
                    | ExprEntryKind::Object(_)
                    | ExprEntryKind::PredicateSymbol(_)
                    | ExprEntryKind::FunctionSymbol(_)
                    | ExprEntryKind::TaskSymbol(_)
                    | ExprEntryKind::PrefName(_)
                    | ExprEntryKind::TaskLabel(_)
                    | ExprEntryKind::Number(_) => {
                        stack.push(RenderOp::WriteContent(id));
                    }

                    _ => {
                        stack.push(RenderOp::WriteDynamic(kind.to_pddl_keyword().to_string()));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Helper factorisant le déroulement des opérations infixées standard PDDL (=, +, -, etc.)
fn render_infix_operation(
    op_symbol: String,
    children: &[ExprId],
    indent: usize,
    stack: &mut Vec<RenderOp>,
) {
    stack.push(RenderOp::Write(")"));
    for (i, &child_id) in children.iter().enumerate().rev() {
        stack.push(RenderOp::Process(child_id, 0));
        if i > 0 {
            stack.push(RenderOp::Write(" "));
        }
    }
    stack.push(RenderOp::WriteDynamic(op_symbol));
    stack.push(RenderOp::Write("("));
    stack.push(RenderOp::Indent(indent));
}

/// Résolution et affichage direct des feuilles terminales depuis le nouveau modèle
fn render_terminal_node(f: &mut Formatter<'_>, id: ExprId, ctx: &RenderContext) -> fmt::Result {
    let kind = ctx.store()[id].kind();
    match kind {
        ExprEntryKind::Variable(v_id) => write!(f, "?x{}", v_id.as_usize()),
        ExprEntryKind::Object(obj_id) => write!(f, "{}", ctx.resolve_object(*obj_id)),
        ExprEntryKind::PredicateSymbol(p_id) => write!(f, "{}", ctx.resolve_predicate(*p_id)),
        ExprEntryKind::FunctionSymbol(func_id) => write!(f, "{}", ctx.resolve_functor(*func_id)),
        ExprEntryKind::TaskSymbol(t_id) => write!(f, "{}", ctx.resolve_task_symbol(*t_id)),
        ExprEntryKind::TaskLabel(l_id) => write!(f, "t{}", l_id.as_usize()),
        ExprEntryKind::Number(val) => write!(f, "{}", val),
        ExprEntryKind::PrefName(p_id) => write!(f, "pref_{}", p_id.as_usize()),
        _ => Ok(()),
    }
}
