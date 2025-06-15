use std::collections::{HashMap, HashSet};
use crate::aiplan4rust::syntax::ast::{Ast, AstKind, AstNode};
use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticKind, DiagnosticManager, Provider};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::syntax::Span;

/// Normalizes type declarations by merging TypedItems with the same PrimitiveType.
///
/// # Arguments
/// * `ast` - The AST to normalize.
/// * `diagnostic_manager` - Manager for diagnostics (currently unused).
///
/// # Returns
/// * `Ok(true)` if any changes were made.
/// * `Ok(false)` if no changes were necessary.
/// * `Err` if validation fails.
pub fn normalize_type_def(
    ast: &mut Ast,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, ParserInternalError> {
    let source = ast.source_name().to_string();

    // Locate the TypesDef node
    let types_def = match find_types_def(ast)? {
        Some(node) => node,
        None => return Ok(false),
    };

    // Retrieve the TypedList node
    let typed_list = types_def
        .children_mut()
        .get_mut(0)
        .ok_or_else(|| ParserInternalError::new("TypesDef node must have a TypedList child".to_string()))?;

    validate_typed_list_node(typed_list)?;

    let mut merged_items: HashMap<String, Box<AstNode>> = HashMap::new();
    let mut type_sources: HashMap<String, (HashSet<String>, Vec<Span>)> = HashMap::new();

    let mut changed = false;

    // Detach and take ownership of current typed items
    let old_typed_items = std::mem::take(typed_list.children_mut());

    for typed_item in old_typed_items.into_iter() {
        validate_typed_item_node(&typed_item)?;

        let children = typed_item.children();
        let primitive_type_node = &children[0];
        let key = validate_primitive_type_node(primitive_type_node)?;

        let ty_opt = children.get(1).cloned();
        let span = primitive_type_node.span().clone();

        // Extract type names (either A B) → [A, B]
        let type_names: HashSet<String> = match &ty_opt {
            Some(ty_node) => extract_type_names(ty_node)?,
            None => HashSet::new(),
        };

        // Track merged types and spans
        let entry = type_sources.entry(key.clone()).or_insert_with(|| (HashSet::new(), Vec::new()));
        entry.0.extend(type_names.iter().cloned());
        entry.1.push(span.clone());

        if let Some(existing_item) = merged_items.get_mut(&key) {
            if let Some(mut new_ty) = ty_opt {
                let existing_children = existing_item.children_mut();
                if existing_children.len() > 1 {
                    existing_children[1].children_mut().extend(new_ty.children_mut().drain(..));
                } else {
                    existing_children.push(new_ty);
                }
            }

            changed = true;
        } else {
            merged_items.insert(key.clone(), typed_item);
            changed = true;
        }
    }

    // Emit diagnostics for each key with multiple type declarations
    for (ty, (types_set, spans)) in &type_sources {
        if spans.len() > 1 {
            let diagnostic = Diagnostic::new(
                DiagnosticKind::ImplicitEitherTypeDeclarationWarning {
                    ty: ty.clone(),
                    types: types_set.iter().cloned().collect(),
                    spans: spans.clone(),
                },
                Provider::Normalizer,
                source.clone(),
                spans[0].clone(),
            );
            diagnostic_manager.add_diagnostic(diagnostic);
        }
    }

    // Rebuild TypedList node
    let new_typed_items: Vec<Box<AstNode>> = merged_items.into_values().collect();
    typed_list.set_children(new_typed_items);

    Ok(changed)
}


/// Finds the mutable TypesDef node in the AST and validates its children.
///
/// # Returns
/// - Ok(Some(&mut AstNode)) if TypesDef node is found and valid
/// - Ok(None) if no TypesDef node found
/// - Err if invalid children found
pub fn find_types_def(
    ast: &mut Ast,
) -> Result<Option<&mut AstNode>, ParserInternalError> {
    if let Some(types_def_node) = ast
        .root_mut()
        .children_mut()
        .iter_mut()
        .find(|node| matches!(node.kind(), AstKind::TypesDef))
    {
        validate_types_def_node(types_def_node)?;
        Ok(Some(types_def_node.as_mut()))
    } else {
        Ok(None)
    }
}

fn extract_type_names(type_node: &AstNode) -> Result<HashSet<String>, ParserInternalError> {
    if let AstKind::Type = type_node.kind() {
        let mut names = HashSet::new();
        for child in type_node.children() {
            match child.kind() {
                AstKind::PrimitiveType(name) => {
                    names.insert(name.clone());
                }
                other => {
                    return Err(ParserInternalError::new(format!(
                        "Expected PrimitiveType inside Type node, found: {:?}",
                        other
                    )));
                }
            }
        }
        Ok(names)
    } else {
        Err(ParserInternalError::new(format!(
            "Expected Type node, found: {:?}",
            type_node.kind()
        )))
    }
}


/// Validate that the given node is a TypedList.
///
/// # Errors
/// Returns an error if the node is not a TypedList.
fn validate_typed_list_node(node: &AstNode) -> Result<(), ParserInternalError> {
    if node.kind() != &AstKind::TypedList {
        Err(ParserInternalError::new("Expected TypedList node".to_string()))
    } else {
        Ok(())
    }
}

/// Validate that the given node is a TypedItem.
///
/// # Errors
/// Returns an error if the node is not a TypedItem.
fn validate_typed_item_node(node: &AstNode) -> Result<(), ParserInternalError> {
    if node.kind() != &AstKind::TypedItem {
        Err(ParserInternalError::new("Expected TypedItem node".to_string()))
    } else if node.children().is_empty() {
        Err(ParserInternalError::new("TypedItem must have at least one child".to_string()))
    } else {
        Ok(())
    }
}

/// Validate that the first child of TypedItem is a PrimitiveType node.
///
/// # Errors
/// Returns an error if the first child is not a PrimitiveType.
fn validate_primitive_type_node(node: &AstNode) -> Result<String, ParserInternalError> {
    match node.kind() {
        AstKind::PrimitiveType(name) => Ok(name.clone()),
        _ => Err(ParserInternalError::new(
            "TypedItem first child must be PrimitiveType".to_string(),
        )),
    }
}

/// Validate that the given `TypesDef` node has exactly one child of kind `TypedList`.
///
/// Returns `Ok(())` if valid, or an error otherwise.
fn validate_types_def_node(types_def_node: &AstNode) -> Result<(), ParserInternalError> {
    let children = types_def_node.children();

    if children.len() != 1 {
        return Err(ParserInternalError::new(format!(
            "TypesDef node must have exactly one child, found {}",
            children.len()
        )));
    }

    if children[0].kind() != &AstKind::TypedList {
        return Err(ParserInternalError::new(
            "TypesDef child must be TypedList".to_string(),
        ));
    }

    Ok(())
}
