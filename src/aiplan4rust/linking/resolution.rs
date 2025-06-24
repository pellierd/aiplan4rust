use std::collections::HashMap;
use crate::aiplan4rust::frontend::ParserInternalError;
use crate::aiplan4rust::semantic::{SemanticContext, SymbolTable};
use crate::aiplan4rust::semantic::symbol::{Declaration, Scope, SymbolOrigin, Usage};
use crate::aiplan4rust::syntax::elements::Ident;
use crate::aiplan4rust::interner::{InternerMergeResult, StringInterner};

/// Resolves symbols by merging string interners from domain and problem,
/// remapping identifiers in the problem to a unified global interner,
/// and updating the problem's symbol table with declarations from the domain.
///
/// This is typically part of the linking phase where the domain context
/// and problem context are combined to produce a consistent semantic environment.
///
/// # Arguments
///
/// * `domain` - Reference to the semantic context of the domain (read-only).
/// * `problem` - Mutable reference to the semantic context of the problem to update.
///
/// # Errors
///
/// Returns `ParserInternalError` if symbol resolution fails during table updates.
///
/// # Example
///
/// ```ignore
/// resolve_symbols(&domain_context, &mut problem_context)?;
/// ```
pub fn resolve_external_references(
    domain: &SemanticContext,
    problem: &mut SemanticContext,
) -> Result<(), ParserInternalError> {




    // 3. Update problem's symbol table by injecting declarations from the domain symbol table
    update_problem_symbols_table_from_domain(problem, domain.symbol_table())?;

    Ok(())
}



/// Updates the problem's symbol table by adding declarations found in the domain's symbol table.
///
/// For each symbol in the problem's symbol table that has no declarations,
/// this function attempts to find matching declarations from the domain symbol table
/// based on the symbol's name and usage kind. Matching declarations are cloned,
/// marked as originating from the domain, and added to the problem's symbol.
///
/// # Arguments
///
/// * `problem` - Mutable reference to the problem semantic context.
/// * `domain_symbol_table` - Reference to the domain's symbol table.
///
/// # Errors
///
/// Propagates `ParserInternalError` if resolving declarations fails.
///
/// # Example
///
/// ```ignore
/// update_problem_symbols_table_from_domain(&mut problem_context, &domain_symbol_table)?;
/// ```
fn update_problem_symbols_table_from_domain<'a>(
    problem: &'a mut SemanticContext,
    domain_symbol_table: &'a SymbolTable,
) -> Result<(), ParserInternalError> {
    let mut declared = Vec::new();
    let mut undeclared = Vec::new();

    collect_declared_and_undeclared_symbols(problem, domain_symbol_table, &mut declared, &mut undeclared)?;

    let problem_symbol_table = problem.symbol_table_mut();

    for (symbol_name, declaration) in declared {
        if let Some(symbol) = problem_symbol_table.get_symbol_mut(symbol_name) {
            symbol.add_declaration(declaration);
        }
    }

    Ok(())
}

/// Collects symbol declarations from the domain symbol table for symbols in the problem
/// that currently lack declarations, and gathers undeclared symbols.
///
/// Does not mutate symbol tables directly; instead collects:
/// - `declared`: pairs of `(symbol_name, declaration)` to add to the problem.
/// - `undeclared`: symbols and their usages that could not be resolved.
///
/// # Arguments
///
/// * `problem` - Reference to the problem semantic context.
/// * `domain_symbol_table` - Reference to the domain's symbol table.
/// * `declared` - Mutable vector collecting declarations to add to problem symbols.
/// * `undeclared` - Mutable vector collecting unresolved symbols.
///
/// # Returns
///
/// Returns `Ok(true)` if all symbols were resolved, `Ok(false)` if some are undeclared,
/// or propagates errors encountered during resolution.
///
/// # Example
///
/// ```ignore
/// let mut declared = Vec::new();
/// let mut undeclared = Vec::new();
/// let all_resolved = collect_declared_and_undeclared_symbols(
///     &problem_context,
///     &domain_symbol_table,
///     &mut declared,
///     &mut undeclared,
/// )?;
/// if !all_resolved {
///     // handle diagnostics for undeclared symbols
/// }
/// ```
fn collect_declared_and_undeclared_symbols<'a>(
    problem: &'a SemanticContext,
    domain_symbol_table: &'a SymbolTable,
    declared: &mut Vec<(Ident, Declaration)>,
    undeclared: &mut Vec<(Ident, &'a Usage)>,
) -> Result<bool, ParserInternalError> {
    let problem_symbol_table = problem.symbol_table();
    let mut all_resolved = true;

    for symbol in problem_symbol_table.values() {
        if symbol.declarations().is_empty() {
            for usage in symbol.usages() {
                let domain_declaration_option = domain_symbol_table.resolve_declaration(
                    &symbol.name(),
                    &usage.symbol_kind(),
                    &Scope::root(),
                )?;

                if let Some(domain_declaration) = domain_declaration_option {
                    let mut domain_declaration = domain_declaration.clone();
                    domain_declaration.set_origin(SymbolOrigin::Domain);
                    declared.push((symbol.name(), domain_declaration));
                } else {
                    undeclared.push((symbol.name(), usage));
                    all_resolved = false;
                }
            }
        }
    }

    Ok(all_resolved)
}
