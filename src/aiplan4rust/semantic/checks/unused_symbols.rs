use crate::aiplan4rust::diagnostic::Diagnostic;
use crate::aiplan4rust::diagnostic::DiagnosticManager;
use crate::aiplan4rust::interner::SymbolInterner;
use crate::aiplan4rust::lang::Requirement::DurativeActions;
use crate::aiplan4rust::lang::Requirement::NumericFluents;
use crate::aiplan4rust::lang::Requirement::Typing;
use crate::aiplan4rust::lang::Requirement::{Adl, Fluents};
use crate::aiplan4rust::semantic::checks::{CheckContext, SemanticCheckError};
use crate::aiplan4rust::semantic::symbol::SymbolKind;
use crate::aiplan4rust::semantic::symbol::{Declaration, Scope, SymbolEntry};
use crate::aiplan4rust::syntax::ast::{AstKind, AstNode};
use crate::aiplan4rust::tree::Tree;

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
    skip_symbols: &[SymbolKind],
    diagnostic_manager: &mut DiagnosticManager,
) -> Result<bool, SemanticCheckError> {
    let symbol_table = context.symbol_table();

    for symbol_entry in symbol_table.values() {
        for declaration in symbol_entry.declarations().values() {
            let declaration_kind = declaration.symbol_kind();

            // 1. Filter out declarations that should be ignored (built-ins, specific kinds, etc.)
            if skip_unused_symbol_declaration(declaration, context)?
                || skip_symbols.iter().any(|kind| *kind == declaration_kind)
            {
                continue;
            }

            // 2. Validate against PDDL built-in keywords (e.g., 'object', 'number')
            check_pddl_builtin_symbol_declaration(declaration, context, diagnostic_manager);

            // 3. Verify if the declaration is actually used or logically bound
            // We pass the entry and current declaration to the helper for clarity.
            if !has_valid_usage(symbol_entry, declaration) {
                let warning = Diagnostic::warning_unused_symbol(
                    declaration.clone(),
                    context.provider(),
                    context.source(),
                    declaration.span(),
                );
                diagnostic_manager.add_diagnostic(warning);
            }
        }
    }

    Ok(true)
}

