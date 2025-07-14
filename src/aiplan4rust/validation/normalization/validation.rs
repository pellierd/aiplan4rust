use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::syntax::ast::{Ast, AstKind, AstNode};
use crate::aiplan4rust::validation::common::{checks, WellNormalizedError};
use crate::aiplan4rust::validation::{common, syntax};

pub fn is_well_normalized(ast: &Ast) -> Result<(), WellNormalizedError> {
    match ast.arena().root_node() {
        Some(root) => check_well_normalized_from(root, ast),
        None => Ok(()), // No root node means empty tree which can be considered well-formed
    }
}

pub fn check_well_normalized(ast: &Ast) -> Result<(), WellNormalizedError> {
    match ast.arena().root_node() {
        Some(root) => check_well_normalized_from(root, ast),
        None => Ok(()),
    }
}

pub fn check_well_normalized_from(node: &AstNode, ast: &Ast) -> Result<(), WellNormalizedError> {
    check_well_normalized_node(node, ast)?;
    for child_id in node.children() {
        let child_node = checks::get_node(ast, node, child_id.as_usize())?;
        check_well_normalized_from(child_node, ast)?;
    }

    Ok(())
}

pub fn check_well_normalized_node(node: &AstNode, ast: &Ast) -> Result<(), WellNormalizedError> {
    match node.kind() {
        AstKind::TypesDef => {
            syntax::checks::check_types_def(ast, node)
        }
        AstKind::TypedList => {
            syntax::checks::check_typed_list(ast, node)
        }
        AstKind::TypedItem => {
            syntax::checks::check_typed_item(ast, node)
        }
        AstKind::TypedItemElements => {
            syntax::checks::check_typed_item_elements(ast, node)
        }
        _ => {
            syntax::validation::check_well_formed_node(node, ast)
        }
    }
}
