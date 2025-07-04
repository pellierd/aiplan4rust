use std::collections::HashSet;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::interner::DisplayWithInterner;
use crate::aiplan4rust::linking::LinkedSemanticContext;
use crate::aiplan4rust::tree::{TreeArena, TreeNode};
use crate::aiplan4rust::lir::expr::Expr;
use crate::aiplan4rust::lang::{Requirement, TypedSymbol};
use crate::aiplan4rust::lir::def::{FunctionDef, PredicateDef, TaskDef};
use crate::aiplan4rust::lir::lifted::{LiftedAction, LiftedMethod};
use crate::aiplan4rust::lir::LiftedProblem;
use crate::aiplan4rust::semantic::AstArenaNode;
use crate::aiplan4rust::syntax::ast::{AstKind, FromAst};

#[derive(Debug)]
pub struct IRBuilder {
    name : String
}

impl IRBuilder {
    pub fn new() -> Self {
        IRBuilder {
            name: "IRBuilder".to_string(),
        }
    }
}

impl IRBuilder {

    /// Construit un LiftedProblem à partir du contexte sémantique complet
    pub fn build(
        &mut self,
        context: &LinkedSemanticContext,
    ) -> Result<LiftedProblem, ParserInternalError> {
        let mut ir = LiftedProblem::new();

        // Extraire d'abord les données du domaine
        self.extract_ir_from_domain(context, &mut ir)?;

        // Puis extraire les données du problème
        self.extract_ir_from_problem(context, &mut ir)?;

        Ok(ir)
    }

    pub fn extract_ir_from_domain(
        &mut self,
        context: &LinkedSemanticContext,
        ir: &mut LiftedProblem
    ) -> Result<LiftedProblem, ParserInternalError> {
        let mut ir = LiftedProblem::new();

        let domain = context.domain();
        for node in domain.preorder() {
            match node.kind() {
                AstKind::DomainName => {
                    ir.set_domain_name(node.try_ident()?);
                }
                AstKind::RequireDef => {
                    ir.add_requirements(extract_requirements_from(node, domain)?);
                }
                AstKind::TypesDef => {
                    ir.add_types(extract_types_from(node, domain)?);
                }
                AstKind::ConstantsDef => {
                    ir.add_constants(extract_constants_from(node, domain)?);
                }
                AstKind::PredicatesDef => {
                    ir.add_predicates(extract_predicates_from(node, domain)?);
                }
                AstKind::FunctionsDef => {
                    ir.add_functions(extract_functions_from(node, domain)?);
                }
                AstKind::Constraints => {
                    ir.set_domain_constraints(Expr::from_ast(node, domain)?);
                }
                AstKind::TaskDef => {
                    ir.add_task(TaskDef::from_ast(node, domain)?);
                }
                AstKind::ActionDef => {
                    ir.add_action(LiftedAction::from_ast(node, domain)?);
                }
                AstKind::MethodDef => {
                    ir.add_method(LiftedMethod::from_ast(node, domain)?);
                }
                _ => {
                    // For now, ignore other kinds.
                    // You can add handling for MethodDef, FunctionDef, etc. here.
                }
            }
        }

        Ok(ir)
    }

    /// Extraction des informations du problème vers IR
    fn extract_ir_from_problem(
        &self,
        context: &LinkedSemanticContext,
        ir: &mut LiftedProblem,
    ) -> Result<(), ParserInternalError> {

        let problem = context.problem();

        for node in problem.preorder() {
            match node.kind() {
                AstKind::ProblemName => {
                    ir.set_problem_name(node.try_ident()?);
                }
                AstKind::RequireDef => {
                    ir.add_requirements(extract_requirements_from(node, problem)?);
                }
                AstKind::ObjectsDef => {
                    ir.add_objects(extract_constants_from(node, problem)?);
                }
                AstKind::Init => {
                    ir.set_init(extract_init_from(node, problem)?);
                }
                AstKind::Goal => {
                    ir.set_goal(extract_goal_from(node, problem)?);
                }
                AstKind::Constraints => {
                    ir.set_problem_constraints(Expr::from_ast(node, problem)?);
                }
                AstKind::Metric => {
                    ir.set_metric_spec(Expr::from_ast(node, problem)?);
                }
                AstKind::Length => {
                    ir.set_length_spec(Expr::from_ast(node, problem)?);
                }
                // Ajoute d'autres kinds si nécessaire pour le problème
                _ => {
                    // Ignorer les autres pour l'instant
                }
            }
        }
        Ok(())
    }
}

