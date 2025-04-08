use crate::aiplan4rust::error::error_manager::ErrorManager;
use crate::aiplan4rust::error::parsing_error::{ParserErrorKind, ParsingError};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic_analyser::annotated_syntax_tree::AnnotatedSyntaxTree;
use crate::aiplan4rust::semantic_analyser::heap_syntax_tree::HeapSyntaxTree;
use crate::aiplan4rust::semantic_analyser::symbol::{Declaration, Symbol, SymbolKind};
use crate::aiplan4rust::syntax::elements::Requirement::{
    Adl, DurativeActions, NumericFluents, Typing,
};
use crate::aiplan4rust::syntax::lexer::token::{
    DURATION_VARIABLE, NUMBER_TYPE, OBJECT_TYPE, TOTAL_TIME,
};
use crate::aiplan4rust::syntax::tree::SyntaxNodeKind;

/// Checks for symbols that are declared but never used in the same or a parent scope.
/// This function reports warnings for any unused symbols found.
///
/// # Parameters
/// - `symbol_table`: A reference to the symbol table containing symbols to be checked.
/// - `skip_symbols`: A list of `SymbolKind` values representing symbols that should be ignored during checking.
///
/// # Returns
/// - `Ok(())` if the check completes successfully. Warnings are logged through the error manager.
/// - `Err(ParserInternalError)` if an error occurs during processing.
///
/// # Note
/// - The built-in PDDL symbols `"object"` and `"number"` are ignored, as they are always valid.
/// - Symbols whose kind appears in `skip_symbols` are not checked.
pub fn check(
    tree: &AnnotatedSyntaxTree,
    skip_symbols: &[SymbolKind],
    errors: &mut ErrorManager,
) -> Result<bool, ParserInternalError> {
    let mut no_error = true;

    let symbol_table = tree.symbol_table();
    let ast_table = tree.syntax_tree();

    // Iterate over each symbol in the symbol table.
    for symbol in symbol_table.values() {
        for declaration in symbol.declarations() {
            if skip_unused_symbol_declaration(symbol, declaration, ast_table)?
                || skip_symbols.contains(declaration.kind())
            {
                /*println!(
                    "SKIP UNUSED SYMBOL: {} {}",
                    declaration.kind(),
                    symbol.name()
                );*/
                continue;
            }
            check_pddl_builtin_symbol_declaration(symbol, declaration, tree, errors)?;

            let declaration_scope = declaration.scope();
            let declaration_kind = declaration.kind();

            // Trouver une utilisation du symbole dans le même ou un sous-scope
            let usage_opt = symbol
                .usages()
                .iter()
                .find(|usage| usage.scope().starts_with(&declaration_scope));

            match usage_opt {
                None => {
                    // Aucun usage trouvé : générer un avertissement
                    let entry = ast_table.get_entry(declaration.ast()).unwrap();
                    let (line, column) = entry.span().start_position();
                    let content = format!(
                        "Symbol '{}' declared at line {} column {} but never used.",
                        symbol.name(),
                        line,
                        column
                    );
                    let warning = ParsingError::new(
                        ParserErrorKind::ParseWarning,
                        Some(tree.filename().clone()),
                        line,
                        column,
                        content,
                    );
                    errors.add_error(warning);
                }
                Some(usage) => {
                    // Vérifier la cohérence du type entre la déclaration et l'utilisation
                    if usage.kind() != declaration_kind {
                        no_error = false;
                        let entry = ast_table.get_entry(usage.ast()).unwrap();
                        let (line, column) = entry.span().start_position();
                        let content = format!(
                            "Symbol '{}' declared as {:?} but used as {:?} at line {} column {}.",
                            symbol.name(),
                            declaration_kind,
                            usage.kind(),
                            line,
                            column
                        );
                        let error = ParsingError::new(
                            ParserErrorKind::ParseError, // Erreur de type
                            Some(tree.filename().clone()),
                            line,
                            column,
                            content,
                        );
                        errors.add_error(error);
                    }
                }
            }
            //println!("USAGE OPT: {} {:?}", symbol, usage_opt);
        }
    }

    Ok(no_error)
}

