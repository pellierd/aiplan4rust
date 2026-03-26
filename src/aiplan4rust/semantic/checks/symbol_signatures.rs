use crate::aiplan4rust::arena::ArenaNode;
use crate::aiplan4rust::diagnostic::{Diagnostic, DiagnosticManager};
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::semantic::symbol::Declaration;
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::Usage;
use crate::aiplan4rust::semantic::symbol_table::SymbolTable;
use crate::aiplan4rust::semantic::{SemanticError, TypeChecker};
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::{Node, NodeId};

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
    type_checker: &TypeChecker,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticError> {
    let symbol_table = context.symbol_table();
    let mut no_error = true;

    // Loop over all symbols in the symbol table.
    for symbol in symbol_table.values() {
        // Check all declarations of the symbol.
        for declaration in symbol.declarations() {
            if !matches!(
                declaration.symbol_kind(),
                SymbolKind::Predicate
                    | SymbolKind::Function
                    | SymbolKind::Task
                    | SymbolKind::Action
            ) {
                continue;
            }

            // Check all usages of the symbol.
            for usage in symbol.usages() {
                let usage_kind = usage.symbol_kind();
                let decl_kind = declaration.symbol_kind();

                // --- STRATÉGIE DE FILTRAGE UNIFIÉE ---

                // 1. Si les genres sont différents et NE PEUVENT PAS partager l'espace de noms,
                //    alors cet usage ne concerne pas cette déclaration.
                if usage_kind != decl_kind && !decl_kind.can_share_name_space_with(&usage_kind) {
                    continue;
                }

                // 2. Cas spécifique des types (Singletons) :
                //    Même si can_share(Type, Constant) est vrai, on ne compare pas leurs signatures.
                //    Une constante n'a pas de paramètres, contrairement à un prédicat ou une tâche.
                if (matches!(decl_kind, SymbolKind::PrimitiveType)
                    || matches!(usage_kind, SymbolKind::PrimitiveType))
                    && usage_kind != decl_kind
                {
                    continue;
                }

                // 3. Validation de la signature
                if !match_declaration_with_usage(
                    declaration,
                    usage,
                    symbol_table,
                    context,
                    type_checker,
                    diagnostic_manager,
                )? {
                    no_error = false; // Utilisation de false directement (plus idiomatique que &=)

                    let entry = context.syntax_tree().get_node(usage.node_id()).unwrap();

                    let error = Diagnostic::error_invalid_symbol_signature(
                        declaration.clone(),
                        usage.clone(),
                        context.provider(),
                        context.source(),
                        entry.span().clone(),
                    );

                    diagnostic_manager.add_diagnostic(error);
                }
            }
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
fn match_declaration_with_usage(
    declaration: &Declaration,
    usage: &Usage,
    symbol_table: &SymbolTable,
    context: &CheckContext,
    type_checker: &TypeChecker,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticError> {
    let ast_usage = context.syntax_tree().try_node(usage.node_id())?;

    for (index, argument_index) in ast_usage.children().iter().skip(1).enumerate() {
        let argument = context.syntax_tree().get_node(*argument_index).unwrap();

        let kind = match argument.kind() {
            AstKind::Variable => SymbolKind::Variable,
            AstKind::Object => SymbolKind::Constant,
            AstKind::Function => SymbolKind::Function,
            found => {
                return Err(SemanticError::unexpected_node_kind(
                    usage.node_id(),
                    vec![AstKind::Variable, AstKind::Object, AstKind::Function], // tous les attendus
                    found,
                ));
            }
        };

        if !match_argument(
            declaration,
            usage,
            symbol_table,
            context,
            argument,
            argument_index.as_usize(),
            kind,
            index,
            type_checker,
            diagnostic_manager,
        )? {
            return Ok(false);
        }
    }

    Ok(true)
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
/// * `symbol_table` - The table containing symbols.
/// * `name` - The name of the argument being matched.
/// * `kind` - The kind of the argument, such as `SymbolKind::Variable` or `SymbolKind::Function`.
/// * `index` - The index of the argument in the argument list.
/// * `type_checker` - A type_checker checker to validate type_checker consistency.
///
/// # Returns
///
/// `Result<bool, ParserInternalError>`: Returns `Ok(true)` if the argument matches the expected
/// declaration, or `Err` with a `ParserInternalError` if any validation error occurs.
fn match_argument(
    declaration: &Declaration,
    usage: &Usage,
    symbol_table: &SymbolTable,
    context: &CheckContext,
    argument: &AstNode,
    argument_index: usize,
    kind: SymbolKind,
    index: usize,
    type_checker: &TypeChecker,
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    // Retrieve the symbol name associated with the argument from the annotated syntax arena
    let name = context
        .syntax_tree()
        .try_node(NodeId::new(argument_index))?
        .try_ident()?;

    // Look up the corresponding declaration in the symbol table,
    // given the expected kind and usage scope
    let symbol_declaration = match symbol_table.resolve_declaration(&name, &kind, usage.scope())? {
        Some(decl) => decl,
        None => {
            return Err(SemanticCheckError::missing_declaration(
                name,
                usage.scope().clone(),
            ));
        }
    };

    // Get the declared arguments of the main declaration (the context declaration)
    let declared_arguments = match declaration.arguments() {
        Some(args) => args,
        None => {
            return Err(SemanticCheckError::missing_declaration_arguments(
                declaration.scope().clone(),
            ));
        }
    };

    // Retrieve the type_checker of the i-th declared argument (the one we are matching)
    let ty1 = match declared_arguments.get(index) {
        Some(arg) => arg.ty(),
        None => {
            return Err(SemanticCheckError::argument_index_out_of_bounds(
                index,
                declaration.scope().clone(),
            ));
        }
    };

    // Retrieve the type_checker of the symbol from the declaration found in the symbol table
    let ty2 = match symbol_declaration.ty() {
        Some(types) => types,
        None => {
            return Err(SemanticCheckError::missing_symbol_types(
                name,
                usage.scope().clone(),
            ));
        }
    };

    // Special case: allow a primitive task `(t ?x)` declared in a method
    // where `?x` has type_checker A to match an action `a` where `?x` has type_checker B,
    // as long as B is a supertype of A. This permits upcasting at usage time.
    //
    // Semantically this is questionable and should be handled explicitly during grounding.
    // This occurs, for example, in the `ultralight_cockpit` domain.
    //
    // Outside this exception, strict subtype checking is applied.

    // Check if ty1 is a subtype of ty2 (ty1 <: ty2)
    let is_subtype = type_checker.is_any_subtype_of(ty1, ty2)?;

    // Special tolerated case: accept a primitive task matching an action/method with a supertype
    // Special tolerated case: accept a primitive task matching an action/method/symbol with a supertype.
    // We use the centralized 'can_share_name_space_with' to validate this HDDL-specific overlap.
    if !is_subtype
        && usage.symbol_kind() == SymbolKind::Task
        && declaration
            .symbol_kind()
            .can_share_name_space_with(&usage.symbol_kind())
    {
        let warning = Diagnostic::warning_task_argument_is_supertype_of_declaration(
            symbol_declaration.clone(),
            ty1.clone(),
            ty2.clone(),
            context.provider(),
            context.source(),
            argument.span().clone(),
        );
        diagnostic_manager.add_diagnostic(warning);

        // Accept the match if ty1 is a supertype of ty2 (ty1 :> ty2)
        // This allows "upcasting" which is sometimes required in complex HDDL domains.
        return Ok(type_checker.is_any_supertype_of(ty1, ty2)?);
    }

    // Normal case: return the result of the subtype check
    Ok(is_subtype)
}
