use crate::aiplan4rust::error::error_manager::ErrorManager;
use crate::aiplan4rust::error::parsing_error::{ParserErrorKind, ParsingError};
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantics::annotated_syntax_tree::AnnotatedSyntaxTree;
use crate::aiplan4rust::semantics::symbol::Scope;
use crate::aiplan4rust::semantics::symbol::{Declaration, SymbolKind};
use std::collections::HashSet;

/// Checks for duplicate symbol declarations in the symbol table and logs errors
/// to the error manager if duplicates are found.
///
/// Duplicate checking is skipped for symbols of kind `DomainName` and
/// `ProblemName` because these kinds are often used as symbols for types or
/// predicates, where duplicates may be allowed.
///
/// # Parameters
/// - `symbol_table`: A reference to the symbol table to check for duplicates.
///
/// # Returns
/// - `Ok(())` if the check completes (errors are logged via the error manager).
/// - `Err(ParserInternalError)` if an error occurs during processing.
pub fn check(
    tree: &AnnotatedSyntaxTree,
    errors: &mut ErrorManager,
) -> Result<bool, ParserInternalError> {
    let mut checked = true;
    let symbol_table = tree.symbol_table();
    let ast_table = tree.syntax_tree();

    // Iterate over each symbol in the symbol table.
    for symbol in symbol_table.values() {
        let mut seen_scopes = HashSet::new();
        let symbol_name = symbol.name(); // Avoid multiple borrows of `symbol`

        // Iterate over each declaration for the symbol.
        for declaration in symbol.declarations() {
            // Skip duplicate checks for symbols of kind DomainName or ProblemName.
            // These can be used as symbols for types or predicates, so duplicates may be allowed.
            if skip_duplicated_declaration(declaration)? {
                /*println!(
                    "SKIP DUPLICATED SYMBOL: {} {}",
                    declaration.kind(),
                    symbol.name()
                );*/
                continue;
            }

            let ast_entry = ast_table.get_entry(declaration.ast()).unwrap();
            if seen_scopes
                .iter()
                .any(|s: &&Scope| declaration.scope().starts_with(s))
            {
                checked = false;
                let (line, column) = ast_entry.span().start_position();
                let content = format!(
                    "Duplicate declaration of symbol '{}' in a related scope at line {} column {}.",
                    symbol_name, line, column
                );
                let error = ParsingError::new(
                    ParserErrorKind::ParseError,
                    Some(tree.filename().clone()),
                    line,
                    column,
                    content,
                );
                errors.add_error(error);
            } else {
                // Ajouter à seen_scopes si aucun élément existant ne commence par declaration.scope()
                seen_scopes.insert(declaration.scope());
            }
        }
    }

    Ok(checked)
}

fn skip_duplicated_declaration(declaration: &Declaration) -> Result<bool, ParserInternalError> {
    if matches!(
        declaration.kind(),
        SymbolKind::DomainName | SymbolKind::ProblemName
    ) {
        return Ok(true);
    }

    Ok(false)
}
