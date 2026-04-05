use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::Requirement::{DurativeActions, NumericFluents};
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::semantic::symbol::{Declaration, Scope, SymbolKind};
use crate::aiplan4rust::semantic::SemanticError;
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::Tree;
use crate::{DiagnosticManager, SymbolTable};

/// Checks for symbol declarations that are never used within their valid scope.
///
/// This function iterates over all declarations in the symbol table and verifies if each one
/// has at least one corresponding usage. It emits a warning diagnostic for any declared
/// symbol that appears to be redundant.
///
/// # Validation Steps
///
/// 1. **Filtering**: Skips symbols based on `skip_symbols` or domain-specific rules
///    (e.g., built-in types, requirements).
/// 2. **Keyword Validation**: Checks if the declaration conflicts with PDDL reserved
///    keywords (delegated to [`check_pddl_builtin_symbol_declaration`]).
/// 3. **Usage Analysis**: Determines if the specific declaration is referenced by
///    at least one usage in a compatible scope.
///
/// # Parameters
///
/// - `context`: A reference to the [`CheckContext`] providing access to the symbol table,
///   interner, and diagnostic metadata.
/// - `skip_symbols`: A slice of [`SymbolKind`] to be explicitly excluded from this check
///   (e.g., `SymbolKind::Requirement`).
/// - `diagnostic_manager`: A mutable reference to the [`DiagnosticManager`] where
///   warning diagnostics are recorded.
///
/// # Returns
///
/// - `Ok(true)`: The check completed successfully. Note that unused symbols do not
///   trigger an `Ok(false)` as they are reported as warnings, not hard errors.
/// - `Err(SemanticCheckError)`: An internal error occurred during symbol table traversal.
///
/// # Example
///
/// ```rust
/// let check_ctx = context.as_check_context(Provider::Analyzer);
/// let skip = [SymbolKind::Requirement];
///
/// check_unused_symbols(&check_ctx, &skip, &mut diagnostic_manager)?;
/// ```
///
/// [`CheckContext`]: crate::semantics::CheckContext
/// [`SymbolKind`]: crate::semantics::SymbolKind
/// [`DiagnosticManager`]: crate::diagnostics::DiagnosticManager
pub fn check_unused_symbols(
    context: &CheckContext,
    symbol_table: &SymbolTable,
    skip_symbols: &[SymbolKind],
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticError> {
    let mut unused_count = 0;
    const MAX_REPORTS: usize = 100;

    for symbol_entry in symbol_table.values() {
        for declaration in symbol_entry.declarations().values() {
            if !declaration.usages().is_empty()
                || !declaration.derivations().is_empty()
                || (declaration.is_derived() && declaration.derived_source().is_some())
            {
                continue;
            }

            // 2. Filtres
            if skip_symbols.contains(&declaration.symbol_kind())
                || skip_unused_symbol_declaration(declaration, context)?
            {
                continue;
            }

            unused_count += 1;

            // 3. LA CASSE : Si on dépasse la limite, on s'arrête IMMÉDIATEMENT
            if unused_count > MAX_REPORTS {
                // Optionnel : ajouter un diagnostic spécial "Trop d'erreurs, j'arrête"
                println!("\x1b[1;31m[!] Trop de symboles inutilisés dans ce fichier. Arrêt de l'analyse.\x1b[0m");
                return Ok(true); // On retourne Ok(true) car l'analyse est finie (même si tronquée)
            }

            diagnostic_manager.add_diagnostic(Diagnostic::warning_unused_symbol(
                declaration.clone(),
                context.provider(),
                context.source(),
                declaration.span(),
            ));
        }
    }

    Ok(true)
}

/// Determines whether a declaration should be skipped during unused symbol checking.
///
/// This function returns `true` if the declaration represents a symbol kind or context
/// that should not be considered when checking for unused symbols. This includes:
/// - Declarations of built-in domain constructs like `Requirement`, `Action`, `DASymbol`, or
///   `Method`.
/// - Declarations of symbols like `DomainName`, `ProblemName`, or those associated with
///   specific PDDL requirements (e.g., `Typing`, `NumericFluents`, `DurativeActions`).
/// - Variables that appear within the scope of atomic formula skeletons, atomic function skeletons,
///   or task definitions—these are typically considered local and not subject to unused symbol
///   diagnostics.
///
/// # Parameters
/// - `declaration`: A reference to the `Declaration` to evaluate.
/// - `annotated_syntax_tree`: A reference to the `AnnotatedSyntaxTree`, used for contextual
///   analysis.
///
/// # Returns
/// - `Ok(true)` if the declaration should be skipped during unused symbol checking.
/// - `Ok(false)` if it should be checked.
/// - `Err(ParserInternalError)` if the required AST context cannot be retrieved.
///
/// # Note
/// Skipping these declarations avoids false positives when analyzing domain-level constructs or
/// locally scoped variables that are not meant to be globally referenced.
fn skip_unused_symbol_declaration(
    declaration: &Declaration,
    context: &CheckContext,
) -> Result<bool, SemanticCheckError> {
    // Skip if the declaration is of a built-in kind: Requirement, Action, DASymbol, or Method
    if matches!(
        declaration.symbol_kind(),
        SymbolKind::DomainName
            | SymbolKind::ProblemName
            | SymbolKind::Requirement
            | SymbolKind::Action
            | SymbolKind::DASymbol
            | SymbolKind::Method
            | SymbolKind::TaskID
    ) {
        return Ok(true);
    }

    let requirements = context.declared_requirements();
    match declaration.symbol_ident() {
        SymbolInterner::NUMBER_SYMBOL_ID | SymbolInterner::TOTAL_TIME_SYMBOL_ID
            if requirements.contains(&NumericFluents) =>
        {
            return Ok(true)
        }
        SymbolInterner::DURATION_VARIABLE_SYMBOL_ID if requirements.contains(&DurativeActions) => {
            return Ok(true)
        }
        _ => {}
    }

    // Skip if the declaration is a variable and its scope contains an atomic skeleton syntax.
    // Variables within such scopes are typically local and do not need to be checked for duplicates.
    let scope = declaration.scope();

    // Check if scope contains specific AST kinds
    let contains_formula =
        scope_contains_node_of_kind(scope, AstKind::AtomicFormulaSkeleton, context.syntax_tree())?;
    let contains_function = scope_contains_node_of_kind(
        scope,
        AstKind::AtomicFunctionSkeleton,
        context.syntax_tree(),
    )?;
    let contains_task =
        scope_contains_node_of_kind(scope, AstKind::TaskDef, context.syntax_tree())?;

    // Check if symbol is a variable
    let is_variable = matches!(declaration.symbol_kind(), SymbolKind::Variable);

    // Return true if variable and any AST kind is present
    if is_variable && (contains_formula || contains_function || contains_task) {
        return Ok(true);
    }

    Ok(false)
}

/// Checks if the given `scope` contains at least one AST node of the specified `kind`.
///
/// This function iterates over all `NodeId`s in the scope's stack and attempts to
/// retrieve the corresponding AST nodes from the provided AST arena. If any node
/// matches the given `kind`, it returns `Ok(true)`. If no nodes match, it returns
/// `Ok(false)`. If any node ID is invalid or cannot be found, it returns an error.
///
/// # Parameters
/// - `scope`: Reference to the `Scope` to search within.
/// - `kind`: The `AstKind` to look for in the scope.
/// - `ast`: Reference to the AST arena containing all AST nodes.
///
/// # Returns
/// - `Ok(true)` if at least one node of the specified kind is found in the scope.
/// - `Ok(false)` if no matching nodes are found.
/// - `Err(ParserInternalError)` if a node ID in the scope does not correspond to any AST node.
///
/// # Example
/// ```
/// let found = scope_contains_node_of_kind(&scope, AstKind::Function, &ast)?;
/// if found {
///     println!("Scope contains a function node");
/// }
/// ```
fn scope_contains_node_of_kind(
    scope: &Scope,
    kind: AstKind,
    ast: &Tree<AstNode>,
) -> Result<bool, SemanticCheckError> {
    for &id in scope.iter() {
        let node = ast.try_node(id)?;
        if node.kind() == kind {
            return Ok(true);
        }
    }
    Ok(false)
}
