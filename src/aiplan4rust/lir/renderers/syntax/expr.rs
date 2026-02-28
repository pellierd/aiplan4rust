use std::fmt;
use std::fmt::Formatter;
use crate::aiplan4rust::lir::expr::{Expr, ExprKind};
use crate::aiplan4rust::lir::expr::content::Content;
use crate::aiplan4rust::lir::expr::kind::Kind;
use crate::aiplan4rust::lir::renderers::context::RenderContext;
use crate::aiplan4rust::lir::renderers::syntax::typed_list;
use crate::aiplan4rust::syntax::lexer::token::{AT, LPAREN, RPAREN};
use crate::aiplan4rust::syntax::write_indent;
use crate::aiplan4rust::tree::NodeId;

enum RenderOp {
    /// Analyse le nœud et empile ses composants (enfants, parenthèses, etc.)
    Process(NodeId, usize),
    /// Écrit une chaîne statique (ex: "(", ")", " - ")
    Write(&'static str),
    /// Écrit le contenu d'un nœud (nom de variable, symbole)
    WriteContent(NodeId),
    /// Gère l'indentation
    Indent(usize),
    /// Saute une ligne
    Newline,
}

pub fn render(
    f: &mut Formatter<'_>,
    expr: &Expr,
    context: &RenderContext,
) -> fmt::Result {
    let root_id = match expr.root_id() {
        Some(id) => id,
        None => return write!(f, "()"),
    };

    // On commence par le nœud racine
    let mut stack = vec![RenderOp::Process(root_id, 0)];

    while let Some(op) = stack.pop() {
        match op {
            RenderOp::Write(s) => write!(f, "{}", s)?,
            RenderOp::WriteContent(id) => {
                if let Some(node) = expr.get_node(id) {
                    render_exp_content(f, node.content(), context)?;
                }
            }
            RenderOp::Indent(n) => write_indent(f, n)?,
            RenderOp::Newline => writeln!(f)?,

            RenderOp::Process(id, indent) => {
                let node = expr.get_node(id).ok_or(fmt::Error)?;
                let children = node.children();

                match node.kind() {
                    // --- FORMULES ATOMIQUES / TÂCHES ---
                    // Résultat attendu : (pointing ?x3 ?x3)
                    ExprKind::AtomicFormula | ExprKind::Function | ExprKind::Task => {
                        stack.push(RenderOp::Write(RPAREN)); // )

                        for (i, &child_id) in children.iter().enumerate().rev() {
                            stack.push(RenderOp::Process(child_id, 0));
                            if i > 0 {
                                stack.push(RenderOp::Write(" ")); // Espace entre les arguments
                            }
                        }

                        stack.push(RenderOp::Write(LPAREN)); // ( (C'était RPAREN dans ton code)
                        stack.push(RenderOp::Indent(indent));
                    }

                    // --- CONNECTEURS LOGIQUES ---
                    // Résultat attendu : (and (pred1) (pred2))
                    ExprKind::And | ExprKind::Or => {
                        stack.push(RenderOp::Write(RPAREN));

                        let is_multiline = children.len() > 1; // On peut ajuster cette condition

                        for (i, &child_id) in children.iter().enumerate().rev() {
                            stack.push(RenderOp::Process(child_id, if is_multiline { indent + 1 } else { 0 }));

                            if is_multiline {
                                stack.push(RenderOp::Indent(indent + 1));
                                stack.push(RenderOp::Newline);
                            } else if i > 0 {
                                // Horizontal : un espace seulement entre les enfants
                                stack.push(RenderOp::Write(" "));
                            }
                        }

                        // Si on est en horizontal, il faut un espace APRES le mot-clé pour le premier enfant
                        if !is_multiline && !children.is_empty() {
                            stack.push(RenderOp::Write(" "));
                        }

                        stack.push(RenderOp::Write(node.kind().to_pddl_keyword()));
                        stack.push(RenderOp::Write(LPAREN));
                        stack.push(RenderOp::Indent(indent));
                    }
                    // Résultat attendu : (and (pred1) (pred2))
                    ExprKind::Not | ExprKind::Imply => {
                        stack.push(RenderOp::Write(RPAREN));

                        for (_, &child_id) in children.iter().enumerate().rev() {
                            stack.push(RenderOp::Process(child_id, 0));
                            // On ne met un espace que s'il y a un élément avant (donc i > 0)
                            // OU on en met un après le mot-clé
                            stack.push(RenderOp::Write(" "));
                        }

                        stack.push(RenderOp::Write(node.kind().to_pddl_keyword()));
                        stack.push(RenderOp::Write(LPAREN));
                        stack.push(RenderOp::Indent(indent));
                    }

                    // --- QUANTIFIERS (forall (?x) (goal)) ---
                    ExprKind::Forall | ExprKind::Exists => {
                        stack.push(RenderOp::Write(RPAREN));
                        // 2. Le corps de la formule
                        if let Some(&goal_id) = children.get(1) {
                            stack.push(RenderOp::Process(goal_id, 0));
                            stack.push(RenderOp::Write(" "));
                        }
                        // 1. La liste des variables (souvent déjà entre parenthèses dans le LIR)
                        if let Some(&vars_id) = children.get(0) {
                            stack.push(RenderOp::Process(vars_id, 0));
                        }

                        stack.push(RenderOp::Write(" "));
                        stack.push(RenderOp::Write(node.kind().to_pddl_keyword()));
                        stack.push(RenderOp::Write(LPAREN));
                        stack.push(RenderOp::Indent(indent));
                    }
                    // --- COMPARISONS & ASSIGNMENTS (= x y) ---
                    ExprKind::Comparison | ExprKind::Assignment | ExprKind::Arithmetic => {
                        stack.push(RenderOp::Write(RPAREN));
                        for (_, &child_id) in children.iter().enumerate().rev() {
                            stack.push(RenderOp::Process(child_id, 0));
                            stack.push(RenderOp::Write(" "));
                        }

                        stack.push(RenderOp::WriteContent(id));
                        stack.push(RenderOp::Write(LPAREN));
                        stack.push(RenderOp::Indent(indent));
                    }
                    // --- TEMPORAL & MODAL (at start, always...) ---
                    ExprKind::AtStart | ExprKind::AtEnd | ExprKind::Overall |
                    ExprKind::Always | ExprKind::Sometime | ExprKind::Within |
                    ExprKind::AtMostOnce | ExprKind::SometimeAfter | ExprKind::SometimeBefore |
                    ExprKind::AlwaysWithin | ExprKind::HoldDuring | ExprKind::HoldAfter => {
                        stack.push(RenderOp::Write(RPAREN));
                        for &child_id in children.iter().rev() {
                            stack.push(RenderOp::Write(" "));
                            stack.push(RenderOp::Process(child_id, 0));
                        }
                        stack.push(RenderOp::Write(node.kind().to_pddl_keyword()));
                        stack.push(RenderOp::Write(LPAREN));
                        stack.push(RenderOp::Indent(indent));
                    }
                    // --- DURATIVE & SPECIALS ---
                    ExprKind::When => {
                        stack.push(RenderOp::Write(RPAREN));
                        if let Some(&effect_id) = children.get(1) {
                            stack.push(RenderOp::Process(effect_id, 0));
                            stack.push(RenderOp::Write(" "));
                        }
                        if let Some(&cond_id) = children.get(0) {
                            stack.push(RenderOp::Process(cond_id, 0));
                        }
                        stack.push(RenderOp::Write("when "));
                        stack.push(RenderOp::Write(LPAREN));
                        stack.push(RenderOp::Indent(indent));
                    }

                    ExprKind::Metric => {
                        stack.push(RenderOp::Write(RPAREN));
                        if let Some(&goal_id) = children.get(1) {
                            stack.push(RenderOp::Process(goal_id, 0));
                            stack.push(RenderOp::Write(" "));
                        }
                        if let Some(&opt_id) = children.get(0) {
                            stack.push(RenderOp::WriteContent(opt_id)); // minimize / maximize
                        }
                        stack.push(RenderOp::Write("(:metric "));
                        stack.push(RenderOp::Indent(indent));
                    }

                    // --- HTN & CONSTRAINTS ---
                    ExprKind::LabeledTask => {
                        stack.push(RenderOp::Write(RPAREN));
                        if let Some(&task_id) = children.get(1) {
                            stack.push(RenderOp::Process(task_id, 0));
                            stack.push(RenderOp::Write(" "));
                        }
                        if let Some(&id_id) = children.get(0) {
                            stack.push(RenderOp::Process(id_id, 0));
                        }
                        stack.push(RenderOp::Write(LPAREN));
                        stack.push(RenderOp::Indent(indent));
                    }
                    ExprKind::TimedInitialLiteral => {
                        stack.push(RenderOp::Write(RPAREN));

                        // 2. L'atome ou l'effet (ex: (at a b))
                        if let Some(&effect_id) = children.get(1) {
                            stack.push(RenderOp::Process(effect_id, 0));
                            stack.push(RenderOp::Write(" "));
                        }

                        // 1. Le temps (ex: 1)
                        if let Some(&time_id) = children.get(0) {
                            stack.push(RenderOp::Process(time_id, 0));
                        }

                        stack.push(RenderOp::Write(" "));
                        stack.push(RenderOp::Write(AT));
                        stack.push(RenderOp::Write(LPAREN));
                        stack.push(RenderOp::Indent(indent));
                    }
                    // --- CONTRAINTE D'ORDONNANCEMENT SIMPLE (< id1 id2) ---
                    ExprKind::TaskOrderingConstraint => {
                        stack.push(RenderOp::Write(RPAREN));

                        let children = node.children();
                        // On suppose que children[0] est id1 et children[1] est id2
                        for (_, &child_id) in children.iter().enumerate().rev() {
                            stack.push(RenderOp::Process(child_id, 0));
                            stack.push(RenderOp::Write(" "));
                        }

                        // On affiche le contenu (le symbole '<')
                        stack.push(RenderOp::WriteContent(id));

                        stack.push(RenderOp::Write(LPAREN));
                        stack.push(RenderOp::Indent(indent));
                    }
                    Kind::Object
                    | Kind::Variable
                    | Kind::FunctionSymbol
                    | Kind::PredicateSymbol
                    | Kind::TaskSymbol
                    | Kind::PrefName
                    | Kind::Preference
                    | Kind::TaskLabel
                    | Kind::Number => {
                        stack.push(RenderOp::WriteContent(id));
                    }
                    Kind::TotalTime
                    | Kind::IsViolated
                    | Kind::Length
                    | Kind::Serial
                    | Kind::Parallel => { stack.push(RenderOp::Write(node.kind().to_pddl_keyword()));}

                }
            }
        }
    }
    Ok(())
}

fn render_exp_content(
    f: &mut fmt::Formatter<'_>,
    content: &Content,
    ctx: &RenderContext,
) -> std::fmt::Result {
    match content {
        Content::None => write!(f, "None"),

        Content::Variable(id) => {
            write!(f, "?x{}", id.as_usize())
        },
        Content::Object(id) => {
            write!(f, "{}", ctx.resolve_object(*id))
        },
        Content::PredicateSymbol(id) => {
            write!(f, "{}", ctx.resolve_predicate(*id))
        },
        Content::FunctionSymbol(id) => {
            write!(f, "{}", ctx.resolve_functor(*id))
        },
        Content::TaskSymbol(id) => {
            write!(f, "{}", ctx.resolve_task_symbol(*id))
        },

        Content::TaskLabelSymbol(id) => {
            write!(f, "t{}",  id.as_usize())
        },

        Content::PreferenceSymbol(_id) => {
            write!(f, "TO DO")
            //write!(f, "pref{}", ctx.resolve_preference(*id))
        }

        // --- Valeurs et Opérateurs (Inchangés car techniques) ---
        Content::Number(val)        => write!(f, "{}", val),
        Content::Comparison(op)    => write!(f, "{}", op),
        Content::Assignment(op)      => write!(f, "{}", op),
        Content::ArithmeticOp(op)  => write!(f, "{}", op),
        Content::OptimizationOp(opt) => write!(f, "{}", opt),

        // --- Listes typées ---
        Content::QuantifierVariables(vars) =>
            typed_list::render_typed_variable_list(f, vars.as_slice(), ctx),

        // Pour les autres IDs techniques, on peut garder le Display par défaut ou enrichir
        Content::FunctionSkeleton(_) => Ok(()),
        Content::TaskSkeleton(_) => Ok(()),
        Content::AtomSkeleton(_) => Ok(())
    }
}
