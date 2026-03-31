use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager};
use crate::aiplan4rust::semantic::checks::util::{
    check_kind_compatibility, match_signature, resolve_declaration,
};
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::semantic::signature_checker::match_result::MatchResult;
use crate::aiplan4rust::semantic::signature_checker::signature_checker::SignatureChecker;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::Usage;
use crate::aiplan4rust::semantic::symbol::{Declaration, Scope};
use crate::aiplan4rust::semantic::symbol_table::SymbolTable;
use crate::aiplan4rust::semantic::{SemanticError, TypeChecker};
use crate::aiplan4rust::syntax::ast::AstNode;
use crate::aiplan4rust::tree::Node;

/// Checks for errors in the symbol declarations and their usages in the given annotated syntax arena.
///
/// This function scans through the `symbol_table` of the provided `arena` to match each symbol's
/// declarations and usages. It ensures that symbols used in the arena are correctly declared and
/// that their types match the expected types. Errors are added to the provided `ErrorManager`
/// during the process.
///
/// # Arguments
///
/// * `arena` - An `AnnotatedSyntaxTree` that contains the symbols to check.
/// * `type_checker` - A `TypeChecker` used to validate types during the check.
/// * `errors` - A mutable reference to an `ErrorManager` where any errors found during the check
///   will be added.
///
/// # Returns
///
/// A `Result<bool, ParserInternalError>` where:
/// * `Ok(true)` indicates that no errors were found during the check.
/// * `Ok(false)` indicates that errors were found and added to the `ErrorManager`.
/// * `Err(ParserInternalError)` indicates an internal error occurred during the process.
///
/// # Example
///
/// ```rust
/// let mut errors = ErrorManager::new();
/// if atomic_formula_checker::check(&arena, &type_checker, &mut errors).is_ok() {
///     // Handle no errors
/// } else {
///     // Handle errors
///     self.error_manager.add_errors_from(&errors);
/// }
/// ```
pub fn check_symbol_signatures(
    context: &CheckContext,
    symbol_table: &mut SymbolTable,
    type_checker: &TypeChecker,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticError> {
    let mut no_error = true;
    let mut bindings = Vec::new();

    // 1. Initialisation du Checker.
    // On passe None pour l'annex_table car ici on vérifie la cohérence interne d'un fichier.
    let checker = SignatureChecker::new(symbol_table, context.syntax_tree(), type_checker, None);

    // Parcourir tous les symboles de la table.
    for symbol in symbol_table.values() {
        // Vérifier toutes les déclarations du symbole.
        for declaration in symbol.declarations().values() {
            if !matches!(
                declaration.symbol_kind(),
                SymbolKind::Predicate
                    | SymbolKind::Function
                    | SymbolKind::Task
                    | SymbolKind::Action
            ) {
                continue;
            }

            // Vérifier tous les usages du symbole.
            for usage in symbol.usages().values() {
                let decl_kind = declaration.symbol_kind();
                let usage_kind = usage.symbol_kind();

                // On ne garde que ce qui a une signature.
                if !matches!(
                    usage_kind,
                    SymbolKind::Predicate
                        | SymbolKind::Function
                        | SymbolKind::Task
                        | SymbolKind::Action
                ) {
                    continue;
                }

                // Vérification de la compatibilité des "genres" (ex: Predicate vs Action).
                if !check_kind_compatibility(decl_kind, usage_kind) {
                    continue;
                }

                // 2. Validation de la signature via le MatchResult
                match checker.match_declaration_with_usage(declaration, usage)? {
                    MatchResult::Match => {
                        // Succès parfait : on enregistre pour le "vissage" final.
                        bindings.push((symbol.ident(), declaration.source(), usage.source()));
                    }
                    MatchResult::UpcastMatch {
                        expected,
                        provided,
                        arg_decl,
                        arg_node_id,
                    } => {
                        // Succès avec réserve : on lie le symbole car c'est un candidat valide...
                        bindings.push((symbol.ident(), declaration.source(), usage.source()));

                        // ... mais on remonte un Warning à l'utilisateur.
                        let arg_node = context.syntax_tree().try_node(arg_node_id)?;
                        let warning = Diagnostic::warning_task_argument_is_supertype_of_declaration(
                            arg_decl,
                            expected,
                            provided,
                            context.provider(),
                            context.source(),
                            arg_node.span(),
                        );
                        diagnostic_manager.add_diagnostic(warning);
                    }
                    MatchResult::NoMatch => {
                        // Échec de signature : cette déclaration ne correspond pas à l'usage.
                        no_error = false;

                        let entry_node = context.syntax_tree().try_node(usage.source())?;
                        let error = Diagnostic::error_invalid_symbol_signature(
                            declaration.clone(),
                            usage.clone(),
                            context.provider(),
                            context.source(),
                            entry_node.span(),
                        );
                        diagnostic_manager.add_diagnostic(error);
                    }
                }
            }
        }
    }

    // --- PHASE 2 : LE VISSAGE (Mutation) ---
    // La boucle précédente est terminée, l'emprunt immuable sur symbol_table est libéré.
    // On peut maintenant demander l'accès mutable exclusif.
    for (symbol_id, decl_node_id, usage_node_id) in bindings {
        let mut entry = symbol_table.try_get_symbol_mut(symbol_id)?;
        // Lien Usage -> Declaration
        if let Some(u) = entry.usages_mut().get_mut(&usage_node_id) {
            u.set_declaration(decl_node_id);
        }
        // Lien Declaration -> Usage
        if let Some(d) = entry.declarations_mut().get_mut(&decl_node_id) {
            d.add_usage(usage_node_id);
        }
    }

    Ok(no_error)
}

/// Matches a declaration to its usage, verifying that the argument types are correct and match.
///
/// This method ensures that the declaration and usage of a symbol are consistent with each other.
/// It checks if the argument types in the usage match the types in the declaration.
///
/// # Arguments
///
/// * `declaration` - The declaration of the symbol being used.
/// * `usage` - The usage of the symbol in the AST.
/// * `symbol_table` - The table containing the symbols for reference.
/// * `ast_old` - The AST table for resolving entries and their types.
/// * `type_checker` - A type_checker checker used to validate the matching types.
///
/// # Returns
///
/// `Result<bool, ParserInternalError>`: Returns `Ok(true)` if the declaration and usage match,
/// `Ok(false)` if they don't, or a `ParserInternalError` if any error occurs.
pub fn match_declaration_with_usage(
    declaration: &Declaration,
    usage: &Usage,
    symbol_table: &SymbolTable,
    context: &CheckContext,
    type_checker: &TypeChecker,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticError> {
    // 1. Vérification de la signature (noms et arité)
    let Some(arguments) = match_signature(declaration, usage) else {
        // DEBUG: Si on entre ici, c'est que le nom ou le nombre d'arguments ne colle pas
        println!(
            "DEBUG [Match]: Rejeté par match_signature. Decl: {} (args attendus: {:?}), Usage: {} (args fournis: {:?})",
            declaration.symbol().id(),
            declaration.arguments().map(|a| a.len()).unwrap_or(0),
            usage.symbol().id(),
            usage.has_arguments()
        );
        return Ok(false);
    };

    // 2. Validation du contenu des arguments
    for (index, &arg_id) in arguments.iter().enumerate() {
        let arg_node = context.syntax_tree().try_node(arg_id)?;

        // Utilisation du helper pour résoudre la déclaration de l'argument

        let arg_decl_opt = resolve_argument_declaration(arg_node, usage.scope(), symbol_table)?;

        if arg_decl_opt.is_none() {
            // DEBUG: L'argument (ex: pc-bPlugType1) n'est pas trouvé dans la table
            println!(
                "DEBUG [Match]: Argument à l'index {} non résolu. Ident: '{}', Scope d'usage: {:?}",
                index,
                arg_node.try_ident()?,
                usage.scope()
            );
            return Ok(false);
        }

        let arg_decl = arg_decl_opt.unwrap();

        // 3. Vérification de la compatibilité des types
        let is_match = match_argument(
            declaration,
            usage,
            &arg_decl,
            context,
            arg_node,
            index,
            type_checker,
            diagnostic_manager,
        )?;

        if !is_match {
            // DEBUG: Le type ne correspond pas (ex: attendu Port, reçu Device)
            println!(
                "DEBUG [Match]: Type mismatch à l'index {}. Argument '{}' (Type: {:?}) ne match pas la signature attendue.",
                index,
                arg_node.try_ident()?,
                arg_decl.ty()
            );
            return Ok(false);
        }
    }

    // Si on arrive ici, tout est OK
    println!(
        "DEBUG [Match]: SUCCÈS pour {} à l'usage node {}",
        declaration.symbol().id(),
        usage.source()
    );
    Ok(true)
}

/// Résout la déclaration d'un argument à partir de son nœud AST.
fn resolve_argument_declaration<'a>(
    argument_node: &AstNode,
    scope: &Scope,
    symbol_table: &'a SymbolTable, // On précise que la table vit au moins 'a
) -> Result<Option<&'a Declaration>, SemanticError> {
    // On renvoie une référence &'a
    let name = argument_node.try_ident()?;
    let kind = SymbolKind::try_from(argument_node.kind())?;

    let entry = symbol_table.try_get_symbol(name)?;
    Ok(resolve_declaration(entry, kind, scope))
}