/// Determines whether a declaration should be skipped during duplicate checking.
///
/// This function returns `true` if the declaration's kind indicates that it is not
/// subject to duplicate checks. In particular, it skips declarations of symbols of kind
/// `Requirement`, `Action`, or `DASymbol`, as well as variables declared within the scope
/// of atomic formula or atomic function skeletons. Such symbols are typically declared
/// in the domain and are not intended to be checked for duplicates in problem files.
///
/// # Parameters
/// - `declaration`: A reference to the `Declaration` to check.
///
/// # Returns
/// - `true` if the declaration should be skipped,
/// - `false` otherwise.
fn skip_unused_symbol_declaration(
    symbol: &Symbol,
    declaration: &Declaration,
    ast_table: &HeapSyntaxTree,
) -> Result<bool, ParserInternalError> {
    // Skip if the declaration is of a built-in kind: Requirement, Action, DASymbol or Method
    if matches!(
        declaration.kind(),
        SymbolKind::DomainName
            | SymbolKind::ProblemName
            | SymbolKind::Requirement
            | SymbolKind::Action
            | SymbolKind::DASymbol
            | SymbolKind::Method
    ) {
        return Ok(true);
    }

    match symbol.name().as_str() {
        OBJECT_TYPE
            if ast_table.requirements().contains(&Typing)
                || ast_table.requirements().contains(&Adl) =>
        {
            return Ok(true)
        }
        NUMBER_TYPE | TOTAL_TIME if ast_table.requirements().contains(&NumericFluents) => {
            return Ok(true)
        }
        DURATION_VARIABLE if ast_table.requirements().contains(&DurativeActions) => {
            return Ok(true)
        }
        _ => {}
    }

    // Skip if the declaration is a variable and its scope contains an atomic skeleton node.
    // The variables have local scope and does not need to be checked
    if matches!(declaration.kind(), SymbolKind::Variable)
        && (declaration
            .scope()
            .contains_ast_of_kind(SyntaxNodeKind::AtomicFormulaSkeleton, ast_table)?
            || declaration
                .scope()
                .contains_ast_of_kind(SyntaxNodeKind::AtomicFunctionSkeleton, ast_table)?
            || declaration // Add for HDDL
                .scope()
                .contains_ast_of_kind(SyntaxNodeKind::TaskDef, ast_table)?)
    {
        return Ok(true);
    }

    Ok(false)
}

/// Checks if a symbol is properly declared as a built-in symbol according to the PDDL
/// specifications.
///
/// This function verifies whether a given symbol matches the expected type of a built-in symbol
/// based on the PDDL requirements present in the `ast_table`. For example, it checks if a
/// symbol like `OBJECT_TYPE`, `NUMBER_TYPE`, or `TOTAL_TIME` is correctly declared with the
/// appropriate kind (e.g., `PrimitiveType`, `Function`, `Variable`) based on the domain's
/// requirements.
///
/// # Arguments
/// * `symbol`: A reference to the `Symbol` that needs to be checked.
/// * `declaration`: A reference to the `Declaration` of the symbol, which contains type
///   information.
/// * `ast_table`: A reference to the `AstTable` that contains the domain's requirements and
///   other metadata.
///
/// # Returns
/// * `Ok(true)` if the symbol's declaration matches the expected type and is correct according
///   to the PDDL requirements.
/// * `Ok(false)` if the symbol's declaration is incorrect or doesn't match any recognized
///   built-in symbol declaration.
///
/// # Errors
/// If the symbol's declaration is invalid, an error is logged with the line and column number
/// of the invalid declaration.

fn check_pddl_builtin_symbol_declaration(
    symbol: &Symbol,
    declaration: &Declaration,
    tree: &AnnotatedSyntaxTree,
    errors: &mut ErrorManager,
) -> Result<bool, ParserInternalError> {
    let ast_table = tree.syntax_tree();
    let (expected_kind, requirement, error_message) = match symbol.name().as_str() {
        OBJECT_TYPE
            if ast_table.requirements().contains(&Typing)
                || ast_table.requirements().contains(&Adl) =>
        {
            (SymbolKind::PrimitiveType, ":typing", "builtin type")
        }
        NUMBER_TYPE if ast_table.requirements().contains(&NumericFluents) => (
            SymbolKind::PrimitiveType,
            ":numeric-fluents",
            "builtin type",
        ),
        TOTAL_TIME if ast_table.requirements().contains(&NumericFluents) => {
            (SymbolKind::Function, ":numeric-fluents", "builtin function")
        }
        DURATION_VARIABLE if ast_table.requirements().contains(&DurativeActions) => (
            SymbolKind::Variable,
            ":durative-actions",
            "builtin variable",
        ),
        _ => return Ok(false), // Aucun cas ne correspond, donc on retourne directement `false`
    };

    // Vérifier si le symbole a le bon type
    if *declaration.kind() != expected_kind {
        if let Some(entry) = ast_table.get_entry(declaration.ast()) {
            let (line, column) = entry.span().start_position();
            let content = format!(
                "'{}' is a {} in a domain with {} requirement.",
                symbol.name(),
                error_message,
                requirement
            );
            let error = ParsingError::new(
                ParserErrorKind::ParseError,
                Some(tree.filename().clone()),
                line,
                column,
                content,
            );
            errors.add_error(error);
        }
        return Ok(false);
    }

    Ok(true)
}
