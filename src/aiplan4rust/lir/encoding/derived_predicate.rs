//! Derived Predicate Encoding
//!
//! This module handles the encoding of PDDL derived predicates (axioms).
//! Derived predicates allow the domain to define new relations based on
//! existing ones, which are automatically updated as the state changes.

use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::lir::encoding::registry::EncodingRegistry;
use crate::aiplan4rust::lir::encoding::{expr, typed_list};
use crate::aiplan4rust::lir::problem::atomic_skeleton::AtomicFormulaSkeleton;
use crate::aiplan4rust::lir::problem::derived_predicate::DerivedPredicate;
use crate::aiplan4rust::lir::problem::LiftedProblem;
use crate::aiplan4rust::lir::LirError;
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::SyntaxSubtree;

/// Encodes a derived predicate from the syntax tree into the LIR.
///
/// This function translates a PDDL axiom by encoding its "head" (the predicate
/// being defined) and its "body" (the logical condition that makes it true).
///
/// # Arguments
///
/// * `subtree` - The syntax subtree representing the `:derived` definition.
/// * `evaluator` - The evaluator for resolving identifiers within the ops.
/// * `ir` - The mutable `LiftedProblem` where the derived predicate is registered.
///
/// # Returns
///
/// * `Ok(DerivedPredicate)` - The encoded axiom.
/// * `Err(LirError)` - If the skeleton or the logical expression fails to encoding.
///
/// # Errors
///
/// This function returns an error if:
/// * The head of the derived predicate (the formula) is malformed.
/// * The body expression cannot be resolved with the current context.

pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
    ir: &mut LiftedProblem,
) -> Result<(), LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    // --- ÉTAPE 1 : Nettoyage du registre ---
    // Indispensable pour que les variables du head commencent à l'index 0
    registry.clear_variables();

    // --- ÉTAPE 2 : Résolution de l'identité du Prédicat ---
    let head_node_id = node.try_child(0)?;
    let head_node = ast.try_node(head_node_id)?;
    let predicate_symbol_node_id = head_node.try_child(0)?;
    let predicate_node = ast.try_node(predicate_symbol_node_id)?;
    let predicate_symbol_id = predicate_node.try_ident()?;

    let symbol_table = registry.symbol_table();
    let declaration =
        symbol_table.try_get_declaration_from(predicate_symbol_id, predicate_symbol_node_id)?;

    //println!("{}", symbol_table.to_string_with_interner(ir.interner()));

    //println!("{}", declaration);
    let base_predicate_node_id = declaration.refines().unwrap();
    let predicate_id = registry.try_resolve_predicate(base_predicate_node_id)?;
    let head_skeleton_id = registry.try_resolve_atom_skeleton(base_predicate_node_id)?;

    // --- ÉTAPE 3 : Encodage du Head (Variables locales à l'axiome) ---
    let params_node_id = head_node.try_child(1)?;
    let params_node = ast.try_node(params_node_id)?;

    // On enregistre les variables (?x, ?y) SANS vider le registre
    let parameters = typed_list::encode_variable_list(
        &SyntaxSubtree::new(params_node, params_node_id, ast),
        registry,
    )?;

    let variable_symbols = registry.get_variable_symbols();
    let head_skeleton = AtomicFormulaSkeleton::new(predicate_id, parameters)
        .with_variable_symbols(variable_symbols);

    // --- ÉTAPE 4 : Encodage du Body ---
    // Maintenant que le registre contient uniquement les variables du head,
    // l'encodage de l'expression utilisera les bons index (0, 1, ...).
    let body_node_id = node.try_child(1)?;
    let body_node = ast.try_node(body_node_id)?;
    let body = expr::encode(&SyntaxSubtree::new(body_node, body_node_id, ast), registry)?;

    // --- ÉTAPE 5 : Finalisation ---
    let variable_symbols = registry.get_variable_symbols();
    let derived_predicate = DerivedPredicate::new(head_skeleton_id, head_skeleton, body)
        .with_variable_symbols(variable_symbols);

    ir.add_derived_predicate_def(derived_predicate);
    Ok(())
}

/*pub fn encode(
    subtree: &SyntaxSubtree<AstNode>,
    registry: &mut EncodingRegistry,
) -> Result<DerivedPredicate, LirError> {
    let node = subtree.node();
    let ast = subtree.tree();

    // 1. Le Head est à l'index 0 (ex: (reachable ?x ?y))
    let head_node_id = node.try_child(0)?;
    let head_node = ast.try_node(head_node_id)?;

    // --- Résolution de l'ID ---
    // On récupère le NodeId du symbole (le nom du prédicat)
    let predicate_symbol_node_id = head_node.try_child(0)?;
    let predicate_node = ast.try_node(predicate_symbol_node_id)?;
    let predicate_symbol_id = predicate_node.try_ident()?;

    let symbol_table = registry.symbol_table();
    let declaration =
        symbol_table.try_get_declaration_from(predicate_symbol_id, predicate_symbol_node_id)?;

    println!("{}", registry.symbol_table());

    // On récupère le PredicateID (L'identité sémantique)
    let predicate_id = registry.try_resolve_predicate(declaration.node_id())?;

    // On récupère aussi l'AtomSkeletonID (L'ID structurel pour le DerivedPredicate final)
    let head_skeleton_id = registry.try_resolve_atom_skeleton(declaration.node_id())?;

    // 2. Encode le Head skeleton.
    // On passe le predicate_id qu'on vient de résoudre à ton nouvel encodeur.
    let head_skeleton = atomic_formula_skeleton::encode(
        &SyntaxSubtree::new(head_node, head_node_id, ast),
        registry,
        predicate_id, // <--- C'est ici qu'on injecte l'ID résolu
    )?;

    // 3. Encode le Body (l'expression logique est à l'index 1).
    let body_node_id = node.try_child(1)?;
    let body_node = ast.try_node(body_node_id)?;

    // Le registre contient maintenant les variables du head (ex: ?x, ?y)
    let body = expr::encode(&SyntaxSubtree::new(body_node, body_node_id, ast), registry)?;
    let variable_symbols = registry.get_variable_symbols();
    let derived_predicate = DerivedPredicate::new(head_skeleton_id, head_skeleton, body)
        .with_variable_symbols(variable_symbols);
    Ok(derived_predicate)
}*/