/// Matches a specific argument in the declaration to its expected type_checker.
///
/// This function verifies that the argument in the usage corresponds to the declaration,
/// ensuring that types match correctly and the argument is within valid bounds.
///
/// # Arguments
///
/// * `declaration` - The declaration of the symbol.
/// * `usage` - The usage of the symbol.
/// * `symbol_declaration` - The already resolved declaration of the argument.
/// * `context` - The check context.
/// * `argument` - The AST node of the argument.
/// * `index` - The index of the argument in the argument list.
/// * `type_checker` - A type_checker checker to validate type_checker consistency.
///
/// # Returns
///
/// `Result<bool, ParserInternalError>`: Returns `Ok(true)` if the argument matches the expected
/// declaration, or `Err` with a `ParserInternalError` if any validation error occurs.

fn match_argument_base(
    atom_decl: &Declaration, // La définition (ex: l'Action "marcher")
    arg_decl: &Declaration,  // L'argument réel (ex: la variable "V1")
    usage_scope: &Scope,
    index: usize,
    type_checker: &TypeChecker,
) -> Result<bool, SemanticCheckError> {
    // 1. ty_expected (Type attendu)
    // On va chercher dans la LISTE des arguments de l'atome à la POSITION index
    let ty_expected = atom_decl
        .arguments()
        .and_then(|args| args.get(index))
        .map(|arg| arg.ty())
        .ok_or_else(|| {
            SemanticCheckError::argument_index_out_of_bounds(index, atom_decl.scope().clone())
        })?;

    // 2. ty_provided (Type fourni)
    // On prend le type DIRECT de l'argument (pas d'index ici !)
    let ty_provided = arg_decl.ty().ok_or_else(|| {
        SemanticCheckError::missing_symbol_types(arg_decl.symbol().id(), usage_scope.clone())
    })?;

    // 3. Vérification : est-ce que "V1" est bien du type attendu par "marcher" ?
    Ok(type_checker.is_any_subtype_of(ty_expected, ty_provided)?)
}