/// Determines if a specific symbol declaration has a valid usage or a logical binding.
///
/// A declaration is considered "used" if:
/// 1. It is explicitly referenced elsewhere in the PDDL (e.g., in an action precondition).
/// 2. It is a `Predicate` that is logically completed by a `DerivedPredicate` definition.
///
/// # Parameters
/// - `entry`: The symbol table entry containing all declarations and usages for this identifier.
/// - `declaration`: The specific declaration being checked for usage.
fn has_valid_usage(entry: &SymbolEntry, declaration: &Declaration) -> bool {
    let decl_kind = declaration.symbol_kind();
    let decl_scope = declaration.scope();

    // --- PART 1: EXPLICIT USAGE SEARCH ---
    // Check if the symbol is consumed in the domain (e.g., in actions or effects).
    for usage in entry.usages().values() {
        let usage_kind = usage.symbol_kind();

        // A usage matches if it occurs within a compatible (descendant) scope.
        let scope_match = usage.scope().starts_with(decl_scope);

        // And if the kind is identical or semantically compatible.
        let kind_match =
            usage_kind == decl_kind || decl_kind.can_share_name_space_with(&usage_kind);

        if scope_match && kind_match {
            return true;
        }
    }

    // --- PART 2: SYMBOLIC BINDING LOGIC ---
    // If no explicit usage was found, check if this is a Predicate "saved"
    // by the existence of a matching Derived Predicate definition.
    if decl_kind == SymbolKind::Predicate {
        for other_decl in entry.declarations().values() {
            if other_decl.symbol_kind() == SymbolKind::DerivedPredicate {
                // The predicate is bound to a derived definition, so it is valid.
                return true;
            }
        }
    }

    // If no explicit usage or binding is found, the declaration is unused.
    false
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
        SymbolInterner::OBJECT_SYMBOL_ID
            if requirements.contains(&Typing) || requirements.contains(&Adl) =>
        {
            return Ok(true)
        }
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

/// Validates whether a symbol declaration conflicts with PDDL reserved built-in symbols.
///
/// This function identifies if a declaration uses a reserved identifier (such as `object`,
/// `number`, or `total-time`) based on the domain's active requirements. It then
/// determines the severity of the overlap:
///
/// 1. **Direct Collision**: The declaration uses the same name and the same [`SymbolKind`]
///    as the built-in (e.g., declaring `object` as a `PrimitiveType`). This is treated as
///    an **Error**.
/// 2. **Ambiguous Usage**: The declaration uses a reserved name but with a different,
///    yet compatible [`SymbolKind`] (e.g., declaring `object` as a `Constant`).
///    This is treated as a **Warning**.
/// 3. **Namespace Conflict**: The declaration uses a reserved name with an incompatible
///    kind. This is treated as a validation failure.
///
/// # Parameters
///
/// - `declaration`: A reference to the [`Declaration`] being validated.
/// - `context`: The [`CheckContext`] providing access to PDDL requirements,
///   the symbol interner, and diagnostic metadata.
/// - `diagnostic_manager`: A mutable reference to the [`DiagnosticManager`] for
///   recording errors and warnings.
///
/// # Returns
///
/// - `true`: If a conflict was identified and handled (even if it resulted in an error/warning).
///   This signal typically stops further standard validation for this specific symbol.
/// - `false`: If no recognized built-in keyword was matched, or if the conflict is
///   not considered a "keyword collision" requiring special diagnostics.
///
/// # Diagnostics
///
/// - **Error**: Emitted when a reserved keyword is re-declared with its native kind.
/// - **Warning**: Emitted when a reserved keyword is used for a different but compatible
///   namespace (as defined by `can_share_name_space_with`).
///
/// [`CheckContext`]: crate::semantics::CheckContext
/// [`Declaration`]: crate::semantics::Declaration
/// [`SymbolKind`]: crate::semantics::SymbolKind
fn check_pddl_builtin_symbol_declaration(
    declaration: &Declaration,
    context: &CheckContext,
    diagnostic_manager: &mut DiagnosticManager,
) -> bool {
    let requirements = context.declared_requirements();
    let current_kind = declaration.symbol_kind();

    // 1. On identifie si le nom est un mot-clé réservé selon les requirements
    let (expected_kind, reqs) = match declaration.symbol_ident() {
        SymbolInterner::OBJECT_SYMBOL_ID
            if requirements.contains(&Typing) || requirements.contains(&Adl) =>
        {
            (SymbolKind::PrimitiveType, vec![Typing, Adl])
        }
        SymbolInterner::NUMBER_SYMBOL_ID if requirements.contains(&NumericFluents) => {
            (SymbolKind::PrimitiveType, vec![NumericFluents, Fluents])
        }
        SymbolInterner::TOTAL_TIME_SYMBOL_ID if requirements.contains(&NumericFluents) => {
            (SymbolKind::Function, vec![NumericFluents, Fluents])
        }
        SymbolInterner::DURATION_VARIABLE_SYMBOL_ID if requirements.contains(&DurativeActions) => {
            (SymbolKind::Variable, vec![DurativeActions])
        }
        _ => return true, // Pas un mot-clé, on valide la déclaration
    };

    // 2. CAS A : Collision Directe (Genre identique)
    // L'utilisateur essaie de déclarer "object" comme un "PrimitiveType".
    // C'est ta stratégie : ERREUR car ça entre en conflit avec ta racine interne.
    if current_kind == expected_kind {
        let error = Diagnostic::error_symbol_conflicts_with_keyword(
            declaration.clone(),
            expected_kind,
            reqs,
            context.provider(),
            context.source(),
            declaration.span().clone(),
        );
        diagnostic_manager.add_diagnostic(error);
        return true; // On marque comme trouvé mais invalide
    }

    // 3. CAS B : Usage Ambigu (Genre différent)
    // L'utilisateur déclare "object" comme "Constant".
    // On vérifie si notre nouvelle stratégie autorise ce partage.
    if current_kind.can_share_name_space_with(&expected_kind) {
        // C'est autorisé (ex: Constant vs Type), mais c'est risqué.
        // -> WARNING (ton ancienne stratégie d'ambiguïté)
        let warning = Diagnostic::warning_symbol_declared_ambiguously_as_keyword(
            declaration.clone(),
            expected_kind,
            reqs,
            context.provider(),
            context.source(),
            declaration.span(),
        );
        diagnostic_manager.add_diagnostic(warning);
        false
    } else {
        // CAS C : Conflit Radical (ex: Variable nommée "object")
        // Ce n'est pas autorisé par can_share_name_space_with.
        // On pourrait ici mettre une erreur plus grave ou rester sur le warning.
        false
    }
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