fn extract_requirements_from(
    require_def_node: &AstArenaNode,
    ast: &TreeArena<AstArenaNode>,
) -> Result<HashSet<Requirement>, ParserInternalError> {
    extract_set_from_children(require_def_node, ast, |node, _ast| node.try_requirement())
}

fn extract_predicates_from(
    predicates_def_node: &AstArenaNode,
    ast: &TreeArena<AstArenaNode>,
) -> Result<HashSet<PredicateDef>, ParserInternalError> {
    extract_set_from_children(predicates_def_node, ast, PredicateDef::from_ast)
}

fn extract_functions_from(
    functions_def_node: &AstArenaNode,
    ast: &TreeArena<AstArenaNode>,
) -> Result<HashSet<FunctionDef>, ParserInternalError> {
    extract_set_from_children(functions_def_node, ast, FunctionDef::from_ast)
}

fn extract_types_from(
    types_def_node: &AstArenaNode,
    ast: &TreeArena<AstArenaNode>,
) -> Result<HashSet<TypedSymbol>, ParserInternalError> {
    extract_set_from_first_child_children(types_def_node, ast, TypedSymbol::from_ast)
}

fn extract_constants_from(
    constants_def_node: &AstArenaNode,
    ast: &TreeArena<AstArenaNode>,
) -> Result<HashSet<TypedSymbol>, ParserInternalError> {
    extract_set_from_first_child_children(constants_def_node, ast, TypedSymbol::from_ast)
}

fn extract_init_from(
    init_node: &AstArenaNode,
    ast: &TreeArena<AstArenaNode>,
) -> Result<Expr, ParserInternalError> {
    extract_expr_from_first_child(init_node, ast)
}

fn extract_goal_from(
    goal_node: &AstArenaNode,
    ast: &TreeArena<AstArenaNode>,
) -> Result<Expr, ParserInternalError> {
    extract_expr_from_first_child(goal_node, ast)
}

fn extract_expr_from_first_child(
    node: &AstArenaNode,
    ast: &TreeArena<AstArenaNode>,
) -> Result<Expr, ParserInternalError> {
    let first_child_id = node.try_child(0)?;
    let first_child_node = ast.try_node(first_child_id)?;
    Expr::from_ast(first_child_node, ast)
}

/// Parcourt les enfants directs du noeud `node` et construit un HashSet<T>
/// avec `extract_fn` appliqué à chaque enfant.
fn extract_set_from_children<T, F>(
    node: &AstArenaNode,
    ast: &TreeArena<AstArenaNode>,
    extract_fn: F,
) -> Result<HashSet<T>, ParserInternalError>
where
    T: std::hash::Hash + Eq,
    F: Fn(&AstArenaNode, &TreeArena<AstArenaNode>) -> Result<T, ParserInternalError>,
{
    let mut set = HashSet::new();
    for child_id in node.children() {
        let child_node = ast.try_node(*child_id)?;
        let item = extract_fn(child_node, ast)?;
        set.insert(item);
    }
    Ok(set)
}

/// Parcourt les enfants du premier enfant du noeud `node` et construit un HashSet<T>
/// avec `extract_fn` appliqué à chaque enfant.
fn extract_set_from_first_child_children<T, F>(
    node: &AstArenaNode,
    ast: &TreeArena<AstArenaNode>,
    extract_fn: F,
) -> Result<HashSet<T>, ParserInternalError>
where
    T: std::hash::Hash + Eq,
    F: Fn(&AstArenaNode, &TreeArena<AstArenaNode>) -> Result<T, ParserInternalError>,
{
    let first_child_id = node.try_child(0)?;
    let first_child_node = ast.try_node(first_child_id)?;

    extract_set_from_children(first_child_node, ast, extract_fn)
}