fn match_argument(
    atom_decl: &Declaration,
    usage: &Usage,
    arg_decl: &Declaration,
    context: &CheckContext,
    argument_node: &AstNode,
    index: usize,
    type_checker: &TypeChecker,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    // 1. On tente le match standard
    let is_subtype = match_argument_base(atom_decl, arg_decl, usage.scope(), index, type_checker)?;

    // 2. Si échec, on vérifie l'exception d'upcasting (uniquement pour les Tasks)
    if !is_subtype
        && usage.symbol().kind() == SymbolKind::Task
        && matches!(atom_decl.symbol().kind(), SymbolKind::Action)
    {
        // On récupère les types (on sait qu'ils existent car match_argument_base a réussi avant)
        let ty_expected = atom_decl.arguments().unwrap().get(index).unwrap().ty();
        let ty_provided = arg_decl.ty().unwrap();

        // Si l'attendu est un super-type du fourni (Upcasting : Expected :> Provided)
        if type_checker.is_any_supertype_of(ty_expected, ty_provided)? {
            let warning = Diagnostic::warning_task_argument_is_supertype_of_declaration(
                arg_decl.clone(),
                ty_expected.clone(),
                ty_provided.clone(),
                context.provider(),
                context.source(),
                argument_node.span(),
            );
            diagnostic_manager.add_diagnostic(warning);
            return Ok(true);
        }
    }

    Ok(is_subtype)
}
